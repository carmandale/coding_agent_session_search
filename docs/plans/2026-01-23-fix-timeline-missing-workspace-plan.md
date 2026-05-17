---
title: "fix: Timeline command returns null workspace for Claude Code sessions"
type: fix
date: 2026-01-23
---

# 🐛 fix: Timeline command returns null workspace for Claude Code sessions

## Problem Statement

The `cass timeline` command returns `"workspace": null` for all Claude Code sessions even though the database correctly stores the workspace information. This breaks downstream tools like `gj last` that filter sessions by workspace.

**Symptoms observed:**
- `gj last` shows only pi_agent sessions, not Claude Code sessions
- `cass timeline --json` returns `workspace: null` for Claude Code entries
- Database query confirms `workspace_id` is correctly populated in `conversations` table

## Root Cause

The `run_timeline` function in `src/lib.rs` (lines 8943-9200) does not:
1. JOIN the `workspaces` table
2. SELECT the workspace path
3. Include the workspace field in JSON output (both grouping modes)

**Current SQL query (missing workspace):**
```sql
SELECT c.id, a.slug as agent, c.title, c.started_at, c.ended_at, c.source_path,
       COUNT(m.id) as message_count, c.source_id, c.origin_host, s.kind as origin_kind
FROM conversations c
JOIN agents a ON c.agent_id = a.id
LEFT JOIN sources s ON c.source_id = s.id
LEFT JOIN messages m ON m.conversation_id = c.id
WHERE c.started_at >= ?1 AND c.started_at <= ?2
```

## Acceptance Criteria

- [ ] `cass timeline --json` includes `"workspace": "/path/to/project"` for sessions with workspace
- [ ] `cass timeline --json` includes `"workspace": null` only for sessions without workspace (correctly null)
- [ ] Both `--group-by none` and `--group-by hour/day` modes include workspace
- [ ] `gj last` shows Claude Code sessions when run from a project with matching workspace
- [ ] Existing tests pass
- [ ] No performance regression (single additional LEFT JOIN)

## Implementation - 6 Changes Required

### Change 1: SQL Query - Add workspace join and select

**File:** `src/lib.rs` ~line 8943

```rust
    let mut sql = String::from(
        "SELECT c.id, a.slug as agent, c.title, c.started_at, c.ended_at, c.source_path,
                COUNT(m.id) as message_count, c.source_id, c.origin_host, s.kind as origin_kind,
                w.path as workspace
         FROM conversations c
         JOIN agents a ON c.agent_id = a.id
         LEFT JOIN sources s ON c.source_id = s.id
         LEFT JOIN messages m ON m.conversation_id = c.id
         LEFT JOIN workspaces w ON c.workspace_id = w.id
         WHERE c.started_at >= ?1 AND c.started_at <= ?2",
    );
```

### Change 2: Row Extraction - Add workspace field (index 10)

**File:** `src/lib.rs` ~line 8998

```rust
    let rows = stmt
        .query_map(param_refs.as_slice(), |row| {
            Ok((
                row.get::<_, i64>(0)?,            // id
                row.get::<_, String>(1)?,         // agent
                row.get::<_, Option<String>>(2)?, // title
                row.get::<_, i64>(3)?,            // started_at
                row.get::<_, Option<i64>>(4)?,    // ended_at
                row.get::<_, String>(5)?,         // source_path
                row.get::<_, i64>(6)?,            // message_count
                row.get::<_, String>(7)?,         // source_id (P3.2)
                row.get::<_, Option<String>>(8)?, // origin_host (P3.5)
                row.get::<_, Option<String>>(9)?, // origin_kind (P3.5)
                row.get::<_, Option<String>>(10)?, // workspace
            ))
        })
```

### Change 3: Tuple Type Definition - Add Option<String> for workspace

**File:** `src/lib.rs` ~line 9021

```rust
    #[allow(clippy::type_complexity)]
    let mut sessions: Vec<(
        i64,
        String,
        Option<String>,
        i64,
        Option<i64>,
        String,
        i64,
        String,
        Option<String>,
        Option<String>,
        Option<String>,  // workspace
    )> = Vec::new();
```

### Change 4: JSON Output (TimelineGrouping::None) - Add workspace field

**File:** `src/lib.rs` ~line 9044

Add `workspace` to the destructuring pattern and JSON output:

```rust
                        |(
                            id,
                            agent,
                            title,
                            started,
                            ended,
                            path,
                            msg_count,
                            source_id,
                            origin_host,
                            origin_kind,
                            workspace,  // ADD THIS
                        )| {
                            let duration = ended.map(|e| e - started);
                            // Use "local" as default origin_kind if not in DB (backward compat)
                            let kind = origin_kind.as_deref().unwrap_or("local");
                            serde_json::json!({
                                "id": id, "agent": agent, "title": title,
                                "started_at": started, "ended_at": ended,
                                "duration_seconds": duration, "source_path": path,
                                "message_count": msg_count,
                                "workspace": workspace,  // ADD THIS
                                // Provenance fields (P3.5)
                                "source_id": source_id,
                                "origin_kind": kind,
                                "origin_host": origin_host,
                            })
                        },
```

### Change 5: JSON Output (TimelineGrouping::Hour | Day) - Add workspace field

**File:** `src/lib.rs` ~line 9080

Add `workspace` to the destructuring pattern and JSON output:

```rust
                for (
                    id,
                    agent,
                    title,
                    started,
                    ended,
                    path,
                    msg_count,
                    source_id,
                    origin_host,
                    origin_kind,
                    workspace,  // ADD THIS
                ) in &sessions
                {
                    // ... existing dt/key logic unchanged ...
                    let kind = origin_kind.as_deref().unwrap_or("local");
                    groups.entry(key).or_default().push(serde_json::json!({
                        "id": id, "agent": agent, "title": title,
                        "started_at": started, "ended_at": ended,
                        "source_path": path, "message_count": msg_count,
                        "workspace": workspace,  // ADD THIS
                        // Provenance fields (P3.5)
                        "source_id": source_id,
                        "origin_kind": kind,
                        "origin_host": origin_host,
                    }));
                }
```

### Change 6: Non-JSON Output - Update tuple destructuring (compile fix)

**File:** `src/lib.rs` ~line 9149

Add `_workspace` to match the new 11-element tuple (not displayed, just needed for destructuring):

```rust
        for (
            _id,
            agent,
            title,
            started,
            ended,
            _path,
            msg_count,
            source_id,
            origin_host,
            _origin_kind,
            _workspace,  // ADD THIS - required for tuple destructuring to compile
        ) in &sessions
```

## Testing

```bash
# Build and verify
cargo check --all-targets
cargo clippy --all-targets -- -D warnings

# Reindex and test
cass index --full

# Test both grouping modes
cass timeline --since 7d --json --group-by none | jq '.sessions[] | select(.agent == "claude_code") | {workspace, source_path}' | head -10
cass timeline --since 7d --json --group-by day | jq '.groups | to_entries[0].value[] | select(.agent == "claude_code") | {workspace}' | head -5

# Verify non-JSON mode still works
cass timeline --since 1d

# Verify gj last works
cd ~/dev/orchestrator
gj last  # Should now show Claude Code sessions
```

## References

- Bug discovered: Current session debugging Claude Code visibility in `gj last`
- Database schema: `src/storage/sqlite.rs` - workspaces table
- Function location: `src/lib.rs:8884` - `fn run_timeline`
