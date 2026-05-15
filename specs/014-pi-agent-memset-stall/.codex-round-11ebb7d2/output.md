One real blocker remains.

**Finding**

T18’s message-coverage verifier counts the wrong Pi fields. The task says to count records with `kind == "message"` and top-level `content` ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/tasks.md:66)). The actual FAD Pi parser uses top-level `type == "message"`, then reads nested `message.content`, flattens it, and skips only if that flattened content is empty ([pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:376), [pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:402), [pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:416)). On the real fixture, the proposed script would count zero messages because the records have `type`, not `kind`, and content is nested under `message`.

Fix T18 to either call the actual Pi parser for expected counts, or implement the same logic: top-level `type == "message"`, require `message` object, flatten `message.content` using the connector’s rules, then count non-empty flattened messages.

**Minor cleanup**

T14 still uses plain `cargo test --release` rather than the repo’s `rch exec -- env CARGO_TARGET_DIR=... cargo test ...` pattern ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/tasks.md:53), [AGENTS.md](/Users/dalecarman/dev/coding_agent_session_search/AGENTS.md:343)). I would clean that while touching the task.

**What I Verified**

I re-read the revised on-disk `plan.md` and `tasks.md`. The previous blockers around C5 non-allocating estimation, UBS/check/clippy/fmt, fixture correction, token-usage correction, and strict quarantine reconciliation are now addressed. The remaining issue is the Pi message-count script.

VERDICT: REVISE