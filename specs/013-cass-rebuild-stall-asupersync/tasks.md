---
title: "cass full-rebuild stalls on raw-mirror archive copy — task list"
date: 2026-05-13
bead: coding_agent_session_search-3vm6
---

<!-- Codex Review: APPROVED after 5 rounds | model: gpt-5.3-codex | date: 2026-05-13 | trust_level: full | round_records: .codex-round-893bb04a/ -->


<!-- plan:complete:v1 | harness: unknown | date: 2026-05-13T15:22:07Z -->



## Group A — Recovery + operational baseline

- [x] T1: Spec.md A2 + R4 amendments completed at plan creation time (committed in `ed590b9b`+revisions): A2 names plain `cass index --full`; A2's note explicitly marks `--force-rebuild`'s canonical-only short-circuit OUT OF SCOPE; R4 now requires lldb backtrace for diagnostic runs that triggered the stall, with heartbeat counters as the always-on logical layer. This task is bookkeeping — verify the amendments are present in spec.md before proceeding to T2.
- [x] T2: Unload launchd agents `com.cass.index-watch`, `com.cass.health-watchdog`, `com.cass.sync-to-mini` and verify no `cass index` process remains.
- [x] T3: WAL-safe pre-backfill snapshot of the canonical DB. After T2 (watcher stopped, no writers): (a) run `sqlite3 "<db_path>" "PRAGMA wal_checkpoint(TRUNCATE);"` to drain the WAL into the main file, (b) APFS clone-copy `agent_search.db`, `agent_search.db-wal`, `agent_search.db-shm` to `*.PRE-BACKFILL-20260513` siblings (the WAL/SHM files may be empty after checkpoint; if absent, document the absence in `recovery.md`), (c) verify `cass stats` against the clone matches against the live file before proceeding.
- [x] T4: Capture and preserve the watcher's `watch_state.json` to `watch_state.json.PRE-BACKFILL-20260513`. The backfill `cass index --full` run will NOT update `watch_state.json` (it updates DB `last_scan_ts` at `src/indexer/mod.rs:8870`; `watch_state.json` is updated only by the watch path at `src/indexer/mod.rs:16013`). After backfill completes and before re-loading the watcher, T35 in Group H reconciles `watch_state.json` against the post-backfill DB state.
- [x] T5: Document the recovery procedure in `specs/013-cass-rebuild-stall-asupersync/recovery.md`. Recovery invocation MUST use `--json` so the `stall_detected` watchdog is armed (the watchdog only emits in structured-output mode per `src/lib.rs:72250, 72531`). Command: `CASS_INDEX_STALL_DETECT_SECS=60 cass index --full --json 2>&1 | tee ~/Library/Logs/cass-backfill-20260513.log`. **The Group E watchdog-gating fix (T19) MUST land before this recovery is attempted on the live corpus**, OR the recovery run must use `--json` as documented here. On clean completion, T35 reconciles watch_state and re-loads the watcher. On hang, kill the process, capture the structured event for diagnosis (Group C), and either retry or pivot to Surface 5 fallback (`CASS_STREAMING_INDEX=0 cass index --full --json`).

## Group B — Liveness instrumentation

- [ ] T6: Add a `IndexerLiveness` struct in `src/indexer/mod.rs` carrying the Surface-2 logical pipeline fields. Wire it through `spawn_connector_producer` (line 9130) and the consumer at line 8848.
- [ ] T7: Add a per-thread heartbeat counter (`Arc<AtomicU64>`) registered with `IndexerLiveness` at thread spawn time. Bump on every message handled. Tag each registration with the explicit thread name set via `std::thread::Builder::name()` (replacing the current bare `thread::spawn` at `src/indexer/mod.rs:8406, 9130`). The coordinator compares counters across ticks to derive park-or-running flag and names the stuck thread by its builder name.
- [ ] T8: For R4's "thread states" requirement, define the two-layer interpretation explicitly. Amend spec.md R4 to read: *"thread states (per-thread name, park-or-running flag, and per-thread heartbeat counter; OS-level symbolic backtrace optional via opt-in env var)"* — this makes the always-on logical layer the default that satisfies A4, with OS-level capture as opt-in. The on-watchdog-fire OS capture invokes `lldb -batch -ex "thread apply all bt" -p $(pgrep -f 'cass index')`, gated behind `CASS_INDEX_STALL_CAPTURE_LLDB=1` (default off — lldb attach requires developer mode entitlement on macOS and may need approval in CI). Update the spec amendment task (T1) to include this R4 clarification.
- [ ] T9: Add the heartbeat emitter under `tracing::info!` with target `cass::indexer::liveness`. Cadence 5s, opt-in via RUST_LOG. Verify the heartbeat does not emit anything on the default `RUST_LOG` setting.
- [ ] T10: Unit test for `IndexerLiveness`: synthesize producer + consumer + heartbeat, verify per-tick fields populate correctly and per-thread counters increment monotonically.
- [ ] T11: Build `cass 0.4.2+liveness` from `dac/main` with the new instrumentation. Verify no warnings beyond pre-existing dead-code warnings.

