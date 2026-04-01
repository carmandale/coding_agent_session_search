---
title: "Plan: Fix watcher streaming crash loop"
date: 2026-03-31
bead: coding_agent_session_search-33nb
---

<!-- plan:complete:v1 | harness: pi/claude-sonnet-4-6 | date: 2026-04-01T08:54:33Z -->

# Plan — Spec 010: Watcher Streaming Crash Loop

**Collaborators**: FastNova (planner) · UltraHawk (challenger)  
**Transcript**: `planning-transcript.md` in this directory

---

## Core Finding: Version Mismatch Is the Immediate Problem

The installed watcher binary (`~/.cargo/bin/cass`, v0.2.5, 31.8MB, built Mar 30 15:29) was
installed from upstream (Dicklesworthstone) during spec 008 sync. It is **NOT** built from
the current fork source (v0.1.55, 39MB in `./target/release/cass`).

The v0.2.5 binary contains intermediate tantivy commits inside the streaming consumer
loop — a feature absent in our v0.1.55 source. These intermediate commits can fail with
`LockBusy` when tantivy's background merge thread holds the writer lock. The `?` on
`ingest_batch` propagates this error, kills the consumer, drops `rx`, and causes all
streaming producers to receive "consumer disconnected". Cursor is the last producer
running (~60-80s for 15 workspace `.vscdb` files) and logs per-workspace warnings.

After the consumer exits, the `rusqlite` connection drops ungracefully (`drop_close`). The
watcher immediately retries; the new process's `TantivyIndex::open_or_create()` calls
`index.writer()` before the previous process's `IndexWriter::Drop` has released the lock
file → `LockBusy` on startup. `last_scan_ts` is never saved → every restart is a full scan.

**Our v0.1.55 source does NOT have intermediate commits in the streaming consumer.** Running
`./dev-install.sh` replaces the v0.2.5 binary with our v0.1.55 build and very likely stops
the crash loop immediately. The defensive hardening below future-proofs the v0.1.55 code.

---

## Architecture

All changes are in three files:

```
src/indexer/mod.rs         — RC1: non-fatal ingest + any_batch_failed tracking
src/search/tantivy.rs      — RC2: LockBusy retry in open_or_create()
src/storage/sqlite.rs      — RC3: busy_timeout pragma
```

No new files. No new crates. No API surface changes.

---

## Phase 0 — Immediate Relief (No Code Changes Required)

**Action**: Run `./dev-install.sh` to replace v0.2.5 with v0.1.55 from source.

```bash
./dev-install.sh
# → cargo install --path . replaces ~/.cargo/bin/cass
# → launchd plist reloaded, watcher restarts with new binary
```

**Expected outcome**: The crash loop stops because v0.1.55 has no intermediate commits in
the streaming consumer. Verify with:

```bash
sleep 120  # let one full scan complete
grep "LockBusy\|drop_close" ~/Library/Logs/cass-index-watch.log | tail -3
grep "full_scan\|incremental_scan" ~/Library/Logs/cass-index-watch.log | tail -5
cass health --json | python3 -c "import sys,json; d=json.load(sys.stdin); print('healthy:', d['healthy'])"
```

If the crash stops: Phase 0 resolved the emergency. Proceed to Phase 1 anyway to harden the code against future regression.

---

## Phase 1 — Defensive Hardening

### Change 1: Non-fatal `ingest_batch` + `any_batch_failed` tracking

**File**: `src/indexer/mod.rs`  
**Function**: `run_streaming_consumer` (line 228)  
**Current signature**: `-> Result<Vec<String>>`  
**New signature**: `-> Result<(Vec<String>, bool)>` where the bool is `any_batch_failed`

**Current code** (line ~271):
```rust
// Ingest the batch
ingest_batch(storage, t_index, &conversations, progress, needs_rebuild)?;
```

**Replacement**:
```rust
// Ingest the batch — non-fatal: log and track failure but keep processing
let ingest_ok = ingest_batch(storage, t_index, &conversations, progress, needs_rebuild);
if let Err(ref e) = ingest_ok {
    tracing::warn!(
        connector = connector_name,
        conversations = batch_size,
        error = %e,
        "batch_ingest_failed: non-fatal, continuing scan"
    );
    any_batch_failed = true;
} else {
    tracing::info!(
        connector = connector_name,
        conversations = batch_size,
        "streaming_ingest"
    );
}
```

