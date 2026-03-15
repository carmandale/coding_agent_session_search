---
title: "feat: cass watchdog subcommand — replace external bash script"
date: 2026-03-15
bead: coding_agent_session_search-2efx
---

# cass watchdog subcommand

## User Story

**As a user who installs cass on a new machine,** I want `cass watchdog install`
to set up both the watcher daemon and the health watchdog automatically — no
manual script deployment, no hardcoded paths, no bash scripts to maintain
separately from the binary.

## Problem

The watchdog is currently a 104-line bash script (`scripts/watchdog.sh`)
deployed externally to `~/.local/share/cass/watchdog.sh`. This has several
problems:

1. **Manual deployment** — after every update to the script, someone must
   `cp scripts/watchdog.sh ~/.local/share/cass/watchdog.sh`. On new machines
   the watchdog doesn't exist until someone remembers to deploy it.

2. **Hardcoded paths** — the bash script hardcodes the macOS-specific heartbeat
   file path (`$HOME/Library/Application Support/com.coding-agent-search.../watcher-heartbeat`).
   The Rust binary computes this path at runtime via `directories::ProjectDirs`.

3. **Two launchd plists** — both `com.cass.index-watch.plist` and
   `com.cass.health-watchdog.plist` are manually created. Neither is
   version-controlled or self-installable.

4. **Untestable** — the bash script has no tests. The heartbeat threshold,
   SIGTERM grace period, log rotation size, and kill logic are all unverified.

5. **Platform coupling** — the bash script is macOS-only. Future Linux
   support would need a separate systemd unit, duplicating logic.

## Proposed Solution

Replace the external bash script with a built-in `cass watchdog` subcommand:

```
cass watchdog run       # One-shot health check (replaces watchdog.sh)
cass watchdog install   # Write launchd plists for both watcher + watchdog
cass watchdog uninstall # Remove launchd plists
cass watchdog status    # Show watcher PID, heartbeat age, health
```

### `cass watchdog run`

Does exactly what `watchdog.sh` does today, but in Rust:

1. **Log rotation** — if watcher log > 100 MB, copytruncate
2. **Heartbeat check** — read `<data_dir>/watcher-heartbeat`, check age
   against threshold (2700s). Path resolved via `default_data_dir()`, not
   hardcoded.
3. **Liveness action** — if stale: send SIGTERM, wait 120s, SIGKILL if
   still alive. launchd KeepAlive restarts the watcher.
4. **No concurrent index** — does NOT run `cass index --full` (lesson from
   spec 005: it fights the watcher for the tantivy lock).

Exit codes:
- 0: watcher healthy
- 1: watcher restarted (was stale)
- 2: watcher not running (no heartbeat file, no process)

### `cass watchdog install`

Writes two launchd plists:

1. `~/Library/LaunchAgents/com.cass.index-watch.plist` — runs `cass index --watch`
   with `KeepAlive=true`, log to `~/Library/Logs/cass-index-watch.log`

2. `~/Library/LaunchAgents/com.cass.health-watchdog.plist` — runs
   `cass watchdog run` every 600s with `StartInterval`

Both plists use the actual `cass` binary path (resolved via `which cass` or
`std::env::current_exe()`). No bash. No external scripts.

Then loads them via `launchctl load`.

### `cass watchdog status`

Shows:
- Watcher PID (from `pgrep` or heartbeat file PID extension)
- Heartbeat age
- Watcher CPU usage (from `ps`)
- Whether watchdog plist is installed
- Health verdict (healthy / stale / not running)

## What's In Scope

- `cass watchdog run` — Rust replacement for watchdog.sh
- `cass watchdog install` / `uninstall` — launchd plist management
- `cass watchdog status` — health dashboard
- Unit tests for heartbeat staleness, threshold math, log rotation logic
- macOS launchd support

## What's Out of Scope

- Linux systemd support (future spec)
- Windows service support (future spec)
- Removing the existing `scripts/watchdog.sh` (keep as reference/fallback)

## Constraints

- Must work with the existing heartbeat file format (unix timestamp string)
- Must be backward-compatible with the existing launchd plist labels
  (`com.cass.index-watch`, `com.cass.health-watchdog`)
- Must not require root/sudo
- The watchdog `run` command must be idempotent (safe to call repeatedly)

## Acceptance Criteria

- [ ] `cass watchdog run` performs heartbeat check + log rotation + restart if stale
- [ ] `cass watchdog install` creates both launchd plists and loads them
- [ ] `cass watchdog uninstall` unloads and removes both plists
- [ ] `cass watchdog status` shows health summary
- [ ] Heartbeat threshold, SIGTERM grace period, and log rotation size are testable
- [ ] All existing tests pass
- [ ] The external `watchdog.sh` is no longer required for normal operation

## Technical Notes

### CLI registration pattern

Follows the existing `Commands` enum in `src/lib.rs` (line 94). Add a
`Watchdog` variant with a nested `WatchdogCommands` subcommand enum
(same pattern as `Sources`, `Models`, `Mappings`).

### Key insertion points

| File | Change |
|------|--------|
| `src/lib.rs` | Add `Watchdog` to `Commands` enum, `WatchdogCommands` sub-enum |
| `src/lib.rs` | Add `run_watchdog_*` handler functions |
| `src/lib.rs` or new `src/watchdog.rs` | Heartbeat check, log rotation, plist generation logic |

### Data dir resolution

Use `default_data_dir()` (src/lib.rs:8130) for heartbeat path. Use
`dirs::home_dir()` for launchd plist paths. Use `std::env::current_exe()`
for the cass binary path in generated plists.
