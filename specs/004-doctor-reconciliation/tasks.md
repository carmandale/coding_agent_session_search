<!-- Codex Review: APPROVED after 4 rounds | model: gpt-5.3-codex | date: 2026-03-07 -->
<!-- Status: RECONCILED -->
<!-- Revisions: Reconciled with Codex-approved plan: per-connector counting specification updated, test plan expanded, threshold/notes/cursor-skip tasks added -->
---
title: "Tasks: cass doctor reconciliation"
date: 2026-03-07
bead: coding_agent_session_search-2gxp
---

# Tasks: Disk-vs-DB Reconciliation in `cass doctor`

## Phase 1: Trait Extension + Per-Connector Implementations

- [x] **1.1 Add `count_disk_files()` and `reconciliation_notes()` to `Connector` trait**
  - `count_disk_files(&self) -> Option<usize>` — required method (no default body)
  - `reconciliation_notes(&self) -> Option<String>` — default returns `None`
  - Files: `src/connectors/mod.rs`

- [x] **1.2 Claude Code `count_disk_files()`**
  - Count files with `.jsonl`, `.json`, `.claude` extensions via `WalkDir` on detected roots
  - Override `reconciliation_notes()`: "Progress-only subagent files with no user/assistant content are intentionally skipped"
  - Test: temp dir with mixed extensions → correct count
  - Files: `src/connectors/claude_code.rs`

- [x] **1.3 Gemini `count_disk_files()`**
  - Delegate to `Some(Self::session_files(root).len())`
  - Test: temp dir with session.json in chats/ subdirs → correct count
  - Files: `src/connectors/gemini.rs`

- [x] **1.4 Factory `count_disk_files()`**
  - Count `.jsonl` files, skip `.settings.json`
  - Override `reconciliation_notes()`: "Some files contain only session_start with no messages — these are intentionally skipped"
  - Test: temp dir with .jsonl + .settings.json → only .jsonl counted
  - Files: `src/connectors/factory.rs`

- [x] **1.5 Codex `count_disk_files()`**
  - Delegate to `Some(Self::rollout_files(root).len())` — matches `rollout-*.jsonl|json` in `sessions/`
  - Test: temp dir with rollout-*.jsonl + other files → only rollouts counted
  - Files: `src/connectors/codex.rs`

- [x] **1.6 Pi-Agent `count_disk_files()`**
  - Delegate to `Some(Self::session_files(root).len())`
  - Test: temp dir with timestamp_uuid.jsonl files → correct count
  - Files: `src/connectors/pi_agent.rs`

- [x] **1.7 ChatGPT `count_disk_files()`**
  - Count `.json` and `.data` extensions
  - Test: temp dir with .json + .data + .txt → only json/data counted
  - Files: `src/connectors/chatgpt.rs`

- [x] **1.8 Aider `count_disk_files()`**
  - Count files named `.aider.chat.history.md` — bounded to CWD + env override, no recursive walk
  - Test: temp dir with history file → count = 1 (or 0 if not present)
  - Files: `src/connectors/aider.rs`

- [x] **1.9 Amp `count_disk_files()`**
  - Count files passing `is_amp_log_file()` (thread/conversation/chat stems, T-{uuid}.json, any .json in threads/)
  - Test: temp dir with matching + non-matching files → correct count
  - Files: `src/connectors/amp.rs`

- [x] **1.10 Cline `count_disk_files()`**
  - Count task directories containing `ui_messages.json` or `api_conversation_history.json`
  - Test: temp dir with task dirs → count = number of dirs with messages
  - Files: `src/connectors/cline.rs`

- [x] **1.11 Codebuff `count_disk_files()`**
  - Count directories containing `chat-messages.json`
  - Test: temp dir with workspace dirs → correct count
  - Files: `src/connectors/codebuff.rs`

- [x] **1.12 OpenCode `count_disk_files()`**
  - Count `session/{projectID}/{sessionID}.json` files in session/ subdir
  - Test: temp dir with session structure → correct count
  - Files: `src/connectors/opencode.rs`

- [x] **1.13 Cursor `count_disk_files()`**
  - Return `None` (non-comparable: SQLite DB ≠ conversation count)
  - Override `reconciliation_notes()`: "Cursor uses SQLite databases; file count is not comparable to conversation count"
  - Test: verify `count_disk_files()` returns `None`
  - Files: `src/connectors/cursor.rs`

## Phase 2: Doctor Integration

- [x] **2.1 Add `--reconciliation-threshold` CLI arg**
  - Add to `Doctor` variant in `Commands` enum, default 10
  - Thread through to `run_doctor()`
  - Files: `src/lib.rs`

- [x] **2.2 Add `db_count_for_agent()` helper**
  - Query `SELECT COUNT(*) FROM conversations c JOIN agents a ON c.agent_id = a.id WHERE a.slug = ?1`
  - Handle missing agent gracefully (return 0)
  - Handle DB errors gracefully (return 0 + note)
  - Test: in-memory DB with known counts → correct results
  - Files: `src/lib.rs`

- [x] **2.3 Add reconciliation check to `run_doctor()`**
  - Reuse `indexer::get_connector_factories()` — no hardcoded connector list
  - Slug mapping: `"claude" → "claude_code"`, all others identity
  - Read-only SQLite connection for DB queries
  - Only run if `db_ok == true`
  - Compute signed `i64` delta for each connector
  - Status: `"pass"` (delta=0), `"warn"` (delta≠0), `"skip"` (cursor/non-comparable)
  - Add `above_threshold: true` flag when delta > threshold
  - Measure elapsed_ms for the reconciliation block
  - Files: `src/lib.rs`

- [x] **2.4 JSON output integration**
  - Add `reconciliation` object to existing JSON payload
  - Schema: `{ balanced: bool, elapsed_ms: u64, threshold: u64, connectors: [{ agent, disk_files, db_entries, delta, status, above_threshold?, notes? }] }`
  - Only include connectors where `detect()` returned true (skip uninstalled agents)
  - Files: `src/lib.rs`

- [x] **2.5 Human-readable output**
  - Add reconciliation to check list using existing `Check` pattern
  - Summary: "N connectors balanced, M skipped" or "N connectors have gaps, M skipped"
  - Detail lines: per-connector breakdown with delta and notes, only for warn/skip
  - Files: `src/lib.rs`

## Phase 3: Verification

- [x] **3.1 Compiler checks**
  - `cargo check --all-targets`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo fmt --check`

- [x] **3.2 Full test suite**
  - `cargo test`
  - All new per-connector tests pass
  - No regressions in existing 1151+ tests

- [x] **3.3 Integration verification**
  - `cass doctor --json` → verify `reconciliation` key exists with expected schema
  - `cass doctor --verbose` → verify reconciliation line appears in human output
  - `cass doctor --json --reconciliation-threshold 0` → verify `above_threshold: true` on any connector with delta > 0

- [x] **3.4 Performance validation**
  - `cass doctor --json` completes in <10s total
  - Reconciliation `elapsed_ms` in JSON output < 5000
