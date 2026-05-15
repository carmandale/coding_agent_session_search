**Findings**

1. Skip accounting still has one stale formula that breaks the disjoint-bucket model. The receipt now correctly includes `scratch_build_skips`, but the skipped-file paragraph still says `parser_skipped_paths = discovered_source_paths - emitted_source_paths` in [plan.md](/Users/dalecarman/dev/coding_agent_session_search/specs/015-watch-once-streaming-scan/plan.md:274). That would classify scratch-build failures as parser skips too. The same stale shape remains in [tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/015-watch-once-streaming-scan/tasks.md:29) and [tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/015-watch-once-streaming-scan/tasks.md:31). Fix to: `parser_skipped_paths = discovered_source_paths - emitted_source_paths - scratch_skipped_paths`.

2. The main streaming pseudocode still treats `build_scratch_root` as returning only a guard, while the helper now returns `(ScratchRootGuard, Vec<ScratchBuildSkip>)`. See [plan.md](/Users/dalecarman/dev/coding_agent_session_search/specs/015-watch-once-streaming-scan/plan.md:108) versus [plan.md](/Users/dalecarman/dev/coding_agent_session_search/specs/015-watch-once-streaming-scan/plan.md:182). The implementation recipe should explicitly accumulate `scratch_skips` so the receipt and quarantine writer have the right source of truth.

**Adversarial Gate**

The 3 riskiest assumptions are:

- External IDs stay identical under scratch-root scans. Verified against FAD pi `sessions_dir` and `strip_prefix` behavior; this looks sound.
- Raw-mirror identity stays canonical. Verified by the plan’s original-root preparse capture plus source-path remap before post-parse capture; this looks sound.
- Skipped-file buckets are disjoint. Not currently true in the written plan because scratch skips are still included in `discovered - emitted`.

A skeptical senior engineer’s first objection would be: “Your receipt says the buckets are disjoint, but your set formula overlaps them.”

What this does not address for production: resumable long backfills, stale scratch cleanup after process kill, and a formal JSON golden for the new top-level receipt beyond the noted schema/golden update. Those are not spec blockers, but they are real follow-up hardening.

Scope-wise, the plan is still within spec: cass-side only, pi-specific Shape A, non-pi bulk path preserved. The only blocker is internal consistency around skip accounting.

VERDICT: REVISE