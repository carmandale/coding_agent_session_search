---
boundary_timestamp: 2026-05-15T223651Z
phase: B
sha: 52e96656
plan_hash: 06607ecc
plan_sha256_full: 06607ecc2e191487b79981036f48129ee3c8f5ac74a9c787be1d4c275fd48181
---

# SPEC-SNAPSHOT-BEGIN
---
title: "watch-once scan must stream like the watcher, not bulk-materialize the corpus"
date: 2026-05-15
bead: coding_agent_session_search-81z91
---

<!-- issue:complete:v1 | harness: unknown | date: 2026-05-15T21:14:56Z -->

## Source (verbatim)

> "historical sessions are more than desired. they are the whole point. cass is only about history. without historical sessions you have nothing. but it doesn't have to happen instantly. I don't care if it takes 2 weeks. what is the right way to do it? if 1 works....if 33 work, than 2700 will work" — user, 2026-05-15

Anchor quotes from the broader `/goal` context that pre-date this spec:

> "the goal is to be in sync with upstream and running properly, capturing all sessions and cass working." — user, 2026-05-13

> "claude code and codex are top priorities, with pi-agent next and then opencode. no other agents are priorities, but if we can get every historical session, that is desired." — user, 2026-05-14

## Problem

`cass index --watch-once <dir>` materialises the entire corpus into a `Vec<NormalizedConversation>` in memory *before* any persist runs. For the user's pi-agent corpus (2,073 jsonl files, ~1.7 GB on disk, max file 72 MB), that materialisation produces tens of GB of in-memory state on top of the already-large in-memory FTS5 inverted index (~30 GB at the user's current corpus size of ~9,300 indexed conversations across claude_code / codex / opencode). The indexer wedges before any pi conversation actually lands in the DB.

Code site: `src/indexer/mod.rs:16248` — `let mut convs = match conn.scan(&ctx)` returns a single `Vec` containing every conversation produced by the connector. Lines 16263–16268 then mutate every element in that vec (`inject_provenance`, `apply_workspace_rewrite`, `compact_large_connector_extras`, `attach_raw_mirror_capture`) before any of them are persisted. The chunking that PR #233 added applies to the *persist* loop after this — too late to bound peak memory.

The forward-capture watcher does not have this problem. It processes one filesystem event at a time: one new conversation, one persist, working set freed before the next event. The user observation "if 1 works, if 33 work, then 2700 will work" is structurally correct — the per-conversation memory cost is bounded; the bulk-materialisation pile-up is what blocks pi backfill.

The Connector trait already exposes the right primitives for a streaming fix (see `~/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/mod.rs:62-104`):

- `Connector::scan_with_callback(ctx, &mut on_conversation)` — emits conversations incrementally; default impl falls back to `scan()` but connectors that can truly stream override it (`supports_streaming_scan()` returns `true`).
- `Connector::discover_source_files(ctx) -> Vec<DiscoveredSourceFile>` — cheap pre-parse file enumeration; pi connector already implements this (`pi_agent.rs:557`).

Cass already uses `scan_with_callback` in two non-watch-once paths (`src/indexer/mod.rs:8534, 8596`) and gates code on `supports_streaming_scan()` at `:9188`. Watch-once is the holdout that still calls bulk `scan()`.

This is the same conceptual fix as PR #233 (chunked persist), extended one level up into the scan phase.

## Requirements

1. The watch-once code path in `src/indexer/mod.rs` must process the corpus in bounded chunks, not as a single `Vec<NormalizedConversation>`. After each chunk persists, that chunk's working set must be droppable so the next chunk's allocations do not stack on top of the previous chunk's.
2. The fix must work for the pi connector specifically. Either (a) cass uses `scan_with_callback` and the pi connector grows a true streaming implementation upstream in `franken_agent_detection`, or (b) cass uses `discover_source_files` + per-file-batch `scan()` calls that stay within the connector trait as it exists today. The plan picks one route; either is acceptable.
3. Forward-capture watcher path must remain unchanged. The chunked watch-once code path may share helpers with the watcher, but the watcher's existing per-event flow must not regress.
4. PR #233's chunked-persist behaviour must remain. The new scan-side chunking sits *above* the persist chunking and respects the same `CASS_WATCH_INGEST_CHUNK_SIZE` semantics (or names a new env knob for the scan chunk size and explains the interaction).
5. The chunk-size knob must be tunable from the environment. Default value must keep watch-once on a non-pi connector (claude_code, codex, opencode) producing the same or better wall-clock and DB outcome as today.

