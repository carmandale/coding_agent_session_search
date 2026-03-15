# Shaping Transcript: 007-watchdog-subcommand

**Date:** 2026-03-15
**Participants:** PureYak (proposer, pi/claude-sonnet-4-20250514), OakStorm (challenger, crew-challenger)
**Spec:** specs/007-watchdog-subcommand/spec.md
**Bead:** coding_agent_session_search-2efx

---

## Phase: Challenge (OakStorm — 7 issues)

### 1. `status` overlaps with `cass health` (MEDIUM-HIGH)
`cass health --json` already reports heartbeat staleness. `cass watchdog status` would be a duplicate. Recommendation: drop `status`, fold plist-installed info into `cass health`.

### 2. PID discovery unspecified (HIGH)
The spec mentions `pgrep` and "heartbeat file PID extension" but neither is implemented. `pgrep -f` is fragile (matches itself, substrings). Need a PID file at `<data_dir>/watcher.pid`, written by the watcher on startup.

### 3. `std::env::current_exe()` resolves symlinks (HIGH)
Dangerous for plist generation — resolved symlink path may break when target changes. Recommendation: use `which cass` as default, add `--binary-path` override flag.

### 4. Missing migration path (MEDIUM)
What happens when user has hand-written plists and runs `install`? Needs overwrite policy.

### 5. 4 subcommands → 3 (DESIGN)
Drop `status`, make bare `cass watchdog` the default health check action. Cleaner: `cass watchdog`, `cass watchdog install`, `cass watchdog uninstall`.

### 6. Copytruncate race window (LOW)
<1ms window where lines are lost between cp and truncate. Document, don't solve.

### 7. Concurrent watchdog runs (LOW)
Two launchd-fired watchdog processes could overlap during the 120s SIGTERM wait. Need a lockfile.

---

## Phase: Revision (PureYak)

All 7 accepted:
- Dropped `status` subcommand, folding plist-installed into `cass health`
- PID file at `<data_dir>/watcher.pid`
- `which cass` + `--binary-path` override
- 3 subcommands: bare `cass watchdog`, `install`, `uninstall`
- Lockfile at `<data_dir>/watchdog.lock`
- Copytruncate race documented only

Shapes B (minimal) and C (self-monitoring) proposed alongside A.

---

## Phase: Challenge Round 2 (OakStorm)

### Shape C killed
Self-monitoring is architecturally unsound. A stuck event loop can't detect its own stuckness — deadlocks on shared locks, can't force its own restart. Watchdogs must be external processes.

### Shape B too minimal
Doesn't solve the deployment problem (the #1 issue from the spec).

### R6 refinement — marker-based overwrite
Instead of always requiring `--force`, use a `<!-- managed by cass -->` marker in generated plists:
- Marker present → overwrite silently (cass updating itself)
- No marker → warn, require `--force` (hand-written plists being replaced)

### R9 added — idempotent install
Re-running `install` should update the binary path (if changed) and reload. Makes `install` the single post-update command.

### Gap: exit codes not in R table
Added to R0.

### Gap: migration path for existing deployments
Users with old bash-based plists need `cass watchdog install` to switch. Old plists without marker trigger `--force` prompt.

---

## Phase: Agreement

Shape A selected with all revisions. Shapes B and C dropped.

### Final Requirements

| ID | Requirement | Status |
|----|-------------|--------|
| R0 | `cass watchdog` performs heartbeat check + log rotation + restart (exit 0/1/2) | Core goal |
| R1 | `cass watchdog install` writes and loads both launchd plists | Must-have |
| R2 | `cass watchdog uninstall` unloads and removes both plists | Must-have |
| R3 | PID file for reliable process identification | Must-have |
| R4 | Lockfile prevents concurrent watchdog runs | Must-have |
| R5 | Binary path explicit, overridable via --binary-path | Must-have |
| R6 | Marker-based overwrite: cass-managed → silent, hand-written → require --force | Must-have |
| R7 | Heartbeat threshold, grace period, rotation size are unit-testable | Must-have |
| R8 | macOS only | Constraint |
| R9 | install is idempotent — re-running updates binary path and reloads | Must-have |

### Shape A (selected)

| Part | Mechanism |
|------|-----------|
| A1 | `cass watchdog` — one-shot: heartbeat check + log rotation + SIGTERM-first restart |
| A2 | `cass watchdog install [--force] [--binary-path]` — write + load plists |
| A3 | `cass watchdog uninstall` — unload + remove plists |
| A4 | PID file at `<data_dir>/watcher.pid` |
| A5 | Lockfile at `<data_dir>/watchdog.lock` |
| A6 | `cass health` extended with plist-installed field |
| A7 | Marker `<!-- managed by cass -->` in generated plists for safe overwrite |
