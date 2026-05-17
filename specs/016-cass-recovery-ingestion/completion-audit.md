---
title: "Completion audit: cass session ingestion recovery"
date: 2026-05-17T07:00:35Z
bead: coding_agent_session_search-1vxuf
full_outcome_complete: false
---

# Completion Audit

## Objective Restated

The user asked for cass to be deliberately in sync with upstream, to process all important local agent sessions into a searchable system, and to run a watcher so new/changed sessions continue to be processed automatically.

Concrete completion criteria:

1. Upstream state is incorporated or explicitly blocked with current evidence.
2. Live cass, not only a shadow copy, has Pi Agent, Claude Code, and Codex sessions processed and searchable.
3. Bonus session families such as OpenCode and factory do not regress and have at least one lexical proof when rows exist.
4. `com.cass.index-watch` is loaded/running and proves a new or modified session becomes searchable.
5. Code changes are verified, durable, and not dependent on a local-only sibling patch.
6. `$code-verify`, `$finalize`, bead closure, commit, and push are complete or explicitly blocked.

## Prompt-To-Artifact Checklist

| Requirement | Evidence inspected | Current result |
| --- | --- | --- |
| GoalBuddy goal exists | `docs/goals/cass-session-ingestion-recovery/goal.md`; `docs/goals/cass-session-ingestion-recovery/state.yaml` | Present. Goal remains active/blocked, not complete. |
| Issue/spec exists | `specs/016-cass-recovery-ingestion/spec.md`; bead `coding_agent_session_search-1vxuf` in receipts | Present. |
| Operator approval surface | `evidence/operator-approval-packet.md`; `evidence/live-promotion-runbook.md`; `thoughts/shared/handoffs/current.md` | Present. The short packet now states the exact approval phrase, authorized actions, non-authorized actions, current blockers, and pointer to the detailed runbook. The detailed runbook now initializes one shared `SPEC016_TS` token before any dependency, promotion, runtime install, watcher, restore, or branch/upstream block so backup/failed-artifact suffixes stay aligned. It fails before release build if the frankensqlite sibling checkout is still dirty/unpushed or CASS still resolves through the local path patch, after refreshing sibling remotes and checking the frankensqlite `[patch]` header itself. It checks live-volume free space against the verified shadow DB/index copy footprint before moving live artifacts. It now also verifies the required live DB/index, shadow DB/index, writable live destination, and executable release candidate before any live DB/index move, then records the release hash and proves the release `watchdog run` command surface. Before preserving live artifacts, promotion and runtime install verify their `PRE-SPEC016` backup destinations are unused so a reused attempt token cannot overwrite previous-live backups. If runtime install rebuilds after pre-promotion archive verification, it compares the rebuilt release hash to the pre-promotion release hash before installing, keeping archive proof and installed runtime tied to the same binary. It rechecks shadow DB integrity, exact expected counts, and release-candidate shadow health before any live move so promotion does not rely on stale shadow proof. The restore block now fails before bootout or failed-artifact preservation if mandatory `PRE-SPEC016` DB/index backups are missing or if the `FAILED-SPEC016` suffix has already been used. Before bootstrapping `com.cass.index-watch`, it requires installed `cass.real` to match the tested release hash and expose the installed `watchdog run` command surface. The watcher marker proof now requires a search hit/result whose `source_path` equals the synthetic file created by that proof attempt. It saves eventual watcher proof under `evidence/watcher-proof/`. It also fails closed before commit/push if post-live upstream proof does not show `upstream/main` as an ancestor of final `HEAD` or if the `dac/main` branch target is unresolved. The runtime install block now requires the post-copy installed `cass.real` hash to equal the tested release hash. This does not complete live recovery; it only makes the approval boundary explicit. |
| Upstream current state known | `git fetch upstream main`; `git rev-parse HEAD`; `git rev-parse upstream/main`; `git rev-list --left-right --count HEAD...upstream/main`; `git merge-tree --write-tree HEAD upstream/main`; dirty/upstream overlap check | Not complete. Local `upstream/main` is now `1f20bd576f2e77a5197783c637fcc771ab9e1867`; local `HEAD` is `b807ef175dcdeeb48b912a22913fbcd68fb86cb8` on `dac/main`; ahead/behind is `19 24`; merge-tree exits 0 with tree `26ec8190e7ef955f263cac17f79eaef43ead9cfb`; branch-policy authorization still blocks incorporation. Uncommitted recovery changes overlap upstream changes, including upstream deletes of `src/indexer/scratch_root.rs` and `tests/spec_015_streaming_watch_once.rs`. |
| Live DB integrity | `sqlite3 "file:$LIVE?mode=ro" "PRAGMA quick_check; ..."` | Not complete. Live DB still reports many `Freelist: freelist leaf count too big` errors. |
| Live priority counts | Same read-only live DB query | Not complete. Live rows: `claude_code=2574`, `codex=5712`, `pi_agent=1077`, `messages=1055517`. Pi Agent remains under-indexed live. |
| Shadow DB integrity | `sqlite3 "$SHADOW/agent_search.db" "PRAGMA integrity_check; ..."` from implementation receipt | Complete only in shadow. Shadow DB integrity is `ok`. |
| Shadow priority counts | Shadow query from implementation receipt | Complete only in shadow. Shadow rows: `pi_agent=2076`, `claude_code=2574`, `codex=5713`, `messages=1238935`. |
| Shadow bonus counts | Shadow query from implementation receipt | Complete only in shadow. Shadow rows: `opencode=976`, `factory=66`. |
| Shadow lexical search | `cass search ... --data-dir "$SHADOW"` canaries in implementation receipt and release-candidate shadow proof | Complete only in shadow. Pi Agent, Claude Code, Codex, OpenCode, and factory canaries returned hits from both debug and release-candidate binaries. |
| Route-policy preflight | `evidence/route-policy.md`; `evidence/runtime-preflight/t6-current-route-preflight.md`; installed `cass status/health/doctor` probes | T6 complete as read-only preflight. Status/health show stale incomplete checkpoint with no active rebuild/watch/doctor repair; no cass writer processes matched; read-only doctor stalled at 4m37s/11.7GB RSS and was stopped with SIGTERM. This does not clear live corruption or authorize live indexing. |
| Stale-index refresh route | `evidence/runtime-refresh/t7-stale-refresh-stop.md`; `evidence/runtime-refresh/refresh-lexical-index.*`; `evidence/runtime-refresh/verify-refresh-status.*`; `evidence/runtime-refresh/verify-refresh-health.*` | T7 complete only as a failed route-policy attempt. The live refresh exited `143` after SIGTERM, reached about `30640864 KB` RSS, had empty stdout/stderr, and paired verification still reported an unhealthy stale incomplete checkpoint. This is not live success and must not be retried against the malformed live archive. |
| Priority canary selection | `evidence/canary/canary-selection.json`; `evidence/canary/t8-canary-selection-readiness.md`; frozen manifests; read-only source-string probes | Preselected only. T8 remains incomplete because no approval-gated live `watch-once`, live DB `source_path`, or live lexical search proof has run. |
| Non-priority connector exclusion | `evidence/recovery-runs/t10-nonpriority-exclusion-not-triggered.md`; `/Users/dalecarman/.local/bin/cass sources agents list --json`; evidence grep for `sources agents exclude/include` | T10 complete as not triggered. No non-priority connector blocked priority recovery, no exclusion command was used, and current global exclusion state is `disabled_agents=[]`, `total=0`. |
| Priority reconciliation | `evidence/reconciliation/t11-shadow-reconciliation-preflight.md`; `evidence/reconciliation/t12-chipbot-classification-followup.md`; frozen manifests; shadow DB `conversations`/`agents` read-only queries | Shadow-only preflight exists. T11 remains incomplete because live DB reconciliation has not run after promotion. The shadow preflight found Pi matched `2076/4174` manifest paths and all `2098` missing Pi manifest paths are under `--clawdbot-chip--`; Claude matched `2413/2425`; Codex matched `5675/5868`; duplicate non-null provenance keys are `0` for all three. Follow-up issue `coding_agent_session_search-2d37b` / `specs/017-chipbot-symlink-indexing/` now tracks the chipbot symlink connector gap; T12 remains unchecked for live reconciliation. |
| Live watcher loaded | `launchctl list`; `launchctl print gui/$(id -u)/com.cass.index-watch` | Not complete. `com.cass.index-watch` is not loaded. |
| Health watchdog status | `launchctl print gui/$(id -u)/com.cass.health-watchdog`; installed/release `cass watchdog run --help`; `specs/018-health-watchdog-command-surface/evidence/local-command-surface-proof.md` | Broken live/nonblocking follow-up, partially repaired through release candidate. Service is loaded but not running, last exit code `2`, latest read-only audit shows `runs=364`, and arguments are `/Users/dalecarman/.local/bin/cass watchdog run`; the installed CASS still has the old parse failure. Local source/debug and the current approval-gated release candidate now wire `cass watchdog run`; `/tmp/cass-release-target/release/cass watchdog run --help` exits `0`, and shadow canaries still pass. No binary install or launchd smoke has run, so spec018 remains incomplete. |
| New/modified session searchable by watcher | No live watcher proof exists | Not complete. |
| Code verifier floor | `cargo fmt --check`; `cargo check --all-targets`; `cargo clippy --all-targets -- -D warnings`; focused tests; debug build; release build; release shadow canaries | Mostly complete. Compiler, clippy, focused tests, and release build pass. After local spec018 command-surface repair, `cargo check --all-targets`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `watchdog_run_help_dispatches`, capabilities CLI tests, affected golden tests, and CLI stats tests pass against `/tmp/cass-check-target`. The current release candidate was rebuilt at `/tmp/cass-release-target/release/cass`, sha256 `a5a139ca503f0f04f2b0baba06e20b16bd7003a7e7d78306cacf94d56eaca9c2`, and it passes watchdog help plus shadow health/search canaries. It is not installed. |
| UBS changed-file gate | `git diff --name-only -- '*.rs' Cargo.toml Cargo.lock | tr '\n' '\0' | xargs -0 ubs --format=json --jsonl-summary-only`; `xargs ubs --ci --fail-on-warning ...`; `evidence/ubs-warning-inventory.md`; `specs/019-ubs-warning-policy-closeout/research.md`; `specs/019-ubs-warning-policy-closeout/policy-decision.md`; spec018 scoped UBS probes | Local criticals are clear. After replacing the existing `panic!` inventory in `tests/cli_robot.rs`, `ubs tests/cli_robot.rs` exits `0` with `0` critical, `1585` warning, and `410` info findings; spec018 touched-set UBS exits `0` with `0` critical, `1694` warnings, and `557` info findings; current changed-file UBS exits `0` with `0` critical, `20733` warnings, and `11159` info findings across `10` Rust files. CI-shaped `--fail-on-warning` still exits `1` because warnings are merge-blocking. No UBS policy/config baseline was added. Follow-up issue `coding_agent_session_search-2v7tv` / `specs/019-ubs-warning-policy-closeout/` still tracks the required UBS policy decision. T20 remains unchecked unless final review accepts the warning-only inventory as outside the live recovery gate, warnings are cleaned up, or a separately reviewed UBS policy/wrapper route is selected. |
| Durable frankensqlite fix | `/Users/dalecarman/dev/spec014-frankensqlite-fix` status, focused tests, and cass `Cargo.toml` patch | Not complete. Local proof is current: `cargo fmt -p fsqlite-pager -p fsqlite-wal --check` passed, WAL zero-byte truncate test passed, and pager freelist tests passed 23/23. The sibling checkout still has uncommitted pager/WAL fixes, and cass currently points to local `../spec014-frankensqlite-fix`. |
| Live deployment | Installed `/Users/dalecarman/.local/bin/cass` and live data dir | Not complete. Release candidate exists in `/tmp/cass-release-target/release/cass`, but verified code has not been installed and shadow DB/index has not been promoted. |
| `$code-verify` | `specs/016-cass-recovery-ingestion/code-verify.md` | Not complete. File absent/not run. |
| `$finalize`, commit, push | Git status and GoalBuddy state | Not complete. Checkout is `dac/main`; many scoped/unrelated dirty/untracked files remain; no final commit/push. |
| Spec 015 routing | `evidence/spec015-routing.md`; `docs/goals/watch-once-streaming-scan/state.yaml`; `specs/015-watch-once-streaming-scan/tasks.md` | Partially routed. Spec 015 is recorded as subordinate Pi watch-once evidence, not the product-level completion owner. Its own board remains active and its full-corpus/watch proof tasks are still unchecked. |
| Evidence hygiene | `evidence/evidence-hygiene.md`; refined credential scan; oversized evidence inventory | Complete for current artifacts. No credential/key patterns found; raw local-path and bulky telemetry paths are recorded as local-only unless verifier replay requires them. |

