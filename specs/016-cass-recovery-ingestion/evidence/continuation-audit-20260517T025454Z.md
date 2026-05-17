---
title: "Continuation audit: live blockers unchanged"
date: 2026-05-17T02:54:54Z
bead: coding_agent_session_search-1vxuf
---

# Continuation Audit: 2026-05-17T02:54:54Z

This is a read-only refresh. It did not promote the shadow archive, install a
binary, load launchd services, mutate live session roots, commit, push, or
change the frankensqlite dependency pin.

## Objective Check

The objective remains: deliberately reconcile with upstream, make Pi Agent,
Claude Code, and Codex sessions searchable in live cass, preserve bonus
OpenCode/factory coverage, and run `com.cass.index-watch` so future sessions
become searchable.

Current result: not complete. The same approval-gated blockers remain.

## Git And Upstream

Command:

```text
git fetch upstream main
git rev-parse HEAD
git rev-parse upstream/main
git merge-base HEAD upstream/main
git rev-list --left-right --count HEAD...upstream/main
git merge-tree --write-tree HEAD upstream/main
```

Result:

```text
HEAD=b807ef175dcdeeb48b912a22913fbcd68fb86cb8
upstream/main=485ff1052b48e8d731a9ca9da03ba1d3dd170a82
merge-base=3763b33132c78ecb541180f05e1b1dd6ec6719e1
ahead/behind=19 22
upstream/main ancestor of HEAD=no
merge-tree exit=0
merge-tree=b0ef9f483fefce743323ab78b857b704dfaa5b13
```

Interpretation: upstream did not move since the prior audit, but incorporation
is still incomplete and branch/commit resolution remains approval-gated.

## Live Archive

Command:

```text
sqlite3 "file:$LIVE/agent_search.db?mode=ro" "PRAGMA quick_check(8); ..."
```

Result:

```text
quick_check sample:
Freelist: freelist leaf count too big on page 1241212
Freelist: freelist leaf count too big on page 1241214
Freelist: freelist leaf count too big on page 1241215
Freelist: freelist leaf count too big on page 1241216
Freelist: freelist leaf count too big on page 1241217
Freelist: freelist leaf count too big on page 1241221
Freelist: freelist leaf count too big on page 1241222
Freelist: freelist leaf count too big on page 1241223

claude_code=2574
codex=5712
factory=66
opencode=976
pi_agent=1077
messages=1055517
```

Interpretation: the live archive is still malformed and Pi Agent remains
under-indexed live. The verified shadow archive has not been promoted.

## Launchd And Processes

Command:

```text
launchctl print gui/$(id -u)/com.cass.index-watch
launchctl print gui/$(id -u)/com.cass.health-watchdog
launchctl list | rg 'cass|coding-agent'
ps -axo pid,state,rss,%cpu,etime,command | rg 'cass index|cass search|cass health|cass doctor|cass watchdog|/tmp/cass-check-target|/tmp/cass-release-target|target/(debug|release)/cass'
```

Result:

```text
com.cass.index-watch: service not found in gui/501
com.cass.health-watchdog: loaded, not running, last exit code 2, runs=340
launchctl list: com.cass.health-watchdog and com.cass.sync-to-mini only
cass worker process scan: no active worker matched beyond the ps/rg probe itself
```

Interpretation: the required index watcher is still absent. The health watchdog
parse failure continues and remains tracked as follow-up issue
`coding_agent_session_search-2gif2`.

## Capacity And Release Candidate

Command:

```text
df -h "$LIVE_DIR" "$SHADOW_DIR"
shasum -a 256 /tmp/cass-release-target/release/cass
/tmp/cass-release-target/release/cass --version
```

Result:

```text
target volume free space: 171Gi
release candidate sha256: 423e2e4c2920ec74a38a5cb4af1f00de362a4a82e493d342b4891179f4955ada
release candidate version: cass 0.4.7
```

Interpretation: release candidate identity and capacity are unchanged, but the
candidate remains uninstalled.

## Dependency Pin

Command:

```text
git -C /Users/dalecarman/dev/spec014-frankensqlite-fix branch --show-current
git -C /Users/dalecarman/dev/spec014-frankensqlite-fix rev-parse HEAD
git -C /Users/dalecarman/dev/spec014-frankensqlite-fix status --short
rg -n '^fsqlite|^\[patch\."https://github.com/Dicklesworthstone/frankensqlite"\]|path = "\.\./spec014-frankensqlite-fix' Cargo.toml
cargo tree -i 'fsqlite@0.1.3' --edges normal
```

Result:

```text
sibling branch=fix/fts5-vtab-snapshot-via-delta-journal
sibling HEAD=f298dfa25064124374551737780fd7729ad350db
sibling dirty files:
 M crates/fsqlite-pager/src/pager.rs
 M crates/fsqlite-wal/src/wal.rs

Cargo.toml still has active local patch:
[patch."https://github.com/Dicklesworthstone/frankensqlite"]
fsqlite = { path = "../spec014-frankensqlite-fix/crates/fsqlite" }
fsqlite-types = { path = "../spec014-frankensqlite-fix/crates/fsqlite-types" }

resolved graph:
fsqlite v0.1.3 (/Users/dalecarman/dev/spec014-frankensqlite-fix/crates/fsqlite)
└── coding-agent-search v0.4.7
```

Interpretation: the durable frankensqlite blocker remains. The next non-read-only
dependency step still requires explicit approval to commit/push the sibling fix
and replace the local path patch with a durable revision or agreed pin.

## Decision

Keep GoalBuddy task `T005` blocked. Do not run `$code-verify`, `$finalize`, live
promotion, watcher proof, commit, or push until the explicit approval gate is
satisfied.
