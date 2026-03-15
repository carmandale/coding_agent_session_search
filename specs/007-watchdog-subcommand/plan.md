---
title: "plan: cass watchdog subcommand — Shape A"
date: 2026-03-15
bead: coding_agent_session_search-2efx
---

# Implementation Plan: cass watchdog subcommand

## Architecture

**New file: `src/watchdog.rs`** (~300 lines) — all watchdog logic.
**Modified: `src/lib.rs`** — `WatchdogCommand` enum + thin dispatch (~40 lines).
**Modified: `src/indexer/mod.rs`** — PID file write/delete (~10 lines).
**Modified: `Cargo.toml`** — add `libc = "*"` as explicit dep.

## Design Decisions

### Signal sending: `libc::kill()`
Use `libc::kill(pid, libc::SIGTERM)` directly. Already a transitive dep
(15+ crates pull it). Add as explicit dep for clarity. Error handling
must distinguish:
- `ESRCH` (errno 3) → process doesn't exist, stale PID file → cleanup
- `EPERM` (errno 1) → process exists, can't kill → exit with permission error
- `0` (success) → signal sent

### File locking: `std::fs::File::try_lock_exclusive()`
Stabilized in Rust 1.77, we're on 1.88. No `fs2` crate needed. The `File`
handle must be kept alive for the entire watchdog run via `_lock_guard` binding.

### PID file: `<data_dir>/watcher.pid`
Plain text, decimal PID only. Written by watcher after startup banner.
Deleted on clean shutdown (SIGTERM handler). Staleness checked via
`libc::kill(pid, 0)`.

**PID recycling mitigation:** Documented — heartbeat freshness covers
this case. A recycled PID with a stale heartbeat is correctly identified
as a stuck/dead watcher and restarted.

### Binary path: `which::which("cass")` at install-time
Called during `cass watchdog install`, NOT during watchdog runtime (launchd
has minimal PATH). Resolved path baked into plist XML as absolute path.
Override via `--binary-path <path>`.

### Plist generation: embedded templates
All paths resolved at install-time via `dirs::home_dir()`. No shell variable
expansion in plist XML. Each generated plist includes
`<!-- managed by cass -->` marker on the first line inside `<plist>`.

### Result enum for testability
```rust
pub enum WatchdogResult {
    Healthy,
    Restarted { was_stale_secs: u64 },
    NotRunning,
    AlreadyLocked,
    Error(String),
}
```
`run()` returns this enum. CLI dispatch maps to exit codes (0/1/2/3).

### Heartbeat deletion before kill
After deciding to restart, delete the heartbeat file before sending SIGTERM.
Post-restart verification checks for a fresh heartbeat file (new file with
recent timestamp), avoiding false negatives from the old stale timestamp.

## File Changes

### `Cargo.toml`
```toml
libc = "*"
```
Already transitive, making explicit for `libc::kill()` usage.

### `src/lib.rs`

Add to `Commands` enum (~line 494, near `Sources`):
```rust
/// Watchdog: monitor and manage the watcher daemon
#[command(subcommand)]
Watchdog(WatchdogCommand),
```

Add `WatchdogCommand` sub-enum:
```rust
#[derive(Subcommand, Debug, Clone)]
pub enum WatchdogCommand {
    /// Run a one-shot health check (heartbeat + log rotation + restart if stale)
    Run,
    /// Install launchd plists for watcher + watchdog
    Install {
        /// Override cass binary path (default: which cass)
        #[arg(long)]
        binary_path: Option<PathBuf>,
        /// Overwrite existing hand-written plists
        #[arg(long)]
        force: bool,
    },
    /// Remove launchd plists for watcher + watchdog
    Uninstall,
}
```

Dispatch in `execute_cli`:
```rust
Commands::Watchdog(subcmd) => {
    crate::watchdog::run_watchdog_command(subcmd)?;
}
```

Bare `cass watchdog` (no subcommand) maps to `Run` via `#[command(default_subcommand)]`
or by checking if no subcommand was provided.

### `src/watchdog.rs` (new, ~300 lines)

Major functions:
- `run_watchdog_command(cmd: WatchdogCommand)` — dispatch
- `run_health_check(data_dir: &Path) -> WatchdogResult` — core logic
- `check_heartbeat(data_dir: &Path) -> Option<u64>` — returns age in seconds
- `rotate_log_if_needed(log_path: &Path)` — copytruncate if > 100 MB
- `kill_watcher(pid: u32, data_dir: &Path) -> Result<()>` — SIGTERM → wait → SIGKILL
- `install_plists(binary_path: Option<PathBuf>, force: bool) -> Result<()>`
- `uninstall_plists() -> Result<()>`
- `generate_watcher_plist(binary_path: &str, home: &str) -> String`
- `generate_watchdog_plist(binary_path: &str, home: &str) -> String`
- `is_cass_managed(plist_path: &Path) -> bool` — check for marker comment

### `src/indexer/mod.rs`

After startup banner (line 916), add PID file write:
```rust
let pid_path = data_dir.join("watcher.pid");
let _ = std::fs::write(&pid_path, std::process::id().to_string());
```

In shutdown handler (after loop break), delete PID file:
```rust
let _ = std::fs::remove_file(&pid_path);
```

## Plist Templates

### com.cass.index-watch.plist
```xml
<?xml version="1.0" encoding="UTF-8"?>
<!-- managed by cass -->
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" ...>
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.cass.index-watch</string>
    <key>ProgramArguments</key>
    <array>
        <string>{binary_path}</string>
        <string>index</string>
        <string>--watch</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{home}/Library/Logs/cass-index-watch.log</string>
    <key>StandardErrorPath</key>
    <string>{home}/Library/Logs/cass-index-watch.log</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>{binary_dir}:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin</string>
    </dict>
</dict>
</plist>
```

### com.cass.health-watchdog.plist
```xml
<?xml version="1.0" encoding="UTF-8"?>
<!-- managed by cass -->
...
    <key>ProgramArguments</key>
    <array>
        <string>{binary_path}</string>
        <string>watchdog</string>
        <string>run</string>
    </array>
    <key>StartInterval</key>
    <integer>600</integer>
...
```

## Tests

| Test | What it verifies | Type |
|------|-----------------|------|
| `heartbeat_age_calculation` | Correct age from timestamp file | Unit |
| `heartbeat_stale_detection` | Age > 2700s → stale | Unit |
| `log_rotation_threshold` | Files > 100MB trigger rotation | Unit |
| `pid_file_read_write` | PID roundtrip to file | Unit |
| `pid_stale_detection` | Non-existent PID → stale | Unit |
| `kill_errno_handling` | ESRCH vs EPERM vs success | Unit |
| `plist_marker_detection` | `is_cass_managed` finds marker | Unit |
| `plist_generation_correct` | Template interpolation produces valid XML | Unit |
| `watchdog_result_to_exit_code` | Enum → code mapping | Unit |
| `lockfile_prevents_concurrent` | Second lock attempt fails | Unit |
