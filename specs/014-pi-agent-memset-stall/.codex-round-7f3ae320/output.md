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