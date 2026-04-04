---
title: "Spec 012: Upstream Sync — Full sync with upstream HEAD + local patch replay"
date: 2026-04-03
bead: coding_agent_session_search-1e57
---

<!-- Codex Review: APPROVED after 2 rounds | model: gpt-5.3-codex | date: 2026-04-03 -->
<!-- Status: UNCHANGED -->
<!-- Revisions: none (plan/tasks revised to conform to this spec) -->

<!-- issue:complete:v1 | harness: unknown | date: 2026-04-03T14:46:57Z -->

## Purpose

Bring the fork to full sync with upstream HEAD, resolving the frankensqlite `26of`
OOM crash loop and picking up all upstream improvements (analytics, search pipeline,
source-filter normalization, TUI hardening).

---

## Why now

Upstream pins frankensqlite to `ff6a114b` (1 GB page buffer default), directly
resolving bead `26of` — the OOM that causes MVCC FK mismatches, page-lock deadlocks,
and WAL corruption in our watcher. Additional upstream value: analytics multi-dimension
breakdown, source-filter normalization, `--source` flag, OOM-loop fix on killed
incremental scans, and 3 new search pipeline files.

---

## Requirements

| ID | Requirement | Status |
|----|-------------|--------|
| R0 | Resolve frankensqlite 26of OOM via upstream page-buffer fix | Core goal |
| R1 | Reach full sync with upstream HEAD — all source files, deps, new modules | Must-have |
| R2 | Preserve all local fork changes (watchdog, FK catch, LIMIT removal, WAL seed, stubs) | Must-have |
| R3 | `cargo check --all-targets` and `cargo clippy` pass clean | Must-have |
| R4 | Watcher deploys and runs healthy — no `drop_close`, no OOM, no crash loop | Must-have |
| R5 | Fork version identity clear (`-gj.N` suffix distinguishes from upstream) | Must-have |
| R6 | opencode and amp connectors stay disabled | Must-have |
| R7 | Sync is reproducible — merge steps documented | Nice-to-have |

---

## Selected Shape: C — Fresh branch from upstream HEAD + replay our patches

Invert the merge problem. Our 6 local patches are small and well-enumerated.
Upstream's 357 commits are massive and diffuse. Apply the small onto the large,
not the large onto the small. Start from a known-clean upstream state — R1 (full
sync) is satisfied by construction.

### Alternatives considered

- **Shape A (git merge upstream into our branch):** Passes R0-R6 but fails R7 —
  3-way merge conflict resolution in 18K+ line files is judgment-dependent and
  hard to reproduce. Merge base may be very old, producing enormous conflict blocks.
- **Shape B (in-place file overwrite + patch replay):** Fails R1 — manual
  file-by-file sync across 43 changed files has no completeness guarantee.
  No merge commit means no merge base for future syncs.

Full fit check and shape details in `shaping-transcript.md`.

### Parts

