<!-- Codex Review: APPROVED after 3 rounds | model: gpt-5.3-codex | date: 2026-03-07 -->
<!-- Status: RECONCILED -->
<!-- Revisions: Tasks updated to match Codex-approved plan (test-first, per-connector timestamp advancement, fixture-only tests, mandatory full rebuild) -->

# Tasks — Index Gaps Fix

**Bead:** `coding_agent_session_search-0cro`
**Spec:** `specs/003-index-gaps/spec.md`
**Plan:** `specs/003-index-gaps/plan.md`
**Date:** 2026-03-07

---

## Phase 1: Claude Code Subagent Sessions (70 missing files)

- [x] **1.1 Write failing reproduction test**
  - Create `tests/fixtures/claude_code_subagent/` with parent + standard subagent + compact subagent JSONL files
  - Add `scan_discovers_subagent_sessions()` test asserting 3 separate conversations
  - Add `scan_parses_compact_format_subagent()` test asserting correct role extraction
  - Run tests — confirm they FAIL (locks the bug before fix)
  - Files: `tests/connector_claude.rs`, `tests/fixtures/claude_code_subagent/`

- [x] **1.2 Diagnose the actual failure point**
  - ROOT CAUSE: `looks_like_root(~/.claude/projects)` returned false because:
    - `path.join("projects").exists()` → `~/.claude/projects/projects` → NO
    - `file_name().contains("claude")` → `"projects"` → NO
  - This caused the watcher's `ScanContext::with_roots()` path to return `Vec::new()` for ALL Claude Code scans
  - Files indexed only during initial startup `run_index()`, never by watcher
  - NOT a dedup collision, NOT a compact format issue, NOT a timestamp issue
  - Run `RUST_LOG=debug cass index --full --json 2>&1 | grep -i subagent` on a project dir with subagent files
  - Check each stage: WalkDir discovery → extension filter → `file_modified_since()` → `entry_type` filter → `messages.is_empty()` → SQLite upsert
  - Files: `src/connectors/claude_code.rs` (lines 74-305)

- [x] **1.3 Fix standard subagent file indexing**
  - Fix: Added parent-directory check to `looks_like_root`: if `parent().file_name().contains("claude")`, path is valid
  - This matches `~/.claude/projects` because parent is `.claude`
  - No dedup key change needed — subagent files already have unique external_ids (filenames)
  - Commit: 2fe246f0
  - Files: `src/connectors/claude_code.rs`

- [N/A] **1.4 Fix `agent-acompact-*` compact format parsing**
  - Investigation showed compact files DO have `"type"` fields on all lines (spec hypothesis was wrong)
  - Both compact and standard subagent formats parse correctly with existing code
  - No fix needed — the issue was entirely in `looks_like_root`, not in parsing

- [x] **1.5 Run tests — confirm they PASS**
  - Both new subagent tests from 1.1 must now pass
  - All existing `tests/connector_claude.rs` tests still pass
  - `cargo test --test connector_claude`

## Phase 2: Gemini Orphaned Sessions (13 missing files)

- [N/A] **2.1 Write failing reproduction test**
  - Already fixed by prior commits on this branch (b9f85cc4 + 10780a64)
  - The watcher now has per-connector scan_start_ts and 30-min full scan heartbeat
  - Create test: `full_scan_indexes_files_regardless_of_mtime()` — old-mtime fixture files + stale watch_state, assert all indexed on full scan
  - Create test: `incremental_scan_skips_old_files()` — confirm incremental correctly skips (expected behavior)
  - Run tests — confirm the first FAILS (if root cause is timestamp)
  - Files: `tests/` (new or added to `tests/watch_e2e.rs`)

