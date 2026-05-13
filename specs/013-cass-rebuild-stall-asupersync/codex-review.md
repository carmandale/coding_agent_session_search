<!-- codex-review:approved:v1 | harness: codex/gpt-5.3-codex | date: 2026-05-13T15:59:20Z | rounds: 5 -->

# /codex-review transcript — specs/013-cass-rebuild-stall-asupersync/

Model: gpt-5.3-codex (codex-round-exec supervised)
Rounds: 5 (Round 5 → VERDICT: APPROVED)


## Round 1 — review id 8e513a93

**Findings**
1. The recovery command will not emit `stall_detected` as written.  
T5 runs `CASS_INDEX_STALL_DETECT_SECS=60 cass index --full` without `--json` or `--robot` ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/013-cass-rebuild-stall-asupersync/tasks.md:17)). Current code only arms the stall watchdog inside `emit_progress_events`, which requires structured output (`structured_output && !no_progress_events`) ([src/lib.rs](/Users/dalecarman/dev/coding_agent_session_search/src/lib.rs:72250), [src/lib.rs](/Users/dalecarman/dev/coding_agent_session_search/src/lib.rs:72531)). So Group A can silently hang again before Group E fixes the gating. Fix: either run the recovery as structured output now, or move T20-T21 before the first corpus run.

2. The plan does not currently satisfy the spec it is being reviewed against.  
Spec A2 still requires `cass index --full --force-rebuild` ([spec.md](/Users/dalecarman/dev/coding_agent_session_search/specs/013-cass-rebuild-stall-asupersync/spec.md:82)); the plan intentionally changes the recovery invocation to plain `cass index --full` ([plan.md](/Users/dalecarman/dev/coding_agent_session_search/specs/013-cass-rebuild-stall-asupersync/plan.md:31), [plan.md](/Users/dalecarman/dev/coding_agent_session_search/specs/013-cass-rebuild-stall-asupersync/plan.md:47)). Source supports the correction: `--force-rebuild` with existing canonical rows activates `canonical_only_full_rebuild` ([src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:9675)), while plain `--full` sets `since_ts = None` and rescans ([src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:10234)). But the review target is current spec + plan, and T1 says “amend the spec later” ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/013-cass-rebuild-stall-asupersync/tasks.md:13)). That is a scope change, not compliance.

3. A2’s “every source file ingested or structured failure reason” is underplanned.  
T19 only checks `cass stats` is within 98% ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/013-cass-rebuild-stall-asupersync/tasks.md:46)). That does not prove every source file has either a canonical conversation or a structured skip/failure. The plan needs a before/after source-file inventory and a per-file reconciliation ledger for claude_code/codex/openclaw/opencode, including allowed skips.

4. The plan weakens R4’s “thread states” instead of meeting it.  
R4 asks for thread states and queue depths ([spec.md](/Users/dalecarman/dev/coding_agent_session_search/specs/013-cass-rebuild-stall-asupersync/spec.md:59)). T8 amends that to heartbeat counters plus optional lldb ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/013-cass-rebuild-stall-asupersync/tasks.md:23)). Heartbeat deltas are not actual OS states; a thread copying/hashing a huge file can look “parked.” If the spec wants actual asupersync worker evidence, lldb/sample capture needs to be required for diagnosis runs, not optional.

5. The plan introduces production launchd wrapper machinery that is not required by the spec.  
C4 requires no respawn loop ([spec.md](/Users/dalecarman/dev/coding_agent_session_search/specs/013-cass-rebuild-stall-asupersync/spec.md:71)); it does not require a new wrapper, sentinel lifecycle, plist rewrite, wrapper tests, and uninstall script ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/013-cass-rebuild-stall-asupersync/tasks.md:53)). This expands a focused indexer stall fix into production daemon redesign. Safer: first make `stall_detected` always emit, write the sentinel if truly needed, and leave launchd wiring out unless the corpus run proves it is necessary.

**Adversarial Gate**
Riskiest assumptions I checked:

1. Plain `cass index --full` rescans: verified yes. See `canonical_only_full_rebuild` gated on `opts.force_rebuild` at [src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:9679), and `since_ts = None` for `opts.full` at [src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:10237).

2. `CASS_INDEX_STALL_DETECT_SECS=60` alone produces a stall event: verified false. Event loop only exists in the structured progress branch at [src/lib.rs](/Users/dalecarman/dev/coding_agent_session_search/src/lib.rs:72531).

3. The fallback `CASS_STREAMING_INDEX=0` preserves raw mirror: verified mostly yes. Batch mode is selected by env var at [src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:8342), and batch scan calls `attach_raw_mirror_capture` at [src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:9307).

A skeptical senior engineer’s first objection: “Why are we adding launchd wrapper infrastructure before proving the deadlock primitive and before fixing the existing watchdog emission bug?”

What production still needs: pinned binary/path verification for every operational command, per-file reconciliation reports, privacy-safe diagnostic export by default, and a rollback plan that does not delete artifacts.

