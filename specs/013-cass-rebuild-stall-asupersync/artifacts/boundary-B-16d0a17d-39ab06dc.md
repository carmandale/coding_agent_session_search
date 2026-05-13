---
boundary_timestamp: 2026-05-13T152158Z
phase: B
sha: 16d0a17d
plan_hash: 39ab06dc
plan_sha256_full: 39ab06dcf84fb0e17314ec4d89293948e6559b91031690f8ffcba1fd82d643e7
---

# SPEC-SNAPSHOT-BEGIN
---
title: "cass full-rebuild stalls on raw-mirror archive copy (asupersync worker deadlock)"
date: 2026-05-13
bead: coding_agent_session_search-3vm6
---

<!-- issue:complete:v1 | harness: unknown | date: 2026-05-13T14:17:15Z -->

## Source (verbatim)

> "I would like to be in sync with upstream. and has cass been grabbing all sessions? or has it stopped/failed?" — user, 2026-05-12

> "it should be my main is in sync with upstream, and then we try to merge to our latest branch [...]. the goal is to be in sync with upstream and running properly, capturing all sessions and cass working. that is the /goal" — user, 2026-05-12

> "the whole point is to have historical data. it sounds like you need to debug why it stalled. should we create an /issue for this?" — user, 2026-05-13

## Problem

After fresh-installing upstream cass v0.4.2 (commit `29c3672a` on `origin/main`, synced from `upstream/main`) on 2026-05-12, `cass index --full --force-rebuild` stalled partway through ingesting historical session data. The stalled process held the index-run lock and consumed 100% CPU for over 11 hours while making no forward progress.

Concrete state at point of stall:

- Process: `cass index --full --force-rebuild`, PID 21824, etime 10h+, 100% CPU, 8.3 GB RSS.
- All 16 `asupersync-worker-*` threads parked in `_pthread_cond_wait` / `__psynch_cvwait`.
- Main thread parked in `nanosleep` / `__semwait_signal`.
- `lsof` on the process showed no open session source files (no `.jsonl`, no `.vscdb`); only the binary itself and one in-progress raw-mirror manifest temp file.
- `raw-mirror/v1/blobs/` growth: 0 bytes in a 10-second sample window.
- `agent_search.db-wal` size frozen at 167.6 MB → 511.5 MB across the 11h window with multi-hour gaps of no writes.
- 0 new log lines emitted for 25+ minute stretches; only periodic `pricing coverage` summaries.

Completed before stall:

- `streaming_scan_complete` fired for 15 connectors (aider, amp, chatgpt, clawdbot, cline, copilot, copilot_cli, crush, cursor, factory, gemini, hermes, kimi, qwen, vibe).
- `streaming_ingest` events landed for amp (33 conv), cursor (242), factory (66), gemini (44), pi_agent (532).
- DB reached 1,648 conversations / 129,954 messages (out of an expected ~21,635 conversations based on the pre-restart DB).
- raw-mirror grew to 25 GB then froze.

Never started:

- `streaming_scan_complete` never fired for the four bulk connectors: claude_code, codex, openclaw, opencode. These hold the bulk of the historical corpus — by file count on disk:
  - claude_code: 2,561 `.jsonl` files (DB has 413 → 2,148 missing, 84%)
  - codex: 3,991 `.jsonl` files (DB has 84 → 3,907 missing, 98%)
  - openclaw: 11,018 `.jsonl` files (DB has 193 from one of seven agents → ~10,825 missing, 98%)
  - opencode: 19 files (DB has 79; appears complete or close to it)
- Net: ~17,000 historical sessions blocked from backfill by the stall.

After manual kill (`SIGTERM` → graceful), the canonical DB (1,648 conv) was durable. The watcher path (`cass index --watch`, loaded via launchd) restarted cleanly, rebuilt the Tantivy lexical index in ~3 hours, and now captures *new* sessions live — but it does not backfill historical files. The watcher initializes its per-`ConnectorKind` `since_ts` high-water marks to startup time when `watch_state.json` is fresh or empty, so files older than startup are out of scope.

