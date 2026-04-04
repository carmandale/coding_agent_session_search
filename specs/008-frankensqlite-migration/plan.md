---
title: "Full upstream sync: git merge upstream/main + re-apply unique additions"
date: 2026-03-27
bead: coding_agent_session_search-3iqk
---
<!-- Codex Review: APPROVED after 5 rounds | model: gpt-5.3-codex | date: 2026-03-27 -->
<!-- Status: REVISED -->
<!-- Revisions: (1) extended gap-fill to 13 columns including api_call_count/tool_call_count/user_message_count/assistant_message_count; (2) explicit watchdog wiring (5 sites) and codebuff wiring (4 sites); (3) SIGTERM/heartbeat marked unconditional reapply; (4) FAD pinned to rev bb3e6132; (5) quiesce protocol added; (6) migration trigger corrected to cass analytics rebuild (FrankenStorage::open path); (7) build gate uses set -euo pipefail; (8) add_col rewritten to if-form for set -e compatibility; (9) all failure paths hard-exit with rollback -->

<!-- plan:complete:v1 | harness: pi/claude-sonnet-4-6 | date: 2026-03-27T16:44:26Z -->

# Plan — Spec 008: Full Upstream Sync (Codex-Approved)

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

### A3: Our unique additions — explicit integration points

**Important**: copying files is not sufficient. Upstream's lib.rs, indexer/mod.rs,
and connectors/mod.rs have no watchdog or codebuff wiring. These patches must be
applied manually after taking upstream's versions.

**Watchdog — 5 wiring sites in upstream lib.rs (upstream has no watchdog):**
1. Top: `pub mod watchdog;`
2. Commands enum: `Watchdog { command: Option<watchdog::WatchdogCommand> }` variant
3. Command dispatch: `Commands::Watchdog { command } => crate::watchdog::run_watchdog_command(command)...`
4. robot_mode match: `Some(Commands::Watchdog { .. }) => "watchdog".to_string()`
5. health JSON: `"watchdog": { plist_installed: ..., watcher_plist_installed: ... }` field

**Codebuff — 4 wiring sites in connectors/mod.rs + indexer/mod.rs:**
1. `src/connectors/mod.rs`: `pub mod codebuff;`
2. `src/indexer/mod.rs` imports: `codebuff::CodebuffConnector`
3. `src/indexer/mod.rs` factory: `("codebuff", || Box::new(CodebuffConnector::new()))`
4. `src/indexer/mod.rs` AgentKind: `Codebuff` enum variant and all match arms

**SIGTERM/heartbeat/PID — unconditional reapply (confirmed absent in both origin/main and upstream/main):**
- signal_hook SIGTERM/SIGINT flag registration
- `write_heartbeat()` function and heartbeat interval loop
- PID file write at startup; PID file cleanup at shutdown
- Replace `rx.recv()` with `rx.recv_timeout(heartbeat_interval)`

