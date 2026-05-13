---
title: "cass full-rebuild stalls on raw-mirror archive copy (asupersync worker deadlock)"
date: 2026-05-13
bead: coding_agent_session_search-3vm6
---

<!-- Codex Review: APPROVED after 5 rounds | model: gpt-5.3-codex | date: 2026-05-13 | trust_level: full | round_records: .codex-round-893bb04a/ -->


<!-- issue:complete:v1 | harness: unknown | date: 2026-05-13T14:17:15Z -->

## Source (verbatim)

> "I would like to be in sync with upstream. and has cass been grabbing all sessions? or has it stopped/failed?" — user, 2026-05-12

> "it should be my main is in sync with upstream, and then we try to merge to our latest branch [...]. the goal is to be in sync with upstream and running properly, capturing all sessions and cass working. that is the /goal" — user, 2026-05-12

> "the whole point is to have historical data. it sounds like you need to debug why it stalled. should we create an /issue for this?" — user, 2026-05-13

## Problem

After fresh-installing upstream cass v0.4.2 (commit `29c3672a` on `origin/main`, synced from `upstream/main`) on 2026-05-12, `cass index --full --force-rebuild` stalled partway through ingesting historical session data. The stalled process held the index-run lock and consumed 100% CPU for over 11 hours while making no forward progress.

Concrete state at point of stall:

- Process: `cass index --full --force-rebuild`, PID 21824, etime 10h+, 100% CPU, 8.3 GB RSS.
- All 16 `asupersync-worker-*` threads parked in `_pthread_cond_wait` / `__psynch_cvwait`.
- Main thread parked in `nanosleep` / `__semwait_signal`.
- `lsof` on the process showed no open session source files (no `.jsonl`, no `.vscdb`); only the binary itself and one in-progress raw-mirror manifest temp file.
- `raw-mirror/v1/blobs/` growth: 0 bytes in a 10-second sample window.
- `agent_search.db-wal` size frozen at 167.6 MB → 511.5 MB across the 11h window with multi-hour gaps of no writes.
- 0 new log lines emitted for 25+ minute stretches; only periodic `pricing coverage` summaries.

Completed before stall:

- `streaming_scan_complete` fired for 15 connectors (aider, amp, chatgpt, clawdbot, cline, copilot, copilot_cli, crush, cursor, factory, gemini, hermes, kimi, qwen, vibe).
- `streaming_ingest` events landed for amp (33 conv), cursor (242), factory (66), gemini (44), pi_agent (532).
- DB reached 1,648 conversations / 129,954 messages (out of an expected ~21,635 conversations based on the pre-restart DB).
- raw-mirror grew to 25 GB then froze.

Never started:

- `streaming_scan_complete` never fired for the four bulk connectors: claude_code, codex, openclaw, opencode. These hold the bulk of the historical corpus — by file count on disk:
  - claude_code: 2,561 `.jsonl` files (DB has 413 → 2,148 missing, 84%)
  - codex: 3,991 `.jsonl` files (DB has 84 → 3,907 missing, 98%)
  - openclaw: 11,018 `.jsonl` files (DB has 193 from one of seven agents → ~10,825 missing, 98%)
  - opencode: 19 files (DB has 79; appears complete or close to it)
- Net: ~17,000 historical sessions blocked from backfill by the stall.

After manual kill (`SIGTERM` → graceful), the canonical DB (1,648 conv) was durable. The watcher path (`cass index --watch`, loaded via launchd) restarted cleanly, rebuilt the Tantivy lexical index in ~3 hours, and now captures *new* sessions live — but it does not backfill historical files. The watcher initializes its per-`ConnectorKind` `since_ts` high-water marks to startup time when `watch_state.json` is fresh or empty, so files older than startup are out of scope.

This blocks the user's stated /goal: capture **all** sessions, including historical claude_code / codex / openclaw archives that represent the bulk of their agent activity from 2025-09-15 onward.

## Requirements

R1. Reach a root-cause diagnosis for the asupersync worker deadlock during `cass index --full`. Diagnosis must be concrete enough to answer: which queue / channel / mutex held the threads, which producer side failed to signal, and why the v0.3.7 stall watchdog did not emit a `stall_detected` event (or did, and was missed).

