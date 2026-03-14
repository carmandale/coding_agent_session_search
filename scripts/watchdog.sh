#!/bin/bash
# cass health watchdog - heartbeat-based liveness check
# Runs every 10 minutes via launchd to ensure watcher is alive.
#
# Changes from original (spec 005-watcher-cpu-spin):
# - Replaced cass health --json staleness check with heartbeat file age check
# - Changed kill sequence: SIGTERM → wait 120s → SIGKILL (was: launchctl kickstart -k = immediate SIGKILL)
# - Removed concurrent cass index --full (fought watcher for tantivy lock, always failed)
# - Added log rotation with copytruncate semantics (preserves launchd fd)

set -euo pipefail

LOG_TAG="cass-watchdog"

log() {
    logger -t "$LOG_TAG" "$1"
    echo "$(date '+%Y-%m-%d %H:%M:%S') $1"
}

# --- Log rotation (copytruncate) ---
LOG_FILE="$HOME/Library/Logs/cass-index-watch.log"
MAX_LOG_SIZE=$((100 * 1024 * 1024))  # 100 MB

if [ -f "$LOG_FILE" ]; then
    LOG_SIZE=$(stat -f%z "$LOG_FILE" 2>/dev/null || echo 0)
    if [ "$LOG_SIZE" -gt "$MAX_LOG_SIZE" ]; then
        log "Log file ${LOG_SIZE} bytes > ${MAX_LOG_SIZE}, rotating with copytruncate"
        cp "$LOG_FILE" "${LOG_FILE}.1"  # backup recent logs for diagnosis
        : > "$LOG_FILE"                 # truncate in-place (preserves launchd fd)
    fi
fi

# --- Heartbeat-based liveness check ---
# The watcher writes a Unix timestamp to this file every 60s from its event loop.
# If the heartbeat is stale, the event loop is stuck or the process died.
HEARTBEAT_FILE="$HOME/Library/Application Support/com.coding-agent-search.coding-agent-search/watcher-heartbeat"
# Threshold: 2700s = full_scan_interval (1800s) + max_scan_duration (~600s) + margin (300s)
# Must exceed the longest possible legitimate scan to avoid killing busy-but-healthy processes.
HEARTBEAT_THRESHOLD=2700

NEEDS_RESTART=false
REASON=""

if [ -f "$HEARTBEAT_FILE" ]; then
    HEARTBEAT_TS=$(cat "$HEARTBEAT_FILE" 2>/dev/null || echo 0)
    NOW=$(date +%s)
    HEARTBEAT_AGE=$(( NOW - HEARTBEAT_TS ))
    log "Heartbeat check: age=${HEARTBEAT_AGE}s (threshold=${HEARTBEAT_THRESHOLD}s)"

    if [ "$HEARTBEAT_AGE" -gt "$HEARTBEAT_THRESHOLD" ]; then
        NEEDS_RESTART=true
        REASON="heartbeat stale (${HEARTBEAT_AGE}s > ${HEARTBEAT_THRESHOLD}s)"
    fi
else
    # No heartbeat file — watcher may not have started yet or is an older version.
    # Fall back to checking if the process is running at all.
    if ! pgrep -f "cass index --watch" > /dev/null 2>&1; then
        NEEDS_RESTART=true
        REASON="no heartbeat file and no watcher process"
    else
        log "✓ No heartbeat file but watcher process is running (pre-heartbeat version?)"
    fi
fi

if [ "$NEEDS_RESTART" = "true" ]; then
    log "⚠️  Watcher needs restart: $REASON"

    # Graceful shutdown: SIGTERM first, wait, then SIGKILL as last resort.
    # The watcher has a SIGTERM handler that flushes tantivy and exits cleanly.
    WATCHER_PID=$(pgrep -f "cass index --watch" 2>/dev/null || true)

    if [ -n "$WATCHER_PID" ]; then
        log "Sending SIGTERM to watcher (PID $WATCHER_PID)"
        kill -TERM "$WATCHER_PID" 2>/dev/null || true

        # Wait up to 120s for graceful shutdown
        WAIT_SECS=0
        while [ "$WAIT_SECS" -lt 120 ] && kill -0 "$WATCHER_PID" 2>/dev/null; do
            sleep 5
            WAIT_SECS=$((WAIT_SECS + 5))
        done

        if kill -0 "$WATCHER_PID" 2>/dev/null; then
            log "⚠️  Watcher did not exit after 120s, sending SIGKILL"
            kill -9 "$WATCHER_PID" 2>/dev/null || true
            sleep 2
        else
            log "✓ Watcher exited gracefully after ${WAIT_SECS}s"
        fi
    fi

    # launchd KeepAlive will restart the watcher automatically.
    # No need to run cass index --full — the watcher does its own initial indexing.
    sleep 5

    # Verify watcher restarted
    if pgrep -f "cass index --watch" > /dev/null 2>&1; then
        log "✓ Watcher restarted successfully"
    else
        log "⚠️  Watcher did not restart — check launchd configuration"
    fi
else
    log "✓ Watcher healthy, no action needed"
fi
