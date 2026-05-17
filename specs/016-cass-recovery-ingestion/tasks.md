---
title: "Tasks: recover cass session ingestion and watcher"
date: 2026-05-16
bead: coding_agent_session_search-1vxuf
---

<!-- plan:complete:v1 | harness: unknown | date: 2026-05-16T13:47:37Z -->

# Tasks: recover cass session ingestion and watcher

Plan source of truth lives in [`plan.md`](plan.md); spec is [`spec.md`](spec.md). Tasks are grouped for `/codex-implement` chunking.

## Group A: Freeze Baseline, Route Policy, And Upstream State

- [x] T1: Create `specs/016-cass-recovery-ingestion/evidence/` and capture baseline files for `git fetch upstream main && git fetch origin`, `git rev-parse HEAD`, `git rev-parse upstream/main`, `git merge-base HEAD upstream/main`, `git rev-list --left-right --count HEAD...upstream/main`, `git status --short --branch`, `cass health --json`, `cass status --json`, `cass stats --json`, `cass sources agents list --json`, `which cass`, `cass --version`, and `shasum -a 256 "$(which cass)"`.
- [x] T2: Capture launchd/runtime baseline for `launchctl list | rg 'cass|coding-agent'`, `launchctl print gui/$(id -u)/com.cass.index-watch`, `launchctl print gui/$(id -u)/com.cass.health-watchdog`, plist contents under `~/Library/LaunchAgents/com.cass.*.plist`, and tail excerpts from `~/Library/Logs/cass-watchdog.log` plus `~/Library/Logs/cass-index-watch.log`.
- [x] T3: Write `specs/016-cass-recovery-ingestion/evidence/route-policy.md` with exact commands, thresholds, retry limits, stop conditions, and re-entry criteria for stale index, missing watcher, index lock/busy, doctor mutation lock, OOM/stall, WAL/FTS corruption, verifier failure, branch-policy conflict, and temporary agent scoping.
- [x] T4: Resolve upstream state. Incorporate `upstream/main` until `git merge-base --is-ancestor upstream/main HEAD` passes, or record a concrete blocker in `evidence/upstream-blocker.md` with exact conflict/policy evidence. Preserve local commits and target-scoped dirty-file safety; do not create a branch. Record the current branch and the finalization authorization needed if work remains on `dac/main`.
- [x] T5: Freeze connector-compatible priority manifests as JSONL under `evidence/manifests/` for Pi Agent, Claude Code, and Codex, recording `path`, `size_bytes`, `mtime_ms`, `agent`, and manifest window. Use connector-compatible filters, not naive all-file counts.

## Group B: Runtime-First Canary And Priority Recovery

- [x] T6: Before mutation, confirm `evidence/route-policy.md` exists and the current status satisfies it. Then inspect `cass status --json` for `rebuild.active`, `pending.watch_active`, `doctor_summary.active_repair`, recommended commands, and lock/busy state. If doctor or index mutation lock is active, follow `route-policy.md`; do not start a second writer.
- [x] T7: Run the stale-index refresh exactly once using the first `refresh-lexical-index` command from `cass health/status --json`, then run the paired health verification command. Save stdout/stderr/exit codes under `evidence/runtime-refresh/`.
- [ ] T8: Select one manifest identity per priority agent with safe non-secret text. Run the smallest valid `cass index --watch-once ... --json --no-progress-events` canary for each identity or root shape, then verify exact DB `source_path` presence and one lexical search hit for the selected string. Save commands and JSON under `evidence/canary/`.
- [ ] T9: If canary proof passes, run broad priority recovery separately for Pi Agent, Claude Code, and Codex using explicit watch-once roots and `--json --no-progress-events`. Capture process RSS/progress samples, stdout/stderr, exit codes, and route-policy decisions under `evidence/recovery-runs/`.
- [x] T10: If a non-priority connector blocks priority recovery, use `cass sources agents exclude <agent> --keep-indexed-data` only after saving before-state. Restore the prior config with `cass sources agents include <agent>`, save after-state, prove no configured agent remains disabled unless explicitly documented, and re-prove priority search after restoration.

## Group C: Identity, Integrity, And Search Proof

