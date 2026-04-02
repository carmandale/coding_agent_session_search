//! Watchdog subcommand: monitor and manage the watcher daemon.
//!
//! Provides heartbeat-based liveness checking, log rotation, PID management,
//! and launchd plist install/uninstall. Replaces the external `scripts/watchdog.sh`.

#[cfg(target_os = "macos")]
mod platform {
    use anyhow::{Context, Result, bail};
    use clap::Subcommand;
    use std::fs;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    // ── Constants (unit-testable) ──────────────────────────────────────────

    /// Heartbeat age threshold in seconds. If the watcher hasn't written a
    /// heartbeat within this window, it's considered stuck.
    pub const HEARTBEAT_THRESHOLD_SECS: u64 = 2700; // 45 minutes

    /// Maximum log file size before rotation (100 MB).
    pub const LOG_MAX_BYTES: u64 = 100 * 1024 * 1024;

    /// SIGTERM grace period in seconds before escalating to SIGKILL.
    pub const SIGTERM_GRACE_SECS: u64 = 120;

    /// Watchdog plist StartInterval (10 minutes).
    pub const WATCHDOG_INTERVAL_SECS: u64 = 600;

    /// Marker comment for cass-managed plists.
    pub const PLIST_MARKER: &str = "<!-- managed by cass -->";

    /// Watcher launchd label.
    pub const WATCHER_LABEL: &str = "com.cass.index-watch";

    /// Watchdog launchd label.
    pub const WATCHDOG_LABEL: &str = "com.cass.health-watchdog";

    // ── Result enum ───────────────────────────────────────────────────────

    /// Outcome of a watchdog health check. Maps to CLI exit codes.
    #[derive(Debug, PartialEq, Eq)]
    pub enum WatchdogResult {
        /// Watcher is alive and heartbeat is fresh. Exit code 0.
        Healthy,
        /// Watcher was stale and has been restarted. Exit code 1.
        Restarted { was_stale_secs: u64 },
        /// Watcher is not running (no PID file or stale PID). Exit code 2.
        NotRunning,
        /// Another watchdog instance holds the lock. Exit code 0 (not an error).
        AlreadyLocked,
        /// An error occurred during the check. Exit code 3.
        Error(String),
    }

    impl WatchdogResult {
        /// Map result to CLI exit code.
        pub fn exit_code(&self) -> i32 {
            match self {
                Self::Healthy => 0,
                Self::Restarted { .. } => 1,
                Self::NotRunning => 2,
                Self::AlreadyLocked => 0,
                Self::Error(_) => 3,
            }
        }
    }

    // ── CLI enum ──────────────────────────────────────────────────────────

    #[derive(Subcommand, Debug, Clone)]
    pub enum WatchdogCommand {
        /// Run a one-shot health check (heartbeat + log rotation + restart if stale)
        Run,
        /// Install launchd plists for watcher + watchdog
        Install {
            /// Override cass binary path (default: `which cass`)
            #[arg(long)]
            binary_path: Option<PathBuf>,
            /// Overwrite existing hand-written plists
            #[arg(long)]
            force: bool,
        },
        /// Remove launchd plists for watcher + watchdog
        Uninstall,
    }

    // ── Heartbeat ─────────────────────────────────────────────────────────

    /// Read the watcher heartbeat file and return its age in seconds.
    /// Returns `None` if the file doesn't exist or can't be parsed.
    pub fn check_heartbeat(data_dir: &Path) -> Option<u64> {
        let heartbeat_path = data_dir.join("watcher-heartbeat");
        let content = fs::read_to_string(&heartbeat_path).ok()?;
        let ts: u64 = content.trim().parse().ok()?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Some(now.saturating_sub(ts))
    }

    /// Check if a heartbeat age exceeds the staleness threshold.
    pub fn is_heartbeat_stale(age: u64) -> bool {
        age > HEARTBEAT_THRESHOLD_SECS
    }

    // ── PID management ────────────────────────────────────────────────────

    /// Read the watcher PID from the PID file.
    pub fn read_pid_file(data_dir: &Path) -> Option<u32> {
        let pid_path = data_dir.join("watcher.pid");
        let content = fs::read_to_string(&pid_path).ok()?;
        content.trim().parse().ok()
    }

