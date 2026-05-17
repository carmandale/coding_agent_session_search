---
title: "Resolve UBS warning policy for spec 016 closeout"
date: 2026-05-17
bead: coding_agent_session_search-2v7tv
---

<!-- issue:complete:v1 | harness: unknown | date: 2026-05-17T02:34:23Z -->

# Resolve UBS warning policy for spec 016 closeout

## Source (verbatim)

> "be in sync with upstream. process all sessions and allow them to be searchable. sessions that matter are pi-agent, claude code, an codex, with opencode, factory and others being bonus sessions. then set up a watcher so that all sessions are processed and part of the searchable system." - user, 2026-05-16

> "create a new $issue if necessary." - user, 2026-05-16

## Problem

Purpose Contract:

- Outcome: spec 016's verifier floor has a truthful, reviewable answer for the repository's UBS warning policy before closeout, without weakening the existing CI gate.
- Done means: the same changed-file UBS shape used by CI either passes, or an explicit reviewed UBS baseline/suppression/acceptance artifact explains why the warning inventory does not block the recovery merge.
- Not done: marking T20 complete because local UBS criticals are zero while `.github/workflows/ci.yml` still runs `ubs --ci --fail-on-warning` and exits nonzero on warnings.

Spec 016 cleared local changed-file UBS criticals, but the CI-shaped command still fails because the repository's UBS pre-merge gate treats warnings as blocking:

```text
xargs ubs --ci --fail-on-warning --format=json --report-json=/tmp/spec016-ubs-ci-report.json < /tmp/spec016-ubs-files.txt
result: fail, exit=1
summary: 0 critical, 20733 warnings, 11159 info, 10 Rust files
```

The strict gate is intentional. `.github/workflows/ci.yml` documents the `ubs-changed-files` job as running `ubs --ci --fail-on-warning`, and `tests/ci_workflow_validates_ubs_gate.rs` asserts that canonical invocation. The warning inventory is therefore not a cosmetic local note; it is a merge/closeout policy blocker unless resolved or explicitly accepted.

Root cause:

1. Symptom: spec 016 T20 cannot honestly close even though local critical findings are zero.
2. Why: the CI command fails on warning findings, not just critical findings.
3. Why: the changed-file set includes large, test-heavy files with whole-file warning inventory such as `unwrap()`/`expect()` and assertion usage.
4. Why: no UBS baseline, config-level suppression, or final-review acceptance currently distinguishes acceptable inherited warning inventory from new blocker findings.

## Requirements

1. Preserve the existing `coding_agent_session_search-dpfvr` UBS gate semantics; do not remove `--fail-on-warning`, neuter the workflow test, or silently skip changed files.
2. Determine whether the spec 016 warning inventory is new, inherited from pre-existing touched files, or intentionally acceptable test code.
3. Choose one durable closeout path:
   - reduce or fix the warning inventory until the CI-shaped command passes;
   - add reviewable UBS policy/config suppressions or baselines for known-acceptable warnings; or
   - record explicit final-review acceptance that this inventory is outside the live-recovery completion gate.
4. Keep the decision tied to `specs/016-cass-recovery-ingestion/evidence/ubs-warning-inventory.md` and update spec 016 T20 only after the selected path is proven.
5. Do not mutate live CASS data, install binaries, promote archives, or reload watchers as part of this issue.
6. Avoid new `rusqlite` code.

## Constraint

- This issue must not become a backdoor for weakening UBS or deleting the CI gate.
- Do not mark spec 016 T20 complete solely from `0 critical` output.
- Do not rewrite broad test files just to placate a scanner unless the rewrite improves real maintainability and preserves test intent.
- If suppressions are used, they must be narrow, reviewable, and documented at the UBS policy/config level or inline where the code owner can audit them.
- If final-review acceptance is used, spec 016 must say exactly what is accepted and why CI behavior remains understood.

## Acceptance Criteria

1. A CI-shaped local command equivalent to `ubs --ci --fail-on-warning` on the intended changed-file set either exits `0`, or the nonzero warning inventory is covered by a documented baseline/suppression/acceptance decision.
2. `.github/workflows/ci.yml` still contains the `ubs-changed-files` job and still runs `ubs --ci --fail-on-warning`.
3. `tests/ci_workflow_validates_ubs_gate.rs` still passes or is updated only to reinforce the strict gate contract.
4. Spec 016's T20/task receipt is updated to point at the selected UBS decision and no longer implies `0 critical` is sufficient.
5. The final receipt records the exact UBS command, exit code, finding counts, and chosen policy path.

## Out of Scope

- Live CASS DB/index promotion.
- Frankensqlite durable pin resolution.
- Upstream merge/branch authorization.
- Watcher launchd reload or synthetic session proof.
- Broad project-wide UBS cleanup unrelated to files changed by spec 016.

## Selected Shape

Direct policy-blocker follow-up. Start with the recorded spec 016 UBS evidence and the strict CI contract, then either make the changed-file command pass or create a narrow, reviewable policy decision that keeps the gate strict while allowing spec 016 closeout to be evaluated honestly.
