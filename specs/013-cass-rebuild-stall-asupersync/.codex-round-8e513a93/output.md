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