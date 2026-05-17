---
title: "Upstream reconciliation map"
date: 2026-05-17T01:29:00Z
bead: coding_agent_session_search-1vxuf
---

# Upstream Reconciliation Map

This is a read-only map of recovery-owned behavior versus fetched `upstream/main` (`e337b9f428e12ea5a0d5b37129d3abb0dea48ab8`). It does not merge or mutate the checkout.

## Summary

The current recovery proof depends on local changes that are not present in upstream. Upstream has moved forward in related areas, but it also deletes two files that this recovery modified. A safe final merge needs deliberate file-by-file reconciliation, not an automatic merge.

## Pattern Presence

| Recovery behavior / symbol | Local recovery tree | Fetched upstream/main | Reconciliation note |
| --- | --- | --- | --- |
| `scratch_root` module | Present in `src/indexer/scratch_root.rs`; imported by `src/indexer/mod.rs`; tested by `tests/spec_015_streaming_watch_once.rs` | Absent; upstream deletes `src/indexer/scratch_root.rs` and `tests/spec_015_streaming_watch_once.rs` | High risk. Local Pi shadow proof used this path. Decide whether to preserve/adapt the helper or replace it with upstream's new large-scan shape. |
| DB-first Pi watch-once (`pi_watch_once_defers_inline_lexical`) | Present | Absent | High risk. This is part of the recovery route that let Pi ingest complete without repeatedly rebuilding lexical assets. |
| `watch_once_streaming_scan_start` trace | Present | Absent | Evidence/diagnostics would be lost unless ported or superseded. |
| Checkpoint refresh shortcut (`CASS_LEXICAL_CHECKPOINT_REFRESH_EXACT_DOC_SCAN_LIMIT`, `expected_lexical_indexed_docs_for_checkpoint_refresh`) | Present | Absent | High risk. This prevented search from triggering huge rebuilds after the shadow lexical index was already usable. |
| Zero-byte WAL parking (`park_zero_byte_wal_before_franken_open`) | Present | Absent | Needed until the durable frankensqlite WAL fix is pinned and installed. |
| `conversation_external_tail_lookup` | Present | Present | Lower risk. Upstream already has this table/migration surface, though local line numbers differ. |
| Lookup trace (`lookups_against_global`) | Present | Present | Lower risk. Upstream already exposes this trace field. |
| frankensqlite dependency pin | Local committed dependency remains old `eba969...` plus local `[patch]` to `../spec014-frankensqlite-fix` | Upstream pins `c8ce64fdce4cd2e3657d56d72719c7a3d99f39c3` | Must be resolved after the sibling fix is committed/pushed. Do not ship a local path patch as final. |

## High-Risk Files

These files are both dirty locally and changed upstream:

```text
M  .beads/issues.jsonl
M  .beads/last-touched
M  Cargo.lock
M  Cargo.toml
M  src/indexer/mod.rs
D  src/indexer/scratch_root.rs
M  src/lib.rs
M  src/storage/sqlite.rs
D  tests/spec_015_streaming_watch_once.rs
```

The `D` entries are especially important: upstream removes files that local recovery still compiles and tests.

Refresh on 2026-05-17T00:25:43Z:

- `upstream/main` advanced from `37b42058312d4aafa4a45ede8ae81ff5b8a07134` to `956f1d3baf2881e792b5d3397d1875789476f587`.
- The overlap set stayed the same.
- The recovery-critical symbol presence stayed the same: upstream still lacks `scratch_root`, `pi_watch_once_defers_inline_lexical`, `watch_once_streaming_scan_start`, `expected_lexical_indexed_docs_for_checkpoint_refresh`, and `park_zero_byte_wal_before_franken_open`; upstream still has `conversation_external_tail_lookup`, `lookups_against_global`, and frankensqlite rev `c8ce64fdce4cd2e3657d56d72719c7a3d99f39c3`.

Refresh on 2026-05-17T01:29:00Z:

- `upstream/main` advanced from `956f1d3baf2881e792b5d3397d1875789476f587` to `e337b9f428e12ea5a0d5b37129d3abb0dea48ab8`.
- The overlap set stayed the same.
- The recovery-critical symbol presence stayed the same: upstream still lacks `scratch_root`, `pi_watch_once_defers_inline_lexical`, `watch_once_streaming_scan_start`, `expected_lexical_indexed_docs_for_checkpoint_refresh`, and `park_zero_byte_wal_before_franken_open`; upstream still has `conversation_external_tail_lookup`, `lookups_against_global`, and frankensqlite rev `c8ce64fdce4cd2e3657d56d72719c7a3d99f39c3`.

## Recommended Merge Order After Approval

1. Make the live recovery proof first, while the currently tested local tree is still intact.
2. Make the frankensqlite fix durable and pin CASS to that durable revision.
3. Commit the verified recovery slice on the authorized branch, or explicitly stash/patch it in a non-destructive way approved by Dale.
4. Merge `upstream/main`.
5. Resolve these behaviors explicitly:
   - Pi watch-once must still ingest the full Pi root without OOM/stall.
   - Search must still trust/repair the completed lexical checkpoint without a giant rebuild.
   - Zero-byte WAL and derived FTS repair paths must either remain in CASS or be proven unnecessary by the durable frankensqlite pin.
   - The CASS dependency pin must not remain a local `[patch]` path.
6. Re-run the shadow/live canaries and compiler/focused tests.

## Why This Matters

The prior clean merge-tree result is useful but insufficient: it used committed `HEAD`, not the current uncommitted recovery work. The working-tree overlap shows the final upstream sync has real reconciliation work even though the committed histories can synthesize a merge tree.
