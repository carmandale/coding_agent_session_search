# Rough Idea: Fix Timeline Missing Workspace

**Source:** docs/plans/2026-01-23-fix-timeline-missing-workspace-plan.md
**Date:** 2026-01-23

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

## Proposed Solution

Add workspace support to the timeline command through 6 code changes:
1. SQL Query - Add workspace join and select
2. Row Extraction - Add workspace field (index 10)
3. Tuple Type Definition - Add Option<String> for workspace
4. JSON Output (TimelineGrouping::None) - Add workspace field
5. JSON Output (TimelineGrouping::Hour | Day) - Add workspace field
6. Non-JSON Output - Update tuple destructuring (compile fix)

## Acceptance Criteria

- `cass timeline --json` includes `"workspace": "/path/to/project"` for sessions with workspace
- `cass timeline --json` includes `"workspace": null` only for sessions without workspace
- Both `--group-by none` and `--group-by hour/day` modes include workspace
- `gj last` shows Claude Code sessions when run from a project with matching workspace
- Existing tests pass
- No performance regression (single additional LEFT JOIN)
