---
shaping: true
---

<!-- shape:complete:v1 | harness: pi/claude-opus-4-6 | date: 2026-04-03T15:37:17Z -->

# Spec 012 — Upstream Sync: Shaping Transcript

**Participants:** User (driver) + DarkHawk (pi/claude-opus-4-6) + OakKnight (crew-challenger)
**Date:** 2026-04-03

---

## Phase 1: Requirements (User + DarkHawk)

### Draft 0

DarkHawk proposed 9 initial requirements. User pushed back on R4/R5/R6 which
hedged about how much upstream to take ("why are you so scared and resistant?").
The intent is **full sync with upstream**, not cherry-picking.

### Draft 1

Consolidated to 8 requirements. R1 became "Reach full sync with upstream HEAD."
User challenged R5 (no forced re-index): "if we have to re-index, we re-index, right?"
Agreed — R5 removed.

### Draft 2 (Final)

| ID | Requirement | Status |
|----|-------------|--------|
| R0 | Resolve frankensqlite 26of OOM via upstream page-buffer fix | Core goal |
| R1 | Reach full sync with upstream HEAD — all source files, deps, new modules | Must-have |
| R2 | Preserve all local fork changes (watchdog, FK catch, LIMIT removal, WAL seed, stubs) | Must-have |
| R3 | `cargo check --all-targets` and `cargo clippy` pass clean | Must-have |
| R4 | Watcher deploys and runs healthy | Must-have |
| R5 | Fork version identity clear (`-gj.N` suffix) | Must-have |
| R6 | opencode and amp connectors stay disabled | Must-have |
| R7 | Sync is reproducible — merge steps documented | Nice-to-have |

---

## Phase 2: Shapes (DarkHawk proposed, User selected)

### Shape A: Git merge upstream into our branch

Single `git merge upstream/main`. Git does 3-way merge, we resolve conflicts at ~6 known sites.

| Part | Mechanism |
|------|-----------|
| A1 | `git merge upstream/main` — resolve conflicts at watchdog, FK catch, WAL seed, Cargo.toml |
| A2 | Post-merge verify connector stubs |
| A3 | Post-merge verify Cargo.toml overrides |
| A4 | cargo check + clippy |
| A5 | Deploy + verify |

Risk: Merge base may be very old → enormous conflict blocks in 18K+ line files.

### Shape B: In-place file overwrite + patch replay

Take upstream's version of each file, manually re-apply our patches on top.

| Part | Mechanism |
|------|-----------|
| B1-B2 | Bump frankensqlite, frankensearch pins |
| B3 | Copy 3 new files |
| B4 | For each large file: overwrite with upstream, re-apply patches |
| B5-B6 | Bump FAD, update Cargo.toml |
| B7-B8 | Build gate + deploy |

Risk: B4 is error-prone in files that shifted by thousands of lines. No merge commit → no merge base for future syncs. No mechanism to guarantee completeness.

### Shape C: Fresh branch from upstream HEAD + replay our patches

Invert the problem: our 6 patches are small and well-known; upstream's 357 commits are massive. Apply the small onto the large.

| Part | Mechanism |
|------|-----------|
| C1 | Create `sync/012` branch from `upstream/main` HEAD |
| C2 | Add `src/watchdog.rs` (pure file add) |
| C3 | Add Watchdog wiring in lib.rs (enum + dispatch sites) |
| C4 | Apply FK catch, LIMIT removal, seen_idx, let-else in sqlite.rs |
| C5 | Apply WAL seed writes in indexer/mod.rs |
| C6 | Overwrite opencode.rs + amp.rs with stubs |
| C7 | Cargo.toml overrides |
| C8 | Build gate |
| C9 | Deploy + verify |

Risk: Lose branch git history. Watchdog wiring into 21K-line lib.rs where line numbers shifted.

### Fit Check

| Req | Requirement | Status | A | B | C |
|-----|-------------|--------|---|---|---|
| R0 | Resolve frankensqlite 26of OOM via upstream page-buffer fix | Core goal | ✅ | ✅ | ✅ |
| R1 | Reach full sync with upstream HEAD | Must-have | ✅ | ❌ | ✅ |
| R2 | Preserve all local fork changes | Must-have | ✅ | ✅ | ✅ |
| R3 | cargo check + clippy pass clean | Must-have | ✅ | ✅ | ✅ |
| R4 | Watcher deploys and runs healthy | Must-have | ✅ | ✅ | ✅ |
| R5 | Fork version identity clear | Must-have | ✅ | ✅ | ✅ |
| R6 | opencode and amp stay disabled | Must-have | ✅ | ✅ | ✅ |
| R7 | Sync is reproducible | Nice-to-have | ❌ | ✅ | ✅ |

