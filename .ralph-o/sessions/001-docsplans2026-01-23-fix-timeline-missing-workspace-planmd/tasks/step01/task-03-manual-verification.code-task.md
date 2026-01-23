---
status: pending
created: 2026-01-23
started: null
completed: null
target_repo: null
---
# Task: Manual Verification and Integration Test

## Target Repository
Current repository

**IMPORTANT:** Before starting implementation, ensure you are in the correct repository.

## Description
Verify the fix works end-to-end with real data. Test both JSON output modes, verify non-JSON output still works, and confirm the `gj last` integration now shows Claude Code sessions with correct workspace filtering.

## Background
The workspace field should now be populated for sessions that have a workspace. This step validates the fix against real indexed data and confirms downstream tool compatibility. Sessions without a workspace should correctly show null (not incorrectly null like before).

## Reference Documentation
**Required:**
- Design: .ralph-o/sessions/001-docsplans2026-01-23-fix-timeline-missing-workspace-planmd/plan/design/detailed-design.md

**Note:** Review the Testing Strategy section for the expected test commands and outcomes.

## Technical Requirements
1. Optionally reindex with `cass index --full` if data is stale
2. Test JSON output with `--group-by none` mode
3. Test JSON output with `--group-by day` mode
4. Test non-JSON (human-readable) output
5. Test `gj last` integration from a project directory

## Dependencies
- Task 01 and Task 02 must be completed (code compiles and tests pass)
- Indexed conversation data in the cass database
- Access to a project directory with Claude Code sessions

## Implementation Approach
1. Run `cass index --full` if needed to ensure fresh data
2. Run `cass timeline --since 7d --json --group-by none` and filter for Claude Code sessions
3. Verify workspace field shows actual paths (not all null)
4. Run `cass timeline --since 7d --json --group-by day` and verify workspace in grouped output
5. Run `cass timeline --since 1d` to verify human-readable output works
6. Run `gj last` from a project directory to verify Claude Code sessions appear

## Acceptance Criteria

1. **JSON None Mode Shows Workspace**
   - Given indexed Claude Code sessions with workspaces
   - When running `cass timeline --since 7d --json --group-by none | jq '.sessions[] | select(.agent == "claude_code") | {workspace, source_path}' | head -10`
   - Then sessions show `"workspace": "/path/to/project"` instead of null

2. **JSON Day Mode Shows Workspace**
   - Given indexed Claude Code sessions with workspaces
   - When running `cass timeline --since 7d --json --group-by day | jq '.groups | to_entries[0].value[] | select(.agent == "claude_code") | {workspace}' | head -5`
   - Then sessions show workspace values in grouped output

3. **Non-JSON Mode Works**
   - Given any timeline query
   - When running `cass timeline --since 1d`
   - Then human-readable output displays without errors

4. **gj last Integration**
   - Given a project directory with Claude Code sessions
   - When running `gj last` from that directory
   - Then Claude Code sessions now appear (not just pi_agent sessions)

5. **Null Workspace Correct**
   - Given sessions without a workspace_id in the database
   - When querying timeline
   - Then those sessions correctly show `"workspace": null`

## Metadata
- **Complexity**: Low
- **Labels**: Verification, Integration, Manual Testing, gj
- **Required Skills**: CLI usage, jq, understanding of expected behavior
