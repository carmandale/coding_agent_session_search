---
title: "T7 stale-index refresh stop evidence"
date: 2026-05-17T01:46:14Z
bead: coding_agent_session_search-1vxuf
task: T7
result: route-policy-stop
---

# T7 Stale-Index Refresh Stop Evidence

## Decision

T7 is complete only as a route-policy attempt. It does not mean live cass is
healthy or refreshed.

The stale-index refresh was already attempted once against the live data dir and
hit the route-policy stop condition. It was stopped with SIGTERM after resident
memory climbed above the 24 GB ceiling, and paired verification still reported a
stale incomplete lexical checkpoint.

Do not run another live stale-index refresh against the malformed live archive.

## Refresh Attempt

Recorded command:

```text
cass index --json --no-progress-events --data-dir '/Users/dalecarman/Library/Application Support/com.coding-agent-search.coding-agent-search'
```

Evidence files:

```text
specs/016-cass-recovery-ingestion/evidence/runtime-refresh/refresh-lexical-index.cmd
specs/016-cass-recovery-ingestion/evidence/runtime-refresh/refresh-lexical-index.exit
specs/016-cass-recovery-ingestion/evidence/runtime-refresh/refresh-lexical-index.stdout
specs/016-cass-recovery-ingestion/evidence/runtime-refresh/refresh-lexical-index.stderr
specs/016-cass-recovery-ingestion/evidence/runtime-refresh/refresh-lexical-index.ps-samples.txt
specs/016-cass-recovery-ingestion/evidence/runtime-refresh/refresh-lexical-index.stop.cmd
specs/016-cass-recovery-ingestion/evidence/runtime-refresh/refresh-lexical-index.stop.exit
specs/016-cass-recovery-ingestion/evidence/runtime-refresh/verify-refresh-status.stdout
specs/016-cass-recovery-ingestion/evidence/runtime-refresh/verify-refresh-status.exit
specs/016-cass-recovery-ingestion/evidence/runtime-refresh/verify-refresh-health.stdout
specs/016-cass-recovery-ingestion/evidence/runtime-refresh/verify-refresh-health.exit
```

Observed result:

```text
refresh exit: 143
stop command: kill -TERM 29772
stop exit: 0
stdout: empty
stderr: empty
max recorded RSS: 30640864 KB
```

The last process sample shows the recovery-owned process at about 30.6 GB RSS:

```text
29772 ... 30640864 ... cass index --json --no-progress-events --data-dir /Users/dalecarman/Library/Application Support/com.coding-agent-search.coding-agent-search
```

## Paired Verification

Status verification:

```text
verify-refresh-status exit: 0
status: unhealthy
index.status: stale
index.reason: lexical rebuild checkpoint is incomplete
index.checkpoint.present: true
index.checkpoint.completed: false
index.checkpoint.db_matches: true
rebuild.active: false
pending.sessions: 0
pending.watch_active: false
recommended_action: Run 'cass index' to refresh the index
```

Health verification:

```text
verify-refresh-health exit: 1
```

## Current Read-Only Refresh

Fresh read-only evidence on 2026-05-17T01:46:14Z confirms the failed refresh did
not make live cass acceptable:

```text
/Users/dalecarman/.local/bin/cass status --json --robot-meta
exit: 0
status: unhealthy
recommended_action: Run 'cass index' to refresh the index

/Users/dalecarman/.local/bin/cass health --json --robot-meta
exit: 1
status: unhealthy
index.status: stale
index.checkpoint.completed: false
index.checkpoint.db_matches: true
rebuild.active: false
recommended_action: Run 'cass index --full' to rebuild the index/database.
```

Live DB quick-check still reports freelist corruption:

```text
*** in database main ***
Freelist: freelist leaf count too big on page 1241212
Freelist: freelist leaf count too big on page 1241214
Freelist: freelist leaf count too big on page 1241215
Freelist: freelist leaf count too big on page 1241216
Freelist: freelist leaf count too big on page 1241217
```

Live counts remain below the verified shadow archive:

```text
claude_code 2574
codex       5712
factory     66
opencode    976
pi_agent    1077
messages    1055517
```

No existing non-probe cass index, doctor, watchdog, check-target, or
release-target worker matched the process scan:

```text
ps -axo pid,ppid,rss,etime,command | rg '[c]ass index|[c]ass doctor|[c]ass watchdog|/tmp/[c]ass-check-target|/tmp/[c]ass-release-target'
exit: 1
```

`com.cass.index-watch` is still absent:

```text
launchctl print gui/501/com.cass.index-watch
exit: 113
Could not find service "com.cass.index-watch" in domain for user gui: 501
```

## Consequence

The next safe route is still the approval-gated one already documented in the
live-promotion runbook:

1. Make the frankensqlite fix durable.
2. Rebuild the release candidate from durable dependencies.
3. Promote the verified shadow DB/index into the live data dir with timestamped
   backups and no deletion.
4. Install the verified binary.
5. Load `com.cass.index-watch`.
6. Prove a new synthetic Codex marker becomes searchable within 120 seconds.

Without that approval, live T8-T15 and watcher T16-T24 remain blocked.