## Group C — Diagnostic localization + root-cause writeup

- [ ] T12: With watcher unloaded, run `RUST_LOG=cass::indexer::liveness=info,info CASS_INDEX_STALL_DETECT_SECS=60 cass index --full --json` on the user's corpus (the `--json` flag is REQUIRED until Group E T21 lands; without it the `stall_detected` watchdog stays silent per `src/lib.rs:72250, 72531`). Capture log to `~/Library/Logs/cass-backfill-20260513.log`. Observe heartbeat output to identify which liveness surface freezes at the deadlock point.
- [ ] T13: If the deadlock reproduces but is ambiguous between candidates, re-run with `CASS_STREAMING_CONSUMER_COMBINE=0` to isolate flat-combine. Document which candidate the evidence eliminates.
- [ ] T14: If still ambiguous, re-run with `CASS_STREAMING_INDEX=0` (batch mode) — if batch completes successfully, the deadlock is in the streaming pipeline (Surface 4 candidates 2, 3, 4, 5); if batch ALSO hangs, the deadlock is in raw-mirror or post-scan rebuild (candidates 1, 6).
- [ ] T15: REQUIRED for any diagnostic run that reproduces the stall: capture lldb symbolic backtraces of all cass-spawned threads at the deadlock moment via `lldb -batch -ex "thread apply all bt" -p <pid>`. This satisfies R4's "thread states" for the diagnostic path. Set `CASS_INDEX_STALL_CAPTURE_LLDB=1` so the watchdog also captures the backtrace at fire time automatically. Cross-reference with the heartbeat output from Surface 2.
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
- [ ] T19a: Per-file reconciliation ledger to satisfy A2's "every source file ingested or structured failure reason." Implementation pieces:
  1. **Add a structured skip ledger** in the indexer. The current code has no `skip_reason` field on `RawMirrorManifestFile` (`src/raw_mirror.rs:94`) and no durable per-source ingest-skip record. Add NDJSON output at `<data_dir>/ingest-skipped.ndjson` written by the producer when a source file is encountered but not ingested. Each line: `{source_path, agent, skip_reason, ts}`. Reasons: `empty-file`, `parse-error`, `encoding-error`, `permission-denied`, `not-conversation-shape`. This is small new infrastructure (one file write, one struct, one call site in `scan_with_callback`).
  2. **Pre-backfill inventory**: capture source-file list for claude_code (`~/.claude/projects/**/*.jsonl`), codex (`~/.codex/sessions/**/*.jsonl`), openclaw (`~/.openclaw/**/*.jsonl`), opencode (`~/.opencode/**/*.{jsonl,json}`).
  3. **Post-backfill DB query**: `(source_path, conversation_id)` pairs per agent from the canonical DB.
  4. **Classify each pre-backfill path** as: (a) ingested (matched in DB), (b) structured skip (matched in `ingest-skipped.ndjson`), (c) silent loss (no entry in either).
  5. **Write the ledger** to `specs/013-cass-rebuild-stall-asupersync/reconciliation-ledger.md` with row-per-file table. **A2 acceptance requires zero files in class (c).**

## Group E — Watchdog enhancement (R4 + C4)

Scope: indexer's watchdog emission + the minimal plist mutation needed for C4 (boolean `KeepAlive` → dictionary `{SuccessfulExit=false}`). Wrapper scripts and uninstaller automation are out of scope.

