<!-- plan:complete:v1 | harness: pi/claude-sonnet-4-6 | date: 2026-04-01T08:54:33Z -->

# Planning Transcript — Spec 010

**Session**: 2026-03-31  
**Planner**: FastNova (pi/claude-sonnet-4-6)  
**Challenger**: UltraHawk (crew-challenger/claude-opus-4-6)  

---

## Round 1: Initial Findings → Challenge

**FastNova presented**:
- RC1: `ingest_batch(?)`  propagates LockBusy from intermediate tantivy commits (v0.2.5)
- RC2: zero-delay retry races IndexWriter Drop → LockBusy
- RC3: `last_scan_ts` only saved on perfect completion
- Version mismatch: installed binary is v0.2.5 (upstream), source is v0.1.55 (fork)

**UltraHawk challenged** [PHASE:challenge]:

1. **RC1 root cause wrong for v0.1.55**: In v0.1.55, `ingest_batch` → `add_messages()` only calls `writer.add_document()`, never `commit()`. LockBusy is a commit-time error. The intermediate-commit behavior exists only in v0.2.5. Diagnosing v0.2.5 crash with v0.1.55 source is a category error.

2. **Done-counting mechanism correct**: `get_connector_factories()` returns 17 connectors. Consumer waits for all 17 Done signals including cursor. The spec's RC1 description (consumer exits before cursor because of Done-counting) is architecturally wrong.

3. **RC3 fix introduces data loss**: Saving `last_scan_ts = scan_start_ts` on partial failure means the next incremental scan skips the connectors whose batches weren't processed. Full scan is painful but correct; partial-save is silent data loss.

4. **Version mismatch is a showstopper, not a "concern"**: The crash chain only makes sense in v0.2.5. Fixing v0.1.55 may not fix what's running.

5. **What kills the consumer in v0.1.55?**: If not LockBusy, could be SQLite BUSY. The `drop_close` warning suggests the connection was in a bad state. WAL contention with concurrent reads is possible.

---

## Round 2: Revision → Approval

**FastNova revised** [PHASE:revise]:

Verified via filesystem:
- `~/.cargo/bin/cass` = v0.2.5, 31.8MB, built Mar 30 15:29 (from upstream, not from fork)
- `./target/release/cass` = v0.1.55, 39MB, built Mar 30 03:20 (from fork source)
- `dev-install.sh` does `cargo install --path .` → replaces `~/.cargo/bin/cass` with fork build

**Conceded**:
- Challenge 1 (RC1 analysis wrong for v0.1.55): ✓ confirmed
- Challenge 2 (Done-counting correct): ✓ confirmed
- Challenge 4 (version mismatch showstopper): ✓ confirmed
- Challenge 3 (RC3 data loss): ✓ partially conceded — agreed not to save timestamp on partial failure

**Disagreed on Challenge 5**: The `drop_close` is a consequence of the consumer exiting (stack unwind drops the borrowed connection), not a cause. SQLite WAL contention is possible but unlikely to be the primary trigger.

**Revised plan**:
- Phase 0: `./dev-install.sh` (immediate relief, no code changes)
- Phase 1: RC1-defense (non-fatal ingest + `any_batch_failed`), RC2-defense (LockBusy retry in `open_or_create`), SQLite `busy_timeout`
- Phase 2: Tests + verify

**UltraHawk approved** [PHASE:approved → COMPLETE] with three conditions:

1. **RC1-defense must track `any_batch_failed`**: Making `ingest_batch` non-fatal means the scan always returns Ok, so `last_scan_ts` would be saved even on partial failure. Must track the flag and gate timestamp on it. The fix: return `(Vec<String>, bool)` from `run_streaming_consumer`; if `any_batch_failed`, skip `set_last_scan_ts`.

2. **RC2-defense targets wrong location**: `run_index_with_data` is not the retry point — the retry is launchd. The LockBusy hits at `TantivyIndex::open_or_create()` → `index.writer(50_000_000)`. Sleep must be inside `open_or_create` on LockBusy, not in a caller that doesn't exist in this capacity.

3. **SQLite `busy_timeout` confirmed missing**: Verified `apply_pragmas` at line 1723 in `src/storage/sqlite.rs` — WAL set, no `busy_timeout`. Adding `PRAGMA busy_timeout = 5000;` is confirmed necessary.

**Note on Condition 2**: FastNova verified that `run_index_with_data` DOES exist in src/lib.rs at line 7751. However, UltraHawk's architectural point is correct: the function runs the index once and exits; it's launchd (not Rust code) that retries. The LockBusy happens in the NEW process's startup, so the retry sleep belongs in `open_or_create`.

---

## Final Plan Summary

Three changes to two files, all defensive:

| File | Change | Addresses |
|------|--------|-----------|
| `src/indexer/mod.rs` | Non-fatal `ingest_batch` with `any_batch_failed` tracking; gate `set_last_scan_ts` | RC1 + RC3 |
| `src/search/tantivy.rs` | LockBusy retry with 5s sleep in `open_or_create` | RC2 |
| `src/storage/sqlite.rs` | `PRAGMA busy_timeout = 5000` in `apply_pragmas` | SQLite contention |
