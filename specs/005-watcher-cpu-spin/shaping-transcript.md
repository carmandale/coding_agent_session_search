# Shaping Transcript: 005-watcher-cpu-spin

**Date:** 2026-03-13
**Participants:** QuickLion (proposer, pi/claude-sonnet-4-20250514), ZenWolf (challenger, crew-challenger/claude-opus-4-6)
**Spec:** specs/005-watcher-cpu-spin/spec.md
**Bead:** coding_agent_session_search-2s40

---

## Phase: Research (QuickLion → ZenWolf)

QuickLion provided initial investigation findings:
- PID 74824 at 99.7% CPU for 40 min, Thread_5277524 (unnamed) in tight open/read/fstat/close loop
- Main thread sleeping normally (recv_timeout), all tokio/tantivy threads idle
- PiAgent detect() returns ~/.pi/agent (22,059 files) instead of ~/.pi/agent/sessions (1,688 files)
- PiAgent WalkDir uses follow_links(true) — symlink to clawdbot adds 2,115 files
- Log file grew to 8.6 GB with no rotation
- Binary stripped — thread identity unknown
- Watcher restarted 5 times in one session period

Initial spec proposed Shape C (defense in depth) = narrow inputs + diagnosability.

ZenWolf asked to read:
1. specs/005-watcher-cpu-spin/spec.md
2. src/indexer/mod.rs (watch_sources, reindex_paths, spawn_connector_producer)
3. src/connectors/pi_agent.rs

---

## Phase: Review (ZenWolf)

ZenWolf verified all claims against the codebase:

1. **Root path claim verified.** PiAgent is the only connector returning a parent dir containing non-session subtrees.
2. **follow_links verified.** PiAgent is the only connector using `follow_links(true)` and the only one with no `max_depth`.
3. **thread::spawn naming verified.** All `thread::spawn` calls in the codebase use bare `thread::spawn`, never `thread::Builder::new().name(...)`.
4. **strip = true verified.** `[profile.release]` has `strip = true` with no debug info.
5. **Watcher flow confirmed.** `build_watch_roots` → `detect()` → `root_paths` → `watcher.watch(root.path, Recursive)`.

---

## Phase: Challenge Round 1 (ZenWolf — 6 concerns)

### Challenge 1: FSEvents noise vs CPU spin are separate phenomena

The spec conflates two issues:
- **Phenomenon A:** PiAgent watching 22K files causes excessive FSEvents and frequent scans.
- **Phenomenon B:** An unnamed thread spinning at 100% CPU for 40 min.

Key evidence: `reindex_paths` runs on the main thread as a synchronous callback. If the scan were the spinning thread, the main thread would NOT be sleeping on `recv_timeout`. The spec's own sample shows the main thread sleeping normally.

**Impact:** Shape A (narrowing inputs) doesn't fix the CPU spin — it fixes unnecessary I/O.

### Challenge 2: 30-minute full scan is bigger than acknowledged

Every 30 minutes, `force_full=true` triggers `reindex_paths` with ALL root paths and `since_ts = None`, re-reading ALL files including 476 MB Codex sessions. The callback blocks the main event loop during this scan. R6 was proposed but then recommended for dropping — premature.

### Challenge 3: Fit check dishonesty

Shape A doesn't "reduce likelihood" of CPU spin — it reduces unnecessary I/O from PiAgent specifically. If root cause is elsewhere, Shape A has zero effect on spin probability.

### Challenge 4: Missing requirement — non-blocking callbacks

`reindex_paths` blocks the event loop. During scan: no events processed, no debouncing, no heartbeat. FSEvents accumulate in crossbeam channel. When callback returns, queued events trigger another scan cycle.

Proposed: **R7: Scan callbacks must not block event loop for unbounded time.**

### Challenge 5: Unexplained 5 restarts

The spec notes restarts but doesn't investigate. Questions:
- Panic logs in crash reporter?
- launchd exit codes?
- ERROR lines near restart boundaries in log?

### Challenge 6: B2 accuracy

Cargo.toml has `debug = true` only in `[profile.profiling]`, not in `[profile.release]`. The fix is adding `debug = 1` AND changing `strip`.

