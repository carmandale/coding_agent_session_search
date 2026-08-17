---
generation: 16
parent-session: c3b442f9-587e-4a1b-a004-6729bbcba01a
next-action-class: executable
---

# Generation 16 — the query hang is FIXED on the shipping pin. No engine bump was needed.

## The goal and authorization, verbatim

Dale, 2026-08-14:

> /my-way fix cass to completion and 100% green working state and completely up to date or tell me why it can't or /grill-me with any questions.

Dale, 2026-08-15:

> your usage is good now. finish this to completion

Dale, 2026-08-16:

> if your senior dev recommendation is to delete those 4 stale directories do it. if that is what progresses us toward the goal. and I would prefer you just do that and only stop when it would break the pipeline or running sessions or if there is a true blocker or ambiguity or conflict rather than sitting here for something that I am going to just ask your recommendation on and agree to

Dale, 2026-08-16:

> there has been SO MUCH WORK done on cass and it feels like you agents are all wondering around blind folded in a mine field in a lava flow. can I PLEASE have an agent like Opus 5 on high thinking....oh, that's what you are... not run around incircles playing whac-a-mole? what the heck?!

**Destructive and external-write approvals expired with the ending session and do
not transfer to you.** You may not file anything upstream on
frankensqlite/asupersync, delete any file, force-push, rewrite history, merge to
`main`, change repo visibility, or run `cass sources agents exclude` (that last
would destroy 3,877 conversations existing nowhere else). Dale's standing rule:
**never delete a file without express permission**, including files you created.

## THE FINDING — read this before anything else

**Generation 15's exact next action was "bump the storage engine to fsqlite
0.1.19". Do not do it. It is not necessary, and the hang is already fixed
without it.**

Generation 15 proved the hang is fsqlite 0.1.5 and concluded — correctly, from
the codegen source — that on that engine there is no seek path for
`MAX(<rowid alias>)` and none for `ORDER BY id DESC LIMIT 1` either. From that it
drew one more inference, and **that inference was wrong**:

> "Do not spend another generation on SQL rewrites at that call site. The
> planner citations above show why: on 0.1.5 there is no formulation of
> 'highest id' that is not a full scan."

The citations it quoted actually say the opposite, and it is visible in the
sentence itself: `Opcode::Last` "needs a WHERE clause". A **non-aggregate
statement carrying a WHERE clause on the rowid** reaches
`extract_rowid_range_target` and seeks. Nobody in four generations tested that
shape.

Measured on the 23 GB archive with a probe pinned to cass's exact engine — every
`fsqlite-*` crate and `asupersync` forced to this repo's lockfile versions,
verified in the lockfile AND by version markers in the built binary
(`~/.claude-accounts/katherine/jobs/c3b442f9/tmp/seekprobe-cheap.log`):

| statement | ms |
|---|---|
| `SELECT COUNT(*) FROM conversations` | 48 |
| `SELECT COUNT(*) FROM messages` | 818 |
| `SELECT 1 FROM messages WHERE id > 9000000000 LIMIT 1` | **0** |
| `SELECT 1 FROM messages WHERE id > ?1 LIMIT 1` (bound param) | **0** |
| `SELECT COUNT(*) FROM messages WHERE id > ?1` | **0** |

A full scan cannot answer "no row above 9e9" in 0 ms when counting the same
table costs 818 ms on the same connection. The range predicate genuinely seeks.

An independent review lane then confirmed the mechanism from the shipping
codegen rather than from the previous generation's notes: `Opcode::SeekGT` is
emitted at `fsqlite-vdbe-0.1.5/src/codegen.rs:2871`, a numbered placeholder is
accepted as a seek-safe bound (`is_rowid_range_constant` :14773-14785,
`rowid_range_bound_is_seek_safe` :3178-3195), and `LIMIT` is honoured inside that
loop via `DecrJumpZero → done_label` (:2940). **A bare `INTEGER PRIMARY KEY`
already has a seek path on 0.1.5; it just needs a WHERE bound to reach it.** The
"you need a real secondary index" reading was too pessimistic.

## What generation 16 landed

Branch `worktree-cass-p3kgr-gen13`. One commit, `src/indexer/mod.rs` only.

`lexical_fingerprint_max_id` computes the exact same `MAX(id)` by bisecting the
signed rowid domain with `WHERE id > ?1 LIMIT 1` existence probes — 66 probes,
constant regardless of table size.

