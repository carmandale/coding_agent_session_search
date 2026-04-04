---
title: "Plan 012: Upstream-sync on upstream/main base with deterministic fork patch replay"
date: 2026-04-03
bead: coding_agent_session_search-1e57
---

<!-- Codex Review: APPROVED after 2 rounds | model: gpt-5.3-codex | date: 2026-04-03 -->
<!-- Status: REVISED -->
<!-- Revisions: strict R1 semantics; git object preflight gate; dependency attestation (cargo metadata/tree); allowlist non-src overlay; stronger watchdog checks; backup/rollback/soak; workflow-security parity review; explicit 26of closure evidence -->

<!-- plan:complete:v1 | harness: pi/gpt-5.3-codex | date: 2026-04-03T18:28:23Z -->

# Overview

Implement Spec 012 by executing Shape C: branch from `upstream/main` HEAD, then replay only the fork-required patch set.

## Invariants

1. **Strict upstream sync baseline (R1):** upstream dependency graph is the default and target state.
2. **Fork behavior preservation (R2):** only explicitly enumerated fork deltas are re-applied.

> Clarification from Codex review: this plan treats `R1` as strict. Dependency overrides are exceptions that require reproducible evidence and explicit logging.

# Architecture / Strategy

## Why this approach

- Avoids high-entropy conflict archaeology from deep 3-way merge.
- Avoids incomplete manual file-by-file replay over large drift.
- Makes parity measurable: upstream baseline first, then prove each intentional fork delta.

## Execution model

- **Phase 0:** git-object completeness preflight (required before ancestry/diff claims)
- **Phase 1:** create `sync/012` from upstream HEAD
- **Phase 2:** allowlist overlay of fork-owned non-src artifacts
- **Phase 3:** replay code patch set (lib/sqlite/indexer/stubs/Cargo)
- **Phase 4:** dependency-attestation gates (`cargo metadata`/`cargo tree`) + build gates
- **Phase 5:** runtime verification with backup/rollback + soak criteria

# Grounded Research Findings

## `src/lib.rs` watchdog insertion points

Required fork deltas on upstream base:
- `pub mod watchdog;`
- `Commands::Watchdog { command: Option<watchdog::WatchdogCommand> }`
- `describe_command()` watchdog arm
- tracing subscriber command list includes watchdog
- sync dispatch arm for watchdog
- `state_meta_json()` watchdog JSON block

## `src/storage/sqlite.rs` cascade patch

- upstream: `franken_insert_message(...) -> Result<i64>`
- fork requirement: `-> Result<Option<i64>>` with FK-violation skip path
- 6 call-sites must convert to `let Some(msg_id) = ... else { continue; }`
- remove `LIMIT 1000` and `LIMIT 100` fingerprint query caps
- retain `seen_idx` guard + context wrappers

## `src/indexer/mod.rs` WAL seed pair

- seed before `set_mode(Watch)` entry
- seed at top of `reindex_paths()` before `classify_paths()`

## Connector/Cargo coupling (hard ordering)

- `opencode`/`amp` stubs must land **before** FAD feature removal from Cargo.
- Otherwise intermediate builds can fail on unresolved `OpenCodeConnector` path.

## Non-src overlay correction

- use **allowlist overlay**, not blanket directory replacement
- upstream already contains `.beads`; do not overwrite wholesale
- overlay only fork-owned artifacts required for workflow continuity and policy

# Requirement → Change Traceability

| Req | Requirement | Planned Changes | Verification |
|-----|-------------|-----------------|--------------|
| R0 | Resolve 26of OOM via upstream page-buffer fix | upstream HEAD base + frankensqlite attestation + runtime soak | `cargo metadata/tree` rev proof + soak criteria + watcher logs |
| R1 | Full sync with upstream HEAD | strict upstream baseline; no silent dependency drift | ancestry check + diff allowlist + dependency attestation |
| R2 | Preserve fork local behavior | replay explicit patch inventory only | targeted diff verification + watchdog/connector checks |
| R3 | Check/clippy clean | full build gates after replay | `cargo check --all-targets`; `cargo clippy --all-targets -- -D warnings` |
| R4 | Watcher healthy | watchdog wiring + WAL seed + runtime verification | watchdog command checks + `cass health --json` schema + soak |
| R5 | Fork version identity clear | Cargo version/repo/license overrides | Cargo diff + `cass --version` |
| R6 | opencode/amp disabled | stubs + FAD feature removal | compile success + connector smoke |
| R7 | Reproducible | explicit ordered tasks + evidence checkpoints | transcript + gate outputs + deterministic allowlists |

