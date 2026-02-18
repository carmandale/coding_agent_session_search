# Tasks: Watcher Reliability Improvements

## Phase 1: Watchdog Launchd Service (Quick Win)

- [ ] **1.1** Create watchdog script at `~/.local/share/cass/watchdog.sh`
  - [ ] Check `cass health --json`
  - [ ] Parse healthy status and age_seconds
  - [ ] If unhealthy OR age > 600: restart watcher via launchctl
  - [ ] Trigger full reindex after restart
  - [ ] Log all actions to syslog via `logger -t cass-watchdog`

- [ ] **1.2** Create launchd plist at `~/Library/LaunchAgents/com.cass.health-watchdog.plist`
  - [ ] StartInterval: 600 (10 minutes)
  - [ ] Run watchdog.sh
  - [ ] Log to `~/Library/Logs/cass-watchdog.log`

- [ ] **1.3** Deploy and test
  - [ ] `chmod +x ~/.local/share/cass/watchdog.sh`
  - [ ] `launchctl load ~/Library/LaunchAgents/com.cass.health-watchdog.plist`
  - [ ] Manually stop watcher, verify watchdog restarts it within 10 min
  - [ ] Check logs for expected output

## Phase 2: Fix Timestamp Advancement Race

- [ ] **2.1** Modify `reindex_paths()` in `src/indexer/mod.rs`
  - [ ] Move `save_watch_state()` call to AFTER confirmed successful `t_index.commit()`
  - [ ] Add error logging if commit fails: `tracing::error!("commit failed, NOT advancing watch_state")`
  - [ ] Wrap timestamp advancement in `if commit_result.is_ok()` block

- [ ] **2.2** Add regression test
  - [ ] Test case: Simulate commit failure, verify watch_state NOT updated
  - [ ] Test case: Successful commit, verify watch_state IS updated

- [ ] **2.3** Code review and merge

## Phase 3: Claude-Specific Aggressive Scanning

- [ ] **3.1** Add per-connector health tracking struct
  ```rust
  struct ConnectorHealth {
      last_success_ts: i64,
      consecutive_empty_scans: u32,
  }
  ```

- [ ] **3.2** Track last successful ingest per connector in watch loop
  - [ ] Update `last_success_ts` when `streaming_ingest conversations > 0`
  - [ ] Increment `consecutive_empty_scans` on empty results

- [ ] **3.3** Add forced full scan logic for Claude
  - [ ] If `now - claude_health.last_success_ts > 1800` (30 min): trigger full scan
  - [ ] Log: `tracing::warn!("Claude connector stale, forcing full scan")`
  - [ ] Reset connector with `since_ts=None`

- [ ] **3.4** Test manually
  - [ ] Create Claude session, wait 30+ min with watcher running
  - [ ] Verify session is picked up even if FSEvents failed

## Phase 4: Improved Logging

- [ ] **4.1** Add startup banner with flush
  ```rust
  tracing::info!("cass watcher starting, version={}", env!("CARGO_PKG_VERSION"));
  std::io::stderr().flush().ok();
  ```

- [ ] **4.2** Add structured logging fields to scan_complete
  - [ ] Include: connector, scanned count, ingested count, since_ts, elapsed_ms

- [ ] **4.3** Verify logs are non-empty after 5 minutes of runtime

## Phase 5: Documentation & Monitoring

- [ ] **5.1** Update napkin.md with findings
- [ ] **5.2** Add troubleshooting section to README or docs
- [ ] **5.3** Create bead for tracking this work
- [ ] **5.4** Close bead when all phases complete

## Verification Checklist

After all tasks complete:

- [ ] Run `cass index --watch` for 24 hours
- [ ] Verify Claude sessions appear in `gj last` within 30 min of creation
- [ ] Check `~/Library/Logs/cass-index-watch.log` is non-empty
- [ ] Check `~/Library/Logs/cass-watchdog.log` shows periodic checks
- [ ] Manually kill watcher, verify watchdog restarts it