## Constraint

- **Cass-side change only.** No changes to `franken_agent_detection` or `frankensqlite` are *required* by this spec. The plan may *propose* an upstream FAD streaming impl for the pi connector as an optimisation, but acceptance must be reachable without it (route (b) above).
- **Single source-of-truth binary.** Whatever lands ships through `~/.local/bin/cass.real` and the launchd watcher daemon, same as PR #233.
- **No destructive recovery.** Existing pi_agent rows in the DB (currently 36 as of 2026-05-15 14:44) must not be lost. Backfill is additive.
- **No regression of spec 013 chunk-size fix.** PR #233's `CASS_WATCH_INGEST_CHUNK_SIZE` behaviour is preserved; the new scan chunking composes with it, not replaces it.
- **Spec 014's 8 GB peak-RSS threshold is explicitly dropped for this spec.** The fsqlite_ext_fts5 in-memory inverted-index floor (~30 GB at current corpus size) is structurally above 8 GB and is *not* what this spec fixes. A separate follow-up (track in a new bead against `frankensqlite`) covers persisting the FTS5 index to disk.
- **Watcher uptime.** The launchd `com.cass.index-watch` daemon keeps running during this work. Stop it only inside tasks that explicitly need the index-run lock (verification runs); reload before the task closes.

## Acceptance Criteria

1. `cass index --watch-once ~/.pi/agent/sessions --json --no-progress-events` on the user's machine completes with `success: true` and indexes **≥ 1,970 pi conversations** (≥ 95 % of 2,073 discovered jsonl files; any skipped files must be recorded in `<data_dir>/quarantine/watch_ingest_poison.jsonl` and reflected in the run receipt). Matches spec 014 Acceptance #1 verbatim.
2. Peak working-set growth *attributable to scan materialisation* must be bounded. Concretely: peak RSS during the pi backfill run minus the steady-state in-memory FTS5 floor (measured pre-run from the running watcher) must be ≤ N × per-file-cost for some configurable N (the new scan chunk size). The plan picks N and the measurement procedure; default N must produce a sub-1-GB delta.
3. Spec 013's chunk-size behaviour (PR #233) is preserved. Regression test (or extension of the existing spec 013 test) demonstrates `CASS_WATCH_INGEST_CHUNK_SIZE` still works.
4. Forward-capture watcher behaviour is unchanged. Existing watcher tests pass. Spot-check: the watcher (PID currently 37121) continues to index any new pi sessions that land in `~/.pi/agent/sessions/` while watch-once is not running.
5. No reduction in pi-agent message coverage. Same harness-based message-count check from spec 014 Acceptance #5 (Rust harness using the pinned FAD `PiAgentConnector`).

## Out of Scope

- Spec 014's 8 GB peak-RSS threshold (see Constraint section — explicitly dropped). The fsqlite_ext_fts5 in-memory floor is a separate problem.
- The fsqlite_ext_fts5 architectural rewrite to persist the inverted index to shadow tables. Filed as a follow-up bead against the `frankensqlite` repo.
- PR #90 (the savepoint-clone fix upstream in frankensqlite). Still merges on its own merits; orthogonal to this spec.
- The contentless-table side-finding bead (`coding_agent_session_search-d907f`). Separate fix path.
- Connector trait changes in `franken_agent_detection`. Spec deliberately stays cass-side per route (b); upstream FAD streaming for pi is an optimisation, not required.

## Selected Shape

**Direct root-cause fix in `src/indexer/mod.rs` watch-once code path with focused regression coverage**, modelled on PR #233's chunk-the-persist pattern extended one level up. Specifically, the watch-once scan loop at `:16248` is replaced with one of:

- **Shape A — `discover_source_files` + per-batch `scan()`** (cass-only, no upstream dependency). Call `connector.discover_source_files(&ctx)` once to enumerate jsonl files. Walk the list in chunks of N files. For each chunk, construct a sub-`ScanContext` scoped to just those N files (mechanism TBD by `/codex-plan` — may require a small `ScanContext` API extension if file-scoped contexts aren't supported today; that's *still* cass-side because `ScanContext` lives in cass or FAD depending on the trait, plan verifies). Call `connector.scan(&sub_ctx)`. Apply the existing transformation loop. Persist via the existing chunked-persist path. Drop the chunk's `Vec` and move on. **Pro**: no FAD coordination. **Con**: invokes `scan()` per chunk so the per-chunk overhead is higher than a true streaming callback.

