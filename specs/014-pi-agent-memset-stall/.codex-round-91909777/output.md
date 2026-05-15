I’d revise. The plan is much stronger than the earlier drafts, but it still has a few implementation-gate holes that could let the team “verify” the wrong binary or lose fidelity guarantees.

**Findings**

1. The plan never proves the fixed binary is the one the watcher and `~/.local/bin/cass` run.
The spec requires one source-of-truth binary through the symlink and daemon path ([spec.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/spec.md:59)). The tasks build `target/release/cass`, then later verify with `~/.local/bin/cass`, but there is no install, symlink target, hash, or version proof between those steps ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/tasks.md:37), [tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/tasks.md:51)). Add an explicit install/symlink verification step before T16.

2. The verification commands are incomplete, and one is likely wrong.
The package name is `coding-agent-search`, not `cass` ([Cargo.toml](/Users/dalecarman/dev/coding_agent_session_search/Cargo.toml:1), [Cargo.toml](/Users/dalecarman/dev/coding_agent_session_search/Cargo.toml:166)), so `cargo test --release -p cass` in T14 is suspect ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/tasks.md:46)). The repo instructions also require `rch exec ... cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, and `cargo fmt --check`; the plan only has `cargo check --bin cass` and a release build ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/tasks.md:37)). Add the mandated check/clippy/fmt/UBS commands.

3. Raw-mirror fidelity can silently break if C4 is selected.
The plan says no raw-mirror changes are required and only linkage must be proven ([plan.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/plan.md:90)). But pre-parse capture depends on `discover_source_files()` returning sources; fallback directory capture is codex-only and returns empty for non-file pi roots ([src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:17209), [src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:17364)). Current FAD Pi implements `discover_source_files()` ([pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:557)). If C4 replaces the connector, the plan must require discover-source parity plus a directory-root raw-mirror regression.

4. The no-data-loss requirement is not explicitly verified.
The spec says the current `pi_agent=33` rows must not be lost ([spec.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/spec.md:61)). T15 snapshots the DB and T18 checks final counts, but no task records existing pi rows before the run and verifies they still exist afterward ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/tasks.md:50), [tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/tasks.md:53)). Add pre/post SQL checks on existing pi conversation IDs or source paths.

5. Skipped-file accounting is underspecified.
Acceptance requires skipped files to be recorded in `quarantine/watch_ingest_poison.jsonl` and reflected in the run receipt ([spec.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/spec.md:66)). The tasks parse success/counts and write a receipt, but do not require reading the quarantine file and reconciling indexed + quarantined against 2,073 discovered files ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/tasks.md:52), [tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/tasks.md:59)).

6. The spec still contradicts itself on acceptance count.
The corrected acceptance says ≥1,970 conversations from 2,073 files ([spec.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/spec.md:66)), but Requirement 3 still says ≥2,800 sessions and `conversations >= 2,500` ([spec.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/spec.md:54)). The plan acknowledges the correction ([plan.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/plan.md:11)), but the requirement text should be amended too.

7. The plan has a branch/process mismatch.
The tasks say commit on `dac/main` and push `origin/dac/main` ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/tasks.md:37), [tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/tasks.md:65)). Current branch is `dac/main`, but the repo instructions say work happens on `main` and ask before committing if already on another branch. This needs an explicit authorization step or plan correction.

**Adversarial Gate**

7. Three riskiest assumptions:
- Assumption 1: the live `_platform_memset` source maps to one of the candidate rows. I verified the plausible FAD scan-time bloat: FAD does `fs::read_to_string` and then `extra: val.clone()` per message ([pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:351), [pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:485)). I did not verify the exact memset frame because I did not run the destructive/live repro in this review.
- Assumption 2: raw mirror can justify dropping pi `extra`. Partially verified: links are conversation-level only ([raw_mirror.rs](/Users/dalecarman/dev/coding_agent_session_search/src/raw_mirror.rs:48)), and capture streams source bytes in 64 KiB chunks ([raw_mirror.rs](/Users/dalecarman/dev/coding_agent_session_search/src/raw_mirror.rs:949)). Not verified for C4 unless discover-source parity is required.
- Assumption 3: verification will test the shipping binary. I verified this is currently unsupported: build and `~/.local/bin/cass` verification are disconnected in tasks.

8. A skeptical senior engineer’s first objection:
“After the source probe already shows FAD `read_to_string` plus per-message `val.clone()`, why is the plan still leaving C3/C1 alive as first-class outcomes instead of forcing C4 or an in-cycle C2 unless profiling proves otherwise?”

9. Production gaps:
The plan does not address DB-row-growth stall detection, automatic RSS guardrails/telemetry, watcher restart cleanup on mid-task failure, raw-mirror retention/security policy, or long-term protection against another connector carrying raw provider JSON into every message.

10. Scope differences:
The plan amends acceptance count but not Requirement 3; it expands possible scope to a cass-owned streaming parser; it stops the watcher during profiling/verification despite the daemon-forward-capture constraint; and it moves upstream PR opening out to a later board even though the spec’s Phase 3 includes upstream PR work.

**What I Verified**

I read the on-disk spec, plan, and tasks; inspected `src/connectors/pi_agent.rs`, `src/indexer/mod.rs`, `src/raw_mirror.rs`, `src/model/conversation_packet.rs`, `src/storage/sqlite.rs`, `src/pages/export.rs`, `tests/connector_pi_agent.rs`, and the pinned FAD pi connector. I counted the live pi corpus with `fd`: 2,073 jsonl files, 1.79 GB on disk via `du -sk`, largest file 71,921,064 bytes. I also confirmed `br ready` is blocked in this sandbox by `Operation not permitted`, so bead state was not verified.

VERDICT: REVISE