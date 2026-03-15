---
title: "tasks: Fork sync — Shape D hybrid FAD port"
date: 2026-03-15
bead: coding_agent_session_search-3fir
---

# Tasks

## Phase 1: Add FAD dependency

- [ ] **T1: Add franken-agent-detection to Cargo.toml**
  - `franken-agent-detection = { git = "https://github.com/Dicklesworthstone/franken_agent_detection", rev = "5b0eb1a", package = "franken-agent-detection", features = ["connectors"] }`
  - Verify: `cargo check` compiles (FAD dep chain is clean — no franken crates)
  - If compile fails: check FAD feature flags, may need to disable feature gates that pull unwanted deps

## Phase 2: Adapter layer

- [ ] **T2: Create fad_adapter.rs**
  - New file: `src/connectors/fad_adapter.rs`
  - Generic `FadAdapter<T>` struct wrapping FAD connectors
  - `impl Connector for FadAdapter<T>` with 4 methods (detect, scan, count_disk_files=None, reconciliation_notes=None)
  - `convert_conversation()` and `convert_message()` functions for type bridge
  - Handle ScanRoot/ScanContext conversion
  - Depends on: T1

- [ ] **T3: Register adapter module**
  - Add `pub mod fad_adapter;` to `src/connectors/mod.rs`
  - Depends on: T2

## Phase 3: Register connectors

- [ ] **T4: Add 4 ConnectorKind variants**
  - Add `Copilot, Clawdbot, OpenClaw, Vibe` to `ConnectorKind` enum (indexer/mod.rs ~line 1239)
  - Add `from_slug()` match arms: `"copilot"`, `"clawdbot"`, `"openclaw"`, `"vibe"`
  - Add `create_connector()` match arms using `FadAdapter`
  - Depends on: T2

- [ ] **T5: Register in get_connector_factories()**
  - Add 4 entries to the factory vec (indexer/mod.rs ~line 776)
  - Depends on: T4

## Phase 4: Verify

- [ ] **T6: cargo check + clippy + fmt**
  - `cargo check --all-targets`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo fmt --check`
  - Depends on: T5

- [ ] **T7: Run test suite**
  - `cargo test --lib` — all existing tests pass
  - Depends on: T6

- [ ] **T8: Build release and test**
  - `cargo build --release`
  - Deploy: `cp target/release/cass ~/.local/bin/cass`
  - Run `cass index --full` and check `cass stats --json` for new connector entries
  - Depends on: T7

- [ ] **T9: Update spec.md with corrected connector names**
  - Fix user story and acceptance criteria to reference copilot, clawdbot, openclaw, vibe (not copilot_cli, kimi, qwen)
  - Depends on: T8

## Dependency graph

```
T1 → T2 → T3 → T4 → T5 → T6 → T7 → T8 → T9
```

Linear dependency chain — each step builds on the previous.
Estimated total: 3-4 hours including verification.
