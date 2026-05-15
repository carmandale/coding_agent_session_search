<!-- codex-review:approved:v1 | harness: codex/gpt-5.3-codex | date: 2026-05-15T22:58:32Z | rounds: 4 -->

## Round 1
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
## Round 2
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
## Round 3
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
## Round 4 — APPROVED
**Findings**

No blocking issues in the revised plan text. The two prior blockers are fixed:

- `build_scratch_root` now returns `(ScratchRootGuard, Vec<ScratchBuildSkip>)`, and the main pseudocode accumulates `scratch_skips` before reconciliation.
- Parser skips now subtract scratch skips: `parser_skipped_paths = discovered_source_paths - emitted_source_paths - scratch_skipped_paths`, preserving the three disjoint file buckets.

One artifact-drift note: the checked-in [tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/015-watch-once-streaming-scan/tasks.md:29) still shows the older receipt shorthand `parser_skip_records:N-E`. The next line block has the corrected disjoint-set formula, and the plan itself is correct, so I’m not treating this as a plan blocker. I would still clean that task line before handing it to an implementation worker.

**Adversarial Gate**

The 3 riskiest assumptions:

1. **Scratch scan preserves pi external IDs.** Verified against FAD: `sessions_dir(home)` resolves to `home/sessions` when present, and `external_id` is derived via `strip_prefix(&sessions_dir)`.
2. **Raw-mirror identity stays canonical.** Verified against current cass behavior: preparse capture uses discovered source paths, so the plan’s original-root-only preparse is the right correction.
3. **Skipped-file accounting remains disjoint.** Verified in the plan: emitted, scratch-skip, and parser-skip paths now have separate buckets and quarantine reasons.

A skeptical senior engineer’s first objection would now be implementation discipline: make sure the worker follows the corrected plan, not the stale shorthand in T9.

What production still needs beyond this spec: stale scratch cleanup after hard process death, resumable backfill receipts, and long-run operational dashboards. Those are hardening follow-ups, not spec blockers.

**What I Verified**

I read the current `plan.md`, `tasks.md`, FAD pi root/external-id code, cass watch-once bulk path, raw-mirror preparse capture, current OOM quarantine outcome, and the index response schema location. Counts checked: 3 disjoint file buckets in the plan, 2 scan knobs, 1 preserved persist knob, and 1 stale task-line shorthand.

VERDICT: APPROVED