This blocks the user's stated /goal: capture **all** sessions, including historical claude_code / codex / openclaw archives that represent the bulk of their agent activity from 2025-09-15 onward.

## Requirements

R1. Reach a root-cause diagnosis for the asupersync worker deadlock during `cass index --full`. Diagnosis must be concrete enough to answer: which queue / channel / mutex held the threads, which producer side failed to signal, and why the v0.3.7 stall watchdog did not emit a `stall_detected` event (or did, and was missed).

R2. Implement a fix or confirmed workaround that lets a single foreground run ingest the full historical corpus (~17,000 conversations across claude_code, codex, openclaw, opencode) to completion without manual intervention. "Completion" means: every source file on disk for those connectors is either ingested into the canonical DB or has a recorded structured failure reason.

R3. Preserve the partial work already on disk: do not require throwing away the current 1,648-conversation canonical DB or the 25 GB raw-mirror archive. The fix should pick up where the killed rebuild left off (via content-hash dedup), not restart from zero.

R4. Add or enable a stall-detection signal that fires within bounded time (≤120s default per upstream `CASS_INDEX_STALL_DETECT_SECS`) when forward progress halts mid-phase. Emission must include enough diagnostic snapshot (thread states, queue depths, current connector, current source path) for an operator to file a precise upstream bug if the underlying defect is upstream.

R5. Regression coverage: add a focused test that exercises whichever channel / dispatch path is at fault, demonstrating that a stuck producer or consumer is recoverable (or detectable) rather than producing the observed silent hang.

## Constraint

C1. Affected version is upstream `cass 0.4.2` at commit `29c3672a`. Local fork branch in scope: `dac/main` (tracks `origin/main` which equals `upstream/main`). Any fix should be pull-request-ready against upstream `Dicklesworthstone/coding_agent_session_search`, not a fork-only patch — the user's stated workflow keeps `main` in sync with upstream and isolates local changes on `dac/main`.

C2. The stall reproduces on the user's current data corpus (~21,635 conversations across ~10 connectors, ~30 GB on disk including Cursor `.vscdb` mirrors). Any reproduction harness or regression test must be runnable without that full corpus — synthesize a minimum-viable fixture that triggers the same dispatch pattern.

C3. The fix must not regress the streaming pipeline's success cases. The small connectors (amp, cursor, factory, gemini, pi_agent) ingested correctly on the same run that stalled; whatever changes are introduced must preserve that working behavior.

C4. macOS launchd is the production runtime for the watcher (`~/Library/LaunchAgents/com.cass.index-watch.plist`, `KeepAlive=true`). The fix must be safe under `KeepAlive` semantics — i.e., not introduce a respawn loop if the indexer exits abnormally after the fix.

C5. The raw-mirror archive-first behavior introduced in upstream v0.3.7 is intentional design (CHANGELOG: "preserve raw archive before any repair"). The fix must not disable raw-mirror as a side effect; if performance is part of the root cause, address it without removing the archive capability.

## Acceptance Criteria

A1. A documented root-cause writeup exists at `specs/013-cass-rebuild-stall-asupersync/root-cause.md` naming:
- The specific sync primitive that deadlocked (channel handle, condvar, semaphore — by file:line in `src/indexer/` or upstream crate).
- The producer / consumer pair involved and which side failed to wake the other.
- Why the existing v0.3.7 stall-detection watchdog did not catch this case (or, if it did, why the emitted event was not visible).

A2. `cass index --full --force-rebuild` against the user's full corpus (or an equivalent synthetic fixture covering the same dispatch pattern) completes end-to-end with zero manual intervention. End-state: DB conversation count ≥ source-file count for each connector minus a documented allowed-skip set (e.g., known-malformed sessions).

A3. After completion, `cass stats` shows non-zero per-connector counts for claude_code, codex, openclaw (all agents), opencode that match within ≤2% of the on-disk source file counts. Date range covers 2025-09-15 through current.

A4. `CASS_INDEX_STALL_DETECT_SECS=60` produces a structured `stall_detected` NDJSON event within 120s of a forced hang in a test fixture, and the event includes the diagnostic snapshot fields described in upstream v0.3.7 CHANGELOG (lexical rebuild checkpoint, Tantivy segment count, run-lock metadata).

