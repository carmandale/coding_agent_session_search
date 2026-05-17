## Current Handoff

**Updated:** 2026-05-17T07:00:35Z
**Scope:** `specs/016-cass-recovery-ingestion/`
**Bead:** `coding_agent_session_search-1vxuf`
**GoalBuddy:** `docs/goals/cass-session-ingestion-recovery/state.yaml`
**Status:** blocked before live mutation

### Outcome Target

The actual user outcome is not artifact completion. CASS must be deliberately synced with upstream, Pi Agent/Claude Code/Codex histories must be processed and searchable in the live installed CASS system, OpenCode/factory must not regress, and `com.cass.index-watch` must keep new sessions searchable.

### Current Live Truth

- Live DB is still malformed: `PRAGMA quick_check` reports `Freelist: freelist leaf count too big`.
- Live counts are still under target: `pi_agent=1077`, `claude_code=2574`, `codex=5712`, `opencode=976`, `factory=66`, `messages=1055517`.
- Verified shadow archive is healthy and searchable: `pi_agent=2076`, `claude_code=2574`, `codex=5713`, `opencode=976`, `factory=66`, `messages=1238935`.
- `com.cass.index-watch` is still absent from `launchctl`.
- `com.cass.health-watchdog` is loaded but failing with last exit code `2` and log lines `Could not parse arguments`; latest read-only audit shows `runs=364`. Installed CASS still returns exit `2` for `watchdog run --help`, so live health-watchdog repair remains a follow-up unless it interferes with index-watch proof. Local source/debug and the rebuilt approval-gated release candidate now expose `cass watchdog run`: `/tmp/cass-check-target/debug/cass watchdog run --help` and `/tmp/cass-release-target/release/cass watchdog run --help` both exit `0`. Capabilities/introspect/robot-docs goldens, `cargo check --all-targets`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, focused stats tests, CLI-test UBS critical cleanup, release build, and release shadow canaries pass. No binary install or launchd smoke was run.
- Upstream remains unresolved: `HEAD=b807ef175dcdeeb48b912a22913fbcd68fb86cb8`, `upstream/main=1f20bd576f2e77a5197783c637fcc771ab9e1867`, ahead/behind `19/24`, `upstream/main` is not an ancestor of `HEAD`; latest non-destructive merge-tree output is `26ec8190e7ef955f263cac17f79eaef43ead9cfb`.
- GoalBuddy active task is now `T008`, a PM maintenance task that keeps the board valid while live mutation is approval-gated; `T005` remains blocked on explicit approval, and `completion-audit.md` says `full_outcome_complete: false`.
- `gate.sh record implement` refuses to mint `implement:complete:v1` because 19 live-proof/closeout tasks remain unchecked, so `/code-verify` is correctly blocked.
- T6 is now complete as current read-only route preflight: installed `cass status/health` show stale incomplete checkpoint with no active rebuild/watch/doctor repair, no cass writer processes matched, and read-only `cass doctor --json` stalled at 4m37s/11.7GB RSS before SIGTERM. This does not authorize live indexing.
- T7 is now complete only as a route-policy stop: the live stale-index refresh attempt was already recorded, exited `143` after SIGTERM, hit about `30640864 KB` RSS, had empty stdout/stderr, and paired verification still reported an unhealthy stale incomplete checkpoint. `evidence/runtime-refresh/t7-stale-refresh-stop.md` says not to retry live refresh against the malformed archive.
- T10 is now complete as not triggered: no non-priority connector blocked priority recovery, no `cass sources agents exclude` command was used, and `cass sources agents list --json` reports `disabled_agents=[]`, `total=0`.
- T8 canary identities are preselected in `evidence/canary/t8-canary-selection-readiness.md`; all selected paths are present in the frozen manifests and the selected strings exist in their source files. T8 remains unchecked because the actual `watch-once`/DB/search proof is live-gated.
- T11 has shadow-only reconciliation preflight in `evidence/reconciliation/t11-shadow-reconciliation-preflight.md`; it is not checked off. Shadow counts show `pi_agent` matched `2076/4174` manifest paths with all `2098` missing under `--clawdbot-chip--`, Claude matched `2413/2425`, Codex matched `5675/5868`, and duplicate non-null provenance keys are `0` for all three.
- The `--clawdbot-chip--` split is now classified enough to route. It is a symlink to `/Users/dalecarman/.clawdbot/agents/main/sessions` with `2098` JSONL files; spec 005 called dropping it an unacceptable regression. Current pinned FAD misses it because `pi_agent` ignores UUID-only filenames and `clawdbot` expects top-level role/content JSONL. Scratch release-candidate index of the symlink produced `0` rows; a normal Pi control file produced `1` row. Follow-up issue/spec created: `coding_agent_session_search-2d37b`, `specs/017-chipbot-symlink-indexing/`. T11/T12 remain unchecked for spec 016 live reconciliation.
- Spec 015 is routed as subordinate Pi watch-once evidence in `evidence/spec015-routing.md`; it is not the product-level completion owner and its own board remains active.
- Evidence hygiene is recorded in `evidence/evidence-hygiene.md`; refined credential scan found no key/secret patterns, and raw local-path telemetry is marked local-only unless verifier replay needs it.
- Latest read-only continuation audit is `evidence/continuation-audit-20260517T070035Z.md`: upstream remains `1f20bd576f2e77a5197783c637fcc771ab9e1867` with ahead/behind `19/24`, live quick_check still reports freelist errors through the proven encoded SQLite `mode=ro` URI, live CASS health reports `status=unhealthy`, `state.index.status=stale`, `state.index.checkpoint.completed=false`, and `state.pending.watch_active=false`, live `pi_agent=1077`, shadow remains healthy with `pi_agent=2076`, `com.cass.index-watch` is absent, health-watchdog has `runs=364` and last exit code `2`, installed `cass.real` still differs from the release candidate, and no live CASS mutation occurred.
- T008 maintenance evidence is `evidence/goalbuddy-board-maintenance-20260517T0433Z.md`: GoalBuddy checker passes with `active_task=T008`, YAML parse passes, runbook shell blocks pass `zsh -n`, `git diff --check` passes, repo-relative reference scanning found no missing first-read recovery evidence artifacts, and the live board API at `http://goalbuddy.localhost:41737/cass-session-ingestion-recovery/api/board` reports `activeTask=T008`.
- The short approval packet `evidence/operator-approval-packet.md` was refreshed at 2026-05-17T05:55:20Z so it matches current upstream drift, the corrected SQLite `mode=ro` probe, the shared `SPEC016_TS` guard, the durable dependency proof guard, the upstream/branch proof guard, the runtime install hash-equality guard, watcher-proof evidence persistence, pre-promotion capacity guard, and current `T008` board state.
- Latest dependency audit is `evidence/dependency-audit-20260517T024844Z.md`: CASS still resolves `fsqlite v0.1.3` and `fsqlite-types v0.1.3` from `/Users/dalecarman/dev/spec014-frankensqlite-fix`, whose branch is `fix/fts5-vtab-snapshot-via-delta-journal` at `f298dfa25064124374551737780fd7729ad350db` with dirty `crates/fsqlite-pager/src/pager.rs` and `crates/fsqlite-wal/src/wal.rs`. Durable pin/commit resolution remains approval-gated.
- The approval-gated runbook now includes a durable dependency proof block that fails before release build if the sibling frankensqlite checkout is still dirty/unpushed or if CASS still resolves `fsqlite`/`fsqlite-types` through the local `../spec014-frankensqlite-fix` path patch. It refreshes sibling remotes before remote-containment proof and catches the frankensqlite `[patch]` header itself.
- GoalBuddy prompt proof is now in `evidence/goalbuddy-board-maintenance-20260517T0433Z.md`: `npx goalbuddy prompt docs/goals/cass-session-ingestion-recovery --json` fails because `npx` resolves the relative path under the package directory, while `npx goalbuddy prompt /Users/dalecarman/dev/coding_agent_session_search/docs/goals/cass-session-ingestion-recovery --json` selects active task `T008` with `metadata.warnings=[]`.
- Latest read-only completion checklist audit is `evidence/continuation-audit-20260517T070035Z.md`; upstream remains 19 ahead/24 behind, live DB still reports freelist errors, live Pi Agent remains 1077 vs shadow 2076, `com.cass.index-watch` is absent, installed `cass.real` still differs from the release candidate, health-watchdog is at `runs=364` with last exit code `2`, and GoalBuddy still exposes active task `T008`.
- Runbook safety audit is `evidence/runbook-safety-audit-20260517T045057Z.md`: no active delete/reset/clean/force-push command is present in the approval packet/runbook; the only destructive-pattern hits were prohibition text, a `No rm` comment, and `launchctl bootout` inside approval-gated restore handling. Live promotion remains blocked on the explicit approval phrase.
- The approval-gated restore shape now also preserves failed promoted `agent_search.db-shm` and `agent_search.db-wal` sidecars before restoring `PRE-SPEC016` sidecars, avoiding restore-time collisions if a failed promoted DB was opened.
- Reference integrity follow-up on the active T008 surfaces found no missing first-read recovery evidence artifact. The only missing reference classifications were expected absent `code-verify.md`, parser false positives from path:line evidence strings, and prose shorthand such as `specs/019` / `tests/build`.
- The approval-gated synthetic Codex watcher proof now checks `test ! -e "$SYNTH_FILE"` and uses shell `noclobber` before writing the marker JSONL, so a rerun cannot overwrite an existing session artifact.
- The approval runbook pre-watcher archive health check now uses `--stale-threshold 86400`, not `1800`: read-only shadow proof showed `1800` exits `1` because the shadow index is older than 30 minutes, while `86400` exits `0` healthy with `checkpoint.completed=true`. Final freshness proof is still the approved watcher marker search.
- The approval runbook pre-install archive verification now uses `/tmp/cass-release-target/release/cass` instead of the old installed binary. Current proof: release candidate sha256 `a5a139ca503f0f04f2b0baba06e20b16bd7003a7e7d78306cacf94d56eaca9c2`, installed `cass.real` sha256 `47f0692af0fd6484e82e4b69b5512ba44b82de1d0c10d64b5a171b2ed279e691`, release watchdog help exits `0`, installed watchdog help exits `2`.
- The approval-gated watcher marker loop now tracks `found=1` only after `cass search` returns `.total_matches > 0`; if the marker is not searchable within 120 seconds, the runbook prints marker/source details to stderr and exits `1`.
- The approval runbook now passes `--data-dir "$LIVE_DIR"` explicitly for pre-install archive health/search and post-install watcher marker search, so all proof targets the promoted live CASS archive rather than relying on default data-dir resolution.
- The approval-gated watcher proof now saves launchctl output, process proof, marker search JSON, post-watcher health JSON, index-watch log tail, marker, and synthetic source path under `specs/016-cass-recovery-ingestion/evidence/watcher-proof/`.
- The approval-gated watcher process proof now excludes its own probe process and asserts a non-empty `cass index --watch` process result before continuing.
- The approval-gated promotion block now fails closed before DB/index moves if `com.cass.index-watch` is loaded or a non-probe CASS index/search/doctor/health process is active.
- The approval-gated promotion block now calculates the verified shadow DB/index copy footprint and current live-volume free space before DB/index moves, records both, and exits if available space is not greater than the copy footprint.
- The approval-gated runtime install block now verifies release and installed binary executability before/after replacing `cass.real`, requires the post-copy installed hash to equal the tested release hash, and the restore block can recover the `PRE-SPEC016` binary even when no failed replacement binary exists.
- The approval-gated promotion block now verifies required live DB/index paths, verified shadow DB/index paths, live destination writability, and the executable release candidate before any live DB/index move. It records the release hash and proves `cass watchdog run --help` from the release candidate before promotion starts.
- The approval-gated promotion and runtime install blocks now verify `PRE-SPEC016-$TS` backup destinations are unused before preserving live artifacts, so a reused approval token cannot overwrite previous-live backups.
- The approval-gated runtime install block now compares any rebuilt release hash to the release hash used for pre-promotion archive verification in the same approved attempt before installing.
- The approval-gated promotion block now rechecks shadow DB integrity, exact expected counts, and release-candidate shadow health before any live DB/index move, so approval-time promotion does not rely on stale shadow proof.
- The approval-gated watcher proof now verifies installed `cass.real` is executable, byte-for-byte equal to the tested release candidate, and exposes installed `cass watchdog run --help` before bootstrapping `com.cass.index-watch`.
- The approval-gated watcher proof now waits up to 30 seconds for a non-probe `cass index --watch` process after launchd bootstrap and exits with launchctl state if the process never appears.
- The approval-gated watcher marker loop now requires a search hit/result whose `source_path` equals the synthetic Codex file created by that proof attempt, not just any marker hit.
- The approval-gated restore block now preserves failed DB/index/watch artifacts only when present, so restore can still recover `PRE-SPEC016` backups if a failed replacement was never created.
- The approval-gated restore block now verifies mandatory `PRE-SPEC016-$TS` DB/index backups exist and no `FAILED-SPEC016-$TS` destination already exists before bootout or failed-artifact preservation begins.
- The approval-gated pre-watcher archive proof now checks exact promoted live counts for Pi Agent, Claude Code, Codex, OpenCode, factory, and messages, then requires all five proven lexical canaries to hit under `--data-dir "$LIVE_DIR"` before starting the watcher.
- The approval-gated post-watcher proof now requires installed live health to be healthy/ready/checkpoint-complete with `pending.watch_active=true` at the 1800-second threshold, plus index-watch log evidence after the marker search.
- The approval-gated runbook now requires one shared `SPEC016_TS` token for the whole approved attempt, so promotion, runtime install, watcher proof, and restore all use the same `PRE-SPEC016-$TS` and `FAILED-SPEC016-$TS` suffixes.
- The approval-gated runbook now has an explicit upstream/branch proof block after live proof; it fails closed before commit/push if `upstream/main` is not an ancestor of final `HEAD` or if the `dac/main` branch target is unresolved.