## Current Live Versus Shadow Summary

Live production archive:

```text
quick_check: malformed freelist entries
pi_agent=1077
claude_code=2574
codex=5712
opencode=976
factory=66
messages=1055517
com.cass.index-watch: not loaded
```

Verified shadow archive:

```text
integrity_check: ok
pi_agent=2076
claude_code=2574
codex=5713
opencode=976
factory=66
messages=1238935
health: healthy, lexical ready, checkpoint completed
watch_active: false
```

## Latest Continuation Audit

Read-only continuation audit rerun on 2026-05-17T07:00:35Z:

```text
remote upstream/main: 1f20bd576f2e77a5197783c637fcc771ab9e1867
HEAD: b807ef175dcdeeb48b912a22913fbcd68fb86cb8
branch: dac/main
merge-base: 3763b33132c78ecb541180f05e1b1dd6ec6719e1
ahead/behind: 19/24
live quick_check sample: still reports Freelist: freelist leaf count too big
live rows: pi_agent=1077, claude_code=2574, codex=5712, opencode=976, factory=66, messages=1055517
live health: status=unhealthy, index=stale, checkpoint.completed=false, pending.watch_active=false
shadow: integrity_check=ok, release health exit=0 at stale-threshold 86400, pi_agent=2076, claude_code=2574, codex=5713, opencode=976, factory=66, messages=1238935
release candidate: sha256 a5a139ca503f0f04f2b0baba06e20b16bd7003a7e7d78306cacf94d56eaca9c2, watchdog run --help exits 0
installed cass.real: sha256 47f0692af0fd6484e82e4b69b5512ba44b82de1d0c10d64b5a171b2ed279e691, watchdog run --help exits 2 with "Could not parse arguments"
com.cass.index-watch: absent from gui/501
com.cass.health-watchdog: loaded but not running, last exit code 2, runs=364
GoalBuddy: check-goal-state passes; absolute npx prompt returns task T008 with no warnings; board API exposes .goal.activeTask=T008
```

