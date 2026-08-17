---
generation: 14
parent-session: c7c7626a-c43e-4897-84e1-1d8e517d6abc
next-action-class: executable
---

# Generation 14 — the rebuild fix is landed and mutant-verified; the query hang is re-rooted and one candidate is untested

## The goal and authorization, verbatim

Dale, 2026-08-14:

> /my-way fix cass to completion and 100% green working state and completely up to date or tell me why it can't or /grill-me with any questions.

Dale, 2026-08-15:

> your usage is good now. finish this to completion

Dale, 2026-08-16:

> if your senior dev recommendation is to delete those 4 stale directories do it. if that is what progresses us toward the goal. and I would prefer you just do that and only stop when it would break the pipeline or running sessions or if there is a true blocker or ambiguity or conflict rather than sitting here for something that I am going to just ask your recommendation on and agree to

Dale, 2026-08-16:

> there has been SO MUCH WORK done on cass and it feels like you agents are all wondering around blind folded in a mine field in a lava flow. can I PLEASE have an agent like Opus 5 on high thinking....oh, that's what you are... not run around incircles playing whac-a-mole? what the heck?!

**No approval transfers to you.** You may not file anything upstream on
frankensqlite/asupersync, delete any file, force-push, rewrite history, change
repo visibility, or run `cass sources agents exclude` (that last would destroy
3,877 conversations existing nowhere else). Dale's standing rule: **never delete
a file without express permission**, including files you created.

## WHERE THE WORK IS — read this first

Everything from generation 13 and 14 is on branch **`worktree-cass-p3kgr-gen13`**,
pushed to origin. It is NOT on `main`.

```
26932422  fix(p3kgr): the lexical rebuild ran a whole-archive GROUP BY on every healthy run
6e7b50c9  wip(p3kgr): express the lexical fingerprint's MAX(id) as a descending seek
```

The generation-13 handoff said to commit on `main` and push `main` and
`main:master`. **This session could not**: it runs as a background job, and the
harness both refuses edits in the shared checkout until the session enters a
worktree and forbids pushing to main/master. That is also what commit `73faacb1`
("background jobs cannot push main") already recorded. So the work is complete
and durable on a branch, and **landing it on `main` is the one action left that
needs a session which can push main.** Nothing else is blocked on it.

The worktree is at `.claude/worktrees/cass-p3kgr-gen13`.

## Defect 1 — the guard inversion: FIXED, TESTED, MUTANT-VERIFIED, COMMITTED

`src/storage/sqlite.rs`, `list_conversation_footprints_for_lexical_rebuild`.
Its doc comment says the exact-count raise is a fallback for conversations
*missing* tail metadata; the guard fired when at least one **had** it. Every
healthy rebuild therefore ran `SELECT conversation_id, COUNT(*) FROM messages
GROUP BY conversation_id` over the whole archive — 77 ms under stock SQLite,
8,927,989 ms (2h28m) under fsqlite. That is why no rebuild ever completed.

The production change is **byte-identical to generation 13's proven tree**
(verified by `diff` before committing), so the measurement it earned still
stands, on the shipping fsqlite 0.1.5 pin with no bump:

```
lexical refresh ledger published total_duration_ms=57351 failed_phase=""
search lexical self-heal completed action="rebuilt-from-canonical-db" indexed_docs=2334366
```

Two tests were added, because nothing caught the inversion:

- `lexical_rebuild_tail_estimates_understate_message_total_reads_the_ungrouped_total`
  pins the predicate directly (current / over-stated / stale-low).
- `list_conversation_footprints_for_lexical_rebuild_skips_exact_count_raise_when_tail_totals_agree`
  pins the call site through the global check's documented ceiling.

**The second is mutant-verified, not assumed.** Restoring the pre-fix guard:

```
baseline   20 passed, 0 failed
mutant     19 passed, 1 failed  — only the new test, footprint raised 2 -> 3
```

`cargo fmt` is clean for `src/storage/sqlite.rs` (the pre-existing fmt drift in
`src/connectors/codex.rs` and `tests/golden_robot_json.rs` is on `main` and was
deliberately not touched).

## Defect 2 — fsqlite cannot plan `GROUP BY`: unchanged, still upstream

