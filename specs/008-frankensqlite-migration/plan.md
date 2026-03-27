---
title: "Full upstream sync: git merge upstream/main + re-apply unique additions"
date: 2026-03-27
bead: coding_agent_session_search-3iqk
---

<!-- plan:complete:v1 | harness: pi/claude-sonnet-4-6 | date: 2026-03-27T16:44:26Z -->

# Plan — Spec 008: Full Upstream Sync

## Problem recap

Our fork (carmandale/coding_agent_session_search) diverged from upstream at commit
`81f25604` ("chore: convert path deps to git deps for v0.2.0 release"). Upstream is now
274 commits ahead at v0.2.4. The gap includes frankensqlite, frankensearch, frankentui,
schema v8→v14, 19 native connectors, and significant new features. Everything in
upstream is desirable; we have 4 additions that are unique to our fork.

## Architecture decisions

### A1: Execution environment

The working branch `feat/007` has a corrupt git object (`cb78850`) that blocks ancestry
traversal. All merge work MUST occur in a fresh worktree or clone of `origin/main` to
avoid the corrupted object store. A git worktree at `origin/main` is already available
at `/tmp/cass-merge-base` (created during research). The merge branch will be pushed
back to `origin` as `feat/008-upstream-sync`.

### A2: Cargo.toml strategy

`origin/main` already has all franken libs as git deps. The Cargo.toml conflict
resolution keeps git deps, bumps to upstream's revs, and adds missing deps.

Explicit path→git conversions required (upstream uses path deps for these):

| Dep | Upstream (path) | Our target (git) |
|-----|-----------------|-----------------|
| `frankensqlite` | `path="../frankensqlite/crates/fsqlite"` | `git="…/frankensqlite", rev="68533b1", package="fsqlite"` |
| `fsqlite-types` | `path="../frankensqlite/crates/fsqlite-types"` | `git="…/frankensqlite", rev="68533b1"` |
| `franken-agent-detection` | `path="../franken_agent_detection"` | `git="…/franken_agent_detection", rev=<latest>` with crush feature |
| `asupersync` | `path="../asupersync"` | `git="…/asupersync", rev="d72f93e"` |
| `franken-decision` | `path="../asupersync/franken_decision"` | `git="…/asupersync", rev="d72f93e", package="franken-decision"` |
| `franken-evidence` | `path="../asupersync/franken_evidence"` | `git="…/asupersync", rev="d72f93e", package="franken-evidence"` |
| `franken-kernel` | `path="../asupersync/franken_kernel"` | `git="…/asupersync", rev="d72f93e", package="franken-kernel"` |

Rev bumps required (upstream moved ahead of origin/main):
- `ftui` + `ftui-runtime` + `ftui-tty` + `ftui-extras`: `18facd3` → `7a91089`
- `frankensearch`: `8b319e9` → `3eec663`

New deps to add (upstream has, origin/main lacked):
- `toon = { git = "…/toon_rust", rev = "bc3f9da", package = "tru" }`
- `portable-pty = "*"`

