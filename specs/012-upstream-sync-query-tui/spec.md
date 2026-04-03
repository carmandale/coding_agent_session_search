---
title: "Spec 012: Upstream Sync — frankensqlite 1GB fix, query engine, semantic policy, TUI overhaul"
date: 2026-04-03
bead: coding_agent_session_search-1e57
---

<!-- issue:complete:v1 | harness: unknown | date: 2026-04-03T14:46:57Z -->

## Purpose

Bring the fork current with upstream HEAD, prioritising the frankensqlite page-buffer
fix that resolves the `26of` OOM crash loop.  The upstream has landed ~2089 commits
since our last sync (spec 011 target).

---

## Why now

Upstream commit `50eac478` pins frankensqlite to `ff6a114b` with the description
**"1GB page buffer default"**.  This changes `page_cache_max_buffers()` from the
previous hardcoded 65,536 pages (256 MB) to a 1 GB default, directly resolving bead
`26of` — the frankensqlite OOM that has been causing MVCC FK mismatches, page-lock
deadlocks, and WAL corruption symptoms in our watcher.  No upstream response on
issue #57 is needed; the fix is already in-tree.

Additional upstream value: analytics multi-dimension breakdown, source-filter
normalization, `--source` flag for cross-source disambiguation, infinite-OOM-loop fix
on killed incremental scans, and 3 new source files in the search pipeline
(`policy.rs`, `semantic_manifest.rs`, `refresh_ledger.rs`).

---

## Scope

### In scope

| Phase | What | Why |
|-------|------|-----|
| **T0** | Bump frankensqlite `dd9b457` → `ff6a114b`; add `fsqlite-types` dep | Fixes 26of — highest value, minimal risk |
| **T1** | Bump frankensearch `3eec663` → `9961c0e7` | Search pipeline improvements |
| **T2** | Copy 3 new source files verbatim from upstream | Pure additions, no conflict |
| **T3** | Merge large-file updates (lib.rs +3035, app.rs +3727, query.rs +1757, indexer/mod.rs +671, storage/sqlite.rs +587, model_download.rs +685) | Requires preserving our local changes |
| **T4** | Bump FAD `c5d3273c` → `ba9c598` | Connector improvements; opencode feature **stays removed** |
| **T5** | Cargo check + clippy + lib tests | Gate before deploy |
| **T6** | Build release binary, deploy, verify watcher 26of is gone | Acceptance |

### Out of scope

- Upstream `.beads/migration_baseline/` directories (their tracker artefacts)
- Re-enabling the opencode connector (disabled in T4 — we keep it stubbed)
- Re-enabling the amp connector (same)
- Changing our version string (`0.2.7-gj.1` → `0.2.9-gj.1` after sync)
- Any spec 011 workarounds that are superseded by the frankensqlite fix

---

## Our local changes that MUST be preserved

These are in our fork and absent from upstream's verbatim files.

### `src/lib.rs`
- 6 `Commands::Watchdog` dispatch sites (spec 007 watchdog subcommand wiring)
- `libc = "*"` dep required by watchdog

### `src/storage/sqlite.rs`
- Removed `LIMIT 1000` from `franken_existing_message_fingerprints_by_idx` (spec 011 fix)
- Removed `LIMIT 100` from `franken_existing_message_replay_fingerprints` (same)
- `seen_idx` HashSet guard in fresh insert path
- `ForeignKeyViolation` catch in `franken_insert_message` → returns `Ok(None)`
- `let Some(msg_id) = ... else { continue }` pattern at 6 call sites
- Context wrappers on `franken_insert_snippets` and `franken_insert_conversation`

### `src/indexer/mod.rs`
- WAL seed write at top of `reindex_paths()` (`storage.try_lock()` → `set_last_indexed_at`)
- WAL seed write before entering watch mode

### `src/connectors/opencode.rs`
- Full stub (must stay disabled)

### `src/connectors/amp.rs`
- Full stub (must stay disabled)

### `Cargo.toml`
- `version = "0.2.7-gj.1"` (bump to `0.2.9-gj.1` after sync)
- `repository = "https://github.com/carmandale/coding_agent_session_search"`
- `license = "MIT"` (not `license-file`)
- opencode feature **removed** from FAD dep
- `libc = "*"` for watchdog

---

## Key risks

| Risk | Mitigation |
|------|-----------|
| frankensqlite `ff6a114b` may break `pragma_table_info` or require nightly | Run `cargo check` immediately after T0; if test failures appear, check napkin correction 2026-04-01 |
| WAL seed workaround becomes unnecessary after page-buffer fix | Keep it — defence in depth, no harm if 26of is resolved |
| Our FK violation catch in `franken_insert_message` becomes unnecessary | Keep it — WARN + skip is safer than crashing if frankensqlite still misbehaves |
| FAD `ba9c598` re-enables opencode in upstream but we removed the feature | Override: explicitly omit `"opencode"` from FAD features in Cargo.toml |
| Large file merges introduce regressions | Run full `cargo test --lib` after T3; compare test count to baseline |
| lib.rs +3035 lines includes analytics/search surface changes that conflict with watchdog wiring | Resolve conflicts per-site; watchdog is 6 isolated dispatch arms, low collision risk |

---

## Acceptance criteria

- [ ] `cargo check --all-targets` clean (zero errors)
- [ ] `cargo clippy --all-targets -- -D warnings` clean
- [ ] `cargo test --lib` — our modules (watchdog, storage, indexer) pass; upstream test failures documented in receipt if any (same policy as spec 011)
- [ ] `cargo build --release` succeeds
- [ ] Watcher deployed, `cass health` returns `✓ Healthy`
- [ ] No `drop_close` or `OOM` WARNs in watcher log after full streaming scan
- [ ] `git diff upstream/main --name-only -- src/` shows only our intentional local files:
  - `src/lib.rs` (watchdog wiring)
  - `src/storage/sqlite.rs` (FK fix, LIMIT removal)
  - `src/indexer/mod.rs` (WAL seed)
  - `src/connectors/opencode.rs` (stub)
  - `src/connectors/amp.rs` (stub)
  - `src/watchdog.rs` (our addition)
- [ ] Version bumped to `0.2.9-gj.1` in Cargo.toml
- [ ] Bead `26of` closed (OOM resolved by frankensqlite page-buffer fix)

---

## Baseline (at spec creation)

| Metric | Value |
|--------|-------|
| frankensqlite pin | `dd9b457` |
| frankensearch pin | `3eec663` |
| FAD pin | `c5d3273c` |
| Our version | `0.2.7-gj.1` |
| lib.rs lines | 18,903 |
| storage/sqlite.rs lines | 14,216 |
| Watcher status | Healthy, 26of workarounds active |
| Known open bugs | 26of (OOM), 2hrs (spike), 3qvr (tests) |