Reproduction at `/tmp/fsq-probe/src/main.rs`. Filing needs Dale's explicit
approval and is not inherited. Defect 1 makes cass stop issuing the statement,
so this no longer blocks anything.

## Defect 3 — the query hang: RE-ROOTED. The inherited hypothesis is FALSIFIED.

Generation 13 said the cause was page-cache thrash and the falsifier was to
raise `cache_size` to 2 GB. **Do not do that.** The eviction churn in the sample
is real but it is the amplifier, not the cause.

### What is now established

Every `cass search` fingerprints the database to decide whether the published
lexical index is still valid — `search_lexical_self_heal_diagnosis` →
`lexical_storage_fingerprint_for_db` (`src/lib.rs:19372`) → two `MAX(id)`
statements (`src/indexer/mod.rs`, `lexical_rebuild_content_fingerprint`).

Using the product's own `CASS_PREP_PROFILE` instrument on the 23 GB probe
archive `/tmp/fsq-probe-data/prod.db`:

| | |
|---|---|
| `MAX(id) FROM conversations` (27,441 rows) | **3,578 ms** quiet, 339 ms traced |
| `MAX(id) FROM messages` (2,335,514 rows) | **never observed to return** |

Two runs were killed at 980 s and ~16 min, both against an **already-built**
2.3 GB index, so this is the query phase and not a rebuild. The VDBE reports
`opcode_count=82337` for the conversations statement — three opcodes per row, a
row-by-row scan — and the trace shows the literal `0` re-tokenized once per row
under `path="fast" reason="table_query_row"`. `id` is `INTEGER PRIMARY KEY`, so
stock SQLite answers with a one-row seek (`EXPLAIN QUERY PLAN` → `SEARCH
messages`, 0.9 ms).

### What is falsified, and how

A standalone probe (`~/.claude-accounts/katherine/jobs/c7c7626a/tmp/fsq-maxprobe`,
fsqlite 0.1.5 with the `fts5` feature cass uses) **cannot reproduce it**. Several
of these cells ran while the cass process was concurrently hung on that exact
statement against that exact file:

| controlled | result |
|---|---|
| read-write open, no pragmas | 7 ms |
| read-write open, cass's four pragmas incl. `cache_size = -65536` | 6 ms |
| `SQLITE_OPEN_READ_ONLY`, no pragmas | 58 ms |
| `SQLITE_OPEN_READ_ONLY` + cass's pragmas — cass's exact recipe | 56 ms |
| the above **plus cass's preceding statements in order** | 0 ms |
| the above through the compat `query_row_map` entry point cass calls | 0 ms |
| `--verbose` tracing overhead | not it — quiet is *worse* (3,578 vs 339 ms) |
| a second connection to the same DB | ruled out by `lsof` — one fd |

**So the trigger is inside the cass process and it is NOT isolated.** Do not
report it as understood. One caveat on the record: cells 1 and 2 opened the file
read-write and prod.db's mtime moved at 09:12:46Z, so the later read-only cells
ran on a mutated specimen — but the contemporaneous cass control re-run at
09:14 was still slow on that same mutated file, which is what makes the
in-process conclusion hold.

### The untested candidate — commit 6e7b50c9

`lexical_rebuild_content_fingerprint` now issues
`SELECT COALESCE((SELECT id FROM t ORDER BY id DESC LIMIT 1), 0)` instead of
`SELECT COALESCE(MAX(id), 0) FROM t`. Same value by construction; measured 0 ms
on the same connection recipe where `MAX` measured 56 ms.

The equivalence is proven — the existing test
`lexical_rebuild_content_fingerprint_uses_table_max_ids` pins the fingerprint
**value** (`content-v1:2:9:11`), not the SQL text, and passes unchanged.

### It was measured, and it is NOT the answer

The release build finished (rc=0, 7m32s) and the acceptance run was executed
before this session wound down. On the same 23 GB probe archive, quiet, same
script as the control:

| | `MAX(id)` | descending seek |
|---|---|---|
| `fingerprint_conversations` | 3,578 ms | **964 ms** |
| `fingerprint_messages` | never returned | **still had not returned at 3.5 min** |

