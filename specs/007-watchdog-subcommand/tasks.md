---
title: "tasks: cass watchdog subcommand"
date: 2026-03-15
bead: coding_agent_session_search-2efx
---

# Tasks

## Phase 1: Infrastructure

- [ ] **T1: Add libc as explicit dep**
  - Add `libc = "*"` to Cargo.toml
  - Verify: `cargo check`

- [ ] **T2: Create src/watchdog.rs with constants and result enum**
  - New file with `WatchdogResult` enum, constants (HEARTBEAT_THRESHOLD_SECS, etc.)
  - Add `pub mod watchdog;` to src/lib.rs
  - Verify: `cargo check`

- [ ] **T3: Add WatchdogCommand to CLI**
  - Add `Watchdog(WatchdogCommand)` to `Commands` enum in src/lib.rs
  - Add `WatchdogCommand` sub-enum with `Run`, `Install`, `Uninstall`
  - Add dispatch in `execute_cli` to call `watchdog::run_watchdog_command`
  - Verify: `cargo check`, `cass watchdog --help` shows subcommands

## Phase 2: PID file in watcher

- [ ] **T4: Write PID file on watcher startup**
  - In `watch_sources` (src/indexer/mod.rs), write `<data_dir>/watcher.pid` after startup banner
  - Delete PID file on clean shutdown (after watch loop breaks)
  - Thread `data_dir` path to the shutdown cleanup
  - Verify: start watcher, check PID file exists, SIGTERM watcher, check PID file deleted

## Phase 3: Core watchdog logic

- [ ] **T5: Implement heartbeat check**
  - `check_heartbeat(data_dir) -> Option<u64>` — reads heartbeat file, returns age in seconds
  - `is_heartbeat_stale(age: u64) -> bool` — age > HEARTBEAT_THRESHOLD_SECS
  - Tests: heartbeat_age_calculation, heartbeat_stale_detection

- [ ] **T6: Implement PID management**
  - `read_pid_file(data_dir) -> Option<u32>` — read and parse PID
  - `is_pid_alive(pid: u32) -> bool` — `libc::kill(pid, 0)`, handle ESRCH/EPERM
  - `kill_watcher(pid: u32, data_dir: &Path) -> Result<()>` — delete heartbeat, SIGTERM, wait loop, SIGKILL
  - Tests: pid_file_read_write, pid_stale_detection, kill_errno_handling

- [ ] **T7: Implement log rotation**
  - `rotate_log_if_needed(log_path: &Path)` — if > LOG_MAX_BYTES, cp then truncate
  - Log path: `~/Library/Logs/cass-index-watch.log` (resolved via `dirs::home_dir()`)
  - Test: log_rotation_threshold

- [ ] **T8: Implement lockfile**
  - `acquire_lock(data_dir: &Path) -> Result<File>` — try_lock_exclusive on watchdog.lock
  - Return File handle (caller keeps in scope as `_lock_guard`)
  - Test: lockfile_prevents_concurrent

- [ ] **T9: Wire up `cass watchdog run`**
  - `run_health_check(data_dir) -> WatchdogResult` — orchestrates T5-T8
  - Acquire lock → rotate log → check heartbeat → restart if stale
  - Map WatchdogResult to exit codes: Healthy=0, Restarted=1, NotRunning=2, AlreadyLocked=0, Error=3
  - Verify: run against live watcher, confirm "healthy" output

## Phase 4: Install/Uninstall

- [ ] **T10: Implement plist generation**
  - `generate_watcher_plist(binary_path, home) -> String`
  - `generate_watchdog_plist(binary_path, home) -> String`
  - Include `<!-- managed by cass -->` marker
  - Test: plist_generation_correct (valid XML, correct paths)

- [ ] **T11: Implement install command**
  - Resolve binary path via `which::which("cass")` or `--binary-path`
  - Check for existing plists — `is_cass_managed(path)` checks for marker
  - If not cass-managed and no `--force`: error with message
  - Write plists to `~/Library/LaunchAgents/`
  - Load via `launchctl load` (or `launchctl bootstrap gui/<uid>`)
  - Test: plist_marker_detection

- [ ] **T12: Implement uninstall command**
  - Unload via `launchctl unload` (or `launchctl bootout gui/<uid>`)
  - Remove plist files
  - Verify: plists removed, watcher stopped

## Phase 5: Verification

- [ ] **T13: cargo check + clippy + fmt**
  - All targets clean

- [ ] **T14: Run all tests**
  - All existing tests pass
  - All new watchdog tests pass

- [ ] **T15: End-to-end manual verification**
  - `cass watchdog install` — creates and loads both plists
  - `cass watchdog` — reports healthy
  - Stop watcher, wait for watchdog to detect and restart
  - `cass watchdog uninstall` — removes plists, stops watcher

## Dependency Graph

```
T1 → T2 → T3 → T4 → T5 ─┐
                    T6 ─┤
                    T7 ─┼→ T9 → T10 → T11 → T12 → T13 → T14 → T15
                    T8 ─┘
```

T5-T8 can be done in parallel. T9 wires them together.
T10-T12 (install/uninstall) depend on T9 being complete.
Estimated total: 4-6 hours.
