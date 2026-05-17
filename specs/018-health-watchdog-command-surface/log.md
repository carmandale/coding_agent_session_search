<!-- issue:complete:v1 | harness: unknown | date: 2026-05-17T02:25:00Z -->

2026-05-17 02:24 | - | codex/gpt-5 | /issue | shape-skip: clear follow-up regression discovered during spec 016 recovery; closed spec 007 implemented `cass watchdog run`, but current installed/release binaries return exit 2 `Could not parse arguments`
2026-05-17 02:24 | - | codex/gpt-5 | /issue | bead coding_agent_session_search-2gif2 - spec.md created
2026-05-17 03:34 | - | codex/gpt-5 | local-command-surface-repair | wired existing src/watchdog.rs into CLI/capabilities/robot docs, added watchdog_run_help_dispatches regression, and verified debug binary `cass watchdog run --help` exits 0; launchd/install smoke still not run
2026-05-17 03:45 | - | codex/gpt-5 | local-ubs-critical-cleanup | cleaned existing tests/cli_robot.rs panic! critical inventory caused by touching the large CLI test file; ubs tests/cli_robot.rs and spec018 touched-set UBS both exit 0 with critical=0; launchd/install smoke still not run
2026-05-17 04:00 | - | codex/gpt-5 | release-candidate-proof | rebuilt approval-gated release candidate sha256=a5a139ca503f0f04f2b0baba06e20b16bd7003a7e7d78306cacf94d56eaca9c2; release `cass watchdog run --help` exits 0; install/launchd smoke still not run