**DoctorConnector extension trait — new src/doctor.rs:**
The Connector trait lives in FAD (a separate repo we don't control). Adding
`count_disk_files` there would require a FAD PR. Instead:

```rust
// src/doctor.rs
pub trait DoctorConnector {
    fn count_disk_files(&self) -> Option<usize>;
    fn reconciliation_notes(&self) -> Option<String> { None }
}
```

The doctor subcommand uses `&dyn DoctorConnector`. FAD-backed connectors show "N/A".

**File copies (new files, no upstream equivalent):**
- `src/watchdog.rs` (951 lines, macOS-only)
- `src/connectors/codebuff.rs` (521 lines)
- `src/doctor.rs` (new)

**Dropped:** fad_adapter.rs (upstream's 19 native connectors replace it).
**PathTrie NOT re-applied** — upstream moved it into FAD, re-exported from connectors/mod.rs.

### A4: Database migration — build gate, quiesce, gap-fill, migrate, verify

**All steps use `set -euo pipefail`. Every failure exits non-zero immediately.
No soft-fail anywhere in the migration flow.**

**Step 0 — Build gate (must succeed before any DB mutation):**
```bash
set -euo pipefail
cd /tmp/cass-merge-base
cargo build --release
cargo test
echo "Build gate: PASSED"
```

**Step 1 — Quiesce (hard stop before any DB operations):**
```bash
DB="$HOME/Library/Application Support/com.coding-agent-search.coding-agent-search/agent_search.db"
launchctl unload ~/Library/LaunchAgents/com.cass.index-watch.plist || true
launchctl unload ~/Library/LaunchAgents/com.cass.health-watchdog.plist || true
sleep 3
if pgrep -f "cass index --watch" > /dev/null; then
  echo "FATAL: watcher still running after unload"; exit 1
fi
echo "watcher stopped OK"
```

**Step 2 — Pre-migration integrity check:**
```bash
sqlite3 "$DB" "PRAGMA integrity_check;" | grep -q "^ok$" \
  || { echo "FATAL: DB integrity check failed before migration"; exit 1; }
CONV_COUNT=$(sqlite3 "$DB" "SELECT COUNT(*) FROM conversations;")
echo "Pre-migration integrity OK, conversations: $CONV_COUNT"
```

**Step 3 — VACUUM INTO backup (exact path in variable):**
```bash
BACKUP_PATH="${HOME}/cass-backup-pre-v14-$(date +%Y%m%d-%H%M%S).db"
sqlite3 "$DB" "VACUUM INTO '${BACKUP_PATH}'"
sqlite3 "$BACKUP_PATH" "PRAGMA integrity_check;" | grep -q "^ok$" \
  || { echo "FATAL: backup integrity check failed"; exit 1; }
echo "Backup created and verified: $BACKUP_PATH"
```

**Step 4 — Surgical gap-fill (13 columns; set -e safe form):**

`MIGRATION_FRESH_SCHEMA` uses `CREATE TABLE IF NOT EXISTS` — silent no-op on existing
tables. Our v8 conversations/messages tables will be missing columns that upstream code
writes to. The `if err=$(...)` form is used because command substitution inside `if`
is exempt from `set -e`'s automatic exit, keeping the duplicate-column branch reachable.

```bash
add_col() {
  local sql="$1" err
  if err=$(sqlite3 "$DB" "$sql" 2>&1); then
    echo "OK: $sql"
  elif echo "$err" | grep -q "duplicate column name"; then
    echo "SKIP (exists): $sql"
  else
    echo "FATAL: $sql — $err"; exit 1
  fi
}

add_col "ALTER TABLE conversations ADD COLUMN metadata_bin BLOB"
add_col "ALTER TABLE messages ADD COLUMN extra_bin BLOB"
add_col "ALTER TABLE conversations ADD COLUMN total_input_tokens INTEGER"
add_col "ALTER TABLE conversations ADD COLUMN total_output_tokens INTEGER"
add_col "ALTER TABLE conversations ADD COLUMN total_cache_read_tokens INTEGER"
add_col "ALTER TABLE conversations ADD COLUMN total_cache_creation_tokens INTEGER"
add_col "ALTER TABLE conversations ADD COLUMN grand_total_tokens INTEGER"
add_col "ALTER TABLE conversations ADD COLUMN estimated_cost_usd REAL"
add_col "ALTER TABLE conversations ADD COLUMN primary_model TEXT"
add_col "ALTER TABLE conversations ADD COLUMN api_call_count INTEGER"
add_col "ALTER TABLE conversations ADD COLUMN tool_call_count INTEGER"
add_col "ALTER TABLE conversations ADD COLUMN user_message_count INTEGER"
add_col "ALTER TABLE conversations ADD COLUMN assistant_message_count INTEGER"
echo "Gap-fill complete"
```

**Step 5 — Trigger MigrationRunner via `cass analytics rebuild --json`:**

Confirmed call chain (upstream source):
- `cass analytics rebuild` → `run_analytics_rebuild()` (lib.rs:3724,3941)
- → `FrankenStorage::open()` (lib.rs:3969)
- → `run_migrations()` (storage/sqlite.rs:2636)
- → `transition_from_meta_version()` + MigrationRunner (V13 + V14)

Note: `cass doctor` uses raw `frankensqlite::Connection::open()` — does NOT trigger migrations.
Note: `cass health` is read-only — does NOT trigger migrations.

```bash
./target/release/cass analytics rebuild --json \
  || { echo "FATAL: migration trigger failed — restoring backup"; cp "$BACKUP_PATH" "$DB"; exit 1; }
echo "Migration via analytics rebuild: complete"
```

**Step 6 — Post-migration verification (hard-fail = rollback + exit):**
```bash
rollback() { echo "FATAL: $1 — restoring backup"; cp "$BACKUP_PATH" "$DB"; exit 1; }

sqlite3 "$DB" "PRAGMA integrity_check;" | grep -q "^ok$" \
  || rollback "DB integrity check failed after migration"

POST_COUNT=$(sqlite3 "$DB" "SELECT COUNT(*) FROM conversations;")
[ "$POST_COUNT" -eq "$CONV_COUNT" ] \
  || rollback "conversation count changed $CONV_COUNT → $POST_COUNT"

sqlite3 "$DB" "SELECT COUNT(*) FROM token_usage;" > /dev/null \
  || rollback "token_usage table missing"
sqlite3 "$DB" "SELECT COUNT(*) FROM message_metrics;" > /dev/null \
  || rollback "message_metrics table missing"

echo "Post-migration verification: PASSED ($POST_COUNT conversations, new tables present)"
```

**Step 7 — Restart services (hard gate for R4):**
```bash
launchctl load ~/Library/LaunchAgents/com.cass.index-watch.plist
launchctl load ~/Library/LaunchAgents/com.cass.health-watchdog.plist
sleep 5
pgrep -f "cass index --watch" > /dev/null \
  || { echo "FATAL: watcher did not start after launchctl load"; exit 1; }
echo "Watcher running: $(pgrep -f 'cass index --watch')"
./target/release/cass watchdog run \
  || { echo "FATAL: watchdog run subcommand failed"; exit 1; }
echo "Watchdog smoke check: PASSED"
```

## Requirement traceability

| Req | How satisfied |
|-----|--------------|
| R0: Full parity with upstream v0.2.4 | git merge upstream/main brings all 274 commits |
| R1: Unique additions survive | watchdog.rs+codebuff.rs copied; 5+4+4 explicit wiring patches; SIGTERM unconditional; DoctorConnector |
| R2: Self-contained git deps | 7 path→git conversions, all pinned SHAs |
| R3: DB migrates safely | Build gate → quiesce → integrity → backup → 13-col gap-fill (set-e-safe) → FrankenStorage::open via analytics rebuild → post-verify with hard rollback |
| R4: Watcher/watchdog/launchd | Hard gate watcher restart + watchdog smoke check; SIGTERM/heartbeat unconditional patch |
| R5: Clean history | Single merge commit + follow-up commits for our 4 additions |

## Files with expected merge conflicts

1. **Cargo.toml** — HIGH. Origin/main has git deps; upstream reverted to path deps.
   Resolution: keep git dep strategy, bump revs, add new deps per A2 table.
2. **src/connectors/mod.rs** — Medium. Origin/main: 55L. Upstream: 59L. Our HEAD: 1,176L.
   Resolution: take upstream's 59 lines; PathTrie comes from FAD; add only codebuff wiring.
3. **src/indexer/mod.rs** — Medium. Both modified heavily.
   Resolution: take upstream's version; apply SIGTERM/heartbeat + codebuff patches.
4. **src/lib.rs** — High. 18,113L upstream. Take upstream entirely; apply watchdog wiring (5 sites).
5. **src/storage/sqlite.rs** — High. Completely rewritten for frankensqlite. Take upstream entirely.

All other files: take upstream's version.

## Known risks and mitigations

| Risk | Mitigation |
|------|-----------|
| MIGRATION_FRESH_SCHEMA silent column omission | 13-column gap-fill (Step 4); hard-fail on unknown errors |
| Concurrent writer during ALTER TABLE | Explicit quiesce (Step 1) before any DB mutation |
| frankensqlite rev mismatch | Verify build; bump rev if needed |
| asupersync sub-crate git deps | Use rev d72f93e from origin/main; verify at build time |
| Watchdog wiring conflicts | 5 precise sites documented; apply surgically |
| Codebuff registration conflicts | 4 precise sites in indexer/mod.rs; apply surgically |
| SIGTERM patch confirmed absent upstream | Unconditional reapply; no "check if present" step |
| feat/007 corruption | All work on fresh worktree; corruption isolated to feat/007 reflog |

## Out of scope

- frankentui adaptation/customization beyond what the merge brings
- Upstream contribution of watchdog subcommand or DoctorConnector trait
- FAD modification to add count_disk_files
- spec 009 (frankentui is a separate spec)
