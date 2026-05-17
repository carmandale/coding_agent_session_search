---
title: "Research: UBS warning-policy closeout"
date: 2026-05-17T03:45:56Z
bead: coding_agent_session_search-2v7tv
---

# Research: UBS warning-policy closeout

## Question

Can spec 016's T20 UBS blocker be closed by a narrow local cleanup or a
drop-in UBS baseline, without weakening the existing `ubs --ci
--fail-on-warning` CI gate?

## Evidence

Changed-file list from spec 016:

```text
Cargo.lock
Cargo.toml
src/indexer/mod.rs
src/indexer/redact_secrets.rs
src/indexer/scratch_root.rs
src/lib.rs
src/main.rs
src/search/asset_state.rs
src/storage/sqlite.rs
src/ui/app.rs
tests/spec_015_streaming_watch_once.rs
```

After the spec018 command-surface repair touched `tests/cli_robot.rs`, the
current changed-file Rust scan also includes that file. The updated warning-only
totals are:

```text
git diff --name-only -- '*.rs' Cargo.toml Cargo.lock | tr '\n' '\0' | xargs -0 ubs --format=json --jsonl-summary-only
exit=0
critical=0
warning=20733
info=11159
files=10
```

The CI-shaped command still fails warning-only:

```text
git diff --name-only -- '*.rs' Cargo.toml Cargo.lock | xargs ubs --ci --fail-on-warning --format=json --report-json=/tmp/spec016-ubs-ci-report-latest.json
exit=1
critical=0
warning=20733
info=11159
files=10
```

UBS version observed in local stderr:

```text
UBS Meta-Runner v5.0.7
```

### Tool Surface

Command:

```text
ubs --help
```

Relevant options:

```text
--ci                    CI mode (stable timestamps)
--fail-on-warning       Exit non-zero if warnings or critical exist
--comparison=FILE       Baseline JSON to diff combined results against
--report-json=FILE      Write combined summary JSON to FILE
--ignore-file=PATH      Read additional ignore globs
```

Interpretation: UBS exposes comparison reporting, but no dedicated
`fail-on-new-warning` or `fail-on-delta` mode. A baseline policy that allows
legacy warnings while failing new warning deltas would therefore require a
reviewed CI wrapper or post-processing step, not a one-flag substitution for the
current gate.

### Baseline Experiment

Command:

```text
xargs ubs --ci --format=json --report-json=/tmp/spec019-ubs-baseline-report.json < /tmp/spec016-ubs-files.txt
```

Result:

```text
exit=0
critical=0
warning=19148
info=10752
files=9
```

Then:

```text
xargs ubs --ci --fail-on-warning --comparison=/tmp/spec019-ubs-baseline-report.json --format=json --report-json=/tmp/spec019-ubs-comparison-report.json < /tmp/spec016-ubs-files.txt
```

Result:

```text
exit=1
comparison.delta.critical=0
comparison.delta.warning=0
comparison.delta.info=0
totals.warning=19148
```

Interpretation: UBS can report a zero delta against a baseline, but the current
`--fail-on-warning` behavior still exits nonzero on total warnings. A baseline
file alone is not a drop-in fix for the existing CI invocation.

### Per-File Distribution

Per-file runs used `ubs --ci --format=json --report-json=<tmp> <file>` and read
`.totals` from the generated report.

| File | Critical | Warning | Info |
| --- | ---: | ---: | ---: |
| `src/indexer/mod.rs` | 0 | 5944 | 2074 |
| `src/indexer/redact_secrets.rs` | 0 | 91 | 28 |
| `src/indexer/scratch_root.rs` | 0 | 65 | 44 |
| `src/lib.rs` | 0 | 4883 | 5579 |
| `src/main.rs` | 0 | 0 | 14 |
| `src/search/asset_state.rs` | 0 | 494 | 212 |
| `src/storage/sqlite.rs` | 0 | 4517 | 958 |
| `src/ui/app.rs` | 0 | 3114 | 1859 |
| `tests/spec_015_streaming_watch_once.rs` | 0 | 41 | 25 |

The largest buckets are broad production files, not a single test fixture or a
single obvious warning class. This does not support a quick, surgical cleanup
inside spec 016.

### Policy Surface

The strict gate is deliberately pinned in multiple places:

- `AGENTS.md` says every PR runs `ubs --ci --fail-on-warning` and warnings stop
  merges.
- `.github/workflows/ci.yml` runs `ubs --ci --fail-on-warning` in the
  `ubs-changed-files` job.
- `tests/ci_workflow_validates_ubs_gate.rs` asserts that the workflow contains
  `ubs --ci --fail-on-warning`.
- `scripts/tests/dpfvr_ubs_gate_e2e.sh` locally reproduces the same gate shape.

Implication: changing the workflow to use `--comparison` without
`--fail-on-warning`, then failing on parsed deltas, would be an intentional
policy change. It may be a reasonable future design, but it is not a stealth
closeout fix for spec 016 and should be reviewed as such.

## Current Conclusion

T20 should stay unchecked. The honest routes are:

1. reduce/fix the warning inventory until the CI-shaped fail-on-warning command
   exits 0;
2. design and review an explicit baseline policy or CI wrapper that changes CI
   to fail on new warning deltas while preserving the strict gate intent; or
3. obtain final reviewer acceptance that the warning inventory is outside spec
   016's live-recovery gate.

No live CASS data, binary, launchd service, or session root was mutated during
this research.

## Decision Artifact

`policy-decision.md` records the follow-on decision: do not add broad ignores,
hidden baselines, or a workflow weakening. For spec 016 closeout, the narrow
non-live route is final-review acceptance that the current warning-only
inventory is outside the live CASS recovery gate. A warning-delta baseline would
need a reviewed CI wrapper/policy change because UBS has `--comparison` but no
`fail-on-new-warning` or `fail-on-delta` mode.