- [ ] T11: Build reconciliation tables under `evidence/reconciliation/` for each priority agent: raw manifest count, DB exact `source_path` count, accounted quarantine/skip count, missing paths, duplicate `source_path` rows, and duplicate non-null `(source_id, agent_id, external_id)` rows.
- [ ] T12: For Pi Agent, prove at least 95% of manifest paths are accounted by DB rows or path-specific quarantine/skip evidence. If below threshold, stop and record the exact missing-path set and blocker.
- [ ] T13: For Claude Code and Codex, prove recent and historical coverage with newest/oldest/random buckets. Sample at least 10 identities per agent, or all if fewer, and verify exact `source_path` mapping for each.
- [ ] T14: Prove lexical search with at least three safe real source strings per priority agent using `cass search "<string>" --agent <agent> --mode lexical --robot --fields minimal --robot-meta --limit 5`. Each proof must show at least one hit from the expected `source_path`.
- [ ] T15: Run idempotency proof by repeating the safe canary or priority watch-once command and verifying no duplicate source-path/provenance rows and no loss of existing priority rows. Then capture full-scope status/stats, record OpenCode, factory, and other non-priority counts against the refreshed baseline, and prove at least one safe lexical hit for OpenCode and factory when rows exist for them. No regression is allowed unless a path-specific blocker is recorded.

## Group D: Watcher Runtime Parity

- [ ] T16: Attempt watcher reload on the current installed runtime only after Group C passes: `launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/com.cass.index-watch.plist` when not already loaded, then verify with `launchctl print`, `launchctl list`, `ps`, `cass status --json`, and binary hash/path proof.
- [ ] T17: If direct `com.cass.index-watch` reload/probe fails, diagnose and fix the smallest non-destructive cause in this recovery, such as plist path/args/env, installed-binary mismatch, launchd reload, or indexer runtime error. Only destructive cleanup, missing credentials, or user-policy authorization can block the required watcher outcome. Do not wire `src/watchdog.rs` without explicit user authorization.
- [ ] T18: If health-watchdog is broken but direct `com.cass.index-watch` proof succeeds, record health-watchdog as a nonblocking follow-up and continue the required watcher proof. Do not route `com.cass.health-watchdog` to a new script or improvise a new command in this recovery.
- [x] T19: If code changed for upstream, indexer, search, or storage behavior, run focused tests for touched behavior. Update golden JSON/docs only through the documented `UPDATE_GOLDENS=1` test command and review every diff.
- [ ] T20: If code changed, run the verifier floor before any deployment: `rch exec -- env CARGO_TARGET_DIR=/tmp/cass-check-target cargo check --all-targets`, `rch exec -- env CARGO_TARGET_DIR=/tmp/cass-check-target cargo clippy --all-targets -- -D warnings`, `rch exec -- env CARGO_TARGET_DIR=/tmp/cass-check-target cargo fmt --check`, UBS on changed files, and focused tests for touched behavior.
- [ ] T21: After verifier success, build and deploy the verified binary without deleting the old one: move `/Users/dalecarman/.local/bin/cass.real` to a timestamped `cass.real.PRE-SPEC016-*` backup, copy `target/release/cass` to `cass.real`, and prove `cass --version`, capabilities, and SHA-256 match the tested binary.
- [ ] T22: Reload or install `com.cass.index-watch`, and reload `com.cass.health-watchdog` only if it is part of the chosen repair path. Capture `launchctl print`, process args, plist contents, and logs. Do not run uninstall or delete plists without explicit user permission.

## Group E: Watcher Durability And Final Evidence

- [ ] T23: Create a harmless unique marker in a natural priority-agent session after watcher reload, preferably the current Codex session. Verify it becomes searchable within 120 seconds through lexical robot search and save the search JSON under `evidence/watcher-proof/`.
- [ ] T24: Repeat launchd/process/search proof after 10 minutes. Confirm `com.cass.index-watch` remains loaded, `cass status --json` has no active rebuild/stale lock, and the unique marker remains searchable. Include health-watchdog status in the receipt, but block only if it disrupts the required index watcher proof.
- [x] T25: Route spec 015 by writing `evidence/spec015-routing.md` and updating the relevant spec/GoalBuddy artifacts so spec 015 is explicitly subordinate, superseded, or closed as evidence for this recovery.
- [x] T26: Run an evidence hygiene pass before committing artifacts: redact or summarize sensitive log/session excerpts, keep probe strings non-secret, and record any intentionally uncommitted evidence paths in the receipt.
- [x] T27: Write `specs/016-cass-recovery-ingestion/implement-receipt.md` with upstream proof, manifest counts, DB/search reconciliation, watcher durability proof, code-change summary if any, verifier commands, exact blockers if any, and the final done/not-done state.
- [ ] T28: Run `gate.sh record implement` and `gate.sh verify implement` for spec 016. Do not advance to `/code-verify` unless implementation evidence exists and route-policy blockers are resolved or explicitly recorded.

