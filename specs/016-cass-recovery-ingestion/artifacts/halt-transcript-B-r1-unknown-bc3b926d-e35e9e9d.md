---
halt_timestamp: 2026-05-16T13:37:15Z
phase: B
round: 1
prior_sha: unknown
plan_hash: bc3b926d
drift: BANANA
bypass_status: pending
halt_event_id: unknown-bc3b926d
---

## Codex Response

## Per-Source-Entry Verdicts
1. **Source 1 — aligned.** The plan keeps the work grounded in concrete failure evidence and repo/runtime inspection before mutation, matching the request to understand what is actually going wrong (spec.md:9; plan.md:23-36,71-81).
2. **Source 2 — mostly aligned with one intent gap risk.** Upstream sync, priority-agent ingestion/searchability, and watcher proof are explicit and central (spec.md:11; plan.md:73,102-104,130-137,173-179), but the “all sessions are processed” portion is not promoted to a clear completion threshold for non-priority connectors (spec.md:11; plan.md:136-137,205-211).
3. **Source 3 — aligned.** The plan is attached to an active tracked recovery path with explicit workflow-state closeout requirements, consistent with “set a goal / create issue” intent (spec.md:13; plan.md:9,203-211).

## Delta Classification
- Adds explicit upstream incorporation proof flow (`merge-base`, SHAs, ahead/behind) and blocker path: +9 lines, traced to R1 (plan.md:102-104,124-126; spec.md:32,56).
- Adds raw-to-database reconciliation model using `source_path` accounting and lexical proof expectations: +14 lines, traced to R4 (plan.md:92-96,130-137; spec.md:35,61).
- Adds launchd watcher durability gate with same-binary process proof, 120-second probe, and 10-minute recheck: +18 lines, traced to R7 (plan.md:159-179; spec.md:38-39,62).

## Drift Flags
BANANA: Potential under-delivery path: the plan permits stopping at “follow-up issue/blocker” if command-surface gaps block watcher durability, which can miss the explicit user intent to set up a working watcher now (spec.md:11; plan.md:167-169).
BLOAT: none
AMBIGUOUS: Completion semantics for “all sessions are processed” outside Pi Agent, Claude Code, and Codex remain unclear; the plan requires restoring prior config/scope but does not define a measurable non-priority completion bar (spec.md:11; plan.md:136-137,205-211).

## Halted Plan Draft (verbatim)


---
title: "Plan: recover cass session ingestion and watcher"
date: 2026-05-16
bead: coding_agent_session_search-1vxuf
---

# Plan: recover cass session ingestion and watcher

Implementation plan for [`specs/016-cass-recovery-ingestion/spec.md`](spec.md). The spec owns the outcome; this plan owns the recovery route and proof surfaces.

## Overview

This recovery is operational first and code second. The user's goal fails when cass is treated as "mostly done" because a spec advanced or a narrow Pi slice landed. Success is one current evidence packet proving that the fork is deliberately synced with upstream, Pi Agent / Claude Code / Codex histories are represented and lexically searchable, and launchd keeps future sessions flowing through the same installed cass binary.

The plan uses a runtime-first canary route before broad mutation. Existing cass surfaces already expose `index --watch-once`, `search --mode lexical --robot`, `health/status --json`, `sources agents list --json`, and DB provenance by `source_path`/`external_id`. Those are enough to prove or falsify the recovery path without inventing another importer. Code changes are limited to the smallest runtime-parity gap the live system proves. The hard watcher acceptance surface is `com.cass.index-watch`; the separate health watchdog parse error is a durability risk to repair only when it blocks or supervises the required watcher proof.

## Root Cause Gate

Symptom: cass currently reports stale lexical readiness, Pi Agent has only 36 archived conversations against 2,076 raw jsonl files, upstream was 16 commits ahead of this checkout in the latest planning baseline, and `com.cass.index-watch` is not loaded. These numbers are baseline evidence only; T1 refreshes them before implementation and the refreshed evidence becomes operative.

Root cause level 1: recent work advanced narrower specs and partial implementation artifacts without requiring one product-level acceptance packet across upstream sync, historical ingestion, searchability, and watcher durability.

