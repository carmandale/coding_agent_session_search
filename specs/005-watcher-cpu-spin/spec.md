<!-- Codex Review: APPROVED after 5 rounds | model: gpt-5.3-codex | date: 2026-03-14 -->
<!-- Status: REVISED -->
<!-- Revisions: R1.2 updated to "symlink following bounded by depth limit" (from "no follow_links"), R3.3 downgraded to Nice-to-have (from Must-have), A2/A4/B1 summary tables updated for consistency with plan -->
---
title: "fix: Watcher CPU spin — unnamed thread burns 100% CPU"
date: 2026-03-13
bead: coding_agent_session_search-2s40
shaping: true
---

# Watcher CPU Spin

## User Story

**As a user running `cass` on my Mac,** I want the background watcher to run
quietly without burning CPU, and if something goes wrong, I want it to
recover on its own — not make things worse.

### What's happening today

The watcher has a self-destruct loop:

1. A health watchdog checks every 10 minutes: "is the index fresh?"
2. The staleness threshold (600s) is too tight, so the answer is almost always "no"
3. The watchdog **SIGKILL**s the watcher — instant death, no cleanup
4. Tantivy's index writer never flushes — segment files left half-written on disk
5. A new watcher starts on the corrupted index, an unnamed thread starts spinning at 100% CPU
6. 10 minutes later, the watchdog kills it again

Meanwhile, the PiAgent connector watches 22,000 files (including
`node_modules`) when only 1,700 session files matter, flooding the system
with unnecessary filesystem events. And the binary is stripped, so when we
`sample` the spinning process, every stack frame shows `??? (in cass)` —
completely useless for diagnosis.

### What Shape C delivers

**Phase 1 — Make it diagnosable** (so the next incident takes 5 minutes, not an hour):
- Keep function names in the release binary (`strip = "debuginfo"`)
- Name every spawned thread (`"connector-codex"`, `"index-watcher"`, etc.)
- Log a warning when any connector scan takes >30 seconds

**Phase 2 — Stop the noise and fix the watchdog:**
- Narrow PiAgent's watch root from `~/.pi/agent` (22K files) to `~/.pi/agent/sessions` (1.7K files)
- Cap the WalkDir at depth 10 (prevents future symlink bombs, keeps existing clawdbot symlink working)
- Replace the watchdog's broken staleness check with a heartbeat file — the watcher writes a timestamp every 60 seconds, the watchdog checks that file's age
- Remove the concurrent `cass index --full` that was fighting the watcher for the tantivy lock and always failing
- Add log rotation so the log doesn't grow to 8.6 GB again

**Phase 3 — Die gracefully:**
- Catch SIGTERM, check a shutdown flag between connector scans, exit cleanly
- Change the watchdog from instant SIGKILL to SIGTERM → wait 120s → SIGKILL
- This means tantivy actually gets to flush — no more corruption from dirty kills

**Phase 4 — Verify it worked:**
- If the CPU spin stops → the watchdog kill loop was the root cause, confirmed by elimination
- If it persists → the new instrumentation tells us exactly which thread and function, and we fix it for real

### The honest caveat

We still can't identify the exact spinning thread (binary was stripped). We're
fixing the most plausible trigger, making kills safe, and instrumenting so
that if it happens again we'll know exactly what and why.

---

## Source

> `cass index --watch` (PID 74824) pegged at 99.7% CPU for 40+ minutes.
> Process sampling via `sample` shows an unnamed background thread in a tight
> `open/read/fstat/close` loop — 2233/2233 samples on file I/O. All other
> threads (main, tokio workers, notify-rs fsevents, tantivy merge/segment)
> were idle/sleeping. The process accumulated 6:41 of CPU time in ~40 minutes.
>
> The process eventually died and was replaced by a new watcher at normal 0% CPU.
> The watcher had been restarting every ~1 hour (5 restarts in the session log).
>
> Load average at time of capture: 3.73, 106.53, 320.74 — indicating sustained
> CPU pressure over minutes-to-hours.

## Problem

Two separate phenomena are at play, possibly with a shared root cause:

**Phenomenon A (FSEvents noise):** PiAgent connector watches 22,059 files
(including 19,991 node_modules files) when only 1,688 session files matter.
Active pi sessions writing to `~/.pi/agent/sessions/` trigger constant
FSEvents → 30+ PiAgent scans in 25 minutes. This is real, verified, and
fixable — but it is NOT the CPU spin.

**Phenomenon B (CPU spin):** A single unnamed thread consumes 100% CPU doing
tight `open/read/fstat/close`. The main thread (running `watch_sources` event
loop) was sleeping normally on `recv_timeout`. All tokio workers, tantivy
merge threads (`segment_updater`, `merge_thread_0-3`), and the notify-rs
FSEvents thread were idle. The spinning thread is NOT the scan callback
(which runs on the main thread) and NOT a tantivy merge thread (those are
named via rayon `ThreadPoolBuilder` and were all idle in the sample).

### Most Plausible Root Cause: Watchdog Kill Loop

Investigation discovered `com.cass.health-watchdog.plist` — a launchd agent
that runs every 10 minutes, checks `cass health --json`, and kills the
watcher if `age_seconds > 600`:

