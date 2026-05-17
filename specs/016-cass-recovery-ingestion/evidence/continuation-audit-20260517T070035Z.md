---
title: "Continuation audit: 2026-05-17T07:00:35Z"
date: 2026-05-17T07:00:35Z
status: blocked
---

# Continuation Audit

This is a read-only T008 refresh. No live CASS data, installed binary,
launchd service, session root, dependency pin, commit, or push was mutated.

## Objective Check

The objective remains: CASS must be in sync with upstream, priority local
sessions must be processed and searchable in live CASS, and
`com.cass.index-watch` must keep new sessions searchable.

## Current Evidence

```text
HEAD: b807ef175dcdeeb48b912a22913fbcd68fb86cb8
branch: dac/main
local upstream/main: 1f20bd576f2e77a5197783c637fcc771ab9e1867
remote upstream/main: 1f20bd576f2e77a5197783c637fcc771ab9e1867
merge-base: 3763b33132c78ecb541180f05e1b1dd6ec6719e1
ahead/behind: 19/24
local upstream/main ancestor of HEAD: no
```

```text
live quick_check sample:
*** in database main ***
Freelist: freelist leaf count too big on page 1241212
Freelist: freelist leaf count too big on page 1241214
Freelist: freelist leaf count too big on page 1241215
Freelist: freelist leaf count too big on page 1241216
Freelist: freelist leaf count too big on page 1241217
Freelist: freelist leaf count too big on page 1241221
Freelist: freelist leaf count too big on page 1241222
```

```text
live rows:
claude_code=2574
codex=5712
factory=66
opencode=976
pi_agent=1077
messages=1055517
```

```text
installed live health:
status=unhealthy
healthy=false
state.index.status=stale
state.index.checkpoint.completed=false
state.index.checkpoint.db_matches=true
state.pending.watch_active=false
recommended_action=Run 'cass index --full' to rebuild the index/database.
```

```text
shadow integrity_check: ok
shadow release health:
status=healthy
healthy=true
state.index.status=ready
state.index.checkpoint.completed=true
state.index.checkpoint.db_matches=true
state.pending.watch_active=false
```

```text
shadow rows:
claude_code=2574
codex=5713
factory=66
opencode=976
pi_agent=2076
messages=1238935
```

```text
launchd:
com.cass.index-watch=absent
com.cass.health-watchdog=not running
health-watchdog runs=364
health-watchdog last exit code=2
```

```text
binary hashes:
release=/tmp/cass-release-target/release/cass
release_sha256=a5a139ca503f0f04f2b0baba06e20b16bd7003a7e7d78306cacf94d56eaca9c2
installed=/Users/dalecarman/.local/bin/cass.real
installed_sha256=47f0692af0fd6484e82e4b69b5512ba44b82de1d0c10d64b5a171b2ed279e691
release_watchdog_run_help_exit=0
installed_watchdog_run_help_exit=2
installed_watchdog_stderr=Could not parse arguments
```

```text
GoalBuddy:
check-goal-state=pass
active_task=T008
absolute_prompt_task=T008
absolute_prompt_warnings=[]
board_api_activeTask=T008
board_api_total=9
```

## Decision

The goal is still not achieved. Live CASS is malformed and under-indexed, the
priority Pi Agent count is still below the verified shadow count, `index-watch`
is absent, upstream is unresolved, installed `cass.real` still lacks the
repaired watchdog command surface, and the approval-gated live mutation remains
blocked.