A5. A regression test in `tests/` exercises the fixed dispatch path. The test fails on the current `29c3672a` baseline and passes after the fix.

A6. If the fix turns out to require an upstream contribution, an upstream PR or issue link is recorded in `specs/013-cass-rebuild-stall-asupersync/log.md` with the upstream commit / PR identifier, and the local fork carries the patch on `dac/main` only until the upstream merge lands.

## Out of Scope

- **Watcher live-capture rate or design.** The watcher already runs healthily after restart and captures new sessions. Steady-state watcher performance, scan-cycle tuning, and `watch_state.json` schema changes are not in scope here.
- **Migrating off frankensqlite.** Storage layer choice is upstream's decision. Any fix must work within frankensqlite.
- **Cursor `.vscdb` archive size.** The 25 GB raw-mirror is dominated by Cursor's SQLite workspace databases. Reducing that footprint is a separate concern.
- **Semantic / HNSW index.** Semantic indexing is gated behind `cass models install` and was never enabled on this run.
- **TUI, search query, or doctor surface changes.** This is an indexer-side stall; UI / query / repair surfaces are out of scope.
- **Migrating the corrupted local `feat/007-watchdog-subcommand` branch state.** That cleanup is a separate housekeeping task tracked elsewhere.

## Selected Shape

**Direct root-cause fix in the asupersync streaming worker dispatch path in `src/indexer/` (`cass 0.4.2` / upstream main), with focused regression coverage.**

Approach is determined: this is a clear bug fix with a concrete reproduction and a narrow code surface. No shaping iteration needed because the problem statement and acceptance criteria are well-defined; the unknown is *which* synchronization primitive deadlocked, not *what* the desired end-state looks like.

The investigation path is:

1. Re-run `cass index --full --force-rebuild` with `CASS_INDEX_STALL_DETECT_SECS=60` and `RUST_LOG=cass=debug,coding_agent_search::indexer=trace` to capture both the structured stall event and high-resolution traces of the asupersync worker dispatch around the freeze.
2. Attach a debugger (lldb) at the deadlock point and capture exact stack frames in symbolic form (the `sample` output during the original stall had unresolved addresses because the binary was stripped); confirm which mutex / channel handle is held by which thread.
3. Cross-reference with upstream issue #196 thread on `Dicklesworthstone/coding_agent_session_search` — the v0.3.7 fix targeted a different deadlock (zero-writer init in `FrankenConnectionManager`); the v0.4.2 stall is in a downstream stage and probably warrants a sibling fix or a missed handoff between the producer and the asupersync workers.
4. Patch the identified primitive (most likely candidates: a bounded channel without a wakeup on backpressure, or a condvar signaled inside a critical section the receiver also holds). Add the regression test from A5.
5. If the patch belongs upstream, open a PR against `Dicklesworthstone/coding_agent_session_search`; cherry-pick onto `dac/main` until it merges.

The fallback if root-cause is not tractable within reasonable effort:

- Use `CASS_STREAMING_INDEX=0` to force the legacy batch indexer for the historical backfill, then re-enable streaming for steady-state. Document the toggle in `~/Library/LaunchAgents/com.cass.index-watch.plist` env vars. This restores the /goal (full backfill) without the root-cause fix, at the cost of slower steady-state ingest. Treat as workaround, not solution; the root-cause investigation continues separately as a P2 follow-up.

# SPEC-SNAPSHOT-END

# PLAN-SNAPSHOT-BEGIN

---
title: "cass full-rebuild stalls on raw-mirror archive copy — implementation plan"
date: 2026-05-13
bead: coding_agent_session_search-3vm6
---

## Overview

The work has three converging tracks: **unblock the user's backfill now** using a corrected operational invocation; **diagnose and fix the streaming-pipeline deadlock** using new liveness instrumentation as the evidence base; and **harden the stall watchdog** so future occurrences fail loud instead of silent. The fix lands on the local `dac/main` branch first (to unblock the user), then propagates upstream as a PR to `Dicklesworthstone/coding_agent_session_search`.

## Shape Comparison (R0)