```
04:37:06 stale (607s > 600s threshold) → launchctl kickstart -k (SIGKILL)
04:37:11 cass index --full → FAILED (tantivy LockBusy)
04:47:13 stale (607s > 600s threshold) → kill → restart
04:47:19 cass index --full → FAILED
[repeats every 10 minutes for hours]
```

The 600s stale threshold is too tight for the watcher's 300s heartbeat +
variable scan duration. The watchdog kills the watcher mid-operation every
10 minutes, then runs `cass index --full` which fails because the new watcher
already holds the tantivy lock.

Combined with zero signal handling and `panic = "abort"` in the release
profile, every kill is unclean — tantivy's `IndexWriter` Drop impl (which
waits for in-flight merges and commits) never runs. This creates a plausible
corruption → instability → CPU spin chain, though the exact spinning thread
remains unidentified due to stripped binary symbols.

### Evidence: Tantivy Merge Threads Were Idle

Tantivy names its threads via `ThreadPoolBuilder`:
```rust
// tantivy/src/indexer/segment_updater.rs
ThreadPoolBuilder::new().thread_name(|_| "segment_updater".to_string())
ThreadPoolBuilder::new().thread_name(|i| format!("merge_thread_{i}"))
```

In the `sample` output, `segment_updater` and `merge_thread_0-3` were all
sleeping. The spinning Thread_5277524 is unnamed and from another source.
Instrumentation (named threads + debug symbols) will identify it on next
reproduction.

### Contributing Factors

| Factor | Impact | Verified |
|--------|--------|:--------:|
| PiAgent watches 22K files (should watch 1.7K) | Excessive FSEvents noise | ✅ |
| PiAgent `follow_links(true)` with no max_depth | Fragile against symlink bombs | ✅ |
| Zero signal handling + `panic = "abort"` | Every kill is unclean | ✅ |
| Watchdog kills watcher every 10 min | Self-sustaining instability loop | ✅ |
| 8.6 GB log file with no rotation | Disk/memory pressure on long runs | ✅ |
| Stripped binary — unnamed threads | Can't identify spinning thread | ✅ |
| 476 MB Codex session files re-read on full scan | I/O pressure every 30 min | ✅ |
| `reindex_paths` blocks event loop synchronously | Main thread starvation during scan | ✅ |

## Constraints

- The watcher must remain a long-lived daemon (launchd KeepAlive)
- Incremental indexing (since_ts) must remain correct — no lost sessions
- The fix should not require users to reconfigure anything
- Performance on macOS with Dropbox/FSEvents quirks is the primary target

---

## Requirements (R)

| ID | Requirement | Status |
|----|-------------|--------|
| R0 | Watcher CPU usage must be bounded | Core goal |
| R0.1 | Main-thread scan callback must complete in bounded time | Core goal |
| R0.2 | No background thread may spin indefinitely without detection | Core goal |
| R1 | Watch scope must be minimized | Must-have |
| R1.1 | Watch roots return session directories, not parent dirs | Must-have |
| R1.2 | WalkDir has max_depth; symlink following bounded by depth limit | Must-have |
| R2 | Release binary + named threads enable post-mortem diagnosis | Must-have |
| R3 | Watchdog and process lifecycle must not create instability | Must-have |
| R3.1 | Watcher handles SIGTERM by flushing tantivy state before exit | Must-have |
| R3.2 | Watchdog uses heartbeat file, not index staleness, for liveness | Must-have |
| R3.3 | Tantivy corruption recoverable without manual intervention | Nice-to-have |
| R4 | Per-connector scan cooldown prevents sustained scan cycling | Nice-to-have |
| R5 | Log file has size limits or rotation | Nice-to-have |

---

## A: Hygiene pass — fix the known-wrong things

| Part | Mechanism |
|------|-----------|
| **A1** | PiAgent `detect()` returns `~/.pi/agent/sessions` instead of `~/.pi/agent` |
| **A2** | PiAgent `session_files()`: add `max_depth(10)` (keep `follow_links(true)` to preserve clawdbot sessions) |
| **A3** | Log rotation: truncate or rotate when log exceeds 100 MB |
| **A4** | Heartbeat file: watcher writes timestamp every 60s from event loop; watchdog checks heartbeat file age (threshold 2700s) instead of `cass health` index staleness |

**A4 detail:** During a blocking `reindex_paths` callback, the heartbeat
does NOT update (main loop is frozen). This is intentional — if the main
loop is stuck >2700s, that IS a legitimate reason to restart. The heartbeat
decouples "is the process alive?" from "has a scan completed?" — these are
different questions.

---

## B: Instrumentation for diagnosis

| Part | Mechanism |
|------|-----------|
| **B1** | Change `strip = true` to `strip = "debuginfo"` in `[profile.release]` (keeps symbol table for `sample` output, strips DWARF debug info) |
| **B2** | Name all `thread::spawn` calls via `thread::Builder::new().name(...)` — currently zero named threads in cass code |
| **B3** | Add elapsed-time logging to `reindex_paths` per connector scan: `tracing::warn!` if scan exceeds 30s |