### Verified Readiness

- Release candidate exists: `/tmp/cass-release-target/release/cass`, sha256 `a5a139ca503f0f04f2b0baba06e20b16bd7003a7e7d78306cacf94d56eaca9c2`.
- Release candidate searches the verified shadow archive for Pi Agent, Claude Code, Codex, OpenCode, and factory canaries, and now exposes `cass watchdog run`.
- Final-close checkpoint regression is fixed in code: `close_storage_after_index` now uses `PRAGMA wal_checkpoint(RESTART)`, the exact failing test passes, and `cargo test checkpoint --lib` passes `58/58`.
- T19 is checked off. T20 remains unchecked for closeout because CI uses `ubs --ci --fail-on-warning`, but local UBS criticals are now clear, including `tests/cli_robot.rs`: current changed-file UBS is `0` critical, `20733` warnings, and `11159` info across `10` Rust files; the CI-shaped command still exits `1` warning-only. `evidence/ubs-warning-inventory.md` records the refreshed totals. Follow-up issue/spec created: `coding_agent_session_search-2v7tv`, `specs/019-ubs-warning-policy-closeout/`; `research.md` shows UBS comparison can report zero delta but still exits `1` with `--fail-on-warning`, and `policy-decision.md` rejects hidden baselines, broad ignores, or workflow weakening. Fmt/check/clippy, focused tests, and `git diff --check` pass. The broad `search_` CLI filter hit one `index-busy` race, but the exact failed test passed in isolation.
- `specs/019-ubs-warning-policy-closeout/policy-decision.md` now records the UBS decision surface: do not add hidden baselines, broad ignores, or workflow weakening. The narrow non-live closeout route for spec 016 T20 is final-review acceptance that the warning-only inventory is outside the live recovery gate; otherwise T20 stays unchecked.
- `com.cass.index-watch.plist` exists, lints OK, and points to `/Users/dalecarman/.local/bin/cass index --watch`.
- Synthetic Codex marker proof works in scratch full-index and `index --watch-once` flows with the release candidate.
- Runbook shell blocks pass `zsh -n`; required tools are available.
- Short operator approval packet is available at `specs/016-cass-recovery-ingestion/evidence/operator-approval-packet.md`.
- Promotion capacity/path permissions were checked: target volume currently has `150Gi` free as of 2026-05-17T05:09:27Z; shadow DB+index copy footprint was about `11.6G` in the prior preflight; live/shadow/bin/LaunchAgents/Logs paths were readable/writable/searchable as needed.
- Runbook includes a no-delete restore shape: failed promoted DB/index/binary artifacts get `FAILED-SPEC016-$TS` suffixes, and `PRE-SPEC016-$TS` backups are moved back into place.

