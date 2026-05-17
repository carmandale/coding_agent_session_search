---
title: "T6 current route preflight"
date: 2026-05-17T01:40:00Z
bead: coding_agent_session_search-1vxuf
---

# T6 Current Route Preflight

This refresh is read-only. It does not start indexing, run doctor with `--fix`,
promote the shadow archive, install a binary, load launchd, or mutate live
session roots.

## Route Policy

`specs/016-cass-recovery-ingestion/evidence/route-policy.md` exists and was
inspected before any mutation. The relevant rule is that stale-index refresh is
allowed only when no corruption-shaped blocker supersedes it, and corruption
evidence must stop destructive or writer routes.

## Installed Runtime Status

Command:

```text
/Users/dalecarman/.local/bin/cass status --json --robot-meta
```

Result:

```text
exit=0
status=unhealthy
healthy=false
index_status=stale
reason=lexical rebuild checkpoint is incomplete
checkpoint_completed=false
checkpoint_db_matches=true
rebuild_active=false
pending_sessions=0
watch_active=false
doctor_active=false
doctor_recommended=true
recommended_action=Run 'cass index' to refresh the index
stderr_bytes=0
```

Command:

```text
/Users/dalecarman/.local/bin/cass health --json --robot-meta
```

Result:

```text
exit=1
status=unhealthy
healthy=false
index_status=stale
reason=lexical rebuild checkpoint is incomplete
checkpoint_completed=false
checkpoint_db_matches=true
rebuild_active=false
pending_sessions=0
watch_active=false
recommended_action=Run 'cass index --full' to rebuild the index/database.
stderr_bytes=0
```

## Process Evidence

Command:

```text
ps -axo pid,ppid,rss,etime,command | rg 'cass index|cass doctor|cass watchdog|/tmp/cass-check-target|/tmp/cass-release-target|target/(debug|release)/cass' | rg -v 'rg '
```

Result:

```text
exit=1
matches=0
```

Interpretation: no active cass index, doctor, watchdog, or local test/release
worker was found before any mutation.

## Live Corruption Supersedes Stale Refresh

The same read-only audit still reports live SQLite freelist corruption:

```text
PRAGMA quick_check:
*** in database main ***
Freelist: freelist leaf count too big on page 1241212
Freelist: freelist leaf count too big on page 1241214
Freelist: freelist leaf count too big on page 1241215
...

live rows:
claude_code=2574
codex=5712
factory=66
opencode=976
pi_agent=1077
messages=1055517
```

Therefore the route-policy decision is not to run the recommended live
`cass index` or `cass index --full` command against the malformed archive.
The verified shadow archive remains the safe promotion candidate after explicit
approval.

## Doctor Probe

Command:

```text
/Users/dalecarman/.local/bin/cass doctor --json
```

This was run without `--fix`. It was stopped because the read-only probe itself
became a stall:

```text
pid=13293
elapsed_before_stop=04:37
rss_kb_before_stop=11770512
cpu_before_stop=99.3%
signal=SIGTERM
exit=143
stdout_bytes=0
stderr_bytes=0
```

Interpretation: current installed doctor is not a viable quick preflight on the
malformed live archive. This reinforces the current blocker instead of clearing
it: do not stack more live writers; proceed only through the approval-gated
shadow promotion and verified watcher proof path.
