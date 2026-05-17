---
title: "feat: cass watchdog subcommand — replace external bash script"
date: 2026-03-15
bead: coding_agent_session_search-2efx
shaping: true
---

# cass watchdog subcommand

## User Story

**As a user who installs cass on a new machine,** I want `cass watchdog install`
to set up both the watcher daemon and the health watchdog automatically — no
manual script deployment, no hardcoded paths, no bash scripts to maintain
separately from the binary. After any cass update, I re-run
`cass watchdog install` and everything is current.

## Problem

The watchdog is a 104-line bash script (`scripts/watchdog.sh`) deployed
externally to `~/.local/share/cass/watchdog.sh`. Problems:

1. **Manual deployment** — must `cp` after every update. New machines have no watchdog.
2. **Hardcoded paths** — bash script hardcodes macOS-specific heartbeat path.
   Rust binary computes it at runtime via `directories::ProjectDirs`.
3. **Two manual plists** — neither version-controlled nor self-installable.
4. **Untestable** — threshold math, kill logic, rotation size are unverified.
5. **PID fragility** — uses `pgrep -f` which matches substrings and itself.
6. **No concurrency guard** — two watchdog invocations can overlap during
   the 120s SIGTERM wait period.

---

## Requirements (R)

| ID | Requirement | Status |
|----|-------------|--------|
| R0 | `cass watchdog` performs heartbeat check + log rotation + restart (exit 0=healthy, 1=restarted, 2=not running) | Core goal |
| R1 | `cass watchdog install` writes and loads both launchd plists | Must-have |
| R2 | `cass watchdog uninstall` unloads and removes both plists | Must-have |
| R3 | PID file at `<data_dir>/watcher.pid` for reliable process identification | Must-have |
| R4 | Lockfile at `<data_dir>/watchdog.lock` prevents concurrent runs | Must-have |
| R5 | Binary path explicit (`which cass`), overridable via `--binary-path` | Must-have |
| R6 | Marker-based overwrite: cass-managed plists → silent overwrite, hand-written → require `--force` | Must-have |
| R7 | Heartbeat threshold, SIGTERM grace period, rotation size are unit-testable constants | Must-have |
| R8 | macOS launchd only (no Linux/Windows in this spec) | Constraint |
| R9 | `install` is idempotent — re-running updates binary path and reloads | Must-have |

---

## Shape A: Full watchdog subcommand (selected)

| Part | Mechanism |
|------|-----------|
| **A1** | `cass watchdog` — one-shot: read heartbeat file age, copytruncate log if >100MB, SIGTERM→120s→SIGKILL if stale |
| **A2** | `cass watchdog install [--force] [--binary-path <path>]` — generate and load both launchd plists |
| **A3** | `cass watchdog uninstall` — unload and remove both plists |
| **A4** | Watcher writes PID to `<data_dir>/watcher.pid` on startup (in `watch_sources`) |
| **A5** | Watchdog acquires advisory lock on `<data_dir>/watchdog.lock` at start; exits if locked |
| **A6** | `cass health --json` extended with `watchdog.plist_installed` field |
| **A7** | Generated plists include `<!-- managed by cass -->` marker for safe overwrite detection |

### Dropped shapes

**Shape B (minimal — just replace bash):** Doesn't solve deployment problem.
**Shape C (self-monitoring watcher):** Architecturally unsound — a stuck event
loop can't detect its own stuckness.

---

## Acceptance Criteria

- [ ] `cass watchdog` performs heartbeat check + log rotation + restart if stale
- [ ] `cass watchdog install` creates both launchd plists with correct binary path
- [ ] `cass watchdog install` on existing cass-managed plists overwrites silently
- [ ] `cass watchdog install` on hand-written plists warns and requires `--force`
- [ ] `cass watchdog install` is idempotent (safe to re-run after binary update)
- [ ] `cass watchdog uninstall` removes both plists
- [ ] PID file written by watcher, read by watchdog (no pgrep)
- [ ] Lockfile prevents concurrent watchdog runs
- [ ] Unit tests for threshold math, rotation decision, heartbeat staleness
- [ ] All existing tests pass
- [ ] `scripts/watchdog.sh` kept as reference but no longer required

## Technical Notes

### CLI pattern
Add `Watchdog` variant to `Commands` enum (src/lib.rs:94) with nested
`WatchdogCommands` sub-enum (same pattern as `Sources`, `Models`).

### New file
`src/watchdog.rs` — heartbeat check, PID management, log rotation,
plist generation, install/uninstall logic. Keep `src/lib.rs` handler thin.

### PID file lifecycle
- Written by watcher in `watch_sources` (src/indexer/mod.rs) immediately
  after startup banner, before entering event loop
- Deleted by watcher on clean shutdown (in SIGTERM handler, after loop break)
- Stale PID file (process doesn't exist) treated as "watcher not running"

### Migration
Users with existing bash-based plists (no `<!-- managed by cass -->` marker)
will see: `"Existing plist detected (not managed by cass). Use --force to overwrite."`
After running `cass watchdog install --force`, the bash watchdog is fully replaced.
