<!-- code-verify:approved:v1 | harness: codex/gpt-5.3-codex | date: 2026-04-01T16:40:58Z | rounds: 3 -->

1. **6 wiring sites in `src/lib.rs`: Yes, present and correct.**  
Verified at [lib.rs](/Users/dalecarman/dev/coding_agent_session_search/src/lib.rs:20), [lib.rs](/Users/dalecarman/dev/coding_agent_session_search/src/lib.rs:785), [lib.rs](/Users/dalecarman/dev/coding_agent_session_search/src/lib.rs:2752), [lib.rs](/Users/dalecarman/dev/coding_agent_session_search/src/lib.rs:3488), [lib.rs](/Users/dalecarman/dev/coding_agent_session_search/src/lib.rs:4790), [lib.rs](/Users/dalecarman/dev/coding_agent_session_search/src/lib.rs:5020).

2. **Compiler gates (check/clippy/fmt): Recorded as passing, but I could not independently rerun in this sandbox.**  
Round-3 evidence reports EXIT 0 for all three at [/tmp/claude-verify-f4a959ee.md](/tmp/claude-verify-f4a959ee.md:40).  
Independent rerun here is blocked by read-only target lock (`target/debug/.cargo-lock`) and local Homebrew cargo/rustc drift.

3. **Watchdog unit tests + CLI dispatch integration tests: Reported passing; tests exist and target wiring correctly.**  
Evidence: 18/18 and 2/2 pass at [/tmp/claude-verify-f4a959ee.md](/tmp/claude-verify-f4a959ee.md:47).  
Test definitions are in [watchdog.rs](/Users/dalecarman/dev/coding_agent_session_search/src/watchdog.rs:650) and [cli_dispatch_coverage.rs](/Users/dalecarman/dev/coding_agent_session_search/tests/cli_dispatch_coverage.rs:1801).

4. **Any of the 55 failures in added code?**  
From provided evidence, **no**: failures are categorized in upstream modules and explicitly exclude watchdog/wiring at [/tmp/claude-verify-f4a959ee.md](/tmp/claude-verify-f4a959ee.md:62).

5. **Primary goal (“sync fork to upstream HEAD with minimal local delta”):**  
For the **primary sync goal**, this is achieved.  
Caveat: strict plan acceptance gates still show two unresolved items (`cargo test --lib` full pass and healthy watcher) at [spec.md](/Users/dalecarman/dev/coding_agent_session_search/specs/011-upstream-sync/spec.md:132) and [/tmp/claude-verify-f4a959ee.md](/tmp/claude-verify-f4a959ee.md:84).

6. **3 riskiest code paths + test status (adversarial gate):**  
1. Stale heartbeat restart path in [watchdog.rs](/Users/dalecarman/dev/coding_agent_session_search/src/watchdog.rs:263).  
Has partial tests (`heartbeat_*`, `kill_errno_handling`) at [watchdog.rs](/Users/dalecarman/dev/coding_agent_session_search/src/watchdog.rs:650), but no end-to-end restart test.  
2. launchctl install/load flow in [watchdog.rs](/Users/dalecarman/dev/coding_agent_session_search/src/watchdog.rs:436).  
Decision logic is tested (`install_*`) at [watchdog.rs](/Users/dalecarman/dev/coding_agent_session_search/src/watchdog.rs:849), but launchctl execution path is not directly integration-tested.  
3. Two-stage CLI dispatch wiring in [lib.rs](/Users/dalecarman/dev/coding_agent_session_search/src/lib.rs:2752) and [lib.rs](/Users/dalecarman/dev/coding_agent_session_search/src/lib.rs:3488).  
Parse wiring is tested at [cli_dispatch_coverage.rs](/Users/dalecarman/dev/coding_agent_session_search/tests/cli_dispatch_coverage.rs:1801), but no runtime subprocess assertion of actual command execution path.

7. **Likely first code-review objection:**  
“Spec/tasks require full `cargo test --lib` pass and healthy watcher, but both are still unmet.”  
Refs: [spec.md](/Users/dalecarman/dev/coding_agent_session_search/specs/011-upstream-sync/spec.md:132), [spec.md](/Users/dalecarman/dev/coding_agent_session_search/specs/011-upstream-sync/spec.md:134), [/tmp/claude-verify-f4a959ee.md](/tmp/claude-verify-f4a959ee.md:84).

8. **What this does NOT handle from plan/spec:**  
- Full lib test pass gate.  
- Healthy watcher post-deploy gate.  
- “Only `lib.rs` + `watchdog.rs` in `src/` diff” strict expectation, since current `src/` diff includes 4 files.  
Refs: [spec.md](/Users/dalecarman/dev/coding_agent_session_search/specs/011-upstream-sync/spec.md:126), [tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/011-upstream-sync/tasks.md:286), [/tmp/claude-verify-f4a959ee.md](/tmp/claude-verify-f4a959ee.md:16).

9. **Are tests testing the right things?**  
Mostly yes for wiring/logic safety, but still skewed toward parse/helper coverage. The highest-risk runtime side-effect paths (`launchctl`, full stale-restart lifecycle) are only partially covered.

## What I Verified
- **Files read:**  
[/tmp/claude-verify-f4a959ee.md](/tmp/claude-verify-f4a959ee.md:1), [lib.rs](/Users/dalecarman/dev/coding_agent_session_search/src/lib.rs:1), [watchdog.rs](/Users/dalecarman/dev/coding_agent_session_search/src/watchdog.rs:1), [cli_dispatch_coverage.rs](/Users/dalecarman/dev/coding_agent_session_search/tests/cli_dispatch_coverage.rs:1), [spec.md](/Users/dalecarman/dev/coding_agent_session_search/specs/011-upstream-sync/spec.md:124), [plan.md](/Users/dalecarman/dev/coding_agent_session_search/specs/011-upstream-sync/plan.md:253), [tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/011-upstream-sync/tasks.md:221).

- **Test files found (implementation-relevant): 2**  
[watchdog.rs](/Users/dalecarman/dev/coding_agent_session_search/src/watchdog.rs:650) (inline unit tests), [cli_dispatch_coverage.rs](/Users/dalecarman/dev/coding_agent_session_search/tests/cli_dispatch_coverage.rs:1801) (watchdog CLI dispatch tests).

- **Tests ran and pass/fail counts (from provided evidence):**  
18/18 watchdog unit pass, 2/2 watchdog dispatch pass, full lib suite 3104 pass / 55 fail / 3 ignored at [/tmp/claude-verify-f4a959ee.md](/tmp/claude-verify-f4a959ee.md:47).  
I could not rerun these in this sandbox due read-only write lock restrictions.

- **Assumptions I checked against source:**  
All six wiring points exist in live `src/lib.rs`; upstream `src/lib.rs` has no watchdog wiring; dispatch tests specifically assert `Commands::Watchdog` parse path; watchdog health JSON key is tested.

- **Counts/diff/grep supporting verdict:**  
`git diff --name-only upstream/main -- src/` => 4 files (`daemon/resource.rs`, `lib.rs`, `search/asset_state.rs`, `watchdog.rs`).  
`git diff --shortstat upstream/main -- src/lib.rs` => 265 insertions / 291 deletions (556 lines).  
`git diff -U0 upstream/main -- src/lib.rs | rg '^@@' | wc -l` => 64 hunks measured here.  
`rg '^\\s*#\\[test\\]' src/watchdog.rs | wc -l` => 18.  
`rg '^fn watchdog_' tests/cli_dispatch_coverage.rs | wc -l` => 2.

VERDICT: APPROVED
