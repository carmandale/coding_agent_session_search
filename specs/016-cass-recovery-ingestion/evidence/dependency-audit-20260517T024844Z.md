---
title: "Dependency audit: frankensqlite local patch"
date: 2026-05-17T02:48:44Z
bead: coding_agent_session_search-1vxuf
---

# Dependency Audit: frankensqlite local patch

This is a read-only audit of the durable frankensqlite blocker. It did not
commit, push, rewrite `Cargo.toml`, or rebuild/install CASS.

## Sibling Checkout

Command:

```text
git -C /Users/dalecarman/dev/spec014-frankensqlite-fix branch --show-current
git -C /Users/dalecarman/dev/spec014-frankensqlite-fix rev-parse HEAD
git -C /Users/dalecarman/dev/spec014-frankensqlite-fix status --short --branch
git -C /Users/dalecarman/dev/spec014-frankensqlite-fix diff --stat
git -C /Users/dalecarman/dev/spec014-frankensqlite-fix log --oneline -5
```

Result:

```text
branch=fix/fts5-vtab-snapshot-via-delta-journal
HEAD=f298dfa25064124374551737780fd7729ad350db
status=## fix/fts5-vtab-snapshot-via-delta-journal...carmandale/fix/fts5-vtab-snapshot-via-delta-journal
 M crates/fsqlite-pager/src/pager.rs
 M crates/fsqlite-wal/src/wal.rs

diff_stat:
crates/fsqlite-pager/src/pager.rs | 55 +++++++++++++++++++++++++++++++--------
crates/fsqlite-wal/src/wal.rs     | 50 +++++++++++++++++++++++++++++++++++
2 files changed, 94 insertions(+), 11 deletions(-)

last5:
f298dfa fix(fts5): replace eager snapshot clone with O(1) reverse-delta journal
c8ce64f fix(core): preserve index term metadata on rename
de2ccdd docs(perf): refresh current matrix references
466067a docs(perf): refresh fresh-eyes evidence pointers
1c7f5b3 fix(btree): restore delete-run cancellation checkpoints
```

## CASS Cargo Patch

Command:

```text
rg -n "\[patch|frankensqlite|fsqlite|spec014-frankensqlite-fix|rev =|path =" Cargo.toml Cargo.lock
```

Relevant result:

```text
Cargo.toml:45:frankensqlite = { version = "0.1.3", git = "https://github.com/Dicklesworthstone/frankensqlite", rev = "eba969ec45d102071b90519d3b819ddbcecf3d61", package = "fsqlite", features = ["fts5"] }
Cargo.toml:163:fsqlite-types = { version = "0.1.3", git = "https://github.com/Dicklesworthstone/frankensqlite", rev = "eba969ec45d102071b90519d3b819ddbcecf3d61", package = "fsqlite-types" }
Cargo.toml:216:[patch."https://github.com/Dicklesworthstone/frankensqlite"]
Cargo.toml:217:fsqlite = { path = "../spec014-frankensqlite-fix/crates/fsqlite" }
Cargo.toml:218:fsqlite-types = { path = "../spec014-frankensqlite-fix/crates/fsqlite-types" }
```

Interpretation: the manifest declares upstream git rev `eba969ec...`, but the
active patch overrides it with the local sibling checkout.

## Resolved Cargo Graph

Command:

```text
cargo tree -i 'fsqlite@0.1.3' --edges normal
cargo tree -i 'fsqlite-types@0.1.3' --edges normal
```

Result excerpt:

```text
fsqlite v0.1.3 (/Users/dalecarman/dev/spec014-frankensqlite-fix/crates/fsqlite)
└── coding-agent-search v0.4.7 (/Users/dalecarman/dev/coding_agent_session_search)

fsqlite-types v0.1.3 (/Users/dalecarman/dev/spec014-frankensqlite-fix/crates/fsqlite-types)
├── fsqlite v0.1.3 (/Users/dalecarman/dev/spec014-frankensqlite-fix/crates/fsqlite)
│   └── coding-agent-search v0.4.7 (/Users/dalecarman/dev/coding_agent_session_search)
...
```

Interpretation: the active CASS build is using the uncommitted local sibling
patch, not a durable git revision. This keeps live promotion blocked until the
frankensqlite fix is committed/pushed and CASS points at that durable revision
or another approved pin.

## Remotes

Command:

```text
git -C /Users/dalecarman/dev/spec014-frankensqlite-fix remote -v
git remote -v
```

Result:

```text
frankensqlite:
carmandale https://github.com/carmandale/frankensqlite.git
origin     https://github.com/Dicklesworthstone/frankensqlite

cass:
origin   https://github.com/carmandale/coding_agent_session_search.git
upstream https://github.com/Dicklesworthstone/coding_agent_session_search.git
```

## Current Decision

The durable dependency blocker remains open. The next non-read-only step still
requires explicit approval because it involves committing/pushing the sibling
fix and resolving the CASS dependency pin.