This rerun keeps the same completion decision: the live production surface is
still not repaired, priority sessions are still not fully searchable from live
CASS, the installed runtime still differs from the verified release candidate,
and the watcher is still absent. The detailed evidence is
`evidence/continuation-audit-20260517T070035Z.md`.

Read-only continuation audit rerun on 2026-05-17T06:07:21Z:

```text
git fetch upstream main: upstream/main remained 1f20bd576f2e77a5197783c637fcc771ab9e1867
HEAD: b807ef175dcdeeb48b912a22913fbcd68fb86cb8
branch: dac/main
merge-base: 3763b33132c78ecb541180f05e1b1dd6ec6719e1
ahead/behind: 19/24
merge-tree: 26ec8190e7ef955f263cac17f79eaef43ead9cfb
live quick_check sample: still reports Freelist: freelist leaf count too big
live rows: pi_agent=1077, claude_code=2574, codex=5712, opencode=976, factory=66, messages=1055517
live health: exit=1, healthy=false, index=stale, checkpoint.completed=false, pending.watch_active=false
shadow: quick_check=ok, health exit=0 at stale-threshold 86400, pi_agent=2076, claude_code=2574, codex=5713, opencode=976, factory=66, messages=1238935
com.cass.index-watch: absent from gui/501
com.cass.health-watchdog: loaded but not running, last exit code 2, runs=359
```

This rerun keeps the same completion decision: the live production surface is
still not repaired, priority sessions are still not fully searchable from live
CASS, and the watcher is still absent. The detailed evidence is
`evidence/continuation-audit-20260517T060721Z.md`.

