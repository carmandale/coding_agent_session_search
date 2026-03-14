# Planning Transcript: 005-watcher-cpu-spin

**Date:** 2026-03-14
**Participants:** QuickLion (proposer, pi/claude-sonnet-4-20250514), LoudUnion (challenger, crew-challenger/claude-opus-4-6)
**Spec:** specs/005-watcher-cpu-spin/spec.md
**Bead:** coding_agent_session_search-2s40

---

## Phase: Research (QuickLion)

QuickLion performed deep codebase research across all Shape C components:

### C1 research:
- **A1:** PiAgent `detect()` at pi_agent.rs:149 returns `root_paths: vec![home]` (entire `~/.pi/agent`). Codex returns `sessions` subdir. Change to `root_paths: vec![home.join("sessions")]` with guard for non-existent sessions dir.
- **A2:** PiAgent `session_files()` at pi_agent.rs:63 uses `follow_links(true)` with no `max_depth`. 5 existing tests cover session_files.
- **A3:** Log managed by launchd plist `StandardOutPath`. No in-code rotation. Watchdog.sh is best insertion point.
- **A4:** Watchdog at `~/.local/share/cass/watchdog.sh` — external, not in repo. Uses `cass health --json` with 600s threshold. Watch state lives in `<data_dir>/watch_state.json`.

### C2 research:
- **B1:** Cargo.toml `[profile.release]` has `strip = true, panic = "abort", opt-level = "z"`.
- **B2:** Found 10 `thread::spawn` locations (7 production, 3 test-only). None use `thread::Builder::new().name(...)`.
- **B3:** `reindex_paths` at mod.rs:999. Per-connector scan at ~line 1063. `watch_scan` tracing at line 1082.

### C3 research:
- Tokio has `signal` feature but it's never used.
- `watch_sources` runs a blocking event loop on a spawned thread (not tokio).
- `ctrlc`, `signal_hook` not in dependencies.

### C4 research:
- Existing preflight at mod.rs:575-585 catches "can't open" → force rebuild.
- Lines 589-591 delete index dir on rebuild.
- Gap: "opens but behaves badly" not caught.

---

## Phase: Challenge (LoudUnion — 7 issues, 2 blockers)

### BLOCKER 1: A2 breaks clawdbot indexing
- Verified: symlink at `~/.pi/agent/sessions/--clawdbot-chip--` → `~/.clawdbot/agents/main/sessions` contains 2,098 .jsonl files
- Removing `follow_links(true)` silently drops all clawdbot sessions
- Proposed **Option B:** keep `follow_links(true)` + add `max_depth(10)` — bounds traversal without breaking known symlink

### BLOCKER 2: B1 strip analysis backwards
- `strip = "symbols"` strips symbol table — wrong direction
- `strip = "debuginfo"` keeps symbol table, strips DWARF — correct for `sample` output
- `strip = "none"` keeps everything — largest binary

### CONCERN 3: SIGTERM delayed during scans
- `reindex_paths` callback runs synchronously — SIGTERM flag not checked during scan
- Proposed: check flag between connector iterations in reindex_paths (30s granularity) + after every callback in watch_sources
- Key insight: no final `t_index.commit()` needed — each connector iteration commits independently

### CONCERN 4: A1 detect() non-existent sessions dir
- If sessions dir doesn't exist, `watcher.watch()` fails with warning
- Solution: return empty `root_paths` when sessions absent, keep `detected: true`

### CONCERN 5: A4 heartbeat path discovery
- Watchdog.sh needs to find heartbeat file without running Rust code
- Resolution: `<data_dir>/watcher-heartbeat` with macOS path hardcoded in watchdog.sh

### CONCERN 6: C4 deferral vs R3.3 Must-have
- Existing preflight already handles "can't open" corruption
- Gap (opens-but-spins) deferred to spike with follow-up bead
- R3.3 partially met by existing code + C3 preventing corruption

### CONCERN 7: No test plan
- Added testing strategy: unit tests for A1, A2, B2, B3, C3; integration test for A4

---

## Phase: Revision (QuickLion)

All 7 issues addressed:
1. **A2:** Option B accepted — keep follow_links + max_depth(10), add symlink test
2. **B1:** `strip = "debuginfo"` — correct
3. **SIGTERM:** Two-level checks (reindex_paths between connectors + watch_sources after callbacks)
4. **A1:** Empty root_paths guard when sessions absent
5. **Heartbeat:** `<data_dir>/watcher-heartbeat`, macOS path hardcoded
6. **C4:** Document existing preflight, defer spike
7. **Tests:** Coverage per component

---

## Phase: Final Challenge (LoudUnion — 2 remaining)

### recv_timeout race with SIGTERM
- If SIGTERM arrives 1s into recv_timeout(300s), flag not checked for 299s
- Watchdog SIGKILL at 120s would fire before check
- Resolution: reduce heartbeat_interval to 60s → recv_timeout is 60s max → within 120s SIGKILL grace

### Heartbeat path still vague
- Multiple options mentioned in one paragraph
- Resolution: single decision — `<data_dir>/watcher-heartbeat`, hardcoded macOS path in watchdog.sh

---

## Phase: Agreement

LoudUnion confirmed all issues resolved. Key verification points:
- Blocker 1 (clawdbot): Option B preserves 2,098 sessions ✅
- Blocker 2 (strip): `strip = "debuginfo"` correct for `sample` ✅
- SIGTERM: two-level AtomicBool + 60s heartbeat interval ✅
- Heartbeat: single path, hardcoded, consistent with watch_state.json location ✅
- C4: existing preflight documented, spike deferred with bead ✅
- Tests: coverage for all components ✅

Plan.md and tasks.md written with all revisions incorporated.
