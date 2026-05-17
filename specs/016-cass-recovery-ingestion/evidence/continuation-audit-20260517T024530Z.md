---
title: "Read-only continuation audit"
date: 2026-05-17T02:45:30Z
bead: coding_agent_session_search-1vxuf
---

# Read-only continuation audit

This audit refreshed the live blockers without promoting data, installing a
binary, loading launchd services, mutating session roots, committing, or
pushing.

## Git / Upstream

Command:

```text
git fetch upstream main
git rev-parse HEAD
git rev-parse upstream/main
git merge-base HEAD upstream/main
git rev-list --left-right --count HEAD...upstream/main
git merge-base --is-ancestor upstream/main HEAD
git merge-tree --write-tree HEAD upstream/main
```

Result:

```text
HEAD=b807ef175dcdeeb48b912a22913fbcd68fb86cb8
upstream/main=485ff1052b48e8d731a9ca9da03ba1d3dd170a82
merge_base=3763b33132c78ecb541180f05e1b1dd6ec6719e1
ahead_behind=19 22
upstream_ancestor=no
merge_tree=b0ef9f483fefce743323ab78b857b704dfaa5b13
```

Interpretation: upstream did not move since the prior audit, but it remains
unincorporated.

## Live DB

Command:

```text
sqlite3 "file:${LIVE}/agent_search.db?mode=ro" 'PRAGMA quick_check(10);'
sqlite3 "file:${LIVE}/agent_search.db?mode=ro" "SELECT ..."
```

Result:

```text
quick_check_sample:
Freelist: freelist leaf count too big on page 1241212
Freelist: freelist leaf count too big on page 1241214
Freelist: freelist leaf count too big on page 1241215
Freelist: freelist leaf count too big on page 1241216
Freelist: freelist leaf count too big on page 1241217
Freelist: freelist leaf count too big on page 1241221
Freelist: freelist leaf count too big on page 1241222
Freelist: freelist leaf count too big on page 1241223
Freelist: freelist leaf count too big on page 1241228
Freelist: freelist leaf count too big on page 1241229

claude_code=2574
codex=5712
factory=66
opencode=976
pi_agent=1077
messages=1055517
```

Interpretation: live DB is still malformed and live Pi Agent coverage is still
below the verified shadow archive.

## Launchd / Processes

Command:

```text
launchctl list | rg 'cass|coding-agent'
launchctl print gui/$(id -u)/com.cass.index-watch
launchctl print gui/$(id -u)/com.cass.health-watchdog
ps -axo ... | rg 'cass index|cass search|cass health|cass doctor|cass watchdog|...'
```

Result:

```text
launchctl list:
- 2 com.cass.health-watchdog
- 1 com.cass.sync-to-mini

com.cass.index-watch:
Could not find service "com.cass.index-watch" in domain for user gui: 501

com.cass.health-watchdog:
state=not running
program=/Users/dalecarman/.local/bin/cass
arguments=/Users/dalecarman/.local/bin/cass watchdog run
runs=339
last_exit_code=2

process scan:
no matching cass worker processes
```

Interpretation: the required index watcher is still absent. The health
watchdog regression remains a separate nonblocking follow-up unless it
interferes with the index watcher proof.

## Capacity

Command:

```text
df -h live shadow paths
```

Result:

```text
Filesystem=/dev/disk3s5
Size=3.6Ti
Used=3.4Ti
Available=171Gi
Capacity=96%
```

Interpretation: capacity is still enough for the approval-gated promotion shape,
but it should be checked again immediately before promotion.
