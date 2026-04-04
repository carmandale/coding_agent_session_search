---
title: "Tasks: Fix watcher streaming crash loop"
date: 2026-03-31
bead: coding_agent_session_search-33nb
---

<!-- plan:complete:v1 | harness: pi/claude-sonnet-4-6 | date: 2026-04-01T08:54:33Z -->

# Tasks — Spec 010: Watcher Streaming Crash Loop

Work against the live checkout. Read plan.md before starting — the why behind each task is documented there.

---

## Phase 0 — Immediate Relief (zero code changes)

- [ ] **T0.1** Run `./dev-install.sh` to replace the upstream v0.2.5 binary with v0.1.55 from source:
  ```bash
  ./dev-install.sh
  ```
  The script does `cargo install --path .`, wires `~/.local/bin/cass → ~/.cargo/bin/cass`, reloads launchd plist.

- [ ] **T0.2** Wait 2 minutes for watcher to complete its first full scan, then verify crash stopped:
  ```bash
  sleep 120
  grep "LockBusy" ~/Library/Logs/cass-index-watch.log | tail -3
  # Expected: all entries pre-date deployment
  grep "full_scan\|incremental_scan" ~/Library/Logs/cass-index-watch.log | tail -5
  cass health --json
  ```
  If crash still occurs: check `~/.cargo/bin/cass --version` (must say 0.1.55). If v0.2.5 persists, the cargo install may have been blocked by a lock — retry.

---

## Phase 1 — Defensive Hardening

### Step 1: RC1-defense — Non-fatal `ingest_batch` with `any_batch_failed` tracking

- [ ] **T1.1** In `run_streaming_consumer` (src/indexer/mod.rs ~line 228), add `let mut any_batch_failed = false;` alongside the existing `let mut` bindings.

- [ ] **T1.2** Replace the `ingest_batch(...)?;` line (~line 271) with a non-fatal match:
  ```rust
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

- [ ] **T1.3** Change the final `Ok(discovered_names)` in `run_streaming_consumer` to `Ok((discovered_names, any_batch_failed))`.

- [ ] **T1.4** Update the return type of `run_streaming_consumer` from `Result<Vec<String>>` to `Result<(Vec<String>, bool)>`.

- [ ] **T1.5** Update `run_streaming_index` (src/indexer/mod.rs ~line 324):
  - Change return type from `Result<()>` to `Result<bool>`
  - Update the `run_streaming_consumer(...)? ` call to destructure: `let (discovered_names, any_batch_failed) = run_streaming_consumer(...)?;`
  - Change final `Ok(())` to `Ok(any_batch_failed)`

- [ ] **T1.6** Update `run_index` (src/indexer/mod.rs ~line 647) to consume the new return type:
  ```rust
  let any_batch_failed = if streaming_index_enabled() {
      run_streaming_index(&mut storage, &mut t_index, &opts, since_ts, needs_rebuild, remote_roots.clone())?
  } else {
      run_batch_index(&mut storage, &mut t_index, &opts, since_ts, needs_rebuild, remote_roots.clone())?;
      false
  };
  ```

- [ ] **T1.7** Gate `set_last_scan_ts` on `any_batch_failed` in `run_index` (~line 668):
  ```rust
  t_index.commit()?;
  if !any_batch_failed {
      storage.set_last_scan_ts(scan_start_ts)?;
      tracing::info!(scan_start_ts, "updated last_scan_ts for incremental indexing");
  } else {
      tracing::warn!(
          "some batches failed during streaming scan; last_scan_ts NOT advanced — next scan will be full"
      );
  }
  ```
  Remove the old `tracing::info!(scan_start_ts, "updated last_scan_ts for incremental indexing")` line.

- [ ] **T1.8** Run `cargo check --all-targets` to catch any missed callers of `run_streaming_index` or `run_streaming_consumer`.

### Step 2: RC2-defense — LockBusy retry in `TantivyIndex::open_or_create`

- [ ] **T2.1** In `src/search/tantivy.rs`, add `use std::time::Duration;` to the imports if not already present (check existing imports first).

- [ ] **T2.2** Replace the writer acquisition block in `open_or_create` (line ~139):
  ```rust
  // Before:
  let writer = index
      .writer(50_000_000)
      .map_err(|e| anyhow!("create index writer: {e:?}"))?;

  // After:
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

