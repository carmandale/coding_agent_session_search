# Plan: Watcher Reliability Improvements

## Overview

Implement a multi-layered reliability strategy to ensure coding agent sessions are never "lost" for extended periods. The fix addresses all identified root causes through watchdog monitoring, timestamp safety, and aggressive connector-specific scanning.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         LAUNCHD LAYER                               │
├─────────────────────────────────────────────────────────────────────┤
│  com.cass.index-watch.plist     com.cass.health-watchdog.plist     │
│  (KeepAlive=true)               (StartInterval=600) ←── NEW         │
└───────────────┬─────────────────────────────┬───────────────────────┘
                │                             │
                ▼                             ▼
┌───────────────────────────┐   ┌─────────────────────────────────────┐
│     cass index --watch    │   │     cass watchdog (new command)     │
│                           │   │                                     │
│  • FSEvents watcher       │   │  1. cass health --json              │
│  • 5-min heartbeat        │   │  2. Check stale > 10 min?           │
│  • Streaming indexer      │◄──│  3. pkill + restart if unhealthy    │
│                           │   │  4. Full reindex if repeated fail   │
└───────────────────────────┘   └─────────────────────────────────────┘
                │
                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    INDEXER SAFETY IMPROVEMENTS                       │
├─────────────────────────────────────────────────────────────────────┤
│  1. Timestamp advancement AFTER successful commit only              │
│  2. Claude-specific 30-min forced full scan                         │
│  3. Per-connector last-success tracking                             │
│  4. Explicit lock release on panic/error                            │
└─────────────────────────────────────────────────────────────────────┘
```

## Implementation Phases

### Phase 1: Watchdog Launchd Service (High Impact, Quick Win)

Create a simple health watchdog that runs every 10 minutes:

**New file: `~/.local/share/cass/watchdog.sh`**
```bash
#!/bin/bash
# cass health watchdog - auto-restart unhealthy watcher

HEALTH=$(cass health --json 2>/dev/null)
HEALTHY=$(echo "$HEALTH" | jq -r '.healthy')
STALE=$(echo "$HEALTH" | jq -r '.state.index.stale')
AGE=$(echo "$HEALTH" | jq -r '.state.index.age_seconds // 0')

if [ "$HEALTHY" != "true" ] || [ "$AGE" -gt 600 ]; then
    logger -t cass-watchdog "Unhealthy or stale ($AGE sec). Restarting watcher."
    launchctl kickstart -k gui/$(id -u)/com.cass.index-watch
    sleep 5
    cass index --full --json >> ~/Library/Logs/cass-index-watch.log 2>&1
fi
```

**New file: `~/Library/LaunchAgents/com.cass.health-watchdog.plist`**
```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" ...>
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.cass.health-watchdog</string>
    <key>ProgramArguments</key>
    <array>
        <string>/bin/bash</string>
        <string>~/.local/share/cass/watchdog.sh</string>
    </array>
    <key>StartInterval</key>
    <integer>600</integer>
    <key>StandardOutPath</key>
    <string>~/Library/Logs/cass-watchdog.log</string>
    <key>StandardErrorPath</key>
    <string>~/Library/Logs/cass-watchdog.log</string>
</dict>
</plist>
```

**Estimated effort:** 1 hour
**Risk:** Low - external to main codebase

### Phase 2: Fix Timestamp Advancement Race (Root Cause Fix)

Current code in `src/indexer/mod.rs` advances `watch_state.json` timestamps immediately after scan, before verifying the commit succeeded.

**Problem location:** `reindex_paths()` function, lines ~1050-1070

```rust
// CURRENT (BUGGY):
ingest_batch(&mut storage, &mut t_index, &convs, ...)?;
t_index.commit()?;
// ↓ If commit() succeeded but later code fails, timestamp is still advanced
if let Some(ts_val) = ts {
    let mut guard = state.lock()...;
    *entry = (*entry).max(ts_val);
    save_watch_state(&opts.data_dir, &guard)?;  // ← timestamp advanced
}
```

**Fix:**
```rust
// FIXED: Only advance timestamp after ALL operations succeed
let commit_result = t_index.commit();
if commit_result.is_ok() {
    did_commit = true;
    // Now safe to advance timestamp
    if let Some(ts_val) = ts {
        let mut guard = state.lock()...;
        *entry = (*entry).max(ts_val);
        save_watch_state(&opts.data_dir, &guard)?;
    }
} else {
    tracing::error!("commit failed, NOT advancing watch_state");
}
```

**Estimated effort:** 30 minutes
**Risk:** Low - logic change only

### Phase 3: Claude-Specific Aggressive Scanning

Add a per-connector "last successful ingest" timestamp and force a full scan if too long without activity.

**New field in watch loop:**
```rust
struct ConnectorHealth {
    last_success_ts: i64,
    consecutive_empty_scans: u32,
}

// In watch loop heartbeat:
if now - claude_health.last_success_ts > 1800 {  // 30 min
    tracing::warn!("Claude connector stale, forcing full scan");
    // Trigger full scan with since_ts=None for Claude only
}
```

**Estimated effort:** 2 hours
**Risk:** Medium - new state tracking

### Phase 4: Improved Logging & Flushing

Ensure logs are flushed so empty log files don't occur:

```rust
// In indexer startup
tracing::info!("cass watcher starting, version={}", env!("CARGO_PKG_VERSION"));
// Flush immediately
std::io::stderr().flush().ok();
```

Also add structured logging for easier debugging:
```rust
tracing::info!(
    connector = "claude",
    scanned = 5,
    ingested = 3,
    since_ts = 12345,
    "scan_complete"
);
```

**Estimated effort:** 1 hour
**Risk:** Low

## Alternative Approaches Considered

### Alternative 1: Replace FSEvents with Polling
**Rejected:** High CPU usage, doesn't solve the core timestamp desync issue.

### Alternative 2: SQLite-based watch_state (instead of JSON)
**Considered for future:** Would allow transactional updates with DB writes. Overkill for now.

### Alternative 3: Separate process per connector
**Rejected:** Overcomplicated, harder to manage.

## Testing Strategy

1. **Unit test:** Verify timestamp isn't advanced on commit failure
2. **Integration test:** Simulate watcher crash, verify watchdog restarts
3. **Manual test:** Let system run for 24h, verify no lost sessions

## Rollout Plan

1. Deploy watchdog script immediately (Phase 1) - can be done today
2. PR for timestamp fix (Phase 2) - code change
3. PR for aggressive scanning (Phase 3) - larger change
4. Monitor for 1 week before removing old heartbeat code

## References

- **Indexer code:** `src/indexer/mod.rs` (watch_sources, reindex_paths, save_watch_state)
- **Launchd service:** `~/Library/LaunchAgents/com.cass.index-watch.plist`
- **Related issue:** This debugging session (2026-02-18)