Root cause level 2: source, installed binary, and launchd runtime drifted apart. The installed binary is `/Users/dalecarman/.local/bin/cass` version `0.4.7`; launchd's health watchdog calls `cass watchdog run`; `cass capabilities --json` exposes no `watchdog` command; `com.cass.index-watch` has a plist but is not registered with launchd.

Root cause level 3: recovery proofs used proxies (`cass stats`, stage artifacts, plist existence, or spec 015's Pi-only route) instead of consumer-context proof from the shipped binary and live search surface.

Fix principle: recover with bounded runtime canaries and identity/search evidence first; patch only the runtime mismatch that prevents watcher durability; finish only after the GoalBuddy board and spec ownership state agree that no required recovery tasks remain.

## Shape Comparison

### Shape A: Runtime-first priority recovery with conditional runtime parity patch (selected)

Use shipped cass surfaces to freeze a priority corpus manifest, run stale-index recovery from `cass health --json` recommendations, canary one Pi/Claude/Codex identity through raw source → DB row → lexical search, run broad priority watch-once recovery, and repair watcher launchd on the installed binary. If a separate command-surface gap prevents required watcher durability, record the concrete blocker or create a follow-up issue rather than expanding this recovery into watchdog CLI work.

Net complexity: medium operationally, low-to-medium in code. It keeps ingestion on the existing indexer path, limits code to proven gaps, and uses the installed binary as the acceptance surface.

Why selected: it directly targets the failure mode Codex caught in Phase A: code-first watchdog work can balloon before proving ingestion/search is recoverable, while artifact-only work can leave the watcher broken. This shape forces live proof before and after any code touch and keeps optional watchdog command work outside the happy path.

### Shape B: Code-first watchdog command wiring before ingestion

Start by wiring `src/watchdog.rs` into the CLI, updating capabilities/robot-docs/goldens, deploying a new binary, and reinstalling launchd services before historical recovery.

Net complexity: medium-to-high. It may be necessary, but doing it first risks spending the recovery on CLI contract churn before proving priority histories can be indexed and searched.

Why rejected as primary: the user's core pain is searchable sessions, with watcher durability as the forward-capture closure. The watcher command gap is real, but it should be patched after the runtime canary proves the broader recovery route or exactly identifies the blocking surface.

### Shape C: Finish spec 015 first, then revisit product recovery

Drive the Pi Agent watch-once streaming scan spec to completion and treat it as the main path.

Net complexity: low for Pi, high for the actual user goal. It does not cover upstream sync, Claude/Codex reconciliation, installed binary parity, launchd health-watchdog failure, or GoalBuddy product-state closeout.

Why rejected: spec 015 is useful subordinate evidence, not the product owner. Completing it alone repeats the earlier failure mode.

### Shape D: Shadow data-dir recovery and cutover

Index priority agents in a fresh data dir, verify there, and cut over the live archive only after proof.

Net complexity: high. It avoids live mutation risk, but introduces cutover, dual-index, stale-reader, and rollback complexity. It also does not repair the broken live watcher by itself.

Why fallback only: use it if live watch-once recovery hits WAL/FTS corruption or repeat OOM/stall under the route policy. Do not start there without evidence that in-place recovery is unsafe.

## Plan Sanity Evidence

Objective: make the live cass installation current with upstream intent, priority-agent histories searchable through lexical robot queries, and launchd watcher capture durable on the installed binary.

Riskiest assumption: existing shipped `cass index --watch-once` and DB `source_path` provenance can recover Pi Agent, Claude Code, and Codex histories without a new importer or destructive archive rebuild.

Smallest probe: ran `cass capabilities --json`, `cass health --json`, `cass status --json`, `cass stats --json`, raw `rg --files` counts, launchd `print`, and source reads of `src/lib.rs`, `src/indexer/mod.rs`, and `src/storage/sqlite.rs`.

Observed result: `cass capabilities --json` exposed `index --watch-once` but no `watchdog`; `cass health --json` exited 1 with stale index and `watch_active=false`; launchd reported missing `com.cass.index-watch`; storage schema keeps `conversations.source_path` and unique `(source_id, agent_id, external_id)` provenance.

Decision impact: if `watchdog` had existed and launchd showed a loaded watcher, `plan.md ## Watcher Repair And Durability` and `tasks.md ## Group D: Watcher Runtime Parity` would drop the conditional CLI patch and keep only reload plus probe verification.

## Evidence Model

All recovery evidence lands under `specs/016-cass-recovery-ingestion/evidence/`. Files are shared workflow artifacts and should be committed if the workflow reaches finalize.

Before any evidence artifact is committed, run an explicit evidence hygiene pass. Log tails and source excerpts must be summarized or redacted when they include credentials, tokens, private customer text, personal data, or unrelated session content. Lexical probe strings must be harmless, user-approved-by-context snippets generated for this recovery or obviously non-secret technical strings from the user's own agent transcripts. The receipt records the hygiene pass and names any artifact intentionally kept out of git.

The manifest is path-based because cass stores `conversations.source_path`. A raw priority-agent file is accounted when an exact DB `source_path` row exists, a path-specific quarantine/skip record exists, or an explicit connector-scope rule excludes it. Reconciliation checks missing paths and duplicate `source_path`/non-null provenance rows. Search proof uses three safe lexical strings per priority agent and requires at least one hit from the expected `source_path`. Semantic search remains out of scope unless the user explicitly requests model installation.

## Route Policy

### Upstream And Branch

Fetch `upstream main` and `origin`. Record `HEAD`, `upstream/main`, merge-base, and ahead/behind counts. Upstream is resolved only when `git merge-base --is-ancestor upstream/main HEAD` succeeds or a concrete blocker is recorded.

Current branch is `dac/main`, not `main`. Do not create a new branch. Runtime recovery and evidence gathering can proceed on the current checkout, but final commit/push cannot. The concrete finalization resolution is: before staging, capture `git status --short --branch`, list the exact files this session intends to stage, and ask the user to authorize either committing/pushing from the current branch or a non-destructive move back to `main`. Do not commit or push from an unauthorized branch. If upstream merge or final commit is blocked by branch policy or unrelated dirty files in target scope, stop with that blocker rather than hiding it.

### Locks And Active Work

Before any indexing mutation, confirm `evidence/route-policy.md` exists and that the current status satisfies it. Then run `cass status --json` and inspect `rebuild.active`, `pending.watch_active`, `doctor_summary.active_repair`, and any structured recommended commands. If a doctor mutation lock is active, capture the status JSON, identify the owner if cass exposes it, wait one bounded interval, and re-check once. Do not run `cass doctor --fix` or a second indexer against an active mutation lock.

If `cass index --json` returns structured `active_index`, parse the owner PID/command and wait only when the owner is alive. Stale-owner handling must use cass/doctor safe paths or an explicit user-approved plan, never lock-file deletion.

### Stale Index

For stale but initialized health, run exactly the first `refresh-lexical-index` command from `cass health --json` or `cass status --json` recommended commands. Then run the paired health verification command. If still stale, capture both JSON payloads and route once to the planned repair path; do not loop.

### Watch-Once Recovery

Run a canary identity before broad priority recovery. Prefer exact source-file `--watch-once` where the installed binary supports it; if a connector requires root-level discovery, use the smallest root that exercises the connector and record why file-level canary was not valid.

Broad recovery runs priority roots separately so failures are attributable by agent. Use `--json --no-progress-events`, capture stdout/stderr, and set a unique idempotency key per run when safe. Do not persistently disable non-priority agents unless a measured bonus connector blocks priority indexing; if temporary exclusion is necessary, use `--keep-indexed-data`, record before/after `cass sources agents list --json`, restore the prior config, and re-prove priority search after restoration.

### OOM, Stall, Quarantine, And Corruption

For each long recovery run, sample elapsed time, DB count delta, output progress, and process RSS at fixed intervals. If one interval shows zero progress or RSS exceeds the planned ceiling, send SIGTERM only to the recovery-owned process and switch once to canary/shadow route. A second stall blocks completion.

Recent logs show OOM quarantines for large Claude/Codex sessions. Quarantine is accounted evidence only when a specific source path and reason are recorded. A quarantine-heavy outcome that misses coverage thresholds is not success.

On WAL/FTS corruption, stop live mutation, preserve health/status/index output, and route once to shadow data-dir or upstream/frankensqlite blocker. No manual index/raw-mirror deletion is allowed.

## Runtime Recovery

The runtime canary path uses the installed binary first: prove the path, version, hash, capabilities, health/status, configured agents, and launchd state before mutating anything. The known starting evidence is that `cass` is `/Users/dalecarman/.local/bin/cass`, `cass --version` is `0.4.7`, `com.cass.index-watch` has a plist but is not loaded, and `com.cass.health-watchdog` calls a command absent from capabilities.

If the current runtime can perform priority watch-once canaries and lexical retrieval, proceed to broad recovery before code changes. If `com.cass.index-watch` can be loaded, kept alive, and proven with a new/modified priority-agent session, the absent `watchdog` subcommand is not a spec-completion blocker; record it as a durability follow-up. Do not inspect or integrate `src/watchdog.rs` in this recovery unless the user explicitly authorizes a separate watchdog issue.

## Watcher Repair And Durability

There are two launchd surfaces:

- `com.cass.index-watch`: must run `/Users/dalecarman/.local/bin/cass index --watch` or equivalent path to the same installed binary.
- `com.cass.health-watchdog`: is observed for context only. It blocks this spec only when evidence shows it unloaded, killed, or otherwise prevented `com.cass.index-watch` from staying running or indexing the probe.

Default repair path: load and prove `com.cass.index-watch` directly. If direct watcher proof fails and the captured failure shows a separate command-surface gap is blocking required watcher durability, stop this recovery path at a concrete blocker or create a follow-up issue. Do not improvise a watchdog command, wire `src/watchdog.rs`, or add capabilities/robot-docs/golden churn inside this recovery.

Do not delete or overwrite existing plists. If a new binary is needed for upstream/indexer/search changes, run the full verifier floor before deployment, preserve the old binary as a timestamped backup, and prove the installed binary hash matches the tested artifact.

Watcher proof is not plist existence. It requires:

- `launchctl list` or `launchctl print` shows `com.cass.index-watch` loaded.
- `ps` proves the process args use the same installed binary path/hash.
- `cass status --json` shows no active rebuild, no stale lock, and truthful watch/health state.
- A new or modified priority-agent session probe becomes searchable within 120 seconds. Use the natural current Codex session as the priority-agent probe when possible, with a unique harmless marker generated after watcher reload.
- Repeat launchd/process/search proof after 10 minutes. Include health-watchdog status in the receipt, but do not turn its parse error into a completion blocker unless it disrupts the required `com.cass.index-watch` proof.

## Verification

If no code changes are made, verification is the live evidence packet: git upstream proof, manifest, DB reconciliation, lexical searches, route-policy outputs, launchd proofs, and GoalBuddy state.

If code changes are made, run the repo-required verifier floor:

```bash
rch exec -- env CARGO_TARGET_DIR=/tmp/cass-check-target cargo check --all-targets
rch exec -- env CARGO_TARGET_DIR=/tmp/cass-check-target cargo clippy --all-targets -- -D warnings
rch exec -- env CARGO_TARGET_DIR=/tmp/cass-check-target cargo fmt --check
ubs $(git diff --name-only origin/main...HEAD)
```

Also run focused tests for touched behavior. If upstream or required code changes alter capabilities or robot docs, update and review goldens with:

```bash
UPDATE_GOLDENS=1 rch exec -- env CARGO_TARGET_DIR=/tmp/cass-golden-target cargo test --test golden_robot_json --test golden_robot_docs
```

Review every `tests/golden/` diff before accepting it.

## Spec 015 And Board Closeout

Spec 015 remains subordinate to this recovery. Its Pi watch-once implementation and receipts can be cited as evidence only after current live verification passes. This spec must explicitly mark spec 015 as superseded, subordinate, or closed; there must be no ambiguous state where spec 015 appears to be the path to the user's full outcome.

GoalBuddy state is a hard completion gate. The recovery is not done while `docs/goals/cass-session-ingestion-recovery/state.yaml` has required active or queued tasks for this outcome. Final audit must record `full_outcome_complete: true` only after upstream, priority ingestion/search, watcher durability, code verification when applicable, bead closeout, commit, and push are genuinely resolved or blocked with concrete evidence.