Three plausible shapes were compared on net-complexity:

| Shape | Description | Net complexity | Why not |
|---|---|---|---|
| A. Workaround only | Set `CASS_STREAMING_INDEX=0` via launchd plist env vars, accept batch indexer permanently | Lowest | Fails R1 (no root-cause writeup), R5 (no regression test), C5 ambiguous (batch path also runs raw-mirror but may behave differently under contention). Leaves the bug present for any other operator. |
| B. Fix-local only | Patch deadlock on `dac/main`, regression-test, document as fork carry | Medium | Creates fork divergence; if upstream ships a structurally different fix in `#218` batch, reconciliation cost is high. Fails C1 (PR-ready against upstream) over time. |
| C. **Fix-local + upstream-PR** | Patch on `dac/main` to unblock user immediately, simultaneously prepare PR upstream so the patch can land where the bug lives | Higher initial, lower long-term | **Selected.** Satisfies all requirements; aligns with user's stated workflow ("main in sync with upstream, local work on a branch"). Initial overhead is the PR-ready packaging, which is required by C1 anyway. |

The implementation work is identical between B and C until the final upstreaming step; selecting C means we author the regression test, code-style, and commit message to upstream conventions from the outset.

## Plan Sanity Evidence

Objective: complete historical backfill of cass session corpus (~17,000 missing claude_code/codex/openclaw/opencode conversations) and land a PR-ready fix for the streaming-pipeline deadlock so future full-rebuild runs against this corpus no longer silently hang, in upstream `Dicklesworthstone/coding_agent_session_search` and on local `dac/main`.

Riskiest assumption: the recovery command this plan directs the user to invoke (`cass index --full` without `--force-rebuild`) will actually re-scan the connector source directories and ingest the ~17,000 missing sessions, rather than silently short-circuiting to a Tantivy-only rebuild of the partial DB and leaving the missing sessions missing. If false, the entire Group A recovery sequence is no-op against the user's primary symptom, and the plan would need to redirect Group A to instead repair the `--force-rebuild` semantics before any user-facing invocation.

Smallest probe: read `/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:9675-9680` to verify the `canonical_only_full_rebuild` short-circuit predicate's exact gating, and cross-check against `src/indexer/mod.rs:10234` for the full-rescan path's `since_ts` setting; corroborate against the live `selected_lexical_population_strategy="deferred_authoritative_db_rebuild"` log line emitted by the running watcher at 10:58:49.

Observed result: ran `sed -n '9670,9690p' src/indexer/mod.rs` — output contained literal `let canonical_only_full_rebuild = opts.force_rebuild && initial_canonical_sessions_before_salvage > 0;` with the comment "skip the expensive filesystem rescan and go straight to rebuild_tantivy_from_db(). Plain --full continues to rescan as expected (preserving the #153 fix)." The boolean conjunction shows the short-circuit fires only when BOTH `force_rebuild` is true AND DB is already populated; the comment explicitly carves out plain `--full` as the rescan path. Cross-check at `src/indexer/mod.rs:10234` shows the full-rescan path uses `since_ts = None` (per code-reading), driving the connector scan loop to enumerate all source files regardless of mtime. Watcher log corroborates: `selected_lexical_population_strategy strategy="deferred_authoritative_db_rebuild"` fires exactly when the short-circuit condition holds. Probe pass: `cass index --full` (without `--force-rebuild`) on the current non-empty DB will take the rescan path, not the short-circuit.

Decision impact: if this probe had failed (i.e., `cass index --full` ALSO short-circuited on a non-empty DB), Group A T2-T4's recovery procedure would be replaced with a tasks.md `## Group A` entry directing fix-of-canonical-short-circuit code (Surface 1 amendment to plan.md Architecture) BEFORE any user-facing invocation, and Group A T2's `cass index --full` command would be replaced with the post-fix `cass index --full --force-rebuild` once the short-circuit is gone. Probe outcome forces the operational recovery invocation to `cass index --full` (no `--force-rebuild`) and forces Group A to be ordered before Group D (the targeted fix), not after.

## Architecture

The plan touches six code/operational surfaces. Their ordering is captured in the task groups below; the architectural decisions are surface-by-surface.