---

## C: Lifecycle hardening + hygiene + instrumentation (selected)

| Part | Mechanism | Flag |
|------|-----------|:----:|
| **C1** | = A1 + A2 + A3 + A4 (hygiene + heartbeat watchdog) | |
| **C2** | = B1 + B2 + B3 (instrumentation) | |
| **C3** | SIGTERM handler: catch SIGTERM via `signal_hook` or tokio signal, set shutdown flag, flush tantivy (`t_index.commit()`), exit cleanly. Requires evaluating whether `panic = "abort"` can coexist with signal handling (it can — SIGTERM is caught before panic). | |
| **C4** | Startup corruption recovery: if tantivy open succeeds but first commit/search fails, delete index directory and rebuild from scratch. Spike direction: attempt `Index::searchable_segment_ids()` + trivial search per segment with 5s timeout to detect segments that cause readers to spin. | ⚠️ |

**Deployment note:** A (hygiene) and B (instrumentation) deploy simultaneously.
If CPU spin stops after deployment, root cause was watchdog-induced instability
(confirmed by elimination). If spin persists despite watchdog fix, B's
instrumentation captures the actual thread identity for targeted fix.

---

## D: Async scan architecture (deferred)

Only pursue if Shape C doesn't resolve the CPU spin AND B's instrumentation
reveals main-thread starvation as the cause.

| Part | Mechanism | Flag |
|------|-----------|:----:|
| **D1** | Move `reindex_paths` to a background thread with bounded concurrency | ⚠️ |
| **D2** | Main event loop stays responsive during scans (heartbeat continues writing) | |
| **D3** | Per-connector scan mutex: if a scan for connector X is in-flight and new events trigger another, skip (in-flight scan's since_ts covers the window) | ⚠️ |
| **D4** | = C1 + C2 + C3 (inherits all of Shape C, including SIGTERM handler) | |

D inherits C3 (SIGTERM) because any shape that improves scan architecture
but still dies uncleanly on kill hasn't learned the lesson.

---

## Fit Check

| Req | Requirement | Status | A | B | C | D |
|-----|-------------|--------|---|---|---|---|
| R0 | Watcher CPU usage must be bounded | Core goal | ❌ | ❌ | ❌ | ❌ |
| R0.1 | Main-thread scan bounded time | Core goal | ❌ | ❌ | ❌ | ✅ |
| R0.2 | No background thread spin without detection | Core goal | ❌ | ❌ | ❌ | ❌ |
| R1.1 | Watch roots scoped to sessions | Must-have | ✅ | ❌ | ✅ | ✅ |
| R1.2 | WalkDir max_depth; symlink bounded | Must-have | ✅ | ❌ | ✅ | ✅ |
| R2 | Diagnosability (symbols + named threads) | Must-have | ❌ | ✅ | ✅ | ✅ |
| R3.1 | SIGTERM handling | Must-have | ❌ | ❌ | ✅ | ✅ |
| R3.2 | Heartbeat-based watchdog | Must-have | ✅ | ❌ | ✅ | ✅ |
| R3.3 | Corruption recovery | Nice-to-have | ❌ | ❌ | ❌ | ❌ |
| R4 | Per-connector cooldown | Nice-to-have | ❌ | ❌ | ❌ | ✅ |
| R5 | Log rotation | Nice-to-have | ✅ | ❌ | ✅ | ✅ |

**Notes:**
- R0 fails all shapes: cannot guarantee CPU threshold without knowing the
  exact spinning thread. C addresses the most plausible trigger (watchdog
  kills); D addresses main-thread starvation. Neither guarantees elimination.
- R0.2 fails all shapes: no shape proposes runtime detection of background
  thread spin. B provides post-mortem forensic tools, not runtime detection.
- R3.1 only passes C and D: only these shapes add SIGTERM handling. A and B
  alone leave the process vulnerable to unclean kills.
- R3.3 fails all shapes: C4 is flagged (⚠️) — detecting corrupt-but-openable
  tantivy segments is non-trivial. Needs a spike to determine if tantivy
  exposes segment health checks.
- D inherits C (including C3), so D passes everything C passes plus R0.1
  and R4.

---

## Recommendation

**Shape C** is the recommended first step.

It is the only shape that addresses the most plausible root cause chain
(watchdog kills → unclean death → instability) via C3 (SIGTERM) + C1/A4
(heartbeat watchdog). It provides hygiene fixes (C1) and instrumentation
(C2) that are clearly correct regardless of root cause.

C4 (corruption recovery) remains flagged and needs a spike before
implementation.

**Shape D** is deferred. Only pursue if Shape C doesn't resolve the CPU
spin AND B's instrumentation (deployed with C) reveals main-thread
starvation as a contributing cause. D builds on C — it is not an
alternative to it.

**What we're honest about:** We cannot guarantee R0 (CPU bounds) because
the spinning thread is unidentified. We are fixing the most plausible
trigger and instrumenting for confirmation. If the spin stops after C,
the watchdog was the root cause. If it persists, we have the tools to
identify the thread and pursue D or a targeted fix.
