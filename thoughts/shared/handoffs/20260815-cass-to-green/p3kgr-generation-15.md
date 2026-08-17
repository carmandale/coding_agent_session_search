---
generation: 15
parent-session: 4c84f454-678f-4b6a-8416-10a5fd846bb7
next-action-class: executable
---

# Generation 15 — the query hang is fsqlite 0.1.5, proven. The probe that "could not reproduce it" for three generations was running a different storage engine.

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
frankensqlite/asupersync, delete any file, force-push, rewrite history, change
repo visibility, or run `cass sources agents exclude` (that last would destroy
3,877 conversations existing nowhere else). Dale's standing rule: **never delete
a file without express permission**, including files you created.

## THE FINDING — read this before anything else

Generations 12, 13 and 14 each concluded that the `cass search` hang "is inside
the cass process and IS NOT ISOLATED", because a standalone probe running
"fsqlite 0.1.5 with the fts5 feature cass uses" answered the same statement on
the same 23 GB file in 0–56 ms while cass never returned.

**The probe was not running cass's storage engine.** `fsqlite` is a thin facade.
Both Cargo.lock files pin the facade at 0.1.5 — which is what every generation
checked, and it is why the difference stayed invisible — but the crates that
contain the planner, the code generator, the B-tree and the pager resolved
differently, because `fsqlite 0.1.5` requires them as `^0.1.5` and a fresh
resolution takes the newest 0.1.x:

| crate | cass Cargo.lock | probe Cargo.lock |
|---|---|---|
| `fsqlite` (facade) | 0.1.5 | 0.1.5 |
| `fsqlite-core` | **0.1.5** | **0.1.19** |
| `fsqlite-btree` | **0.1.5** | **0.1.19** |
| `fsqlite-pager` | **0.1.5** | **0.1.19** |
| `fsqlite-vdbe` | **0.1.5** | **0.1.19** |
| `fsqlite-planner` | **0.1.5** | **0.1.19** |
| `asupersync` | **0.3.2** | **0.3.10** |

**This is now proven in both directions, not inferred.** A probe built with
every `fsqlite-*` crate *and* `asupersync` forced to cass's exact versions — a
bare standalone binary with no cass process, no tantivy index, no tracing
subscriber and no asupersync runtime activity — reproduces the hang on the same
23 GB file (`~/.claude-accounts/katherine/jobs/4c84f454/tmp/pinned-probe3.log`,
crate at `.../tmp/fsq-probe-pinned2`, guard output in the same log):

