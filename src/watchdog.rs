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
                cmdline.contains("cass")
                    && cmdline.contains("index")
                    && cmdline.contains("--watch")
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

        tracing::info!(pid, "sent SIGTERM to watcher, waiting up to {SIGTERM_GRACE_SECS}s");

        // Wait for process to exit
        let deadline = std::time::Instant::now()
            + std::time::Duration::from_secs(SIGTERM_GRACE_SECS);
        loop {
            if !is_pid_alive(pid) {
                tracing::info!(pid, "watcher exited after SIGTERM");
                break;
            }
            if std::time::Instant::now() >= deadline {
                tracing::warn!(pid, "watcher didn't exit after {SIGTERM_GRACE_SECS}s, sending SIGKILL");
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
        let ret = unsafe {
            libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB)
        };
        if ret == 0 {
            // Write our PID for debugging
            let mut f = &file;
            let _ = writeln!(f, "{}", std::process::id());
            Ok(file)
        } else {
            bail!("another watchdog instance is already running")
        }
    }

    // ── Core health check ─────────────────────────────────────────────────

    /// Run the full watchdog health check: lock → rotate log → check heartbeat → restart if stale.
    pub fn run_health_check(data_dir: &Path) -> WatchdogResult {
        // 1. Acquire lock
        let _lock_guard = match acquire_lock(data_dir) {
            Ok(f) => f,
            Err(_) => return WatchdogResult::AlreadyLocked,
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

        let home = dirs::home_dir()
            .with_context(|| "cannot determine home directory")?;
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
            // Install decision tree (R6 + R9)
            if plist_path.exists() {
                if is_cass_managed(plist_path) {
                    // Cass-managed → overwrite silently
                    tracing::info!(label, "updating cass-managed plist");
                } else if force {
                    // Hand-written + --force → overwrite with warning
                    eprintln!(
                        "⚠ Overwriting hand-written plist: {}",
                        plist_path.display()
                    );
                } else {
                    // Hand-written + no --force → error
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
        let home = dirs::home_dir()
            .with_context(|| "cannot determine home directory")?;
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
                        println!("⚠ Watcher was stale ({was_stale_secs}s), restarted via launchd KeepAlive");
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
            WatchdogCommand::Install {
                binary_path,
                force,
            } => {
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
