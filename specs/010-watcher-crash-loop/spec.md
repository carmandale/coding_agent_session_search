---
title: "Fix watcher streaming crash loop: cursor ordering, LockBusy retry, stale last_scan_ts"
date: 2026-03-31
bead: coding_agent_session_search-33nb
---

<!-- issue:complete:v1 | harness: unknown | date: 2026-04-01T01:03:17Z -->

# Spec 010 — Watcher Streaming Crash Loop

## Background

The cass watcher (`cass index --watch`) has been stuck in a continuous crash/restart loop
for 27+ hours (since 2026-03-30T21:00 UTC). As of discovery, the log contained **3,934**
occurrences of the crash signature. The watcher burns ~80-95% CPU, searches return stale
results, and `last_scan_ts` has not been persisted in 27 hours.

### What "the loop" looks like

Every ~30-45 seconds, the watcher runs a full scan, crashes, and immediately restarts:

```
index starting (full=false, watch=true, watch_once=0)
full_scan: no last_scan_ts or rebuild requested      ← always, because ts never saved
[25s of streaming_ingest from codex/claude/pi_agent/opencode/openclaw]
streaming consumer disconnected; stopping producer connector="opencode"
streaming consumer disconnected; stopping producer connector="pi_agent"
...
WARN cursor failed to extract from db path=.../state.vscdb error=streaming consumer disconnected
...×15 workspaces
WARN drop_close db_path=agent_search.db
index failed: tantivy error: Failed to acquire Lockfile: LockBusy
index starting (full=false, watch=true, watch_once=0)       ← immediately restarts
```

## Root Causes (Three Compound Issues)

### RC1 — Consumer closes before cursor finishes (primary trigger)

The streaming consumer (`run_streaming_consumer`) exits when `active_producers` reaches 0.
Producers send `IndexMessage::Done` when they finish. The faster connectors (codex, claude,
pi_agent, opencode, openclaw) finish their scans in ~25 seconds and send Done. The consumer
counts them down to 0 and exits.

Cursor is also registered as a streaming producer, but is the slowest connector — it reads
~15 separate workspace SQLite `.vscdb` files sequentially. It takes 60-80+ seconds. By the
time cursor tries to send its batches, the channel receiver has been dropped.

Cursor receives `Err` on send (channel closed) and logs WARN for each workspace file.

**This is not a cursor bug.** Cursor is the victim: the consumer left while cursor was still
working. The consumer's Done-counting logic does not have a timeout — it either waits
forever or exits when the count reaches zero, depending on whether Done messages arrive
before channel close.

The real issue: **cursor takes 3-4× longer than all other connectors combined.** The consumer
design assumes connectors finish at roughly similar rates.

### RC2 — Zero-delay retry races against IndexWriter Drop (secondary cascade)

After `run_streaming_index()` returns `Err`, the watcher immediately restarts. The previous
`TantivyIndex` (which holds an `IndexWriter`) is being dropped. Rust's drop is synchronous
within the same thread, but in the `--watch` mode (installed v0.2.5), the watcher may spawn
a new attempt while the old drop is in progress.

The tantivy lock file (`.tantivy-writer.lock`) has not been deleted by the time the new
IndexWriter tries to acquire it. Result: `Failed to acquire Lockfile: LockBusy`.

*Evidence*: the lock file does not exist on disk between cycles (checked live) — meaning it
*does* eventually get released. The retry is simply too fast.

### RC3 — `last_scan_ts` only saved on fully clean completion (consequence amplifier)

`storage.set_last_scan_ts(scan_start_ts)` is called only *after* both:
- `run_streaming_index()` succeeds (returns `Ok`), AND
- `t_index.commit()` succeeds

Since RC1 causes `run_streaming_index()` to return `Err` every time, `last_scan_ts` is
never persisted. Every restart sees `full_scan: no last_scan_ts or rebuild requested` and
performs a complete rescan from time zero — even though all non-cursor connectors completed
successfully in the previous attempt.

This turns a ~25s crash into a ~25s-every-restart perpetual rescan instead of a 2-3s
incremental check.

## Scope