    /// Check if a process is alive using `kill(pid, 0)`.
    /// Returns `true` if the process exists (even if we can't signal it — EPERM).
    pub fn is_pid_alive(pid: u32) -> bool {
        // SAFETY: kill(pid, 0) is a standard POSIX probe — no signal sent.
        let ret = unsafe { libc::kill(pid as i32, 0) };
        if ret == 0 {
            return true;
        }
        // errno check: ESRCH = no such process, EPERM = exists but can't signal
        let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
        errno == libc::EPERM
    }

    /// Verify the process at `pid` is actually `cass index --watch`.
    /// Prevents killing wrong process on PID reuse.
    pub fn verify_pid_is_watcher(pid: u32) -> bool {
        let output = std::process::Command::new("ps")
            .args(["-p", &pid.to_string(), "-o", "args="])
            .output();
        match output {
            Ok(out) => {
                let cmdline = String::from_utf8_lossy(&out.stdout);
                cmdline.contains("cass") && cmdline.contains("index") && cmdline.contains("--watch")
            }
            Err(_) => false,
        }
    }

    /// Kill the watcher process: delete heartbeat, SIGTERM, wait, SIGKILL if needed.
    pub fn kill_watcher(pid: u32, data_dir: &Path) -> Result<()> {
        // Delete heartbeat before kill so post-restart verification can detect fresh file
        let heartbeat_path = data_dir.join("watcher-heartbeat");
        let _ = fs::remove_file(&heartbeat_path);

        // Send SIGTERM
        let ret = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
        if ret != 0 {
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
            if errno == libc::ESRCH {
                // Process already gone — clean up PID file
                let _ = fs::remove_file(data_dir.join("watcher.pid"));
                return Ok(());
            }
            if errno == libc::EPERM {
                bail!("Permission denied: cannot signal watcher (PID {pid})");
            }
            bail!("kill({pid}, SIGTERM) failed with errno {errno}");
        }

        tracing::info!(
            pid,
            "sent SIGTERM to watcher, waiting up to {SIGTERM_GRACE_SECS}s"
        );

        // Wait for process to exit
        let deadline =
            std::time::Instant::now() + std::time::Duration::from_secs(SIGTERM_GRACE_SECS);
        loop {
            if !is_pid_alive(pid) {
                tracing::info!(pid, "watcher exited after SIGTERM");
                break;
            }
            if std::time::Instant::now() >= deadline {
                tracing::warn!(
                    pid,
                    "watcher didn't exit after {SIGTERM_GRACE_SECS}s, sending SIGKILL"
                );
                unsafe { libc::kill(pid as i32, libc::SIGKILL) };
                // Give a moment for SIGKILL to take effect
                std::thread::sleep(std::time::Duration::from_millis(500));
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }

        // Clean up PID file
        let _ = fs::remove_file(data_dir.join("watcher.pid"));
        Ok(())
    }

    // ── Log rotation ──────────────────────────────────────────────────────

    /// Rotate the watcher log if it exceeds `LOG_MAX_BYTES` using copytruncate
    /// semantics (preserves launchd file descriptor).
    pub fn rotate_log_if_needed(log_path: &Path) -> Result<()> {
        let metadata = match fs::metadata(log_path) {
            Ok(m) => m,
            Err(_) => return Ok(()), // No log file, nothing to rotate
        };

        if metadata.len() <= LOG_MAX_BYTES {
            return Ok(());
        }

        // Copy to .1 backup
        let rotated = log_path.with_extension("log.1");
        fs::copy(log_path, &rotated)
            .with_context(|| format!("failed to copy log to {}", rotated.display()))?;

        // Truncate in-place (preserves fd for launchd)
        let file = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(log_path)
            .with_context(|| format!("failed to truncate {}", log_path.display()))?;
        drop(file);

        tracing::info!(
            path = %log_path.display(),
            size_mb = metadata.len() / (1024 * 1024),
            "rotated log file"
        );
        Ok(())
    }

    // ── Lockfile ──────────────────────────────────────────────────────────

    /// Acquire an advisory lock on the watchdog lockfile.
    /// Returns the `File` handle — caller must keep it alive for the lock duration.
    /// Returns `Err` if another watchdog instance holds the lock.
    pub fn acquire_lock(data_dir: &Path) -> Result<fs::File> {
        use std::os::unix::io::AsRawFd;

        let lock_path = data_dir.join("watchdog.lock");
        let file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)
            .with_context(|| format!("failed to open lockfile {}", lock_path.display()))?;

        // SAFETY: flock with LOCK_EX | LOCK_NB is a standard POSIX advisory lock.
        let ret = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if ret == 0 {
            // Write our PID for debugging
            let mut f = &file;
            let _ = writeln!(f, "{}", std::process::id());
            Ok(file)
        } else {
            // Prefix with "contention:" so callers can distinguish this
            // from real I/O errors (permission denied, disk full, etc.).
            bail!("contention: another watchdog instance is already running")
        }
    }