(ZenWolf later corrected themselves on #6 — the release profile has NO debug info, so the spec's B2 proposal was correct.)

---

## Phase: Research Update (QuickLion)

### Critical Discovery: Health Watchdog Kill Loop

QuickLion found `com.cass.health-watchdog.plist` — a launchd agent running every 10 minutes that:

1. Checks `cass health --json`
2. If `age_seconds > 600` (10 min), kills watcher via `launchctl kickstart -k`
3. Runs `cass index --full` (fails with tantivy LockBusy since new watcher holds lock)
4. Repeats every 10 minutes, for hours

Watchdog log evidence:
```
04:37:06 stale (607s > 600s threshold) → restart
04:37:11 Full reindex failed
04:47:13 stale (607s > 600s threshold) → restart
04:47:19 Full reindex failed
[continues every 10 minutes]
```

**Impact:** The 5 restarts weren't crashes — the watchdog was CAUSING them. The concurrent `cass index --full` fights the watcher for the tantivy lock. The 600s stale threshold is too tight for the 300s heartbeat + scan time.

### Verified: No signal handling exists

Tokio has `signal` feature enabled but it's never used. Zero signal handling in the codebase. Combined with `panic = "abort"`, every process death is unclean — tantivy's IndexWriter Drop impl never runs.

---

## Phase: Challenge Round 2 (ZenWolf — kill→corruption→spin hypothesis)

ZenWolf proposed a causal chain:

1. Watchdog SIGKILLs watcher during tantivy background merge
2. Process dies mid-write on segment files (SIGKILL = uncatchable, no cleanup)
3. New watcher opens corrupted index, tantivy spawns threads to deal with corrupt segments
4. Spinning threads prevent scan completion → health reports stale → watchdog kills again
5. Self-sustaining failure loop

ZenWolf proposed three critical fixes:
- **Fix 1:** Fix or remove the health watchdog (threshold, SIGTERM before SIGKILL, heartbeat file)
- **Fix 2:** Add graceful SIGTERM handling (flush tantivy, wait for merges)
- **Fix 3:** Tantivy corruption recovery on startup

And two new requirements:
- **R9:** Watcher must handle SIGTERM by flushing tantivy state before exit
- **R11:** Tantivy index corruption must be recoverable without manual intervention

---

## Phase: Revision (QuickLion)

QuickLion accepted all challenges and proposed revised R and S:

### Revised Requirements

| ID | Requirement | Status |
|----|-------------|--------|
| R0 | Watcher CPU usage must be bounded | Core goal |
| R0.1 | Main-thread scan callback must complete in bounded time | Core goal |
| R0.2 | No background thread may spin indefinitely without detection | Core goal |
| R1.1 | Watch roots return session directories, not parent dirs | Must-have |
| R1.2 | WalkDir has max_depth and does not follow symlinks | Must-have |
| R2 | Diagnosability (symbols + named threads) | Must-have |
| R3 | Watchdog and process lifecycle must not create instability | Must-have |
| R3.1 | Watcher handles SIGTERM by flushing tantivy before exit | Must-have |
| R3.2 | Watchdog uses heartbeat file for liveness | Must-have |
| R3.3 | Tantivy corruption recoverable without manual intervention | Must-have |
| R4 | Per-connector scan cooldown | Nice-to-have |
| R5 | Log rotation | Nice-to-have |

### Revised Shapes

- **A:** Hygiene pass (narrow roots, bound WalkDir, log rotation, heartbeat watchdog)
- **B:** Instrumentation (debug symbols, named threads, elapsed-time logging)
- **C:** Lifecycle hardening + hygiene + instrumentation (A + B + SIGTERM + corruption recovery)
- **D:** Async scan architecture (deferred — only if C doesn't resolve)

---

## Phase: Challenge Round 3 (ZenWolf — 4 remaining issues)

### Issue 1: R3.1 and R3.3 were initially dropped

ZenWolf demanded they be added back. Verified: zero signal handling in codebase (grepped for ctrlc, signal, SIGTERM, sigaction, set_handler — nothing). Combined with `panic = "abort"`, tantivy's IndexWriter Drop impl never runs on any abnormal exit.

### Issue 2: A4 threshold math wrong

With full_scan_interval=1800s and max scan duration up to 10 min, a 1200s threshold could kill the watcher 30 seconds into a full scan. Proposed: heartbeat file approach — watcher writes timestamp every 60s, watchdog checks file age. Decouples "is process alive?" from "has scan completed?"

Subtle point: during blocking `reindex_paths`, heartbeat doesn't update. This is correct — if main loop is stuck >600s, that IS a legitimate restart trigger.

### Issue 3: A4/B sequencing tension

If A4 deployed first, spin might never recur (killed the trigger). If B deployed first, watchdog keeps killing (deliberate harm for diagnostics). Resolution: deploy simultaneously. If spin stops → watchdog was root cause. If persists → instrumentation catches real thread.

### Issue 4: D4 typo

D4 = A1+A2+A4 omitted A3 (log rotation). Fixed.

---

## Phase: Final Revision (QuickLion)

All four issues addressed. Incorporated ZenWolf's three final notes:
1. C4 spike direction: attempt trivial search per segment with timeout
2. D inherits C3 (SIGTERM handler) — don't repeat the unclean-exit mistake
3. Document tantivy merge thread evidence (all named, all idle in sample)

### Key Evidence: Tantivy Merge Threads Named and Idle

QuickLion verified tantivy source (segment_updater.rs):
```rust
ThreadPoolBuilder::new().thread_name(|_| "segment_updater".to_string())
ThreadPoolBuilder::new().thread_name(|i| format!("merge_thread_{i}"))
```

In the sample: segment_updater and merge_thread_0-3 were ALL idle/sleeping. Thread_5277524 is unnamed and NOT a tantivy merge thread. ZenWolf's kill→corruption→spin hypothesis remains plausible as a trigger mechanism, but the spinning thread is from another source. Instrumentation (B) will identify it.

---

## Phase: Agreement

ZenWolf confirmed the revised spec passes scrutiny:

> "The spec went from 'fix PiAgent inputs and hope the spin goes away' to 'fix the kill loop that's probably causing it, harden the lifecycle, and instrument for confirmation.' That's a meaningful improvement."

### Shape selected: C (Lifecycle hardening + hygiene + instrumentation)

**Rationale grounded in fit check:**
- C is the only shape that passes R3.1 (SIGTERM handling)
- C passes R1.1, R1.2, R2, R3.2, R5 (hygiene + diagnosability + heartbeat watchdog)
- C honestly fails R0, R0.2, R3.3, R4 — because we can't guarantee CPU bounds without root cause, and corruption recovery (C4) needs a spike
- D is deferred until evidence from B's instrumentation justifies architectural change
- Simultaneous deployment: if spin stops after C, watchdog was root cause (confirmed by elimination). If spin persists, B catches the thread.