# Decision Gates (Evidence-Required)

## DG-0: Git object completeness preflight

Before any upstream diff/ancestry gates:
1. `git fetch --prune --tags upstream`
2. verify required objects are readable (no missing-tree/object errors)
3. only then run ancestry/diff audits

If object integrity fails, stop and remediate repo object state before proceeding.

## DG-1: frankensqlite/FAD effective resolution proof (mandatory)

1. Run `cargo metadata --format-version 1` and capture lock graph snapshot.
2. Run `cargo tree -i fsqlite` and `cargo tree -i fsqlite-types`.
3. Assert effective resolved source/rev matches intended upstream baseline (or explicitly approved exception).
4. If active `[patch]` stanza is needed, record exact reason and resulting resolved revs.

Decision rule:
- keep patch only if required and evidenced; otherwise remove patch for strict upstream parity.

## DG-2: asupersync revision posture (strict by default)

- default and preferred: keep upstream `asupersync` rev.
- override only if a **reproducible failing condition** is demonstrated on upstream rev and fixed by override.
- if override is required, mark as a scoped exception to R1 and document explicitly in acceptance evidence.

# Non-src Allowlist Overlay (revised)

## Overlay from fork branch
- `specs/`
- `thoughts/`
- `.claude/`
- `hooks/`
- `AGENTS.md`
- `.gitignore`

## Do NOT blanket-overwrite
- `.beads/` (upstream carries it; reconcile state via controlled `br sync --flush-only` and explicit file policy)

## Workflow security parity review

After workflow normalization, run explicit review:
- diff upstream workflows vs fork workflows
- document which security-relevant checks were removed/retained and why
- ensure no accidental loss of required guardrails

# Runtime Safety: Backup / Rollback / Soak

## Pre-deploy backup (mandatory)

Before replacing runtime binary or launching watch cycle:
1. backup DB + WAL + SHM (`agent_search.db`, `agent_search.db-wal`, `agent_search.db-shm` if present)
2. capture current binary path + hash
3. capture watcher plist and lockfile state

## Rollback triggers

Rollback immediately if any of:
- repeated OOM/drop_close signatures reappear
- `cass health` regresses to unhealthy due to sync changes
- dependency-attested revs mismatch intended state after deploy

## Soak criteria

Require sustained stability window (e.g., >=2 full scan/reindex cycles or >=30 minutes watch runtime) with:
- no OOM/drop_close recurrence
- no crash-loop restarts
- expected ingest behavior present

# Verification Plan

1. **Preflight:** DG-0 object completeness
2. **Compile gates:** `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo test --lib` (document known baseline failures)
3. **Dependency attestation:** DG-1 evidence (`cargo metadata/tree` outputs archived)
4. **Watchdog correctness gate:**
   - run `cass watchdog` and assert non-empty output + non-clap-error output
   - run `cass health --json` and assert watchdog keys are present in JSON payload
5. **Runtime health:** `cass health` healthy + watcher logs clean
6. **Soak:** satisfy defined soak window criteria
7. **Diff/ancestry audit:** run only after DG-0 preflight; verify only allowlisted fork deltas remain
8. **Bead closure evidence:** explicitly record criteria/results for closing `26of`

# Alternative execution path (optional)

If semantic replay becomes error-prone:
- generate `format-patch` from known fork commits touching target files
- apply onto upstream base with `git am`/`cherry-pick` where clean
- fall back to semantic replay only for conflicted hunks

# Risks and Controls

| Risk | Control |
|------|---------|
| "Full sync" contradiction via hidden dependency drift | strict R1 wording + DG-1/DG-2 evidence and exception logging |
| Missing-object git state invalidates diff gates | DG-0 preflight required |
| False watchdog pass from weak smoke check | command-output + health JSON schema assertions |
| Overlay clobbers important upstream artifacts | allowlist overlay + explicit workflow security review |
| 26of appears fixed only at compile-time | dependency attestation + runtime soak + rollback triggers |
| Data loss during deployment verification | mandatory DB/WAL/SHM backup before deploy |

# Deliverables

- `plan.md` (this file)
- `tasks.md` (reconciled with revised gates)
- `planning-transcript.md`
- `codex-review.md`