    // ── Core health check ─────────────────────────────────────────────────

    /// Run the full watchdog health check: lock → rotate log → check heartbeat → restart if stale.
    pub fn run_health_check(data_dir: &Path) -> WatchdogResult {
        // 1. Acquire lock
        let _lock_guard = match acquire_lock(data_dir) {
            Ok(f) => f,
            Err(e) => {
                let msg = e.to_string();
                if msg.starts_with("contention:") {
                    // Another watchdog instance is holding the lock — expected scenario.
                    return WatchdogResult::AlreadyLocked;
                }
                // Real I/O error (permission denied, disk full, etc.) — not contention.
                return WatchdogResult::Error(format!("watchdog lock error: {e}"));
            }
        };

        // 2. Rotate log if needed
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        let log_path = home.join("Library/Logs/cass-index-watch.log");
        if let Err(e) = rotate_log_if_needed(&log_path) {
            tracing::warn!("log rotation failed: {e}");
        }

        // 3. Check PID file
        let pid = match read_pid_file(data_dir) {
            Some(p) => p,
            None => return WatchdogResult::NotRunning,
        };

        // 4. Check if process exists
        if !is_pid_alive(pid) {
            // Stale PID file — clean up
            let _ = fs::remove_file(data_dir.join("watcher.pid"));
            return WatchdogResult::NotRunning;
        }

        // 5. Verify it's actually cass index --watch
        if !verify_pid_is_watcher(pid) {
            // PID reuse — not our process
            let _ = fs::remove_file(data_dir.join("watcher.pid"));
            return WatchdogResult::NotRunning;
        }

        // 6. Check heartbeat age
        let age = match check_heartbeat(data_dir) {
            Some(a) => a,
            None => {
                // No heartbeat file but process is alive — treat as healthy
                // (watcher may have just started and not written first heartbeat yet)
                return WatchdogResult::Healthy;
            }
        };

        if !is_heartbeat_stale(age) {
            return WatchdogResult::Healthy;
        }

        // 7. Heartbeat is stale — kill and rely on launchd KeepAlive to restart
        tracing::warn!(
            pid,
            age_secs = age,
            threshold = HEARTBEAT_THRESHOLD_SECS,
            "watcher heartbeat is stale, restarting"
        );

        if let Err(e) = kill_watcher(pid, data_dir) {
            return WatchdogResult::Error(format!("failed to kill stale watcher: {e}"));
        }

        WatchdogResult::Restarted {
            was_stale_secs: age,
        }
    }

    // ── Plist generation ──────────────────────────────────────────────────

