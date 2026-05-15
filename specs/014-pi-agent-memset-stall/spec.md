---
title: "pi_agent watch-once stalls after first chunk with 22 GB RSS memset loop"
date: 2026-05-15
bead: coding_agent_session_search-373b1
---

<!-- issue:complete:v1 | harness: unknown | date: 2026-05-15T15:12:28Z -->
<!-- Codex Review: APPROVED after 5 rounds | model: gpt-5.3-codex | date: 2026-05-15 | trust_level: full | round_records: .codex-round-91909777/, .codex-round-7f3ae320/, .codex-round-11ebb7d2/, .codex-round-8f680d33/, .codex-round-a8204eb3/ | Status: REVISED (acceptance #1, Evidence, Requirement 3 amended with provenance) -->

## Source (verbatim)

> "/issue for pi_agent stall (~10 min) — captures findings (PID, RSS pattern, that it was a different signature from spec-013) before they get stale" — user, 2026-05-15

> "the goal is to be in sync with upstream and running properly, capturing all sessions and cass working. that is the /goal" — user, 2026-05-13

> "claude code and codex are top priorities, with pi-agent next and then opencode. no other agents are priorities, but if we can get every historical session, that is desired" — user, 2026-05-14

## Problem

On v0.4.7 with PR #233 applied, `cass index --watch-once ~/.pi/agent/sessions` stalls after indexing ~33 conversations. The indexer process holds 100 % CPU and the lock-file heartbeat keeps ticking, but DB row growth stops completely for an hour or more while RSS grows to roughly 22 GB. Killing the process cleanly unblocks the rest of the driver queue.

This is **structurally distinct** from the watch-once chunk-size stall fixed by `e429eaab` / upstream PR #233:

| Signature                          | Spec 013 stall                         | Spec 014 stall (this one)               |
|------------------------------------|----------------------------------------|-----------------------------------------|
| Top frame on active thread         | inside writer mutex code path          | `libsystem_platform.dylib`_platform_memset` |
| RSS                                | ~6–8 GB, mostly stable                 | grows to 22 GB and holds                |
| Effect of chunk-size patch (#233)  | resolves it completely                 | no effect — patch is applied            |
| Watchdog visibility                | `stall_detected` fires reliably        | not yet confirmed (lock heartbeat alive misleads watchdog) |
| Pattern of starved workers         | identical (asupersync + rayon + Tantivy parked) | identical                       |

So the wedged-workers shape is the same — one producer is starving every consumer — but the producer is allocating/clearing memory in a hot loop, not blocking on a writer mutex.

### Evidence captured during recovery session (2026-05-14 → 2026-05-15)

- Indexer PID 66713 alive 2 h 4 min, 99.4 % CPU, **22.3 GB RSS**.
- Lock file `index-run.lock` `updated_at_ms` was ticking (heartbeat thread alive), but DB conversation count frozen at `pi_agent=33` for 1 h+.
- `lldb -batch -p 66713 -o "thread list"` showed:
  ```
  thread #18 (active): cass.real`_platform_memset + 180  (in _platform.dylib)
  16 threads named asupersync-worker-0..15  -> __psynch_cvwait
  16 threads named thrd-tantivy-index*       -> semaphore_wait_trap
  16 anonymous rayon threads                 -> __psynch_cvwait
  ```
- Disk inventory: `~/.pi/agent/sessions/` holds **2,073** jsonl files (1.70 GB total, biggest 72 MB) in workspace-encoded subdirectories (e.g. `--Users-dalecarman-dev-hsbc--`). *[Corrected 2026-05-15 during /codex-plan: original "≥ 2,800" estimate was wrong; verified via `find ~/.pi/agent/sessions -name "*.jsonl" -type f -exec stat -f "%z" {} \;`.]*
- First successful pi indexing in this session captured 1 row before the watcher restart, then 32 more during the stuck run = 33 total before kill.
- The same patched binary (v0.4.7 + chunk-size fix) processed claude_code (2,573), codex (5,712), and opencode (976) end-to-end without recurrence. So the bug is specific to the pi-connector data shape, not the v0.4.7 indexer in general.

## Requirements

1. Identify the specific allocation site that drives the `_platform_memset` loop (Rust source line + struct field). The Rust frames behind that memset are stripped in the release binary — a profiling build (`cargo build --profile profiling`) plus `sample <pid>` or `instruments -t "Time Profiler"` should resolve them.
2. Either:
   - **(a) Eliminate or cap the unbounded allocation** on the pi-connector ingest path (preferred), or
   - **(b) Add a per-conversation memory-pressure check** in `ingest_watch_batch_with_oom_split()` that splits the batch (or quarantines the conversation) before RSS explodes, the same way the existing OOM-split handles `error_is_out_of_memory()`.
3. Pi-agent historical backfill (`cass index --watch-once ~/.pi/agent/sessions`) must complete without a manual kill on the user's corpus (**2,073 sessions** — see Evidence section). Acceptance is reaching `success: true` with `conversations >= 1,970` (≥ 95 % of 2,073). *[Amended 2026-05-15 during /codex-review round 1: corpus is 2,073 not 2,800; threshold corrected to match acceptance #1.]*
4. The fix must not regress the chunk-size behaviour from spec 013 — claude_code and codex backfill must keep completing cleanly.

## Constraint

- **Single source-of-truth binary.** Whatever lands here also ships through the same `~/.local/bin/cass` symlink + watcher daemon path. No special "pi-only" build flavour.
- **Upstreamable.** Like PR #233, the fix should be small and structurally explicable, so it can land in `Dicklesworthstone/coding_agent_session_search` without per-host carve-outs. Stay within `src/indexer/`, `src/persist.rs`, or the franken-agent-detection pi connector glue; do not patch external `franken-agent-detection` directly from this repo.
- **No destructive recovery.** The current pi_agent=33 rows must not be lost. Backfill is additive; if the fix changes the schema for the affected columns (e.g. shrinks an `extra_bin` BLOB), include a migration in `src/storage/sqlite.rs` matching the existing additive style.
- **Honour `/goal` priority order.** This work is third priority (claude_code + codex first, then pi_agent, then opencode). Do not let the investigation gate the watcher's forward capture — the daemon must keep running while this is debugged.

## Acceptance Criteria

1. `cass index --watch-once ~/.pi/agent/sessions --json --no-progress-events` on the user's machine completes with `success: true` and indexes **≥ 1,970 pi conversations** (≥ 95 % of the 2,073 discovered jsonl files; any skipped files must be recorded in `<data_dir>/quarantine/watch_ingest_poison.jsonl` and reflected in the run receipt). *[Amended 2026-05-15 during /codex-plan: original "≥ 2,500" was based on the wrong corpus count (≥ 2,800 jsonl) — actual corpus is 2,073 jsonl files (see Evidence section), so the original threshold was structurally unsatisfiable. New threshold preserves the spirit (substantial coverage of the user's pi history) while being achievable on the real corpus.]*
2. Peak RSS for that run stays under **8 GB** (matching the post-fix claude_code / codex / opencode runs).
3. The watch ingest loop continues to chunk through `watch_ingest_chunk_size()` (PR #233 behaviour preserved). Regression test or focused unit test demonstrates this.
4. Symbolised `sample` output (or equivalent) of a pre-fix repro is attached to the PR, identifying the Rust frame that was looping in `_platform_memset`. Post-fix sample shows that frame is gone.
5. No reduction in pi-agent message coverage: every conversation indexed includes all of its in-file messages (verified by a spot check against a known multi-message pi session jsonl).

## Out of Scope

- The watch-once single-chunk stall — already fixed by `e429eaab` and PR #233.
- The pi connector's *discovery* path — files are found correctly; the stall is in ingest, not scan.
- `~/.pi/agent/sessions` schema redesign — keep the pi connector contract from `franken-agent-detection` unchanged.
- General memory-pressure work for non-pi connectors. If the root cause turns out to be in shared ingest code (likely candidate: `extra_bin` BLOB building), narrow the fix to the pi path or make the cap a shared knob; do not refactor the broader ingest pipeline.
- WAL / `CASS_DEFER_LEXICAL_UPDATES` interaction. That knob unlocked the codex giant-file path but is unrelated to the pi memset behaviour.

## Selected Shape

Targeted root-cause fix in the pi ingest path, gated on symbolised evidence.

**Phase 1 — Localise (1–2 hours)**
1. Build cass with `cargo build --profile profiling` (debug=true, strip=false).
2. Reproduce the stall: `cass index --watch-once ~/.pi/agent/sessions --json`.
3. While stuck, run `sample <pid> 5 -wait > sample.txt` and resolve Rust frames around the `_platform_memset` site.
4. Inspect the offending conversation (likely identifiable from `lsof <pid>` showing the currently-open jsonl) — record its size + message count + extras-payload size.

**Phase 2 — Fix (1–4 hours, depends on Phase 1)**
- If Phase 1 points at `metadata_bin` or `extra_bin` BLOB construction, add a size cap in the pi-connector normalisation step (similar to `compact_large_connector_extras()` which already runs for `claude_code` and `codex` per `src/indexer/mod.rs:16266`).
- If Phase 1 points at unbounded buffer growth in `ingest_watch_batch_with_oom_split()`, add a pre-flight estimate that splits the batch when projected RSS exceeds a configurable threshold (default 4 GB), reusing the existing OOM-split helper.

**Phase 3 — Verify and upstream (1 hour)**
- Run the full pi backfill, assert acceptance criteria 1–5.
- Bundle the fix into one focused commit (mirror PR #233 style), open a follow-up PR to `Dicklesworthstone/coding_agent_session_search`.
- Update spec 013 `findings-2026-05-14.md` with a cross-reference to this spec so the historical record stays coherent.

Why this shape (and not `/shape`): the symptom is a clear single-thread memory blow-up with a precise reproducer, and the *class* of fix (cap or split) is well-precedented in the same codebase (`compact_large_connector_extras`, `ingest_watch_batch_with_oom_split`). No genuine ambiguity in the solution space — what is uncertain is which specific allocation, and that is a profiling question, not a shaping question.
