---
boundary_timestamp: 2026-05-15T162233Z
phase: B
sha: 05ba881b
plan_hash: 2a416815
plan_sha256_full: 2a416815ec6c5ca75593496afe36652926ca49ae5894ac3d28476d9e8d12a444
---

# SPEC-SNAPSHOT-BEGIN
---
title: "pi_agent watch-once stalls after first chunk with 22 GB RSS memset loop"
date: 2026-05-15
bead: coding_agent_session_search-373b1
---

<!-- issue:complete:v1 | harness: unknown | date: 2026-05-15T15:12:28Z -->

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
3. Pi-agent historical backfill (`cass index --watch-once ~/.pi/agent/sessions`) must complete without a manual kill on the user's corpus (≥ 2,800 sessions). Acceptance is reaching `success: true` with `conversations >= 2,500`.
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

# SPEC-SNAPSHOT-END

# PLAN-SNAPSHOT-BEGIN

## Overview

Spec 014 names a clear failure mode but its Selected Shape underspecifies which fix candidate addresses it. Phase A research surfaced four candidates with different cost / reach profiles:

- **C1** — Cass-local glue replacing the bare `src/connectors/pi_agent.rs` re-export with a wrapper that strips or shrinks `NormalizedMessage.extra` after FAD's `scan()` returns. Reduces persist-time and Tantivy pressure; **does not reduce scan-time peak RSS** if the `_platform_memset` frame is inside FAD's `read_to_string` or `val.clone()`.
- **C2** — Upstream PR to `franken_agent_detection` that stops the full-source-JSON clone into `message.extra` (or makes it configurable). Structurally the cleanest peak-RSS fix; multi-repo, slower to ship.
- **C3** — Extend cass's existing `compact_large_connector_extras()` gate from "codex only" to "codex OR pi_agent". Smallest possible diff. Persist-time mitigation only — same peak-RSS limitation as C1.
- **C4** — Cass-owned streaming pi parser replacing the FAD re-export, processing the jsonl line-by-line and never holding the whole file in memory. The only cass-local candidate that reduces scan-time peak. Most invasive cass change.
- **C5** — Spec's named candidate (b): per-conversation memory-pressure pre-split in `ingest_watch_batch_with_oom_split()`. Estimate batch byte size (sum of message.content + message.extra serialized sizes) before persist; if over a threshold, recursively halve preemptively instead of waiting for OOM. Mitigates persist + Tantivy memory pressure for *any* connector, not just pi. Per-conversation single-conv stalls still need a quarantine path because a single 72 MB conv can't be split further.

The candidate selection is **gated on Phase 1 profiling**. Decision tree downstream:

| Profile points at | In-cycle fix to ship | Acceptance #2 reach |
|--------------------|----------------------|---------------------|
| Cass clone chain (`map_to_internal`, MessagePack serialize, lexical packets) | C3 alone, or C3 + C5 if batch-byte pressure also shows | satisfies #2 fully |
| FAD scan-time (`read_to_string`, `val.clone()`) | C4 cass-owned streaming parser, **OR** the C2 upstream FAD PR must land within this spec cycle (not deferred to a follow-up) with a dependency bump to the fixed rev. C3 is mitigation-only and is NOT sufficient for acceptance #2. | satisfies #2 only via C4 or C2-landed-in-cycle |
| Watch-ingest batch construction inside cass | C5 with quarantine path for single-conv stalls | satisfies #2 fully |

**No fix candidate is allowed to defer the peak-RSS satisfaction to a follow-up.** If the profile says FAD-scan, this spec cycle ships either C4 or waits for C2 to merge upstream and re-runs verification before /finalize.

## Shape Comparison

R0 gate: compared three shapes on net-complexity to select Shape X.

### Shape X — Empirical-gated single fix, no deferred peak-RSS (SELECTED)

Run the profiling repro, identify the allocation site, then implement the single smallest candidate that meets all five acceptance criteria **in-cycle**. If the profile points at the cass-side clone chain, ship C3 (smallest) or C1 (small-medium) alone. If it points at watch-ingest batch memory pressure, ship C5. If it points at FAD scan-time peak, ship C4 (cass-owned streaming parser) OR ensure the C2 upstream FAD PR lands and the dep rev is bumped within this spec cycle's window. The decision tree in Architecture is binding: no "ship mitigation + defer the real fix" path is acceptable.