### Step 3: SQLite `busy_timeout` pragma

- [ ] **T3.1** In `src/storage/sqlite.rs`, update `apply_pragmas` (line 1723) to add `PRAGMA busy_timeout = 5000;`:
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

---

## Phase 2 — Tests and Verification

### Regression test

- [ ] **T4.1** Add a regression test in `src/indexer/mod.rs` test block verifying `run_streaming_consumer` returns `Ok((names, false))` on a successful single-producer scan (confirms new return type and happy path):
  ```rust
  #[test]
  fn streaming_consumer_new_return_type_happy_path() {
      use crossbeam_channel::bounded;
      let (tx, rx) = bounded::<IndexMessage>(32);
      let tmp = TempDir::new().unwrap();
      let db_path = tmp.path().join("db.sqlite");
      let mut storage = SqliteStorage::open(&db_path).unwrap();
      let data_dir = tmp.path().join("data");
      std::fs::create_dir_all(&data_dir).unwrap();
      let mut t_index = TantivyIndex::open_or_create(
          &index_dir(&data_dir).unwrap()
      ).unwrap();

      let _ = tx.send(IndexMessage::Done { connector_name: "test" });
      drop(tx);

      let result = run_streaming_consumer(rx, 1, &mut storage, &mut t_index, &None, false);
      assert!(result.is_ok());
      let (names, any_failed) = result.unwrap();
      assert!(!any_failed, "no failures on happy path");
      assert!(names.is_empty(), "no discovered names when no batches sent");
  }
  ```

### Compiler checks

- [ ] **T4.2** `~/.cargo/bin/cargo check --all-targets` — zero errors

- [ ] **T4.3** `~/.cargo/bin/cargo clippy --all-targets -- -D warnings` — zero warnings

- [ ] **T4.4** `~/.cargo/bin/cargo fmt --check` — no formatting changes needed (run `cargo fmt` first if needed)

- [ ] **T4.5** `~/.cargo/bin/cargo test --lib 2>&1 | tail -30` — all tests pass

### Deploy and validate

- [ ] **T5.1** `./dev-install.sh` — builds and installs from source (if not already done in T0.1 or if code changed since then)

- [ ] **T5.2** Restart the watcher (launchd handles this automatically after plist reload; verify with `pgrep -a cass`):
  ```bash
  pgrep -fa "cass index --watch" | head -3
  ```

- [ ] **T5.3** Wait 3 minutes, then check logs:
  ```bash
  # No new LockBusy entries after deployment timestamp
  grep "LockBusy" ~/Library/Logs/cass-index-watch.log | tail -3

  # No new drop_close entries
  grep "drop_close" ~/Library/Logs/cass-index-watch.log | tail -3

  # Second+ restart should be incremental
  grep "full_scan\|incremental_scan" ~/Library/Logs/cass-index-watch.log | tail -10
  ```

- [ ] **T5.4** Confirm health:
  ```bash
  cass health --json | python3 -c "
  import sys, json; d = json.load(sys.stdin)
  print('healthy:', d['healthy'])
  print('age_s:', d.get('state', {}).get('index', {}).get('age_seconds', '?'))
  "
  # Expected: healthy: true, age_s < 300
  ```

- [ ] **T5.5** Confirm searches work:
  ```bash
  cass search "test" --robot --limit 3 | python3 -c "
  import sys, json; d = json.load(sys.stdin)
  print('hits:', len(d.get('hits', [])))
  "
  ```

---

## Bead Closeout

- [ ] **T6.1** Update bead status: `br update coding_agent_session_search-33nb --status=in_progress` (already set) → `br close coding_agent_session_search-33nb --reason="Fixed: deploy v0.1.55 (RC1-defense, RC2-defense, busy_timeout). Crash loop stopped."`

- [ ] **T6.2** `br sync --flush-only`

- [ ] **T6.3** Commit: `git add -A && git commit -m "fix(watcher): non-fatal batch ingest, LockBusy retry, busy_timeout (spec 010)"`
