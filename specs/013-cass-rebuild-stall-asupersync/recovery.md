---
title: "cass full-rebuild stalls — recovery procedure"
date: 2026-05-13
bead: coding_agent_session_search-3vm6
---

Recovery procedure for /codex-implement Group A (T2 → T5 sequence + T36 + T37 cutback).

> This document is read by the operator running the backfill, not by /codex-implement itself. The agent should reference this file rather than re-deriving the procedure during execution.

## Preconditions

- `dac/main` HEAD is at the Phase T-approved implementation commit (or the plan-approved commit `a7d09255` if running Group A standalone).
- `cass 0.4.2` or newer binary at `~/.local/bin/cass`. Confirm with `cass --version`.
- macOS launchd-managed watcher currently running (`com.cass.index-watch`).
- ~30 GB free on the data dir filesystem (raw-mirror archive growth + WAL).

## T2 — Stop the watcher (and health-watchdog + sync-to-mini)

```bash
launchctl unload ~/Library/LaunchAgents/com.cass.index-watch.plist
launchctl unload ~/Library/LaunchAgents/com.cass.health-watchdog.plist
launchctl unload ~/Library/LaunchAgents/com.cass.sync-to-mini.plist
```

Verify no `cass index` process remains:

```bash
pgrep -fla "cass index" | grep -v "zsh -c" || echo "stopped"
```

## T3 — WAL-safe snapshot of the canonical DB

The watcher is stopped (T2). With no writers active, drain the WAL into the main file, then APFS-clone the triplet aside:

```bash
DB_DIR="/Users/dalecarman/Library/Application Support/com.coding-agent-search.coding-agent-search"
DATE=$(date +%Y%m%d)
sqlite3 "$DB_DIR/agent_search.db" "PRAGMA wal_checkpoint(TRUNCATE);"
cp -c "$DB_DIR/agent_search.db" "$DB_DIR/agent_search.db.PRE-BACKFILL-$DATE"
[ -f "$DB_DIR/agent_search.db-wal" ] && cp -c "$DB_DIR/agent_search.db-wal" "$DB_DIR/agent_search.db-wal.PRE-BACKFILL-$DATE"
[ -f "$DB_DIR/agent_search.db-shm" ] && cp -c "$DB_DIR/agent_search.db-shm" "$DB_DIR/agent_search.db-shm.PRE-BACKFILL-$DATE"
```

Note: after a successful `wal_checkpoint(TRUNCATE)`, the WAL file may be 0 bytes or absent; that's expected.

Sanity-check the snapshot reads:

```bash
sqlite3 "$DB_DIR/agent_search.db.PRE-BACKFILL-$DATE" "SELECT COUNT(*) FROM conversations;"
# compare to:
cass stats | grep Conversations:
```

## T4 — Capture watch_state.json

```bash
cp -c "$DB_DIR/watch_state.json" "$DB_DIR/watch_state.json.PRE-BACKFILL-$DATE"
```

The backfill `cass index --full` run will NOT update `watch_state.json` (the watch path at `src/indexer/mod.rs:16013` is what updates it). After backfill, T35 reconciles it against the DB's `last_scan_ts`.

## T5 — Run the backfill

**MUST use `--json`.** Without it, the watchdog stays silent (`emit_progress_events = structured_output && ...` at `src/lib.rs:72250, 72531`); a hang produces zero diagnostic output. The Group E watchdog-gating fix (T21) removes this gate, but until that lands the `--json` flag is mandatory.

```bash
DATE=$(date +%Y%m%d)
CASS_INDEX_STALL_DETECT_SECS=60 \
  cass index --full --json \
  2>&1 | tee ~/Library/Logs/cass-backfill-$DATE.log
```

Monitor for `stall_detected` events:

```bash
grep "stall_detected" ~/Library/Logs/cass-backfill-$DATE.log
```

### Success path

- Process exits 0.
- `cass stats` shows conversation counts within ≤2% of on-disk source-file counts (A3).
- T19a's reconciliation ledger has zero class-(c) entries (silent loss).
- Proceed to T35 → T36 → T36a → T37.

### Stall path (deadlock reproduces)

- `stall_detected` event in the log carries the diagnostic snapshot (queue depths, current connector, source path; per-thread heartbeat counters; lldb backtrace if `CASS_INDEX_STALL_CAPTURE_LLDB=1`).
- Kill the process: `pkill -TERM -f "cass index --full"`.
- Capture the structured event for Group C diagnosis.
- Decision point:
  - **Retry** with `CASS_STREAMING_CONSUMER_COMBINE=0` (Group C T13) to isolate flat-combine.
  - **Pivot to Surface 5** with `CASS_STREAMING_INDEX=0 cass index --full --json` (slower but no streaming pipeline).
  - **Pause and feed evidence into Group D** (targeted fix).

