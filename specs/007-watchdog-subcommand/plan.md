<!-- Codex Review: APPROVED after 3 rounds | model: gpt-5.3-codex | date: 2026-03-15 -->
<!-- Status: REVISED -->
<!-- Revisions: R1: A6 health field, PID identity verification, macOS guard, launchctl sequence, bare cass watchdog default, 14→19 tests. R2: launchctl error handling, install decision tree, uninstall sequence, 5 more tests. -->
---
title: "plan: cass watchdog subcommand — Shape A"
date: 2026-03-15
bead: coding_agent_session_search-2efx
---

# Implementation Plan: cass watchdog subcommand

## Architecture

**New file: `src/watchdog.rs`** (~350 lines) — all watchdog logic, wrapped
in `#[cfg(target_os = "macos")]`. On non-macOS, a stub module provides the
`WatchdogCommand` enum and a `run_watchdog_command()` that prints
"watchdog is only supported on macOS" and exits with code 2.
**Modified: `src/lib.rs`** — `WatchdogCommand` enum + thin dispatch (~40 lines),
health JSON extension for plist status.
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

**PID identity verification before kill:** Before sending SIGTERM, verify
the process at that PID is actually `cass index --watch` by reading its
command line via `sysctl kern.procargs2` (macOS). If the command line
doesn't contain `cass` AND `index` AND `--watch`, treat as stale PID
(clean up PID file, return `NotRunning`). This prevents killing the wrong
process on PID reuse or PID file tampering.

```rust
fn verify_pid_is_watcher(pid: u32) -> bool {
    // Use sysctl KERN_PROCARGS2 to get the process command line
    let output = std::process::Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "args="])
        .output();
    match output {
        Ok(out) => {
            let cmdline = String::from_utf8_lossy(&out.stdout);
            cmdline.contains("cass") && cmdline.contains("index") && cmdline.contains("--watch")
        }
        Err(_) => false, // Can't verify → treat as not-watcher
    }
}
```

**PID recycling mitigation:** Two-layer defense:
1. Command-line verification (above) catches PID reuse by unrelated processes
2. Heartbeat freshness covers the case where a recycled PID happens to be
   another `cass` process (astronomically unlikely but documented)

### Binary path: `which::which("cass")` at install-time
Called during `cass watchdog install`, NOT during watchdog runtime (launchd
has minimal PATH). Resolved path baked into plist XML as absolute path.
Override via `--binary-path <path>`.

### Plist generation: embedded templates
All paths resolved at install-time via `dirs::home_dir()`. No shell variable
expansion in plist XML. Each generated plist includes
`<!-- managed by cass -->` marker on the first line inside `<plist>`.

### Idempotent launchctl sequence (install)
The install command uses a deterministic bootout→write→bootstrap flow:
```rust
fn install_and_load(plist_path: &Path, label: &str) -> Result<()> {
    let uid = unsafe { libc::getuid() };
    let domain = format!("gui/{uid}");

    // 1. Bootout if already loaded (ignore errors — may not be loaded)
    let _ = std::process::Command::new("launchctl")
        .args(["bootout", &format!("{domain}/{label}")])
        .output();

    // 2. Write the plist file (already done by caller)

    // 3. Bootstrap (load) the new plist
    let output = std::process::Command::new("launchctl")
        .args(["bootstrap", &domain, &plist_path.display().to_string()])
        .output()?;
    if !output.status.success() {
        // Fallback: try legacy `launchctl load` for older macOS
        let fallback = std::process::Command::new("launchctl")
            .args(["load", &plist_path.display().to_string()])
            .output()?;
        if !fallback.status.success() {
            let stderr = String::from_utf8_lossy(&fallback.stderr);
            anyhow::bail!(
                "Failed to load plist {}: bootstrap and load both failed. {}",
                plist_path.display(),
                stderr
            );
        }
    }
    Ok(())
}
```
This is idempotent: bootout removes the old job if loaded, bootstrap
registers the new one. Both load paths check for failure and return
an error if neither succeeds.

### Install decision tree (R6 + R9)

```
For each plist (watcher + watchdog):
  1. Does the plist file exist?
     NO → write new plist, load it. Done.
     YES → go to 2.
  
  2. Is it cass-managed? (contains <!-- managed by cass --> marker)
     YES → overwrite silently, reload. Done.
     NO → go to 3.

  3. Was --force passed?
     YES → overwrite, reload. Print warning: "Overwriting hand-written plist."
     NO → return error: "Existing plist not managed by cass. Use --force."
```

### Uninstall sequence

```
For each plist (watcher + watchdog):
  1. Bootout the launchd job: launchctl bootout gui/<uid>/<label>
     (ignore errors — may not be loaded)
  2. Remove the plist file: std::fs::remove_file()
     (return error if file doesn't exist and was expected)
  3. Print confirmation: "Removed <label>"
```

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
Watchdog {
    #[command(subcommand)]
    command: Option<WatchdogCommand>,
},
```

Using `Option<WatchdogCommand>` so bare `cass watchdog` (no subcommand)
defaults to `Run` behavior. The dispatch checks `None` → run health check.

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
Commands::Watchdog { command } => {
    crate::watchdog::run_watchdog_command(command)?;
}
```

**Extend `state_meta_json()` (lib.rs ~line 2280):** Add watchdog plist
installed status to the health JSON output:
```rust
// In state_meta_json(), add after the "pending" section:
"watchdog": {
    "plist_installed": home.join("Library/LaunchAgents/com.cass.health-watchdog.plist").exists(),
    "watcher_plist_installed": home.join("Library/LaunchAgents/com.cass.index-watch.plist").exists(),
}
```
This satisfies spec Shape A6.

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
| `pid_identity_verification` | Command-line check for cass index --watch | Unit |
| `kill_errno_handling` | ESRCH vs EPERM vs success | Unit |
| `plist_marker_detection` | `is_cass_managed` finds marker | Unit |
| `plist_generation_correct` | Template interpolation produces valid XML | Unit |
| `watchdog_result_to_exit_code` | Enum → code mapping | Unit |
| `lockfile_prevents_concurrent` | Second lock attempt fails | Unit |
| `health_includes_watchdog_field` | `cass health --json` includes `watchdog.plist_installed` | Integration |
| `install_creates_both_plists` | Both plist files created at expected paths | Integration |
| `install_overwrites_cass_managed` | Cass-managed plists overwritten silently | Unit |
| `install_blocks_hand_written_without_force` | Hand-written plists require --force | Unit |
| `install_force_overwrites_hand_written` | --force overwrites non-managed plists | Unit |
| `uninstall_removes_both_plists` | Both plist files removed | Unit |
| `install_fails_on_launchctl_error` | Returns error when both bootstrap and load fail | Unit |
| `non_macos_shows_error` | On non-macOS, command prints unsupported message | Unit |
