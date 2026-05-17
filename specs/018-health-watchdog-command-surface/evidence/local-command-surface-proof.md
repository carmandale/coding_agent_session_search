---
title: "Command-surface proof: health watchdog"
date: 2026-05-17T04:00:58Z
bead: coding_agent_session_search-2gif2
---

# Local Command-Surface Proof

## Scope

This is local source/debug-binary plus approval-gated release-candidate proof.
It does not install a binary, mutate launchd, reload
`com.cass.health-watchdog`, or prove the live plist is fixed.

## Root Cause

Symptom: installed and pre-repair spec016 release-candidate binaries returned
exit `2` with `Could not parse arguments` for `cass watchdog run --help`, while
the loaded `com.cass.health-watchdog` plist invokes `cass watchdog run`.

Root cause:

1. `src/watchdog.rs` existed with `WatchdogCommand` and `run_watchdog_command`.
2. `src/lib.rs` did not expose `pub mod watchdog`.
3. The top-level `Commands` enum had no `Watchdog` variant.
4. CLI dispatch, command description, capabilities, and robot docs had no
   watchdog surface.
5. Because the module was dead code, compiling it also surfaced the missing
   direct `libc` dependency and one clippy issue.

Fix: wire the existing watchdog module into the top-level CLI and frozen robot
contract, add a direct `libc` dependency for its POSIX calls, and add a focused
regression test for `watchdog run --help`.

## Changed Local Surface

- `Cargo.toml`: added direct `libc = "*"` dependency.
- `src/lib.rs`: exposed `pub mod watchdog`, added `Commands::Watchdog`, dispatch,
  command description, canonical command list, and robot help/docs entries.
- `src/watchdog.rs`: removed clippy `needless_return` once the module compiled.
- `tests/cli_robot.rs`: added `watchdog_run_help_dispatches` and capabilities
  assertion for `watchdog`; replaced existing assertion-helper `panic!` macros
  with `std::panic::panic_any(...)` so the touched test file no longer has UBS
  criticals.
- Golden command contracts updated:
  - `tests/golden/robot/capabilities.json.golden`
  - `tests/golden/robot/introspect.json.golden`
  - `tests/golden/robot_docs/commands.txt.golden`
  - `tests/golden/robot_docs/robot_help.txt.golden`

## Verification

```text
env CARGO_TARGET_DIR=/tmp/cass-check-target "$HOME/.cargo/bin/cargo" test watchdog_run_help_dispatches --test cli_robot
result: pass, 1 passed
```

```text
env CARGO_TARGET_DIR=/tmp/cass-check-target "$HOME/.cargo/bin/cargo" test capabilities --test cli_robot
result: pass, 13 passed
```

```text
env CARGO_TARGET_DIR=/tmp/cass-check-target "$HOME/.cargo/bin/cargo" test --test cli_robot stats_
result: pass, 6 passed

env CARGO_TARGET_DIR=/tmp/cass-check-target "$HOME/.cargo/bin/cargo" test search_cursor_manifest_marks_rebuilding_generation_best_effort --test cli_robot
result: pass, 1 passed
```

Note: a broad parallel `cargo test --test cli_robot search_` run passed `67/68`
tests and failed `search_cursor_manifest_marks_rebuilding_generation_best_effort`
with `kind="index-busy"` while another search repair path was active. The exact
failed test passed in isolation, so the failure is recorded as existing
lock-contention behavior in the broad filter, not as caused by the panic cleanup.

```text
env CARGO_TARGET_DIR=/tmp/cass-check-target "$HOME/.cargo/bin/cargo" test capabilities_json_matches_golden --test golden_robot_json
result: pass, 1 passed

env CARGO_TARGET_DIR=/tmp/cass-check-target "$HOME/.cargo/bin/cargo" test introspect_json_matches_golden --test golden_robot_json
result: pass, 1 passed

env CARGO_TARGET_DIR=/tmp/cass-check-target "$HOME/.cargo/bin/cargo" test robot_docs_commands_matches_golden --test golden_robot_docs
result: pass, 1 passed

env CARGO_TARGET_DIR=/tmp/cass-check-target "$HOME/.cargo/bin/cargo" test robot_help_matches_golden --test golden_robot_docs
result: pass, 1 passed
```

```text
env CARGO_TARGET_DIR=/tmp/cass-check-target "$HOME/.cargo/bin/cargo" check --all-targets
result: pass

env CARGO_TARGET_DIR=/tmp/cass-check-target "$HOME/.cargo/bin/cargo" fmt --check
result: pass

env CARGO_TARGET_DIR=/tmp/cass-check-target "$HOME/.cargo/bin/cargo" clippy --all-targets -- -D warnings
result: pass
```

```text
/tmp/cass-check-target/debug/cass watchdog run --help
result: exit 0
observed: "Run a one-shot health check (heartbeat + log rotation + restart if stale)"
```

Release-candidate proof:

```text
env CARGO_TARGET_DIR=/tmp/cass-release-target "$HOME/.cargo/bin/cargo" build --release --bin cass
result: pass

/tmp/cass-release-target/release/cass --version
result: cass 0.4.7

shasum -a 256 /tmp/cass-release-target/release/cass
result: a5a139ca503f0f04f2b0baba06e20b16bd7003a7e7d78306cacf94d56eaca9c2

/tmp/cass-release-target/release/cass watchdog run --help
result: exit 0
observed: "Run a one-shot health check (heartbeat + log rotation + restart if stale)"
```

UBS scope check:

```text
ubs --format=json --jsonl-summary-only src/watchdog.rs
result: exit 0
totals: critical=0, warning=109, info=153

ubs --format=json --jsonl-summary-only tests/cli_robot.rs
result: exit 0
totals: critical=0, warning=1585, info=410
classification: panic! critical inventory removed from the touched CLI test file.

ubs --format=json --jsonl-summary-only src/watchdog.rs tests/cli_robot.rs Cargo.toml Cargo.lock
result: exit 0
totals: critical=0, warning=1694, info=557
classification: warning-only inventory remains; no criticals in the spec018 touched set.
```

## Remaining Acceptance

Acceptance criteria 1 and 2 are satisfied for the local debug/source surface and
the approval-gated release candidate. Acceptance criteria 3 and 4 are not
satisfied: the installed binary and launchd plist have not been changed, and no
launchd smoke has run.

This follow-up is therefore partially implemented, not complete.