## Continuation Audit Refresh

Refreshed read-only live evidence on 2026-05-16T23:18:54Z:

```text
git fetch upstream main: upstream/main remains 37b42058312d4aafa4a45ede8ae81ff5b8a07134
HEAD: b807ef175dcdeeb48b912a22913fbcd68fb86cb8
merge-base: 3763b33132c78ecb541180f05e1b1dd6ec6719e1
ahead/behind: 19 17
upstream/main ancestor of HEAD: no
```

```text
live quick_check: still reports Freelist: freelist leaf count too big
live rows: pi_agent=1077, claude_code=2574, codex=5712, opencode=976, factory=66, messages=1055517
com.cass.index-watch: service absent from gui/501
com.cass.health-watchdog: loaded but not running, last exit code 2
```

The continuation audit does not change the decision: the live production surface is still not repaired, searchable for all priority histories, or watched.

Current-state audit rerun on 2026-05-16T23:38:36Z:

```text
live quick_check: still reports Freelist: freelist leaf count too big
live rows: pi_agent=1077, claude_code=2574, codex=5712, opencode=976, factory=66, messages=1055517
com.cass.index-watch: still absent from gui/501
com.cass.health-watchdog: still loaded but not running, last exit code 2
git: HEAD=b807ef175dcdeeb48b912a22913fbcd68fb86cb8, upstream/main=37b42058312d4aafa4a45ede8ae81ff5b8a07134, ahead/behind=19/17, upstream/main ancestor of HEAD=no
GoalBuddy: active_task=T008, T008 status=active; T005 remains blocked on explicit approval
```

This rerun confirms no live acceptance criterion has silently become true.

Read-only continuation audit rerun on 2026-05-16T23:51:25Z:

```text
git: HEAD=b807ef175dcdeeb48b912a22913fbcd68fb86cb8, upstream/main=37b42058312d4aafa4a45ede8ae81ff5b8a07134, ahead/behind=19/17, upstream/main ancestor of HEAD=no
live quick_check: still reports Freelist: freelist leaf count too big
live rows: pi_agent=1077, claude_code=2574, codex=5712, opencode=976, factory=66, messages=1055517
com.cass.index-watch: still absent from gui/501
com.cass.health-watchdog: still loaded but not running, last exit code 2
promotion capacity: target volume has 174Gi free; shadow dir is 13G
release candidate: /tmp/cass-release-target/release/cass sha256=fbf044a4fa9c081cf83d0d56a5b83320e4d1a0008d104ccc09e13461d06e904b
```

Focused checkpoint and release refresh on 2026-05-17T00:18:20Z:

```text
final checkpoint fix: close_storage_after_index now runs PRAGMA wal_checkpoint(RESTART)
exact failing test: close_storage_after_index_checkpointing_close_does_not_leave_backfillable_wal_frames passed
checkpoint test group: cargo test checkpoint --lib passed 58/58
fmt/check/clippy: passed with direct cargo fallback because rch is unavailable
release candidate: /tmp/cass-release-target/release/cass sha256=077674c65899936a79885d24cf141e1ac05632e5bd201958a1a6a992fda20594
release shadow proof: healthy=true at 86400s threshold; pi_agent=30, claude_code=37, codex=10, opencode=2484, factory=21 lexical matches
UBS focused rerun: ubs src/indexer/mod.rs failed on existing broad inventory, with internal fmt/clippy/check/test-build clean
```

This rerun keeps the same decision. The remaining work is still blocked on explicit approval for live promotion, durable dependency/branch resolution, watcher loading, and closeout.

Read-only continuation audit rerun on 2026-05-17T00:25:43Z:

```text
git fetch upstream main: upstream/main advanced to 956f1d3baf2881e792b5d3397d1875789476f587
HEAD: b807ef175dcdeeb48b912a22913fbcd68fb86cb8
merge-base: 3763b33132c78ecb541180f05e1b1dd6ec6719e1
ahead/behind: 19/18
merge-tree: 239b49b7afc81c228be8c63a1b3cbb19d84f309b
live quick_check: still reports Freelist: freelist leaf count too big
live rows: pi_agent=1077, claude_code=2574, codex=5712, opencode=976, factory=66, messages=1055517
com.cass.index-watch: still absent from gui/501
com.cass.health-watchdog: still loaded but not running, last exit code 2
release candidate: /tmp/cass-release-target/release/cass sha256=077674c65899936a79885d24cf141e1ac05632e5bd201958a1a6a992fda20594
```

This rerun kept the same decision at that point, with one sharper blocker:
upstream had moved again. The upstream target was later superseded by the
2026-05-17T01:29:00Z audit below.

Verifier refresh on 2026-05-17T00:47:53Z:

```text
release candidate: /tmp/cass-release-target/release/cass sha256=db3dbb0a9652bc5cadfa9a7d824da13a529d9cd2ad6ad85dc169a0760b0a7f1c
release shadow proof: healthy=true at 86400s threshold; pi_agent=30, claude_code=37, codex=10, opencode=2484, factory=21 lexical matches
ubs src/indexer/mod.rs: pass with 0 critical, 5962 warnings, 2052 info
changed-file UBS summary: fail with 41 critical, 19148 warnings, 10735 info across 9 files
```

This kept the same decision at that point. That release candidate and UBS state were later superseded by the 2026-05-17T01:18:03Z verifier refresh below.

Verifier refresh on 2026-05-17T01:18:03Z:

