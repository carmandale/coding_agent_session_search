---
title: "Continuation audit: 2026-05-17T06:07:21Z"
date: 2026-05-17T06:07:21Z
status: blocked
---

# Continuation Audit

This is a read-only refresh. No live CASS data, installed binary, launchd
service, session root, commit, or push was mutated.

## Objective Check

The objective remains: CASS must be in sync with upstream, priority local
sessions must be processed and searchable in live CASS, and
`com.cass.index-watch` must keep new sessions searchable.

## Current Evidence

```text
HEAD: b807ef175dcdeeb48b912a22913fbcd68fb86cb8
upstream/main: 1f20bd576f2e77a5197783c637fcc771ab9e1867
merge-base: 3763b33132c78ecb541180f05e1b1dd6ec6719e1
ahead/behind: 19/24
merge-tree: 26ec8190e7ef955f263cac17f79eaef43ead9cfb
```

```text
live quick_check sample:
*** in database main ***
Freelist: freelist leaf count too big on page 1241212
Freelist: freelist leaf count too big on page 1241214
Freelist: freelist leaf count too big on page 1241215
Freelist: freelist leaf count too big on page 1241216
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
exit=1
healthy=false
index=stale
checkpoint_completed=false
watch_active=false
recommended_action=Run 'cass index --full' to rebuild the index/database.
```

```text
shadow quick_check: ok
shadow health:
exit=0
healthy=true
index=ready
checkpoint_completed=true
db_matches=true
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
health-watchdog runs=359
health-watchdog last exit code=2
```

## Decision

The goal is still not achieved. Live CASS is malformed and under-indexed, the
priority Pi Agent count is still below the verified shadow count, `index-watch`
is absent, upstream is unresolved, and the approval-gated live mutation remains
blocked.

## Validation Note

The first health probe attempted to store the command exit code in zsh's
read-only `status` variable. It was rerun with `health_rc`; the rerun produced
the installed live health evidence above.