### Surface 1 — Recovery operational baseline

Recovery is operational, not code. The user's backfill is unblocked by:

1. Unloading the launchd watcher and watchdog so the foreground rebuild can acquire the index-run lock.
2. Invoking `cass index --full` (without `--force-rebuild`) — keeps the rescan path active per `src/indexer/mod.rs:9972`.
3. Monitoring with `CASS_INDEX_STALL_DETECT_SECS=60` set in the environment, so a hang produces a structured event rather than silent CPU burn.
4. After backfill completes (or hangs again), re-loading the launchd agents.

This work is *prerequisite* for diagnosis if we choose to reproduce the deadlock; it is *the primary deliverable* if we choose to fix-then-verify-on-corpus.

### Surface 2 — Liveness instrumentation (new code)

Add a heartbeat emitter to `src/indexer/mod.rs` that, when `RUST_LOG=cass::indexer::liveness=info` is set, emits an INFO log every 5 seconds carrying:

- **Logical pipeline layer:**
  - active producer count + per-producer current connector + current source path
  - channel `len()` / `capacity()`
  - `StreamingByteLimiter` `bytes_in_flight` / `max_bytes_in_flight` + outstanding `acquire_with_wait` callers (counter)
  - consumer phase (idle / draining / committing / flushing)
  - `StreamingBatchSender::flush()` in-flight flag
  - flat-combine state at `src/indexer/mod.rs:8624` (combined-batch-pending count)
  - raw-mirror `MANIFEST_UPDATE_LOCK` held flag + current manifest path under update

- **OS thread layer** (R4 requirement):
  - per-spawned-thread name + park-or-running flag, tracked via a per-thread atomic heartbeat counter that the thread bumps every N operations + a coordinator that compares the counters across emission ticks

The heartbeat is opt-in (gated by RUST_LOG target) so it does not affect normal-operation log volume. It is the evidence source for Surface 4 / 5's diagnosis output.

### Surface 3 — Watchdog (R4 + C4 reconciliation)

The existing `stall_detected` event at `src/lib.rs:72639` carries lexical checkpoint, Tantivy segment count, run-lock metadata. R4 requires extending the payload with the Surface-2 fields. Three sub-deliveries on this surface:

- **Payload extension** — embed the same fields the heartbeat emits, taken at the moment the watchdog fires. This is additive; existing consumers are not broken.
- **Emission gating fix** — the event currently fires only in structured-output / progress-events mode (`src/lib.rs:72250, 72531`). Remove the gating so the event always emits to the structured log channel; consumers who want JSON still set `--json`.
- **Escalation posture (C4-safe)** — given `KeepAlive=true` boolean (not dictionary), the safest reconciliation is: indexer writes a sentinel file at `<data_dir>/stall-detected.json` carrying the full event payload AND emits the structured event, but does NOT exit on its own. A small wrapper script invoked by launchd (or the existing health-watchdog plist) checks the sentinel before re-exec; if present, it backs off the relaunch and surfaces a noticeable failure. This keeps the indexer's exit path unchanged, avoids a `KeepAlive` respawn loop, and gives R4 the loud signal it asks for.

### Surface 4 — Targeted fix (one of six candidates)

The fix shape is decided by the Surface 2 / 3 diagnostic output. The candidate list, by file:line:

1. `src/raw_mirror.rs:22` `MANIFEST_UPDATE_LOCK` global mutex — split into per-manifest-id sharded locks or async writer thread.
2. `src/indexer/mod.rs:8000-8105` `StreamingByteLimiter` — audit predicate-protect contract; sibling to upstream `470451ea`.
3. `src/indexer/mod.rs:8864` `t_index.commit()` — move to dedicated thread.
4. `src/indexer/mod.rs:8293` `StreamingBatchSender::flush()` — reservation-before-send amplification; restructure reserve / send pair to be deadlock-free.
5. `src/indexer/mod.rs:8624` flat-combine — release reservations per-message instead of per-combined-batch.
6. `src/indexer/mod.rs:10344` staged lexical merge after streaming — same shape as upstream `544402b9` deadlock fix; audit for hard-zero clamp.

