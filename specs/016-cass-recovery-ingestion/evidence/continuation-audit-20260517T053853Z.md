---
title: "Continuation audit: 2026-05-17T05:38:53Z"
date: 2026-05-17T05:38:53Z
bead: coding_agent_session_search-1vxuf
goal_task: T008
status: live-outcome-still-blocked
---

# Continuation Audit

## Purpose

Refresh drift-prone facts while spec 016 remains approval-gated. This audit did
not promote the live CASS archive, install a binary, load launchd jobs, write
session roots, commit, push, or resolve the frankensqlite pin.

## Upstream State

Command shape:

```bash
git fetch upstream main
git fetch origin
git branch --show-current
git rev-parse HEAD
git rev-parse upstream/main
git merge-base HEAD upstream/main
git rev-list --left-right --count HEAD...upstream/main
git merge-tree --write-tree HEAD upstream/main
```

Result:

```text
branch=dac/main
HEAD=b807ef175dcdeeb48b912a22913fbcd68fb86cb8
upstream_main=1f20bd576f2e77a5197783c637fcc771ab9e1867
merge_base=3763b33132c78ecb541180f05e1b1dd6ec6719e1
ahead_behind=19 24
merge_tree=26ec8190e7ef955f263cac17f79eaef43ead9cfb
```

Completion impact: upstream remains unresolved. The branch/commit authorization
blocker still applies.

## Live Archive

Command shape:

```bash
LIVE_DIR="$HOME/Library/Application Support/com.coding-agent-search.coding-agent-search"
LIVE_DB="$LIVE_DIR/agent_search.db"
LIVE_DB_RO_URI="file:${LIVE_DB// /%20}?mode=ro"
sqlite3 "$LIVE_DB_RO_URI" 'PRAGMA quick_check;'
sqlite3 "$LIVE_DB_RO_URI" "SELECT ..."
/Users/dalecarman/.local/bin/cass health --json --data-dir "$LIVE_DIR"
```

Result:

```text
quick_check sample:
*** in database main ***
Freelist: freelist leaf count too big on page 1241212
Freelist: freelist leaf count too big on page 1241214
Freelist: freelist leaf count too big on page 1241215
Freelist: freelist leaf count too big on page 1241216
Freelist: freelist leaf count too big on page 1241217

counts:
claude_code=2574
codex=5712
factory=66
opencode=976
pi_agent=1077
messages=1055517

installed cass health:
healthy=false
status=unhealthy
index.status=stale
checkpoint.completed=false
pending.watch_active=false
recommended_action=Run 'cass index --full' to rebuild the index/database.
```

Completion impact: live CASS remains malformed, stale, and under-indexed. Pi
Agent is still `1077` live rows versus `2076` in the verified shadow archive.

## Shadow Archive

Command shape:

```bash
SHADOW="$HOME/Library/Application Support/com.coding-agent-search.coding-agent-search-spec016-shadow-20260516T2025Z"
sqlite3 "$SHADOW/agent_search.db" 'PRAGMA quick_check;'
sqlite3 "$SHADOW/agent_search.db" "SELECT ..."
/tmp/cass-release-target/release/cass health --json --stale-threshold 86400 --data-dir "$SHADOW"
```

Result:

```text
quick_check=ok

counts:
claude_code=2574
codex=5713
factory=66
opencode=976
pi_agent=2076
messages=1238935

release cass shadow health:
healthy=true
status=healthy
index.status=ready
checkpoint.completed=true
pending.watch_active=false
```

Completion impact: shadow remains good enough for the approval-gated promotion
plan. It is still not the live archive.

## Watcher And Runtime State

Command shape:

```bash
launchctl list | rg 'cass|coding-agent'
launchctl print "gui/$(id -u)/com.cass.index-watch"
launchctl print "gui/$(id -u)/com.cass.health-watchdog"
ps -axo pid,state,rss,%cpu,etime,command | awk '...cass...'
df -h "$LIVE_DIR"
shasum -a 256 /tmp/cass-release-target/release/cass "$HOME/.local/bin/cass.real"
/tmp/cass-release-target/release/cass watchdog run --help
/Users/dalecarman/.local/bin/cass watchdog run --help
```

Result:

```text
launchctl list:
- 2 com.cass.health-watchdog
- 1 com.cass.sync-to-mini

com.cass.index-watch:
Bad request.
Could not find service "com.cass.index-watch" in domain for user gui: 501

com.cass.health-watchdog:
state=not running
program=/Users/dalecarman/.local/bin/cass
runs=356
last exit code=2

cass_processes:
no rows

target free space:
150Gi available on /System/Volumes/Data

binary hashes:
a5a139ca503f0f04f2b0baba06e20b16bd7003a7e7d78306cacf94d56eaca9c2  /tmp/cass-release-target/release/cass
47f0692af0fd6484e82e4b69b5512ba44b82de1d0c10d64b5a171b2ed279e691  /Users/dalecarman/.local/bin/cass.real

watchdog help:
release_watchdog_help_exit=0
installed_watchdog_help_exit=2
```

Completion impact: the live watcher is still absent. The approval-gated release
candidate still contains the repaired watchdog command surface, but it is not
installed, and no launchd smoke has run.

## GoalBuddy Surface

Command shape:

```bash
node .../check-goal-state.mjs docs/goals/cass-session-ingestion-recovery/state.yaml
npx goalbuddy prompt /Users/dalecarman/dev/coding_agent_session_search/docs/goals/cass-session-ingestion-recovery --json
curl -fsS http://goalbuddy.localhost:41737/cass-session-ingestion-recovery/api/board
```

Result:

```text
GoalBuddy checker: ok=true, active_task=T008, task_count=9, errors=[], warnings=[]
prompt: task_id=T008, task_status=active, warnings=[]
board API: activeTask=T008, total=9, active=["T008"]
```

Completion impact: the board remains valid and correctly holds the active PM
maintenance task while live recovery is blocked.

## Decision

`full_outcome_complete` remains `false`.

The next real unblocker is still explicit operator approval:

```text
I approve live CASS promotion, frankensqlite durable fix, and branch/commit resolution.
```
