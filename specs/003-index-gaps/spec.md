<!-- Codex Review: APPROVED after 3 rounds | model: gpt-5.3-codex | date: 2026-03-07 -->
<!-- Status: UNCHANGED -->
<!-- Revisions: Acceptance criteria tightened per Codex feedback (reconciliation formula, test-first mandate, security audit, deferred doctor --fix) -->

# fix: Index gaps — Claude subagent sessions, Gemini missed files, Factory stubs

**Bead:** `coding_agent_session_search-0cro`
**Type:** bug
**Priority:** P2
**Date:** 2026-03-07

---

## Problem Statement

The cass index reports as healthy with 0 pending sessions, but investigation reveals **three categories of missing sessions** totaling ~93 unindexed files:

| Category | Count | Agent | Root Cause |
|----------|-------|-------|------------|
| Claude Code subagent sessions | 70 | claude_code | `WalkDir` traverses `~/.claude/projects` but subagent JSONL files live under `<session-uuid>/subagents/` — these contain real user/assistant conversations but are never discovered |
| Gemini sessions skipped by watcher | 13 | gemini | Files existed before the full scan but were missed; `watch_state.json` timestamp then advanced past them, permanently orphaning them from incremental scans |
| Factory stub sessions | 10 | factory | 1-line JSONL files containing only `session_start` event with no messages — connector returns `None` and they're silently dropped |

Additionally, **47 Claude Code root-level files** contain only `progress` events (no user/assistant content). These are correctly skipped — they have no searchable content.

---

## Root Cause Analysis (§2.5 Gate)

### Gap 1: Claude Code Subagent Sessions (70 files)

**Symptom:** 70 JSONL files under `~/.claude/projects/<project>/<session-uuid>/subagents/agent-*.jsonl` are not indexed despite containing valid user/assistant conversations.

**Root cause (5 Whys):**
1. **Why are they missing?** The Claude Code connector's `WalkDir` scan finds `.jsonl` files but the indexer doesn't know about subagent session structure.
2. **Why doesn't WalkDir find them?** It DOES find them — `WalkDir::new(&root)` is recursive by default. The real question is why they're not in the DB.
3. **Why aren't they in the DB?** After further investigation: these files ARE found by WalkDir but may share `external_id` or content-hash with their parent sessions, causing deduplication to drop them. Alternatively, the connector may be producing conversations with the same `source_path` as the parent, causing upsert collisions.
4. **Why would they collide?** The `session_id` extracted from the JSONL may match the parent session's ID since subagents inherit the parent session context.

**Verification needed:** Trace the exact scan → ingest → dedup path for a subagent file to confirm whether it's a discovery issue, a dedup collision, or a session_id conflict.

**Fix direction:** Ensure subagent sessions get unique identifiers (e.g., append the subagent file's basename to the external_id) and are not deduplicated against their parent session.

### Gap 2: Gemini Sessions Missed by Watcher (13 files)

**Symptom:** 13 valid Gemini session files (mostly from 2026-02-22) exist on disk with proper `<hash>/chats/session-*.json` structure but are not in the DB. All have mtimes BEFORE the current `watch_state.json` Gemini timestamp (1772849627513 = 2026-03-06T20:13:47).

**Root cause (5 Whys):**
1. **Why are they missing?** The watcher's `since_ts` is past their mtime, so incremental scans skip them.
2. **Why did the initial full scan miss them?** They existed at scan time (files from Feb 22) but full scan on Mar 6+ didn't pick them up.
3. **Why didn't the full scan find them?** The `file_modified_since()` check during a `--full` scan should use `since_ts = None` (scan everything). If `since_ts` was incorrectly set during the full scan, files would be skipped.
4. **Why might since_ts be wrong?** The `watch_state.json` persists per-connector timestamps. If a `--full` scan doesn't properly reset the Gemini entry to 0/None, it may carry forward a stale timestamp.

**Fix direction:** Ensure `--full` scans truly reset `since_ts` to `None`/0 for ALL connectors. Add a self-healing mechanism: periodic full-scan heartbeat that catches orphaned files.

### Gap 3: Factory Stub Sessions (10 files)

**Symptom:** 10 Factory JSONL files with 1 line each (only `session_start` event, 188-533 bytes) are not indexed.

**Root cause:** The Factory connector's `parse_factory_session()` returns `None` when no messages are found after parsing. This is **intentionally correct** — a session with only a `session_start` event and zero user/assistant messages has no searchable content.

**Fix direction:** This is **by design**. No fix needed. These are empty sessions that Factory created but the user never interacted with. However, we should document this behavior and optionally count them in `cass stats` as "empty sessions skipped."

---

## Acceptance Criteria

### Must Have
- [ ] All 70 Claude Code subagent JSONL files are indexed with unique identifiers
- [ ] All 13 orphaned Gemini sessions are indexed
- [ ] Formal reconciliation invariant per connector: `indexed + intentionally_skipped + active_inflight = disk_total (±1)`
- [ ] `--full` scan with `--force-rebuild` indexes every valid session file regardless of timestamps
- [ ] Post-rebuild duplicate detection: no `source_path` with `COUNT(*) > 1` in conversations table
- [ ] Migration safety: dedup key changes must not fragment or duplicate conversation histories
- [ ] Failing reproduction tests written BEFORE any fix code (test-first)
- [ ] No regression in existing tests
- [ ] No logging of message content, file content, or user data — paths and counts only

### Should Have
- [ ] `cass doctor` diagnostics-only check reporting disk-vs-DB accounting per connector (read-only, no auto-fix)
- [ ] `tracing::warn!` level logging across all connectors when files are skipped (rate-limited: first 10 per scan at warn, rest at debug)
- [ ] `agent-acompact-*` format support with strict filename-based detection (not content heuristic)

### Nice to Have (deferred to follow-up PRs)
- [ ] `cass doctor --fix` with explicit scope (`--agent`, `--workspace`), dry-run preview, and success-only timestamp advancement
- [ ] Periodic watcher full-scan heartbeat (separate PR after targeted fix is proven)
- [ ] `empty_sessions_skipped` count in `cass stats --json` output
