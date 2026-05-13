---
title: "cass full-rebuild stalls on raw-mirror archive copy — task list"
date: 2026-05-13
bead: coding_agent_session_search-3vm6
---

<!-- plan:complete:v1 | harness: unknown | date: 2026-05-13T15:22:07Z -->



## Group A — Recovery + operational baseline

- [ ] T1: Update spec.md to amend (a) A2's named invocation from `cass index --full --force-rebuild` to `cass index --full`, (b) explicitly mark the `--force-rebuild` canonical-only short-circuit on non-empty DB as OUT OF SCOPE for this spec (a separate spec will be filed if/when that defect is prioritized), (c) clarify R4's "thread states" wording per T8 — per-thread name, park-or-running flag, per-thread heartbeat counter as the default; OS-level symbolic backtrace as opt-in via `CASS_INDEX_STALL_CAPTURE_LLDB=1`.
- [ ] T2: Unload launchd agents `com.cass.index-watch`, `com.cass.health-watchdog`, `com.cass.sync-to-mini` and verify no `cass index` process remains.
- [ ] T3: WAL-safe pre-backfill snapshot of the canonical DB. After T2 (watcher stopped, no writers): (a) run `sqlite3 "<db_path>" "PRAGMA wal_checkpoint(TRUNCATE);"` to drain the WAL into the main file, (b) APFS clone-copy `agent_search.db`, `agent_search.db-wal`, `agent_search.db-shm` to `*.PRE-BACKFILL-20260513` siblings (the WAL/SHM files may be empty after checkpoint; if absent, document the absence in `recovery.md`), (c) verify `cass stats` against the clone matches against the live file before proceeding.
- [ ] T4: Capture and preserve the watcher's `watch_state.json` to `watch_state.json.PRE-BACKFILL-20260513`. The backfill `cass index --full` run will NOT update `watch_state.json` (it updates DB `last_scan_ts` at `src/indexer/mod.rs:8870`; `watch_state.json` is updated only by the watch path at `src/indexer/mod.rs:16013`). After backfill completes and before re-loading the watcher, T35 in Group H reconciles `watch_state.json` against the post-backfill DB state.
- [ ] T5: Document the recovery procedure in `specs/013-cass-rebuild-stall-asupersync/recovery.md`: T2 stop sequence, T3 WAL-safe snapshot, T4 watch_state capture, then run `CASS_INDEX_STALL_DETECT_SECS=60 cass index --full 2>&1 | tee ~/Library/Logs/cass-backfill-20260513.log`, monitor for `stall_detected` event. On clean completion, T35 reconciles watch_state and re-loads the watcher. On hang, kill the process, capture the structured event for diagnosis (Group C), and either retry or pivot to Surface 5 fallback (`CASS_STREAMING_INDEX=0`).

## Group B — Liveness instrumentation

- [ ] T6: Add a `IndexerLiveness` struct in `src/indexer/mod.rs` carrying the Surface-2 logical pipeline fields. Wire it through `spawn_connector_producer` (line 9130) and the consumer at line 8848.
- [ ] T7: Add a per-thread heartbeat counter (`Arc<AtomicU64>`) registered with `IndexerLiveness` at thread spawn time. Bump on every message handled. Tag each registration with the explicit thread name set via `std::thread::Builder::name()` (replacing the current bare `thread::spawn` at `src/indexer/mod.rs:8406, 9130`). The coordinator compares counters across ticks to derive park-or-running flag and names the stuck thread by its builder name.
- [ ] T8: For R4's "thread states" requirement, define the two-layer interpretation explicitly. Amend spec.md R4 to read: *"thread states (per-thread name, park-or-running flag, and per-thread heartbeat counter; OS-level symbolic backtrace optional via opt-in env var)"* — this makes the always-on logical layer the default that satisfies A4, with OS-level capture as opt-in. The on-watchdog-fire OS capture invokes `lldb -batch -ex "thread apply all bt" -p $(pgrep -f 'cass index')`, gated behind `CASS_INDEX_STALL_CAPTURE_LLDB=1` (default off — lldb attach requires developer mode entitlement on macOS and may need approval in CI). Update the spec amendment task (T1) to include this R4 clarification.
- [ ] T9: Add the heartbeat emitter under `tracing::info!` with target `cass::indexer::liveness`. Cadence 5s, opt-in via RUST_LOG. Verify the heartbeat does not emit anything on the default `RUST_LOG` setting.
- [ ] T10: Unit test for `IndexerLiveness`: synthesize producer + consumer + heartbeat, verify per-tick fields populate correctly and per-thread counters increment monotonically.
- [ ] T11: Build `cass 0.4.2+liveness` from `dac/main` with the new instrumentation. Verify no warnings beyond pre-existing dead-code warnings.

## Group C — Diagnostic localization + root-cause writeup