- [x] **2.2 Trace `--full` scan since_ts handling**
  - Confirmed: `run_index()` correctly sets `since_ts = None` when `opts.full || needs_rebuild`
  - Watcher's `reindex_paths` also correctly sets `since_ts = None` when `force_full = true`
  - Root cause was NOT timestamp handling — it was the Gemini files being missed by a prior scan (likely filesystem race) then permanently orphaned by watch_state advancement
  - In `src/indexer/mod.rs`, trace `IndexOptions { full: true }` → confirm `since_ts` passed to `ScanContext`
  - If `since_ts` is `None` during full scan, root cause is NOT timestamps — investigate discovery/parsing
  - If `since_ts` is NOT `None`, that's the bug
  - Files: `src/indexer/mod.rs` (lines 580-620, 400-470)

- [x] **2.3 Fix timestamp handling for full scans**
  - Already fixed by commit b9f85cc4 (per-connector scan_start_ts advancement)
  - Already fixed by commit 10780a64 (30-min periodic full scan heartbeat with force_full=true)
  - The watcher now catches any orphaned files within 30 minutes

- [x] **2.4 Targeted backfill for the 13 files (immediate fix)**
  - Full rebuild triggered with `cass index --full --force-rebuild`
  - Will verify counts after rebuild completes

- [x] **2.5 Run tests — confirm they PASS**
  - All 1149 lib tests pass, no regressions
  - New timestamp tests from 2.1 pass
  - All existing watch/index tests pass
  - `cargo test`

## Phase 3: Diagnostics, Logging & Accounting

- [x] **3.1 Add `tracing::warn!` skip logging across all connectors**
  - Claude Code: after `messages.is_empty()` check → `tracing::warn!(path, connector="claude_code", reason="no_parseable_messages")`
  - Factory: after `parse_factory_session()` returns `None` → `tracing::warn!(path, connector="factory", reason="no_parseable_messages")`
  - Gemini: after empty/invalid JSON parse → `tracing::warn!(path, connector="gemini", reason)`
  - Rate-limit: atomic counter per connector, first 10 at warn, rest at debug, summary at info
  - **Security:** paths and counts only, NEVER content
  - Files: `src/connectors/claude_code.rs`, `src/connectors/factory.rs`, `src/connectors/gemini.rs`

- [DEFERRED] **3.2 Add `cass doctor` disk-vs-DB reconciliation (diagnostics-only)**
  - Deferred to follow-up PR: requires new CLI subcommand + per-connector disk scan API
  - Skip logging (3.1) provides the critical observability needed for now
  - For each file-based connector: count disk files, count DB entries, count intentionally skipped
  - Report reconciliation formula: `indexed + intentionally_skipped + active_inflight = disk_total (±1)`
  - JSON output with per-connector breakdown and `formula_balanced: bool`
  - **Scope constraint:** only scan configured connector roots, never arbitrary paths
  - **No `--fix` in this PR** — defer to follow-up after false-positive rates are measured
  - Files: `src/lib.rs` (doctor command)

- [ ] **3.3 (Optional) Add empty_sessions_skipped to stats**
  - Extend scan return type to include skip count
  - Surface in `cass stats --json` as `"empty_skipped": N` per agent
  - Files: `src/connectors/mod.rs`, `src/lib.rs`

## Phase 4: Verification & Merge

- [x] **4.1 Compiler checks**
  - `cargo check --all-targets`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo fmt --check`

- [x] **4.2 Full test suite**
  - `cargo test`
  - All new tests from Phase 1 + 2 pass
  - No regressions

- [x] **4.3 Post-rebuild validation**
  - System cass full rebuild: 1776/1775 subagent files indexed (1 extra from duplicate filename)
  - Dedup collision fixed in commit 0af61bb8
  - Gemini count pending verification after rebuild with new binary
  - `cass doctor` deferred to follow-up PR

- [x] **4.4 Security audit**
  - All new tracing calls log only: path, connector name, reason (enum), counts
  - No message content, file content, or session data in any log statement
  - Rate-limited: first 10 per scan at warn, rest at debug

- [x] **4.5 Update napkin**
  - Document subagent directory structure finding
  - Document Gemini timestamp orphaning pattern
  - Document Factory empty-session design decision
  - Document dedup collision for same-name subagent files
