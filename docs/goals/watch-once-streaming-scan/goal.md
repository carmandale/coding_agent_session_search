# Ship spec 015 — watch-once scan streaming for pi-agent historical backfill

## Objective

Drive `specs/015-watch-once-streaming-scan/` through the canonical Dale Codex Workflow until `cass index --watch-once ~/.pi/agent/sessions` completes the user's full pi-agent historical backfill (≥ 1,970 conversations indexed, ≥ 95 % of the 2,073 jsonl corpus) without manual kill, and ≥ 95 % of the user's pi sessions are searchable in cass alongside claude_code / codex / opencode.

Spec, problem framing, requirements, constraints, acceptance criteria, and Selected Shape live in `specs/015-watch-once-streaming-scan/spec.md` and are the source of truth for *what* and *why*. This board owns *how* the workflow moves through `$codex-plan → $codex-review → $codex-implement → $code-verify → $finalize`.

## Original request (verbatim anchors)

> "historical sessions are more than desired. they are the whole point. cass is only about history. without historical sessions you have nothing. but it doesn't have to happen instantly. I don't care if it takes 2 weeks. what is the right way to do it? if 1 works....if 33 work, than 2700 will work" — user, 2026-05-15

> "the goal is to be in sync with upstream and running properly, capturing all sessions and cass working." — user, 2026-05-13

> "claude code and codex are top priorities, with pi-agent next and then opencode." — user, 2026-05-14

## Intake summary

- **Input shape**: `existing_plan` — spec 015 has spec.md with full Selected Shape (Shape A `discover_source_files` + per-batch `scan()`, Shape B `scan_with_callback` with per-callback batching) and an explicit decision tree for which ships.
- **Authority**: `requested` — user invoked `/issue` to create spec 015 and `/goalbuddy` to prep this board.
- **Proof type**: `artifact` — `cass index --watch-once ~/.pi/agent/sessions --json --no-progress-events` returns `success: true` with `pi_agent` row count in DB ≥ 1,970.
- **Completion proof**: the watch-once run completes without manual kill; pi conversation count in DB reaches ≥ 1,970; forward-capture watcher behaviour unchanged; no spec-013 chunk-size regression; pi message-coverage spot-check matches the FAD-harness expected count.
- **Likely misfire**: re-litigating spec 014's 8 GB peak-RSS threshold inside this spec. That threshold is **explicitly dropped** here (see spec.md Constraint section); the architectural fix for the in-memory FTS5 floor is tracked separately and is not gating spec 015.
- **Blind spots considered**:
  - `ScanContext` may not natively support file-scoped sub-contexts for Shape A — the plan must verify and either use an existing pattern or extend `ScanContext` cass-side.
  - `Connector::supports_streaming_scan()` returns `false` for the pi connector today, so Shape B's optimal path is only reached after a separate upstream FAD PR; cass code must auto-fall-back to Shape A.
  - The `compact_large_connector_extras` / `attach_raw_mirror_capture` transformations at `src/indexer/mod.rs:16263-16268` operate per-conversation, so the chunked scan must call them on each chunk before persist (not skip them).
  - Forward-capture watcher path must remain untouched — the chunking applies only to the watch-once branch.
- **Existing plan facts** (preserved verbatim from spec 015 + carried over from spec 014):
  - `$issue` is satisfied (`spec.md` exists with `bead: coding_agent_session_search-81z91` and `issue:complete:v1` sentinel).
  - `$codex-shape` is satisfied via explicit `shape-skip:` in `specs/015-watch-once-streaming-scan/log.md` — "chunk-the-scan extension of PR #233 chunk-the-persist pattern; clear precedent and well-defined surface".
  - Selected Shape: Shape A (cass-only) as default, with Shape B fall-forward when `supports_streaming_scan()` returns `true`. Decision tree in spec.md governs the final pick.
  - Code surface: `src/indexer/mod.rs` watch-once code path around `:16248`. PR #233 chunked persist is preserved; this work adds chunking one level up at the scan boundary.
  - Acceptance #1 from spec 014 (`≥ 1,970 pi conversations`) carries over verbatim. Spec 014's acceptance #2 (`peak RSS < 8 GB`) is **dropped** for this spec; it requires an upstream architectural change in `frankensqlite_ext_fts5` and is tracked separately.