- [ ] T12: With watcher unloaded, run `RUST_LOG=cass::indexer::liveness=info,info CASS_INDEX_STALL_DETECT_SECS=60 cass index --full` on the user's corpus. Capture log to `~/Library/Logs/cass-backfill-20260513.log`. Observe heartbeat output to identify which liveness surface freezes at the deadlock point.
- [ ] T13: If the deadlock reproduces but is ambiguous between candidates, re-run with `CASS_STREAMING_CONSUMER_COMBINE=0` to isolate flat-combine. Document which candidate the evidence eliminates.
- [ ] T14: If still ambiguous, re-run with `CASS_STREAMING_INDEX=0` (batch mode) — if batch completes successfully, the deadlock is in the streaming pipeline (Surface 4 candidates 2, 3, 4, 5); if batch ALSO hangs, the deadlock is in raw-mirror or post-scan rebuild (candidates 1, 6).
- [ ] T15: With lldb attached (via `CASS_INDEX_STALL_CAPTURE_LLDB=1` from T8) to the next reproducing run, capture symbolic backtraces of all cass-spawned threads at the deadlock moment. Cross-reference with the heartbeat output.
- [ ] T16: Write `specs/013-cass-rebuild-stall-asupersync/root-cause.md` per A1: name the specific sync primitive (file:line), the producer/consumer pair, why the v0.3.7 watchdog did not catch it, and which candidate from Surface 4 is the fix target.

## Group D — Targeted fix

- [ ] T17: Implement the fix selected by T16's root-cause output. The fix is one of:
  - D1: Split `MANIFEST_UPDATE_LOCK` (raw_mirror.rs:22) into per-manifest-id sharded locks
  - D2: Audit and fix `StreamingByteLimiter` predicate-protect contract (mod.rs:8000-8105)
  - D3: Move `t_index.commit()` (mod.rs:8864) to a dedicated thread
  - D4: Restructure `StreamingBatchSender::flush()` reserve-vs-send order (mod.rs:8293)
  - D5: Per-message reservation release in flat-combine (mod.rs:8624)
  - D6: Audit and fix staged lexical merge controller (mod.rs:10344) for hard-zero clamp
- [ ] T18: Run the targeted fix's local regression assertions (cargo test for the affected module) — must pass before continuing.
- [ ] T19: Rebuild `cass 0.4.2+fix`, re-run T12's invocation on the user's corpus, confirm `cass stats` shows ≥98% of expected conversation counts per A3.

## Group E — Watchdog enhancement (R4 + C4)

- [ ] T20: Extend `stall_detected` event payload at `src/lib.rs:72639` with the Surface-2 fields (active producers, channel len/cap, byte-limiter state, consumer phase, raw-mirror lock state, per-thread heartbeat counters at fire time, optional lldb backtrace from T8).
- [ ] T21: Remove the structured-output / progress-events gating at `src/lib.rs:72250, 72531` so the event always emits to the structured log channel.
- [ ] T22: Add sentinel-file write to `<data_dir>/stall-detected.json` at fire time, carrying the full event payload + ISO-8601 timestamp. Use atomic write (write to `.tmp` sibling then `rename`) to avoid wrapper reading a half-written sentinel.
- [ ] T23: Define sentinel lifecycle policy in `scripts/cass-watch-wrapper.sh`:
  - **Stale sentinel detection**: sentinel older than `STALE_SECS` (default 86400 = 24h) is treated as stale, removed, and wrapper proceeds as if absent. Configurable via `CASS_STALL_SENTINEL_STALE_SECS` env var.
  - **Operator-cleared sentinel**: a sibling file `stall-detected.json.cleared` (any contents) signals operator acknowledgement; wrapper removes both files before exec.
  - **Active sentinel**: wrapper sleeps with exponential backoff (start 60s, 2x, cap 900s), emits a `tracing::error` event each cycle including the sentinel's age + payload summary, but does not respawn the indexer.