R2. Implement a fix or confirmed workaround that lets a single foreground run ingest the full historical corpus (~17,000 conversations across claude_code, codex, openclaw, opencode) to completion without manual intervention. "Completion" means: every source file on disk for those connectors is either ingested into the canonical DB or has a recorded structured failure reason.

R3. Preserve the partial work already on disk: do not require throwing away the current 1,648-conversation canonical DB or the 25 GB raw-mirror archive. The fix should pick up where the killed rebuild left off (via content-hash dedup), not restart from zero.

R4. Add or enable a stall-detection signal that fires within bounded time (≤120s default per upstream `CASS_INDEX_STALL_DETECT_SECS`) when forward progress halts mid-phase. Emission must include enough diagnostic snapshot for an operator to file a precise upstream bug if the underlying defect is upstream. "Thread states" in this requirement is satisfied by EITHER: (a) OS-level symbolic backtrace via `lldb -batch -ex "thread apply all bt"` captured by the watchdog on fire (required for diagnostic runs that triggered the stall), OR (b) per-thread `std::thread::Builder::name()` plus a heartbeat counter exposing logical park-or-running state (sufficient when no real stall has occurred yet). Queue depths, current connector, current source path must always be in the payload.

R5. Regression coverage: add a focused test that exercises whichever channel / dispatch path is at fault, demonstrating that a stuck producer or consumer is recoverable (or detectable) rather than producing the observed silent hang.

## Constraint

C1. Affected version is upstream `cass 0.4.2` at commit `29c3672a`. Local fork branch in scope: `dac/main` (tracks `origin/main` which equals `upstream/main`). Any fix should be pull-request-ready against upstream `Dicklesworthstone/coding_agent_session_search`, not a fork-only patch — the user's stated workflow keeps `main` in sync with upstream and isolates local changes on `dac/main`.

C2. The stall reproduces on the user's current data corpus (~21,635 conversations across ~10 connectors, ~30 GB on disk including Cursor `.vscdb` mirrors). Any reproduction harness or regression test must be runnable without that full corpus — synthesize a minimum-viable fixture that triggers the same dispatch pattern.

C3. The fix must not regress the streaming pipeline's success cases. The small connectors (amp, cursor, factory, gemini, pi_agent) ingested correctly on the same run that stalled; whatever changes are introduced must preserve that working behavior.

C4. macOS launchd is the production runtime for the watcher (`~/Library/LaunchAgents/com.cass.index-watch.plist`, `KeepAlive=true`). The fix must be safe under `KeepAlive` semantics — i.e., not introduce a respawn loop if the indexer exits abnormally after the fix.

C5. The raw-mirror archive-first behavior introduced in upstream v0.3.7 is intentional design (CHANGELOG: "preserve raw archive before any repair"). The fix must not disable raw-mirror as a side effect; if performance is part of the root cause, address it without removing the archive capability.

## Acceptance Criteria

A1. A documented root-cause writeup exists at `specs/013-cass-rebuild-stall-asupersync/root-cause.md` naming:
- The specific sync primitive that deadlocked (channel handle, condvar, semaphore — by file:line in `src/indexer/` or upstream crate).
- The producer / consumer pair involved and which side failed to wake the other.
- Why the existing v0.3.7 stall-detection watchdog did not catch this case (or, if it did, why the emitted event was not visible).

A2. `cass index --full` against the user's full corpus (or an equivalent synthetic fixture covering the same dispatch pattern) completes end-to-end with zero manual intervention. End-state: DB conversation count ≥ source-file count for each connector minus a documented allowed-skip set (e.g., known-malformed sessions). Note: the `--full --force-rebuild` variant takes the canonical-only short-circuit on a non-empty DB (`src/indexer/mod.rs:9675-9680`) and is intentionally out of scope for this spec; fixing that short-circuit would be a separate spec.

A3. After completion, `cass stats` shows non-zero per-connector counts for claude_code, codex, openclaw (all agents), opencode that match within ≤2% of the on-disk source file counts. Date range covers 2025-09-15 through current.