- **In scope**: Fix the three root causes above in the streaming indexer path
- **In scope**: Confirm fix works with installed binary (run dev-install.sh to deploy)
- **In scope**: Verify the 30-min periodic heartbeat full scan still works after fix
- **Out of scope**: Cursor connector performance (why it's slow is a separate concern)
- **Out of scope**: The pre-existing P0 bead `9hgf` (different project/domain — oh-my-opencode TypeScript hooks)
- **Out of scope**: Tantivy segment accumulation cleanup (may be needed separately post-fix)

## Affected Code

All paths in `src/indexer/mod.rs` (source v0.1.55; installed binary v0.2.5 has same
streaming architecture with additional connectors):

- `run_streaming_consumer()` — RC1: consumer exit logic
- `run_streaming_index()` — RC2: error propagation and retry path
- `run_index()` — RC3: `set_last_scan_ts` placement

## Acceptance Criteria

- [ ] **AC1**: After a full scan completes (even partially — all non-cursor connectors done),
  `last_scan_ts` is persisted. A subsequent restart performs an incremental scan, not full.

- [ ] **AC2**: The cursor connector failure (channel closed while sending) does not propagate
  as an `Err` that aborts the entire scan. Either: (a) cursor completes before consumer
  closes, or (b) cursor errors are treated as non-fatal WARN, and the scan continues to
  commit + save timestamp.

- [ ] **AC3**: After a `LockBusy` error on retry, the watcher waits at least 5 seconds
  before attempting to create a new `IndexWriter`. No `LockBusy` errors in normal operation.

- [ ] **AC4**: The log no longer shows repeated `full_scan: no last_scan_ts` on consecutive
  restarts. Second restart (and beyond) shows `incremental_scan: using last_scan_ts`.

- [ ] **AC5**: `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`,
  `cargo fmt --check`, and `cargo test --lib` all pass.

- [ ] **AC6**: After deploying via `./dev-install.sh`, `cass health --json` reports
  `"healthy": true` within 5 minutes of first clean scan completing.

- [ ] **AC7**: No regression to the 30-min periodic heartbeat full scan behavior in
  `watch_sources` (that path is separate from the initial streaming scan).

## Proposed Fix Approaches

### Fix for RC1 (cursor ordering)

**Option A — Drain remaining producers on channel close**  
When `rx.recv()` returns `Err(_)` (channel closed), the consumer currently breaks
immediately. Instead, continue processing buffered messages. This won't help if cursor's
sender is also dropped, but it's a defensive posture.

**Option B — Give cursor a separate post-scan pass**  
After the streaming scan commits, run cursor in a second sequential pass that's allowed
to fail non-fatally. This keeps the streaming architecture intact for fast connectors
and handles the slow outlier separately.

**Option C — Treat cursor send errors as non-fatal in the producer**  
In `spawn_connector_producer`, when `tx.send(Batch)` fails, log a WARN but still send
`Done`. This ensures the consumer receives Done for cursor even if its data was lost.
The next incremental scan will pick up cursor's sessions since `last_scan_ts` advances.

*Recommended: Option C — minimal change, allows cursor data to be picked up incrementally.*

### Fix for RC2 (LockBusy retry)

Add `std::thread::sleep(Duration::from_secs(5))` before retrying after any `LockBusy`
error. Or detect `LockBusy` specifically and apply exponential backoff (2s → 4s → 8s).

### Fix for RC3 (last_scan_ts persistence)

Move `set_last_scan_ts` to after the streaming consumer loop completes (even on partial
success). Track a `committed_at_least_once: bool` flag; if true, save the timestamp.
Alternatively, save the timestamp if `run_streaming_consumer` returns `Ok` even when
some producers failed.

## Verification

```bash
# After deploying fix:
~/.cargo/bin/cargo test --lib -- streaming 2>&1 | tail -20
./dev-install.sh
pkill -TERM -f "cass index --watch"   # let launchd restart with new binary
sleep 10
cass health --json
# Expected: healthy: true (or rebuilding for first scan, then healthy within 5min)

# Confirm no LockBusy in logs after restart:
grep "LockBusy" ~/Library/Logs/cass-index-watch.log | tail -5
# Expected: all entries pre-date the fix deployment

# Confirm incremental scans are happening (not full):
grep "incremental_scan\|full_scan" ~/Library/Logs/cass-index-watch.log | tail -10
# Expected: first restart=full, subsequent=incremental
```

## Notes

- The installed binary is v0.2.5; source is v0.1.55. After fixing source, run
  `./dev-install.sh` to build and deploy.
- The watcher log is at `~/Library/Logs/cass-index-watch.log` (1.2M lines as of discovery).
- Two zombie watcher processes (PIDs 20997, 24422 from Monday) are harmless; launchd will
  reap them on next restart.
- Tantivy's `index/` directory may have accumulated segment files from 3,934 partial scans.
  Monitor segment count after fix; a `--force-rebuild` may be needed if merge policy can't
  clean them up.