Remove: the `[patch."https://github.com/Dicklesworthstone/asupersync"]` section
(upstream's workaround for their path dep; not needed with git deps).

### A3: Our unique additions strategy

Only 4 items are unique to our fork. All will be re-applied as targeted diffs after the
merge, using `git diff origin/main HEAD -- <path>` which reads tree objects (intact
despite commit corruption).

| Addition | Strategy | File(s) |
|----------|----------|---------|
| Watchdog subcommand | New file — copy directly | `src/watchdog.rs` |
| Codebuff connector | New file — copy directly | `src/connectors/codebuff.rs` |
| SIGTERM/heartbeat | Targeted diff — ~20 lines | `src/indexer/mod.rs` |
| DoctorConnector trait | Extension trait in new file | `src/doctor.rs` (new) |

**fad_adapter.rs is dropped.** At origin/main, upstream had already implemented native
clawdbot.rs, copilot.rs, openclaw.rs. Our branch removed them and added fad_adapter as
spec 006. The merge restores upstream's native connectors (now 19 total). fad_adapter
is not re-applied.

**PathTrie is NOT re-applied.** Our connectors/mod.rs contained a full PathTrie
implementation (1,180 lines). Upstream moved PathTrie into the `franken_agent_detection`
crate and re-exports it from connectors/mod.rs. Our implementation is obsolete —
upstream's FAD version is used instead.

**count_disk_files strategy — DoctorConnector extension trait:**
The Connector trait lives in FAD (a separate repo we don't control). Adding
`count_disk_files` there would require a FAD PR. Instead, a separate extension trait
in `src/doctor.rs`:

```rust
// src/doctor.rs
pub trait DoctorConnector {
    fn count_disk_files(&self) -> Option<usize>;
    fn reconciliation_notes(&self) -> Option<String> { None }
}
```

The doctor subcommand uses `&dyn DoctorConnector` instead of `&dyn Connector`.
Implement for codebuff (concrete) and leave other connectors unimplemented for now
(doctor will show "N/A" for FAD-backed connectors until FAD upstream adds support).

### A4: Database migration strategy

**Critical finding:** Upstream's `MIGRATION_FRESH_SCHEMA` (the v13 migration) uses
`CREATE TABLE IF NOT EXISTS`. In SQLite, this is a **silent no-op** if the table already
exists — it does NOT check column parity. Our v8 DB has the `conversations` and
`messages` tables, so v13's CREATE TABLE statements will be silently skipped. The v13
schema adds ~10 new columns (token tracking, metadata_bin, etc.) that upstream code
writes to. Without those columns, the server will crash at runtime with
"table conversations has no column named total_input_tokens".

**Resolution: Surgical ALTER TABLE gap-fill before migration.**

Before starting the merged binary against the live 8.8GB DB, run a one-off migration
script that adds missing columns:

```sql
-- Pre-migration gap-fill: add columns present in v13 schema but absent from v8
-- Run once against live DB before first startup of merged binary.
-- All "IF NOT EXISTS" equivalent: ALTER TABLE ADD COLUMN is idempotent in SQLite
-- (will error if column exists; wrap in a migration guard or check PRAGMA table_info first)
ALTER TABLE conversations ADD COLUMN metadata_bin BLOB;
ALTER TABLE conversations ADD COLUMN total_input_tokens INTEGER;
ALTER TABLE conversations ADD COLUMN total_output_tokens INTEGER;
ALTER TABLE conversations ADD COLUMN total_cache_read_tokens INTEGER;
ALTER TABLE conversations ADD COLUMN total_cache_creation_tokens INTEGER;
ALTER TABLE conversations ADD COLUMN grand_total_tokens INTEGER;
ALTER TABLE conversations ADD COLUMN estimated_cost_usd REAL;
ALTER TABLE conversations ADD COLUMN primary_model TEXT;
ALTER TABLE messages ADD COLUMN extra_bin BLOB;
```

These must be wrapped in `PRAGMA table_info` checks or run in a try-catch, since some
columns may already exist (e.g., metadata_bin was added by upstream's own V7 migration
which ran before our v8 fork point).

**Full migration sequence for the live 8.8GB DB:**
1. `sqlite3 live.db "VACUUM INTO 'backup-pre-migration.db'"` — backup (required)
2. Run gap-fill SQL above against live.db (adds missing columns; no-ops on existing)
3. Start merged binary — `transition_from_meta_version()` runs, MigrationRunner applies V13 + V14
4. V13: CREATE TABLE IF NOT EXISTS for new tables (token_usage, message_metrics, etc.) — these succeed
5. V14: DROP + recreate fts_messages in contentless mode — succeeds
6. If any migration step fails: `cp backup-pre-migration.db live.db` to restore

**Fallback:** If gap-fill is complex or produces errors, run `cass index --full
--force-rebuild` from the merged binary. This rebuilds the 8.8GB DB from source
session files. Expected duration: 20-60 minutes depending on disk speed.

## Requirement traceability

| Req | How satisfied |
|-----|--------------|
| R0: Full parity with upstream v0.2.4 | git merge upstream/main brings all 274 commits |
| R1: Unique additions survive | watchdog.rs, codebuff.rs copied; SIGTERM patch applied; DoctorConnector extension trait |
| R2: Self-contained git deps | Every path dep converted to git dep; see Cargo.toml table above |
| R3: DB migrates safely | VACUUM INTO backup + gap-fill + MigrationRunner; fallback is full rebuild |
| R4: Watcher/watchdog/launchd continue | SIGTERM patch preserved in indexer; watchdog.rs re-applied |
| R5: Clean history | Merge commit on feat/008-upstream-sync; our additions as follow-up commits |

## Files with expected merge conflicts

Based on the diff between origin/main and upstream/main:

1. **Cargo.toml** — HIGH conflict probability. Origin/main has git deps; upstream reverted
   to path deps. Resolution: keep git dep strategy, bump revs, add new deps.
2. **src/connectors/mod.rs** — Medium. Origin/main: 55 lines. Upstream: 59 lines.
   Our HEAD: 1,176 lines (PathTrie etc. — all obsolete). Resolution: take upstream's
   59 lines; PathTrie comes from FAD; add nothing.
3. **src/indexer/mod.rs** — Medium. Both upstream and we modified this heavily.
   Resolution: take upstream's version; then apply our ~20-line SIGTERM/heartbeat patch.
4. **src/lib.rs** — High. lib.rs grew from 11,703 lines (ours) to 18,113 lines (upstream).
   Resolution: take upstream's version entirely — our additions are in separate files.
5. **src/storage/sqlite.rs** — High. Completely rewritten around frankensqlite.
   Resolution: take upstream's version entirely.

All other files: take upstream's version. Our unique code is in new files or small
targeted patches.

## Known risks and mitigations

| Risk | Mitigation |
|------|-----------|
| MIGRATION_FRESH_SCHEMA silent column omission | Surgical gap-fill before first startup |
| frankensqlite rev mismatch between origin/main and upstream | Verify build compiles; if not, bump to upstream's working rev |
| asupersync sub-crate git deps not in a usable revision | Use rev `d72f93e` from origin/main; verify at build time |
| Our SIGTERM patch conflicts with upstream's indexer changes | Apply patch manually if `git apply` fails; the logic is ~20 lines and well-understood |
| DoctorConnector breaks doctor subcommand | doctor command lists "N/A" for FAD connectors gracefully |
| Corruption in feat/007 bleeds into new branch | All work on fresh worktree/clone — corruption is isolated to feat/007's reflog |

## Out of scope

- frankentui adaptation/customization beyond what the merge brings
- Upstream contribution of watchdog subcommand or DoctorConnector trait
- FAD modification to add count_disk_files
- spec 009 (frankentui separate spec is not this work)
