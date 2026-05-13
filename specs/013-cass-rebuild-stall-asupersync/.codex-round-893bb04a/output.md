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