---
title: "Add disk-vs-DB reconciliation check to cass doctor"
date: 2026-03-07
bead: coding_agent_session_search-2gxp
---

# Spec: Disk-vs-DB Reconciliation in `cass doctor`

## Problem

`cass doctor` currently validates infrastructure health (data dir, DB, Tantivy index, lock files, config) but has **no check for content completeness**. A perfectly healthy infrastructure can still have missing sessions — the exact situation that led to the 70 missing Claude Code subagent files and 13 orphaned Gemini sessions discovered in spec 003.

Without reconciliation, the only way to detect index gaps is manual SQL queries against the DB compared to filesystem `find` commands. Users and agents have no automated way to know sessions are missing.

## Root Cause

`cass doctor` was designed as an infrastructure diagnostic tool. It checks *can the system work* but not *has the system worked correctly*. Content completeness requires a fundamentally different check: comparing the set of files each connector would discover on disk against the set of sessions actually stored in the DB.

## Requirements

### R1: Per-connector disk-vs-DB reconciliation check

For each file-based connector, `cass doctor` must:
1. Count the total session files on disk (using the connector's own root detection and file enumeration logic)
2. Count the sessions in the DB for that agent
3. Report the delta: `disk_files - db_entries`
4. Flag as `warn` if delta > 0 (files on disk not in DB), `fail` if delta > threshold (configurable, default 10)

### R2: JSON output with per-connector breakdown

The existing `--json` output must include a new `reconciliation` object:

```json
{
  "reconciliation": {
    "balanced": false,
    "connectors": [
      {
        "agent": "claude_code",
        "disk_files": 3825,
        "db_entries": 3825,
        "delta": 0,
        "status": "pass"
      },
      {
        "agent": "gemini",
        "disk_files": 41,
        "db_entries": 28,
        "delta": 13,
        "status": "warn"
      }
    ]
  }
}
```

### R3: Human-readable output

In non-JSON mode, display reconciliation as a check like existing checks:

```
✓ reconciliation: All 12 connectors balanced (disk = DB)
```

or:

```
⚠ reconciliation: 2 connectors have gaps
  gemini: 41 on disk, 28 in DB (13 missing)
  factory: 76 on disk, 66 in DB (10 missing — likely empty stubs)
```

### R4: Scope constraint

Only scan configured connector roots (the paths each connector's `detect()` returns). Never scan arbitrary user paths. This keeps the check fast (<5s for typical installs) and predictable.

### R5: No `--fix` for reconciliation in this PR

The reconciliation check is diagnostics-only. Auto-repair (triggering `--full` reindex when gaps are found) is a follow-up. Rationale: we need to measure false-positive rates first — some connectors intentionally skip files (Factory stubs, progress-only subagent files).

### R6: Intentionally skipped files

The delta may include files the connector legitimately skips (empty sessions, progress-only files, malformed JSON). The check should NOT treat these as failures. Instead:
- Report `disk_files` as the raw count of files matching the connector's file pattern
- Report `db_entries` as the count of indexed sessions
- The delta represents "unindexed files" which may include intentional skips
- A `notes` field can provide context (e.g., "10 Factory stubs are session_start-only by design")

## Acceptance Criteria

1. `cass doctor --json` includes a `reconciliation` object with per-connector file counts
2. `cass doctor` (human mode) shows reconciliation status with connector-level detail when gaps exist
3. All 12 connectors are covered (even if some always show `disk_files: 0`)
4. The check runs in <5 seconds for a typical install (~10K sessions)
5. No content or user data is logged — only paths and counts
6. Existing doctor checks are not broken
7. Tests cover: balanced state, gap detected, connector not found on disk, JSON output format

## Non-Goals

- Auto-repair / `--fix` for reconciliation gaps (follow-up)
- Remote source reconciliation (only local sources)
- Per-file listing of missing sessions (too verbose for doctor; use SQL queries for that)
- Determining *why* files are missing (that's debugging, not diagnostics)
