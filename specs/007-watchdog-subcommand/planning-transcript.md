# Planning Transcript: 007-watchdog-subcommand

**Date:** 2026-03-15
**Participants:** PureYak (proposer, pi/claude-sonnet-4-20250514), SwiftJaguar (challenger, crew-challenger)
**Bead:** coding_agent_session_search-2efx

---

## Key findings from challenger

1. **kill() errno handling** — must distinguish ESRCH (stale PID) from EPERM (permission denied). Plan now includes explicit errno match.
2. **PID recycling** — mitigated by heartbeat freshness check. If PID exists but heartbeat is stale, treated as stuck.
3. **Lock fd lifetime** — File handle must survive entire watchdog run via `_lock_guard` binding.
4. **which at install-time** — binary path resolved during `cass watchdog install`, not during watchdog runtime (launchd has minimal PATH).
5. **HOME at install-time** — all plist paths fully resolved, no shell variable expansion.
6. **Heartbeat deletion before kill** — delete stale heartbeat before SIGTERM so post-restart verification sees fresh file.
7. **WatchdogResult enum** — adopted for testability. Maps to exit codes at CLI dispatch boundary.