- **Shape B — `scan_with_callback` with per-callback batching** (uses existing trait, but optimal only after FAD pi-connector gains a real streaming impl). Call `connector.scan_with_callback(&ctx, &mut |conv| { … })`. Inside the callback, push `conv` into a buffer; when the buffer reaches N entries, run the transformation loop on it, persist via the existing chunked-persist path, drop the buffer, return `Ok(())`. **Pro**: smaller per-conversation overhead. **Con**: until FAD's `PiAgentConnector` overrides `scan_with_callback` upstream, pi still hits the default impl that materialises via `scan()` — so peak memory benefit is delayed. (`supports_streaming_scan()` returns `false` for pi today; cass should check and route to Shape A for non-streaming connectors.)

`/codex-plan` picks the route. Likely outcome: Shape A as the cass-side baseline (works immediately for pi), with a stub for Shape B that activates automatically when `supports_streaming_scan()` returns `true`, so the cass side is forward-compatible with a future FAD streaming PR.

Regression coverage: extend `tests/connector_pi_agent.rs` (or add a sibling) with a fixture-based test that asserts the new scan path persists conversations incrementally — observable via a fixture connector that emits N conversations and a custom storage that counts the number of distinct persist transactions or peak working-set size.

## Decision tree downstream

| Profile / observation | Shape ships |
|---|---|
| `discover_source_files` exists for every active connector, and per-file ScanContext is a one-liner in cass | Shape A as default |
| Connector trait gains a true streaming impl for pi within this cycle | Shape B; Shape A becomes the fallback |
| Connector trait API forces an awkward sub-ScanContext shape | Shape B + temporary FAD-side override file for pi only |

Plan verifies which row applies before settling on the diff.

# SPEC-SNAPSHOT-END

# PLAN-SNAPSHOT-BEGIN

---
title: "Plan: watch-once scan must stream like the watcher, not bulk-materialize the corpus"
date: 2026-05-15
bead: coding_agent_session_search-81z91
---

# Plan: watch-once streaming scan (spec 015)

Implementation plan for [`specs/015-watch-once-streaming-scan/spec.md`](spec.md). Spec owns *what* and *why*; this document owns *how*. Bead `coding_agent_session_search-81z91`.

## Overview

Change cass's `--watch-once` code path so it no longer materialises the entire corpus as `Vec<NormalizedConversation>` before any persist runs. The per-conversation memory cost is already bounded (the forward-capture watcher proves this every time it ingests a single new pi session). The watch-once path is the holdout that piles everything up first. This plan extends PR #233's chunk-the-persist pattern one level up into the scan phase: process N files per batch, persist them, drop the working set, repeat.

The user's framing is the load-bearing argument: "if 1 works, if 33 work, then 2700 will work." Pi already has 36 conversations indexed via the watcher (forward capture, one event at a time). Scaling that pattern to 2,073 files via a chunked watch-once loop is the smallest change that produces the user-visible outcome (pi historical backfill completes).

Spec 014's 8 GB peak-RSS threshold is explicitly out of scope for this spec — it requires a separate upstream architectural change in `fsqlite_ext_fts5`. Spec 015's acceptance #2 measures the *delta* the watch-once scan adds on top of the steady-state FTS5 floor, not absolute RSS.

## Shape Comparison

R0 gate: comparing three shapes on net complexity.

### Shape X — Per-batch hardlink/copy scratch root + source_path remap (SELECTED, Route 5)

Cass enumerates pi files via `connector.discover_source_files(&ctx)`, processes them in batches of N. For each batch, cass builds a real (non-symlink) scratch directory tree mirroring the canonical `sessions/<workspace-encoded>/<file>` layout, populated via `std::fs::hard_link` with copy fallback for cross-device cases. Cass calls `connector.scan(&scratch_ctx)`. FAD's `sessions_dir` resolution at `pi_agent.rs:74-105` produces `<scratch>/sessions`; FAD's `external_id` derivation at `pi_agent.rs:322-348` produces `<workspace>/<file>` — bit-for-bit identical to a full-root scan. After scan, cass remaps each emitted `NormalizedConversation.source_path` back from the scratch path to the original `~/.pi/agent/sessions/<workspace>/<file>` path before per-conversation transformations, persist, and raw-mirror linkage. The pre-parse raw-mirror capture (`capture_connector_sources_before_parse` at `src/indexer/mod.rs:16237`/`:17209`) runs ONCE against the original `~/.pi/agent/sessions` root at the top of the run, BEFORE per-batch scratch scans — this preserves manifest identity (`src/raw_mirror.rs:844, :2000` key off original paths). FAD parser stays the single source of truth — no parser duplication.

