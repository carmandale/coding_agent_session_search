---
title: "Upstream working-tree overlap"
date: 2026-05-17T01:29:00Z
bead: coding_agent_session_search-1vxuf
---

# Upstream Working-Tree Overlap

This is a non-destructive merge risk check. It compares files currently dirty in this recovery checkout against files changed between local `HEAD` and fetched `upstream/main`.

Commands:

```text
git diff --name-only > /tmp/spec016-local-dirty-files.txt
git diff --name-only HEAD..upstream/main > /tmp/spec016-upstream-changed-files.txt
comm -12 <(sort /tmp/spec016-local-dirty-files.txt) <(sort /tmp/spec016-upstream-changed-files.txt)
git diff --name-status HEAD..upstream/main -- <overlap files>
```

Overlap:

```text
M  .beads/issues.jsonl
M  .beads/last-touched
M  Cargo.lock
M  Cargo.toml
M  src/indexer/mod.rs
D  src/indexer/scratch_root.rs
M  src/lib.rs
M  src/storage/sqlite.rs
D  tests/spec_015_streaming_watch_once.rs
```

Refresh on 2026-05-17T00:25:43Z:

```text
upstream/main: 956f1d3baf2881e792b5d3397d1875789476f587
changed files between HEAD and upstream/main: 122
overlap unchanged:
.beads/issues.jsonl
.beads/last-touched
Cargo.lock
Cargo.toml
src/indexer/mod.rs
src/indexer/scratch_root.rs
src/lib.rs
src/storage/sqlite.rs
tests/spec_015_streaming_watch_once.rs
```

Refresh on 2026-05-17T01:29:00Z:

```text
upstream/main: e337b9f428e12ea5a0d5b37129d3abb0dea48ab8
changed files between HEAD and upstream/main: 122
overlap unchanged:
.beads/issues.jsonl
.beads/last-touched
Cargo.lock
Cargo.toml
src/indexer/mod.rs
src/indexer/scratch_root.rs
src/lib.rs
src/storage/sqlite.rs
tests/spec_015_streaming_watch_once.rs
```

Interpretation:

- The previous `git merge-tree --write-tree HEAD upstream/main` proves committed local `HEAD` can merge fetched upstream without textual conflicts.
- It does not prove the current uncommitted recovery work can be carried across that merge without manual reconciliation.
- Two recovery-touched files are deleted upstream: `src/indexer/scratch_root.rs` and `tests/spec_015_streaming_watch_once.rs`.
- Several core files touched by the recovery are also modified upstream: `Cargo.toml`, `Cargo.lock`, `src/indexer/mod.rs`, `src/lib.rs`, and `src/storage/sqlite.rs`.
- Therefore upstream incorporation is not just a branch-policy question. It also needs an explicit reconciliation step for local uncommitted recovery work versus upstream's removal/rework of spec 015 streaming-scan pieces.

Operational conclusion:

- Do not run `git merge upstream/main` until live promotion and branch/commit authorization are explicit.
- After approval, either commit the current recovery slice first on the authorized branch or perform an explicit file-by-file reconciliation against upstream's new shapes.
- Treat `src/indexer/scratch_root.rs` and `tests/spec_015_streaming_watch_once.rs` as high-risk paths because upstream deleted them while this recovery modified them.
