<!-- Codex Review: APPROVED after 3 rounds | model: gpt-5.3-codex | date: 2026-03-07 -->
<!-- Status: REVISED -->
<!-- Revisions: (R1) Added failing repro tests, migration strategy, reconciliation formula, warn-level logging, deferred doctor --fix, strict compact detection. (R2) Per-connector scan_start_ts timestamp advancement, fixture-only automated tests, mandatory full rebuild migration. -->

# Implementation Plan — Index Gaps Fix

**Bead:** `coding_agent_session_search-0cro`
**Spec:** `specs/003-index-gaps/spec.md`
**Date:** 2026-03-07

---

## Architecture Context

The indexing pipeline flows:

```
Connector.detect() → Connector.scan(ctx) → SqliteStorage.ingest() → TantivyIndex.add_doc()
                          ↑
                     file_modified_since(path, ctx.since_ts)
                          ↑
                     watch_state.json (per-connector timestamps)
```

### Key Files
| File | Role |
|------|------|
| `src/connectors/claude_code.rs` | Claude Code connector — file discovery + JSONL parsing |
| `src/connectors/gemini.rs` | Gemini connector — session file discovery in `<hash>/chats/` |
| `src/connectors/factory.rs` | Factory connector — session JSONL parsing |
| `src/connectors/mod.rs` | `file_modified_since()`, `Connector` trait, `ScanContext`, `NormalizedConversation` |
| `src/indexer/mod.rs` | Orchestrates scan→ingest, manages `watch_state.json`, `--full` flag handling |
| `src/storage/sqlite.rs` | `upsert_conversation()` — dedup by `UNIQUE(agent_id, external_id)` and `source_path` |

### Dedup Key Analysis (from `src/storage/sqlite.rs:329-336`)

```sql
CREATE TABLE conversations (
    ...
    external_id TEXT,
    source_path TEXT NOT NULL,
    ...
    UNIQUE(agent_id, external_id)
);
```

The dedup key is `(agent_id, external_id)`. The Claude Code connector sets `external_id` from the filename stem (`src/connectors/claude_code.rs:298-304`). Since subagent files have unique filenames (`agent-ad42c53a2d68f2286.jsonl`), their `external_id` should not collide with parent sessions. The root cause is more likely that conversations with 0 extracted messages are silently dropped (`src/connectors/claude_code.rs:250-253`).

---

## Phase 1: Diagnose & Fix Claude Code Subagent Gap (70 files)

### Step 1.1: Write a failing reproduction test FIRST

Before any fix, create a test fixture and test that demonstrates the current failure:

```
tests/fixtures/claude_code_subagent/
  projectA/
    parent-session.jsonl          # Standard session
    parent-session/
      subagents/
        agent-a1234567890abcdef.jsonl   # Standard subagent
        agent-acompact-abcdef1234.jsonl # Compact format subagent
```

Add test in `tests/connector_claude.rs`:
```rust
#[test]
fn scan_discovers_subagent_sessions() {
    // Asserts connector.scan() returns separate NormalizedConversation
    // entries for parent AND both subagent files (3 total)
}

#[test]
fn scan_parses_compact_format_subagent() {
    // Asserts compact format file produces messages with correct roles
}
```

**Run the test to confirm it FAILS.** This locks the bug before we fix it.

### Step 1.2: Diagnose the actual failure point

Instrument with `RUST_LOG=debug` and trace a known subagent file through:
1. Discovery by `WalkDir` — is the file found? (expected: yes)
2. Extension filter — does `.jsonl` pass? (expected: yes)
3. `file_modified_since()` — is it skipped by incremental timestamp? (possible root cause)
4. `entry_type` filter — does `matches!(entry_type, Some("user" | "assistant"))` match? (possible root cause for compact format)
5. `messages.is_empty()` check at line ~250 — are all messages filtered out, causing the `continue`?
6. SQLite upsert — does `UNIQUE(agent_id, external_id)` collision drop it?

**Expected finding:** For standard subagent files (`agent-a*.jsonl` with normal format), the issue is likely #3 (timestamp) or #6 (dedup). For compact files (`agent-acompact-*`), the issue is #4 (missing `type` field → all messages filtered → #5 triggers).

### Step 1.3: Fix based on diagnosis