### Approval Gate

Do not promote live DB/index, install the release binary, load the watcher, mutate `~/.codex/sessions`, commit, push, or resolve the frankensqlite branch/pin until Dale explicitly approves:

```text
I approve live CASS promotion, frankensqlite durable fix, and branch/commit resolution.
```

### Next Steps After Approval

1. Initialize one `SPEC016_TS` approval-attempt token and reuse it across dependency proof, promotion, install, watcher, restore, and branch/upstream proof.
2. Make `/Users/dalecarman/dev/spec014-frankensqlite-fix` durable: dirty files are `crates/fsqlite-pager/src/pager.rs` and `crates/fsqlite-wal/src/wal.rs` on `fix/fts5-vtab-snapshot-via-delta-journal`.
3. Replace CASS's local `../spec014-frankensqlite-fix` path patch with a durable committed/pushed frankensqlite revision or agreed fork pin.
4. Run the durable dependency proof block; stop if the sibling checkout is dirty/unpushed or CASS still resolves frankensqlite through a local path patch.
5. Rebuild/reverify release CASS from the durable dependency.
6. Promote verified shadow DB/index into live CASS with timestamped backups, no deletion.
7. Install the verified release binary, preserving old `cass.real` with a timestamped backup.
8. Load `com.cass.index-watch` and prove a synthetic Codex session marker under real `~/.codex/sessions/YYYY/MM/DD/` becomes searchable within 120 seconds.
9. If promotion/install verification fails, use the runbook restore shape with the same `SPEC016_TS`; do not delete failed promoted artifacts.
10. Re-run upstream/branch proof; stop before commit/push if upstream is not incorporated or if the branch target remains unresolved.
11. Refresh `completion-audit.md`, run `$code-verify`, then `$finalize`.

