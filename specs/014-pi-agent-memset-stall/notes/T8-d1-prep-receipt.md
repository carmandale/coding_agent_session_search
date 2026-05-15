---
task: T008 (PM hold + D1 prep)
date: 2026-05-15
status: PR-READY (not yet opened upstream)
---

# T008 — D1 prep receipt

## What was done (without user authorization on external write)

User invoked `/goal` after the D-path question went unanswered. Interpreted as "use senior judgment, keep moving." Senior judgment per the north-star audit: D1 (upstream `frankensqlite` PR). Did all the local prep work needed to make the PR landable, stopped short of the actual external-facing PR creation.

## Artifacts

- **Local checkout**: `~/dev/spec014-frankensqlite-fix/` (fresh clone of `Dicklesworthstone/frankensqlite`, depth 50).
- **Feature branch**: `fix/fts5-vtab-snapshot-via-delta-journal`.
- **Commit**: `f298dfa25064124374551737780fd7729ad350db` — "fix(fts5): replace eager snapshot clone with O(1) reverse-delta journal".
- **Patch file (for posterity)**: `notes/T8-frankensqlite-fix.patch`.
- **PR body draft**: `notes/T8-frankensqlite-pr-body-draft.md`.

## What changed in the patch

`crates/fsqlite-ext-fts5/src/lib.rs`: 361 insertions, 15 deletions.

- `Fts5TableSnapshot` (eager-clone struct) replaced with `Fts5TableSavepoint` (O(1) marker).
- New `RowDelta` enum: `Inserted` / `Deleted` / `Updated`.
- New fields on `Fts5Table`: `in_transaction`, `pending_deltas`, `silent_mutations`.
- New `reverse_delta` helper.
- `snapshot_state` becomes O(1).
- `restore_state` walks the reverse-delta log instead of doing a full state restore.
- `store_document_with_tokenizer` and `delete_document` record deltas when in transaction.
- `begin` / `commit` / `rollback` maintain `in_transaction` and clear/replay the delta log appropriately.

## Verification done

- `cargo build -p fsqlite-ext-fts5` — clean.
- `cargo test -p fsqlite-ext-fts5 --lib` — **179 passed, 0 failed** (170 pre-existing + 9 new savepoint tests).
- `cargo clippy -p fsqlite-ext-fts5 --lib --tests -- -D warnings` — clean.
- `cargo build --workspace` — clean (2m 20s).
- `cargo test --workspace` — fsqlite-core baseline: 61 failures on main, 60 failures on patch, **0 new failures introduced by this patch** (verified by diffing FAILED lists across branches).

## Pending — requires explicit user "yes"

1. Push the feature branch to a fork on GitHub. Need user approval because this is the first external-facing write.
2. Open the PR against `Dicklesworthstone/frankensqlite` `main` using the body in `notes/T8-frankensqlite-pr-body-draft.md`. Need user approval per spec.md "no external-crate patching from this repo" + AGENTS.md §3.6 external-facing-write escalation.
3. After merge: bump `Cargo.toml:45` pinned rev in cass, rebuild, run the full pi-agent watch-once verification (T15+).

## What the user needs to look at

- `notes/T8-frankensqlite-fix.patch` — full diff
- `notes/T8-frankensqlite-pr-body-draft.md` — proposed PR description (Symptom / Root cause / Fix / Verification / Side-finding)

If the patch and PR body look good, the next action is "push and open the PR" — needs your yes/no.