Scope drift: the plan changes A2 from `--full --force-rebuild` to `--full`, weakens R4 thread-state semantics, and adds wrapper/plist/uninstaller work beyond the indexer-side stall spec.

**What I Verified**
I read the current `spec.md`, `plan.md`, `tasks.md`, source paths in `src/indexer/mod.rs`, `src/lib.rs`, `src/raw_mirror.rs`, `src/storage/sqlite.rs`, and existing historical recovery scripts. I checked the exact short-circuit, scan timestamp path, streaming channel setup, byte limiter, batch sender flush, flat-combine release, raw-mirror lock, watchdog gating, and watch_state storage.

VERDICT: REVISE

## Round 2 — review id bbf15845

**Findings**
1. The revised C4 design creates the exact respawn loop it is trying to avoid.  
You changed from wrapper/backoff to “indexer startup checks sentinel; if present, logs and exits cleanly.” But the live plist has `KeepAlive => true` and runs `/Users/dalecarman/.local/bin/cass index --watch`. With boolean `KeepAlive`, a clean immediate exit still gets relaunched, then exits again, repeatedly. I verified the live plist with `plutil -p /Users/dalecarman/Library/LaunchAgents/com.cass.index-watch.plist`.  
Action: either keep the process alive in a sentinel backoff loop, change launchd semantics explicitly, or restore the wrapper/backoff approach. “Exit cleanly” is not C4-safe.

2. Surface 1 still overstates the current recovery command.  
The revised plan says `CASS_INDEX_STALL_DETECT_SECS=60` makes the foreground recovery fail loud, then later says the interim workaround is `--json` until the gating fix lands. Current source proves the env var alone is not enough: `emit_progress_events = structured_output && ...` at [src/lib.rs](/Users/dalecarman/dev/coding_agent_session_search/src/lib.rs:72261), and the watchdog branch only runs under `emit_progress_events` at [src/lib.rs](/Users/dalecarman/dev/coding_agent_session_search/src/lib.rs:72531).  
Action: make Surface 1 explicitly use `cass index --full --json` before T21, or move the gating fix before the first corpus run.

3. The spec’s “Selected Shape” still contains stale `--full --force-rebuild` investigation text.  
A2 is now correctly amended to plain `cass index --full`, but the investigation path still says to rerun `cass index --full --force-rebuild`. Source confirms that on a non-empty DB, `--force-rebuild` triggers the canonical-only short-circuit ([src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:9675)), while plain `--full` gives `since_ts = None` ([src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:10237)).  
Action: update that stale investigation step or narrowly qualify it as “only against a synthetic/empty baseline reproducer.”

4. A2’s per-source completion proof is still missing from the plan.  
The plan still relies on stats/count matching, but R2/A2 require every source file to be ingested or have a structured failure reason. A3’s ≤2% aggregate check is not a substitute for a file-level ledger.  
Action: add an inventory/reconciliation step: enumerate source files for claude_code/codex/openclaw/opencode before the run, then produce `source_path -> ingested conversation id(s) | structured skip reason`.

**What I Verified**
I rechecked the live launchd plist, the current watchdog gating in `src/lib.rs`, the `--force-rebuild` short-circuit, the plain `--full` scan path, and the producer completion path in `src/indexer/mod.rs`. The `--full` correction is solid; the new sentinel startup behavior is not.

VERDICT: REVISE

## Round 3 — review id bc55e985

**Findings**
1. The C4 section contradicts itself.  
Surface 3 says “plist rewrite ... deferred,” then immediately makes plist mutation the C4 mechanism: `KeepAlive=true` → `{SuccessfulExit=false}`. The task file has the same contradiction: Group E says plist rewrites are deferred, then T23 changes the plist ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/013-cass-rebuild-stall-asupersync/tasks.md:51), [tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/013-cass-rebuild-stall-asupersync/tasks.md:56)). Decide one: either plist mutation is in scope and must be tested/reverted, or it is out of scope and C4 needs another mechanism.

2. T12 still runs a diagnostic corpus command without `--json` before the gating fix lands.  
T5 is fixed, but T12 says `CASS_INDEX_STALL_DETECT_SECS=60 cass index --full` ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/013-cass-rebuild-stall-asupersync/tasks.md:30)). Group C runs before Group E, so current source still requires structured output: `emit_progress_events = structured_output && ...` at [src/lib.rs](/Users/dalecarman/dev/coding_agent_session_search/src/lib.rs:72261). Make T12 match the revised investigation path: `cass index --full --json`.

3. The new per-source ledger depends on a `skip_reason` surface that does not exist for raw-mirror/indexer files.  
T19a says missing files can be accepted if the raw-mirror manifest has `skip_reason` ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/013-cass-rebuild-stall-asupersync/tasks.md:47)). But `RawMirrorManifestFile` has provider/path/blob/db_links fields and no skip reason ([src/raw_mirror.rs](/Users/dalecarman/dev/coding_agent_session_search/src/raw_mirror.rs:94)). The `skip_reason` hits I found are doctor/action surfaces, not source-ingest skips. Add a real durable skip ledger/table/NDJSON emitted by the indexer, or classify missing files only from actual existing evidence.