- [ ] T24: Add a wrapper script `scripts/cass-watch-wrapper.sh` implementing T23's policy. Plus: emits to its own log at `~/Library/Logs/cass-watch-wrapper.log` for operator visibility distinct from cass-index-watch.log.
- [ ] T25: Add `tests/cass-watch-wrapper-test.sh` — POSIX shell harness exercising T23's three cases (no sentinel, fresh sentinel, stale sentinel, cleared sentinel). Runs without invoking real cass (uses a stub binary that exits 0 immediately) and asserts wrapper exit codes + log output.
- [ ] T26: Update `~/Library/LaunchAgents/com.cass.index-watch.plist` to invoke the wrapper. Verify with `launchctl load` + `launchctl unload` cycle. Document the plist diff in `scripts/cass-watch-wrapper.sh` comments.
- [ ] T27: Add `scripts/cass-watch-wrapper-uninstall.sh` per §2.9 (background automation needs an uninstall path). The uninstaller: (a) `launchctl unload` the plist, (b) restore the plist to pre-wrapper form (or remove it entirely if it was newly created), (c) delete the wrapper script and its log, (d) delete any sentinel files, (e) `launchctl load` the restored plist if it exists. Document in the wrapper script's header comment.
- [ ] T27a: Security / privacy hardening for diagnostic artifacts. Liveness logs (Surface 2) and stall-detected sentinel payloads (Group E) may carry: absolute file paths under `~/`, current-source-path values that include connector vendor directories with workspace names, lldb backtraces with stack frames that name source files. Operational requirements:
  - File mode `0600` on `<data_dir>/stall-detected.json` and the wrapper's log; mode `0700` on the data dir itself if not already set. Enforced in the Rust write site (T22) via `fs::OpenOptions::mode(0o600)` and tested in the wrapper unit test (T25).
  - Pre-share redaction: `scripts/cass-diagnostics-redact.sh` filters absolute paths to `<HOME>`, strips PR/workspace names that match the user's known top-level workspace dirs (configurable via `CASS_DIAG_REDACT_PATTERNS`). The wrapper log header carries a warning: "Diagnostic artifacts may include local paths and prompt fragments. Run `cass-diagnostics-redact.sh` before sharing with upstream or external parties."
  - Operator runbook in `recovery.md` calls out the redaction step before any `gh issue comment` or attachment upload to `Dicklesworthstone/coding_agent_session_search`.

## Group F — Regression test

- [ ] T28: Author a focused regression test in `tests/streaming_deadlock_regression.rs` exercising the primitive identified in T16. Verify the test FAILS on the exact baseline commit `29c3672a` (not relative-rev): `git worktree add /tmp/cass-baseline-29c3672a 29c3672a && cd /tmp/cass-baseline-29c3672a && cp -R <test-file>.. && cargo test streaming_deadlock_regression && cd - && git worktree remove /tmp/cass-baseline-29c3672a`. The test source must be portable to the baseline tree (no API calls that didn't exist at 29c3672a) OR the test runs the production-branch binary against fixture data in a way that the baseline binary fails on.
- [ ] T29: Confirm the test PASSES on the post-fix `dac/main` HEAD per A5.
- [ ] T30: Add coverage for the success-path connectors (amp, cursor, factory, gemini, pi_agent) in the same test or an adjacent one — assert that the fix does not regress their ingest behavior per C3.

## Group G — Upstream PR preparation

- [ ] T31: Open or update upstream issue at `Dicklesworthstone/coding_agent_session_search` referencing #196 / #213 / #218 with our root-cause findings and the proposed fix shape.
- [ ] T32: Format the fix commit(s) per upstream conventions: small focused commits, conventional commit prefix (`fix(indexer):` etc.), Rust style matching surrounding code.
- [ ] T33: Push the upstream-ready branch to a fork or directly create the PR. Record the PR URL in `specs/013-cass-rebuild-stall-asupersync/log.md` per A6.
- [ ] T34: Record the implementation provenance: write `specs/013-cass-rebuild-stall-asupersync/implement-receipt.md` summarizing what was done, and run `gate.sh record implement specs/013-cass-rebuild-stall-asupersync/` followed by `gate.sh verify implement` after T28-T30 pass.

## Group H — Post-backfill reconciliation + cutback

- [ ] T35: Reconcile `watch_state.json` against the post-backfill DB state. The DB stores ONE global `meta.last_scan_ts` (`src/storage/sqlite.rs:6244`), not per-connector timestamps; the per-connector `since_ts` map is in `watch_state.json` (`src/indexer/mod.rs:16138`). After Group A's backfill completes:
  - Read the post-backfill global `meta.last_scan_ts` from the DB.
  - Rewrite `<data_dir>/watch_state.json` with every `ConnectorKind` in the schema set to the global `last_scan_ts` value — this is conservative (no connector will pick up files older than the backfill's high-water mark) and uses only data that actually exists.
  - Acceptable alternative: delete `watch_state.json` entirely. On next watcher tick, the watcher initialises a fresh map; on its first scan it discovers everything since the watcher's startup time. Files written between backfill end and watcher reload may need a one-time `cass index --watch --watch-once <connector-path>` per affected connector to pick them up. The recovery.md procedure (T5) documents which path the operator chose.
  - Verify the resulting JSON parses with `cass --version` (which loads the watch_state at startup) and `python3 -c 'import json; json.load(open("watch_state.json"))'`.
- [ ] T36: Re-load launchd agents per T26 sequence: `launchctl load com.cass.index-watch.plist`, `launchctl load com.cass.health-watchdog.plist`, `launchctl load com.cass.sync-to-mini.plist`. Verify via `launchctl list | grep cass` and `pgrep -fla "cass index"`.
- [ ] T37: After 5 minutes of watcher uptime, run `cass status` and confirm "Healthy", date-range upper bound advances past T36's start time, no new `stall_detected` events.
