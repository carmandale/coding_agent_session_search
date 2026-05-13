**Findings**
1. The task list still has stale C4 wiring after the plist-mutation rewrite.  
T23 now owns the plist change, but T36 still says “re-load launchd agents per T26 sequence” and T26 no longer exists ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/013-cass-rebuild-stall-asupersync/tasks.md:85)). More importantly, there is no concrete task to verify the mutated plist shape with `plutil`, unload/load it, confirm launchd accepts `{SuccessfulExit=false}`, and prove the sentinel exit-0 path does not respawn. The plan text is now coherent, but the execution tasks still have a hole.

2. T35 still includes a forbidden deletion fallback.  
The task says an acceptable alternative is to “delete `watch_state.json` entirely” ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/013-cass-rebuild-stall-asupersync/tasks.md:83)). Project instructions explicitly forbid file deletion without written permission. This should be changed to preserve/rename/copy-aside only, or require an explicit approval checkpoint.

Everything else I rechecked is now aligned: T12 uses `--json`, Surface 7 adds a real `ingest-skipped.ndjson` rather than pretending raw-mirror has `skip_reason`, and Surface 3 no longer contradicts itself in the plan text.

**What I Verified**
I read the updated `plan.md` and `tasks.md`, rechecked the live plist (`KeepAlive => true`), and rechecked the source facts for watchdog gating and `--full` behavior. The remaining blockers are stale task mechanics, not the main architecture.

VERDICT: REVISE