## Group D fix application

After Group D commits land (the targeted deadlock fix), re-run T5. The fix is verified successful by:

- A2: full backfill completes end-to-end with zero intervention.
- A3: per-connector counts within ≤2% of on-disk source files.
- A5: regression test in `tests/streaming_deadlock_regression.rs` fails on `29c3672a` baseline, passes after fix.

## T35 — Post-backfill `watch_state.json` reconcile

Project rule §2 forbids deletion without written permission, so this is rename-aside, not delete:

```bash
DATE=$(date +%Y%m%d)
mv "$DB_DIR/watch_state.json" "$DB_DIR/watch_state.json.POST-BACKFILL-$DATE"

# Read post-backfill last_scan_ts from DB:
LAST_SCAN_TS=$(sqlite3 "$DB_DIR/agent_search.db" "SELECT value FROM meta WHERE key='last_scan_ts';")

# Write new watch_state.json with every ConnectorKind set to LAST_SCAN_TS:
python3 - <<PY
import json
last = int(${LAST_SCAN_TS:-0})
# ConnectorKind short codes from src/indexer/mod.rs:
kinds = ["cx","cl","gm","cd","cb","vb","am","oc","fc","kp","ws","ai","ch","cp","cu","ti","tw","hr"]
state = {"v": 1, "m": {k: last for k in kinds}}
print(json.dumps(state))
PY > "$DB_DIR/watch_state.json"
```

Verify it parses:

```bash
python3 -c 'import json; json.load(open("'$DB_DIR'/watch_state.json"))'
```

## T36 — Reload launchd agents (verify plist mutation first)

After Group E T23's plist mutation lands, verify the dictionary `KeepAlive` form:

```bash
plutil -p ~/Library/LaunchAgents/com.cass.index-watch.plist | grep -A1 KeepAlive
# Expected: "KeepAlive" => { "SuccessfulExit" => 0 }
# NOT:      "KeepAlive" => 1
```

Reload via bootout+bootstrap (idempotent):

```bash
for plist in com.cass.index-watch.plist com.cass.health-watchdog.plist com.cass.sync-to-mini.plist; do
  launchctl bootout gui/$(id -u)/${plist%.plist} 2>/dev/null || true
  launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/$plist
done
launchctl list | grep cass
```

## T36a — C4 acceptance test

Simulate a stall to prove the `KeepAlive={SuccessfulExit=false}` semantics don't respawn:

```bash
# Write synthetic sentinel (mode 0600):
echo '{"ts": "'$(date -Iseconds)'", "event": "stall_detected", "synthetic": true}' \
  > "$DB_DIR/stall-detected.json.tmp"
chmod 0600 "$DB_DIR/stall-detected.json.tmp"
mv "$DB_DIR/stall-detected.json.tmp" "$DB_DIR/stall-detected.json"

# Kill the watcher:
launchctl kill TERM gui/$(id -u)/com.cass.index-watch

# Wait 60s, then verify:
sleep 60
launchctl list | grep com.cass.index-watch
# PID column should be "-" (not running)
# Last exit status should be 0 (the indexer's exit on detecting sentinel)
```

To recover from the synthetic stall (operator-clear procedure):

```bash
mv "$DB_DIR/stall-detected.json" "$DB_DIR/stall-detected.json.CLEARED-$(date +%Y%m%d-%H%M%S)"
launchctl kickstart -k gui/$(id -u)/com.cass.index-watch
```

## T37 — Health verification

Wait 5 minutes after T36a teardown, then:

```bash
cass status
# Expected: "Healthy"; date-range upper bound advances past T36 start.
grep "stall_detected" ~/Library/Logs/cass-index-watch.log
# Expected: no new events since T36a's synthetic sentinel.
```

## Security / privacy reminder (per T27a/T24)

The backfill log and any `stall-detected.json` payloads may contain absolute paths under `~/`, connector vendor directories with workspace names, and (if `CASS_INDEX_STALL_CAPTURE_LLDB=1`) lldb backtraces with source-file frames. Before sharing diagnostics with upstream `Dicklesworthstone/coding_agent_session_search`:

```bash
scripts/cass-diagnostics-redact.sh ~/Library/Logs/cass-backfill-$DATE.log \
  > ~/Library/Logs/cass-backfill-$DATE.redacted.log
scripts/cass-diagnostics-redact.sh "$DB_DIR/stall-detected.json" \
  > "$DB_DIR/stall-detected.redacted.json"
```

Attach only the redacted versions to `gh issue comment` or upstream PR descriptions.
