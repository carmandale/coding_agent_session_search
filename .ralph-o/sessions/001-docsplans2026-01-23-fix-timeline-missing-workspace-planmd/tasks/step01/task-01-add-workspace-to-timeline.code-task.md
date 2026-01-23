---
status: pending
created: 2026-01-23
started: null
completed: null
target_repo: null
---
# Task: Add Workspace Field to Timeline Output

## Target Repository
Current repository

**IMPORTANT:** Before starting implementation, ensure you are in the correct repository.

## Description
Add the missing workspace field to the `cass timeline` command output. The timeline currently returns `"workspace": null` for all sessions despite the database correctly storing workspace information. This fix adds the workspace JOIN and field to enable downstream tools like `gj last` to properly filter sessions by workspace.

## Background
The `run_timeline` function in `src/lib.rs` queries conversation data but does not join the workspaces table. This causes all timeline output to show null for workspace, breaking tools that depend on workspace filtering. The fix follows the existing pattern used in other functions like `list_conversations` and `get_workspace_conversation_count`.

## Reference Documentation
**Required:**
- Design: .ralph-o/sessions/001-docsplans2026-01-23-fix-timeline-missing-workspace-planmd/plan/design/detailed-design.md

**Note:** You MUST read the detailed design document before beginning implementation. It contains exact line numbers and code snippets for each change.

## Technical Requirements
1. Modify SQL query to add `w.path as workspace` to SELECT and `LEFT JOIN workspaces w ON c.workspace_id = w.id`
2. Add workspace field extraction at row index 10: `row.get::<_, Option<String>>(10)?`
3. Update tuple type definition to include 11th element: `Option<String>` for workspace
4. Add `workspace` to JSON output in `TimelineGrouping::None` mode
5. Add `workspace` to JSON output in `TimelineGrouping::Hour | Day` mode
6. Add `_workspace` to non-JSON output tuple destructuring (required for compilation)

## Dependencies
- SQLite database with existing `workspaces` table and `workspace_id` foreign key on conversations
- Existing LEFT JOIN pattern from other functions in the codebase

## Implementation Approach
1. Locate the SQL query string in `run_timeline` (~line 8943) and add the workspace join/select
2. Find the row extraction closure (~line 8998) and add index 10 extraction
3. Find the tuple type definition (~line 9021) and add 11th `Option<String>` element
4. Update the `TimelineGrouping::None` closure (~line 9044) - add to destructuring and JSON object
5. Update the `TimelineGrouping::Hour | Day` for loop (~line 9080) - add to destructuring and JSON object
6. Update the non-JSON for loop (~line 9149) - add `_workspace` to destructuring

## Acceptance Criteria

1. **SQL Query Includes Workspace Join**
   - Given the `run_timeline` function
   - When the SQL query is constructed
   - Then it includes `w.path as workspace` in SELECT and `LEFT JOIN workspaces w ON c.workspace_id = w.id`

2. **Row Extraction Includes Workspace**
   - Given a database row with workspace data
   - When the row is extracted
   - Then the 11th tuple element contains the workspace path or None

3. **JSON None Mode Includes Workspace**
   - Given `--group-by none` mode with JSON output
   - When timeline data is serialized
   - Then each session object includes `"workspace": "/path/to/project"` or `"workspace": null`

4. **JSON Grouped Mode Includes Workspace**
   - Given `--group-by hour` or `--group-by day` mode with JSON output
   - When timeline data is serialized
   - Then each session object in groups includes the workspace field

5. **Non-JSON Mode Compiles**
   - Given non-JSON output mode
   - When the sessions are iterated
   - Then the 11-element tuple destructures correctly (workspace can be ignored with `_workspace`)

6. **Unit Test Coverage**
   - Given the implementation changes
   - When running the test suite
   - Then all existing tests pass (no new tests required for this minimal fix)

## Metadata
- **Complexity**: Medium
- **Labels**: Bug Fix, SQL, Timeline, Workspace, JSON
- **Required Skills**: Rust, SQL JOINs, tuple types, serde_json