| Part | Mechanism |
|------|-----------|  
| **C1** | Create `sync/012` branch from `upstream/main` HEAD — starts with 100% upstream code |
| **C1.5** | Overlay fork-specific non-src files: `AGENTS.md`, `.gitignore`, disable upstream CI workflows that reference secrets we don't have |
| **C2** | Add `src/watchdog.rs` (1055 lines, pure file add — no conflict) |
| **C3a** | Add `Watchdog` variant to `Commands` enum + exhaustive dispatch sites (compiler-enforced: `describe_command()` has no wildcard — missing arm = compile error) |
| **C3b** | Add `Watchdog` arm to wildcard dispatch sites (manual verification required — silent no-op if missed): main dispatch `_ => {}` (~line 3540), tracing subscriber match (~line 2778 — Watchdog needs stderr compact tracing, not TUI), `is_robot`/`is_doc` guards |
| **C4** | In `storage/sqlite.rs` (by semantic function name, not line number): ForeignKeyViolation catch in `franken_insert_message()` → `Ok(None)`, remove `LIMIT 1000` from `franken_existing_message_fingerprints_by_idx()`, remove `LIMIT 100` from `franken_existing_message_replay_fingerprints()`, add `seen_idx` HashSet guard in fresh insert path, add `let Some(msg_id) = ... else { continue }` at 6 call sites, context wrappers on `franken_insert_snippets` and `franken_insert_conversation` |
| **C5** | In `indexer/mod.rs` (by semantic location): WAL seed at top of `reindex_paths()` before `classify_paths()` call, WAL seed before watch mode entry after `restore_watch_steady_state_checkpoint_policy()` |
| **C6** | Overwrite upstream's `opencode.rs` and `amp.rs` with our 31-line stubs |
| **C7** | `Cargo.toml`: version `0.2.9-gj.1`, repository URL → carmandale fork, `license = "MIT"` (not `license-file`), `libc = "*"` for watchdog, remove `"opencode"` from FAD features, update `[patch]` section for frankensqlite |
| **C8** | `cargo check --all-targets && cargo clippy --all-targets -- -D warnings` gate + Watchdog smoke test: run `cass watchdog` and verify **output presence** (silent = dispatch not wired; any output = wired correctly) |
| **C9** | `cargo build --release`, deploy binary, verify `cass health` returns healthy, verify no `drop_close`/OOM in watcher log after full streaming scan, close bead `26of` |
| **C10** | Branch integration: `sync/012` becomes the working branch; `feat/007-watchdog-subcommand` archived (not deleted) |

### Notes on specific parts

**C3b — wildcard dispatch sites are the highest-risk step.** Unlike exhaustive matches
(where the compiler catches a missing arm), wildcard `_ => {}` arms silently swallow
unmatched variants. The smoke test in C8 is the safety net: `cass watchdog` with no
subcommand defaults to `Run`. If dispatch is wired, it produces output (healthy/unhealthy
status). If swallowed by `_ => {}`, it produces zero output. Test for output presence.

**C4 — upstream still has the LIMIT clauses we removed.** Verified: upstream's
`franken_existing_message_fingerprints_by_idx()` has `LIMIT 1000` (line 6764) and
`franken_existing_message_replay_fingerprints()` has `LIMIT 100` (line 6796). Our
removal patch is required on top of upstream code.

**C1.5 — files confirmed divergent:**
- `AGENTS.md`: ours has cass-specific rules (NO FILE DELETION, Morph Warp Grep, beads workflow); upstream has generic version
- `.gitignore`: ours has entries for build artifacts, clippy output, agent scripts, test fixtures that upstream lacks
- `.github/workflows/`: upstream has CI referencing their secrets/deploy targets
- `Cargo.lock`: regenerated automatically by C8's `cargo check`; no explicit step needed

**asset_state.rs:** No local modifications — our 692-line version is just stale upstream.
Upstream's 872-line version (adds `SearchMaintenanceJobKind` enum + 5 struct fields)
supersedes ours. Starting from upstream HEAD gives the correct version automatically.

---

## Our local patches that MUST be preserved

These are in our fork and absent from upstream. None exist upstream — clean separation
confirmed via grep against `upstream/main`.

### `src/watchdog.rs` (C2)
- 1055-line file, our addition — daemon monitoring and management

### `src/lib.rs` (C3a + C3b)
- `Watchdog` variant in `Commands` enum
- `describe_command()` arm (exhaustive match — compiler-enforced)
- Main dispatch arm (wildcard `_ => {}` — MUST NOT MISS)
- Tracing subscriber match (Watchdog needs stderr compact, not TUI)
- `is_robot`/`is_doc` guards
- `libc = "*"` dep required by watchdog