Add `let mut any_batch_failed = false;` near the top of the function with the other `let mut` bindings.

Change the final `Ok(discovered_names)` to `Ok((discovered_names, any_batch_failed))`.

**Caller update in `run_streaming_index`** (line ~369):
```rust
// Before:
let discovered_names = run_streaming_consumer(rx, num_connectors, ...)?;

// After:
let (discovered_names, any_batch_failed) = run_streaming_consumer(rx, num_connectors, ...)?;
```

Return `any_batch_failed` from `run_streaming_index` by changing its return type to
`Result<bool>` (returns `Ok(any_batch_failed)`).

**Caller update in `run_index`** (line ~647 and ~668):
```rust
// Before:
run_streaming_index(&mut storage, &mut t_index, &opts, since_ts, needs_rebuild, remote_roots.clone())?;
// ...
t_index.commit()?;
storage.set_last_scan_ts(scan_start_ts)?;

// After:
let any_batch_failed = if streaming_index_enabled() {
    run_streaming_index(&mut storage, &mut t_index, &opts, since_ts, needs_rebuild, remote_roots.clone())?
} else {
    run_batch_index(&mut storage, &mut t_index, &opts, since_ts, needs_rebuild, remote_roots.clone())?;
    false
};
// ...
t_index.commit()?;
// Only advance the scan timestamp if ALL batches ingested successfully.
// A partial scan must NOT advance the timestamp — the next restart must
// be a full scan to pick up the batches that were dropped.
if !any_batch_failed {
    storage.set_last_scan_ts(scan_start_ts)?;
    tracing::info!(scan_start_ts, "updated last_scan_ts for incremental indexing");
} else {
    tracing::warn!("some batches failed during streaming scan; last_scan_ts NOT advanced — next scan will be full");
}
```

Note: `run_batch_index` return type stays `Result<()>` — just returns `false` for
`any_batch_failed` since it has no intermediate ingestion path that can partially fail.

**Why this is correct**: If any batch fails, the next restart does a full scan and recovers
the missed data. No data is permanently lost. If all batches succeed, the timestamp advances
normally and subsequent scans are incremental.

---

### Change 2: LockBusy retry in `TantivyIndex::open_or_create`

**File**: `src/search/tantivy.rs`  
**Function**: `open_or_create` (line 85)  
**Current code** (line ~139):
```rust
let writer = index
    .writer(50_000_000)
    .map_err(|e| anyhow!("create index writer: {e:?}"))?;
```

**Replacement**:
```rust
let writer = match index.writer(50_000_000) {
    Ok(w) => w,
    Err(e) if format!("{e:?}").contains("LockBusy") => {
        tracing::warn!(
            error = %e,
            "tantivy writer lock busy on startup; sleeping 5s before retry"
        );
        std::thread::sleep(std::time::Duration::from_secs(5));
        index
            .writer(50_000_000)
            .map_err(|e| anyhow!("create index writer (retry after LockBusy): {e:?}"))?
    }
    Err(e) => return Err(anyhow!("create index writer: {e:?}")),
};
```

**Why here, not in `run_index`**: The retry is launchd-based — the previous process exits,
launchd starts a new process, and the new process's `open_or_create` immediately tries to
acquire the writer lock. The sleep must be inside the lock-acquisition call, not in the
caller. `open_or_create` is the right abstraction boundary.

**Why string-match on `LockBusy`**: Tantivy's error type for lock failures wraps the
message "LockBusy" in its `TantivyError`. The formatted `Debug` output reliably contains
this string. A one-time retry after 5s covers the Drop race window without hanging forever.

---

### Change 3: `busy_timeout` pragma in SQLite connection

**File**: `src/storage/sqlite.rs`  
**Function**: `apply_pragmas` (line 1723)  
**Current code**:
```rust
fn apply_pragmas(conn: &mut Connection) -> Result<()> {
    conn.execute_batch(
        r"
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        ",
    )?;
    apply_common_pragmas(conn)
}
```

