<!-- issue:complete:v1 | harness: unknown | date: 2026-05-13T14:17:15Z -->

2026-05-13 14:16 | — | claude-code/opus-4.7 | /issue | bead coding_agent_session_search-3vm6 — spec.md created
2026-05-13 14:16 | — | claude-code/opus-4.7 | /issue | shape-skip: well-understood bug with concrete reproduction; root cause unknown but desired end state and code surface are well-defined; user invoked /issue directly without /shape
2026-05-13 09:34 | — | claude-code/unknown | viability-gate-skip | reason=not-allowlisted | spec=specs/013-cass-rebuild-stall-asupersync/ | stage=plan
2026-05-13 10:21 | — | claude-code/opus-4.7 | /codex-plan | started — challenger: codex/gpt-5-codex
2026-05-13 10:21 | — | claude-code/opus-4.7 | /codex-plan | Phase A — Round 4 (final): 8 substantive challenges, all incorporated; no VERDICT in phase
2026-05-13 10:21 | — | claude-code/opus-4.7 | /codex-plan | Phase B — Round 3: VERDICT: APPROVED
2026-05-13 10:21 | — | claude-code/opus-4.7 | /codex-plan | North-Star Boundary B: BOOTSTRAP — snapshot boundary-B-16d0a17d-39ab06dc.md
2026-05-13 10:21 | — | claude-code/opus-4.7 | /codex-plan | completed — plan.md + tasks.md + planning-transcript.md persisted
2026-05-13 15:36 | — | codex-round-exec | supervisor | round minted: review_id=bd97be04 command=codex-review phase=default
2026-05-13 15:39 | — | codex-round-exec | supervisor | round minted: review_id=8e513a93 command=codex-review phase=default
2026-05-13 15:40 | — | codex-round-exec | supervisor | codex produced empty output
2026-05-13 15:40 | — | codex-round-exec | supervisor | round transition: open → closed-aborted review_id=bd97be04
2026-05-13 15:40 | — | codex-round-exec | supervisor | R4 stop: review_id=bd97be04 reason=empty-codex-output
2026-05-13 15:43 | — | codex-round-exec | supervisor | round transition: open → closed-success review_id=8e513a93
2026-05-13 15:43 | — | codex-round-exec | supervisor | round completed: review_id=8e513a93 state=closed-success
2026-05-13 15:47 | — | codex-round-exec | supervisor | round minted: review_id=bbf15845 command=codex-review phase=default
2026-05-13 15:47 | — | codex-round-exec | supervisor | resume round minted: source_review_id=8e513a93 review_id=bbf15845 session_id=019e21fe-73d0-78d2-82a8-7c92341d929b
2026-05-13 15:48 | — | codex-round-exec | supervisor | round transition: open → closed-success review_id=bbf15845
2026-05-13 15:48 | — | codex-round-exec | supervisor | round completed: review_id=bbf15845 state=closed-success
2026-05-13 15:50 | — | codex-round-exec | supervisor | round minted: review_id=bc55e985 command=codex-review phase=default
2026-05-13 15:50 | — | codex-round-exec | supervisor | resume round minted: source_review_id=bbf15845 review_id=bc55e985 session_id=019e21fe-73d0-78d2-82a8-7c92341d929b
2026-05-13 15:52 | — | codex-round-exec | supervisor | round transition: open → closed-success review_id=bc55e985
2026-05-13 15:52 | — | codex-round-exec | supervisor | round completed: review_id=bc55e985 state=closed-success
2026-05-13 15:54 | — | codex-round-exec | supervisor | round minted: review_id=7f82f066 command=codex-review phase=default
2026-05-13 15:54 | — | codex-round-exec | supervisor | resume round minted: source_review_id=bc55e985 review_id=7f82f066 session_id=019e21fe-73d0-78d2-82a8-7c92341d929b
2026-05-13 15:55 | — | codex-round-exec | supervisor | round transition: open → closed-success review_id=7f82f066
2026-05-13 15:55 | — | codex-round-exec | supervisor | round completed: review_id=7f82f066 state=closed-success
2026-05-13 15:56 | — | codex-round-exec | supervisor | round minted: review_id=893bb04a command=codex-review phase=default
2026-05-13 15:56 | — | codex-round-exec | supervisor | resume round minted: source_review_id=7f82f066 review_id=893bb04a session_id=019e21fe-73d0-78d2-82a8-7c92341d929b
2026-05-13 15:58 | — | codex-round-exec | supervisor | round transition: open → closed-success review_id=893bb04a
2026-05-13 15:58 | — | codex-round-exec | supervisor | round completed: review_id=893bb04a state=closed-success
2026-05-13 11:10 | — | claude-code/opus-4.7 | /codex-review | round 1 — VERDICT: REVISE
2026-05-13 11:11 | — | claude-code/opus-4.7 | /codex-review | round 2 — VERDICT: REVISE
2026-05-13 11:12 | — | claude-code/opus-4.7 | /codex-review | round 3 — VERDICT: REVISE
2026-05-13 11:13 | — | claude-code/opus-4.7 | /codex-review | round 4 — VERDICT: REVISE
2026-05-13 11:14 | — | claude-code/opus-4.7 | /codex-review | round 5 — VERDICT: APPROVED

## 2026-05-14 — deep recovery session

- Identified two new failure modes on top of the original spec-013 stall:
  - sqlite_master DDL corruption (assistant_message_count column name fused
    with type metadata). Fixed via writable_schema rewrite.
  - conversations btree rowid corruption (hundreds of rowids out of order on
    Tree 11 page 2410638). Snapshot DB was unrecoverable; archived and started
    fresh.
- Upgraded binary from v0.4.2 → v0.4.7 (latest upstream release with
  frankensqlite 0.1.3 bump and "harden watch ingest, schema repair,
  duplicate-index handling" — none of those landed a fix for the stall).
- Merged upstream/main into dac/main; source tree now matches v0.4.7 binary.
- Spec-013 stall reproduces on a fresh DB across every knob combination tried
  (streaming, batch, serial, watch, watch-once). Watchdog correctly fires
  stall_detected after 300s; thread sample shows the expected
  cond_wait/single-active-IO-thread pattern.
- Watcher restarted on v0.4.7 binary; forward capture is healthy
  (367 conversations and growing as live sessions accrue).
- Full findings in `findings-2026-05-14.md`. Groups B-H of `tasks.md` remain
  the right path to a permanent fix.

## 2026-05-14T15:25Z — root cause located AND fixed (commit e429eaa8)

The "stall" in the watch-once code path is not a deadlock in
raw_mirror/asupersync/staged-merge. It is a **single-chunk hot path**:
`reindex_paths_with_semantic_delta` forced `ingest_chunk_size =
conv_count.max(1)` when `explicit_watch_once` was true, so a connector root
with thousands of files (~/.claude/projects has 2,500+) was attempted as one
transaction. The OOM-split safety net only fires on OOM, so a merely-slow
giant batch held the writer mutex forever while every worker thread parked
as a starved producer.

Fix: use `watch_ingest_chunk_size()` unconditionally
(`src/indexer/mod.rs:16325-16334`). With the fix, claude_code backfill
progresses ~40 conversations/min in 32-conv chunks, each chunk committing
visibly. The watcher daemon, watch-once, and full rebuild are now
re-aligned on a single chunked ingest path.

What this does NOT fix: the `--full` rebuild path uses the lexical-rebuild
pipeline rather than the watch-ingest loop; D1-D6 candidates remain valid
investigation targets for that path. Out of scope for the immediate user
goal (priority connector backfill).
