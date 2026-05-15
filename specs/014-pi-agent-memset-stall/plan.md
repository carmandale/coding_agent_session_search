---
title: "Plan: pi_agent watch-once memset stall — empirical-gated cass-local fix"
date: 2026-05-15
bead: coding_agent_session_search-373b1
---

<!-- plan:complete:v1 | harness: unknown | date: 2026-05-15T16:23:04Z -->
<!-- Codex Review: APPROVED after 5 rounds | model: gpt-5.3-codex | date: 2026-05-15 | trust_level: full | round_records: .codex-round-91909777/, .codex-round-7f3ae320/, .codex-round-11ebb7d2/, .codex-round-8f680d33/, .codex-round-a8204eb3/ | Status: REVISED -->

# Plan: pi_agent watch-once memset stall

Implementation plan for [`specs/014-pi-agent-memset-stall/spec.md`](spec.md). The spec owns *what* and *why*; this document owns *how*. Acceptance criteria #1 and Requirement 3 were amended during /codex-plan and /codex-review (≥ 1,970 conversations replaces ≥ 2,500; corpus is 2,073 jsonl files; both Acceptance and Requirement texts now consistent) — see spec for provenance.


## Overview

Spec 014 names a clear failure mode but its Selected Shape underspecifies which fix candidate addresses it. Phase A research surfaced four candidates with different cost / reach profiles:

- **C1** — Cass-local glue replacing the bare `src/connectors/pi_agent.rs` re-export with a wrapper that strips or shrinks `NormalizedMessage.extra` after FAD's `scan()` returns. Reduces persist-time and Tantivy pressure; **does not reduce scan-time peak RSS** if the `_platform_memset` frame is inside FAD's `read_to_string` or `val.clone()`.
- **C2** — Upstream PR to `franken_agent_detection` that stops the full-source-JSON clone into `message.extra` (or makes it configurable). Structurally the cleanest peak-RSS fix; multi-repo, slower to ship.
- **C3** — Extend cass's existing `compact_large_connector_extras()` gate from "codex only" to "codex OR pi_agent". Smallest possible diff. Persist-time mitigation only — same peak-RSS limitation as C1.
- **C4** — Cass-owned streaming pi parser replacing the FAD re-export, processing the jsonl line-by-line and never holding the whole file in memory. The only cass-local candidate that reduces scan-time peak. Most invasive cass change. **Discover-source parity is mandatory**: cass's `capture_connector_sources_before_parse()` at `src/indexer/mod.rs:17209` calls the connector's `discover_source_files()` before parsing, and FAD's pi implementation at `pi_agent.rs:557` is what makes raw-mirror linkage work today. C4 must replicate this exactly — empty discover-source means empty raw-mirror, which silently breaks the fidelity contract for every pi conversation (Codex /codex-review round 1 finding). A directory-root raw-mirror regression test is part of T13.
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
4. **`tests/` — regression coverage.** Today: one 8-line pi fixture (`tests/fixtures/pi_agent/sessions/.../...jsonl`) and a 1000-message connector test at `tests/connector_pi_agent.rs:997`. Plan adds a separate synthetic large-pi-conversation fixture (one synthesized jsonl with high message count and large `toolCall.arguments` / `thinking` content) plus a unit test that proves: (i) post-fix peak RSS stays under a cap on that fixture, (ii) post-fix `message.extra` preserves model + attachments + token-usage, (iii) raw-mirror linkage remains intact. (Earlier "one 3-line fixture" claim corrected during /codex-review round 2.)

## Caller-preservation contract for `message.extra`

Codex's Phase A challenge identified five sites in cass that consume `NormalizedMessage.extra` downstream. The chosen fix candidate must preserve their inputs:

| Site | What it reads | Preservation requirement |
|------|---------------|-------------------------|
| `src/model/conversation_packet.rs:513` | `extra_json` for packet body | Strip raw provider JSON; keep `cass.*` slot-typed fields (model, token usage, attachments) |
| `src/model/conversation_packet.rs:668` | Participates in packet hash | Hash is computed AFTER compaction — verified by writing the hash assertion in the new regression test before the fix lands |
| `src/indexer/mod.rs:18763` | `Message.extra_json` with redaction | Compactor must emit the same `cass.*` envelope shape so redaction logic is unchanged |
| `src/storage/sqlite.rs:10309` | Token / model analytics | Token usage MUST be preserved in `cass.token_usage`. The current codex compactor at `src/indexer/mod.rs:17577` clones existing `cass` metadata wholesale, and the test at `src/indexer/mod.rs:34337` asserts `cass.token_usage` survives — so token-usage preservation is already in the existing pattern; the pi extension inherits this for free. (Stale earlier claim corrected during /codex-review round 2.) |
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

Observed result: ran `find ~/.pi/agent/sessions -name "*.jsonl" -type f | wc -l` (exit 0, output `2073`); ran `find ~/.pi/agent/sessions -name "*.jsonl" -type f -exec stat -f "%z" {} \; | awk '{t+=$1} END {printf "%d (%.2f GB)\n", t, t/1024/1024/1024}'` (exit 0, output `1829922393 (1.70 GB)`); ran `find ~/.pi/agent/sessions -name "*.jsonl" -exec stat -f "%z %N" {} \; | sort -rn | head -1` (exit 0, biggest file `71921064` bytes ≈ 72 MB at `~/.pi/agent/sessions/--Users-dalecarman-Groove Jones Dropbox-Dale Carman-Projects-dev-generator--/2026-03-23T16-29-33-049Z_e8774ea3-cad5-40a8-9f5b-e0e8ec3c931f.jsonl`). Codex Phase A round-2 read of FAD source at `~/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:351` and `:485` confirmed FAD does `fs::read_to_string(file)` then `extra: val.clone()` per message — entire source JSON line duplicated into each message's `extra` field. Cass then clones `extra` again at `src/indexer/mod.rs:18763` (map_to_internal), MessagePack-serializes for `extra_bin` in the persist path, and clones for lexical packets at `src/model/conversation_packet.rs:513` — a 4-times-duplicated path before persistence.

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

