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