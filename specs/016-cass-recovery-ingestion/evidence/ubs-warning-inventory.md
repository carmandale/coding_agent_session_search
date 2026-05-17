---
title: "UBS warning inventory"
date: 2026-05-17T03:45:56Z
bead: coding_agent_session_search-1vxuf
---

# UBS Warning Inventory

This file records the remaining CI-policy caveat after changed-file UBS
criticals were cleared. It does not authorize weakening the CI gate and does
not mark T20 complete.

## Commands

Changed-file list:

```text
git diff --name-only -- '*.rs' Cargo.toml Cargo.lock > /tmp/spec016-ubs-files.txt
```

Local preflight:

```text
git diff --name-only -- '*.rs' Cargo.toml Cargo.lock | tr '\n' '\0' | xargs -0 ubs --format=json --jsonl-summary-only
result: pass
summary: 0 critical, 20733 warnings, 11159 info, 10 files
```

CI-shaped command:

```text
xargs ubs --ci --fail-on-warning --format=json --report-json=/tmp/spec016-ubs-ci-report.json < /tmp/spec016-ubs-files.txt
result: fail, exit=1
summary: 0 critical, 20733 warnings, 11159 info, 10 files
```

Interpretation: the CI-shaped command fails only because
`--fail-on-warning` treats warning inventory as merge-blocking. The critical
count is zero.

## Critical Cleanup Refresh

The spec018 command-surface repair touched `tests/cli_robot.rs`, which had an
existing UBS critical inventory from `panic!` macros in assertion helpers. Those
macros were replaced with `std::panic::panic_any(...)`.

```text
rg -n "panic!" tests/cli_robot.rs
result: no matches

ubs --format=json --jsonl-summary-only tests/cli_robot.rs
result: exit 0; critical=0, warning=1585, info=410

ubs --format=json --jsonl-summary-only src/watchdog.rs tests/cli_robot.rs Cargo.toml Cargo.lock
result: exit 0; critical=0, warning=1694, info=557, files=2
```

This changes the current changed-file UBS state from critical-clean to still
critical-clean with one additional Rust file in scope. T20 still does not close:
the CI-shaped command remains warning-blocked by `--fail-on-warning`.

## Warning Classes

The text UBS stream on the same changed-file set reports these warning classes:

```text
unwrap()/expect() usage: 12110
panic!/unreachable!/todo!/unimplemented!: 7 unreachable! warnings, 0 panic! criticals
Mutex::lock().unwrap()/expect(): 80
thread::sleep in async: 31
assert!/assert_eq!/assert_ne! inventory: 6887
parse::<T>().unwrap()/expect(): 2
serde_json::from_str(...).unwrap()/expect(): 30
```

The largest classes are whole-file inventory in large changed test-heavy files,
not new targeted findings introduced by the latest critical cleanup. Example
samples came from `tests/spec_015_streaming_watch_once.rs` test fixture setup
using `unwrap()` and `src/search/asset_state.rs` test assertions using
`expect(...)`.

## Decision

No UBS policy/config baseline was added in this pass. A baseline or config-level
suppression would affect the repository CI contract and should be handled as an
explicit policy decision during branch/commit resolution, not slipped into the
live-recovery patch.

T20 should remain unchecked until one of these is true:

- The warning inventory is fixed or reduced enough for
  `ubs --ci --fail-on-warning` to pass on changed files.
- The repository intentionally baselines or suppresses known-acceptable warnings
  at the UBS policy/config level with reviewable justification.
- The final reviewer explicitly accepts this warning inventory as outside the
  live-recovery completion gate.