Measured with the **shipped** algorithm (a first probe measured a galloping
variant that was never shipped; the doc comment carries the shipped numbers):

| | `COALESCE(MAX(id), 0)` | this bisection |
|---|---|---|
| `max(conversations.id)` (27,441 rows) | 1,075–3,578 ms | **65 ms cold, 7 ms warm** |
| `max(messages.id)` (2,335,514 rows) | **never returned in 45 minutes** | **7 ms** |

Both returned the true maximum, cross-checked against independently measured row
counts. The 2.3M-row table is the *faster* of the two, because the cost is 66
seeks rather than anything proportional to rows.

**Nothing else changed.** The fingerprint string `content-v1:{N}:{C}:{M}` is
byte-identical for identical data, so the comparison in
`src/search/asset_state.rs`, the checkpoint lifecycle, the writer, and every
persisted artifact are untouched. No rebuild or re-embed is forced by this
change.

Also fixed in the same commit: `max_conversation_id_exact`
(`src/indexer/mod.rs`) still carried the raw `SELECT COALESCE(MAX(id), 0) FROM
conversations` — the same defect on the rebuild/checkpoint path, missed by
`6e7b50c9`'s sweep. It now uses the same helper.

### Proof

- **Suite**: 5151/0/3 at the baseline commit → **5154/0/3** with the fix. Exactly
  +3, the three new tests. Zero regressions.
- **Mutants** (`~/.claude-accounts/katherine/jobs/c3b442f9/tmp/mutants.log`),
  which is the only honest check that a new test can fail:

  | mutant | tests that went red |
  |---|---|
  | return `lo` instead of the boundary | the 3 value tests — empty-table stays green, correctly |
  | bisect only the positive half (gallop from zero) | **only** `finds_negative_max_ids` |
  | drop the empty-table early return | **only** `is_zero_for_empty_tables` |

  The second row is the discriminating control: it proves the negative test
  measures the signed domain rather than duplicating the others.

## This also settles commit 6e7b50c9

Generation 15 left it open, to revert or to keep with a corrected doc comment.
Neither is needed: this commit **replaces** the statement it introduced, and its
false doc comment ("a descending-order seek" — false on this engine) goes with
it. No revert, no history churn, no force-push.

Its premise is now disproven twice over. Generation 15's own acceptance run,
`cass search` on the descending-seek binary, was killed at **45 minutes** with
`step=fingerprint_messages` never printing.

## The end-to-end acceptance was ATTEMPTED and is NOT achievable here — read why

The fix is landed and proven (see Proof above). What is **not** proven is the
last mile: the cass binary itself, on the real archive, through the healthy
search path. Two attempts were made and both failed to measure the changed code.
Neither failure impugns the fix; both are worth carrying so the next session does
not spend the same hours.

**Attempt 1 — `cass status --json` against the 23 GB archive. Wrong vehicle.**
Both binaries produced **zero bytes** and did not return in 240 s. Not one
`CASS_PREP_PROFILE` line printed, which is the tell: the block is upstream of
everything that profiles. A `sample` of the live process (3,395 samples, main
thread, fully on-CPU) named it exactly:

```
run_status -> collect_doctor_raw_mirror_backfill_report   (src/lib.rs:36383)
  -> fsqlite execute_join_select -> inline_subqueries_in_expr
    -> VdbeEngine -> BtCursor::advance_next_impl -> load_page
      -> ShardedPageCache::evict_any
        -> S3FifoEvictionTracker::build_model      2,496 of 3,395 samples
```

`cass status` **cannot reach the lexical fingerprint at all**, before or after
this fix, because doctor's backfill census sits in front of
`inspect_search_assets`. Now filed as **`-zumve` (P0)**: the statement at
`src/lib.rs:35903-35908` carries a correlated `(SELECT COUNT(*) FROM messages m
WHERE m.conversation_id = c.id)` per conversation, with no `LIMIT`, no `WHERE`,
and `query_map_collect` materialising every row. The `Duration::from_secs(1)` at
`:36419` bounds acquiring the connection, not the statement. The sibling `COUNT(*)`
probe was bounded under `-nvq59`; **this call was missed by that sweep, exactly as
`max_conversation_id_exact` was missed by `6e7b50c9`'s.** Same shape, third time.

