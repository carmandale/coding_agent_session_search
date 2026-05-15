<!-- codex-review:approved:v1 | harness: codex/gpt-5.3-codex | date: 2026-05-15T16:40:55Z | rounds: 5 -->

# /codex-review for specs/014-pi-agent-memset-stall/

## Round 1

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

## Round 2

Still needs a small revision. The big structural fixes landed, but a few load-bearing details are still off.

**Findings**

1. T10 still does not match the repo’s required verification contract.
The revised task uses plain `cargo check`, `cargo clippy`, `cargo fmt`, and `cargo build` ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/tasks.md:37)). The repo requires Codex to use `rch exec -- env CARGO_TARGET_DIR=...` for check/clippy/fmt, and UBS before commit ([AGENTS.md](/Users/dalecarman/dev/coding_agent_session_search/AGENTS.md:202), [AGENTS.md](/Users/dalecarman/dev/coding_agent_session_search/AGENTS.md:221), [AGENTS.md](/Users/dalecarman/dev/coding_agent_session_search/AGENTS.md:981)). Fix T10/T14 to name the exact `rch exec` commands and `ubs $(git diff --name-only --cached)` or equivalent changed-file scope.

2. Quarantine reconciliation still allows silent misses.
Acceptance says any skipped files must be recorded in `watch_ingest_poison.jsonl` and reflected in the receipt ([spec.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/spec.md:66)). T18 allows an “unexplained gap > 10” before escalation ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/tasks.md:61)). That violates the spec. It should build three source-path sets: discovered pi jsonl files, DB `source_path`s, and quarantine `source_path`s; the unaccounted set must be empty or every exception must be explicitly classified in the receipt. Also handle absent quarantine file as count zero instead of `cat` failing.

3. C5 still risks allocating while trying to measure allocation.
The C5 plan estimates `message.extra` by serialized size ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/tasks.md:35)). Given the suspected bad shape is full raw JSON cloned into every `message.extra` ([pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:485)), serializing those values for estimation can itself allocate heavily. Require a non-allocating recursive `serde_json::Value` byte estimator or a counting writer, not `to_vec`/`to_string`.

4. The message-coverage check is underspecified.
T18 says compare against “source jsonl line count” ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/tasks.md:59)), but Pi files include `session`, `model_change`, `thinking_level_change`, malformed/skipped lines, and empty-content messages. FAD only pushes `"message"` entries with non-empty flattened content ([pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:378), [pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:422)). Define the expected count as “message records that the connector contract should index,” not total JSONL lines.

5. Two stale factual notes remain.
The plan still says the codex compactor does not preserve token usage ([plan.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/plan.md:85)), but it clones existing `cass` metadata and the test asserts `cass.token_usage` survives ([src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:17577), [src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:34337)). It also says there is “one 3-line pi fixture” ([plan.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/plan.md:74)), while the fixture is 8 lines and there is already a 1000-message connector test ([tests/connector_pi_agent.rs](/Users/dalecarman/dev/coding_agent_session_search/tests/connector_pi_agent.rs:997)). Not fatal, but clean it before implementation so the implementer does not chase stale context.

**What I Verified**

I re-read the on-disk revised `spec.md`, `plan.md`, and `tasks.md`, not just the pasted text. I checked the updated source-count requirement, the new C4 discover-source parity language, the binary install step, the no-data-loss baseline, and the quarantine reconciliation. I also re-checked the FAD Pi parser and current compactor tests for the remaining claims above.

VERDICT: REVISE
## Round 3

One real blocker remains.

**Finding**

T18’s message-coverage verifier counts the wrong Pi fields. The task says to count records with `kind == "message"` and top-level `content` ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/tasks.md:66)). The actual FAD Pi parser uses top-level `type == "message"`, then reads nested `message.content`, flattens it, and skips only if that flattened content is empty ([pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:376), [pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:402), [pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:416)). On the real fixture, the proposed script would count zero messages because the records have `type`, not `kind`, and content is nested under `message`.

Fix T18 to either call the actual Pi parser for expected counts, or implement the same logic: top-level `type == "message"`, require `message` object, flatten `message.content` using the connector’s rules, then count non-empty flattened messages.

**Minor cleanup**

T14 still uses plain `cargo test --release` rather than the repo’s `rch exec -- env CARGO_TARGET_DIR=... cargo test ...` pattern ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/tasks.md:53), [AGENTS.md](/Users/dalecarman/dev/coding_agent_session_search/AGENTS.md:343)). I would clean that while touching the task.

**What I Verified**

I re-read the revised on-disk `plan.md` and `tasks.md`. The previous blockers around C5 non-allocating estimation, UBS/check/clippy/fmt, fixture correction, token-usage correction, and strict quarantine reconciliation are now addressed. The remaining issue is the Pi message-count script.

VERDICT: REVISE
## Round 4

Almost there, but one verifier is still too weak to approve.

**Finding**

T18 now uses the right top-level path (`type == "message"` and nested `message.content`), but the proposed Python flattener still does not match FAD’s actual Pi flattener. The task script only preserves string content, `{text: ...}`, and list item `.text` values ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/tasks.md:70)). FAD also counts `thinking` blocks and `toolCall` blocks as non-empty searchable content ([pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:156), [pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:158), [pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:164)). Since the spec’s own regression fixture is supposed to include large `toolCall.arguments` and `thinking` content, this simplified verifier can undercount exactly the message shapes we need to protect.

Make T18 require the actual parser path for acceptance, or update the script to mirror FAD’s `flatten_message_content()` exactly: strings, text blocks, thinking blocks as `[Thinking] ...`, tool calls as `[Tool: name] ...`, skip images/unknowns. I would prefer the small Rust harness using the pinned connector, because then C4/FAD parity is tested against the real contract.

Everything else I previously flagged is now addressed: C5 estimator is non-allocating, `rch`/UBS is present, quarantine reconciliation is strict, fixture/test context is corrected, and token-usage preservation is corrected.

VERDICT: REVISE
## Round 5

No blocking issues remain.

I re-checked the on-disk `tasks.md` and the last weak verifier is fixed: T18 now requires a small Rust harness using the pinned FAD `PiAgentConnector`, rather than a hand-rolled Python flattener, so message coverage is checked against the actual parser contract ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/tasks.md:66)). The earlier blockers are also covered: non-allocating C5 estimate, `rch`/UBS verification, install-path proof for `~/.local/bin/cass`, strict quarantine reconciliation, no-data-loss baseline, and C4 discover-source parity.

Residual note: the live profiling and full-corpus proof still have to be executed during implementation, but the plan now requires the right evidence and gates.

VERDICT: APPROVED