### Primary Evidence Files

- `specs/016-cass-recovery-ingestion/implement-receipt.md`
- `specs/016-cass-recovery-ingestion/completion-audit.md`
- `specs/016-cass-recovery-ingestion/evidence/live-promotion-runbook.md`
- `specs/016-cass-recovery-ingestion/evidence/operator-approval-packet.md`
- `specs/016-cass-recovery-ingestion/evidence/runtime-preflight/t6-current-route-preflight.md`
- `specs/016-cass-recovery-ingestion/evidence/runtime-refresh/t7-stale-refresh-stop.md`
- `specs/016-cass-recovery-ingestion/evidence/recovery-runs/t10-nonpriority-exclusion-not-triggered.md`
- `specs/016-cass-recovery-ingestion/evidence/canary/t8-canary-selection-readiness.md`
- `specs/016-cass-recovery-ingestion/evidence/reconciliation/t11-shadow-reconciliation-preflight.md`
- `specs/016-cass-recovery-ingestion/evidence/reconciliation/t12-chipbot-classification-followup.md`
- `specs/016-cass-recovery-ingestion/evidence/release-candidate-shadow-proof.md`
- `specs/016-cass-recovery-ingestion/evidence/final-checkpoint-restart-proof.md`
- `specs/016-cass-recovery-ingestion/evidence/ubs-warning-inventory.md`
- `specs/016-cass-recovery-ingestion/evidence/frankensqlite-fix-proof.md`
- `specs/016-cass-recovery-ingestion/evidence/upstream-blocker.md`
- `specs/016-cass-recovery-ingestion/evidence/upstream-working-tree-overlap.md`
- `specs/016-cass-recovery-ingestion/evidence/upstream-reconciliation-map.md`
- `specs/016-cass-recovery-ingestion/evidence/spec015-routing.md`
- `specs/016-cass-recovery-ingestion/evidence/evidence-hygiene.md`
- `specs/016-cass-recovery-ingestion/evidence/continuation-audit-20260517T024530Z.md`
- `specs/016-cass-recovery-ingestion/evidence/continuation-audit-20260517T030946Z.md`
- `specs/016-cass-recovery-ingestion/evidence/continuation-audit-20260517T040610Z.md`
- `specs/016-cass-recovery-ingestion/evidence/continuation-audit-20260517T041533Z.md`
- `specs/016-cass-recovery-ingestion/evidence/continuation-audit-20260517T042428Z.md`
- `specs/016-cass-recovery-ingestion/evidence/continuation-audit-20260517T050927Z.md`
- `specs/016-cass-recovery-ingestion/evidence/continuation-audit-20260517T062315Z.md`
- `specs/016-cass-recovery-ingestion/evidence/continuation-audit-20260517T070035Z.md`
- `specs/016-cass-recovery-ingestion/evidence/goalbuddy-board-maintenance-20260517T0433Z.md`
- `specs/016-cass-recovery-ingestion/evidence/dependency-audit-20260517T024844Z.md`
- `specs/017-chipbot-symlink-indexing/spec.md`
- `specs/018-health-watchdog-command-surface/spec.md`
- `specs/018-health-watchdog-command-surface/evidence/local-command-surface-proof.md`
- `specs/019-ubs-warning-policy-closeout/spec.md`
- `specs/019-ubs-warning-policy-closeout/research.md`
- `specs/019-ubs-warning-policy-closeout/policy-decision.md`