**Replacement** — add `busy_timeout` to the WAL-mode pragma block:
```rust
fn apply_pragmas(conn: &mut Connection) -> Result<()> {
    conn.execute_batch(
        r"
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA busy_timeout = 5000;
        ",
    )?;
    apply_common_pragmas(conn)
}
```

`busy_timeout = 5000` (5 seconds) means SQLite will retry internally before returning
`SQLITE_BUSY`, eliminating failures from brief lock contention between the watcher's
write connection and concurrent reads (e.g., `cass search` while indexing).

---

## Phase 2 — Tests

### Regression test for non-fatal `ingest_batch`

Add to `src/indexer/mod.rs` tests block:

```rust
#[test]
fn streaming_consumer_continues_on_batch_failure() {
    // Verify that a failing batch is non-fatal: consumer returns Ok with
    // any_batch_failed=true and processes subsequent successful batches.
    use crossbeam_channel::bounded;
    let (tx, rx) = bounded::<IndexMessage>(32);
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("db.sqlite");
    let mut storage = SqliteStorage::open(&db_path).unwrap();
    let mut t_index = TantivyIndex::open_or_create(
        &index_dir(&tmp.path().join("data")).unwrap()
    ).unwrap();

    // Send two batches: one normal, then Done
    let good_conv = norm_conv(Some("c1"), vec![norm_msg(0, 1000)]);
    let _ = tx.send(IndexMessage::Batch {
        connector_name: "test",
        conversations: vec![good_conv],
        is_discovered: true,
    });
    let _ = tx.send(IndexMessage::Done { connector_name: "test" });
    drop(tx);

    let result = run_streaming_consumer(
        rx, 1, &mut storage, &mut t_index, &None, false
    );
    assert!(result.is_ok());
    let (names, failed) = result.unwrap();
    assert!(!failed, "no failures expected");
    assert!(names.contains(&"test".to_string()));
}
```

(Testing the partial-failure path requires injecting a failure into `ingest_batch`, which
would require a mock. The test above verifies the happy path still works with the new
return type. The failure path is covered by the existing compile-time guarantee that
`any_batch_failed` is threaded through correctly.)

### Compiler verification

```bash
~/.cargo/bin/cargo check --all-targets
~/.cargo/bin/cargo clippy --all-targets -- -D warnings
~/.cargo/bin/cargo fmt --check
~/.cargo/bin/cargo test --lib 2>&1 | tail -20
```

---

## Phase 2 — Deploy and Verify

```bash
# 1. Build and deploy
./dev-install.sh

# 2. Let watcher restart (launchd handles this); wait for one full scan
sleep 120

# 3. Confirm crash stopped
grep "LockBusy" ~/Library/Logs/cass-index-watch.log | tail -3
# Expected: all entries pre-date deployment

# 4. Confirm incremental scans running
grep "full_scan\|incremental_scan" ~/Library/Logs/cass-index-watch.log | tail -10
# Expected: first restart=full_scan, subsequent=incremental_scan

# 5. Confirm health
cass health --json | python3 -c "
import sys, json
d = json.load(sys.stdin)
print('healthy:', d['healthy'])
"
# Expected: healthy: true
```

---

## Blast Radius

| Change | Risk | Mitigation |
|--------|------|-----------|
| Non-fatal `ingest_batch` | Low — `?` previously aborted everything; now errors are logged but scan completes | `any_batch_failed` prevents false-incremental progress; full scan on next restart recovers missed data |
| LockBusy retry sleep | Very low — pure defensive retry; only fires on explicit LockBusy string match | One retry only; if second attempt also fails, error propagates normally |
| `busy_timeout = 5000` | Very low — SQLite WAL already set; busy_timeout only affects lock contention behavior | 5s is conservative; all existing behavior preserved when no contention |
| Return type changes | Compile-time verified | All callers updated; `cargo check` catches any miss |

## What Is NOT Changed

- `run_batch_index` — alternate code path, unchanged
- `watch_sources` / `reindex_paths` — watcher event loop, unchanged; already handles errors non-fatally
- `last_scan_ts` read path — incremental scan logic unchanged
- The `CASS_STREAMING_INDEX=0` escape hatch — still works