| statement | fsqlite-core 0.1.5 (cass's pin) | fsqlite-core 0.1.19 |
|---|---|---|
| `SELECT COUNT(*) FROM conversations` | 34 ms | 34 ms |
| `SELECT COALESCE(MAX(id), 0) FROM conversations` | **1,075 ms** | **0 ms** |
| `SELECT COALESCE(MAX(id), 0) FROM messages` | **did not return in 240 s** | **0 ms** |

The 1,075 ms agrees with the 964 ms that cass itself measured for the same
table, so the standalone binary and the cass process are now behaving
identically. **The hang is fsqlite 0.1.5. It is not cass's code, not a pragma,
not the connection recipe, and not anything about the cass process.**

Reproduce in one command from each directory:

```bash
python3 -c "import re;d=dict(re.findall(r'\[\[package\]\]\nname = \"([^\"]+)\"\nversion = \"([^\"]+)\"',open('Cargo.lock').read()));print({k:v for k,v in d.items() if k.startswith('fsqlite')})"
```

So every "controlled and eliminated" row in the generation-14 commit message —
the pragmas, the open flags, the statement sequence, the `query_row_map` entry
point, the second connection — was measured against a control that was never the
specimen. This is the differential-specimen failure in
`~/.agent-config/.claude/rules/instrument-labels.md`, reached through a
transitive version rather than a mutated file.

## Why the statement is slow on 0.1.5 — settled from source, with citations

A source lane read fsqlite-core 0.1.5 and fsqlite-vdbe 0.1.5 directly. Paths
below are under
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`.

**There is no seek path for either statement on 0.1.5. Both are full
row-by-row scans, in every process, every time.**

- No fast path recognises `MAX(<rowid alias>)`. Positive control:
  `rg -c 'eq_ignore_ascii_case("count")' fsqlite-core-0.1.5/src/connection.rs`
  → 9; the same query for `"max"`/`"min"` → 0 matches.
- `fsqlite-vdbe-0.1.5/src/codegen.rs:1791-1796` — `is_aggregate` forces
  `rowid_target = None`, and `rowid_range` (1806), `index_range` (1815) and
  `index_eq` (1840) all follow. `SELECT MAX(id) FROM t` therefore emits
  `OpenRead` → `Rewind` → `AggStep` → `Next` → `AggFinal`. A full scan.
- `SELECT id FROM t ORDER BY id DESC LIMIT 1` emits `SorterOpen` → `OpenRead` →
  `Rewind` → `SorterInsert` → `Next` (codegen.rs:5048-5150). The `LIMIT` bounds
  the *sorter*, not the scan. **Also a full scan, plus a sorter.**
- `Opcode::Last` — the real one-row seek — is emitted only at codegen.rs:2865
  (needs a WHERE clause, `extract_rowid_range_target` opens `let expr =
  where_clause?;` at 14832) and codegen.rs:4570 (needs a real secondary index; a
  bare `id INTEGER PRIMARY KEY` has none).

That settles commit `6e7b50c9` on its own terms: its premise, that
`ORDER BY id DESC LIMIT 1` is "a descending-order seek", is **false on this
engine**. Its measured 3,578 → 964 ms was one scan shape beating another, not a
seek. See "What to do about 6e7b50c9" below.

A 20-second `sample` of the live hang (pid 74465), symbolicated against an
unstripped rebuild, shows the cost is malloc/free churn inside the pager —
`ShardedPageCache::prefetch_page_hint`, `PageCacheEvictionTracker::queue_snapshot`,
`PageBufPool::acquire`, `pager::lane_flush_stats` (`sort_unstable_by_key`),
`ConflictMetrics::top_hotspots` (`sort_by_key`), and `reserve_rehash` on a
`DashMap<PageNumber, PageData>`. ~2,100 of 15,409 samples are in `free`, ~490 in
`malloc`. The `asupersync-worker-0` thread is blocked in `__psynch_cvwait` for
all 15,409 samples — it is idle, not contending. Treat the *symbol set* as
evidence and the *call chain* as suggestive only: the binary has LTO with
`codegen-units = 1` and no debug info, so `atos` maps each address to the
nearest preceding symbol and inlined frames are misattributed.

Artifacts: `~/.claude-accounts/katherine/jobs/4c84f454/tmp/hang-sample-gen14.txt`,
and the binary that produced it preserved at `.../tmp/cass-sampled-stripped`.

## What is DONE

1. **Defect 1 (the rebuild guard inversion) is fixed, tested, mutant-verified and
   committed** — `26932422`, inherited from generation 13, unchanged by this
   session. 2h28m → 57s. This is the one that makes rebuilds finish.
2. **Bead `p3kgr` carries generation 14** — `e54af39d`, written by the parent
   session after its handoff was composed. `br` works in this worktree once
   `.beads/beads.db` is copied in from the main checkout.
3. **Five more hypotheses falsified**, each measured on the 23 GB probe archive
   (`~/.claude-accounts/katherine/jobs/4c84f454/tmp/probe-cells.log`,
   `.../ab-witness-cap.log`, `.../trace-probe.log`) — though note every one of
   these ran on the 0.1.19 engine, so they are evidence about 0.1.19, not 0.1.5:
   `FSQLITE_READ_WITNESS_CAP=16384` (cass sets it at `src/main.rs:207-222`; no
   effect); `PRAGMA fsqlite.concurrent_mode = ON`; a read-write connection with
   cass's full `apply_config()` held open first; an explicit `BEGIN` (the
   `had_owned_txn` branch at fsqlite-core connection.rs:5566); two concurrent
   connections; an installed `tracing` subscriber at three filter levels.
4. **Cargo feature unification ruled out** — every `fsqlite-*` crate resolves to
   an identical feature set in both builds except a cosmetic `default` vs
   `native` on `fsqlite-types` (`default = ["native"]`, so the same code).
   `~/.claude-accounts/katherine/jobs/4c84f454/tmp/feature-diff.sh`.
5. **The design defect is named precisely.** `search_lexical_self_heal_diagnosis`
   (`src/lib.rs:19317`) fingerprints the whole canonical database to validate the
   published index, and `ensure_lexical_assets_for_search` calls it **once** on
   the healthy path (`src/lib.rs:19521`) — the other four call sites (19489,
   19512, 19531, 19581) are alternative degraded branches, not a 5x multiplier.
   The fingerprint is `COUNT(*) FROM conversations` plus two full-table
   aggregates (`src/indexer/mod.rs:7990-8030`), i.e. one unbounded scan of the
   archive per query.
6. **An unstripped cass binary exists** at `/tmp/cass-fix-target/release/cass`
   (66 MB, 99,300 symbols), built with
   `cargo build --release --config profile.release.strip=false`. Code identical
   to the shipping build; only `strip` differs.

## The exact next action

**Bump the storage engine and verify `cass search` completes.** The diagnosis is
finished; this is the fix that follows from it, and it is the first candidate in
four generations that addresses the cause rather than the statement.

Two earlier probe attempts failed and are instructive if you need to rebuild the
control: sequential
`cargo update -p <c> --precise 0.1.5` did not hold the tree down
(`pinned-probe.log`), and pinning fsqlite alone fails to compile because
`fsqlite-types 0.1.5` does not build against `asupersync 0.3.10`
(`NativeCx::scope_with_budget`, E0599) — which is exactly the alignment the
repo's own `Cargo.toml:217-219` warns about.

The bump itself:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cd /Users/dalecarman/dev/coding_agent_session_search/.claude/worktrees/cass-p3kgr-gen13
CARGO_TARGET_DIR=/tmp/cass-bump-target cargo update -p fsqlite --precise 0.1.19
CARGO_TARGET_DIR=/tmp/cass-bump-target cargo build --release
```

Expect friction and treat it as real work, not a blocker: `asupersync` will want
to move off 0.3.2, `build.rs` validates dependency source contracts, and
`.github/workflows/fresh-clone-build.yml` fails if a `[patch]` block pointing at
sibling paths is ever committed. Do NOT commit a `[patch]` block. If the bump
compiles, the acceptance test is a `cass search` against
`/tmp/fsq-probe-data/prod.db` **after** an index is published, with
`CASS_PREP_PROFILE=1`, reading `step=fingerprint_messages`.

**Do not spend another generation on SQL rewrites at that call site.** The
planner citations above show why: on 0.1.5 there is no formulation of "highest
id" that is not a full scan.

## What to do about commit 6e7b50c9

Unresolved, deliberately — it now depends on the probe result, and this session
did not want to churn history on a guess.

Its own commit message states the criterion: *"If it does not return, this shape
is not the answer and the change should be reverted rather than kept."* It did
not return. The counter-argument is that it measured a real 3,578 → 964 ms with a
proven-identical result and no test debt.

Recommendation: **if the engine bump lands, revert it** — `MAX(id)` becomes fine
and the descending-seek form is then unexplained complexity carrying an
obsolete doc comment. **If the bump does not land, keep the code but fix the doc
comment at `src/indexer/mod.rs:7969-7988`**, which currently calls the statement
"a descending-order seek" — false on this engine per the codegen citations
above — and implies the rewrite solved the hang. Rewriting the commit *message*
would require a force-push, which is not authorized.

## Still open, and owed to Dale

- **`26932422` is not on `main`.** Everything is on branch
  `worktree-cass-p3kgr-gen13`, pushed. Background jobs cannot push main/master
  in this harness, so landing it needs a session that can:
  `git merge --ff-only worktree-cass-p3kgr-gen13`, then push `main` and
  `main:master`. **`26932422` is worth landing on its own** — it is the rebuild
  fix and depends on none of the above.
- **A sibling session is burning 16+ hours on the pre-fix binary.** pid 38174,
  account `george`: `cass index --force-rebuild` against the real archive using
  `~/.local/bin/cass`, which predates `26932422`. At last check 15h54m elapsed,
  931 CPU-minutes. That is the guard-inversion defect — a whole-archive
  `GROUP BY` on every healthy run. With the fix the same refresh measured 57 s.
  Do not kill another session's job; tell Dale, and note that installing the
  fixed binary is the remedy.
- Filing the two upstream defects on frankensqlite needs Dale's explicit
  approval, which did not transfer. They are: (a) `MAX(rowid alias)` and
  `ORDER BY rowid DESC LIMIT 1` both compile to full scans on 0.1.5 with no
  `Opcode::Last` path reachable; (b) per-page pager bookkeeping makes each
  scanned page cost ~35 µs. Both appear fixed in 0.1.19.

## State

- Branch `worktree-cass-p3kgr-gen13` at `e54af39d`, pushed, clean but for
  `.agent-state/` (session-local, ignored).
- `main` / `origin/main` at `5d1718a3`; `10575de2` is an ancestor, so the branch
  fast-forwards onto it cleanly.
- Open beads: `p3kgr` (P0), `759l7`, `9fnbr`, `qtn0e`.
- Disk was ~63-70 GB free, under the janitor's 150 GB floor.
  `/tmp/fsq-probe-data` is 29 GB and `/tmp/cass-fix-target` 5 GB. **Do not delete
  either without asking.**
- Long-running processes left deliberately: pid 74465 (the original hang, 30+
  min, kept as the parent session asked), pid 75534 (a rebuild that had been
  silent 10 min at 74% CPU — check whether it ever finished;
  `~/.claude-accounts/katherine/jobs/4c84f454/tmp/acceptance.log` will say).

## Environment facts that cost real time

1. `rustup` is not on PATH here — `export PATH="$HOME/.cargo/bin:$PATH"` first.
   The default toolchain cannot build fsqlite's dependency tree; use
   `cargo +nightly-2026-08-10` for probe crates. The repo itself builds on the
   default toolchain.
2. A worktree-isolated background session's Bash tool refuses compound commands
   with redirects, `$(...)`, or a `cd` outside the worktree. Write scripts with
   the file tool and run them.
3. `setsid` and `gtimeout` do not exist here. Use `nohup … & disown`.
4. `fsqlite::Connection::open` is read-write and **will modify the file**. Open
   probes with
   `compat::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)` when the
   specimen matters. prod.db's mtime moved twice under probe cells this session.
5. The release profile sets `strip = true`, so `sample` output is unsymbolicated.
   `--config profile.release.strip=false` is enough for names; you do not need
   the `profiling` profile's full debug info.
6. **Any probe crate you build to compare against cass must pin the whole
   `fsqlite-*` and `asupersync` tree, and must verify the resolution before
   trusting a single measurement.** That is the entire lesson of this generation.