## Group F: Required Verification And Workflow Closeout

- [ ] T29: Run `$code-verify` after implementation and iterate until the verifier approves live acceptance. Treat missing manual/live proof as a blocker, not as a skipped test.
- [ ] T30: Run `$finalize` only after code verification approval. Close bead `coding_agent_session_search-1vxuf`, write handoff artifacts, stage only this spec's files and this session's code changes by name, then resolve branch authorization explicitly before commit/push: capture `git status --short --branch`, list intended staged files, and ask the user to authorize either committing/pushing from the current branch or a non-destructive move back to `main`. Push to origin only after that authorization, and sync `master` to `main` only if branch policy requires it and the target branch is resolved.
- [ ] T31: Update `docs/goals/cass-session-ingestion-recovery/state.yaml` receipts through the GoalBuddy workflow. Final audit must record `full_outcome_complete: true` only after upstream, priority searchability, watcher durability, code verification, finalize, and push are actually done or concretely blocked.

## Implementation Status Addendum

Updated 2026-05-17T04:24:28Z.

Do not infer completion from the unchecked task shape above. T005 is blocked at the live-approval boundary, not abandoned. The verified shadow archive, release candidate, approval runbook, restore shape, and first-read handoff now live in:

- `specs/016-cass-recovery-ingestion/implement-receipt.md`
- `specs/016-cass-recovery-ingestion/completion-audit.md`
- `specs/016-cass-recovery-ingestion/evidence/live-promotion-runbook.md`
- `specs/016-cass-recovery-ingestion/evidence/operator-approval-packet.md`
- `specs/016-cass-recovery-ingestion/evidence/runtime-preflight/t6-current-route-preflight.md`
- `specs/016-cass-recovery-ingestion/evidence/final-checkpoint-restart-proof.md`
- `thoughts/shared/handoffs/current.md`
- `docs/goals/cass-session-ingestion-recovery/state.yaml`

Current live blockers:

