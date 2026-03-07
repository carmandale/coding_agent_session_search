---
title: "Tasks: cass doctor reconciliation"
date: 2026-03-07
bead: coding_agent_session_search-2gxp
---

# Tasks: Disk-vs-DB Reconciliation in `cass doctor`

## Phase 1: Trait Extension

- [ ] **1.1 Add `count_disk_files()` to `Connector` trait**
  - Add default implementation in `src/connectors/mod.rs`
  - Default walks `detect().root_paths` counting all files
  - Add `file_extensions()` helper method returning `Option<&[&str]>` for filtering (default: None = all files)
  - Write unit test: default impl counts files in temp dir
  - Files: `src/connectors/mod.rs`

- [ ] **1.2 Override for Claude Code connector**
  - Count `.jsonl` and `.json` files in detected roots
  - Exclude `.settings.json` pattern
  - Walk subdirectories (projects + subagents)
  - Write test: counts match expected for fixture directory
  - Files: `src/connectors/claude_code.rs`

- [ ] **1.3 Override for Gemini connector**
  - Delegate to existing `session_files(root).len()`
  - Write test: count matches `session_files` output
  - Files: `src/connectors/gemini.rs`

- [ ] **1.4 Override for Factory connector**
  - Count `.jsonl` files, skip `.settings.json`
  - Write test with fixture directory
  - Files: `src/connectors/factory.rs`

- [ ] **1.5 Override for Cursor connector**
  - Count workspace SQLite DB files (Cursor uses SQLite, not JSONL)
  - Write test with fixture directory
  - Files: `src/connectors/cursor.rs`

- [ ] **1.6 Override for ChatGPT connector**
  - Count `.json` conversation export files
  - Write test with fixture directory
  - Files: `src/connectors/chatgpt.rs`

- [ ] **1.7 Verify default impl works for remaining connectors**
  - Aider, Amp, Cline, Codebuff, Codex, OpenCode, Pi-Agent
  - Write one representative test: default `count_disk_files()` on a connector with temp fixtures
  - Files: `src/connectors/mod.rs` (test)

## Phase 2: Doctor Integration

- [ ] **2.1 Add `db_count_for_agent()` helper**
  - Query `SELECT COUNT(*) FROM conversations c JOIN agents a ON c.agent_id = a.id WHERE a.slug = ?1`
  - Handle missing agent gracefully (return 0)
  - Write unit test with in-memory DB
  - Files: `src/lib.rs`

- [ ] **2.2 Add reconciliation check to `run_doctor()`**
  - Instantiate all 12 connectors
  - Call `count_disk_files()` + `db_count_for_agent()` for each
  - Compute delta and status: `pass` (delta=0), `warn` (delta 1-10), `fail` (delta >10)
  - Skip reconciliation if DB is not OK (`db_ok == false`)
  - Add timing measurement for the reconciliation block
  - Files: `src/lib.rs`

- [ ] **2.3 JSON output integration**
  - Add `reconciliation` object to existing JSON payload
  - Structure: `{ balanced: bool, elapsed_ms: u64, connectors: [{ agent, disk_files, db_entries, delta, status }] }`
  - Only include connectors where `detect()` returned true (skip uninstalled agents)
  - Files: `src/lib.rs`

- [ ] **2.4 Human-readable output**
  - Add reconciliation to the check list using existing `Check` pattern
  - Summary line: "All N connectors balanced" or "M connectors have gaps"
  - Detail lines: per-connector breakdown only when status is `warn` or `fail`
  - Files: `src/lib.rs`

## Phase 3: Verification

- [ ] **3.1 Compiler checks**
  - `cargo check --all-targets`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo fmt --check`

- [ ] **3.2 Full test suite**
  - `cargo test`
  - All new tests from Phase 1 + 2 pass
  - No regressions in existing tests

- [ ] **3.3 Integration test**
  - Run `cass doctor --json` and verify `reconciliation` key exists
  - Run `cass doctor --verbose` and verify reconciliation line appears
  - Files: manual verification or integration test

- [ ] **3.4 Performance validation**
  - `cass doctor --json` completes in <10s total (existing checks + reconciliation)
  - Reconciliation block alone completes in <5s
  - Files: check `elapsed_ms` in JSON output
