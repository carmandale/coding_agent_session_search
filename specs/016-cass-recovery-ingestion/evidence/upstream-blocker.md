---
title: "Spec 016 upstream blocker"
date: 2026-05-17T01:29:00Z
bead: coding_agent_session_search-1vxuf
---

# Upstream State

`upstream/main` is not yet incorporated into the current checkout.

Captured evidence:

- Current branch: `dac/main`
- Local HEAD: `b807ef175dcdeeb48b912a22913fbcd68fb86cb8`
- Upstream HEAD: `e337b9f428e12ea5a0d5b37129d3abb0dea48ab8`
- Merge base: `3763b33132c78ecb541180f05e1b1dd6ec6719e1`
- Ahead/behind: `19 19`
- `git merge-base --is-ancestor upstream/main HEAD` exit code: `1`
- `git merge-tree --write-tree HEAD upstream/main` exit code: `0`
- Merge-tree output tree: `124403bc99be2effce1bbc9bc9cc39d330639ef6`

Refresh on 2026-05-16T22:59:41Z:

```text
git fetch upstream main
From https://github.com/Dicklesworthstone/coding_agent_session_search
 * branch              main       -> FETCH_HEAD
   66e5039e..37b42058  main       -> upstream/main

HEAD                  b807ef175dcdeeb48b912a22913fbcd68fb86cb8
branch                dac/main
upstream/main         37b42058312d4aafa4a45ede8ae81ff5b8a07134
merge-base            3763b33132c78ecb541180f05e1b1dd6ec6719e1
ahead behind          19 17
ancestor_exit         1
merge-tree            fb45a78749059dde70779c27b7ff0da0b8cf4b2d
merge_tree_exit       0
```

Refresh on 2026-05-17T00:25:43Z:

```text
git fetch upstream main
From https://github.com/Dicklesworthstone/coding_agent_session_search
 * branch              main       -> FETCH_HEAD
   37b42058..956f1d3b  main       -> upstream/main

HEAD                  b807ef175dcdeeb48b912a22913fbcd68fb86cb8
branch                dac/main
upstream/main         956f1d3baf2881e792b5d3397d1875789476f587
merge-base            3763b33132c78ecb541180f05e1b1dd6ec6719e1
ahead behind          19 18
ancestor_exit         1
merge-tree            239b49b7afc81c228be8c63a1b3cbb19d84f309b
merge_tree_exit       0
```

Refresh on 2026-05-17T01:29:00Z:

```text
git fetch upstream main
fetch_exit            0

HEAD                  b807ef175dcdeeb48b912a22913fbcd68fb86cb8
branch                dac/main
upstream/main         e337b9f428e12ea5a0d5b37129d3abb0dea48ab8
merge-base            3763b33132c78ecb541180f05e1b1dd6ec6719e1
ahead behind          19 19
ancestor              no
merge-tree            124403bc99be2effce1bbc9bc9cc39d330639ef6
merge_tree_exit       0
```

Interpretation:

- The non-destructive merge-tree probe found a clean synthetic merge tree.
- Incorporation still requires mutating the checkout and creating a merge commit
  because local `HEAD` and `upstream/main` have diverged.
- This session started on `dac/main`, not `main`, and the repo policy requires
  explicit authorization before committing from an already-active branch.
- Therefore upstream sync is currently blocked by branch-policy authorization,
  not by a committed-history merge conflict.
- A separate working-tree overlap check found that uncommitted recovery changes
  overlap upstream changes, including upstream deletes of
  `src/indexer/scratch_root.rs` and `tests/spec_015_streaming_watch_once.rs`.
  See `evidence/upstream-working-tree-overlap.md`.

Required authorization before this blocker can close:

1. Authorize merging `upstream/main` into the current branch `dac/main`, then
   committing/pushing from that branch; or
2. Authorize a non-destructive move back to `main` before merging and finalizing.

Until then, runtime recovery and evidence collection can proceed, but final
upstream acceptance cannot be claimed.