Each candidate has a known fix shape that has succeeded in adjacent code (cited upstream commits). The targeted fix is committed only after the diagnostic output names which candidate is the actual bug.

### Surface 5 — Fallback path

If diagnosis converges on an upstream-asupersync defect (not under our control) or the targeted fix is rejected by upstream review for design reasons, the fallback is `CASS_STREAMING_INDEX=0` set via the launchd plist env vars. The batch indexer code path (verified at `src/indexer/mod.rs:8342-8350`) shares `attach_raw_mirror_capture()` so C5 is preserved; it serializes ingestion (slower steady-state) but does not have the streaming pipeline's deadlock surface. This is a workaround line in the plan, not a fix.

### Surface 6 — Regression test scope

The fixture exercises the actual deadlocked primitive — not panic propagation (already covered at `src/indexer/mod.rs:27750`), not byte-limiter lost-wakeup (covered by `streaming_byte_limiter_update_does_not_lose_wakeup_under_repeated_shrink_grow` from `470451ea`). The test shape:

- Constructs a bounded channel + byte limiter + raw-mirror manifest mutex
- Drives load that reproduces the named primitive's blocking shape (which primitive depends on Surface 4 selection)
- Asserts either bounded recovery (the fix path) or a bounded `stall_detected` event with the new R4 fields (the watchdog path, used when the fix path is not yet implemented)

The fixture must NOT require the user's full corpus (C2); a synthetic generator inside the test fakes the producer/consumer pressure shape.

## Risk analysis

| Risk | Mitigation |
|---|---|
| Surface 2 instrumentation perturbs the timing of the deadlock (heisenbug) | Heartbeat is opt-in (RUST_LOG target); production runs without it. Per-thread atomic counters are cheap; emission cadence is 5 s, far below the per-message rate. |
| The targeted fix passes our regression but does not actually unblock the user's corpus run | Recovery (Surface 1) re-runs against the user's full corpus as part of acceptance — we don't claim victory until A2 holds on the real data. |
| Upstream maintainer rejects the PR for design reasons (e.g., wants different fix shape) | The local patch stays on `dac/main` until upstream's `#218` batch lands or until our PR is accepted; C1 is satisfied "PR-ready" not "merged". |
| The deadlock is in `asupersync` crate (not cass), invalidating Surface 4 entirely | Surface 5 fallback (`CASS_STREAMING_INDEX=0`) bypasses the streaming pipeline; recovery is still achievable. R1 root-cause writeup names the asupersync defect; A6 records the upstream-of-upstream PR. |
| Test fixture cannot reproduce the deadlock without the full corpus | Use `loom` (Rust concurrency model checker) on the suspected primitive; if `loom` can't reproduce, document the limitation in `root-cause.md` and rely on the live watchdog event as evidence. |
| Watcher unloaded during recovery → live capture gap | Recovery window is documented in `recovery.md`. The `cass index --full` rescan path updates DB `last_scan_ts` (`src/indexer/mod.rs:8870`) but does NOT update `watch_state.json` (which is touched only by the watch path at `src/indexer/mod.rs:16013`). Group H T35 explicitly reconciles `watch_state.json` against the post-backfill `last_scan_ts` before re-loading the watcher, so the watcher resumes from the correct high-water mark on its next tick. |

## Traceability — requirement to surface

| Req | Surface(s) |
|---|---|
| R1 root-cause | 2, 3, 4 |
| R2 fix or workaround | 1, 4, 5 |
| R3 preserve partial work | 1 |
| R4 stall signal with new fields | 2, 3 |
| R5 regression test | 6 |
| C1 PR-ready upstream | 4 (commit / test style) |
| C2 test without full corpus | 6 |
| C3 success cases preserved | 4, 6 (test must include success-path assertions) |
| C4 launchd-safe | 3 |
| C5 raw-mirror preserved | 4 (any fix must keep `attach_raw_mirror_capture` semantics), 5 |
| A2 invocation amendment | **explicit in this plan; spec must be updated to name `cass index --full`** |
| A6 upstream link | task group G |

---


# PLAN-SNAPSHOT-END
