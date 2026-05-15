## Symptom

`Fts5Table::snapshot_state` clones the entire in-memory `InvertedIndex` and `documents` HashMap on every `xBegin`, `xSavepoint`, and every per-statement vtab savepoint that `fsqlite_core` wraps around DML. For tables with non-trivial content, each clone is GB-scale, and the cost is paid per INSERT.

A consumer reproducer (Dale's `cass` tool indexing ~9,300 chat conversations into the `fts_messages` virtual table) reproduces this as:

- Peak RSS climbs to ~50 GB within ~5 minutes of indexing
- CPU pinned at 99–100% in `libsystem_memset` / `libsystem_memmove` inside `_xzm_free` → `<InvertedIndex>::drop_in_place`
- Forward progress stops entirely (DB row count frozen) while the indexer churns through ~42.9 million live allocations
- `lsof` on the stalled process shows zero open source files — the work is all happening in-memory inside the SQLite per-row savepoint wrap

Symbolised sample of the busy thread (from a `--profile profiling` build):

```
main → run_index_with_data
  → Connection::execute_prepared_with_params_after_background_status
    → execute_precompiled_prepared_insert_fast
      ├─ live_vtab_savepoint_all                                [2,822 samples]
      │  └─ Fts5Table::ErasedVtabInstance::savepoint
      │     └─ Fts5Table::snapshot_state                        [1,924 samples]
      │        └─ HashMap<SmallText, SmallVec<Posting>>::clone  [1,677 samples]
      │           └─ _platform_memmove                          [502 samples]
      └─ live_vtab_release_all                                  [776 samples]
         └─ <InvertedIndex>::drop_in_place                      [254+254+31]
            └─ _xzm_free → _platform_memset / __bzero           [115+35 samples]
```

## Root cause

`Fts5Table::snapshot_state` does an eager deep-clone of the index and documents map on every savepoint:

```rust
fn snapshot_state(&self) -> Fts5TableSnapshot {
    Fts5TableSnapshot {
        config: self.config,
        tokenizer_name: self.tokenizer_name.clone(),
        prefix_lengths: self.prefix_lengths.clone(),
        index: self.index.clone(),       // ← GB-scale for large indexes
        documents: self.documents.clone(), // ← also GB-scale
        next_rowid: self.next_rowid,
    }
}
```

`fsqlite_core::Connection::with_internal_statement_savepoint_and_cx` wraps every DML statement in a savepoint, so the clone fires per-INSERT. Cost is `O(state_size)` per INSERT; with `N` rows the total cost is `O(N²)`.

The stock SQLite FTS5 implementation doesn't pay this cost because the FTS5 index lives in shadow tables and the underlying SQLite journal/WAL handles rollback. `frankensqlite`'s FTS5 implementation is in-memory only, so it needs its own rollback mechanism — but the per-savepoint full snapshot was paying state-size cost when it only needed to pay deltas-since-savepoint cost.

## Fix

Replace the eager-clone snapshot with a **reverse-delta journal**:

1. `Fts5TableSavepoint` becomes a small marker (schema scalars + a `usize` position into the delta log). `snapshot_state` is O(1) — no index or documents clone.
2. A new `pending_deltas: Vec<RowDelta>` field on `Fts5Table` accumulates `Inserted` / `Deleted` / `Updated` entries during a transaction.
3. `restore_state` walks the log backwards from its current tail down to the saved position, replaying each delta's inverse via `reverse_delta`. A `silent_mutations` flag prevents the reverse mutations from recording themselves.
4. `commit` drops the log; `rollback` walks it fully back to the begin marker.
5. Mutations outside a transaction (the existing non-transactional bootstrap path used by `rebuild_documents` etc.) bypass the log entirely.

The full SQLite xSavepoint/xRelease/xRollback/xRollbackTo contract is preserved (the new tests below exercise each).

Cost change per INSERT inside a transaction:

|                  | Before                | After    |
|------------------|-----------------------|----------|
| `snapshot_state` | `O(state_size)`       | `O(1)`   |
| `restore_state`  | `O(state_size)`       | `O(deltas-since-savepoint)` |
| `release(n)`     | drops snapshot (free) | drops marker (free) |
| `commit`         | drops snapshot (free) | drops marker + clears log (`O(deltas)` free) |

For the consumer reproducer above, this means per-INSERT savepoint cost drops from ~30 GB allocate+memset to a single `Vec<RowDelta>::push`.

## Verification

- All 170 pre-existing `fsqlite-ext-fts5` tests still pass (`cargo test -p fsqlite-ext-fts5 --lib`).
- 9 new tests added covering the previously-uncovered savepoint/rollback paths:
  - `test_snapshot_state_does_not_clone_index_or_documents`
  - `test_begin_commit_inserts_persist_and_clear_delta_log`
  - `test_full_rollback_undoes_all_inserts`
  - `test_savepoint_release_keeps_inserts`
  - `test_rollback_to_inner_savepoint_keeps_outer_inserts`
  - `test_rollback_undoes_delete_of_pre_existing_row`
  - `test_rollback_undoes_update_of_pre_existing_row`
  - `test_mutations_outside_transaction_do_not_record_deltas`
  - `test_many_inserts_inside_transaction_grow_log_linearly`
- `cargo clippy -p fsqlite-ext-fts5 --lib --tests -- -D warnings` clean.
- Workspace test delta vs. main: **zero new failures** (verified by diffing the `FAILED` lists from `cargo test --workspace` on both branches — the 61 pre-existing `fsqlite-core` failures are unchanged).
- Downstream consumer (`cass` indexing the same pi-agent corpus that originally reproduced the stall) — pending after this PR merges and the rev pin is bumped.

## Side-finding (not addressed in this PR)

While reading the FTS5 vtab source, I noticed that `store_document_with_tokenizer` unconditionally inserts `column_values` into `self.documents` even when the table is declared with `ContentMode::Contentless`. The SQLite stock FTS5 contract is that contentless tables store ONLY the tokenized index, not raw column values. The current behavior costs roughly 2× memory for contentless tables (raw text held in both the source SQL table and the FTS5 documents map) and compounds the snapshot cost addressed in this PR.

Happy to file as a separate issue. Not fixing in this PR to keep the diff focused on the per-savepoint cost.