**Attempt 2 — `cass search --mode lexical` against the same archive. Wrong
state.** The scratch data dir holds no lexical assets, so search logged
`searchable lexical metadata missing` and went straight into a full rebuild via
`step=prepare_db_state_deferred_fingerprint` — the **deferred** fingerprint
(`content-pending-v1:{N}`), which by design never computes the max ids. Both
binaries behaved identically because **neither one executes the changed code on
that path.** Reaching the hot path requires a data dir with *healthy* assets, and
building those on a 23 GB archive is the multi-hour, tens-of-GiB operation that
cannot be afforded here — free space is already 49 GiB against a 150 GiB floor.

**So the honest proof boundary is:** the changed algorithm is measured against
the real 23 GB archive by a probe pinned to cass's exact engine and transcribed
line-for-line from the shipped code (65 ms cold / 7 ms warm, against a statement
that never returned in 45 minutes); the change is measured at unit level by the
suite and by three mutants; the binaries are verified by two-sided markers. The
cass binary has **not** been observed answering a query on Dale's real archive.
Say it that way — do not round it up.

**What the next session should actually run.** The cheap end-to-end is `cass
status`, and it is blocked by `-zumve` — so **fix `-zumve` first and the
acceptance becomes a one-liner.** That is the shortest path, and it clears a P0
on the way. Do not try to build lexical assets for the 23 GB probe archive just
to run an acceptance.

Binaries, both preserved and two-sided-marker-verified — **verify by those
markers, never by timestamp**, that is a recorded incident in this repo:

- `~/.claude-accounts/katherine/jobs/c3b442f9/tmp/binaries/cass-AFTER-bisect`
  — carries `SELECT 1 FROM messages WHERE id > `, not the old statement.
  sha256 `01896686c543cb4f3e106e39aa48d52ac44ee8d394dbdd68da45691993cecef9`
- `.../binaries/cass-BEFORE-6e7b50c9` — carries `ORDER BY id DESC LIMIT 1`.
  sha256 `53047601ef7f035df9185021f14131f93b379d124ea11ed4102430d4b15da28c`

## Still open, and owed to Dale — none of it is agent-doable

1. **Nothing is on `main`.** Both `26932422` (the rebuild guard inversion,
   2h28m → 57s) and this session's fix are on `worktree-cass-p3kgr-gen13`,
   pushed. Background jobs cannot push `main`. Landing needs a session that can:
   `git merge --ff-only worktree-cass-p3kgr-gen13`, then push `main` and
   `main:master`. **`26932422` is worth landing on its own.**
2. **A sibling job has burned 16+ hours on the pre-fix binary** — pid 38174,
   account `george`, `cass index --force-rebuild` via `~/.local/bin/cass`, which
   predates `26932422`. Do not kill another session's job. Installing the fixed
   binary is the remedy.
3. **The fsqlite pin move is now optional, not required.** The parallel 759l7
   chain priced it in
   `thoughts/shared/handoffs/20260816-759l7-spin-wait-gen13/pin-move-cost.md`
   (8 test failures; 4 trivial, 2 re-adjudications, 1 production gate fix, 1
   upstream regression). With the hang fixed on the shipping pin, that move is
   now a quality decision on its own merits rather than the fix for a P0.
4. **Bead `-hd4u5`** (the `rootpage > 0` FTS gate) is a production behaviour
   change on the shipping pin that three independent sessions have judged Dale's
   call. It is not on this chain's critical path and was deliberately left alone.
5. **Two follow-ups this session found and did not act on**, both worth a bead:
   `src/search/model_manager.rs:390` computes the fingerprint a *second*
   independent time on every default search (`SearchMode::Hybrid` is `#[default]`)
   — ~40 ms of redundancy now rather than a second hang; and no test covers
   appending a message to an *existing* conversation, which is the one change
   that moves only the message half of the fingerprint.
6. **`-zumve` (P0, filed this session): `cass status` never returns on a large
   archive.** Doctor's raw-mirror backfill census runs an unbounded correlated
   `COUNT(*)` per conversation. Independent of the fingerprint defect — fixing
   either does not fix the other. Full diagnosis, sampled stack, and three
   candidate fixes are in the bead. **This is also what blocks the cheap
   acceptance above, so it is the highest-value next fix on this chain.**
