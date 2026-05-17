## Findings

1. **High — Spec/plan fulfillment is incomplete (minimal-delta contract not met).**  
Spec requires `src/` diff to be only watchdog wiring files ([spec.md](/Users/dalecarman/dev/coding_agent_session_search/specs/011-upstream-sync/spec.md:126)) and plan goal says only `src/watchdog.rs` + 6 `src/lib.rs` sites ([plan.md](/Users/dalecarman/dev/coding_agent_session_search/specs/011-upstream-sync/plan.md:20)).  
Actual diff has 4 `src/` files: `src/daemon/resource.rs`, `src/lib.rs`, `src/search/asset_state.rs`, `src/watchdog.rs` (verified via `git diff upstream/main --name-only -- src/`; also acknowledged in [/tmp/claude-verify-593f1f27.md](/tmp/claude-verify-593f1f27.md:22)).  
Action: either remove the extra `src` edits or explicitly revise spec/plan/tasks to accept them.

2. **High — Acceptance gates marked done despite unresolved hard criteria.**  
Spec acceptance says `cargo test --lib` must all pass and health must be true ([spec.md](/Users/dalecarman/dev/coding_agent_session_search/specs/011-upstream-sync/spec.md:132), [spec.md](/Users/dalecarman/dev/coding_agent_session_search/specs/011-upstream-sync/spec.md:134)).  
Evidence still reports `3104 passed / 55 failed` ([/tmp/claude-verify-593f1f27.md](/tmp/claude-verify-593f1f27.md:44), [implement-receipt.md](/Users/dalecarman/dev/coding_agent_session_search/specs/011-upstream-sync/implement-receipt.md:71)) and `test_result: partial-pass` ([implement-receipt.md](/Users/dalecarman/dev/coding_agent_session_search/specs/011-upstream-sync/implement-receipt.md:5)).  
Action: either close the remaining failures/health issues or change acceptance criteria and reopen the affected tasks.

3. **Medium — Dependency/Cargo plan drift is real (not just cosmetic).**  
Plan pins `frankensqlite`/FAD revisions and removes patch expectations ([plan.md](/Users/dalecarman/dev/coding_agent_session_search/specs/011-upstream-sync/plan.md:80), [plan.md](/Users/dalecarman/dev/coding_agent_session_search/specs/011-upstream-sync/plan.md:84)).  
Current `Cargo.toml` uses `dd9b457`, `c5d3273c`, and adds a FAD patch section ([Cargo.toml](/Users/dalecarman/dev/coding_agent_session_search/Cargo.toml:36), [Cargo.toml](/Users/dalecarman/dev/coding_agent_session_search/Cargo.toml:74), [Cargo.toml](/Users/dalecarman/dev/coding_agent_session_search/Cargo.toml:159)).  
Action: formalize these as approved plan revisions instead of checklist-pass drift.

4. **Medium — Test coverage is better, but still misses key failure paths.**  
The lock discrimination and dispatch behavior got meaningful tests ([src/watchdog.rs](/Users/dalecarman/dev/coding_agent_session_search/src/watchdog.rs:990), [tests/cli_dispatch_coverage.rs](/Users/dalecarman/dev/coding_agent_session_search/tests/cli_dispatch_coverage.rs:1844)).  
But the riskiest runtime branches still lack direct behavior tests: stale-heartbeat restart path ([src/watchdog.rs](/Users/dalecarman/dev/coding_agent_session_search/src/watchdog.rs:332), [src/watchdog.rs](/Users/dalecarman/dev/coding_agent_session_search/src/watchdog.rs:357)) and kill escalation/permission branches ([src/watchdog.rs](/Users/dalecarman/dev/coding_agent_session_search/src/watchdog.rs:163), only ESRCH case tested at [src/watchdog.rs](/Users/dalecarman/dev/coding_agent_session_search/src/watchdog.rs:752)).  
Action: add deterministic tests for `Restarted`, `kill` EPERM/SIGKILL paths, and a true non-contention `flock` failure branch.

5. **Informational — Commit pattern is good.**  
The code-verify sequence is incremental and legible (R1→R4), with fix+test progression ([/tmp/claude-verify-593f1f27.md](/tmp/claude-verify-593f1f27.md:5), `git log --oneline --reverse 36186ada^..9a7518ed`).

## Adversarial Gate (Required)

6. **3 riskiest code paths and test status**
1. `run_health_check` stale/restart branch ([src/watchdog.rs](/Users/dalecarman/dev/coding_agent_session_search/src/watchdog.rs:332), [src/watchdog.rs](/Users/dalecarman/dev/coding_agent_session_search/src/watchdog.rs:357)): **No direct behavior test for `Restarted` path**.
2. `kill_watcher` signal/escalation/permission handling ([src/watchdog.rs](/Users/dalecarman/dev/coding_agent_session_search/src/watchdog.rs:163)): **Partially tested** (`ESRCH` only at [src/watchdog.rs](/Users/dalecarman/dev/coding_agent_session_search/src/watchdog.rs:752)); no EPERM/SIGKILL-escape coverage.
3. two-stage watchdog CLI dispatch in `execute_cli` ([src/lib.rs](/Users/dalecarman/dev/coding_agent_session_search/src/lib.rs:2752), [src/lib.rs](/Users/dalecarman/dev/coding_agent_session_search/src/lib.rs:3488)): **Yes, tested** parse + subprocess runtime ([tests/cli_dispatch_coverage.rs](/Users/dalecarman/dev/coding_agent_session_search/tests/cli_dispatch_coverage.rs:1801), [tests/cli_dispatch_coverage.rs](/Users/dalecarman/dev/coding_agent_session_search/tests/cli_dispatch_coverage.rs:1844)).

