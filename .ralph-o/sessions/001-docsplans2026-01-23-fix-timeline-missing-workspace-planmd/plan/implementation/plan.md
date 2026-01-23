# Implementation Plan: Fix Timeline Missing Workspace

**Date:** 2026-01-23
**Type:** Bug Fix
**Design:** [detailed-design.md](../design/detailed-design.md)

---

## Implementation Checklist

- [ ] Step 1: Modify SQL query to join workspaces table
- [ ] Step 2: Update row extraction and tuple type
- [ ] Step 3: Add workspace to JSON output (both modes)
- [ ] Step 4: Fix non-JSON tuple destructuring
- [ ] Step 5: Verify compilation and tests
- [ ] Step 6: Manual verification and integration test

---

## Step 1: Modify SQL Query to Join Workspaces Table

### Objective
Add the workspace table JOIN and SELECT to the timeline SQL query.

### Implementation Guidance

**File:** `src/lib.rs` ~line 8943

Modify the SQL string to:
1. Add `w.path as workspace` to the SELECT clause (after `s.kind as origin_kind`)
2. Add `LEFT JOIN workspaces w ON c.workspace_id = w.id` after the sources JOIN

**Current:**
```rust
let mut sql = String::from(
    "SELECT c.id, a.slug as agent, c.title, c.started_at, c.ended_at, c.source_path,
            COUNT(m.id) as message_count, c.source_id, c.origin_host, s.kind as origin_kind
     FROM conversations c
     JOIN agents a ON c.agent_id = a.id
     LEFT JOIN sources s ON c.source_id = s.id
     LEFT JOIN messages m ON m.conversation_id = c.id
     WHERE c.started_at >= ?1 AND c.started_at <= ?2",
);
```

**Target:**
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

### Test Requirements
Code will not compile until Step 2-4 complete. No isolated test possible.

### Integration
This is the foundation - subsequent steps depend on this query change.

### Demo
N/A - Must complete through Step 5 for demoable result.

---

## Step 2: Update Row Extraction and Tuple Type

### Objective
Modify the row extraction to include workspace (index 10) and update the tuple type definition.

### Implementation Guidance

**File:** `src/lib.rs`

**Part A: Row extraction (~line 8998)**

Add the 11th field extraction after `origin_kind`:

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
            row.get::<_, String>(7)?,         // source_id
            row.get::<_, Option<String>>(8)?, // origin_host
            row.get::<_, Option<String>>(9)?, // origin_kind
            row.get::<_, Option<String>>(10)?, // workspace <-- ADD
        ))
    })
```

**Part B: Tuple type definition (~line 9021)**

Add `Option<String>` as the 11th type:

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
    Option<String>,  // workspace <-- ADD
)> = Vec::new();
```

### Test Requirements
Code will not compile until Step 3-4 complete.

### Integration
Builds on Step 1 query; enables Step 3 JSON output.

### Demo
N/A - Must complete through Step 5.

---

## Step 3: Add Workspace to JSON Output (Both Modes)

### Objective
Include the workspace field in both JSON output modes (None and Hour/Day grouping).

### Implementation Guidance

**File:** `src/lib.rs`

**Part A: TimelineGrouping::None (~line 9044)**

Update the destructuring pattern and JSON object:

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
    workspace,  // <-- ADD
)| {
    let duration = ended.map(|e| e - started);
    let kind = origin_kind.as_deref().unwrap_or("local");
    serde_json::json!({
        "id": id, "agent": agent, "title": title,
        "started_at": started, "ended_at": ended,
        "duration_seconds": duration, "source_path": path,
        "message_count": msg_count,
        "workspace": workspace,  // <-- ADD
        "source_id": source_id,
        "origin_kind": kind,
        "origin_host": origin_host,
    })
},
```

**Part B: TimelineGrouping::Hour | Day (~line 9080)**

Update the for loop destructuring and JSON object:

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
    workspace,  // <-- ADD
) in &sessions
{
    // ... existing dt/key logic unchanged ...
    let kind = origin_kind.as_deref().unwrap_or("local");
    groups.entry(key).or_default().push(serde_json::json!({
        "id": id, "agent": agent, "title": title,
        "started_at": started, "ended_at": ended,
        "source_path": path, "message_count": msg_count,
        "workspace": workspace,  // <-- ADD
        "source_id": source_id,
        "origin_kind": kind,
        "origin_host": origin_host,
    }));
}
```