### `src/storage/sqlite.rs` (C4)
- `ForeignKeyViolation` catch in `franken_insert_message()` → returns `Ok(None)`
- Removed `LIMIT 1000` from `franken_existing_message_fingerprints_by_idx()`
- Removed `LIMIT 100` from `franken_existing_message_replay_fingerprints()`
- `seen_idx` HashSet guard in fresh insert path
- `let Some(msg_id) = ... else { continue }` pattern at 6 call sites
- Context wrappers on `franken_insert_snippets` and `franken_insert_conversation`

### `src/indexer/mod.rs` (C5)
- WAL seed write at top of `reindex_paths()` before `classify_paths()`
- WAL seed write before entering watch mode after `restore_watch_steady_state_checkpoint_policy()`

### `src/connectors/opencode.rs` (C6)
- Full stub (31 lines) — must stay disabled

### `src/connectors/amp.rs` (C6)
- Full stub (31 lines) — must stay disabled

### `Cargo.toml` (C7)
- `version = "0.2.9-gj.1"`
- `repository = "https://github.com/carmandale/coding_agent_session_search"`
- `license = "MIT"` (not `license-file`)
- opencode feature removed from FAD dep
- `libc = "*"` for watchdog
- `[patch]` section for frankensqlite

---

## Key risks

| Risk | Mitigation |
|------|-----------|
| C3b wildcard dispatch: Watchdog arm missed → silent no-op in production | Smoke test in C8: `cass watchdog` must produce output; zero output = FAIL |
| frankensqlite `ff6a114b` breaks `pragma_table_info` or requires nightly features | `cargo check` in C8 catches immediately; check napkin correction 2026-04-01 |
| C4 patches land on wrong code if upstream refactored `franken_insert_message()` | Use semantic function names, not line numbers; verify function signatures match |
| Upstream CI workflows fire on push to our fork | C1.5 disables/removes upstream workflow files |
| Re-index needed after frankensqlite pin change | Acceptable — if needed, re-index |
| WAL seed / FK catch become unnecessary after page-buffer fix | Keep both — defence in depth, no harm |

---

## Acceptance criteria

- [ ] `cargo check --all-targets` clean (zero errors)
- [ ] `cargo clippy --all-targets -- -D warnings` clean
- [ ] `cargo test --lib` — our modules (watchdog, storage, indexer) pass; upstream test failures documented if any
- [ ] `cass watchdog` produces output (dispatch wired, not swallowed by wildcard)
- [ ] `cargo build --release` succeeds
- [ ] Watcher deployed, `cass health` returns healthy
- [ ] No `drop_close` or OOM WARNs in watcher log after full streaming scan
- [ ] `git diff upstream/main --name-only -- src/` shows only our intentional local changes:
  - `src/lib.rs` (watchdog wiring)
  - `src/storage/sqlite.rs` (FK fix, LIMIT removal)
  - `src/indexer/mod.rs` (WAL seed)
  - `src/connectors/opencode.rs` (stub)
  - `src/connectors/amp.rs` (stub)
  - `src/watchdog.rs` (our addition)
- [ ] Version is `0.2.9-gj.1` in Cargo.toml
- [ ] Bead `26of` closed
- [ ] `sync/012` is the active working branch; `feat/007` archived

---

## Baseline (at spec creation)

| Metric | Value |
|--------|-------|
| frankensqlite pin | `dd9b457` |
| frankensearch pin | `3eec663` |
| FAD pin | `c5d3273c` |
| Our version | `0.2.7-gj.1` |
| Upstream version | `0.2.5` |
| lib.rs (ours / upstream) | 18,903 / 21,938 |
| storage/sqlite.rs (ours / upstream) | 14,216 / 14,803 |
| indexer/mod.rs (ours / upstream) | 8,695 / 9,366 |
| Files differing from upstream | 43 |
| Upstream commits since last sync | 357 (since 2026-03-01) |
| Watcher status | Healthy, 26of workarounds active |
| Known open bugs | 26of (OOM), 2hrs (spike), 3qvr (tests) |
