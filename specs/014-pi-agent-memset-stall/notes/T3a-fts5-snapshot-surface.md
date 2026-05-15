---
task: T007 (Scout — fsqlite_ext_fts5 surface mapping)
date: 2026-05-15
spec: 014-pi-agent-memset-stall
scope: read-only research; no source edits, no upstream issue/PR creation
---

# T007 — FTS5 snapshot surface, savepoint contract, and D1/D2/D3 cost estimates

## Code references (all paths resolved to the pinned rev `eba969e`)

Pinned in `Cargo.toml:45`:
```
frankensqlite = { …, rev = "eba969ec45d102071b90519d3b819ddbcecf3d61",
                  package = "fsqlite", features = ["fts5"] }
```

Checkout: `~/.cargo/git/checkouts/frankensqlite-8b961831d226b905/eba969e/`

| Concern | File | Line(s) |
|---|---|---|
| `Fts5Table` struct + fields (`index: InvertedIndex`, `documents: HashMap<i64, Vec<String>>`, `txn_state: TransactionalVtabState<Fts5TableSnapshot>`) | `crates/fsqlite-ext-fts5/src/lib.rs` | 2102–2120 |
| `Fts5Table::snapshot_state` — **the hot path** | `crates/fsqlite-ext-fts5/src/lib.rs` | 2143–2151 |
| `Fts5Table::restore_state` (rollback target) | `crates/fsqlite-ext-fts5/src/lib.rs` | 2153–2160 |
| `Fts5Table::begin` (xBegin) — calls `txn_state.begin(self.snapshot_state())` | `crates/fsqlite-ext-fts5/src/lib.rs` | 2529–2532 |
| `Fts5Table::savepoint` (xSavepoint) — calls `txn_state.savepoint(n, self.snapshot_state())` | `crates/fsqlite-ext-fts5/src/lib.rs` | 2617–2620 |
| `Fts5Table::release` (xRelease) | `crates/fsqlite-ext-fts5/src/lib.rs` | 2622–2625 |
| `Fts5Table::rollback` / `rollback_to` | `crates/fsqlite-ext-fts5/src/lib.rs` | 2610–2632 |
| `TransactionalVtabState<S>` definition + savepoint/release semantics (gated on `base_snapshot.is_some()`) | `crates/fsqlite-func/src/vtab.rs` | 155–228 |
| `Connection::live_vtab_savepoint_all` — fires xSavepoint on every live vtab | `crates/fsqlite-core/src/connection.rs` | 9998–10115 |
| `Connection::live_vtab_release_all` | `crates/fsqlite-core/src/connection.rs` | 10117–10132 |
| `with_internal_statement_savepoint_and_cx` — wraps every DML in savepoint+release | `crates/fsqlite-core/src/connection.rs` | 20618–20800 |
| `execute_precompiled_prepared_insert_fast` — the per-row INSERT path the watch-once ingest takes | `crates/fsqlite-core/src/connection.rs` | 16977+ |
| Cass FTS5 vtab schema (`fts_messages` virtual table, `content = ''`, `tokenize = 'porter'`) | `src/storage/sqlite.rs` | 1085 |
| Cass INSERTs into `fts_messages` (many) | `src/storage/sqlite.rs`, `src/search/query.rs`, `src/lib.rs:60738` | numerous |
| Cass `defer_lexical_updates` (DEFERS **Tantivy**, NOT FTS5 vtab) | `src/indexer/mod.rs` | 18066–18070; gates at 18610–18632, 19120+; `src/storage/sqlite.rs:8431+` |
| Cass `defer_storage_lexical_updates_enabled` | `src/storage/sqlite.rs` | 8431, 8681 |

## How the stall actually happens

1. Cass writes a message row via `INSERT INTO messages …` and then `INSERT INTO fts_messages(rowid, content, …)`.
2. SQLite wraps each statement in a savepoint: `with_internal_statement_savepoint_and_cx` (`connection.rs:20618+`) opens an internal savepoint, runs the body, releases.
3. The savepoint open path calls `live_vtab_savepoint_all(level)` (`connection.rs:20677`). That iterates every registered live vtab (here: `fts_messages` plus any other live vtab cass attaches) and calls `instance.savepoint(level)`.
4. `Fts5Table::savepoint` (`lib.rs:2617`) calls `self.txn_state.savepoint(n, self.snapshot_state())`.
5. `snapshot_state` (`lib.rs:2143–2151`) clones **everything that matters**:
   ```rust
   fn snapshot_state(&self) -> Fts5TableSnapshot {
       Fts5TableSnapshot {
           config: self.config,
           tokenizer_name: self.tokenizer_name.clone(),
           prefix_lengths: self.prefix_lengths.clone(),
           index: self.index.clone(),            // ← O(terms × postings) — the GB clone
           documents: self.documents.clone(),    // ← O(total stored text) — see "contentless lies" below
           next_rowid: self.next_rowid,
       }
   }
   ```
