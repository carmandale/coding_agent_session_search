---
task: T5
date: 2026-05-15
spec: 014-pi-agent-memset-stall
status: ESCALATION_TO_USER (not a candidate selection)
---

# T5 — Candidate Decision (escalation required)

## TL;DR

The profile evidence from T3/T4 places the dominant memory allocator inside
`frankensqlite_ext_fts5::Fts5Table::snapshot_state` (`fsqlite_ext_fts5/lib.rs:2147–2148`),
called from SQLite's per-row vtab-savepoint plumbing
(`fsqlite_core::Connection::live_vtab_savepoint_all` at `fsqlite_core/connection.rs:10009`).

The plan's Architecture decision tree maps this to **no in-scope candidate**:

| plan.md row | profile fits? | reason |
|---|---|---|
| Cass clone chain → C1/C3 (one-line gate or wrapper) | **NO** | Memset is in fsqlite, not cass `map_to_internal` / packet / MessagePack |
| FAD scan-time → C2 upstream PR or C4 cass streaming parser | **NO** | `lsof` shows zero open jsonls — `fs::read_to_string` already returned |
| Watch-ingest batch construction → C5 pre-flight estimator | **NO** | Memset is per-row inside SQLite savepoints, not at batch boundaries |

The plan's binding rule — "no fix candidate is allowed to defer the peak-RSS
satisfaction to a follow-up" — means we cannot land C1/C3/C5 mitigations and
declare acceptance #2 satisfied. Their cost-reduction is real but they do not
shrink the dominant 30 GB+ FTS5 snapshot allocation.

## What the plan/spec assumed vs. what the evidence shows

| | Plan/spec assumption | Profile evidence |
|---|---|---|
| Allocation site | `NormalizedMessage.extra` clones inside FAD's `pi_agent.rs:485` and cass `map_to_internal` / packet build / MessagePack `extra_bin` (`extra: val.clone()`) | `Fts5Table::snapshot_state` cloning a 30 GB HashMap on every SQLite savepoint |
| Layer | cass `src/indexer/mod.rs` and/or FAD pi connector | `frankensqlite` (pinned at `eba969e`), specifically its bundled FTS5 vtab extension |
| Connector-specific? | Yes — pi only, because of pi-shaped `extra` blobs | No — connector-agnostic. Pi reproduces it because pi runs last, when the in-memory FTS5 index is already populated by claude_code (2,573) + codex (5,712) + opencode (976) + others (≈9.3 K conversations). Any sufficiently-large index hit by enough inserts would show the same shape. |
| Triggered by | Single 72 MB `.jsonl` causing one pathological in-memory `Value` clone | Cumulative HashMap clone cost per-savepoint × per-insert across many small inserts |
| Workable cass-local fix | Yes (C1/C3/C5) or in-cycle upstream (C2/C4) | No cass-local fix shrinks the snapshot cost; the snapshot lives in `fsqlite_ext_fts5` |

## Candidate space (re-derived from real profile)

None of C1–C5 fit. New candidates implied by the actual root cause:

- **D1 (preferred): Fix `frankensqlite_ext_fts5::Fts5Table::snapshot_state` upstream.**
  Replace the eager `InvertedIndex::clone` with one of: (a) copy-on-write
  (`Arc<InvertedIndex>` + diff log on the savepoint stack, lazily applied on
  rollback), (b) journal-only snapshot (record deltas, don't snapshot the full
  state), or (c) skip the snapshot entirely for the cass use case where rollback
  is not actually used. Requires upstream PR on
  `https://github.com/Dicklesworthstone/frankensqlite`. Cleanest fix; would
  benefit every consumer of the crate. Outside spec 014's "stay within
  `src/indexer/`, `src/persist.rs`, or the franken-agent-detection pi connector
  glue" scope.

- **D2: Cass-side workaround — disable the FTS5 vtab savepoint for the
  watch-once ingest path.**
  If `fsqlite_ext_fts5` exposes a runtime knob (or one can be added cheaply)
  to skip the snapshot during high-throughput batch inserts where we don't
  need transactional rollback of FTS5 state, cass can set it for the watch-once
  ingest transaction. Still requires upstream coordination but smaller surface.

- **D3: Cass-side workaround — skip FTS5 indexing during initial backfill,
  run a separate batch FTS5 build at the end.**
  Use `defer_lexical_updates` / `CASS_DEFER_LEXICAL_UPDATES` (already a runtime
  flag in cass per the strings in the release binary) to bypass per-row FTS5
  updates during pi backfill. Likely already partially implemented. Confirms
  whether this is wired into the watch-once path and whether it bypasses the
  snapshot cost, or whether it merely defers different work.

- **D4: Pause to dig deeper.**
  Read `frankensqlite_ext_fts5` source (`~/.cargo/git/checkouts/frankensqlite-*/src/ext/fts5/`)
  to understand the real shape of `snapshot_state`, the rollback contract it
  supports, and the size of the cass-side commit that would land any of D1/D2/D3.
  This is what /codex-shape or a re-scoped spec would normally produce; T5 alone
  doesn't have authority to do it without user buy-in on the scope change.

## What is NOT changed by this finding

- Spec 014's **observable symptom** still matches: pi backfill stalls at 22 GB+ RSS in `_platform_memset`. The new evidence just identifies the allocation site differently and shows the peak can climb past 48 GB given enough wall-time.
- Spec 014's **acceptance criteria** (≥ 1,970 pi conversations, peak RSS < 8 GB, chunk-size preserved, post-fix sample shows no memset hot frame, message coverage) are still the right success conditions. They just can't be hit from within the C1–C5 fix space.
- PR #233 (chunk-size fix) is still correct and is still in the binary used for this profile.
- The pi connector's existing pi_agent=33 rows in the DB are untouched.

## Recommendation

Escalate to user with this evidence. Do not pick a C1–C5 candidate (would
violate the binding "no deferred peak-RSS" rule by shipping a mitigation that
provably does not address the dominant allocator). The user's call is:

1. **Widen the cycle** to land a `frankensqlite_ext_fts5` upstream fix (D1) and
   bump the pinned rev in `Cargo.toml:45` within this spec cycle. Cleanest
   long-term, slowest.

2. **Re-scope spec 014** to the cass-side defer-and-batch path (D3 confirmed
   or D2 worked out). Re-run `/codex-shape` / `/codex-plan` with the new
   evidence; spec.md is amended again.

3. **Spike on D4** (read the FTS5 vtab source) for one Worker session to firm
   up the fix shape before deciding between (1) and (2).

I do not have authority under the current plan to invent a new candidate or
patch a different crate without your input.

## Watcher state (post-T4)

- `com.cass.index-watch` reloaded successfully (PID 30892), forward capture is
  resumed.
- The 50 GB stalled process (87884) has been killed; system memory restored.
- DB pi_agent count unchanged at 33.

## References

- T4 evidence: `specs/014-pi-agent-memset-stall/notes/T4-profile-evidence.md`
- Raw sample: `specs/014-pi-agent-memset-stall/notes/T3-sample.txt` (lines 200–400 are the hot path)
- vmmap: `specs/014-pi-agent-memset-stall/notes/T3-vmmap.txt`
- RSS timeline: `specs/014-pi-agent-memset-stall/notes/T3-monitor.csv`
