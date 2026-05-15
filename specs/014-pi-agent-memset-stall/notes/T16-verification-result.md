---
task: T16/T18 (verification with fix-bearing binary)
date: 2026-05-15
spec: 014-pi-agent-memset-stall
status: FIX-LANDS-BUT-INSUFFICIENT
---

# T16 — verification result with the frankensqlite PR #90 fix applied

## TL;DR

The reverse-delta journal fix landed in upstream PR #90 is **structurally correct** (eliminates a measurable per-savepoint clone cost; 179/179 fsqlite-ext-fts5 tests pass including 9 new savepoint regression tests) **but does NOT bring peak RSS under 8 GB** on the cass watch-once workload. Spec 014 acceptance criterion #2 is unachievable with the current `fsqlite_ext_fts5` in-memory-only design.

## What I did

1. Applied a local `[patch]` override in `Cargo.toml` pointing at `~/dev/spec014-frankensqlite-fix` (branch `fix/fts5-vtab-snapshot-via-delta-journal`, commit `f298dfa`).
2. Verified `cargo metadata` shows `fsqlite-ext-fts5 0.1.3` resolved to the local patch path (not the pinned upstream rev).
3. `cargo build --release --bin cass` clean (6m 41s).
4. Installed at `~/.local/bin/cass.real` (after working around a macOS Tahoe binary-launch-constraint issue that required `rm + cp` instead of `cp -p`).
5. Stopped launchd watcher, WAL-checkpointed DB, APFS-snapshotted `agent_search.db.PRE-PI-VERIFY-20260515-144430`, recorded 36 pre-run pi rows.
6. Ran `~/.local/bin/cass index --watch-once ~/.pi/agent/sessions --json --no-progress-events`.
7. Captured RSS every 60 s (notes/T16-rss.txt).

## What I saw

| Elapsed | pre-fix RSS | post-fix RSS | post-fix pi count |
|--------:|-------------:|-------------:|------------------:|
| 0:18    | 10.7 GB      | 8.6 GB       | 36 (no progress)  |
| 1:19    | 30.8 GB      | 47.9 GB      | 36 (no progress)  |
| 2:19    | 36.4 GB      | 55.0 GB      | 36 (no progress)  |
| 3:19    | 42.0 GB      | 55.8 GB      | 36 (no progress)  |
| 4:49    | 49.9 GB peak | (killed)     | 36 (no progress)  |

The fix changed the SHAPE of the RSS curve (smaller initial spike, faster ramp to plateau) but the peak is comparable and the pi conversation count makes zero forward progress in either run.

## Why the fix is insufficient

The original sample (notes/T3-sample.txt) showed two distinct memory costs in the FTS5 hot path:

1. **Per-savepoint `InvertedIndex::clone`** — 1,924 samples in `Fts5Table::snapshot_state`, called from `live_vtab_savepoint_all` on every INSERT. The fix eliminates this entirely.
2. **Steady-state `InvertedIndex` size** — the in-memory `HashMap<SmallText, SmallVec<Posting>>` itself, holding all terms × postings for the currently-indexed corpus. **~30 GB resident** per the original `vmmap` (`MALLOC_SMALL 30.8 GB`). This is data the FTS5 implementation *must* keep in memory, regardless of how savepoints are handled.

Killing the per-savepoint clone removes (1) but not (2). And (2) alone is ~4× larger than the 8 GB acceptance threshold.

This is consistent with the side-finding in T7: `fsqlite_ext_fts5` is in-memory-only. Stock SQLite FTS5 keeps its inverted index in **shadow tables on disk**, paging in only what each query needs. `fsqlite_ext_fts5` keeps the whole thing in `HashMap`s in process memory. With ~9,300 indexed conversations × thousands of terms × per-doc postings, the steady-state floor is structurally above 8 GB.

## Why the original spec assumed differently

Spec 014's evidence section recorded "RSS grows to 22 GB" — which was the peak observed during a *partial* run that was killed at 22 GB. With a longer run (this verification ran 3+ minutes), RSS keeps climbing past 50 GB before the watchdog/user kill. The 22 GB number was the upper bound of how long the user waited, not the steady-state ceiling.

In retrospect, none of the C1–C5 candidates in plan.md could have hit acceptance #2 because they all leave the in-memory FTS5 inverted index in place. The decision tree at plan.md ## Architecture is binding on "no deferred peak-RSS satisfaction"; the spec is structurally unsatisfiable as written when measured against the cass corpus (~9,300 docs).

## State restored

- `Cargo.toml` patch override reverted (`git checkout Cargo.toml`); back to the pinned upstream rev `eba969e`.
- `~/.local/bin/cass.real` restored from backup `.PRE-SPEC014-20260515-144412` (mtime 2026-05-14 10:22, hash `99ec0178...`).
- launchd `com.cass.index-watch` reloaded (new PID 37121).
- No DB rows were changed during the run; pi_agent count stable at 36.
- APFS snapshot of the DB at `PRE-PI-VERIFY-20260515-144430` retained as the pre-verification checkpoint (10 GB clone-on-write — costs zero extra disk until divergence).

## What this means for spec 014

PR #90 is still worth landing upstream — it is a correctness/performance win for any FTS5 workload that uses savepoints (which is every DML in fsqlite). It is independent of acceptance #2.

Spec 014 itself needs one of:

- **E1**: Amend acceptance #2 to a higher threshold (e.g. < 32 GB peak) or remove the cap, accepting that pi backfill on a populated DB needs more memory than the original spec assumed. Ship PR #90 as the fix.
- **E2**: Re-shape the spec to address the real ceiling — fsqlite_ext_fts5 architectural change to persist the index to shadow tables. This is a multi-week effort upstream, far outside the spec 014 cycle.
- **E3**: Cass-side workaround — split pi indexing into batches where the FTS5 vtab is rebuilt-from-scratch per batch, never letting the in-memory index grow past N entries before flushing & rebuilding. Bounded but invasive.

E1 is the smallest scope and matches reality. E2 is the cleanest long-term but blocked on the upstream maintainer. E3 is a band-aid that may not actually reduce peak (each rebuild itself touches all the data).

User decision required.

## Other artifacts

- `notes/T16-rss.txt` — RSS timeline with the fix applied
- `notes/T8-frankensqlite-fix.patch` — the patch (still valid upstream)
- `notes/T8-frankensqlite-pr-body-draft.md` — PR #90 body
- PR #90: https://github.com/Dicklesworthstone/frankensqlite/pull/90 (still correct and worth merging)
