<!-- codex-review:approved:v1 | harness: unknown | date: 2026-05-16T13:48:19Z -->

1. **Completeness**: Pass. The draft covers each spec requirement: upstream ancestor/blocker, priority raw→DB reconciliation, 3 lexical probes per priority agent, watcher proof with new/modified session, spec-015 routing, command evidence, and GoalBuddy completion gate ([draft](/tmp/codex-plan-draft-6853697e.md:11), [draft](/tmp/codex-plan-draft-6853697e.md:103), [draft](/tmp/codex-plan-draft-6853697e.md:207), [draft](/tmp/codex-plan-draft-6853697e.md:210), [draft](/tmp/codex-plan-draft-6853697e.md:225), [draft](/tmp/codex-plan-draft-6853697e.md:227), [draft](/tmp/codex-plan-draft-6853697e.md:236)).

2. **Correctness**: Pass. The selected runtime-first shape plus hard watcher requirement and smallest-cause repair is coherent and executable ([draft](/tmp/codex-plan-draft-6853697e.md:47), [draft](/tmp/codex-plan-draft-6853697e.md:138), [draft](/tmp/codex-plan-draft-6853697e.md:216)).

3. **Risks**: Residual but addressed. Main remaining risks are lock contention, OOM/quarantine-heavy runs, and branch authorization delay at finalize; all now have explicit handling paths ([draft](/tmp/codex-plan-draft-6853697e.md:109), [draft](/tmp/codex-plan-draft-6853697e.md:123), [draft](/tmp/codex-plan-draft-6853697e.md:105)).

4. **Missing steps**: No blocking gaps found. Final deltas are reflected, including non-priority measurable bar and explicit prohibition on unsolicited watchdog wiring ([draft](/tmp/codex-plan-draft-6853697e.md:97), [draft](/tmp/codex-plan-draft-6853697e.md:129), [draft](/tmp/codex-plan-draft-6853697e.md:211)).

5. **Alternatives**: Simpler alternative would be “priority-only then stop,” but it fails your new non-priority regression bar. Current plan is appropriately stricter for this recovery scope ([draft](/tmp/codex-plan-draft-6853697e.md:97), [draft](/tmp/codex-plan-draft-6853697e.md:211)).

6. **Security**: Acceptable. Evidence hygiene/redaction is explicit; remaining risk is operator discipline in log excerpt handling, but task coverage is present ([draft](/tmp/codex-plan-draft-6853697e.md:93), [draft](/tmp/codex-plan-draft-6853697e.md:228)).

7. **3 riskiest assumptions (verified against source)**:
- Assumption A: shipped CLI supports `--watch-once` (needed for route). Verified in [src/lib.rs](/Users/dalecarman/dev/coding_agent_session_search/src/lib.rs:284) and [src/lib.rs](/Users/dalecarman/dev/coding_agent_session_search/src/lib.rs:300).
- Assumption B: Pi watch-once behavior differs from other connectors. Verified in [src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:16308) and connector hint behavior in [src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:17124).
- Assumption C: provenance uniqueness alone is insufficient when `external_id` is null; source-path/start-time merge logic matters. Verified in [src/storage/sqlite.rs](/Users/dalecarman/dev/coding_agent_session_search/src/storage/sqlite.rs:4386) and [src/storage/sqlite.rs](/Users/dalecarman/dev/coding_agent_session_search/src/storage/sqlite.rs:11081).

8. **Skeptical senior engineer’s first objection**: “This is still operationally heavy; prove fast success criteria early so non-priority checks don’t turn into open-ended churn.”

9. **What production would still need**: automated reconciliation/alerting, continuous watcher liveness SLOs, and automated evidence redaction enforcement instead of manual hygiene steps.

10. **Plan scope vs spec scope**: Scope is aligned and improved. It narrowed watchdog command work (now opt-in with user authorization), expanded non-priority measurable acceptance, and strengthened branch/finalization safeguards ([draft](/tmp/codex-plan-draft-6853697e.md:129), [draft](/tmp/codex-plan-draft-6853697e.md:138), [draft](/tmp/codex-plan-draft-6853697e.md:97), [draft](/tmp/codex-plan-draft-6853697e.md:105)).

11. **R0 Shape Comparison gate**: Pass. `## Shape Comparison` exists and compares 4 plausible shapes with net-complexity framing ([draft](/tmp/codex-plan-draft-6853697e.md:43)).

12. **Plan Sanity Evidence gate (spec 117)**: Pass. `## Plan Sanity Evidence` includes all 5 labeled lines, each substantive; observed result includes executable evidence (commands + exit/result facts); decision impact names concrete downstream sections ([draft](/tmp/codex-plan-draft-6853697e.md:77), [draft](/tmp/codex-plan-draft-6853697e.md:79), [draft](/tmp/codex-plan-draft-6853697e.md:81), [draft](/tmp/codex-plan-draft-6853697e.md:83), [draft](/tmp/codex-plan-draft-6853697e.md:85), [draft](/tmp/codex-plan-draft-6853697e.md:87)).

VERDICT: APPROVED