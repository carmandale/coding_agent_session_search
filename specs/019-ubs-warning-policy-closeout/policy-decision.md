---
title: "Policy decision: UBS warning inventory"
date: 2026-05-17T03:45:56Z
bead: coding_agent_session_search-2v7tv
status: acceptance-required
---

# Policy Decision: UBS Warning Inventory

## Decision

Do not add a hidden baseline, broad ignore list, or CI workflow weakening to
close spec 016 T20.

The current UBS warning inventory should be handled by one of these explicit
paths:

1. fix/reduce the changed-file warnings until the existing CI-shaped
   `ubs --ci --fail-on-warning` command exits `0`;
2. implement a reviewed future CI wrapper that fails on new warning deltas while
   preserving the strict gate intent; or
3. record final-review acceptance that the current warning inventory is outside
   the live CASS recovery gate for spec 016.

For spec 016 closeout, path 3 is the only narrow route that does not broaden the
already approval-gated recovery.

## Evidence

The strict gate is still present:

```text
.github/workflows/ci.yml: ubs --ci --fail-on-warning
tests/ci_workflow_validates_ubs_gate.rs: asserts the canonical invocation
AGENTS.md: warnings stop merges
```

The current changed-file UBS report is warning-only:

```text
command: xargs ubs --ci --fail-on-warning --format=json --report-json=/tmp/spec016-ubs-ci-report.json < /tmp/spec016-ubs-files.txt
exit: 1
totals: critical=0, warning=20733, info=11159, files=10
```

The latest spec018-local cleanup also cleared the touched CLI test criticals:

```text
ubs --format=json --jsonl-summary-only tests/cli_robot.rs
exit: 0
totals: critical=0, warning=1585, info=410
```

The baseline experiment showed zero delta but still failed under the current
strict gate:

```text
command: xargs ubs --ci --fail-on-warning --comparison=/tmp/spec019-ubs-baseline-report.json --format=json --report-json=/tmp/spec019-ubs-comparison-report.json < /tmp/spec016-ubs-files.txt
exit: 1
comparison.delta: critical=0, warning=0, info=0
totals: warning=19148
```

UBS exposes `--comparison`, but no `fail-on-new-warning` or
`fail-on-delta` mode. Therefore a delta-baseline policy would require an
intentional CI wrapper or post-processing design, not a drop-in flag change.

## Why Not Suppress

The warning inventory spans broad production files:

```text
src/indexer/mod.rs=5944 warnings
src/lib.rs=4883 warnings
src/storage/sqlite.rs=4517 warnings
src/ui/app.rs=3114 warnings
```

Adding file-level ignores or broad suppressions for these files would hide too
much signal. That would violate the spec 019 requirement to keep suppressions
narrow, reviewable, and tied to known-acceptable cases.

## Spec 016 Impact

Spec 016 T20 should remain unchecked until final review either:

- accepts this warning-only inventory as outside the live-recovery completion
  gate; or
- directs a separate UBS policy/wrapper implementation; or
- directs broad warning cleanup as part of the recovery.

This decision does not mutate live CASS data, install a binary, promote an
archive, reload launchd, change git history, or weaken CI.