7. **`-iekel` (filed this session): 13 GiB needs deleting and I may not delete
   it.** `~/.claude-accounts/katherine/jobs/c3b442f9/tmp/acceptance-data/index/`
   holds partial Tantivy shards from the two killed acceptance runs — incomplete
   by construction, nothing depends on them. They took free space from 63 GiB to
   49 GiB against a 150 GiB floor that disk-janitor had *already* reported
   breached the same day. Dale's standing rule forbids deleting a file without
   express permission, including files the agent created, so this was filed
   rather than done. The command is in the bead. **Do not extend it to
   `/tmp/fsq-probe-data/prod.db`** — that specimen is protected and is the only
   large archive available for measuring this class of defect.

## A correction to the other chain's handoff, worth carrying

`20260816-759l7-spin-wait-gen15/p3kgr-upstream-continuation-gen16.md` says
pinning `rust-toolchain.toml` "changes the compiler for **every other session and
worktree in this repo at once**". That is false as stated, and it is why the pin
move looked more blocked than it was: **each git worktree has its own working
copy of `rust-toolchain.toml`** — verified, all seven checked, each with its own
file. A toolchain pin on a branch in a worktree is isolated to that worktree
until it is merged. It only becomes repo-wide when it lands on `main`.

## Environment facts that cost real time

1. `rustup`/`cargo` are NOT on PATH here — `/opt/homebrew/bin` precedes
   `~/.cargo/bin`, so a bare `cargo` is Homebrew stable 1.96 and dies with
   `E0554` before a test runs. `export PATH="$HOME/.cargo/bin:$PATH"` first, then
   confirm `rustc --version` reads `1.94.0-nightly (f52090008 2025-12-10)`.
2. The worktree's own `./target` is warm — do NOT set `CARGO_TARGET_DIR` for
   tests. A full `cargo test --lib` is ~2 min compile + ~2.5 min tests.
3. `cargo check --lib` does NOT type-check `#[cfg(test)]` code. A green check
   says nothing about your new tests.
4. This session's Bash tool blocks bare `sleep N; cmd` chains — use
   `run_in_background` with an `until` loop. `setsid`/`gtimeout` do not exist;
   use `nohup … & disown`. dcg blocks any command containing `rm -rf` under a
   home path, including harmless ones.
5. Any probe crate compared against cass must pin the whole `fsqlite-*` tree AND
   `asupersync` with `=` requirements and verify the resolution before a single
   measurement is trusted. A working one with a resolution guard is at
   `~/.claude-accounts/katherine/jobs/c3b442f9/tmp/seekprobe` (build it with
   `cargo +nightly-2026-08-10`; the repo's default toolchain cannot build
   fsqlite's dependency tree).
6. Disk was ~63 GB free against the janitor's 150 GB floor, and is now **49 GiB**
   — this session's two killed acceptance runs wrote 13 GiB (bead `-iekel`).
   `/tmp/fsq-probe-data` is 29 GB, `/tmp/cass-fix-target` ~8 GB, `./target`
   9.6 GB. **Do not delete any of them without asking.** Before running anything
   against that archive, check `df -g /` and assume a rebuild will write tens of
   GiB.
7. **`os.walk` + `os.stat` follows symlinks.** Measuring "what did my run write"
   in a directory holding a symlink to a 23 GB specimen reported 34.67 GiB when
   the true answer was 12.96 GiB — the specimen counted as written bytes. Use
   `os.lstat` and skip links explicitly. Caught here before it reached a bead,
   but it is the `instrument-labels.md` shape and it would have overstated the
   damage by 2.7x.
8. **Kill an acceptance run the moment its measured step appears.** The second
   attempt let two full rebuilds run to a 240 s wall clock, and that wall clock —
   not the measurement — is what cost 13 GiB. Poll the log for the step you are
   timing and stop there.

## State

- Branch `worktree-cass-p3kgr-gen13`, pushed. Tree clean but for `.agent-state/`
  (session-local, ignored).
- `main` / `origin/main` at `5d1718a3`; the branch fast-forwards onto it cleanly.
- Open beads: `p3kgr` (P0), `759l7`, `9fnbr`, `qtn0e`, `hd4u5`, `xybl9`, and the
  three the 759l7 chain filed.
- Both p3kgr sibling sessions confirmed read-only before this session wrote
  anything, and both replied within minutes. That exchange is cheap and it
  works — run `ListAgents` and ask before you edit.
