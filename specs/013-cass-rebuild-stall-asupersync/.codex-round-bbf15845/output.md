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