Notes:
- B fails R1: Manual file-by-file has no completeness guarantee across 43 changed files
- A fails R7: 3-way merge conflict resolution in 18K+ line files is judgment-dependent

**User selected Shape C.**

---

## Phase 3: Challenge (OakKnight, crew-challenger)

OakKnight raised 6 challenges against Shape C:

### 1. CRITICAL: Silent Watchdog no-op

Main dispatch at upstream line ~3540 has `_ => {}`. If Watchdog arm is missed there,
it compiles clean but does nothing. `cargo check` won't catch it.

**Resolution:** Split C3 into C3a (exhaustive sites, compiler catches) and C3b (wildcard
sites, must verify manually with smoke test).

### 2. CRITICAL: asset_state.rs divergence unaccounted

Our version is 692 lines, upstream is 872. Verified: no local modifications — our version
is just stale. Starting from upstream HEAD gives the correct version automatically.

**Resolution:** Documented as "no local modifications, upstream supersedes."

### 3. SIGNIFICANT: Non-src files lost

Starting from upstream HEAD loses our AGENTS.md (with NO FILE DELETION rule), .gitignore
(missing our entries), and brings in upstream CI workflows referencing their secrets.

**Resolution:** Added C1.5 — overlay fork-specific non-src files after branch creation.

### 4. MODERATE: Line numbers reference wrong branch

C4/C5 used our branch's line numbers, but the sync branch starts from upstream where
line numbers differ.

**Resolution:** Switched to semantic function-name locations instead of line numbers.

### 5. MODERATE: No branch integration path

What happens to `feat/007-watchdog-subcommand` after sync?

**Resolution:** Added C10 — `sync/012` becomes working branch, `feat/007` archived.

### 6. MINOR: Tracing sink mismatch

Watchdog needs stderr compact tracing, not TUI tracing. Must be added to the tracing
subscriber match.

**Resolution:** Folded into C3b wildcard dispatch sites.

### Smoke test correction (OakKnight, final)

Original C8 proposed `cass watchdog status` — that subcommand doesn't exist. Corrected
to `cass watchdog` (no subcommand, defaults to Run) with **output-presence check**:
silent no-op = zero output = FAIL; wired dispatch = produces output = PASS.

**Verdict:** OakKnight approved revised Shape C after all 6 challenges addressed.

---

## Final Shape C (Revised)

| Part | Mechanism |
|------|-----------|
| **C1** | Create `sync/012` branch from `upstream/main` HEAD |
| **C1.5** | Overlay fork-specific non-src files: AGENTS.md, .gitignore, disable upstream CI workflows |
| **C2** | Add `src/watchdog.rs` (1055 lines, pure file add) |
| **C3a** | Add `Watchdog` variant to `Commands` enum + exhaustive dispatch sites (compiler-enforced: `describe_command()`) |
| **C3b** | Add `Watchdog` arm to wildcard dispatch sites (manual verification required): main dispatch `_ => {}`, tracing subscriber match, is_robot/is_doc guards |
| **C4** | In `storage/sqlite.rs` by semantic function name: FK catch in `franken_insert_message()`, remove LIMIT 1000/100 from fingerprint queries, add `seen_idx` guard, add `let Some(msg_id)` at 6 call sites |
| **C5** | In `indexer/mod.rs` by semantic location: WAL seed at top of `reindex_paths()` before `classify_paths()`, WAL seed before watch mode entry after `restore_watch_steady_state_checkpoint_policy()` |
| **C6** | Overwrite opencode.rs + amp.rs with our 31-line stubs |
| **C7** | Cargo.toml: version `0.2.9-gj.1`, repo URL to carmandale fork, `libc = "*"`, remove opencode from FAD features, update patch section |
| **C8** | `cargo check --all-targets && cargo clippy` gate + `cass watchdog` output-presence smoke test |
| **C9** | Build release binary, deploy, verify watcher healthy, close bead 26of |
| **C10** | Branch integration: `sync/012` becomes working branch, `feat/007` archived (not deleted) |
