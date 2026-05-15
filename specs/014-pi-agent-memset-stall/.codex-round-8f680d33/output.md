Almost there, but one verifier is still too weak to approve.

**Finding**

T18 now uses the right top-level path (`type == "message"` and nested `message.content`), but the proposed Python flattener still does not match FAD’s actual Pi flattener. The task script only preserves string content, `{text: ...}`, and list item `.text` values ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/tasks.md:70)). FAD also counts `thinking` blocks and `toolCall` blocks as non-empty searchable content ([pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:156), [pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:158), [pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:164)). Since the spec’s own regression fixture is supposed to include large `toolCall.arguments` and `thinking` content, this simplified verifier can undercount exactly the message shapes we need to protect.

Make T18 require the actual parser path for acceptance, or update the script to mirror FAD’s `flatten_message_content()` exactly: strings, text blocks, thinking blocks as `[Thinking] ...`, tool calls as `[Tool: name] ...`, skip images/unknowns. I would prefer the small Rust harness using the pinned connector, because then C4/FAD parity is tested against the real contract.

Everything else I previously flagged is now addressed: C5 estimator is non-allocating, `rch`/UBS is present, quarantine reconciliation is strict, fixture/test context is corrected, and token-usage preservation is corrected.

VERDICT: REVISE