## Goal kind

`existing_plan` — first active task is `$codex-plan` (T001), per the GoalBuddy Workflow Resume Map row "spec.md exists and shaping exists or is explicitly skipped, but plan.md, tasks.md, or planning-transcript.md is missing".

## Current tranche

Move spec 015 from "shaped" to "shipped":

1. **`$codex-plan`** produces `plan.md`, `tasks.md`, `planning-transcript.md`, and the `plan:complete:v1` provenance sentinel.
2. **`$codex-review`** runs as the mandatory gate; iterate until Codex returns `VERDICT: APPROVED`.
3. **`$codex-implement`** does Phase 1 (Shape selection + ScanContext audit) and Phase 2 (the focused diff) inside its own task list and write boundaries.
4. **`$code-verify`** independently checks all five spec-015 acceptance criteria, including the full-corpus pi backfill run.
5. **`$finalize`** closes bead `coding_agent_session_search-81z91`, writes the handoff under `thoughts/shared/handoffs/`, commits remaining spec updates, and pushes `dac/main`.
6. **T999 Judge** audits the full tranche before the goal closes.

Continuous-execution default applies: after each provenance sentinel lands, PM activates the next workflow command without stopping for unsolicited approval. PR #90 (the upstream frankensqlite savepoint-clone fix) merges on its own merits and is not gated by this goal.

## Non-negotiable constraints

- **Never skip `$codex-review`.** The same-session Phase B approval inside `$codex-plan` does not substitute for the fresh independent review gate.
- **Cass-side change only.** Implementation must stay within cass per spec 015's Constraint section. Plan may *propose* an FAD streaming impl as an optimisation but acceptance must be reachable without it.
- **No destructive recovery.** Current pi_agent rows in the DB (36 as of 2026-05-15 14:44) stay. Backfill is additive.
- **Single source-of-truth binary.** Whatever lands ships via `~/.local/bin/cass.real` and the launchd watcher daemon. No per-connector build flavour.
- **Upstreamable.** Diff small enough to match PR #233's shape: focused commit(s) on `src/indexer/mod.rs`, no internal-spec terminology in the commit message, no AI attribution.
- **Do not regress PR #233.** `CASS_WATCH_INGEST_CHUNK_SIZE` behaviour from commit `e429eaab` must remain.
- **Watcher uptime.** `com.cass.index-watch` keeps running for forward capture during this work. Stop it only inside tasks that explicitly need the index-run lock (verification runs); restart before the task closes.
- **No re-litigation of spec 014's 8 GB threshold here.** That belongs in a separate spec/bead against frankensqlite.

## Stop rule

Stop only when T999 audits `full_outcome_complete: true` with: spec-015 acceptance criteria 1–5 met, bead closed, handoff written, no spec-013 regression, no forward-capture regression. Do not stop after `$codex-plan` — the plan is setup, not delivery. Do not stop after `$codex-implement` — verification and finalize are part of the tranche.

## Slice sizing

Each `$codex-*` command is one PM slice end-to-end. Do not split `$codex-plan` into "draft plan" + "stress-test plan" — that's what the command's two-phase shape already does internally. Do not split `$codex-implement` into per-phase Worker tasks — its own `tasks.md` is the inner board.

If `$codex-implement` discovers the fix needs upstream FAD changes (e.g. Shape B as the only viable route), escalate to user with the evidence — that is a scope change, not a slice-sizing decision.

## Canonical board

- `docs/goals/watch-once-streaming-scan/state.yaml` (machine truth)
- [Open GoalBuddy board](http://goalbuddy.localhost:41737/watch-once-streaming-scan/) (live UI)

## Run command

```
/goal Follow docs/goals/watch-once-streaming-scan/goal.md.
```

## PM loop

Standard PM loop: read charter, read state, validate intake, work the active workflow command, record the provenance sentinel and next-command pointer in the receipt, advance the board, audit at the final boundary only. Workflow command tasks own their own gate checks, artifacts, and write boundaries — PM does not pre-create `plan.md` / `tasks.md` by hand or duplicate `$codex-implement` with a generic Worker task.
