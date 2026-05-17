<!-- Codex Review: APPROVED after 5 rounds | model: gpt-5.3-codex | date: 2026-03-14 -->
<!-- Status: RECONCILED -->
<!-- Revisions: T4 updated with exact Self::home() comparison (replaces is_pi_agent_dir heuristic), T7 threshold 2700s, T7 log rotation uses copytruncate -->
---
title: "tasks: Watcher CPU spin — Shape C implementation"
date: 2026-03-14
bead: coding_agent_session_search-2s40
---

# Tasks

## Phase 1: Instrumentation (C2) — ship first for diagnosability

- [ ] **T1: Cargo.toml strip fix**
  - Change `strip = true` to `strip = "debuginfo"` in `[profile.release]`
  - File: `Cargo.toml`
  - Verify: `cargo build --release`, then `nm target/release/cass | head` shows function names

- [ ] **T2: Name all spawned threads**
  - Replace `thread::spawn` with `thread::Builder::new().name(...).spawn(...)` at 10 locations
  - Files: `src/indexer/mod.rs:120`, `src/lib.rs:5941,6048,7684,7846`, `src/update_check.rs:422`, `src/ui/tui.rs:4912`, `src/search/query.rs:3495`, `src/ui/data.rs:518,531`
  - Handle `io::Result<JoinHandle>` return from `Builder::spawn` (`.expect()` or `?`)
  - Verify: in spawned thread body, `thread::current().name()` returns expected name

- [ ] **T3: Add slow-scan elapsed logging**
  - In `reindex_paths` (src/indexer/mod.rs ~line 1050), wrap `conn.scan()` in `Instant::now()` + `elapsed()`
  - Log `tracing::warn!` if any single connector scan exceeds 30s
  - Verify: write unit test with mock slow connector

## Phase 2: Hygiene (C1) — reduce noise and fix watchdog

- [ ] **T4: Narrow PiAgent detect() root + fix scan() root acceptance**
  - Change `root_paths: vec![home]` to `root_paths: if sessions.exists() { vec![sessions] } else { vec![] }`
  - **CRITICAL:** Replace `is_pi_agent_dir` path-substring heuristic in `scan()` with exact `Self::home()` / `Self::sessions_dir()` comparison (`is_pi_path = ctx.data_dir == pi_home || ctx.data_dir == pi_sessions`). This prevents remote Codex/Factory `sessions` dirs from being accepted while supporting arbitrary `PI_CODING_AGENT_DIR` values.
  - File: `src/connectors/pi_agent.rs:149-160` (detect) and `src/connectors/pi_agent.rs:162-190` (scan entry)
  - Add test: detect returns sessions path (not home)
  - Add test: detect returns empty root_paths when sessions dir absent
  - Add test: scan() works with explicit sessions root via ScanContext::with_roots
  - Add test: scan() with custom PI_CODING_AGENT_DIR explicit root
  - Add test: scan() rejects Codex/Factory sessions dirs via remote roots
  - Depends on: nothing

- [ ] **T5: Bound PiAgent WalkDir**
  - Add `.max_depth(10)` to `WalkDir::new(sessions)` (keep `follow_links(true)`)
  - File: `src/connectors/pi_agent.rs:63`
  - Add test: `session_files_follows_symlinks_with_depth_bound` (create symlink in tempdir)
  - Verify: existing 5 session_files tests still pass
  - Depends on: nothing

- [ ] **T6: Add heartbeat file writing**
  - Add heartbeat write logic to `watch_sources` (src/indexer/mod.rs ~line 899)
  - Write to `<data_dir>/watcher-heartbeat` at: recv_timeout return, before callback, after callback
  - Reduce `heartbeat_interval` from 300s to 60s
  - Pass `data_dir` path into `watch_sources` (currently not available — thread through from `run_index`)
  - Verify: integration test that heartbeat file updates after event loop iteration
  - Depends on: nothing

- [ ] **T7: Update watchdog.sh**
  - Replace `cass health --json` staleness check with heartbeat file age check
  - Heartbeat path: `$HOME/Library/Application Support/com.coding-agent-search.coding-agent-search/watcher-heartbeat`
  - Heartbeat threshold: 2700s (exceeds full_scan_interval 1800s + max scan ~600s + margin 300s)
  - Change kill sequence: SIGTERM → wait 120s → SIGKILL (instead of `launchctl kickstart -k`)
  - Remove `cass index --full` call (fights watcher for tantivy lock, always fails)
  - Add log rotation: copytruncate to 100 MB (`: > "$LOG_FILE"` preserves fd; `mv` fails with launchd)
  - File: `~/.local/share/cass/watchdog.sh` (external, not in repo — provide updated script)
  - Depends on: T6

## Phase 3: SIGTERM handler (C3) — graceful shutdown

- [ ] **T8: Add signal_hook dependency**
  - Add `signal_hook = "*"` to `[dependencies]` in Cargo.toml
  - Verify: `cargo check`
  - Depends on: nothing

- [ ] **T9: Register SIGTERM/SIGINT handler**
  - In `run_index` (src/indexer/mod.rs:551), before `watch_sources` call:
    - Create `Arc<AtomicBool>` shutdown flag
    - Register SIGTERM and SIGINT via `signal_hook::flag::register`
  - Pass shutdown flag to `watch_sources` and `reindex_paths`
  - Depends on: T8

- [ ] **T10: Add shutdown checks in watch_sources**
  - Check `shutdown.load(Ordering::Relaxed)` after every callback call (~lines 913, 928, 935, 961, 965)
  - If set, break out of the loop
  - File: `src/indexer/mod.rs`
  - Depends on: T9

- [ ] **T11: Add shutdown checks in reindex_paths**
  - Add `shutdown: &AtomicBool` parameter to `reindex_paths`
  - Check between connector iterations in `for (kind, root, _ts) in triggers` loop (~line 1022)
  - If set, break early (no final flush needed — each iteration commits independently)
  - File: `src/indexer/mod.rs`
  - Depends on: T9

## Phase 4: Verification & cleanup

- [ ] **T12: Cargo check + clippy + fmt**
  - `cargo check --all-targets`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo fmt --check`
  - Depends on: T1–T11

- [ ] **T13: Run existing test suite**
  - `cargo test`
  - Verify all existing tests pass (especially pi_agent connector tests)
  - Depends on: T12

- [ ] **T14: Manual verification**
  - Build release: `cargo build --release`
  - Verify `nm target/release/cass | rg reindex_paths` shows symbols
  - Verify `sample` output shows named threads
  - Deploy updated watchdog.sh
  - Monitor watcher CPU usage for 30+ minutes
  - Check watchdog log for "heartbeat stale" (should NOT appear)
  - Depends on: T13

- [ ] **T15: Create follow-up bead for C4 spike**
  - Create bead: "Spike: detect tantivy opens-but-spins corruption"
  - Document: existing preflight handles "can't open"; gap is "opens but merge threads spin"
  - Link to this spec (005-watcher-cpu-spin) and R3.3
  - Depends on: T14

## Dependency graph

```
T1 ──┐
T2 ──┤
T3 ──┤
T4 ──┤
T5 ──┼──→ T12 → T13 → T14 → T15
T6 ──┤
T7 ──┤ (depends on T6)
T8 ──┤
T9 ──┤ (depends on T8)
T10 ─┤ (depends on T9)
T11 ─┘ (depends on T9)
```

T1–T6, T8 can all be done in parallel. T7 depends on T6. T9–T11 depend on T8. T12+ are sequential verification.
