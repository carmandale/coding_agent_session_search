**Findings**

1. Scratch-build skips break the receipt math. The revised receipt adds `scratch_build_skips`, but `parser_skip_records` is still defined as `N - E` and `parse_unaccounted_files` ignores `K` ([plan.md](/Users/dalecarman/dev/coding_agent_session_search/specs/015-watch-once-streaming-scan/plan.md:237)). Since a scratch-build failure is discovered but never emitted, it gets counted both as a parser skip and a scratch skip. Fix the file-level formula to:
`parse_unaccounted_files = discovered_files - emitted_source_files - parser_skip_records - scratch_build_skips`
and compute `parser_skipped_paths = discovered - scratch_skipped - emitted`.

2. Scratch-build failure handling is contradictory. The helper pseudocode still returns `Result<ScratchRootGuard>` and aborts on non-EXDEV errors ([plan.md](/Users/dalecarman/dev/coding_agent_session_search/specs/015-watch-once-streaming-scan/plan.md:173), [plan.md](/Users/dalecarman/dev/coding_agent_session_search/specs/015-watch-once-streaming-scan/plan.md:192)), while the text says per-file failures must not abort ([plan.md](/Users/dalecarman/dev/coding_agent_session_search/specs/015-watch-once-streaming-scan/plan.md:201)) and tasks use `Result<(ScratchRootGuard, Vec<ScratchBuildSkip>)>` ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/015-watch-once-streaming-scan/tasks.md:15)). Also the plan text says the helper records directly to quarantine, but its signature has no `data_dir`; tasks say caller records later. Make one source of truth: helper returns skips, caller writes them exactly once.

3. The non-pi discovery regression test is impossible as written. T14 says assert `discover_source_files` is not invoked for non-pi watch-once ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/015-watch-once-streaming-scan/tasks.md:42)), but the existing unchanged bulk path calls `capture_connector_sources_before_parse` before scan ([mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:16237)), and that helper calls `connector.discover_source_files(ctx)` ([mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:17209)). The right assertion is “no extra pre-routing discovery before falling into the existing bulk path,” or simply “streaming branch not entered and non-pi output unchanged.”

**Adversarial Gate**

Riskiest assumptions:
- External IDs stay identical under scratch roots: verified from FAD `sessions_dir` and `strip_prefix` behavior ([pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:74), [pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:340)).
- Original-root raw-mirror capture prevents scratch manifest forks: verified against raw-mirror manifest identity using original path hash ([raw_mirror.rs](/Users/dalecarman/dev/coding_agent_session_search/src/raw_mirror.rs:844)).
- Non-pi connectors avoid added discovery work: not verified; current plan wording overcorrects and conflicts with the existing preparse capture path.

A skeptical senior engineer’s first objection would be: “The plan now has two competing definitions of skipped files and two competing implementations of scratch failure handling. Which one should the implementer trust?”

What production still needs: SIGKILL-era scratch GC, exact receipt schema/golden update, and a race policy for files modified while hardlinked into scratch.

**What I Verified**

I read the actual modified `plan.md` and `tasks.md` diff, not just the pasted summary. I rechecked the watch-once bulk path, preparse raw-mirror discovery, FAD Pi root/external-id logic, and raw-mirror manifest identity. The prior review’s five issues are mostly addressed, but the new scratch-skip accounting and non-pi test wording introduced material ambiguity.

VERDICT: REVISE