6. On success the statement's release fires `live_vtab_release_all(level)`, which drops the cloned snapshot. The `drop_in_place::<InvertedIndex>` walks every key and Posting and calls `_xzm_free` (per-entry malloc), which calls `_platform_memset`/`__bzero` to zero freed memory. With ~42.9 million allocations (per `vmmap`), the free path is the dominant CPU consumer at sample time.

Net: every INSERT to `fts_messages` clones, then immediately drops, the entire in-memory FTS5 state. Cost is O(state_size) per INSERT, and state_size grows linearly with conversations indexed.

## Two structural problems, not one

### Problem A (snapshot/restore for INSERT-only)

For SQLite's official xSavepoint/xRelease/xRollback contract, the snapshot is only needed if rollback is possible. The cass watch-once ingest is **transactionally insert-only**: the only rollback paths are catastrophic error returns (e.g., OOM hitting `error_is_out_of_memory()` in `src/indexer/mod.rs:15579+`), not user-driven SAVEPOINT/ROLLBACK TO. So the snapshot work is paid every insert; the snapshot is essentially never used.

A correct FTS5 vtab could implement xSavepoint/xRelease/xRollback with copy-on-write or a journal-of-deltas: record the per-row deltas at the savepoint depth, replay-inverse them on rollback. Cost would be O(rows-touched-since-savepoint), not O(entire-state).

### Problem B ("contentless" tables are not actually content-free in this FTS5 implementation)

The cass schema declares `fts_messages` as contentless: `CREATE VIRTUAL TABLE … USING fts5(…, content = '', tokenize = 'porter')`. In SQLite's stock FTS5, `content = ''` means the table does not store column values; only the inverted index is kept.

In **this** fsqlite_ext_fts5 implementation, however, `store_document_with_tokenizer` unconditionally stuffs `column_values` into `self.documents` (`lib.rs:2185`), regardless of `ContentMode`. The cass message content is therefore stored **three times** in memory: in the regular `messages` table cache, in the FTS5 `documents` HashMap, and tokenized into the `InvertedIndex`. This is what the vmmap "30.8 GB MALLOC_SMALL" plus "19.8 GB MALLOC_SMALL (empty)" reflects — and it makes the snapshot clone enormously bigger than a fixed FTS5 would.

## What `CASS_DEFER_LEXICAL_UPDATES` actually defers

It defers cass's **Tantivy** index population (see `src/indexer/mod.rs:18610–18623` for the watch path and `src/search/query.rs` for the deferred rebuild). It does **not** skip the SQL `INSERT INTO fts_messages` statements — those still fire per message, still wrap in vtab savepoints, still clone the FTS5 in-memory state. So setting `CASS_DEFER_LEXICAL_UPDATES=1` would not by itself avoid this stall.

D3 (cass-side defer of the FTS5 vtab itself) would need a different deferral knob, deferring the `INSERT INTO fts_messages` calls until the end of the batch and either issuing them then OR using the maintenance path at `src/storage/sqlite.rs:9459+` that already supports bulk repopulation. That maintenance INSERT path is the one used at backfill time today; the question is whether it ALSO triggers the same savepoint clones (per-row vs per-batch) — almost certainly yes, because the wrapping `with_internal_statement_savepoint_and_cx` is per-statement, not per-batch.

## Cost estimates per D-path

### D1 — Upstream `frankensqlite_ext_fts5` fix

**Surface**: `crates/fsqlite-ext-fts5/src/lib.rs` lines 2143–2160, 2617–2632; possibly `crates/fsqlite-func/src/vtab.rs` 155–228 if `TransactionalVtabState` grows a "deltas" variant.