    /// Generate the watcher launchd plist XML.
    pub fn generate_watcher_plist(binary_path: &str, home: &str) -> String {
        let binary_dir = Path::new(binary_path)
            .parent()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
{PLIST_MARKER}
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{WATCHER_LABEL}</string>
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
"#
        )
    }

    /// Generate the watchdog launchd plist XML.
    pub fn generate_watchdog_plist(binary_path: &str, home: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
{PLIST_MARKER}
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{WATCHDOG_LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{binary_path}</string>
        <string>watchdog</string>
        <string>run</string>
    </array>
    <key>StartInterval</key>
    <integer>{WATCHDOG_INTERVAL_SECS}</integer>
    <key>StandardOutPath</key>
    <string>{home}/Library/Logs/cass-watchdog.log</string>
    <key>StandardErrorPath</key>
    <string>{home}/Library/Logs/cass-watchdog.log</string>
</dict>
</plist>
"#
        )
    }

    /// Check if a plist file contains the cass-managed marker.
    pub fn is_cass_managed(plist_path: &Path) -> bool {
        fs::read_to_string(plist_path)
            .map(|content| content.contains(PLIST_MARKER))
            .unwrap_or(false)
    }

    // ── Install decision logic ──────────────────────────────────────────

    /// Decision outcome for a single plist file during install.
    #[derive(Debug, PartialEq, Eq)]
    pub enum InstallDecision {
        /// No existing plist — write a new one.
        WriteNew,
        /// Existing cass-managed plist — overwrite silently.
        OverwriteManaged,
        /// Existing hand-written plist + --force — overwrite with warning.
        OverwriteForced,
        /// Existing hand-written plist, no --force — block with error.
        BlockNotManaged,
    }

    /// Decide what to do for a single plist file (R6 + R9 decision tree).
    /// Pure logic — no I/O, no side effects.
    pub fn decide_install(plist_exists: bool, cass_managed: bool, force: bool) -> InstallDecision {
        if !plist_exists {
            InstallDecision::WriteNew
        } else if cass_managed {
            InstallDecision::OverwriteManaged
        } else if force {
            InstallDecision::OverwriteForced
        } else {
            InstallDecision::BlockNotManaged
        }
    }

    // ── Install / Uninstall ───────────────────────────────────────────────

    /// Install and load a launchd plist using the idempotent bootout→write→bootstrap flow.
    fn install_and_load(plist_path: &Path, label: &str) -> Result<()> {
        let uid = unsafe { libc::getuid() };
        let domain = format!("gui/{uid}");

        // 1. Bootout if already loaded (ignore errors — may not be loaded)
        let _ = std::process::Command::new("launchctl")
            .args(["bootout", &format!("{domain}/{label}")])
            .output();

        // 2. Bootstrap (load) the new plist
        let output = std::process::Command::new("launchctl")
            .args(["bootstrap", &domain, &plist_path.display().to_string()])
            .output()
            .with_context(|| "failed to run launchctl bootstrap")?;

        if !output.status.success() {
            // Fallback: try legacy `launchctl load` for older macOS
            let fallback = std::process::Command::new("launchctl")
                .args(["load", &plist_path.display().to_string()])
                .output()
                .with_context(|| "failed to run launchctl load")?;

            if !fallback.status.success() {
                let stderr = String::from_utf8_lossy(&fallback.stderr);
                bail!(
                    "Failed to load plist {}: bootstrap and load both failed. {}",
                    plist_path.display(),
                    stderr
                );
            }
        }
        Ok(())
    }

    /// Install launchd plists for watcher + watchdog.
    pub fn install_plists(binary_path: Option<PathBuf>, force: bool) -> Result<()> {
        let binary = match binary_path {
            Some(p) => {
                if !p.exists() {
                    bail!("specified binary path does not exist: {}", p.display());
                }
                p.display().to_string()
            }
            None => {
                let p = which::which("cass")
                    .with_context(|| "cass not found in PATH. Use --binary-path to specify.")?;
                p.display().to_string()
            }
        };

        let home = dirs::home_dir().with_context(|| "cannot determine home directory")?;
        let home_str = home.display().to_string();
        let launch_agents = home.join("Library/LaunchAgents");
        fs::create_dir_all(&launch_agents)
            .with_context(|| format!("failed to create {}", launch_agents.display()))?;

        let plists = [
            (
                launch_agents.join(format!("{WATCHER_LABEL}.plist")),
                WATCHER_LABEL,
                generate_watcher_plist(&binary, &home_str),
            ),
            (
                launch_agents.join(format!("{WATCHDOG_LABEL}.plist")),
                WATCHDOG_LABEL,
                generate_watchdog_plist(&binary, &home_str),
            ),
        ];

        for (plist_path, label, content) in &plists {
            let decision = decide_install(
                plist_path.exists(),
                plist_path.exists() && is_cass_managed(plist_path),
                force,
            );

            match decision {
                InstallDecision::WriteNew => {}
                InstallDecision::OverwriteManaged => {
                    tracing::info!(label, "updating cass-managed plist");
                }
                InstallDecision::OverwriteForced => {
                    eprintln!("⚠ Overwriting hand-written plist: {}", plist_path.display());
                }
                InstallDecision::BlockNotManaged => {
                    bail!(
                        "Existing plist not managed by cass: {}. Use --force to overwrite.",
                        plist_path.display()
                    );
                }
            }

            fs::write(plist_path, content)
                .with_context(|| format!("failed to write {}", plist_path.display()))?;
            install_and_load(plist_path, label)?;
            println!("✓ Installed and loaded {label}");
        }

        Ok(())
    }

    /// Uninstall (unload + remove) both launchd plists.
    pub fn uninstall_plists() -> Result<()> {
        let home = dirs::home_dir().with_context(|| "cannot determine home directory")?;
        let launch_agents = home.join("Library/LaunchAgents");
        let uid = unsafe { libc::getuid() };
        let domain = format!("gui/{uid}");

        let labels = [WATCHER_LABEL, WATCHDOG_LABEL];
        for label in &labels {
            // 1. Bootout (ignore errors — may not be loaded)
            let _ = std::process::Command::new("launchctl")
                .args(["bootout", &format!("{domain}/{label}")])
                .output();

            // 2. Remove plist file
            let plist_path = launch_agents.join(format!("{label}.plist"));
            if plist_path.exists() {
                fs::remove_file(&plist_path)
                    .with_context(|| format!("failed to remove {}", plist_path.display()))?;
                println!("✓ Removed {label}");
            } else {
                println!("  {label} — plist not found, skipping");
            }
        }

        Ok(())
    }

    // ── Dispatch ──────────────────────────────────────────────────────────

    /// Entry point: dispatch watchdog subcommands.
    pub fn run_watchdog_command(command: Option<WatchdogCommand>) -> Result<()> {
        let data_dir = crate::default_data_dir();

        match command.unwrap_or(WatchdogCommand::Run) {
            WatchdogCommand::Run => {
                let result = run_health_check(&data_dir);
                let code = result.exit_code();
                match &result {
                    WatchdogResult::Healthy => println!("✓ Watcher is healthy"),
                    WatchdogResult::Restarted { was_stale_secs } => {
                        println!(
                            "⚠ Watcher was stale ({was_stale_secs}s), restarted via launchd KeepAlive"
                        );
                    }
                    WatchdogResult::NotRunning => {
                        println!("✗ Watcher is not running");
                    }
                    WatchdogResult::AlreadyLocked => {
                        println!("✓ Another watchdog instance is already running");
                    }
                    WatchdogResult::Error(msg) => {
                        eprintln!("✗ Error: {msg}");
                    }
                }
                if code != 0 {
                    std::process::exit(code);
                }
            }
            WatchdogCommand::Install { binary_path, force } => {
                install_plists(binary_path, force)?;
            }
            WatchdogCommand::Uninstall => {
                uninstall_plists()?;
            }
        }
        Ok(())
    }
}