- Live DB still fails `PRAGMA quick_check` with freelist errors.
- Live `pi_agent` remains `1077`; verified shadow has `2076`.
- `com.cass.index-watch` is absent.
- `com.cass.health-watchdog` remains a nonblocking follow-up at the live surface; latest read-only audit shows exit `2` after `348` runs, and installed CASS still returns exit `2` for `watchdog run --help`. Local source/debug and the rebuilt approval-gated release candidate now expose `cass watchdog run`; release `cass watchdog run --help` exits `0`. Evidence is recorded in `coding_agent_session_search-2gif2`, `specs/018-health-watchdog-command-surface/evidence/local-command-surface-proof.md`. No binary install or launchd smoke has run.
- Latest upstream refresh moved `upstream/main` to `5156af7ecbfe3aa757a838ebfd6444d55f647896`; current divergence is `19` ahead / `23` behind and still blocked on branch/commit authorization.
- CASS still uses a local `../spec014-frankensqlite-fix` patch.
- T6 is complete as a read-only current route preflight: installed `cass status --json --robot-meta` exits `0` with stale checkpoint/no active rebuild/no watch/no active doctor repair; `cass health --json --robot-meta` exits `1` and recommends `cass index --full`; no active cass writer processes were found; read-only `cass doctor --json` stalled at 4m37s and 11.7GB RSS, was stopped with SIGTERM, and is recorded as non-viable quick preflight on the malformed live archive.
- T7 is complete only as a route-policy stop, not as a live repair. The already-recorded live stale-index refresh attempt exited `143` after SIGTERM, had empty stdout/stderr, reached about `30640864 KB` RSS, and paired verification still reported an unhealthy stale incomplete checkpoint. `evidence/runtime-refresh/t7-stale-refresh-stop.md` consolidates the evidence and says not to retry live refresh against the malformed archive.
- T10 is complete as not triggered. No non-priority connector blocked priority recovery, no `cass sources agents exclude` command was used, and `/Users/dalecarman/.local/bin/cass sources agents list --json` currently reports `disabled_agents=[]`, `total=0`. Evidence is in `evidence/recovery-runs/t10-nonpriority-exclusion-not-triggered.md`.
- T8 has a ready preselection artifact but remains unchecked. `evidence/canary/t8-canary-selection-readiness.md` records selected Claude Code, Codex, and Pi Agent source paths/strings; all selected paths are in the frozen manifests and all selected strings exist in the source files. The actual live `watch-once`, DB `source_path`, and lexical search proof remains approval-gated.
- T11 has a shadow-only reconciliation preflight but remains unchecked. `evidence/reconciliation/t11-shadow-reconciliation-preflight.md` records raw-manifest vs shadow-DB counts, missing-path shape, duplicate `source_path` groups, and duplicate provenance counts. It must be regenerated against live after promotion before T11/T12/T15 can close.
- The `--clawdbot-chip--` split now has a separate follow-up issue instead of staying ambiguous inside T12. `evidence/reconciliation/t12-chipbot-classification-followup.md` records that the path is a symlink to `/Users/dalecarman/.clawdbot/agents/main/sessions`, current pinned FAD `pi_agent` ignores UUID-only filenames, current `clawdbot` expects top-level role/content records, and scratch indexing the symlink produced `0` conversations while a normal Pi control file produced `1`. New bead/spec: `coding_agent_session_search-2d37b`, `specs/017-chipbot-symlink-indexing/`. T11/T12 remain unchecked because live promotion and live reconciliation have not run.
- T19 is complete after the final-close checkpoint regression fix: the exact failing test now passes, `cargo test checkpoint --lib` passes `58/58`, and later focused redaction/UI/CLI tests also pass.
- T20 Rust verifier is clear after the local health-watchdog command-surface repair and the touched CLI test critical cleanup: fmt/check/clippy pass; focused CLI/capabilities/robot-docs/stats tests pass; the one broad `search_` filter failure was `index-busy` in parallel and the exact test passed in isolation. `ubs tests/cli_robot.rs` now exits `0` with `0` criticals after replacing existing assertion-helper `panic!` macros, and current changed-file UBS exits `0` with `0` critical, `20733` warnings, and `11159` info findings across `10` Rust files. UBS remains a closeout blocker because the repository CI-shaped `--fail-on-warning` command exits `1` on warning inventory. Follow-up issue/spec: `coding_agent_session_search-2v7tv`, `specs/019-ubs-warning-policy-closeout/`. Its `policy-decision.md` rejects hidden baselines, broad ignores, or workflow weakening; T20 still needs final-review acceptance, warning cleanup, or a separately reviewed UBS policy/wrapper route.
- Latest `gate.sh record implement specs/016-cass-recovery-ingestion/` rerun after T10 checkoff still refuses completion with `19` unchecked tasks. The remaining unchecked tasks are the live canary/recovery/reconciliation/search/idempotency tasks, watcher reload/proof tasks, deployment/final gates, and GoalBuddy final audit.
- Latest release candidate hash is `a5a139ca503f0f04f2b0baba06e20b16bd7003a7e7d78306cacf94d56eaca9c2`; it remains approval-gated and is not installed.
- Latest live SQLite read-only command-shape audit is `evidence/continuation-audit-20260517T042428Z.md`: `sqlite3 -readonly "$LIVE_DIR/agent_search.db"` fails with SQLite code `14` against the current live DB, while the encoded `mode=ro` URI works and still reports the same freelist errors. `live-promotion-runbook.md` now uses the encoded URI shape for approval-gated live integrity and restore checks.
- GoalBuddy board mechanics are now valid again: `T005` remains blocked, and `T008` is the active PM maintenance task that keeps the continuous board alive without authorizing live mutation. `check-goal-state.mjs docs/goals/cass-session-ingestion-recovery/state.yaml` passes.

Next real step remains explicit approval:

```text
I approve live CASS promotion, frankensqlite durable fix, and branch/commit resolution.
```
