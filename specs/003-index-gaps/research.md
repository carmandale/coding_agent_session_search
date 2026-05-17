# Research — Index Gaps Investigation

**Date:** 2026-03-07

---

## Investigation Summary

### Methodology
1. Compared `cass stats --json` DB counts against actual files on disk per agent
2. Used `sqlite3` to query the DB for orphaned/missing entries
3. Examined connector source code to understand scan + dedup logic
4. Inspected unindexed files to determine content/format

### Index Health Status
- **Overall:** Healthy, watcher active, 0 pending sessions, last indexed seconds ago
- **Total:** 10,687 conversations / 411,932 messages
- **Date range:** Nov 2023 → present

---

## Gap 1: Claude Code — 117 Unindexed Files

### Breakdown
| Category | Count | Description |
|----------|-------|-------------|
| Subagent files (`/subagents/agent-*.jsonl`) | 70 | Real sessions with user/assistant content — **should be indexed** |
| Root-level progress-only files | 47 | Only `progress` events, no user/assistant — correctly skipped |
| Root-level WITH content | 0 | None — all root files with content ARE indexed |

### Subagent File Structure
```
~/.claude/projects/<project-slug>/<session-uuid>/subagents/agent-ad42c53a2d68f2286.jsonl
```

Sample content analysis of `agent-ad42c53a2d68f2286.jsonl`:
- 184 lines: 30 user entries, 40 assistant entries, 114 progress entries
- Contains real conversation data — valuable for search

### Compact Format (`agent-acompact-*`)
```
Line 1: {"type": "user", "slug": "...", "agentId": "...", "sessionId": "...", ...}
Line 2: {"message": {...}, "parentUuid": "...", ...}  // no explicit "type" field
```
- 2-6 lines per file
- The `type` field on line 2 is missing — would cause the `entry_type` filter to skip it

### Connector Code Path (`src/connectors/claude_code.rs`)
```rust
// WalkDir IS recursive — finds subagent files ✓
for entry in WalkDir::new(&root).into_iter().flatten() {
    // Extension check allows .jsonl ✓
    if ext != Some("jsonl") && ext != Some("json") && ext != Some("claude") { continue; }
    // Incremental check
    if !crate::connectors::file_modified_since(entry.path(), ctx.since_ts) { continue; }
    // Message filter — ONLY user/assistant
    if !matches!(entry_type, Some("user" | "assistant")) { continue; }
}
```

**Hypothesis:** Files ARE discovered but either:
- (a) `session_id` from subagent JSONL matches parent → dedup collision in SQLite upsert
- (b) The compact format's missing `type` field causes all messages to be filtered out

---

## Gap 2: Gemini — 13 Unindexed Files

### File Details
All have valid JSON with `sessionId`, `messages`, proper `<hash>/chats/session-*.json` structure:

| File | Directory | Modified | Messages |
|------|-----------|----------|----------|
| session-2026-02-22T15-09-*.json | avpstreamkit | Feb 22 | 2 |
| session-2026-02-22T12-21-*.json | avpstreamkit | Feb 22 | 2 |
| session-2026-03-03T17-37-*.json | groovetech-media-player | Mar 3 | 2 |
| session-2026-02-22T12-24-*.json | groovetech-media-player | Feb 22 | 2 |
| ... (9 more, mostly Feb 22 cluster) | various | Feb 22 | 2 |

### Timestamp Analysis
```
Gemini watch_state since_ts:   1772849627513 ms (= 2026-03-06T20:13:47)
Oldest unindexed file mtime:   1771773044000 ms (= 2026-02-22T09:10:44)
Newest unindexed file mtime:   1772575773000 ms (= 2026-03-03T16:09:33)
```

All unindexed files have mtimes BEFORE the watch_state timestamp → incremental scans will permanently skip them.

### Root Cause
These directories (`avpstreamkit`, `groovetech-media-player`, `orchestrator`, etc.) use **project-name slugs** rather than hash-based directory names. The Gemini connector's `session_files()` method uses `WalkDir` which IS recursive, so the directory naming shouldn't matter.

Most likely: these files were created in a burst on Feb 22 but the full scan that ran at that time had a race condition or partial failure that missed some files. The watch_state timestamp then advanced past them.

---

## Gap 3: Factory — 10 Unindexed Files

### File Analysis
All 10 files are 1-line, 188-533 bytes, containing only `session_start` event:
```json
{"type": "session_start", "id": "...", "title": null, "sessionTitle": null, "owner": "...", "version": "...", "cwd": "..."}
```

No user/assistant messages → `parse_factory_session()` correctly returns `None`.

**Conclusion:** This is by design. Empty sessions created by Factory but never used.

---

## Connector Comparison: Existing DB Orphans

| Agent | Disk Files | DB Sessions | Orphan DB Entries | Notes |
|-------|-----------|-------------|-------------------|-------|
| Pi-Agent | 3,550 | 3,549 | 0 | ✅ Near-perfect (includes 2,098 via symlink) |
| Claude Code | 2,085 | 2,066 | 98 (deleted files) | 117 unindexed on disk |
| Codex | 2,143 | 3,764 | 1,621 (deleted files) | Append-only DB retains old |
| Gemini | 41 sessions | 33 | 0 | 13 unindexed on disk |
| Factory | 76 | 66 | 0 | 10 empty stubs on disk |
| OpenCode | (SQLite) | 975 | — | Can't compare |
| Cursor | (SQLite) | 229 | — | Can't compare |
| Amp | — | 2 | — | ✅ |
| Codebuff | — | 3 | — | ✅ |

---

## Key Code References

- `src/connectors/claude_code.rs:74-80` — WalkDir scan (recursive, finds subagents)
- `src/connectors/claude_code.rs:116` — `entry_type` filter: `matches!(entry_type, Some("user" | "assistant"))`
- `src/connectors/gemini.rs:159-169` — `session_files()` discovery method
- `src/connectors/gemini.rs:232` — `file_modified_since()` incremental check
- `src/connectors/factory.rs:125` — `parse_factory_session()` returns `None` for empty
- `src/indexer/mod.rs` — `watch_state.json` management, `--full` flag handling
- `src/connectors/mod.rs` — `file_modified_since()` utility
