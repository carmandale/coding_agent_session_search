---
title: "Finalize deferred lexical refresh without full rebuild"
date: 2026-05-17
bead: coding_agent_session_search-3135s
---

<!-- issue:complete:v1 | harness: unknown | date: 2026-05-17T23:59:40Z -->

## Source (verbatim)

> "I agree with the next 4 steps you are proposing. $goalbuddy these and get them done" — user, 2026-05-17

## Problem

After the spec 017 watch-once fix, targeted ingestion can persist a recent session into SQLite and defer inline lexical maintenance instead of losing the conversation. On Dale's large live corpus, the follow-up lexical rebuild can reach a completed DB-matching checkpoint and make recent sessions searchable, but `cass status --json` can still report the index as stale and `lexical-refresh-needed.json` can remain pending.

That stale metadata is not harmless: running the documented `cass index` repair command can restart the huge lexical rebuild from scratch and OOM again instead of recognizing that the completed checkpoint already matches the current DB and finalizing the stale marker/metadata.

Purpose Contract:

- Outcome: Dale can use cass for recent and historical sessions without custom local watcher/index band-aids, and a completed large-corpus lexical refresh is recognized as complete.
- Done means: when a completed DB-matching lexical checkpoint exists, cass can finalize the stale/pending refresh state without rebuilding the whole corpus again, recent sessions remain searchable, and robot/status surfaces are truthful.
- Not done: `cass index` starts another full rebuild solely because `last_indexed_at` or `lexical-refresh-needed.json` lagged behind a completed checkpoint; status stays stale after finalization; or the fix depends on deleting index directories/manual DB edits/local watcher scripts.

Verified starting evidence:

- `cass search "fresh-context independent north-star checker" --json --fields minimal --limit 5` returned a May 17 Codex session hit from the live index.
- `cass status --json` reported `healthy:false`, index `status:"stale"`, `last_indexed_at:"1970-01-01T00:00:00+00:00"`, and a completed DB-matching checkpoint.
- `lexical-refresh-needed.json` remained pending with `reason:"watch_lexical_updates_deferred"` and `conversations:1`.
- A prior low-memory lexical refresh processed roughly the full live corpus before OOM, after which a second `cass index` began rebuilding again instead of doing an idempotent finalization pass.
- Follow-up live proof exposed a second edge in the same user-facing recovery path: after fixing finalization, the documented `cass index --json --no-progress-events --data-dir ...` refresh still failed when one irreducible streaming conversation OOMed even after inline lexical updates were deferred. That should quarantine the poison conversation and continue the documented refresh, not make cass unusable.

## Requirements

- Detect the case where a pending deferred lexical refresh marker coexists with a completed DB-matching lexical rebuild checkpoint.
- Finalize metadata/marker state without rebuilding the full lexical index when the existing lexical artifacts and completed checkpoint are sufficient.
- Preserve the "SQLite is source of truth, lexical is derived" contract from the cass documentation and spec 017.
- Keep the fix upstream-clean. Do not rely on local launchd scripts, manual DB edits, index deletion, or environment-variable operator rituals.
- Add focused regression coverage for the decision/finalization path so the stale-marker/completed-checkpoint case cannot regress silently.
- Keep robot/status surfaces truthful if the checkpoint is not actually complete or does not match the DB.
- When a single streaming conversation still OOMs after lexical deferral, record a poison quarantine entry and continue the documented refresh instead of failing the entire run.

## Constraint

- Do not delete DB, index, quarantine, or raw-mirror artifacts.
- Do not add new `rusqlite` code.
- Do not broaden this into solving upstream #239's full-rebuild `daily_stats` OOM unless source evidence proves it is the same root cause.
- Do not conflate this with upstream #240's original watch-once false-success bug; this is the follow-up finalization/rebuild-loop bug.
- Preserve JSON contract discipline; update goldens only if robot schemas intentionally change.

## Acceptance Criteria

- A regression test proves the stale deferred marker plus completed DB-matching checkpoint path resolves/finalizes without selecting a full rebuild.
- A regression test or focused unit path proves the finalization guard refuses to resolve the marker when checkpoint state is incomplete or DB mismatched.
- `cass index --json --no-progress-events` on the live state completes quickly or otherwise takes the no-rebuild finalization path when the completed checkpoint is valid.
- `cass status --json` no longer reports stale metadata solely because the deferred marker/`last_indexed_at` lagged behind the completed checkpoint.
- A recent May 17 search probe still returns the expected session after the fix.
- A single poison streaming conversation is reported as quarantined without preventing index freshness or search readiness for the rest of the corpus.
- Relevant cargo test/check/fmt/clippy verification passes for touched code.

## Out of Scope

- Deleting or compacting old index/quarantine data.
- Installing semantic models or changing semantic search behavior.
- Rewriting the watcher or replacing the documented watcher with custom infrastructure.
- Solving all possible large-corpus OOMs.
- Making every quarantined poison session searchable in this patch; those sessions need a separate split/repair strategy.
- Posting speculative upstream comments before the local evidence is clean.

## Selected Shape

Add an idempotent lexical refresh finalization path before expensive rebuild work. If the deferred lexical refresh marker is pending and the latest lexical checkpoint is completed, schema/DB/page-size compatible, and tied to the current live DB state, cass should update final index metadata and mark the deferred refresh resolved instead of rebuilding the corpus again. If those conditions are not met, cass should keep the existing repair recommendation and rebuild behavior.
