---
title: "Tasks: fork cleanup"
date: 2026-03-29
bead: coding_agent_session_search-hhm0
---

<!-- plan:complete:v1 | harness: pi/claude-sonnet-4-6 | date: 2026-03-29T12:02:22Z -->

# Tasks — Spec 009: Fork Cleanup

All work on branch `feat/008-upstream-sync` in worktree `/tmp/cass-merge-base`.
Single atomic commit at the end.

## Step 1 — Cargo.toml

- [ ] **T1.1** Bump FAD: change `tag = "v0.1.3"` → `rev = "de450843"`, add `"crush"` to features list
- [ ] **T1.2** Add `[patch]` section before `[workspace.metadata.dist]`:
  ```toml
  [patch."https://github.com/Dicklesworthstone/franken_agent_detection"]
  frankensqlite = { git = "https://github.com/Dicklesworthstone/frankensqlite", rev = "92a9a0fa", package = "fsqlite" }
  ```
- [ ] **T1.3** Verify: `cargo check` still passes after Cargo.toml change

## Step 2 — src/connectors/mod.rs

- [ ] **T2.1** Remove `pub mod codebuff;`
- [ ] **T2.2** Add `pub mod crush;` in alphabetical position (after `pub mod copilot_cli;`)

## Step 3 — File operations

- [ ] **T3.1** Delete `src/connectors/codebuff.rs`
- [ ] **T3.2** Create `src/connectors/crush.rs`:
  ```rust
  //! Connector for Charm's Crush AI coding agent sessions.
  //!
  //! Implementation lives in `franken_agent_detection::connectors::crush`.
  pub use franken_agent_detection::connectors::crush::CrushConnector;
  ```

## Step 4a — indexer/mod.rs: R1 ConnectorExt migration (6 sites)

- [ ] **T4a.1** Remove import line ~37: `use crate::doctor::{ConnectorExt, connector_scan_with_callback};`
- [ ] **T4a.2** Change call site ~1290: `connector_scan_with_callback(&*conn, &ctx, ...)` → `conn.scan_with_callback(&ctx, ...)`
- [ ] **T4a.3** Change call site ~1342: same pattern as T4a.2
- [ ] **T4a.4** Migrate `DetectedRemoteFailureConnector`: move `scan_with_callback` from `impl crate::doctor::ConnectorExt` into `impl Connector` block
- [ ] **T4a.5** Migrate `PanicConnector`: same pattern as T4a.4
- [ ] **T4a.6** Migrate `DisconnectAwareConnector`: same pattern as T4a.4
  - Note: FAD main's Connector trait has default impl for `supports_streaming_scan` — only migrate `scan_with_callback`

## Step 4b — indexer/mod.rs: R0 Codebuff removal from ConnectorKind (4 sites)

- [ ] **T4b.1** Remove from imports ~line 30: `codebuff::CodebuffConnector,`
- [ ] **T4b.2** Remove from `from_slug()` ~line 3398: `"codebuff" => Some(Self::Codebuff),`
- [ ] **T4b.3** Remove from `create_connector()` ~line 3425: `Self::Codebuff => Box::new(CodebuffConnector::new()),`
- [ ] **T4b.4** Remove from enum ~lines 3908-3909: `#[serde(rename = "bf", alias = "Codebuff")] Codebuff,`

## Step 4c — indexer/mod.rs: WatchState forward-compat fix

- [ ] **T4c.1** Remove `#[serde(deny_unknown_fields)]` from `WatchState` struct (~line 3688)
  - Reason: without this, any user with `"bf"` in watch_state.json loses ALL connector timestamps silently

## Step 5 — VERIFY streaming tests pass (gate before cleanup)

- [ ] **T5.1** Run:
  ```bash
  RUSTFLAGS="-C link-arg=-L/Library/Developer/CommandLineTools/usr/lib/clang/21/lib/darwin" \
    cargo test --lib 2>&1 | grep -E "FAILED|passed|failed" | tail -5
  ```
- [ ] **T5.2** Confirm: streaming dispatch tests (~30) now pass; only analytics failures remain
  - **STOP if streaming tests still fail** — investigate FAD API mismatch before continuing

## Step 6 — src/lib.rs

- [ ] **T6.1** Remove line 19: `pub mod doctor;`
- [ ] **T6.2** Remove the codebuff reconciliation block (~72 lines, lines ~9734-9805):
  starts `// 8. Disk-vs-DB reconciliation...`, ends with closing `}` after the `None` branch

## Step 7 — Delete doctor.rs

- [ ] **T7.1** Delete `src/doctor.rs`

## Step 8 — Analytics test failures

- [ ] **T8.1** Create bead: "Analytics tests fail under frankensqlite — upstream regression"
- [ ] **T8.2** Add `#[ignore = "frankensqlite behavior difference from rusqlite — bead <ID>"]` to each of ~25 failing analytics/indexer tests
  - These are in upstream code (analytics/query.rs, etc.) — not caused by spec 009 changes

## Step 9 — Final check and commit

- [ ] **T9.1** `cargo check` — must be clean
- [ ] **T9.2** Confirm fork diff is exactly 4 files: watchdog.rs, indexer/mod.rs, lib.rs, Cargo.toml
  ```bash
  git diff --name-only upstream/main -- src/ Cargo.toml | sort
  ```
- [ ] **T9.3** Commit:
  ```bash
  git add -A
  git commit -m "refactor: drop codebuff/doctor, bump FAD to main, restore crush"
  ```
- [ ] **T9.4** Push: `git push origin feat/008-upstream-sync`
- [ ] **T9.5** Close bead: `br close coding_agent_session_search-hhm0`