```text
fmt/check/clippy: passed with direct cargo fallback because rch is unavailable
focused tests: redaction quarantine, state_save, robot-format/default-search/refresh flags all passed
git diff --check: passed
changed-file UBS summary: pass with 0 critical, 19148 warnings, 10752 info across 9 files
CI-shaped UBS: exit 1 because --fail-on-warning treats the same warning inventory as merge-blocking
warning inventory artifact: specs/016-cass-recovery-ingestion/evidence/ubs-warning-inventory.md
release candidate: /tmp/cass-release-target/release/cass sha256=423e2e4c2920ec74a38a5cb4af1f00de362a4a82e493d342b4891179f4955ada
release shadow proof: healthy=true at 86400s threshold; pi_agent=30, claude_code=37, codex=10, opencode=2484, factory=21 lexical matches
```

This keeps the same completion decision. The local verifier criticals are clear, but the live production surface is still not repaired, promoted, watched, or finalized; warning-level UBS inventory also remains a CI-policy caveat before merge claims.

Read-only continuation audit rerun on 2026-05-17T01:29:00Z:

```text
git fetch upstream main: upstream/main advanced to e337b9f428e12ea5a0d5b37129d3abb0dea48ab8
HEAD: b807ef175dcdeeb48b912a22913fbcd68fb86cb8
merge-base: 3763b33132c78ecb541180f05e1b1dd6ec6719e1
ahead/behind: 19/19
merge-tree: 124403bc99be2effce1bbc9bc9cc39d330639ef6
live quick_check: still reports Freelist: freelist leaf count too big
live rows: pi_agent=1077, claude_code=2574, codex=5712, opencode=976, factory=66, messages=1055517
com.cass.index-watch: still absent from gui/501
com.cass.health-watchdog: still loaded but not running, last exit code 2
```

This keeps the same completion decision and updates the upstream target for the eventual approved reconciliation.

Read-only continuation audit rerun on 2026-05-17T02:20:32Z:

```text
git fetch upstream main: upstream/main advanced to 485ff1052b48e8d731a9ca9da03ba1d3dd170a82
HEAD: b807ef175dcdeeb48b912a22913fbcd68fb86cb8
merge-base: 3763b33132c78ecb541180f05e1b1dd6ec6719e1
ahead/behind: 19/22
merge-tree: b0ef9f483fefce743323ab78b857b704dfaa5b13
live quick_check sample: still reports Freelist: freelist leaf count too big
live rows: pi_agent=1077, claude_code=2574, codex=5712, opencode=976, factory=66, messages=1055517
com.cass.index-watch: still absent from gui/501
com.cass.health-watchdog: still loaded but not running, last exit code 2
process scan: no cass index/doctor/watchdog/release/debug worker matched
follow-up issue: coding_agent_session_search-2d37b remains in_progress
```

This keeps the same completion decision with a sharper upstream blocker:
upstream moved again while the live data and watcher blockers did not improve.

Read-only continuation audit rerun on 2026-05-17T02:45:30Z:

```text
git fetch upstream main: upstream/main remained 485ff1052b48e8d731a9ca9da03ba1d3dd170a82
HEAD: b807ef175dcdeeb48b912a22913fbcd68fb86cb8
merge-base: 3763b33132c78ecb541180f05e1b1dd6ec6719e1
ahead/behind: 19/22
merge-tree: b0ef9f483fefce743323ab78b857b704dfaa5b13
live quick_check sample: still reports Freelist: freelist leaf count too big
live rows: pi_agent=1077, claude_code=2574, codex=5712, opencode=976, factory=66, messages=1055517
com.cass.index-watch: still absent from gui/501
com.cass.health-watchdog: loaded but not running, last exit code 2, runs=339
process scan: no cass worker processes matched
target free space: 171Gi
```

This keeps the same completion decision. The latest audit is recorded in
`evidence/continuation-audit-20260517T024530Z.md`.

Local verifier/UBS refresh on 2026-05-17T03:45:56Z:

```text
tests/cli_robot.rs panic! inventory: rg found no matches after replacement with std::panic::panic_any
fmt/check/clippy: passed with direct cargo fallback
focused CLI tests: capabilities=13 passed, watchdog_run_help_dispatches=1 passed, stats_=6 passed
broad search_ filter: 67/68 passed; search_cursor_manifest_marks_rebuilding_generation_best_effort returned index-busy under parallel broad run
exact failed search test rerun: passed in isolation
ubs tests/cli_robot.rs: exit 0, critical=0, warning=1585, info=410
ubs src/watchdog.rs tests/cli_robot.rs Cargo.toml Cargo.lock: exit 0, critical=0, warning=1694, info=557
current changed-file UBS: exit 0, critical=0, warning=20733, info=11159, files=10
CI-shaped UBS: exit 1, critical=0, warning=20733, info=11159, files=10
```

This removes the touched CLI test criticals but keeps the same completion
decision: T20 remains unchecked because the repository's CI-shaped
`--fail-on-warning` gate is still warning-blocked, and the live production
surface remains unpromoted, unwatched, and malformed.

Approval-gated release rebuild on 2026-05-17T04:00:58Z:

```text
release build: passed in /tmp/cass-release-target
release candidate: /tmp/cass-release-target/release/cass
version: cass 0.4.7
sha256: a5a139ca503f0f04f2b0baba06e20b16bd7003a7e7d78306cacf94d56eaca9c2
release watchdog help: exits 0 and prints "Run a one-shot health check"
release shadow health: healthy=true, index.status=ready, index.fresh=true, checkpoint.completed=true, checkpoint.db_matches=true
release shadow lexical canaries: pi_agent=30, claude_code=37, codex=10, opencode=2484, factory=21
```