// ── Non-macOS stub ────────────────────────────────────────────────────────

#[cfg(not(target_os = "macos"))]
mod platform {
    use clap::Subcommand;
    use std::path::PathBuf;

    #[derive(Subcommand, Debug, Clone)]
    pub enum WatchdogCommand {
        /// Run a one-shot health check (heartbeat + log rotation + restart if stale)
        Run,
        /// Install launchd plists for watcher + watchdog
        Install {
            #[arg(long)]
            binary_path: Option<PathBuf>,
            #[arg(long)]
            force: bool,
        },
        /// Remove launchd plists for watcher + watchdog
        Uninstall,
    }

    pub fn run_watchdog_command(_command: Option<WatchdogCommand>) -> anyhow::Result<()> {
        eprintln!("watchdog is only supported on macOS");
        std::process::exit(2);
    }
}

// Re-export platform-specific items at the module level
pub use platform::WatchdogCommand;
pub use platform::run_watchdog_command;

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::platform::*;
    use std::fs;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tempfile::TempDir;

    // ── T5: Heartbeat tests ──────────────────────────────────────────

    #[test]
    fn heartbeat_age_calculation() {
        let dir = TempDir::new().unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        // Write a heartbeat 100 seconds ago
        let heartbeat_path = dir.path().join("watcher-heartbeat");
        fs::write(&heartbeat_path, (now - 100).to_string()).unwrap();

        let age = check_heartbeat(dir.path()).unwrap();
        // Allow 2 seconds of test execution jitter
        assert!((99..=102).contains(&age), "expected ~100, got {age}");
    }

    #[test]
    fn heartbeat_missing_returns_none() {
        let dir = TempDir::new().unwrap();
        assert!(check_heartbeat(dir.path()).is_none());
    }

    #[test]
    fn heartbeat_stale_detection() {
        // Just below threshold → not stale
        assert!(!is_heartbeat_stale(HEARTBEAT_THRESHOLD_SECS));
        assert!(!is_heartbeat_stale(HEARTBEAT_THRESHOLD_SECS - 1));
        // Above threshold → stale
        assert!(is_heartbeat_stale(HEARTBEAT_THRESHOLD_SECS + 1));
        assert!(is_heartbeat_stale(5000));
    }

    // ── T6: PID management tests ─────────────────────────────────────

    #[test]
    fn pid_file_read_write() {
        let dir = TempDir::new().unwrap();
        let pid_path = dir.path().join("watcher.pid");

        // No PID file → None
        assert!(read_pid_file(dir.path()).is_none());

        // Write PID → read it back
        fs::write(&pid_path, "12345").unwrap();
        assert_eq!(read_pid_file(dir.path()), Some(12345));

        // Garbage content → None
        fs::write(&pid_path, "not-a-number").unwrap();
        assert!(read_pid_file(dir.path()).is_none());
    }

    #[test]
    fn pid_stale_detection() {
        // PID 1 (launchd/init) is always alive
        assert!(is_pid_alive(1));
        // Non-existent PID (very high number)
        assert!(!is_pid_alive(4_000_000));
    }

    #[test]
    fn pid_identity_verification() {
        // Our own process should NOT match "cass index --watch"
        let our_pid = std::process::id();
        assert!(!verify_pid_is_watcher(our_pid));
        // Non-existent PID → false
        assert!(!verify_pid_is_watcher(4_000_000));
    }

    #[test]
    fn kill_errno_handling() {
        let dir = TempDir::new().unwrap();
        // Trying to kill a non-existent PID should succeed
        // (kill_watcher handles ESRCH by cleaning up PID file)
        let pid_path = dir.path().join("watcher.pid");
        fs::write(&pid_path, "4000000").unwrap();

        let result = kill_watcher(4_000_000, dir.path());
        assert!(result.is_ok(), "kill of non-existent PID should succeed");
        // PID file should be cleaned up
        assert!(!pid_path.exists(), "PID file should be removed");
    }

    // ── T7: Log rotation test ────────────────────────────────────────

    #[test]
    fn log_rotation_threshold() {
        let dir = TempDir::new().unwrap();
        let log_path = dir.path().join("cass-index-watch.log");

        // Small file → no rotation
        fs::write(&log_path, "small content").unwrap();
        rotate_log_if_needed(&log_path).unwrap();
        assert!(!dir.path().join("cass-index-watch.log.1").exists());

        // File > 100MB → rotation occurs
        {
            let mut f = fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&log_path)
                .unwrap();
            // Write 101MB
            let chunk = vec![b'x'; 1024 * 1024]; // 1MB
            for _ in 0..101 {
                f.write_all(&chunk).unwrap();
            }
        }

        rotate_log_if_needed(&log_path).unwrap();
        let rotated = dir.path().join("cass-index-watch.log.1");
        assert!(rotated.exists(), "rotated file should exist");
        // Original should be truncated
        let meta = fs::metadata(&log_path).unwrap();
        assert_eq!(meta.len(), 0, "original log should be truncated");
        // Rotated should have the content
        let rotated_meta = fs::metadata(&rotated).unwrap();
        assert!(
            rotated_meta.len() > LOG_MAX_BYTES,
            "rotated log should have full content"
        );

        // No log file → no error
        let missing = dir.path().join("nonexistent.log");
        assert!(rotate_log_if_needed(&missing).is_ok());
    }

    // ── T8: Lockfile test ────────────────────────────────────────────

    #[test]
    fn lockfile_prevents_concurrent() {
        let dir = TempDir::new().unwrap();

        // First lock should succeed
        let guard1 = acquire_lock(dir.path()).expect("first lock should succeed");

        // Second lock should fail while first is held
        let result2 = acquire_lock(dir.path());
        assert!(result2.is_err(), "second lock should fail");
        assert!(
            result2.unwrap_err().to_string().contains("already running"),
            "error should mention already running"
        );

        // Explicitly release first lock
        drop(guard1);

        // Small delay to ensure OS has processed the close + unlock
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Third lock should succeed after release
        let _guard3 = acquire_lock(dir.path()).expect("lock after release should succeed");
    }

    // ── T10: Plist generation tests ──────────────────────────────────

    #[test]
    fn plist_generation_correct() {
        let watcher_plist = generate_watcher_plist("/usr/local/bin/cass", "/Users/testuser");
        assert!(watcher_plist.contains(PLIST_MARKER), "must contain marker");
        assert!(watcher_plist.contains("<string>com.cass.index-watch</string>"));
        assert!(watcher_plist.contains("<string>/usr/local/bin/cass</string>"));
        assert!(watcher_plist.contains("<string>index</string>"));
        assert!(watcher_plist.contains("<string>--watch</string>"));
        assert!(
            watcher_plist.contains("<true/>"),
            "KeepAlive should be true"
        );
        assert!(watcher_plist.contains("/Users/testuser/Library/Logs/cass-index-watch.log"));

        let watchdog_plist = generate_watchdog_plist("/usr/local/bin/cass", "/Users/testuser");
        assert!(watchdog_plist.contains(PLIST_MARKER), "must contain marker");
        assert!(watchdog_plist.contains("<string>com.cass.health-watchdog</string>"));
        assert!(watchdog_plist.contains("<string>watchdog</string>"));
        assert!(watchdog_plist.contains("<string>run</string>"));
        assert!(watchdog_plist.contains(&format!("<integer>{WATCHDOG_INTERVAL_SECS}</integer>")));
    }

    #[test]
    fn plist_marker_detection() {
        let dir = TempDir::new().unwrap();
        let plist_path = dir.path().join("test.plist");

        // Cass-managed plist → true
        let managed = "<?xml?><!-- managed by cass --><plist>content</plist>".to_string();
        fs::write(&plist_path, &managed).unwrap();
        assert!(is_cass_managed(&plist_path));

        // Hand-written plist → false
        let hand_written = "<?xml?><plist>content</plist>";
        fs::write(&plist_path, hand_written).unwrap();
        assert!(!is_cass_managed(&plist_path));

        // Non-existent file → false
        assert!(!is_cass_managed(&dir.path().join("nonexistent.plist")));
    }

    // ── T11: Install decision tree tests ─────────────────────────────

    #[test]
    fn install_decision_write_new() {
        // No existing plist → WriteNew
        assert_eq!(
            decide_install(false, false, false),
            InstallDecision::WriteNew
        );
        // Force doesn't matter when file doesn't exist
        assert_eq!(
            decide_install(false, false, true),
            InstallDecision::WriteNew
        );
    }

    #[test]
    fn install_overwrites_cass_managed() {
        // Existing cass-managed plist → overwrite silently, regardless of --force
        assert_eq!(
            decide_install(true, true, false),
            InstallDecision::OverwriteManaged
        );
        assert_eq!(
            decide_install(true, true, true),
            InstallDecision::OverwriteManaged
        );
    }

    #[test]
    fn install_blocks_hand_written_without_force() {
        // Existing hand-written plist, no --force → blocked
        assert_eq!(
            decide_install(true, false, false),
            InstallDecision::BlockNotManaged
        );
    }

    #[test]
    fn install_force_overwrites_hand_written() {
        // Existing hand-written plist + --force → overwrite with warning
        assert_eq!(
            decide_install(true, false, true),
            InstallDecision::OverwriteForced
        );
    }

    // ── T9: WatchdogResult mapping ───────────────────────────────────

    #[test]
    fn watchdog_result_to_exit_code() {
        assert_eq!(WatchdogResult::Healthy.exit_code(), 0);
        assert_eq!(
            WatchdogResult::Restarted {
                was_stale_secs: 3000
            }
            .exit_code(),
            1
        );
        assert_eq!(WatchdogResult::NotRunning.exit_code(), 2);
        assert_eq!(WatchdogResult::AlreadyLocked.exit_code(), 0);
        assert_eq!(WatchdogResult::Error("test".to_string()).exit_code(), 3);
    }

    // ── T12: Uninstall test ──────────────────────────────────────────

    #[test]
    fn uninstall_labels_are_correct() {
        // Verify the plist label constants match the expected naming convention
        assert_eq!(WATCHER_LABEL, "com.cass.index-watch");
        assert_eq!(WATCHDOG_LABEL, "com.cass.health-watchdog");
        // Verify plist filenames derived from labels
        let watcher_filename = format!("{WATCHER_LABEL}.plist");
        let watchdog_filename = format!("{WATCHDOG_LABEL}.plist");
        assert_eq!(watcher_filename, "com.cass.index-watch.plist");
        assert_eq!(watchdog_filename, "com.cass.health-watchdog.plist");
    }

    // ── Health JSON test ─────────────────────────────────────────────

    #[test]
    fn health_includes_watchdog_field() {
        // Test via state_meta_json (requires data dir + db)
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("agent_search.db");

        // Create a minimal SQLite DB
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE conversations (id INTEGER);
             CREATE TABLE messages (id INTEGER);
             CREATE TABLE meta (key TEXT, value TEXT);",
        )
        .unwrap();

        let state = crate::state_meta_json(dir.path(), &db_path, 1800, true);
        let watchdog = state.get("watchdog");
        assert!(
            watchdog.is_some(),
            "state_meta_json should include 'watchdog' key"
        );
        let wd = watchdog.unwrap();
        assert!(wd.get("plist_installed").is_some());
        assert!(wd.get("watcher_plist_installed").is_some());
    }

    // ── T13: run_health_check behavioral tests ────────────────────────

    #[test]
    fn run_health_check_returns_already_locked_when_lock_held() {
        // If the lock is already held (another watchdog instance running),
        // run_health_check must return AlreadyLocked without side effects.
        let dir = TempDir::new().unwrap();
        // Acquire lock in this thread first
        let _guard = acquire_lock(dir.path()).expect("should acquire lock");
        // Now run_health_check on same dir — lock acquisition fails
        let result = run_health_check(dir.path());
        assert_eq!(
            result,
            WatchdogResult::AlreadyLocked,
            "when lock is held, run_health_check must return AlreadyLocked"
        );
    }

    #[test]
    fn run_health_check_returns_not_running_when_no_pid_file() {
        // With no PID file, run_health_check must return NotRunning.
        let dir = TempDir::new().unwrap();
        let result = run_health_check(dir.path());
        assert_eq!(
            result,
            WatchdogResult::NotRunning,
            "without a PID file, run_health_check must return NotRunning"
        );
    }

    #[test]
    fn run_health_check_returns_not_running_for_stale_pid() {
        // With a PID file pointing to a non-existent process,
        // run_health_check must return NotRunning and clean up the PID file.
        let dir = TempDir::new().unwrap();
        let pid_path = dir.path().join("watcher.pid");
        // PID 4_000_000 does not exist
        fs::write(&pid_path, "4000000").unwrap();
        let result = run_health_check(dir.path());
        assert_eq!(
            result,
            WatchdogResult::NotRunning,
            "stale PID (non-existent process) must yield NotRunning"
        );
        assert!(
            !pid_path.exists(),
            "run_health_check must clean up the stale PID file"
        );
    }
}
