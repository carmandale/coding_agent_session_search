---
planning: true
---
# Planning Transcript — Spec 008

**Driver:** FastRaven (pi/claude-sonnet-4-6)
**Challenger:** HappyMoon (crew-challenger)
**Date:** 2026-03-27

## Key findings from driver research (pre-collaboration)

- Fresh worktree at /tmp/cass-merge-base (origin/main) confirmed working
- Upstream schema is v14, not v13 as shaping assumed
- MigrationRunner registers only V13 (MIGRATION_FRESH_SCHEMA) + V14 — not incremental
- Our branch removed native clawdbot/copilot/openclaw (replaced with fad_adapter in spec 006)
- Unique diffs saved: watchdog.patch (957L), codebuff.patch (527L), indexer.patch (2534L), connectors_mod.patch (1224L)
- Cargo.toml: origin/main already has git deps for all franken libs; upstream reverted to path deps

## Challenges from HappyMoon (all accepted)

**Challenge 1 — Migration path is fundamentally wrong:**
MigrationRunner only registers V13 + V14. V11-V13 are NOT separate SQL migrations. V9/V10 constants exist as dead code. The actual path: transition_from_meta_version() → MigrationRunner(13=FRESH_SCHEMA, 14=fts_contentless). Not incremental v8→v9→...

**Challenge 2 — MIGRATION_FRESH_SCHEMA CREATE TABLE IF NOT EXISTS silent no-op:**
ZERO ALTER TABLE in MIGRATION_FRESH_SCHEMA (driver was wrong — the ALTER TABLEs are in dead-code V5/V7/V10 constants). But CREATE TABLE IF NOT EXISTS is a silent no-op when table exists. Our v8 conversations table will be silently skipped, leaving it without ~10 new columns (total_input_tokens etc.). Results in runtime crash "no column named total_input_tokens" without intervention. Solution: surgical ALTER TABLE gap-fill before first startup.

**Challenge 3 — PathTrie question answered:**
Upstream's connectors/mod.rs re-exports PathTrie from FAD. Our 1,180-line PathTrie implementation in connectors/mod.rs is obsolete. Only 2 method signatures (count_disk_files, reconciliation_notes) are truly unique.

**Challenge 4 — count_disk_files strategy must be decided now:**
Extension trait in src/doctor.rs. Connector trait lives in FAD — we can't modify it. DoctorConnector is our own trait.

**Challenge 5 — List all Cargo.toml path→git conversions explicitly:**
7 path deps in upstream; all need explicit git dep entries. Plan includes complete table.

## Final state

All challenges accepted. plan.md and tasks.md reflect the corrected understanding.
