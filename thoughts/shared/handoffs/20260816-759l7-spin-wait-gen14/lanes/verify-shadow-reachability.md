# Lane: verify-shadow-reachability (REACHABILITY lens)

Adversarial verifier. Target classification: `compatible-library-behavior-change`,
`blocks_pin: true`, for the open-path rejection of a duplicate legacy
`fts_messages` sqlite_master row.

Lens: REACHABILITY. Can a real database reach this state, and what happens to it
if it does? Correctness and does-it-hold lenses already returned refuted=false.

Append-only. Commands and verbatim output.

---

## 00. Setup

Working dir (shipping checkout):
`/Users/dalecarman/dev/coding_agent_session_search/.claude/worktrees/cass-759l7-spin-wait`