So the shape is a 3.7x improvement and **not a fix**. 964 ms for one row out of
27,441 is still a scan, which means the in-process trigger dominates the SQL
shape. Do not report defect 3 as fixed, and do not spend another generation on
SQL rewrites at this call site.

That run may still have been going when this session ended — it was pid 74465,
writing `~/.claude-accounts/katherine/jobs/c7c7626a/tmp/quiet/search.err`. If it
eventually printed `step=fingerprint_messages`, that number is worth having:
"finishes in N minutes" is a different defect from "never finishes".

## The exact next action

1. **Decide what to do with 6e7b50c9, then act.** It is a real 3.7x improvement
   with a proven-identical result and no new test debt, so the recommendation is
   to **keep it** and rewrite its commit message from `wip` to a plain `perf`
   that states the measured 3,578 → 964 ms and says plainly that it does not
   resolve the hang. Reverting is defensible too — it is a change whose
   mechanism is not understood. What is not defensible is leaving a commit
   titled `wip` claiming the measurement is pending when it is not.
2. **Then attack the real trigger**, which is inside the cass process and is not
   any of the things already eliminated below. The probe has exhausted the file,
   the connection recipe, the statement sequence and the call entry point. The
   untried lead is what the cass process holds that a bare probe does not: the
   tantivy index is already open (2.3 GB, 159 shards) and the asupersync runtime
   is installed by the time the fingerprint runs. The cheapest discriminator is
   to make the probe do the same — or, from the other side, to find a cass entry
   point that computes the fingerprint *without* first opening the search index,
   and see whether it is fast there.
3. **Land the branch on `main`** from a session that can push main:
   `git merge --ff-only worktree-cass-p3kgr-gen13`, then push `main` and
   `main:master`. **Commit 26932422 is worth landing on its own** — it is the
   one that makes rebuilds finish, and it does not depend on any of the above.
4. **Record the findings on bead `p3kgr`.** This session could not: `br` in a
   worktree fails with `Sync conflict … the authorized database is missing`
   (`.beads/beads.db` is session-local and absent there), and the main
   checkout's `.beads/issues.jsonl` was already dirty at baseline and cannot be
   committed from an isolated background job.

## Environment facts that cost real time

1. `rustup` is not on PATH in this harness's shell — `export
   PATH="$HOME/.cargo/bin:$PATH"` first. The default toolchain is rustc
   1.94.0-nightly, which **cannot build fsqlite 0.1.5's own dependency tree**
   (`sysinfo 0.39.6 requires rustc 1.95`). Use `cargo +nightly-2026-08-10` for
   probe crates. The repo itself builds fine on the default toolchain.
2. A worktree-isolated background session's Bash tool **refuses compound
   commands** containing redirects, `$(...)`, or a `cd` outside the worktree.
   Write scripts with the file tool and run them; do not fight it.
3. `setsid` and `gtimeout` do not exist here. Use `nohup … & disown`.
4. `frankensqlite` in this repo is `package = "fsqlite"` with
   `features = ["fts5"]` — a probe crate must match both or it is not the same
   engine.
5. `fsqlite::Connection::open` is read-write and **will modify the file**;
   prod.db's mtime moved under two probe cells. Open probes with
   `compat::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)` when the
   specimen matters.
6. Disk is under the janitor's 150 GB floor. `/tmp/fsq-probe-data` is ~31 GB.
   **Do not delete it without asking.**
7. A sibling session's `cass index --force-rebuild` (pid 38174, account
   `george`) was still grinding at 15h+. Check `claude agents` and `lsof` before
   assuming a lock is yours.

## State

- `main` is at `10575de2`, green on fsqlite 0.1.5, still the shipping pin.
  Do not merge `worktree-cass-gen5-honesty`.
- Open beads: `p3kgr` (P0), `759l7`, `9fnbr`, `qtn0e`.
- `indexed_docs=2334366` is 1,148 short of the 2,335,514 rows in `messages` —
  possibly the same population as `9fnbr`. Check, do not assume.
- Preserved artifacts: the generation-13 proven binary is copied to
  `~/.claude-accounts/katherine/jobs/c7c7626a/tmp/cass-guardfix-baseline`
  (`/tmp/cass-fix-target/release/cass` is being overwritten by the new build).
  The sample that first localised the hang is `/tmp/rerun-sample.txt`.