This supersedes the prior release-candidate hash and proves the release binary
now includes the watchdog command-surface repair. It does not change the
completion decision because the binary is not installed, the live archive is
not promoted, and launchd watcher proof has not run.

Read-only continuation audit rerun on 2026-05-17T02:54:54Z:

```text
git fetch upstream main: upstream/main remained 485ff1052b48e8d731a9ca9da03ba1d3dd170a82
HEAD: b807ef175dcdeeb48b912a22913fbcd68fb86cb8
merge-base: 3763b33132c78ecb541180f05e1b1dd6ec6719e1
ahead/behind: 19/22
merge-tree: b0ef9f483fefce743323ab78b857b704dfaa5b13
live quick_check sample: still reports Freelist: freelist leaf count too big
live rows: pi_agent=1077, claude_code=2574, codex=5712, opencode=976, factory=66, messages=1055517
com.cass.index-watch: still absent from gui/501
com.cass.health-watchdog: loaded but not running, last exit code 2, runs=340
process scan: no cass worker processes matched
target free space: 171Gi
release candidate: /tmp/cass-release-target/release/cass sha256=423e2e4c2920ec74a38a5cb4af1f00de362a4a82e493d342b4891179f4955ada
dependency pin: still resolves fsqlite 0.1.3 from dirty local ../spec014-frankensqlite-fix
```

This keeps the same completion decision. The latest audit is recorded in
`evidence/continuation-audit-20260517T025454Z.md`.

Read-only route preflight on 2026-05-17T01:40:00Z:

```text
route-policy.md: present
installed cass status: exit 0, unhealthy, stale checkpoint incomplete, rebuild_active=false, watch_active=false, doctor_active=false
installed cass health: exit 1, unhealthy, stale checkpoint incomplete, recommended cass index --full
process scan: no cass index/doctor/watchdog/local test or release worker matched
read-only cass doctor --json: stalled at 04:37 and 11770512 KB RSS; stopped with SIGTERM, exit 143, stdout/stderr empty
```

This completes T6 as current pre-mutation evidence. It does not change the live
completion decision because the malformed live archive still must not receive
more writes.

Stale-index refresh stop consolidated on 2026-05-17T01:46:14Z:

```text
T7 refresh command: cass index --json --no-progress-events --data-dir /Users/dalecarman/Library/Application Support/com.coding-agent-search.coding-agent-search
refresh exit: 143 after SIGTERM
max recorded RSS: 30640864 KB
stdout/stderr: empty
paired status verification: unhealthy, stale, checkpoint.completed=false, checkpoint.db_matches=true
paired health verification: exit 1
fresh quick_check: still reports Freelist: freelist leaf count too big
fresh live rows: pi_agent=1077, claude_code=2574, codex=5712, opencode=976, factory=66, messages=1055517
fresh process scan: no non-probe cass index/doctor/watchdog worker matched
com.cass.index-watch: still absent from gui/501
```

This checks off the required single refresh attempt and stop-condition evidence,
but it does not change the live completion decision. T8-T15 and T16-T24 remain
blocked until the verified shadow archive can be promoted and watched under
explicit approval.

Conditional connector-scope audit on 2026-05-17T01:56:00Z:

```text
/Users/dalecarman/.local/bin/cass sources agents list --json
exit: 0
disabled_agents: []
total: 0
```

No `cass sources agents exclude` or `cass sources agents include` command was
used in this recovery. T10 is therefore closed as not triggered, without
changing the remaining live acceptance blockers.

T8 preselection refresh on 2026-05-17T01:58:00Z:

```text
canary-selection.json contains one selected identity each for claude_code, codex, and pi_agent
all three selected paths are present in their frozen manifests
all three selected source files exist locally
selected query strings are present in their source files
T8 remains unchecked because live watch-once, DB source_path, and lexical search proof have not run
```

T11 shadow reconciliation preflight on 2026-05-17T02:03:00Z:

```text
claude_code: manifest=2425, shadow_unique_source_paths=2551, matched=2413, missing=12, duplicate_source_path_groups=23, duplicate_nonnull_provenance_keys=0
codex: manifest=5868, shadow_unique_source_paths=5681, matched=5675, missing=193, duplicate_source_path_groups=32, duplicate_nonnull_provenance_keys=0
pi_agent: manifest=4174, shadow_unique_source_paths=2076, matched=2076, missing=2098, duplicate_source_path_groups=0, duplicate_nonnull_provenance_keys=0
pi_agent missing shape: all 2098 missing manifest paths are under --clawdbot-chip--
```

This is not live T11 completion. It is a useful warning that after promotion the
live reconciliation must decide whether `--clawdbot-chip--` belongs in priority
Pi Agent accounting, bonus ClawdBot/OpenClaw-family accounting, or path-specific
skip evidence.

Chipbot classification follow-up on 2026-05-17T02:12:37Z:

```text
symlink: /Users/dalecarman/.pi/agent/sessions/--clawdbot-chip-- -> /Users/dalecarman/.clawdbot/agents/main/sessions
file count: 2098 JSONL files
older evidence: spec 005 says dropping this symlink would be an unacceptable regression
current FAD pi_agent: ignores UUID-only filenames because session_files requires "_"
current FAD clawdbot: parses top-level role/content JSONL, not nested Pi-style message records
scratch chipbot index: exit 0, conversations=0, messages=0
scratch normal Pi control: exit 0, conversations=1, messages=6, by_agent pi_agent=1
follow-up bead/spec: coding_agent_session_search-2d37b / specs/017-chipbot-symlink-indexing/
```