**Fix shapes:**
- (a) Copy-on-write: change `index: InvertedIndex` to `index: Arc<InvertedIndex>`; on first mutation after begin/savepoint, clone-and-replace; on rollback, restore the cached Arc. Savepoint is O(1) clone of the Arc instead of O(state).
- (b) Delta journal: replace `TransactionalVtabState<Fts5TableSnapshot>` with `TransactionalVtabState<Vec<RowDelta>>`. Each insert pushes a `RowDelta::Insert(rowid, columns)`; release at level drops the deltas; rollback replays inverse. Savepoint is O(1) push, release is O(deltas).
- (c) Fix Problem B first: stop storing column_values into `self.documents` when `ContentMode::Contentless`. This shrinks the snapshot by the size of the raw text but does NOT reduce the InvertedIndex clone cost — still O(state) per insert. (Necessary but insufficient for acceptance #2.)

**Code volume**: 30–150 LOC depending on which shape. (a) is smallest; (b) is largest but most efficient long-term.

**Coordination**: upstream PR on `Dicklesworthstone/frankensqlite`, review, merge, release; then bump `Cargo.toml:45` rev. Mirrors PR #233 shape but in a different repo.

**Acceptance reach**: hits #1, #2, #4 cleanly. #3 (chunk-size preserved) unaffected. #5 (message coverage) unaffected.

**Spec 014 implications**: explicit scope change. spec.md "Constraint" section says "Stay within `src/indexer/`, `src/persist.rs`, or the franken-agent-detection pi connector glue; do not patch external `franken-agent-detection` directly from this repo." This rule was written about `franken-agent-detection`, not `frankensqlite`, but the spirit is the same — fix is in an external crate. Cycle must widen, or the spec must be amended to allow external-crate work, or the work moves to a sibling spec.

### D2 — Cass-side vtab savepoint knob

**Surface**: would require adding API to `fsqlite_ext_fts5` and `fsqlite_core` to expose a "skip vtab xSavepoint for vtab X during a connection" knob, then cass sets it on the watch-once ingest connection. Essentially D1 fold-in.

**Code volume**: very small in cass (1 line: set the knob), but the knob has to exist in `frankensqlite` first. Same external-crate coordination cost as D1.

**Acceptance reach**: hits #1, #2, #4 *if* the knob is correctly designed. Risk: silently disabling savepoint on a table that another path expects to be rollback-safe can corrupt the FTS5 ↔ messages consistency on a partial failure.

**Spec 014 implications**: same scope-change implication as D1.

### D3 — Pure cass-side defer + bulk maintenance

**Surface**: cass-side only. Probable shape:
1. Add a new flag (e.g., `CASS_DEFER_FTS5_INSERT`) that bypasses `INSERT INTO fts_messages` in the per-row indexer path.
2. After the batch (or at the end of `--watch-once` watchdog), run the existing maintenance path at `src/storage/sqlite.rs:9459+` that bulk-inserts `INSERT INTO fts_messages(rowid, …) SELECT … FROM messages m WHERE NOT EXISTS …` to populate the missing rows.

**Catch**: the maintenance path's bulk INSERT is still wrapped in `with_internal_statement_savepoint_and_cx` per statement. SQLite/fsqlite doesn't fire per-row savepoint for a multi-row INSERT — it fires once per `execute_with_params` call. So if the maintenance path runs `INSERT … SELECT …` as a single statement, it triggers ONE savepoint clone (current state, before the bulk insert) instead of N per-row clones. That's still a big clone (final state), but only one, not N.

**Code volume**: ~30–80 LOC in cass (`src/storage/sqlite.rs` to flag the deferral, `src/indexer/mod.rs` to skip per-row insert, glue to invoke the maintenance path at end-of-batch).

**Acceptance reach**: hits #1, hits #4 (no `_platform_memset` hot frame because no per-row clones during the long pi run; final bulk clone is one-shot and short). #2 (peak RSS < 8 GB) is the open question — even one final clone of the full state will be GB. The eventual peak depends on how large the state is at maintenance time. May NOT hit #2 cleanly without additional work (Problem B fix in frankensqlite).

**Spec 014 implications**: cass-local fix path. Honors spec.md "Constraint" section verbatim. Smallest scope change. Risk: doesn't actually solve #2 if the final maintenance clone is still > 8 GB.

### D4 — One-Worker spike (read fsqlite_ext_fts5 source deeper)

This Scout note IS that spike for the read-only portion. The remaining open question is whether D3's "one big clone at maintenance time" stays under 8 GB; that needs a measured test with a real-sized DB, not more source reading. So D4 as "read more source" is now exhausted; D4 as "synth a benchmark" overlaps with the regression test work in Group D of tasks.md (T11–T14).

## Recommendation for the user

The pure cass-local options (D3) are the lowest-cost in PR-shape but have an acceptance-#2 risk. The upstream options (D1, D2) are higher-cost in coordination but cleanly hit acceptance #2.

If a fast cycle close matters more than acceptance #2 precision: **D3** with an explicit spec-014 amendment that acceptance #2 may not be hit until D1 lands as a follow-up.

If keeping acceptance #2 in this cycle's contract matters more than cycle width: **D1** (shape (a) — Arc-based CoW) is the cleanest landing because shape (b) is more invasive and shape (c) doesn't actually solve the dominant cost.

Either way, this is the user's scope-change call, not mine.

## Watcher / process state

- `com.cass.index-watch` running (PID 30892), reloaded after T2/T3.
- DB pi_agent count unchanged at 33.
- No source edits made by this task; only the spec's notes/ files added.

## Side-finding worth a separate bead (not blocking this decision)

Per Problem B above, fsqlite_ext_fts5 stores column values in `self.documents` even for `ContentMode::Contentless` tables. This is a structural correctness gap (it isn't honoring the "contentless" contract) and a memory-cost gap. Worth a separate bead against `frankensqlite` regardless of which D-path is chosen for spec 014.
