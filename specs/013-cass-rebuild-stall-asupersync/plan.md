---
title: "cass full-rebuild stalls on raw-mirror archive copy — implementation plan"
date: 2026-05-13
bead: coding_agent_session_search-3vm6
---

<!-- plan:complete:v1 | harness: unknown | date: 2026-05-13T15:22:07Z -->



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

Observed result: ran `sed -n '9670,9690p' src/indexer/mod.rs` (bash command, exit 0). Captured stdout contained the literal source text `let canonical_only_full_rebuild = opts.force_rebuild && initial_canonical_sessions_before_salvage > 0;` and the in-source comment "skip the expensive filesystem rescan and go straight to rebuild_tantivy_from_db(). Plain --full continues to rescan as expected (preserving the #153 fix)." Cross-check `sed -n '10230,10240p' src/indexer/mod.rs` (bash, exit 0) confirmed the full-rescan path passes an unset / empty `since_ts` parameter, driving the connector scan loop to enumerate every source file regardless of mtime. Watcher log at `~/Library/Logs/cass-index-watch.log` was grepped for `selected_lexical_population_strategy strategy="deferred_authoritative_db_rebuild"`; the line was captured at timestamp 2026-05-13T10:58:49Z and corroborates the short-circuit firing when the watcher detects a populated DB. Probe outcome: `cass index --full` (without `--force-rebuild`) on the current non-empty DB will take the rescan path, not the short-circuit.

Decision impact: if this probe had failed (i.e., `cass index --full` ALSO short-circuited on a non-empty DB), the recovery directive in tasks.md Group A would be replaced — specifically tasks.md Group A T5 would need to direct fixing the canonical-short-circuit code before invoking the user-facing backfill command, and tasks.md Group D (the targeted fix) would be reordered to execute before any user-facing recovery in tasks.md Group A. Probe outcome instead forces tasks.md Group A T5 to name `cass index --full` (no `--force-rebuild`) as the recovery command and forces tasks.md Group A to be ordered before tasks.md Group D in the workflow.

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

