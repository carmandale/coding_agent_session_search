No blocking findings remain. The last correctness gap is closed: the plan now switches from path-shape heuristics to exact configured-root comparison in [/tmp/claude-plan-7ea3f333.md:362](</tmp/claude-plan-7ea3f333.md:362>), which matches the real `PI_CODING_AGENT_DIR` contract in [pi_agent.rs:40](/Users/dalecarman/Groove Jones Dropbox/Dale Carman/Projects/dev/coding_agent_session_search/src/connectors/pi_agent.rs:40>) and [pi_agent.rs:47](/Users/dalecarman/Groove Jones Dropbox/Dale Carman/Projects/dev/coding_agent_session_search/src/connectors/pi_agent.rs:47>). That removes the false-negative custom-root problem without reopening the remote-root false-positive bug.

**Adversarial Gate**
7. Three riskiest assumptions:
- Exact `Self::home()` / `sessions_dir()` comparison really covers arbitrary `PI_CODING_AGENT_DIR` values. I verified that against [pi_agent.rs:40](/Users/dalecarman/Groove Jones Dropbox/Dale Carman/Projects/dev/coding_agent_session_search/src/connectors/pi_agent.rs:40>), [pi_agent.rs:47](/Users/dalecarman/Groove Jones Dropbox/Dale Carman/Projects/dev/coding_agent_session_search/src/connectors/pi_agent.rs:47>), and the revised snippet in [/tmp/claude-plan-7ea3f333.md:362](</tmp/claude-plan-7ea3f333.md:362>). This now checks out.
- Remote Codex/Factory `sessions` roots will not be accepted by PiAgent after the change. I verified that against remote root fanout in [mod.rs:798](/Users/dalecarman/Groove Jones Dropbox/Dale Carman/Projects/dev/coding_agent_session_search/src/indexer/mod.rs:798>), the real Codex root in [codex.rs:65](/Users/dalecarman/Groove Jones Dropbox/Dale Carman/Projects/dev/coding_agent_session_search/src/connectors/codex.rs:65>), the real Factory root in [factory.rs:42](/Users/dalecarman/Groove Jones Dropbox/Dale Carman/Projects/dev/coding_agent_session_search/src/connectors/factory.rs:42>), and the revised guard in [/tmp/claude-plan-7ea3f333.md:388](</tmp/claude-plan-7ea3f333.md:388>). This now looks correct.
- `2700s` is a safe watchdog threshold. The `1800s` full-scan interval is source-verifiable in [mod.rs:894](/Users/dalecarman/Groove Jones Dropbox/Dale Carman/Projects/dev/coding_agent_session_search/src/indexer/mod.rs:894>); the `~600s` max-scan assumption is not directly verifiable from source, but the plan now treats that as an operational assumption with explicit test/monitoring hooks in [/tmp/claude-plan-7ea3f333.md:510](</tmp/claude-plan-7ea3f333.md:510>) and [/tmp/claude-plan-7ea3f333.md:695](</tmp/claude-plan-7ea3f333.md:695>).

8. A skeptical senior engineer’s first objection would have been the old remote-root contamination risk. The exact configured-root comparison is the right answer to that objection.

9. What the plan still does not address that production will need:
- rollout/update mechanics for the external `watchdog.sh` and launchd plist
- live observation of scan-duration percentiles after deployment to validate the `2700s` assumption

10. Scope now matches cleanly. I did not find a remaining spec/plan drift on the PiAgent root fix, heartbeat threshold, symlink policy, or R3.3 classification.

I re-checked the revised `/tmp` plan plus the live source in [pi_agent.rs](/Users/dalecarman/Groove Jones Dropbox/Dale Carman/Projects/dev/coding_agent_session_search/src/connectors/pi_agent.rs), [mod.rs](/Users/dalecarman/Groove Jones Dropbox/Dale Carman/Projects/dev/coding_agent_session_search/src/indexer/mod.rs), [codex.rs](/Users/dalecarman/Groove Jones Dropbox/Dale Carman/Projects/dev/coding_agent_session_search/src/connectors/codex.rs), and [factory.rs](/Users/dalecarman/Groove Jones Dropbox/Dale Carman/Projects/dev/coding_agent_session_search/src/connectors/factory.rs). The plan is solid and ready to implement.

VERDICT: APPROVED
