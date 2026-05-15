---
title: "Plan: watch-once scan must stream like the watcher, not bulk-materialize the corpus"
date: 2026-05-15
bead: coding_agent_session_search-81z91
---

<!-- plan:complete:v1 | harness: unknown | date: 2026-05-15T22:37:12Z -->
<!-- Codex Review: APPROVED after 4 rounds | model: gpt-5.3-codex | date: 2026-05-15 | trust_level: full | round_records: .codex-round-be974918/, .codex-round-a7d0fb93/, .codex-round-0f612fb8/, .codex-round-b8cbdaf0/ | Status: REVISED -->

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
// Short-circuit on kind/mode BEFORE discovery (Codex-review Round 1 finding):
// without this short-circuit, claude/codex/opencode would pay for a full
// discover_source_files call even though they take the bulk path. The check
// for pi-only routing must come FIRST.
if !explicit_watch_once || kind.slug() != "pi_agent" {
    // FALL THROUGH to the existing bulk Vec path (unchanged)
    return existing_bulk_watch_once(...);
}

let discovered = conn.discover_source_files(&ctx).unwrap_or_default();
if discovered.is_empty() {
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
let mut scratch_skips: Vec<ScratchBuildSkip> = Vec::new();   // Codex-review Round 3: accumulate skips

for file_batch in chunk_by_files_and_bytes(&discovered, scan_batch_limits) {
    let (scratch, batch_skips) = build_scratch_root(&file_batch, &workdir, &original_sessions_root)?;
    scratch_skips.extend(batch_skips);
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

    // MANDATORY flush at the end of every scan batch (Codex-review Round 1
    // finding): without this, conversations could carry across batches in the
    // buffer, breaking the "drop the working set after each batch" guarantee
    // that spec 015 Requirement #1 makes load-bearing.
    if !buffer.is_empty() {
        flush_buffer(&mut buffer, &mut counters, storage, t_index, ...)?;
    }
    // scratch dropped here — RAII cleanup
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

pub struct ScratchBuildSkip {
    pub source_path: PathBuf,
    pub error_message: String,
}

pub fn build_scratch_root(
    batch: &[DiscoveredSourceFile],
    workdir: &Path,
    original_sessions_root: &Path,
) -> Result<(ScratchRootGuard, Vec<ScratchBuildSkip>)> {
    let id = uuid::Uuid::new_v4();
    let root = workdir.join(format!("{}", id));
    let sessions = root.join("sessions");
    std::fs::create_dir_all(&sessions)?;  // systemic — propagates
    let mut skips: Vec<ScratchBuildSkip> = Vec::new();
    for src in batch {
        let rel = match src.source_path.strip_prefix(original_sessions_root) {
            Ok(r) => r,
            Err(e) => { skips.push(ScratchBuildSkip { source_path: src.source_path.clone(), error_message: format!("path not under sessions root: {e}") }); continue; }
        };
        let dest = sessions.join(rel);
        if let Some(parent) = dest.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                skips.push(ScratchBuildSkip { source_path: src.source_path.clone(), error_message: format!("create_dir_all: {e}") });
                continue;
            }
        }
        match std::fs::hard_link(&src.source_path, &dest) {
            Ok(()) => {}
            Err(e) if e.raw_os_error() == Some(libc::EXDEV) => {
                if let Err(copy_err) = std::fs::copy(&src.source_path, &dest) {
                    skips.push(ScratchBuildSkip { source_path: src.source_path.clone(), error_message: format!("copy fallback: {copy_err}") });
                }
            }
            Err(e) => {
                skips.push(ScratchBuildSkip { source_path: src.source_path.clone(), error_message: format!("hard_link: {e}") });
            }
        }
    }
    Ok((ScratchRootGuard { path: root }, skips))
}
```

Helper returns the guard plus the skip list; the **caller writes the quarantine records** (single source of truth — Codex-review Round 2 finding: prior wording had the helper writing quarantine directly AND the caller doing it, which was contradictory).

`workdir` defaults to `<data_dir>/scratch/watch-once-streaming/`. `original_sessions_root` is derived using FAD-equivalent logic (Codex-review Round 1 finding): given the user's scan root `R` (the `--watch-once <R>` argument, e.g. `~/.pi/agent` OR `~/.pi/agent/sessions`), if `R/sessions` exists, `original_sessions_root = R/sessions`; else `original_sessions_root = R`. This matches FAD's `sessions_dir(home)` resolution at `pi_agent.rs:74-105` exactly. The cass-side helper that derives this MUST be exercised in tests for both `~/.pi/agent` and `~/.pi/agent/sessions` user roots (T4 / new T4a). RAII cleanup via `Drop` ensures scratch directories are removed even on panic.

**Scratch-build failure handling**: `build_scratch_root` MUST NOT propagate per-file hardlink/copy errors as fatal. Per-file failures (missing source, permission, non-EXDEV filesystem error) become `ScratchBuildSkip` entries in the returned `Vec<ScratchBuildSkip>` and the helper continues with the rest of the batch. Only systemic errors (workdir creation failure, scratch root permission issues, `original_sessions_root` doesn't exist) propagate via `Result::Err`. The caller is the single source of truth for writing quarantine records (Codex-review Round 2 finding — prior wording had both the helper and the caller writing, which was contradictory). Caller writes a quarantine line per `ScratchBuildSkip` at run completion with `{"source_path": ..., "reason": "scratch_build_failure", "io_error": <error_message>, "agent": "pi_agent", "timestamp_ms": ...}`.

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

End-of-run JSON record, placed in the existing `cass index` JSON output at a new TOP-LEVEL key `watch_once_receipt` (Codex-review Round 1 finding — receipt location must be a stable JSON contract; top-level keeps it discoverable and survives existing `indexing_stats` schema goldens unchanged). T9 includes updating the response schema at `src/lib.rs:70961+` and any associated goldens. Shape:

```json
{
  "watch_once_receipt": {
    "discovered_files": N,
    "emitted_source_files": E,
    "scratch_build_skips": K,
    "parser_skip_records": N - E - K,
    "emitted_conversations": C,
    "ingest_success_conversations": S,
    "quarantined_oom": Q,
    "parse_unaccounted_files": 0,            // discovered_files - emitted_source_files - parser_skip_records - scratch_build_skips
    "ingest_unaccounted_conversations": 0    // emitted_conversations - ingest_success_conversations - quarantined_oom
  }
}
```

Files in three disjoint terminal buckets (Codex-review Round 2 finding): `emitted_source_files` (made it through scan), `scratch_build_skips` (failed to scratch-mirror; never saw the parser), `parser_skip_records` (scratch-mirrored fine but parser yielded zero conversations). These three sum exactly to `discovered_files`; any deviation surfaces as `parse_unaccounted_files != 0`, which the run-completion assertion catches.

`parse_unaccounted_files` and `ingest_unaccounted_conversations` MUST be 0. If non-zero, list paths/conversations in the receipt with reasons. Acceptance #1 maps to `ingest_success_conversations >= 1970` for the pi connector.

**Skipped-file recording**: spec 015 Acceptance #1 requires "any skipped files MUST be recorded in `<data_dir>/quarantine/watch_ingest_poison.jsonl`." The reconciliation receipt's counts are necessary but not sufficient — each skipped file's path must also appear in the quarantine file with a reason. Concretely: cass computes the three disjoint terminal sets:

- `scratch_skipped_paths`: collected during run from each batch's `ScratchBuildSkip` list.
- `parser_skipped_paths = discovered_source_paths - emitted_source_paths - scratch_skipped_paths` (Codex-review Round 3 finding — subtracting `scratch_skipped_paths` keeps the three buckets disjoint per the receipt model above).
- `emitted_source_paths`: collected during run from the streaming callback.

For each path in `scratch_skipped_paths`, append `{"source_path":"...", "reason":"scratch_build_failure", "io_error":<error_message>, "agent":"pi_agent", "timestamp_ms":...}`. For each path in `parser_skipped_paths`, append `{"source_path":"...", "reason":"parser_skip", "agent":"pi_agent", "timestamp_ms":...}`. The existing OOM-quarantine path at `:15717` continues to record `reason: "ingest_oom"` for single-conv OOMs. All three reason values share the same `<data_dir>/quarantine/watch_ingest_poison.jsonl` file.

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