This does not complete T11 or T12. It separates a real bonus/other-session
connector gap from the priority Pi Agent recovery proof that still must be
rerun against live after promotion.

## Approval-Readiness Evidence Refresh

Additional non-live checks recorded through 2026-05-16T23:42:54Z:

```text
pre-approval process refresh: no active cass index/search/health/doctor/watchdog or local debug/release cass worker matched beyond the ps/rg probe itself
durable dependency refresh: /Users/dalecarman/dev/spec014-frankensqlite-fix is on fix/fts5-vtab-snapshot-via-delta-journal with dirty pager/WAL fixes; CASS still uses a local ../spec014-frankensqlite-fix patch
promotion capacity: target volume has 175Gi free; shadow DB+index copy footprint is about 11.6G; df must be re-checked immediately before promotion
watcher plist/runtime readiness: com.cass.index-watch plist exists and points to cass index --watch; installed and release-candidate binaries both expose --watch/--watch-once/--watch-interval
synthetic Codex proof format: release candidate indexed a scratch synthetic Codex JSONL and lexical search returned 2 codex hits
synthetic Codex watch-once path: release candidate processed the same proof shape through index --watch-once and lexical search returned 2 codex hits
runbook shell syntax: extracted approval-gated shell blocks passed zsh -n
runbook tool availability: sqlite3, jq, launchctl, plutil, rg, shasum, date, mkdir, cp, mv, and ls are available; plist lints OK
artifact hygiene: git diff --check passed; trailing-space scan only matched preserved generated vmmap summary evidence
watcher log context: cass-index-watch.log has 184 OOM-related watcher entries and last timestamp 2026-05-15T23:27:13Z; cass-watchdog.log has 448 "Could not parse arguments" entries
path permissions: live data, shadow data, ~/.local/bin, LaunchAgents, and Logs paths are accessible with required read/write/search bits for approved promotion
restore shape: runbook now preserves failed promoted DB/index/binary artifacts with FAILED-SPEC016 suffixes and moves PRE-SPEC016 DB/index/binary backups back into place if approved promotion/install verification fails
health-watchdog command surface: installed and release-candidate CASS both return exit 2 "Could not parse arguments" for watchdog run --help; keep it as nonblocking follow-up unless it interferes with index-watch proof
health-watchdog follow-up: coding_agent_session_search-2gif2 / specs/018-health-watchdog-command-surface/ now tracks the regression from closed spec 007
implement gate backstop: gate.sh record implement refused to mint implement:complete:v1 because 19 unchecked tasks remain after T10 checkoff; no sentinel was written
spec 015 routing: evidence/spec015-routing.md records spec 015 as subordinate evidence, not completion
evidence hygiene: evidence/evidence-hygiene.md records refined credential scan, raw local-path telemetry handling, and commit-ready summary artifacts
```

These checks reduce approval-time unknowns, but they do not satisfy the live objective. None of them prove that live cass has been promoted, that `com.cass.index-watch` is loaded, or that a real watched session-root marker has become searchable.

Short operator approval packet added on 2026-05-17T03:02:00Z:

```text
specs/016-cass-recovery-ingestion/evidence/operator-approval-packet.md
```

This packet is the concise human-facing approval surface. It keeps the same
completion decision: live recovery remains blocked until the exact approval
phrase is provided and the detailed runbook is executed.

Read-only continuation audit rerun on 2026-05-17T03:09:46Z:

```text
git fetch upstream main: upstream/main remained 485ff1052b48e8d731a9ca9da03ba1d3dd170a82
branch: dac/main
HEAD: b807ef175dcdeeb48b912a22913fbcd68fb86cb8
merge-base: 3763b33132c78ecb541180f05e1b1dd6ec6719e1
ahead/behind: 19/22
merge-tree: b0ef9f483fefce743323ab78b857b704dfaa5b13
live quick_check sample: still reports Freelist: freelist leaf count too big
live rows: pi_agent=1077, claude_code=2574, codex=5712, opencode=976, factory=66, messages=1055517
com.cass.index-watch: still absent from gui/501
com.cass.health-watchdog: loaded but not running, last exit code 2, runs=341
process scan: no cass worker processes matched
target free space: 171Gi
release candidate: /tmp/cass-release-target/release/cass sha256=423e2e4c2920ec74a38a5cb4af1f00de362a4a82e493d342b4891179f4955ada, version=cass 0.4.7
```

This keeps the same completion decision. The latest audit is recorded in
`evidence/continuation-audit-20260517T030946Z.md`.

UBS policy decision artifact added on 2026-05-17T03:15:34Z:

```text
specs/019-ubs-warning-policy-closeout/policy-decision.md
decision: do not add hidden baselines, broad ignores, or workflow weakening
spec 016 impact: T20 remains unchecked unless final review accepts the warning-only inventory as outside the live recovery gate, or a separate UBS policy/wrapper/cleanup route is explicitly selected
```

Read-only continuation audit rerun on 2026-05-17T04:06:10Z:

```text
git fetch upstream main: upstream/main advanced to 5156af7ecbfe3aa757a838ebfd6444d55f647896
branch: dac/main
HEAD: b807ef175dcdeeb48b912a22913fbcd68fb86cb8
merge-base: 3763b33132c78ecb541180f05e1b1dd6ec6719e1
ahead/behind: 19/23
merge-tree: 95ec000ced664cc83a1d1f8fd8b4d54c7cd3330d
live quick_check sample: still reports Freelist: freelist leaf count too big
live rows: pi_agent=1077, claude_code=2574, codex=5712, opencode=976, factory=66, messages=1055517
com.cass.index-watch: still absent from gui/501
com.cass.health-watchdog: loaded but not running, last exit code 2, runs=347
installed cass watchdog run --help: exit 2, Could not parse arguments
release cass watchdog run --help: exit 0
process scan: no non-probe cass worker matched
target free space: 151Gi
release candidate: /tmp/cass-release-target/release/cass sha256=a5a139ca503f0f04f2b0baba06e20b16bd7003a7e7d78306cacf94d56eaca9c2, version=cass 0.4.7
```

This keeps the same completion decision. The latest audit is recorded in
`evidence/continuation-audit-20260517T040610Z.md`.

Read-only continuation audit rerun on 2026-05-17T04:15:33Z:

```text
git fetch upstream main: upstream/main remained 5156af7ecbfe3aa757a838ebfd6444d55f647896
branch: dac/main
HEAD: b807ef175dcdeeb48b912a22913fbcd68fb86cb8
merge-base: 3763b33132c78ecb541180f05e1b1dd6ec6719e1
ahead/behind: 19/23
merge-tree: 95ec000ced664cc83a1d1f8fd8b4d54c7cd3330d
live quick_check sample: still reports Freelist: freelist leaf count too big
live rows: pi_agent=1077, claude_code=2574, codex=5712, opencode=976, factory=66, messages=1055517
com.cass.index-watch: still absent from gui/501
com.cass.health-watchdog: loaded but not running, last exit code 2, runs=348
installed cass watchdog run --help: exit 2, Could not parse arguments
release cass watchdog run --help: exit 0
process scan: no non-probe cass worker matched
target free space: 151Gi
release candidate: /tmp/cass-release-target/release/cass sha256=a5a139ca503f0f04f2b0baba06e20b16bd7003a7e7d78306cacf94d56eaca9c2, version=cass 0.4.7
```

This keeps the same completion decision. The latest audit is recorded in
`evidence/continuation-audit-20260517T041533Z.md`.

Read-only continuation audit rerun on 2026-05-17T04:24:28Z:

```text
git fetch upstream main: upstream/main remained 5156af7ecbfe3aa757a838ebfd6444d55f647896
HEAD: b807ef175dcdeeb48b912a22913fbcd68fb86cb8
ahead/behind: 19/23
merge-tree: 95ec000ced664cc83a1d1f8fd8b4d54c7cd3330d
sqlite3 -readonly "$LIVE_DIR/agent_search.db": failed with SQLite code 14 against the current live DB
encoded mode=ro URI quick_check: still reports Freelist: freelist leaf count too big
live rows: pi_agent=1077, claude_code=2574, codex=5712, opencode=976, factory=66, messages=1055517
com.cass.index-watch: still absent from gui/501
com.cass.health-watchdog: loaded but not running, last exit code 2, runs=348
```

The approval runbook now uses the encoded SQLite `mode=ro` URI shape for live
integrity/restore probes because that is the read-only command shape proven
against the current live path. This keeps the same completion decision. The
latest audit is recorded in
`evidence/continuation-audit-20260517T042428Z.md`.

Read-only continuation audit rerun on 2026-05-17T04:45:42Z:

```text
git fetch upstream main: upstream/main remained 5156af7ecbfe3aa757a838ebfd6444d55f647896
branch: dac/main
HEAD: b807ef175dcdeeb48b912a22913fbcd68fb86cb8
merge-base: 3763b33132c78ecb541180f05e1b1dd6ec6719e1
ahead/behind: 19/23
upstream/main ancestor of HEAD: no
merge-tree: 95ec000ced664cc83a1d1f8fd8b4d54c7cd3330d
live encoded mode=ro quick_check: still reports Freelist freelist-leaf-count errors
live rows: pi_agent=1077, claude_code=2574, codex=5712, opencode=976, factory=66, messages=1055517
installed cass health: exit 1, unhealthy, index stale, checkpoint incomplete, watch_active=false
shadow integrity/counts: ok; pi_agent=2076, claude_code=2574, codex=5713, opencode=976, factory=66, messages=1238935
com.cass.index-watch: absent from gui/501
com.cass.health-watchdog: loaded but not running, last exit code 2, runs=351
installed cass watchdog run --help: exit 2, Could not parse arguments
release cass watchdog run --help: exit 0
release candidate: /tmp/cass-release-target/release/cass sha256=a5a139ca503f0f04f2b0baba06e20b16bd7003a7e7d78306cacf94d56eaca9c2, version=cass 0.4.7
GoalBuddy: check-goal-state active_task=T008; absolute npx prompt returns task T008; board API exposes .goal.activeTask=T008 and task T008 active
target free space: 150Gi
```

This keeps the same completion decision. The latest checklist-style audit is
recorded in `evidence/continuation-audit-20260517T044542Z.md`.

## Completion Decision

`full_outcome_complete: false`

The recovery is not complete. The most important user-visible state still has not changed: installed/live cass is using a malformed DB, Pi Agent is still under-indexed live, the index watcher is not loaded, upstream/branch finalization is unresolved, and the implement gate still refuses completion.

The next concrete action remains the operator-approved live promotion and branch/dependency resolution:

```text
I approve live CASS promotion, frankensqlite durable fix, and branch/commit resolution.
```
