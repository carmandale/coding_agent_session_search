---
title: "Fix watch-once lexical OOM quarantine"
date: 2026-05-17
bead: coding_agent_session_search-2rtk7
---

<!-- issue:complete:v1 | harness: unknown | date: 2026-05-17T14:55:30Z -->

## Source (verbatim)

> "My requirement is very simple. I want to be able to use cass. I want to be able to use it on historical sessions. If we are running CAS on live infrastructure and that's not recommended, then we shouldn't do it. What is the documented recommended way to do this?" — user, 2026-05-17

> "So this is what you really need to diagnose. See if you don't get more clear on this. Is there something about those conversations? Is it possible to break those into smaller chunks?" — user, 2026-05-17

> "do you think you can do the patch and get it all working?" — user, 2026-05-17

> "$goalbuddy create an $issue and get this work done" — user, 2026-05-17

## Problem

`cass index --watch-once <path>` can return `success:true` while inserting zero messages and writing a `watch-ingest-out-of-memory` poison record for the requested conversation. This breaks the operator-facing contract: cass appears healthy and current while a parseable session is missing from SQLite and therefore unavailable for future search.

Purpose Contract:

- Outcome: Dale can use documented cass indexing/watch-once behavior to get recent and historical agent sessions into the index without custom local band-aids.
- Done means: a verified small-session reproducer that previously wrote `watch_ingest_poison.jsonl` instead lands in SQLite or reports a truthful partial/failure state with an actionable repair path.
- Not done: robot output says `success:true` with `messages:0` while the requested path is absent from SQLite; status looks fresh while poison conversations are hidden; or the fix relies on local watcher scripts, manual DB edits, or `CASS_DEFER_LEXICAL_UPDATES` as an operator workaround.

Verified evidence, also posted upstream as https://github.com/Dicklesworthstone/coding_agent_session_search/issues/240:

- A small Codex session file (`247315` bytes, poison record `message_count: 7`) indexes into an empty cass data dir with `messages:6`.
- The same file against a clone of the live `7.1G` DB returns `success:true`, reports `messages:0`, writes a poison OOM record, and leaves the conversation absent from SQLite.
- `CASS_DEFER_ANALYTICS_UPDATES=1` does not help.
- `CASS_DEFER_LEXICAL_UPDATES=1` inserts the conversation and six messages into SQLite.

## Requirements

- Fix targeted `watch-once` so a requested, parseable conversation is not silently lost behind a poison record when lexical maintenance fails.
- Preserve SQLite as the source of truth: if message persistence succeeds and lexical maintenance fails, the conversation should remain durably stored in SQLite.
- Make robot output truthful. A targeted `watch-once` must expose quarantined or lexically-deferred work instead of returning a plain success shape that implies the requested session landed cleanly.
- Mark lexical assets stale/repair-needed, or otherwise surface an actionable repair recommendation, when inline lexical maintenance is skipped or fails after DB persistence.
- Add focused regression coverage for the small-session single-conversation failure shape, including the current misleading success path.
- Keep the implementation upstream-clean and avoid local watcher scripts, DB mutation recipes, or other operator band-aids.

## Constraint

- Do not delete files or use destructive git/filesystem commands.
- Do not add new `rusqlite` code; use the existing frankensqlite-backed storage path.
- Do not change unrelated full-rebuild / `daily_stats` behavior from #239 unless required by the fix.
- Do not restart the documented watcher against live data until the targeted failure is fixed and verified in a temp/live-clone path.
- Preserve robot JSON schema discipline; update goldens if a contract field is added or changed.

## Acceptance Criteria

- A regression test proves that a lexical-update OOM during targeted `watch-once` cannot produce plain `success:true` with `messages:0` while the requested path is absent.
- A regression test proves DB-first behavior: when DB persistence succeeds but inline lexical maintenance fails, the conversation and messages remain queryable from SQLite and the lexical failure is surfaced truthfully.
- The verified small Codex reproducer succeeds against a live-sized DB clone without setting `CASS_DEFER_LEXICAL_UPDATES`, or else exits/reports a truthful partial failure with the session preserved in SQLite.
- `cass status --json`, `cass index --watch-once ... --json`, or the relevant robot surface exposes any poison/deferred lexical state needed by operators and agents to avoid false “fresh/healthy” conclusions.
- Relevant Rust checks pass for the touched code and focused tests.

## Out of Scope

- Solving the full-rebuild `daily_stats` OOM tracked by upstream #239.
- Installing semantic models or changing semantic indexing behavior.
- Replacing the documented watcher with custom launchd/scripts.
- Deleting quarantine files, DB files, index directories, or raw-mirror data.
- Broad connector rewrites unrelated to the targeted watch-once lexical failure.

## Selected Shape

Direct root-cause fix in the watch ingest / lexical maintenance path with focused regression coverage. The intended shape is DB-first watch ingest: persist the parseable conversation to SQLite, treat inline lexical maintenance as a derived-asset update that may fail open, mark/report lexical repair needs truthfully, and avoid quarantining an entire single conversation solely because the lexical update path OOMed after DB persistence was viable.
