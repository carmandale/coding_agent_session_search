# Research: Codebase Analysis for Timeline Workspace Fix

**Date:** 2026-01-23

## 1. Timeline Function Verification

### Location Confirmed
- **File:** `src/lib.rs`
- **Function:** `run_timeline`
- **SQL Query:** Lines 8943-8951
- **Row Extraction:** Lines 8998-9011
- **Tuple Type:** Lines 9021-9033
- **JSON Output (None mode):** Lines 9040-9076
- **JSON Output (Hour/Day mode):** Lines 9078-9119
- **Non-JSON Output:** Lines 9149-9160

### Current State
The plan's line numbers are **accurate**. The SQL query currently:
```sql
SELECT c.id, a.slug as agent, c.title, c.started_at, c.ended_at, c.source_path,
       COUNT(m.id) as message_count, c.source_id, c.origin_host, s.kind as origin_kind
FROM conversations c
JOIN agents a ON c.agent_id = a.id
LEFT JOIN sources s ON c.source_id = s.id
LEFT JOIN messages m ON m.conversation_id = c.id
WHERE c.started_at >= ?1 AND c.started_at <= ?2
```

**Missing:** No JOIN to `workspaces` table, no `w.path as workspace` in SELECT.

## 2. Workspace Schema Verification

### Schema (`src/storage/sqlite.rs`)
```sql
-- conversations table has:
workspace_id INTEGER REFERENCES workspaces(id)

-- workspaces table has:
id INTEGER PRIMARY KEY
path TEXT NOT NULL
```

### Existing Pattern for Workspace JOIN
Multiple places in the codebase already join workspaces correctly:

**Line 975-978 (list_conversations):**
```sql
FROM conversations c
JOIN agents a ON c.agent_id = a.id
LEFT JOIN workspaces w ON c.workspace_id = w.id
```

**Line 4377 (stats):**
```sql
SELECT w.path, COUNT(*) FROM conversations c
JOIN workspaces w ON c.workspace_id = w.id...
```

**Line 6118-6126 (context):**
Fetches workspace separately by ID - but timeline should use JOIN for efficiency.

## 3. gj last-sessions Analysis

### Location
- **File:** `~/bin/gj` (shell script)
- **Function:** `cmd_last_sessions` (lines 2135-2350)

### How It Consumes Timeline Output
```bash
cass_cmd="cass timeline --since 7d --json --group-by none"
```

### Current Workspace Filtering (Workaround)
Since `workspace` field is null, the function uses `source_path` string matching:
```jq
.sessions | map(select(
    (.source_path | contains($ws)) or
    ((.title // "") | capture("/(?<last>[^/]+)$") | .last as $last | $ws | startswith($last))
))
```

### Expected Behavior After Fix
With the `workspace` field available, filtering could be simplified to:
```jq
.sessions | map(select(.workspace | endswith($ws)))
```

Or kept backward-compatible with the current approach plus workspace matching.

## 4. Test Coverage

### Timeline Tests
- **No dedicated timeline tests found** in `tests/` directory
- Tests exist for storage operations but not for `run_timeline` specifically

### Recommendation
Consider adding a test case for timeline workspace output, but this is outside the scope of the bug fix.

## 5. Related Code (No Changes Needed)

### analytics.rs
The `src/pages/analytics.rs` file is for pre-computed analytics during export - it's a **different timeline** than `cass timeline`. No changes needed there.

### Other Functions
The `run_context` function (line 6118+) already properly includes workspace in its output. It uses a separate query, but the timeline function should use JOIN for better performance.

## 6. Confirmed Implementation Plan

All 6 changes from the rough idea are **validated**:

| Change | Location | Verification |
|--------|----------|--------------|
| 1. SQL Query | Line 8943 | Confirmed - need to add `LEFT JOIN workspaces w ON c.workspace_id = w.id` and `w.path as workspace` |
| 2. Row Extraction | Line 8998 | Confirmed - 10 columns now, need to add 11th |
| 3. Tuple Type | Line 9021 | Confirmed - need to add `Option<String>` for workspace |
| 4. JSON None Mode | Line 9044 | Confirmed - need to add workspace to destructure and output |
| 5. JSON Hour/Day Mode | Line 9080 | Confirmed - need to add workspace to destructure and output |
| 6. Non-JSON Mode | Line 9149 | Confirmed - need to add `_workspace` to tuple destructure |

## Key Findings

1. **Line numbers accurate** - The plan's code locations are correct
2. **Pattern exists** - Other code already JOINs workspaces correctly
3. **gj workaround confirmed** - Uses source_path matching due to missing workspace field
4. **No blocking issues** - Straightforward 6-change implementation
5. **Performance safe** - LEFT JOIN on indexed foreign key is minimal overhead
