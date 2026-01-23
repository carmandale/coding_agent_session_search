# Fix Timeline Missing Workspace

## Objective
Add the missing workspace field to `cass timeline` JSON output so downstream tools like `gj last` can properly filter sessions by workspace.

## Code Tasks
Execute the following tasks in order from `.ralph-o/sessions/001-docsplans2026-01-23-fix-timeline-missing-workspace-planmd/tasks/step01/`:

1. **task-01-add-workspace-to-timeline.code-task.md** - Core implementation
   - Modify SQL query to JOIN workspaces table
   - Update row extraction and tuple type (10 → 11 elements)
   - Add workspace to JSON output (both None and Hour/Day modes)
   - Fix non-JSON tuple destructuring

2. **task-02-verify-build-and-tests.code-task.md** - Validation checkpoint
   - Run cargo check, clippy, fmt, test
   - All must pass before proceeding

3. **task-03-manual-verification.code-task.md** - End-to-end verification
   - Test JSON output modes with jq
   - Verify gj last integration

## Design Reference
Read `.ralph-o/sessions/001-docsplans2026-01-23-fix-timeline-missing-workspace-planmd/plan/design/detailed-design.md` for exact line numbers and code snippets.

## Acceptance Criteria
- [ ] `cass timeline --json` includes workspace field with actual paths
- [ ] Both `--group-by none` and `--group-by day` modes include workspace
- [ ] All existing tests pass
- [ ] `gj last` shows Claude Code sessions when run from a project directory