- Net complexity: low-to-medium. New code: a scratch-root builder helper (~80 LOC), a source_path remap helper (~30 LOC), and modifications inside `do_index_run` / the watch-once branch at `src/indexer/mod.rs:16237-16420` (~150 LOC). No FAD or frankensqlite changes.
- Time to ship: 1-3 working sessions for the diff + verification run.
- Risk: low. External-id equivalence verified at `pi_agent.rs:74-105, :322-348`. Symlink rejection at `src/indexer/mod.rs:17306-17317` does NOT apply (hardlinks/copies are regular files). FAD parser remains the oracle — Acceptance #5 trivially holds because the persist path sees exactly the same `NormalizedConversation` shape FAD would emit.
- Acceptance reach: hits #1 (≥1,970 pi conversations indexed), #2 (sub-1-GB scan-side delta), #3 (PR #233 persist chunking preserved — nested below scan chunking), #4 (forward-capture watcher untouched — streaming change gated to `explicit_watch_once && kind.slug() == "pi_agent" && !discovered.is_empty()`; everything else stays on the bulk path), #5 (no message-coverage regression — FAD parser handles flattening).

### Shape Y — `scan_with_callback` + upstream FAD pi streaming override (Route 2 / Shape B)

Cass calls `connector.scan_with_callback(&ctx, &mut |conv| ...)` and buffers per-callback. The pi connector's default `scan_with_callback` impl materialises via `scan()` first — equivalent to the bulk path — so this only wins if FAD's `PiAgentConnector` grows a true streaming override (drop the `Vec` push at `pi_agent.rs:479-488`, replace with callback invocation).

- Net complexity: medium. Cass-side change is small (mirrors `:8534`). FAD-side change is also small but requires upstream coordination, PR review, merge, dep bump.
- Time to ship: 1-3 weeks (upstream coordination dominates).
- Risk: low for the implementation; high for cycle-fit because spec 015 hard-constraints acceptance reachable WITHOUT FAD changes.
- Acceptance reach: cleaner long-term, but blocked on upstream maintainer schedule. Not viable as spec 015's spec-cycle baseline.

### Shape Z — Status quo + tighter persist chunking

Shrink `WATCH_INGEST_DEFAULT_CHUNK_SIZE` from 32 to 1 (`src/indexer/mod.rs:89`). Does NOT help — bulk-materialisation happens BEFORE persist chunks. Smaller persist chunks just spread the same memory pressure over more transactions.

- Net complexity: trivial (one-line change).
- Risk: high — does not address the bottleneck. Pi backfill still stalls.
- Acceptance reach: none.

**Shape X selected.** Route 5 cleanly implements spec 015's Selected Shape A ("`discover_source_files` + per-batch `scan()`") without exceeding scope, without FAD coordination, and without external-id / parser-duplication risks. Phase A research evaluated Routes 1-5 across 7 rounds of adversarial review; Route 5 is the only candidate that satisfied every spec constraint.

## Architecture

Four cass surfaces change; no other crates touched.

### 1. `src/indexer/mod.rs` watch-once branch at `:16237-16420`

Current bulk structure:

```
capture_connector_sources_before_parse(ctx, ...)       // preparse raw-mirror capture
let convs = conn.scan(&ctx)?;                          // materialises entire corpus
for conv in &mut convs { /* per-conv transforms */ }   // in-place transforms
progress.total.fetch_add(convs.len(), ...);            // counter set after full scan
for chunk in convs.chunks(ingest_chunk_size) {         // PR #233 chunked persist
    ingest_watch_batch_with_oom_split(...)
    t_index.commit()
    save_watch_state_watermark(...)  // only if !explicit_watch_once
}
```

New streaming structure (gated to `explicit_watch_once && kind.slug() == "pi_agent" && !discovered.is_empty()`):

