---
title: "T10 non-priority connector exclusion not triggered"
date: 2026-05-17T01:56:00Z
bead: coding_agent_session_search-1vxuf
task: T10
result: not-triggered
---

# T10 Non-Priority Connector Exclusion

## Decision

T10 is complete as not triggered.

No non-priority connector blocked priority recovery. The active blocker is the
malformed live archive plus the explicit approval boundary for live promotion,
durable frankensqlite pinning, branch/commit resolution, and watcher proof.

No `cass sources agents exclude <agent> --keep-indexed-data` command was run by
this recovery, so there is no include/restore step to perform for T10.

## Current Exclusion State

Read-only command:

```text
/Users/dalecarman/.local/bin/cass sources agents list --json
```

Result:

```json
{
  "disabled_agents": [],
  "total": 0
}
```

Interpretation: there are no globally excluded agents to restore.

## Evidence Search

Search over the spec 016 evidence, log, receipt, and GoalBuddy state found no
executed source-agent exclusion or inclusion command. The only matches are
route-policy documentation and historical upstream commit messages:

```text
rg -n "sources agents exclude|sources agents include|keep-indexed-data|disabled|excluded" specs/016-cass-recovery-ingestion/evidence specs/016-cass-recovery-ingestion/log.md specs/016-cass-recovery-ingestion/implement-receipt.md docs/goals/cass-session-ingestion-recovery/state.yaml
```

The route-policy lines document what to do if a non-priority connector blocks
priority recovery; they do not show that the fallback was used.

## Consequence

T10 does not reduce the remaining live blockers. Priority canary/recovery and
reconciliation remain blocked until the approval-gated live promotion route can
continue.