- [ ] T20: Extend `stall_detected` event payload at `src/lib.rs:72639` with the Surface-2 fields (active producers, channel len/cap, byte-limiter state, consumer phase, raw-mirror lock state, per-thread heartbeat counters at fire time, lldb backtrace when `CASS_INDEX_STALL_CAPTURE_LLDB=1`).
- [ ] T21: Remove the structured-output / progress-events gating at `src/lib.rs:72250, 72531` so the `stall_detected` event always emits regardless of `--json` / `--robot` flags. This is the load-bearing fix for Group A's recovery — without it the recovery command silently hangs even with `CASS_INDEX_STALL_DETECT_SECS=60` set.
- [ ] T22: Add sentinel-file write to `<data_dir>/stall-detected.json` at fire time, carrying the full event payload + ISO-8601 timestamp. Use atomic write (`.tmp` + `fs::rename`). File mode 0600.
- [ ] T23: C4 compliance — change the plist's `KeepAlive` from boolean true to dictionary `{SuccessfulExit=false}` form, and have the indexer exit with status 0 when it detects an existing `stall-detected.json` sentinel on startup. Dictionary `KeepAlive={SuccessfulExit=false}` treats exit-0 as terminal (no respawn) and exit-non-zero as transient (respawn). This avoids both the respawn loop and the wrapper-script scope creep. Plist diff is small (boolean → dict) and reversible. Recovery.md documents the manual revert path: edit plist back to `KeepAlive=true` boolean.
- [ ] T24: Add `scripts/cass-diagnostics-redact.sh` — pre-share redaction filter that strips absolute paths to `<HOME>` and known workspace prefixes from `stall-detected.json` and indexer logs before upstream sharing. Wrapper log header carries a warning: "Diagnostic artifacts may include local paths and prompt fragments. Run `cass-diagnostics-redact.sh` before sharing with upstream."
- [ ] T25: Document in `recovery.md`: redaction is required before any `gh issue comment` or attachment upload to `Dicklesworthstone/coding_agent_session_search`.

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
  - Move `<data_dir>/watch_state.json` aside to `watch_state.json.POST-BACKFILL-<DATE>` (rename only — never delete; project rule §2 forbids deletion without written permission).
  - Read the post-backfill global `meta.last_scan_ts` from the DB.
  - Write a new `<data_dir>/watch_state.json` with every `ConnectorKind` in the schema set to the global `last_scan_ts` value — this is conservative (no connector will pick up files older than the backfill's high-water mark) and uses only data that actually exists.
  - Verify the resulting JSON parses with `python3 -c 'import json; json.load(open("watch_state.json"))'` and that `cass status` reads it without error.
  - If the operator needs to wipe to "fresh" semantics instead, the rename-aside file in step 1 is the rollback path; they explicitly request the wipe via a separate documented procedure in `recovery.md`.
- [ ] T36: Verify and load the C4-amended plist. Steps: (a) `plutil -p ~/Library/LaunchAgents/com.cass.index-watch.plist` and confirm `KeepAlive` is a dictionary `{SuccessfulExit=0}` per T23, (b) `launchctl bootout gui/$(id -u)/com.cass.index-watch` (idempotent — succeeds if already unloaded), (c) `launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/com.cass.index-watch.plist`, (d) for health-watchdog and sync-to-mini plists, repeat bootout+bootstrap. Confirm via `launchctl list | grep cass` shows all three with non-empty PID.
- [ ] T36a: C4 acceptance test. With watcher running, simulate stall: write a synthetic `<data_dir>/stall-detected.json` (atomic write, mode 0600), then `launchctl kill TERM gui/$(id -u)/com.cass.index-watch`. Observe: (a) indexer exits with status 0 (logged in cass-index-watch.log), (b) launchd does NOT respawn (confirmed by `launchctl list | grep com.cass.index-watch` showing PID `-` and no new process within 60s), (c) operator-clear procedure (delete the synthetic sentinel manually with permission, then `launchctl kickstart`) re-starts the watcher cleanly.
- [ ] T37: After 5 minutes of watcher uptime (post T36a teardown of the synthetic sentinel), run `cass status` and confirm "Healthy", date-range upper bound advances past T36's start time, no new `stall_detected` events.