### Test Requirements
Code will not compile until Step 4 complete.

### Integration
Builds on Step 2 tuple definition.

### Demo
N/A - Must complete through Step 5.

---

## Step 4: Fix Non-JSON Tuple Destructuring

### Objective
Update the non-JSON output loop to destructure the 11-element tuple (required for compilation).

### Implementation Guidance

**File:** `src/lib.rs` ~line 9149

Add `_workspace` to the destructuring pattern (prefixed with underscore since it's not displayed):

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
    _workspace,  // <-- ADD (required for tuple destructuring)
) in &sessions
```

### Test Requirements
After this step, code should compile.

### Integration
Completes the data flow for all output modes.

### Demo
N/A - Must complete Step 5 to verify.

---

## Step 5: Verify Compilation and Tests

### Objective
Ensure all changes compile without errors/warnings and existing tests pass.

### Implementation Guidance

Run the following commands in order:

```bash
# 1. Check for compiler errors
cargo check --all-targets

# 2. Check for clippy lints
cargo clippy --all-targets -- -D warnings

# 3. Verify formatting
cargo fmt --check

# 4. Run existing tests
cargo test
```

**If errors occur:**
- Read the error message carefully
- Check line numbers match the expected locations
- Verify tuple element count matches (11 elements)
- Ensure all destructuring patterns have 11 variables

### Test Requirements
All existing tests must pass with exit code 0.

### Integration
Validates all previous steps work together.

### Demo
```bash
cargo check --all-targets && echo "✅ Compilation successful"
cargo test && echo "✅ All tests pass"
```

---

## Step 6: Manual Verification and Integration Test

### Objective
Verify the fix works end-to-end with real data.

### Implementation Guidance

**Part A: Reindex (if needed)**
```bash
cass index --full
```

**Part B: Test JSON output (None mode)**
```bash
cass timeline --since 7d --json --group-by none | \
  jq '.sessions[] | select(.agent == "claude_code") | {workspace, source_path}' | head -10
```

Expected: Sessions show `"workspace": "/path/to/project"` instead of null.

**Part C: Test JSON output (Day mode)**
```bash
cass timeline --since 7d --json --group-by day | \
  jq '.groups | to_entries[0].value[] | select(.agent == "claude_code") | {workspace}' | head -5
```

Expected: Same workspace values appear in grouped output.

**Part D: Test non-JSON output (regression check)**
```bash
cass timeline --since 1d
```

Expected: Human-readable output works without errors.

**Part E: Integration test with gj last**
```bash
cd ~/dev/orchestrator  # or any project directory
gj last
```

Expected: Claude Code sessions now appear (not just pi_agent sessions).

### Test Requirements
All manual tests should show expected results.

### Integration
Validates the complete fix including downstream tool compatibility.

### Demo
```bash
# Quick one-liner to verify the fix
cass timeline --since 1d --json | jq '.sessions[0] | {agent, workspace}'
```

Should output something like:
```json
{
  "agent": "claude_code",
  "workspace": "/Users/user/Projects/my-project"
}
```

---

## Notes

### All-or-Nothing Changes
Steps 1-4 must be completed together for the code to compile. They are separated for clarity but should be implemented in a single editing session.

### Rollback
If issues arise, all changes are in a single file (`src/lib.rs`) and can be reverted with:
```bash
git checkout src/lib.rs
```

### Performance
The LEFT JOIN on `workspace_id` uses an indexed foreign key. Expected overhead: negligible (<1ms per query).