A4. `CASS_INDEX_STALL_DETECT_SECS=60` produces a structured `stall_detected` NDJSON event within 120s of a forced hang in a test fixture, and the event includes the diagnostic snapshot fields described in upstream v0.3.7 CHANGELOG (lexical rebuild checkpoint, Tantivy segment count, run-lock metadata).

A5. A regression test in `tests/` exercises the fixed dispatch path. The test fails on the current `29c3672a` baseline and passes after the fix.

A6. If the fix turns out to require an upstream contribution, an upstream PR or issue link is recorded in `specs/013-cass-rebuild-stall-asupersync/log.md` with the upstream commit / PR identifier, and the local fork carries the patch on `dac/main` only until the upstream merge lands.

## Out of Scope

- **Watcher live-capture rate or design.** The watcher already runs healthily after restart and captures new sessions. Steady-state watcher performance, scan-cycle tuning, and `watch_state.json` schema changes are not in scope here.
- **Migrating off frankensqlite.** Storage layer choice is upstream's decision. Any fix must work within frankensqlite.
- **Cursor `.vscdb` archive size.** The 25 GB raw-mirror is dominated by Cursor's SQLite workspace databases. Reducing that footprint is a separate concern.
- **Semantic / HNSW index.** Semantic indexing is gated behind `cass models install` and was never enabled on this run.
- **TUI, search query, or doctor surface changes.** This is an indexer-side stall; UI / query / repair surfaces are out of scope.
- **Migrating the corrupted local `feat/007-watchdog-subcommand` branch state.** That cleanup is a separate housekeeping task tracked elsewhere.

## Selected Shape

**Direct root-cause fix in the asupersync streaming worker dispatch path in `src/indexer/` (`cass 0.4.2` / upstream main), with focused regression coverage.**

Approach is determined: this is a clear bug fix with a concrete reproduction and a narrow code surface. No shaping iteration needed because the problem statement and acceptance criteria are well-defined; the unknown is *which* synchronization primitive deadlocked, not *what* the desired end-state looks like.

The investigation path is:

1. Re-run `cass index --full --json` (NOT `--full --force-rebuild` — that takes the canonical-only short-circuit on a non-empty DB per `src/indexer/mod.rs:9675-9680` and never enters the streaming pipeline) with `CASS_INDEX_STALL_DETECT_SECS=60` and `RUST_LOG=cass=debug,coding_agent_search::indexer=trace`. The `--json` flag is required because the existing watchdog only emits inside `emit_progress_events = structured_output && !no_progress_events` at `src/lib.rs:72250, 72531`; without it, the stall event is silent. Captures both the structured stall event and high-resolution traces of the asupersync worker dispatch around the freeze.
2. Attach a debugger (lldb) at the deadlock point and capture exact stack frames in symbolic form (the `sample` output during the original stall had unresolved addresses because the binary was stripped); confirm which mutex / channel handle is held by which thread.
3. Cross-reference with upstream issue #196 thread on `Dicklesworthstone/coding_agent_session_search` — the v0.3.7 fix targeted a different deadlock (zero-writer init in `FrankenConnectionManager`); the v0.4.2 stall is in a downstream stage and probably warrants a sibling fix or a missed handoff between the producer and the asupersync workers.
4. Patch the identified primitive (most likely candidates: a bounded channel without a wakeup on backpressure, or a condvar signaled inside a critical section the receiver also holds). Add the regression test from A5.
5. If the patch belongs upstream, open a PR against `Dicklesworthstone/coding_agent_session_search`; cherry-pick onto `dac/main` until it merges.

The fallback if root-cause is not tractable within reasonable effort:

- Use `CASS_STREAMING_INDEX=0` to force the legacy batch indexer for the historical backfill, then re-enable streaming for steady-state. Document the toggle in `~/Library/LaunchAgents/com.cass.index-watch.plist` env vars. This restores the /goal (full backfill) without the root-cause fix, at the cost of slower steady-state ingest. Treat as workaround, not solution; the root-cause investigation continues separately as a P2 follow-up.
