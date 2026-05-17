---
title: "Continuation audit 2026-05-17T04:15:33Z"
date: 2026-05-17T04:15:33Z
bead: coding_agent_session_search-1vxuf
full_outcome_complete: false
---

# Continuation Audit

## Objective Restated

CASS is complete only when it is deliberately reconciled with upstream, Pi
Agent / Claude Code / Codex sessions are processed and searchable in the live
installed CASS system, OpenCode/factory do not regress, and a live watcher keeps
new sessions searchable.

This audit is read-only except for `git fetch upstream main`, which refreshed
the local remote-tracking ref. It did not promote the shadow archive, install a
binary, reload launchd, mutate session roots, commit, push, or resolve the
frankensqlite branch/pin.

## Prompt-To-Artifact Checklist

| Requirement | Current evidence | Result |
| --- | --- | --- |
| Upstream is incorporated or explicitly blocked | `git fetch upstream main`; `HEAD=b807ef175dcdeeb48b912a22913fbcd68fb86cb8`; `upstream/main=5156af7ecbfe3aa757a838ebfd6444d55f647896`; merge-base `3763b33132c78ecb541180f05e1b1dd6ec6719e1`; ahead/behind `19/23`; `upstream/main` is not an ancestor of `HEAD`; `git merge-tree --write-tree HEAD upstream/main` produced tree `95ec000ced664cc83a1d1f8fd8b4d54c7cd3330d`. | Not complete. Upstream is still not incorporated and branch/commit resolution remains approval-gated. |
| Live priority sessions processed/searchable | Live DB read-only query: `pi_agent=1077`, `claude_code=2574`, `codex=5712`, `messages=1055517`. | Not complete. Pi Agent remains below verified shadow count. |
| Live DB safe for writes | Encoded immutable-URI `PRAGMA quick_check(5)` still reports `Freelist: freelist leaf count too big` errors. | Not complete. Live archive remains malformed. |
| Bonus sessions do not regress | Live DB read-only query: `opencode=976`, `factory=66`. Shadow/release canaries still prove bonus search in the repaired archive. | Not complete for live acceptance because live archive is still malformed and unpromoted; no new regression observed. |
| `com.cass.index-watch` loaded/running | `launchctl list | rg 'cass|coding-agent'` shows only `com.cass.health-watchdog` and `com.cass.sync-to-mini`; `launchctl print gui/501/com.cass.index-watch` reports service not found. | Not complete. |
| New/modified session becomes searchable through watcher | No live watcher is loaded, and no live marker proof has run. | Not complete. |
| Health watchdog command surface | Installed `/Users/dalecarman/.local/bin/cass watchdog run --help` still exits `2` with `Could not parse arguments`; release candidate `/tmp/cass-release-target/release/cass watchdog run --help` exits `0`. `launchctl print gui/501/com.cass.health-watchdog` reports loaded/not running, `runs=348`, `last exit code=2`. | Partially repaired in release candidate, not live. |
| Verified approval-gated release candidate | `/tmp/cass-release-target/release/cass` version `cass 0.4.7`, sha256 `a5a139ca503f0f04f2b0baba06e20b16bd7003a7e7d78306cacf94d56eaca9c2`; release watchdog help exits `0`; previous shadow health/search canaries passed after this rebuild. | Ready for approval-gated install testing, not live. |
| No active conflicting CASS worker | `ps` scan found no non-probe `cass index/search/health/doctor/watchdog` or local debug/release worker process. | Clear for now. |
| Capacity for approved promotion | `df -h` shows target volume has `151Gi` available. | Capacity remains adequate, but re-check before promotion. |
| Code verify/finalize/commit/push | `gate.sh record implement` previously refused completion with unchecked live-proof tasks; no code-verify/finalize/commit/push has run. | Not complete. |

## Key Command Evidence

```text
branch=dac/main
HEAD=b807ef175dcdeeb48b912a22913fbcd68fb86cb8
upstream_main=5156af7ecbfe3aa757a838ebfd6444d55f647896
merge_base=3763b33132c78ecb541180f05e1b1dd6ec6719e1
ahead_behind=19 23
upstream_main_ancestor_of_HEAD=no
merge_tree=95ec000ced664cc83a1d1f8fd8b4d54c7cd3330d
```

```text
PRAGMA quick_check(5):
*** in database main ***
Freelist: freelist leaf count too big on page 1241212
Freelist: freelist leaf count too big on page 1241214
Freelist: freelist leaf count too big on page 1241215
Freelist: freelist leaf count too big on page 1241216
Freelist: freelist leaf count too big on page 1241217

claude_code|2574
codex|5712
factory|66
opencode|976
pi_agent|1077
messages|1055517
```

```text
launchctl list | rg 'cass|coding-agent'
- 2 com.cass.health-watchdog
- 1 com.cass.sync-to-mini

launchctl print gui/501/com.cass.index-watch
Bad request.
Could not find service "com.cass.index-watch" in domain for user gui: 501

launchctl print gui/501/com.cass.health-watchdog
state = not running
program = /Users/dalecarman/.local/bin/cass
runs = 348
last exit code = 2
```

```text
/tmp/cass-release-target/release/cass --version
cass 0.4.7

shasum -a 256 /tmp/cass-release-target/release/cass
a5a139ca503f0f04f2b0baba06e20b16bd7003a7e7d78306cacf94d56eaca9c2

/tmp/cass-release-target/release/cass watchdog run --help
exit 0; prints watchdog run help

/Users/dalecarman/.local/bin/cass watchdog run --help
exit 2; stderr: Could not parse arguments
```

## Decision

The goal is not complete. The current live system still fails the user-visible
criteria: malformed live DB, Pi Agent under-indexed live, upstream not
incorporated, `com.cass.index-watch` absent, no live watcher marker proof, no
install, no launchd smoke, and no code-verify/finalize.