**What I Verified**
I rechecked the live launchd plist (`KeepAlive => true`), the current watchdog gating in `src/lib.rs`, the updated `tasks.md`, the raw-mirror manifest schema, and searched for `skip_reason` across `src`, `tests`, and this spec. The `--full` correction and file-level reconciliation idea are good; the remaining mechanics are not yet internally consistent.

VERDICT: REVISE

## Round 4 — review id 7f82f066

**Findings**
1. The task list still has stale C4 wiring after the plist-mutation rewrite.  
T23 now owns the plist change, but T36 still says “re-load launchd agents per T26 sequence” and T26 no longer exists ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/013-cass-rebuild-stall-asupersync/tasks.md:85)). More importantly, there is no concrete task to verify the mutated plist shape with `plutil`, unload/load it, confirm launchd accepts `{SuccessfulExit=false}`, and prove the sentinel exit-0 path does not respawn. The plan text is now coherent, but the execution tasks still have a hole.

2. T35 still includes a forbidden deletion fallback.  
The task says an acceptable alternative is to “delete `watch_state.json` entirely” ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/013-cass-rebuild-stall-asupersync/tasks.md:83)). Project instructions explicitly forbid file deletion without written permission. This should be changed to preserve/rename/copy-aside only, or require an explicit approval checkpoint.

Everything else I rechecked is now aligned: T12 uses `--json`, Surface 7 adds a real `ingest-skipped.ndjson` rather than pretending raw-mirror has `skip_reason`, and Surface 3 no longer contradicts itself in the plan text.

**What I Verified**
I read the updated `plan.md` and `tasks.md`, rechecked the live plist (`KeepAlive => true`), and rechecked the source facts for watchdog gating and `--full` behavior. The remaining blockers are stale task mechanics, not the main architecture.

VERDICT: REVISE

## Round 5 — review id 893bb04a

**No Blocking Findings**

The revised plan now matches the amended spec closely enough to implement. The previous blockers are resolved:

- Recovery/diagnostic invocation now uses `cass index --full --json`, matching the source reality that `--full --force-rebuild` short-circuits on non-empty canonical DBs at [src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:9679), while plain `--full` uses `since_ts = None` and rescans at [src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:10237).
- The watchdog gating issue is explicitly handled. Current code only emits progress events when structured output is enabled at [src/lib.rs](/Users/dalecarman/dev/coding_agent_session_search/src/lib.rs:72261), and T12 now uses `--json` until the gating fix lands in [tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/013-cass-rebuild-stall-asupersync/tasks.md:30).
- C4 is now coherent: plist mutation is in scope in [plan.md](/Users/dalecarman/dev/coding_agent_session_search/specs/013-cass-rebuild-stall-asupersync/plan.md:74), T23 changes `KeepAlive` to the dictionary form in [tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/013-cass-rebuild-stall-asupersync/tasks.md:61), and T36/T36a include concrete launchd verification in [tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/013-cass-rebuild-stall-asupersync/tasks.md:86).
- A2’s “every file ingested or structured skip” requirement is now covered by the new `ingest-skipped.ndjson` ledger in [plan.md](/Users/dalecarman/dev/coding_agent_session_search/specs/013-cass-rebuild-stall-asupersync/plan.md:117) and T19a in [tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/013-cass-rebuild-stall-asupersync/tasks.md:48). This correctly avoids relying on raw-mirror manifest schema, which lacks skip reasons at [src/raw_mirror.rs](/Users/dalecarman/dev/coding_agent_session_search/src/raw_mirror.rs:94).
- The dangerous watch-state cleanup fallback was fixed: T35 now says move aside, never delete, in [tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/013-cass-rebuild-stall-asupersync/tasks.md:80).

**Riskiest Assumptions Checked**

1. Plain `cass index --full` really rescans: verified against [src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:10237).  
2. Existing watchdog needs `--json` before the gating fix: verified against [src/lib.rs](/Users/dalecarman/dev/coding_agent_session_search/src/lib.rs:72261).  
3. Launchd loop prevention needs plist semantics, not just indexer exit behavior: verified live plist currently has boolean `KeepAlive=true` at [/Users/dalecarman/Library/LaunchAgents/com.cass.index-watch.plist](/Users/dalecarman/Library/LaunchAgents/com.cass.index-watch.plist:1), and the plan now mutates that explicitly.

A skeptical senior engineer’s first objection would be: “Are we proving file-level completion, or just getting lucky with aggregate counts?” Surface 7/T19a now answers that with a per-source ledger and zero silent-loss gate.

What this still does not address, appropriately, is long-term productization of stall management: no permanent wrapper, no full uninstall command, no broader watcher redesign. That differs from production-hardening scope, but it is now deliberately outside this spec and no longer conflicts with C4.

VERDICT: APPROVED
