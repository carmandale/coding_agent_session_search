---
title: "Tasks: fork cleanup"
date: 2026-03-29
bead: coding_agent_session_search-hhm0
---

<!-- Codex Review: APPROVED after 3 rounds | model: gpt-5.3-codex | date: 2026-03-29 -->
<!-- Status: REVISED -->
<!-- Revisions: removed stale doctor/shim tasks; made crush restoration mandatory and adapter-backed; moved watch-state compatibility ahead of Codebuff removal; upgraded verification to checkout-local fmt/check/clippy/test plus sandboxed watchdog smoke -->
<!-- plan:complete:v1 | harness: pi/claude-sonnet-4-6 | date: 2026-03-29T12:02:22Z -->

# Tasks — Spec 009: Fork Cleanup

Work against the live checkout, not the stale shaping snapshot. The old `doctor.rs` /
`ConnectorExt` tasks are removed because those artifacts are already absent in the current tree.

## Step 0 — Reconcile spec contract

- [x] **T0.1** Acceptance language is aligned on disk via the Codex-reviewed spec/plan writeback:
  - generic `cass doctor` reconciliation stays
  - `src/connectors/crush.rs` is restored as a local wrapper with adapter-backed integration
  - doctor/shim cleanup is recorded as already satisfied in this checkout

## Step 1 — Cargo + dependency update

- [x] **T1.1** Bump FAD from `rev = "5b0eb1a"` to `rev = "de450843"` and enable `"crush"`
- [x] **T1.2** Add `[patch."https://github.com/Dicklesworthstone/franken_agent_detection"]` for `fsqlite`
- [x] **T1.3** Regenerate and inspect `Cargo.lock` so the dependency-source delta is explicit
- [x] **T1.4** Run `cargo check --all-targets` immediately after the dependency change

## Step 2 — Re-baseline stale doctor/shim work as already satisfied

- [x] **T2.1** Record that `src/doctor.rs` is already absent in the live tree
- [x] **T2.2** Record that `pub mod doctor;` is already absent from `src/lib.rs`
- [x] **T2.3** Record that `ConnectorExt` / `connector_scan_with_callback` call sites are already absent
- [x] **T2.4** Remove stale doctor/shim implementation tasks from this task list and keep only the writeback note

## Step 3 — Make watch-state loading tolerant before connector removal

- [x] **T3.1** Replace strict `load_watch_state()` enum-map parsing with tolerant object parsing
- [x] **T3.2** Ignore unknown or removed connector keys instead of zeroing the whole state
- [x] **T3.3** Add regression test: legacy removed key + current keys still loads current keys
- [x] **T3.4** Add regression test: current save/load round-trip remains unchanged

## Step 4 — Remove Codebuff from the live registry

- [x] **T4.1** Remove `pub mod codebuff;` from `src/connectors/mod.rs`
- [x] **T4.2** Remove `codebuff::CodebuffConnector` from the `src/indexer/mod.rs` import block
- [x] **T4.3** Remove the `"codebuff"` entry from `get_connector_factories()`
- [x] **T4.4** Remove `"codebuff"` from `ConnectorKind::from_slug()`
- [x] **T4.5** Remove `Self::Codebuff` from `ConnectorKind::create_connector()`
- [x] **T4.6** Remove the `Codebuff` variant from `ConnectorKind`
- [x] **T4.7** Search for remaining live `codebuff` references with:
  ```bash
  rg -n "codebuff|Codebuff" README.md src tests Cargo.toml Cargo.lock
  ```
- [x] **T4.8** Request explicit written permission before deleting `src/connectors/codebuff.rs`
- [x] **T4.9** Leave `src/connectors/codebuff.rs` detached in-tree because explicit delete approval was not granted

## Step 5 — Restore `src/connectors/crush.rs` with adapter-backed integration

- [x] **T5.1** Create/restore `src/connectors/crush.rs` as a local wrapper module
- [x] **T5.2** Extend `src/connectors/fad_adapter.rs` to import FAD's `CrushConnector`
- [x] **T5.3** Add `fad_adapter::crush()` returning `Box<dyn Connector + Send>`
- [x] **T5.4** Make the `crush.rs` wrapper delegate to the adapter-backed integration
- [x] **T5.5** Wire Crush into `src/connectors/mod.rs`
- [x] **T5.6** Wire Crush into `src/indexer/mod.rs` factories, slug mapping, and `ConnectorKind`
- [x] **T5.7** Add/update crush factory or registry test coverage

## Step 6 — Preserve generic doctor reconciliation

- [x] **T6.1** Do not remove the `run_doctor()` reconciliation loop in `src/lib.rs`
- [x] **T6.2** Verify Codebuff disappears from reconciliation naturally because it is no longer in `get_connector_factories()`
- [x] **T6.3** Add/update a focused regression test if current coverage does not already prove that behavior

## Step 7 — Verification in this checkout only

- [x] **T7.1** `cargo fmt --check`
- [x] **T7.2** `cargo check --all-targets`
- [x] **T7.3** `cargo clippy --all-targets -- -D warnings`
- [x] **T7.4** `cargo test --lib`
- [x] **T7.5** Ensure watch-state regression tests pass
- [x] **T7.6** Ensure Crush integration tests pass
- [x] **T7.7** Run sandboxed watchdog smoke with the checkout-local binary:
  ```bash
  CASS_DATA_DIR=/tmp/cass-watchdog-<run-id> target/debug/cass watchdog run
  ```
- [x] **T7.8** Treat documented watchdog exit codes as valid smoke outcomes:
  - `0` healthy or already locked
  - `1` stale watcher restarted
  - `2` watcher not running
  - any undocumented exit code, panic, or CLI crash is a failure

## Step 8 — Finalize implementation diff and commit

- [x] **T8.1** Confirm the final diff reflects the intentional current-fork delta, not stale doctor/shim deletions
- [x] **T8.2** If delete approval was not granted, note `src/connectors/codebuff.rs` as the only remaining cleanup precondition
- [x] **T8.3** `git add -A`
- [x] **T8.4** `git commit -m "refactor: drop codebuff, bump FAD, restore crush"`
- [x] **T8.5** `git push origin <current-branch>`
- [x] **T8.6** `br close coding_agent_session_search-hhm0`
