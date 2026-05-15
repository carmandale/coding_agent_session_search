---
title: "watch-once scan must stream like the watcher, not bulk-materialize the corpus"
date: 2026-05-15
bead: coding_agent_session_search-81z91
---

<!-- issue:complete:v1 | harness: unknown | date: 2026-05-15T21:14:56Z -->
<!-- Codex Review: APPROVED after 4 rounds | model: gpt-5.3-codex | date: 2026-05-15 | trust_level: full | round_records: .codex-round-be974918/, .codex-round-a7d0fb93/, .codex-round-0f612fb8/, .codex-round-b8cbdaf0/ | Status: UNCHANGED -->

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
