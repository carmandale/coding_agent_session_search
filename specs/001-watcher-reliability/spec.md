---
title: "fix: Improve Watcher Reliability to Prevent Lost Sessions"
type: fix
status: active
date: 2026-02-18
---

# fix: Improve Watcher Reliability to Prevent Lost Sessions

## Problem Statement

Claude Code sessions (and potentially other connectors) periodically stop being indexed, even though:
1. The watcher process is running (launchd `KeepAlive=true`)
2. Session files exist on disk
3. `watch_state.json` shows recent timestamps

Users discover this only when running `gj last` and noticing missing sessions. The only fix is a manual full reindex.

### Root Causes Identified

| Issue | Impact | Evidence |
|-------|--------|----------|
| **FSEvents unreliability** | macOS FSEvents misses file changes (especially Dropbox folders, sleep/wake) | 5-minute heartbeat exists but isn't enough |
| **Watcher silent failure** | Process runs but stops logging/indexing | Empty log file (0 bytes) after 45+ hours of runtime |
| **watch_state desync** | Timestamps advance before successful DB commit | Sessions skipped permanently until full reindex |
| **No health monitoring** | Stale index goes undetected | `cass health` exists but nothing runs it automatically |
| **Tantivy lock issues** | Failed lock acquisition fails silently | `LockFailure(LockBusy...)` errors in logs |

## Success Criteria

### Must Have
- [ ] Sessions are never "lost" for more than 30 minutes
- [ ] Automatic recovery from watcher failures without user intervention
- [ ] Clear logging of what's being indexed and any failures

### Should Have
- [ ] Health status visible in TUI status bar
- [ ] Alert mechanism when sessions are stale (e.g., macOS notification)

### Nice to Have
- [ ] Per-connector health metrics
- [ ] Automatic `--full` reindex after N consecutive failures

## Acceptance Scenarios

### Scenario 1: FSEvents Failure Recovery
**Given** the watcher is running and FSEvents stops delivering events  
**When** the 5-minute heartbeat fires  
**Then** all recent sessions are discovered and indexed

### Scenario 2: Watcher Process Failure
**Given** the watcher process crashes or hangs  
**When** the health watchdog detects unhealthy state  
**Then** the watcher is restarted and a full reindex is triggered

### Scenario 3: watch_state Desync Prevention
**Given** a session file is being indexed  
**When** the DB commit fails for any reason  
**Then** the watch_state timestamp is NOT advanced, ensuring retry on next scan

### Scenario 4: Claude-Specific Forced Scan
**Given** Claude Code is a high-value, high-failure connector  
**When** 30 minutes pass without any Claude sessions indexed  
**Then** a forced full scan of `~/.claude/projects` is triggered

### Scenario 5: Health Visibility
**Given** a user is in the cass TUI  
**When** the index becomes stale (>5 min)  
**Then** a warning indicator appears in the status bar
