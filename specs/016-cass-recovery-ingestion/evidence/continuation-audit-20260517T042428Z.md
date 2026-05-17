---
title: "Continuation audit 2026-05-17T04:24:28Z"
date: 2026-05-17T04:24:28Z
bead: coding_agent_session_search-1vxuf
full_outcome_complete: false
---

# Continuation Audit

## Purpose

Refresh the moving live blockers after the release-candidate/runbook refresh, and verify the live SQLite read-only command shape that the approval runbook will use.

No live data, binary, launchd service, session root, git commit, or remote branch was mutated.

## Upstream

```text
git fetch upstream main: upstream/main remained 5156af7ecbfe3aa757a838ebfd6444d55f647896
HEAD: b807ef175dcdeeb48b912a22913fbcd68fb86cb8
upstream/main: 5156af7ecbfe3aa757a838ebfd6444d55f647896
merge-base: 3763b33132c78ecb541180f05e1b1dd6ec6719e1
ahead/behind: 19/23
merge-tree: 95ec000ced664cc83a1d1f8fd8b4d54c7cd3330d
```

## Live SQLite Read-Only Probe

The plain `sqlite3 -readonly "$LIVE_DIR/agent_search.db"` path open failed against the current live database with SQLite code 14:

```text
Error: in prepare, unable to open database file (14)
```

The working read-only command shape is the encoded SQLite URI:

```bash
LIVE_DB="$HOME/Library/Application Support/com.coding-agent-search.coding-agent-search/agent_search.db"
LIVE_DB_RO_URI="file:${LIVE_DB// /%20}?mode=ro"
sqlite3 "$LIVE_DB_RO_URI" 'PRAGMA quick_check(5);'
```

Result:

```text
*** in database main ***
Freelist: freelist leaf count too big on page 1241212
Freelist: freelist leaf count too big on page 1241214
Freelist: freelist leaf count too big on page 1241215
Freelist: freelist leaf count too big on page 1241216
Freelist: freelist leaf count too big on page 1241217
```

Live row counts from the same encoded read-only URI:

```text
claude_code|2574
codex|5712
factory|66
opencode|976
pi_agent|1077
messages|1055517
```

## Launchd

```text
launchctl list: com.cass.health-watchdog present with last exit code 2
launchctl print gui/501/com.cass.index-watch: service not found
launchctl print gui/501/com.cass.health-watchdog: state=not running, runs=348, last exit code=2
```

## Runbook Verification

`specs/016-cass-recovery-ingestion/evidence/live-promotion-runbook.md` was updated to use the encoded `mode=ro` URI shape for the live archive integrity check and restore quick check.

```text
zsh -n /tmp/spec016-runbook-shell-blocks.sh: pass
state.yaml YAML load: pass
git diff --check: pass
br sync --flush-only: nothing to export
```

## Decision

The full outcome remains incomplete. Live CASS still uses a malformed archive, live Pi Agent remains under-indexed, `com.cass.index-watch` remains absent, and branch/dependency/live-promotion actions remain approval-gated.