- Net complexity: low-to-medium (one focused diff if cass-local; medium if C4 streaming parser or C2 upstream coordination)
- Time to ship: 1–3 days for cass-local candidates; up to 5 days if C4 or C2-coordinated path is needed
- Risk: low (profile decides; the in-cycle rule prevents scope deferral)
- Acceptance reach: satisfies all five acceptance criteria **in this spec cycle**, by construction (the decision tree forbids paths that don't)

### Shape Y — Parallel C3-mitigation + FAD upstream PR

Implement C3 immediately (extend cass compactor to pi), in parallel draft and submit FAD upstream PR for C2. Don't wait for profiling.

- Net complexity: medium (two diffs in two repos in parallel)
- Time to ship: similar to Shape X but with more concurrent work
- Risk: ships C3 without knowing if it's the right fix — if profiling later proves C3 is irrelevant (FAD-only peak), we ship a no-op patch and confuse the audit trail
- Acceptance reach: weak — could ship a code change that doesn't actually pass acceptance #2 (peak RSS < 8 GB)

### Shape Z — Defer until FAD upstream fix lands

Don't ship anything cass-side; file FAD upstream issue and wait. Document the workaround as "skip pi historical backfill, rely on watcher for forward capture only."

- Net complexity: lowest (no cass change)
- Time to ship: indefinite (depends on upstream)
- Risk: leaves the user without the 2,073 historical pi conversations indefinitely
- Acceptance reach: zero — none of the spec acceptance criteria are met

**Shape X selected.** Empirical gate prevents wrong-target fixes; the binding decision tree (no deferred peak-RSS) ensures acceptance #2 is satisfied in-cycle regardless of which allocation site the profile points at.

## Architecture

The plan operates at four cass surfaces, each with explicit allowed scope:

1. **`src/indexer/mod.rs` — extras compactor and gate.** Today: codex-only gate (line 17574), compacts `message.extra` only. Candidate C3 broadens the gate; candidate C1 swaps the call site; candidate C4 replaces the connector source.
2. **`src/connectors/pi_agent.rs` — pi connector re-export.** Today: two lines. C1 replaces with a thin wrapper struct that calls FAD's `scan()` and post-processes; C4 replaces with a cass-owned streaming parser.
3. **`src/raw_mirror.rs` — raw mirror manifest and linkage.** Today: captures source bytes pre-parse, links conversation-level post-parse. **No changes** required — we only need to prove the linkage survives the chosen compaction.
4. **`tests/` — regression coverage.** Today: one 3-line pi fixture. Plan adds a synthetic large-pi-conversation fixture (one synthesized jsonl with high message count and large `toolCall.arguments` / `thinking` content) plus a unit test that proves: (i) post-fix peak RSS stays under a cap on that fixture, (ii) post-fix `message.extra` preserves model + attachments + token-usage, (iii) raw-mirror linkage remains intact.

## Caller-preservation contract for `message.extra`

Codex's Phase A challenge identified five sites in cass that consume `NormalizedMessage.extra` downstream. The chosen fix candidate must preserve their inputs:

| Site | What it reads | Preservation requirement |
|------|---------------|-------------------------|
| `src/model/conversation_packet.rs:513` | `extra_json` for packet body | Strip raw provider JSON; keep `cass.*` slot-typed fields (model, token usage, attachments) |
| `src/model/conversation_packet.rs:668` | Participates in packet hash | Hash is computed AFTER compaction — verified by writing the hash assertion in the new regression test before the fix lands |
| `src/indexer/mod.rs:18763` | `Message.extra_json` with redaction | Compactor must emit the same `cass.*` envelope shape so redaction logic is unchanged |
| `src/storage/sqlite.rs:10309` | Token / model analytics | Token usage MUST be preserved in `cass.token_usage` (current codex compactor preserves `cass.model` and `cass.attachments` but NOT token usage — pi extension must add this) |
| `src/pages/export.rs:351` | Model + attachment refs for static export | Same `cass.*` envelope; existing test fixture pattern from `large_codex_extra_compaction_preserves_cass_metadata()` is the template |

This contract is enforceable as a single unit test: build a normalized pi conversation with raw `extra`, run the compactor, assert (a) `extra` JSON byte-size is bounded, (b) `cass.model`, `cass.token_usage`, `cass.attachments` survive, (c) packet hash computed on compacted form matches a fixture-pinned expected hash.

## Raw-mirror fidelity argument

`capture_connector_sources_before_parse()` copies the full pi `.jsonl` bytes to the raw mirror **before** FAD's `scan()` runs. `attach_raw_mirror_capture()` links each conversation to its source blob by `source_path`. After compaction, `source_path` is unchanged, so the linkage holds.

The raw mirror does NOT store per-message line offsets. Reconstructing exact pre-compaction `message.extra` requires re-parsing the source jsonl through FAD's pi parser logic. This is not a per-message-byte recovery contract — it is a "source bytes retained, parser logic available, recovery possible if needed" contract. Acceptance: the regression test reconstructs `message.extra` from the raw-mirror blob using a known parser version and asserts equivalence with the pre-compaction Value. If FAD's parser changes upstream, the recovery contract degrades to "source bytes are retained; if you need pre-compaction extras, run the fixed-version cass against the raw mirror." That degradation is acceptable for this fix.

## Order-of-operations check

Compaction runs at `src/indexer/mod.rs:16266` (`compact_large_connector_extras("", conv)`). Raw-mirror linkage runs at `src/indexer/mod.rs:16267` (`attach_raw_mirror_capture(&opts.data_dir, conv)`). Today compaction operates only on `conv.messages[].extra` and does not touch `source_path` or anything `attach_raw_mirror_capture` reads. Any chosen candidate must preserve this — verified by reading the function bodies before the regression test is written.

## Plan Sanity Evidence

Objective: ship a focused cass-local fix that lets `cass index --watch-once ~/.pi/agent/sessions` complete the full pi-corpus backfill on the user's machine with peak RSS under 8 GB and no regression to claude_code / codex / opencode coverage.

Riskiest assumption: the bytes driving the `_platform_memset` frame live in cass-reachable allocations (post-FAD-scan `Vec<NormalizedConversation>`, `map_to_internal` clones, MessagePack serialization). If false — peak is inside FAD's `fs::read_to_string` or `serde_json::Value` parse before cass sees data — then cass-only candidates (C1, C3, C4-without-streaming) reduce persist pressure but not peak RSS, and acceptance #2 (peak < 8 GB) requires either C2 upstream PR or C4 streaming parser.

Smallest probe: read FAD's `~/.cargo/git/checkouts/franken_agent_detection-*/src/connectors/pi_agent.rs` lines 341-500 to identify the actual scan-time allocation pattern; corroborate with `find ~/.pi/agent/sessions -name "*.jsonl" -exec stat -f "%z" {} \;` to size the corpus and worst-case file.

Observed result: FAD pi connector does `fs::read_to_string(file)` (one allocation per file, up to 72 MB), parses per line into `serde_json::Value`, then `extra: val.clone()` per message — duplicates the entire source JSON line into each message's `extra` field. Pi corpus: 2,073 `.jsonl` files, 1.70 GB total, biggest 72 MB. cass downstream then clones `extra` again in `map_to_internal` + MessagePack-serializes for `extra_bin` + clones for lexical packets — a 4-times-duplicated path before persistence.

Decision impact: if the probe result had been "FAD streams line-by-line and clones only slot-typed fields" (i.e. the bloat is purely a cass-side clone chain), `tasks.md ## Group C — Implement chosen candidate` would default to C3 (extend `compact_large_connector_extras` to pi, one-line gate change) and the FAD upstream PR would be irrelevant. With the actual probe result (FAD reads whole file and clones full JSON Value), the binding decision tree in `plan.md ## Architecture` requires either C4 (cass-owned streaming parser, written in this spec cycle) or C2 (upstream FAD PR landed and dep rev bumped within this cycle); `tasks.md ## Group C T9` enumerates both routes and `T9a` runs only if C2 is selected. Either way, `plan.md ## Architecture` adds raw-mirror replay as the fidelity contract for the compacted/streamed pi rows.

## Out of scope

- The `stall_detected` watchdog gap. The existing watchdog from #196/v0.3.7 monitors lock-heartbeat freshness; pi keeps the heartbeat alive even when DB row growth stops. A separate bead will capture "watchdog should also detect DB-row-growth stalls."
- Schema migrations. No candidate currently changes schema. If C4 (streaming parser) later requires schema changes (unlikely), that's a separate slice.
- claude_code / codex / opencode behaviour. Existing test fixtures at `mod.rs:34308+` cover the codex compactor path; pi extension must not change codex behaviour.
- The watcher daemon's incremental capture path. Forward capture from pi sessions modified after the fix lands works because the chunked ingest loop is already correct (PR #233). This plan only addresses historical backfill.

## Risks (mapped to spec.md Risks + Phase A/B discoveries)

- **Profile points at FAD scan-time.** C3 alone cannot satisfy acceptance #2 in this case. Required in-cycle response: ship C4 (cass-owned streaming parser) **or** land C2 upstream FAD PR within the spec cycle and bump the dependency rev before verification. Deferring acceptance #2 to a follow-up spec is explicitly disallowed (see decision tree in Architecture). If neither C4 nor in-cycle C2 is feasible, escalate to user to either widen the cycle or accept a different acceptance threshold.
- **Caller-preservation contract slips.** Hash-affecting compaction changes packet identity, which can confuse dedupe. Mitigated by including the packet-hash assertion in the regression test before fix lands.
- **Corpus ceiling.** Spec amendment to acceptance #1 from "≥ 2,500" to "≥ 95 % of 2,073 = 1,970 with documented skips" is a Group A precondition.
- **Watcher uptime during verification.** Bounded: profiling and full-corpus runs each take 10–60 minutes; watcher is stopped and reloaded around them.

## Test plan

- Unit: caller-preservation contract on a synthetic large-pi conversation fixture (Group E).
- Unit: post-compaction packet hash stability against fixture-pinned expected hash.
- Unit: raw-mirror linkage assertion (source_path lookup returns the pre-compaction blob).
- Integration: regression test that loads the synthetic fixture, runs ingest, asserts peak RSS < cap, and asserts all messages persisted.
- Manual: full pi corpus run on the user's machine (`cass index --watch-once ~/.pi/agent/sessions --json --no-progress-events`); peak RSS captured every 60 s; acceptance #1 + #2 + #4 + #5 verified.
- Regression: existing claude_code / codex / opencode test fixtures must continue to pass.

---


# PLAN-SNAPSHOT-END