```
let discovered = conn.discover_source_files(&ctx).unwrap_or_default();

// Connector-specific gate (Phase B Round 2 correction):
// Streaming via Route 5 assumes pi's `sessions/<workspace>/<file>` layout
// and FAD pi's sessions_dir/external_id derivation. claude_code, codex, and
// opencode also implement discover_source_files but have DIFFERENT canonical
// paths and DIFFERENT external_id derivations. Capability-detection alone
// (Phase B Round 1) would mis-route those connectors into a scratch-root
// that doesn't match their layout. Gate explicitly to the pi_agent connector
// kind. Future connectors that want streaming need their own scratch-root
// contract OR a generic connector-side hook (out of scope for spec 015).
if !explicit_watch_once || kind.slug() != "pi_agent" || discovered.is_empty() {
    // FALL THROUGH to the existing bulk Vec path (unchanged)
    return existing_bulk_watch_once(...);
}

// One preparse capture against the ORIGINAL root (NOT under any scratch ctx).
// Raw-mirror's capture_connector_sources_before_parse reads discover_source_files
// and copies+hashes the original source bytes; that must key off the original
// path (raw_mirror.rs:844, :2000) for manifest identity.
capture_connector_sources_before_parse(&original_ctx, ...)

progress.total.fetch_add(discovered.len(), ...);

let original_sessions_root = derive_sessions_root_for(&original_ctx, kind);

// Counters accumulated incrementally across batches
let mut emitted_source_files: HashSet<PathBuf> = HashSet::new();
let mut emitted_conversations: usize = 0;
let mut ingest_success_conversations: usize = 0;
let mut quarantined_oom: usize = 0;
let mut inserted_messages: usize = 0;

for file_batch in chunk_by_files_and_bytes(&discovered, scan_batch_limits) {
    let scratch = build_scratch_root(&file_batch, &workdir, &original_sessions_root)?;
    let scratch_sessions_root = scratch.path().join("sessions");
    let scratch_ctx = ScanContext::with_roots(
        data_dir.clone(),
        vec![ScanRoot::from(scratch.path())],  // <scratch> — FAD resolves sessions_dir to <scratch>/sessions
        since_ts,
    );
    let mut scratch_convs = conn.scan(&scratch_ctx)?;

    for mut conv in scratch_convs.drain(..) {
        remap_source_path(&mut conv, &scratch_sessions_root, &original_sessions_root);
        inject_provenance(&mut conv, &root.origin);
        apply_workspace_rewrite(&mut conv, &root);
        compact_large_connector_extras("", &mut conv);
        attach_raw_mirror_capture(&data_dir, &mut conv);  // sees canonical source_path
        emitted_source_files.insert(conv.source_path.clone());
        emitted_conversations += 1;
        buffer.push(conv);

        if buffer.full_by_count_or_bytes_or_messages() {
            flush_buffer(&mut buffer, &mut counters, storage, t_index, ...)?;
        }
    }
    // scratch dropped here — RAII cleanup
}
// Final flush
if !buffer.is_empty() {
    flush_buffer(&mut buffer, &mut counters, storage, t_index, ...)?;
}
```