**For standard subagent files:**
- If timestamp-based skip: ensure `--full` scan uses `since_ts = None` (see Phase 2)
- If dedup collision: change `external_id` to use normalized relative `source_path` from the project root — this is the canonical, stable, unique key across all file-based connectors (per Codex alternative suggestion #1)

**Migration strategy for dedup key change:**
- **Mandatory full rebuild** on dedup key rollout: users must run `cass index --full --force-rebuild` after updating. The `--full` flag calls `reset_storage()` which clears all SQLite tables, then `t_index.delete_all()` clears Tantivy. This means old entries with stale dedup keys are wiped — there are no orphaned entries.
- Post-rebuild verification: `SELECT source_path, COUNT(*) FROM conversations GROUP BY source_path HAVING COUNT(*) > 1` to confirm 0 duplicates. This is verification only — the full rebuild should inherently prevent duplicates.
- **No schema migration needed** — the column type doesn't change, only the value generation logic.

**For compact format subagent files (`agent-acompact-*`):**

Strict schema guards (not heuristic detection):
1. **Detect compact format** by checking: file basename starts with `agent-acompact-` (filename-based, not content-based — zero false positive risk)
2. **Parse compact format:**
   - Line with `"type": "user"` → extract user message normally
   - Line with `"message"` key but no `"type"` field → infer role from `message.role` field
3. **Add fixture tests** for both formats with exact expected outputs:
   ```
   tests/fixtures/claude_compact_format.jsonl  # Known-good compact file
   tests/fixtures/claude_standard_format.jsonl # Known-good standard file
   ```
4. **Guard against misclassification:** If a file matches `agent-acompact-*` pattern but fails compact parsing, fall back to standard parsing with a `tracing::warn!` (paths only, never content).

### Step 1.4: Reconciliation accounting

After fix, define formal disk-vs-DB accounting for Claude Code:

```
disk_total = WalkDir .jsonl count (recursive, follow symlinks)
indexed = DB conversations with agent_slug='claude_code'
intentionally_skipped = files where messages.is_empty() after parsing (progress-only, corrupt)
active_inflight = 0 or 1 (current session being written)

INVARIANT: indexed + intentionally_skipped + active_inflight = disk_total (±1)
```

Automate this check in `cass doctor` (see Phase 3).

---

## Phase 2: Fix Gemini Orphaned Sessions (13 files)

### Step 2.1: Write a failing reproduction test FIRST

Create a test that reproduces the exact failure scenario:

```rust
#[test]
fn full_scan_indexes_files_regardless_of_mtime() {
    // 1. Create fixture with session files having old mtimes
    // 2. Set watch_state.json with a timestamp AFTER the file mtimes
    // 3. Run a full scan (since_ts = None)
    // 4. Assert all files are indexed
}

#[test]
fn incremental_scan_skips_old_files() {
    // 1. Create fixture with session files having old mtimes
    // 2. Set since_ts to AFTER the file mtimes
    // 3. Run incremental scan
    // 4. Assert old files are skipped (expected behavior)
}
```

### Step 2.2: Targeted backfill for orphaned files (immediate fix)

Before changing timestamp infrastructure, add a simpler targeted repair:

In `cass doctor`, add an orphan detection check per connector:
1. List all valid session files on disk (using connector's own `session_files()` / `WalkDir` logic)
2. Query DB for conversations matching those `source_path` values
3. Files on disk with no DB entry = orphaned
4. Report them diagnostically (read-only by default)

This is **diagnostics-only** — no auto-fix in the first pass.

### Step 2.3: Verify `--full` scan timestamp handling

Trace the code path in `src/indexer/mod.rs` for `IndexOptions { full: true }`:

```rust
// src/indexer/mod.rs ~line 602
if opts.full {
    reset_storage(&mut storage)?;  // Clears SQLite tables
    t_index.delete_all()?;          // Clears Tantivy index
}
```

Key question: what `since_ts` is passed to `ScanContext` during a `--full` scan? Trace through the streaming indexer to confirm it's `None`.

If `since_ts` is NOT `None` during full scans, fix it. If it IS `None`, the root cause is elsewhere (e.g., file discovery failure, parsing error swallowed silently).

### Step 2.4: Fix timestamp handling (if confirmed as root cause)

If the issue is that `watch_state.json` persists stale timestamps across `--full` rebuilds:
- Clear all entries in `watch_state.json` when `--full` flag is active
- Set all timestamps to 0 BEFORE scanning begins
- After successful scan + ingest/commit for each connector, advance that connector's timestamp to `scan_start_ts` (captured BEFORE the scan began) — NOT to "current time," which would skip files created during the scan window but discovered after their connector pass
- Per-connector advancement: only update a connector's timestamp after its ingest succeeds, not globally

If the issue is a discovery/parsing failure (not timestamp):
- Add structured error logging per file: `tracing::warn!(path, error, "gemini: failed to parse session")` — paths only, never content
- Ensure errors don't abort the entire connector scan (already handled by `match serde_json::from_str`)

### Step 2.5: Periodic full-scan heartbeat for watcher (deferred)

This is a **separate concern** from the orphan fix. Defer to a follow-up PR after the targeted backfill and timestamp fix are proven. The heartbeat adds complexity and needs its own performance testing.

---

## Phase 3: Documentation, Diagnostics & Logging (Factory stubs + cross-cutting)

### Step 3.1: Add structured skip logging across ALL connectors

Per Codex finding #4, use `tracing::warn!` (not `debug!`) with rate limiting:

```rust
// In each connector's scan method, after messages.is_empty() check:
tracing::warn!(
    path = %entry.path().display(),
    connector = "claude_code",  // or "factory", "gemini", etc.
    reason = "no_parseable_messages",
    "skipping session file with no user/assistant content"
);
```

Rate-limit: Use a per-connector atomic counter. Log the first 10 skips per scan at `warn` level, remaining at `debug` level. Summary count at `info` level at end of scan.

**Security constraint:** Never log message content, file content, or session data. Paths and counts only.

### Step 3.2: `cass doctor` — diagnostics-only first pass

Per Codex finding #5, ship `doctor` as **read-only diagnostics** first:

```
cass doctor --json
→ {
    "checks": {
      "claude_code": {
        "disk_files": 2085,
        "indexed": 2066,
        "intentionally_skipped": 47,
        "orphaned": 70,    // subagent files not indexed
        "active": 1,
        "formula_balanced": false
      },
      "gemini": {
        "disk_files": 41,
        "indexed": 33,
        "orphaned": 13,
        "formula_balanced": false
      },
      "factory": {
        "disk_files": 76,
        "indexed": 66,
        "intentionally_skipped": 10,
        "formula_balanced": true
      }
    }
  }
```

**Scope constraint:** Only scan paths from configured connector roots. Never scan arbitrary filesystem paths.

**`--fix` deferred** to a second PR after telemetry proves the diagnostic check has low false-positive rates.

### Step 3.3: Formal reconciliation formula

Define and automate the accounting invariant:

```
indexed + intentionally_skipped + active_inflight = disk_total (±1)
```

Where:
- `indexed` = `SELECT COUNT(*) FROM conversations WHERE agent_slug = X`
- `intentionally_skipped` = files that parse to 0 messages (progress-only, empty session_start)
- `active_inflight` = 0 or 1 (session being actively written)
- `disk_total` = connector's file discovery count

Automated testing uses fixture-based tests only (deterministic, CI-safe, no privacy concerns). Real-data reconciliation is a manual operator command:
```bash
cass doctor --json  # Manual verification against live data
```

---

## Phase 4: Verification & Testing

### Step 4.1: All new tests pass

```bash
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test
```

### Step 4.2: Post-rebuild duplicate detection

After `cass index --full --force-rebuild`:

```sql
-- Verify no duplicate conversations by source_path
SELECT source_path, COUNT(*) as cnt
FROM conversations
GROUP BY source_path
HAVING cnt > 1;
-- Expected: 0 rows
```

### Step 4.3: Manual validation

```bash
cass stats --json  # Compare per-agent counts
cass doctor --json # Verify reconciliation formula
cass search "subagent" --robot --limit 5  # Confirm subagent sessions searchable
```

### Step 4.4: Security audit of logging

Verify no new logging statements include message content, file content, or user data. Only paths, counts, and connector names.

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Subagent dedup key change causes duplicates | Low | Medium | Post-rebuild duplicate check SQL query; `--full` clears old entries |
| Compact format parser misclassifies standard files | Very Low | Medium | Filename-based detection (`agent-acompact-*`), not content heuristic; fixture tests |
| `--full` scan timestamp fix introduces new skip windows | Low | High | Failing repro test first; timestamp reset happens atomically before scan |
| `doctor` diagnostic check is expensive on large datasets | Medium | Low | Scope to connector roots only; add timeout; defer `--fix` |
| Logging leaks sensitive content | Low | High | Security audit step; paths-only constraint enforced in review |

---

## Sequencing

```
Phase 1 (Claude subagents) ──┐
                              ├── Can develop in parallel
Phase 2 (Gemini orphans) ────┘

Phase 3 (Diagnostics/logging) ── After Phase 1+2 fixes are confirmed working

Phase 4 (Verification) ── Gates the merge
```

Phase 1 is highest value (70 real sessions). Phase 2 is second (13 sessions + prevents recurrence). Phase 3 is infrastructure improvement. Phase 4 is the quality gate.
