---
title: "Tasks 012: Execute upstream-sync with fork patch replay"
date: 2026-04-03
bead: coding_agent_session_search-1e57
---

<!-- Codex Review: APPROVED after 2 rounds | model: gpt-5.3-codex | date: 2026-04-03 -->
<!-- Status: RECONCILED -->
<!-- Revisions: added DG-0 preflight, DG-1 dependency attestation, tightened DG-2, allowlist overlay, workflow-security parity check, backup/rollback/soak gates, explicit 26of closure evidence -->

<!-- plan:complete:v1 | harness: pi/gpt-5.3-codex | date: 2026-04-03T18:28:23Z -->

# Execution Tasks

## T0 — Preflight git integrity gate (DG-0)

- [x] T0.1 `git fetch --prune --tags upstream`
- [x] T0.2 Verify upstream objects/tree readability (no missing-object/tree errors)
- [x] T0.3 Record baseline SHAs: `HEAD`, `upstream/main`, source branch

**Depends on:** none
**Gate:** must pass before ancestry/diff-based assertions

---

## T1 — Branch setup

- [x] T1.1 Create and switch to `sync/012` from `upstream/main`
- [x] T1.2 Confirm branch ancestry is upstream-based

**Depends on:** T0

---

## T2 — Allowlist overlay (non-src)

- [x] T2.1 Overlay fork-owned directories from source branch:
  - [x] `specs/`
  - [x] `thoughts/`
  - [x] `.claude/`
  - [x] `hooks/`
- [x] T2.2 Overlay root files:
  - [x] `AGENTS.md`
  - [x] `.gitignore`
- [x] T2.3 Do **not** blanket-overwrite `.beads/`
- [x] T2.4 Reconcile beads state via controlled `br sync --flush-only` policy if needed

**Depends on:** T1

---

## T3 — Workflow normalization + security parity review

- [x] T3.1 Normalize `.github/workflows/` to fork-owned intended set
- [x] T3.2 Diff upstream vs fork workflows
- [x] T3.3 Document which security-relevant checks were removed/retained and rationale
- [x] T3.4 Confirm no accidental loss of required security guardrails

**Depends on:** T2

---

## T4 — Watchdog module + lib.rs replay

- [x] T4.1 Add `src/watchdog.rs`
- [x] T4.2 Add `pub mod watchdog;` in `src/lib.rs`
- [x] T4.3 Add `Commands::Watchdog` variant
- [x] T4.4 Add `describe_command()` watchdog arm
- [x] T4.5 Add watchdog to tracing subscriber command list
- [x] T4.6 Add sync dispatch arm for `Commands::Watchdog`
- [x] T4.7 Restore `state_meta_json()` watchdog JSON block

**Depends on:** T3

---

## T5 — sqlite reliability replay

- [x] T5.1 Change `franken_insert_message` return type to `Result<Option<i64>>`
- [x] T5.2 Add FK-violation catch returning `Ok(None)`
- [x] T5.3 Update all 6 call sites to `let Some(msg_id) = ... else { continue; }`
- [x] T5.4 Remove `LIMIT 1000` in fingerprint-by-idx query
- [x] T5.5 Remove `LIMIT 100` in replay-fingerprint query
- [x] T5.6 Restore `seen_idx` guard and context wrappers

**Depends on:** T4

---

## T6 — indexer WAL seed replay

- [x] T6.1 Add watch-entry WAL seed before `set_mode(Watch)`
- [x] T6.2 Add `reindex_paths()` WAL seed before `classify_paths()`

**Depends on:** T5

---

## T7 — Connector stubs (hard dependency before Cargo feature removal)

- [x] T7.1 Replace `src/connectors/opencode.rs` with fork stub
- [x] T7.2 Replace `src/connectors/amp.rs` with fork stub
- [x] T7.3 (Optional parity) restore explanatory comment in `connectors/mod.rs`

**Depends on:** T6

---

## T8 — Cargo identity + feature controls

- [x] T8.1 Set version to `0.2.9-gj.1`
- [x] T8.2 Set repository to fork URL
- [x] T8.3 Ensure license field matches fork policy (`MIT`)
- [x] T8.4 Ensure `libc = "*"` present for watchdog
- [x] T8.5 Remove `"opencode"` from FAD feature set
- [x] T8.6 Refresh lockfile as part of build gate

**Depends on:** T7
**Hard ordering constraint:** T7 must complete before T8

---

## T9 — DG-1 dependency attestation (mandatory)

- [x] T9.1 Run `cargo metadata --format-version 1` and archive output
- [x] T9.2 Run `cargo tree -i fsqlite` and `cargo tree -i fsqlite-types`
- [x] T9.3 Verify effective resolved source/revs match intended baseline
- [x] T9.4 Evaluate active `[patch]` necessity:
  - [x] test without patch
  - [x] if required, re-enable with explicit evidence and record reason

**Depends on:** T8
**Gate:** must pass before runtime verification

---

## T10 — DG-2 asupersync posture (strict by default)

- [x] T10.1 Keep upstream asupersync rev by default
- [x] T10.2 Only test override if reproducible failure implicates upstream rev
- [x] T10.3 If override used, record explicit scoped exception to R1 with evidence

**Depends on:** T9

---

## T11 — Compile/test verification gates

- [x] T11.1 `cargo check --all-targets`
- [x] T11.2 `cargo clippy --all-targets -- -D warnings`
- [x] T11.3 `cargo test --lib` (document known baseline upstream failures)

**Depends on:** T10

---

## T12 — Runtime safety + smoke checks

- [x] T12.1 Pre-deploy backup: DB + WAL + SHM + binary hash + lock/plist state
- [x] T12.2 Watchdog correctness:
  - [x] run `cass watchdog` and assert non-empty non-clap-error output
  - [x] run `cass health --json` and assert watchdog keys are present
- [x] T12.3 Deploy release binary and run `cass health`
- [x] T12.4 Monitor logs for OOM/drop_close regressions

**Depends on:** T11

---

## T13 — Soak + rollback gate

- [x] T13.1 Run soak window (>=2 scan/reindex cycles or >=30 min watch runtime)
- [x] T13.2 Confirm no crash-loop restart behavior
- [x] T13.3 If rollback trigger fires, execute rollback procedure immediately

**Depends on:** T12

---

## T14 — Final audit and closure

- [x] T14.1 Diff audit vs upstream (allowlisted deltas only)
- [x] T14.2 Record dependency-attestation evidence (DG-1/DG-2 outcomes)
- [x] T14.3 Record workflow-security parity review outcome
- [x] T14.4 Record explicit bead `26of` closure evidence
- [x] T14.5 Mark `sync/012` active, archive `feat/007-watchdog-subcommand`

**Depends on:** T13

---

## T15 — Handoff artifacts

- [x] T15.1 Update log/receipts with all gate outcomes
- [x] T15.2 Capture command outputs used for go/no-go decisions
- [x] T15.3 Prepare implementation handoff summary

**Depends on:** T14
