**Findings**

1. The non-pi “unchanged” guarantee is not fully protected. The plan’s pseudocode calls `discover_source_files(&ctx)` before checking `kind.slug() == "pi_agent"` ([plan.md](/Users/dalecarman/dev/coding_agent_session_search/specs/015-watch-once-streaming-scan/plan.md:76), [plan.md](/Users/dalecarman/dev/coding_agent_session_search/specs/015-watch-once-streaming-scan/plan.md:87)). Spec requirement #5 says non-pi watch-once must be same or better ([spec.md](/Users/dalecarman/dev/coding_agent_session_search/specs/015-watch-once-streaming-scan/spec.md:42)). If claude/codex/opencode do a full discovery and then still bulk-scan, this can double traversal cost. Fix: short-circuit on `!explicit_watch_once || kind != PiAgent` before discovery, and test that non-pi does not enter discovery/streaming glue.

2. The buffer can carry conversations across scan batches, so “persist a chunk, drop it, repeat” is not guaranteed. The plan scans a file batch, pushes conversations into a buffer, and only flushes when buffer limits fire or at final flush ([plan.md](/Users/dalecarman/dev/coding_agent_session_search/specs/015-watch-once-streaming-scan/plan.md:109), [plan.md](/Users/dalecarman/dev/coding_agent_session_search/specs/015-watch-once-streaming-scan/plan.md:135)). Requirement #1 says each chunk’s working set must be droppable after persist ([spec.md](/Users/dalecarman/dev/coding_agent_session_search/specs/015-watch-once-streaming-scan/spec.md:38)). Fix: flush at the end of every scan batch, or explicitly redefine acceptance #2 as “current scan batch + bounded carry buffer” and measure that.

3. Acceptance-path root derivation is under-specified. FAD detection currently reports the Pi root as `~/.pi/agent/sessions` ([lib.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/lib.rs:1037)), and FAD has a regression for `ScanContext` whose data dir is the sessions directory itself ([pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:1609)). The plan says original root is `<home>/sessions` where `<home>` is “typically `~/.pi/agent`” ([plan.md](/Users/dalecarman/dev/coding_agent_session_search/specs/015-watch-once-streaming-scan/plan.md:197)). If implemented as `root.path.join("sessions")`, the exact acceptance command `--watch-once ~/.pi/agent/sessions` breaks. Fix: derive with FAD-equivalent logic: if `root/sessions` exists use that, else use `root`; test both `~/.pi/agent` and `~/.pi/agent/sessions`.

4. The JSON receipt location/schema is not nailed down. The plan requires new receipt fields ([plan.md](/Users/dalecarman/dev/coding_agent_session_search/specs/015-watch-once-streaming-scan/plan.md:227)), but the current index JSON payload is assembled in `src/lib.rs` ([lib.rs](/Users/dalecarman/dev/coding_agent_session_search/src/lib.rs:74158)) and the response schema currently does not list those top-level fields ([lib.rs](/Users/dalecarman/dev/coding_agent_session_search/src/lib.rs:70961)). Fix: specify `watch_once_receipt` top-level vs inside `indexing_stats`; if top-level, update response schema/goldens/docs.

5. Skipped-file handling does not cover scratch-build failures. The helper plan returns an error on hardlink/copy failures other than `EXDEV` ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/015-watch-once-streaming-scan/tasks.md:15)), but the spec requires skipped files recorded in quarantine and the run to complete ([spec.md](/Users/dalecarman/dev/coding_agent_session_search/specs/015-watch-once-streaming-scan/spec.md:55)). Fix: treat disappeared/unreadable/copy-failed source files as explicit skip records with reasons, unless the error is truly systemic.

**Adversarial Gate**

Riskiest assumptions:

1. External IDs stay identical under scratch roots. Verified conditionally. FAD uses `sessions_dir(home)` as `home/sessions` if present, else `home` ([pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:74)), then strips that prefix for `external_id` ([pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:340)). This holds if scratch and original sessions roots are derived correctly.

2. Raw-mirror identity is preserved. Verified the plan now has the right shape: current preparse capture records discovered `source_path` directly ([mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:17209), [mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:17252)), and manifest identity includes original path hash ([raw_mirror.rs](/Users/dalecarman/dev/coding_agent_session_search/src/raw_mirror.rs:844)). Original-root preparse plus source-path remap before `attach_raw_mirror_capture` is required.

3. Non-pi connectors are unchanged. Not fully verified; the plan intends this, but the pseudocode’s discovery-before-kind-check contradicts it. This is one of the revision points.

A skeptical senior engineer’s first objection would be: “You’re using a filesystem scratch mirror to simulate per-file scan support. Prove the exact user command hits this branch, prove non-pi connectors do not pay for it, and prove the receipt is a stable JSON contract.”

Production needs not covered: crash cleanup/GC for scratch dirs after SIGKILL, robust handling for source files changing mid-run, explicit JSON schema/golden updates, and a durable resume story if a two-week backfill is interrupted.

Scope differences: the spec allowed generic Shape A/B; the plan narrows to pi-only scratch-root batching. It also adds three new knobs and a receipt/quarantine contract, which is reasonable, but those are new API surface and need schema/test treatment. It drops the future `scan_with_callback` activation from implementation scope, which is acceptable for this spec.

**What I Verified**

I read the current spec, plan, tasks, watch-once ingest path, FAD Connector trait, FAD `ScanContext`, FAD Pi root/discovery/parser logic, raw-mirror capture/manifest identity, and current index JSON output assembly. I also checked preflight: branch is `dac/main`, the tree is dirty, and `br ready` fails here with `Operation not permitted`. I did not run tests; this was a read-only plan review.

VERDICT: REVISE