The streaming branch fires when ALL of: `explicit_watch_once == true`, `kind.slug() == "pi_agent"`, AND `conn.discover_source_files(&ctx)` returned a non-empty list. Otherwise the existing bulk-Vec path runs unchanged. Phase B Round 2 finding: claude_code, codex, and opencode all implement `discover_source_files` (`claude_code.rs:589`, `codex.rs:721`, `opencode.rs:1013`); capability-detection alone (Round 1's fix) would route them into the pi-shaped scratch-root contract. Pi's `sessions/<workspace>/<file>` layout and `external_id` derivation are pi-specific; gating to `kind.slug() == "pi_agent"` is required.

`scan_batch_limits` is a new struct with two env-tunable fields at the scan level (Phase B Round 1 dropped the messages-at-scan limit because `DiscoveredSourceFile.size_bytes` exists per `scan.rs:205-217` but message count is post-parse only):

- `CASS_WATCH_SCAN_BATCH_FILES` (default 50)
- `CASS_WATCH_SCAN_BATCH_BYTES` (default 64 MB cumulative `size_bytes`)

The persist-buffer flush DOES still gate on message count, but at the post-scan boundary where `NormalizedConversation.messages.len()` is real data. The buffer's `full_by_count_or_bytes_or_messages()` check uses `CASS_WATCH_BUFFER_MAX_MESSAGES` (default 8,192) on actual emitted message counts — a different knob from the scan-batch limit, named distinctly to reflect its different semantics.

The existing persist knob `CASS_WATCH_INGEST_CHUNK_SIZE` (default 32 per `src/indexer/mod.rs:89`, max 512 per `:90`) remains the inner-loop chunk for `ingest_watch_batch_with_oom_split` — nested inside the buffer flush.

### 2. New helper: `build_scratch_root`

New module `src/indexer/scratch_root.rs` (or inline submodule in `mod.rs`):

```rust
pub struct ScratchRootGuard {
    path: PathBuf,
}

impl ScratchRootGuard {
    pub fn path(&self) -> &Path { &self.path }
}

impl Drop for ScratchRootGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);  // best-effort cleanup
    }
}

pub fn build_scratch_root(
    batch: &[DiscoveredSourceFile],
    workdir: &Path,
    original_sessions_root: &Path,   // e.g. ~/.pi/agent/sessions
) -> Result<ScratchRootGuard> {
    let id = uuid::Uuid::new_v4();
    let root = workdir.join(format!("{}", id));
    let sessions = root.join("sessions");
    std::fs::create_dir_all(&sessions)?;
    for src in batch {
        // src.source_path is e.g. <original_sessions_root>/<workspace>/<file>.jsonl
        let rel = src.source_path.strip_prefix(original_sessions_root)?;
        let dest = sessions.join(rel);
        std::fs::create_dir_all(dest.parent().unwrap())?;
        match std::fs::hard_link(&src.source_path, &dest) {
            Ok(()) => {}
            Err(e) if e.raw_os_error() == Some(libc::EXDEV) => {
                std::fs::copy(&src.source_path, &dest)?;  // APFS clonefile on macOS
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(ScratchRootGuard { path: root })
}
```

`workdir` defaults to `<data_dir>/scratch/watch-once-streaming/`. `original_sessions_root` is the canonical pi sessions root (derived from the connector's `ScanContext` — for pi that's `<home>/sessions` where `<home>` is the cass-side scan root, typically `~/.pi/agent`). RAII cleanup via `Drop` ensures scratch directories are removed even on panic.

### 3. New helper: `remap_source_path`

The remap operates on the `sessions/<workspace>/<file>` suffix common to both scratch and original paths. The scratch root is `<scratch>` (no trailing `/sessions`); FAD computes `external_id` relative to `<scratch>/sessions` because its `sessions_dir(home) = home.join("sessions") if exists else home` rule (`pi_agent.rs:74-105`). The emitted `source_path` from FAD is `<scratch>/sessions/<workspace>/<file>`. The original path is `~/.pi/agent/sessions/<workspace>/<file>` where `~/.pi/agent` plays the same `home` role.

```rust
fn remap_source_path(
    conv: &mut NormalizedConversation,
    scratch_sessions_root: &Path,    // <scratch>/sessions
    original_sessions_root: &Path,   // ~/.pi/agent/sessions
) {
    if let Ok(rel) = conv.source_path.strip_prefix(scratch_sessions_root) {
        conv.source_path = original_sessions_root.join(rel);
    }
}
```

This preserves the canonical `~/.pi/agent/sessions/<workspace>/<file>` shape that raw-mirror and DB `source_path` consumers expect (Phase B Round 1 caught a prior version that accidentally produced `~/.pi/agent/sessions/sessions/<workspace>/<file>` by stripping only `<scratch>` instead of `<scratch>/sessions`). Per-message `extra` fields are not touched — they're opaque JSON internal to `NormalizedMessage.extra`.

Phase B Round 1 sign-off: "You proved duplicate avoidance, but not that the indexed rows point back to the real files." The fixed remap above is the structural answer. `tasks.md ## Group D — Regression tests` adds an explicit assertion that DB `source_path` matches `~/.pi/agent/sessions/<workspace>/<file>` after the streaming branch runs (T12, T21 no-data-loss check).

### 4. Progress / stats accounting refactor

Three call-sites change semantics from "compute from full Vec post-scan" to "accumulate incrementally":

- `:16275` `p.total.fetch_add(convs.len(), ...)` → set once up-front from `discovered.len()`.
- `:16389` `stats.connectors.push(ConnectorStats { conversations, messages, scan_ms })` — `conversations = ingest_success_conversations + quarantined_oom`, `messages = inserted_messages` (post-persist; matches existing watch-mode semantics; quarantined contribute 0). `scan_ms` becomes wall-clock from `discover_source_files` start to last-flush end.
- `:16415` `total_indexed = processed_conversations.sum()` — accumulate from per-batch `WatchIngestBatchOutcome.processed_conversations` ONLY (already includes single-conv quarantine cases per `:15656`; don't double-add `quarantined_conversations`).

### 5. Run receipt + reconciliation + skipped-file recording

End-of-run JSON record (extension of existing watch-once log output):

```json
{
  "discovered_files": N,
  "emitted_source_files": E,
  "parser_skip_records": N - E,
  "emitted_conversations": C,
  "ingest_success_conversations": S,
  "quarantined_oom": Q,
  "parse_unaccounted_files": 0,            // discovered_files - emitted_source_files - parser_skip_records
  "ingest_unaccounted_conversations": 0    // emitted_conversations - ingest_success_conversations - quarantined_oom
}
```

`parse_unaccounted_files` and `ingest_unaccounted_conversations` MUST be 0. If non-zero, list paths/conversations in the receipt with reasons. Acceptance #1 maps to `ingest_success_conversations >= 1970` for the pi connector.

**Skipped-file recording (Phase B Round 4 finding)**: spec 015 Acceptance #1 requires "any skipped files MUST be recorded in `<data_dir>/quarantine/watch_ingest_poison.jsonl`." The reconciliation receipt's `parser_skip_records` count is necessary but not sufficient — each skipped file's path must also appear in the quarantine file with a reason. Concretely: cass computes `parser_skipped_paths = discovered_source_paths - emitted_source_paths` (set difference at end of run) and for each path in that set, appends a line to `<data_dir>/quarantine/watch_ingest_poison.jsonl` shaped like `{"source_path": "...", "reason": "parser_skip", "agent": "pi_agent", "timestamp_ms": ...}`. The existing OOM-quarantine path at `:15717` continues to record `reason: "ingest_oom"` for single-conv OOMs. Both reason values share the same file. The reconciliation's `parse_unaccounted_files == 0` invariant is structurally guaranteed by the set-difference computation, but the spec contract requires the explicit per-path record for downstream tooling.

### 6. Forward-capture watcher path unchanged

The streaming branch is gated on the three-part predicate above (`explicit_watch_once && kind.slug() == "pi_agent" && !discovered.is_empty()`). `save_watch_state_watermark` at `:16372` is already conditional on `!explicit_watch_once`, so the gating composes correctly: streaming branch never saves the watermark; bulk branch keeps sort + watermark contract intact for the continuous watcher. Acceptance #4 satisfied by code-path isolation.

## Caller-preservation contract

No callers downstream of `do_index_run` see a different `NormalizedConversation` shape. The `source_path` remap restores the canonical path before any persist or raw-mirror touch. `external_id` is bit-for-bit identical. Per-conversation transformations (`inject_provenance`, `apply_workspace_rewrite`, `compact_large_connector_extras`, `attach_raw_mirror_capture`) run unchanged.

## Storage-lock holding pattern

Acquire the storage lock per-batch-flush (not for the whole streaming duration), matching today's per-chunk acquisition at `:16307-16377`. Forward-capture watcher can interleave between batches.

## Out of scope

- Spec 014's 8 GB absolute peak-RSS threshold. Tracked in `coding_agent_session_search-d907f` (fsqlite_ext_fts5 disk-persistence).
- `franken_agent_detection` `PiAgentConnector` streaming override (Shape B / Route 2). Streaming branch is forward-compatible with a future FAD PR but does not depend on one.
- Contentless-table side-finding (separate bead).
- Spec 013 / PR #233 chunk-size logic. Preserved by nested composition.
- Forward-capture watcher behaviour. Streaming gated to `explicit_watch_once && kind.slug() == "pi_agent" && !discovered.is_empty()`; the continuous watcher is on the bulk path either way.

## Risks

- **Scratch root cleanup on panic**: RAII `Drop` via `ScratchRootGuard` calling `std::fs::remove_dir_all` on the per-batch directory. Phase B Round 2 sign-off note: cass's "no-file-deletion" rule applies to user-owned content (DB, source jsonl, snapshots, etc.), not to generated scratch artifacts. The scratch dir is created by cass under its own `<data_dir>/scratch/watch-once-streaming/` namespace, contains only hardlinks/clones of files that still exist at their canonical paths, and is removed at end-of-batch via the same pattern `std::env::tempdir()` consumers use. The carve-out is the canonical tempdir pattern, not a policy violation. Verified by injecting a panic in T11 fixture test (T15).
- **EXDEV cross-device hardlink**: `<data_dir>/scratch/watch-once-streaming/` defaults to same volume as `~/.pi`. `fs::copy` fallback handles the edge case; APFS `clonefile` makes copy near-free.
- **Large single file**: 72 MB max pi jsonl (spec evidence). `CASS_WATCH_SCAN_BATCH_BYTES = 64 MB` means a 72 MB file flushes as its own batch. Safe degenerate.
- **Progress UI**: `progress.total` set up-front from `discovered_files` could over-count if some files are parser-skipped. The receipt catches this in `parser_skip_records`; UI just shows stalled tail.
- **Tantivy commit cadence**: streaming commits per batch (~40 batches for pi). Commit overhead small; verified in T14.

## Test plan

- **Unit/fixture**: extend `tests/connector_pi_agent.rs` with a synthesised 100-conv pi corpus (synthetic, no real data). Run `cass index --watch-once <tmpdir>` with `explicit_watch_once=true`. Assert: 100 conversations land in DB, peak working set delta under 256 MB, receipt counters balance.
- **Regression — chunk-size**: assert `CASS_WATCH_INGEST_CHUNK_SIZE=8` still causes 8-conv persist chunks under the streaming branch. Same shape as spec 013's regression test.
- **Regression — forward-capture**: `explicit_watch_once=false` runs the bulk-Vec path unchanged. Existing watcher tests pass.
- **Manual full-corpus**: stop launchd watcher, APFS-snapshot DB, run `~/.local/bin/cass index --watch-once ~/.pi/agent/sessions --json --no-progress-events`, sample RSS every 60 s, verify run completes with `ingest_success_conversations >= 1970`, peak working-set delta sub-1-GB, message-coverage harness (FAD pinned parser on full root → map by external_id → compare DB messages count) clean.

## Plan Sanity Evidence

Objective: replace cass's watch-once eager-scan path with per-batch hardlink/copy scratch-root scanning so `cass index --watch-once ~/.pi/agent/sessions` completes pi backfill (≥1,970 conversations indexed) without piling 2,073 `NormalizedConversations` into memory at once. Forward-capture watcher and non-pi connectors continue working unchanged.

Riskiest assumption: FAD's `external_id` derivation is bit-for-bit identical when scanning `<scratch>/sessions/<workspace>/<file>.jsonl` vs `~/.pi/agent/sessions/<workspace>/<file>.jsonl`. If false (different `external_id` strings), the per-batch ingest creates duplicate rows alongside existing pi conversations (the watcher-captured 36 today), violating Acceptance #1's "no data loss" implication.

Smallest probe: read FAD pi connector's `sessions_dir` resolution and `external_id` derivation end-to-end at pinned rev `5115da8e515ee8a76cf676e78bc2d351e14abc82`.

Observed result: ran `awk 'NR>=74 && NR<=105' ~/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs` — confirmed `sessions_dir(home) = home.join("sessions") if home.join("sessions").exists() else home`. Ran `sed -n '320,350p'` on the same file — confirmed `external_id = source_path.strip_prefix(&sessions_dir).to_str()`. Under `<scratch>/sessions/<workspace>/<file>`, `sessions_dir = <scratch>/sessions`; strip yields `<workspace>/<file>`. Under `~/.pi/agent/sessions/<workspace>/<file>`, `sessions_dir = ~/.pi/agent/sessions`; strip yields `<workspace>/<file>`. Identical. Codex Phase A round 4 independently verified at `pi_agent.rs:74, :322, :340`.

Decision impact: if the probe had shown `external_id` was path-absolute or computed without `strip_prefix`, `plan.md ## Architecture` section "1. `src/indexer/mod.rs` watch-once branch" would drop Route 5 in favour of Route 4 (cass-owned parser), and `tasks.md ## Group A: Helpers` would expand from "scratch-root builder + source_path remap" to "cass-side pi connector + FAD-parity harness." The verified result lets Group A ship a small focused diff.

---


# PLAN-SNAPSHOT-END