7. **Likely first reviewer objection**  
“Why are tasks/verification marked complete when spec acceptance still requires all lib tests passing and healthy watcher?” ([spec.md](/Users/dalecarman/dev/coding_agent_session_search/specs/011-upstream-sync/spec.md:132), [spec.md](/Users/dalecarman/dev/coding_agent_session_search/specs/011-upstream-sync/spec.md:134), [/tmp/claude-verify-593f1f27.md](/tmp/claude-verify-593f1f27.md:2355)).

8. **What this implementation does NOT handle that plan specified**
1. Strict two-file `src/` delta target ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/011-upstream-sync/tasks.md:286), actual 4-file delta acknowledged at [/tmp/claude-verify-593f1f27.md](/tmp/claude-verify-593f1f27.md:22)).
2. Full `cargo test --lib` pass gate ([plan.md](/Users/dalecarman/dev/coding_agent_session_search/specs/011-upstream-sync/plan.md:259), evidence still 55 fails at [/tmp/claude-verify-593f1f27.md](/tmp/claude-verify-593f1f27.md:44)).
3. Verified healthy post-deploy state gate ([plan.md](/Users/dalecarman/dev/coding_agent_session_search/specs/011-upstream-sync/plan.md:282)); evidence bundle doesn’t show a passing health assertion, and receipt records crash-loop issue ([implement-receipt.md](/Users/dalecarman/dev/coding_agent_session_search/specs/011-upstream-sync/implement-receipt.md:91)).

9. **Are tests testing the right things, or just achieving coverage?**  
Mixed. The new tests are behavior-oriented for dispatch and lock result classification, which is good. But the highest-risk operational paths (restart lifecycle and signal escalation) are still under-tested, so this is not just a coverage problem; it is a missing-risk problem.

## What I Verified

- **Files read**
1. [/tmp/claude-verify-593f1f27.md](/tmp/claude-verify-593f1f27.md:1)  
2. [spec.md](/Users/dalecarman/dev/coding_agent_session_search/specs/011-upstream-sync/spec.md:76)  
3. [plan.md](/Users/dalecarman/dev/coding_agent_session_search/specs/011-upstream-sync/plan.md:18)  
4. [tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/011-upstream-sync/tasks.md:126)  
5. [src/watchdog.rs](/Users/dalecarman/dev/coding_agent_session_search/src/watchdog.rs:254)  
6. [src/lib.rs](/Users/dalecarman/dev/coding_agent_session_search/src/lib.rs:20)  
7. [tests/cli_dispatch_coverage.rs](/Users/dalecarman/dev/coding_agent_session_search/tests/cli_dispatch_coverage.rs:1793)  
8. [Cargo.toml](/Users/dalecarman/dev/coding_agent_session_search/Cargo.toml:1)  
9. [implement-receipt.md](/Users/dalecarman/dev/coding_agent_session_search/specs/011-upstream-sync/implement-receipt.md:1)  
10. [code-verify.md](/Users/dalecarman/dev/coding_agent_session_search/specs/011-upstream-sync/code-verify.md:1)

- **Test files found**
1. 2 implementation-targeted test files in this verification bundle:
`src/watchdog.rs` (inline tests) and `tests/cli_dispatch_coverage.rs` (watchdog dispatch tests).

- **How many tests ran and whether passed**
1. From provided evidence: watchdog `22/22` pass, dispatch `3/3` pass, full suite `3104 pass / 55 fail / 3 ignored` ([/tmp/claude-verify-593f1f27.md](/tmp/claude-verify-593f1f27.md:42), [/tmp/claude-verify-593f1f27.md](/tmp/claude-verify-593f1f27.md:44), [/tmp/claude-verify-593f1f27.md](/tmp/claude-verify-593f1f27.md:2353)).  
2. Independent rerun here: blocked by read-only sandbox (`target/debug/.cargo-lock` permission denied) when executing `~/.cargo/bin/cargo test --lib`.

- **Assumptions tested against source code**
1. Assumption: all 6 watchdog wiring sites exist in `lib.rs` — verified by direct line reads and `rg` hits.
2. Assumption: lock contention vs I/O errors are now type-distinguished — verified in `acquire_lock` + `run_health_check`.
3. Assumption: runtime dispatch is tested, not only parse-time — verified by subprocess test.
4. Assumption: plan’s minimal-delta guard still holds — disproven by actual upstream diff.
5. Assumption: acceptance gates are fully closed — disproven by explicit remaining failure counts and receipt state.

- **Counts/diffs/grep outputs used**
1. `git diff upstream/main --name-only -- src/` => 4 files (`src/daemon/resource.rs`, `src/lib.rs`, `src/search/asset_state.rs`, `src/watchdog.rs`).
2. `git diff --shortstat upstream/main -- src/lib.rs` => `1 file changed, 265 insertions(+), 291 deletions(-)`.
3. `git diff -U0 upstream/main -- src/lib.rs | rg '^@@' | wc -l` => `64`.
4. `rg '^\s*#\[test\]' src/watchdog.rs | wc -l` => `22`.
5. `rg '^fn watchdog_' tests/cli_dispatch_coverage.rs | wc -l` => `3`.
6. `tasks.md` checkbox counts => `37/37` checked, despite unresolved spec gates.

VERDICT: REVISE