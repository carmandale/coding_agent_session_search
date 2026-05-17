---
title: "Repair health-watchdog command surface regression"
date: 2026-05-17
bead: coding_agent_session_search-2gif2
---

<!-- issue:complete:v1 | harness: unknown | date: 2026-05-17T02:25:00Z -->

# Repair health-watchdog command surface regression

## Source (verbatim)

> "then set up a watcher so that all sessions are processed and part of the searchable system." - user, 2026-05-16

> "create a new $issue if necessary." - user, 2026-05-16

## Problem

Purpose Contract:

- Outcome: the `com.cass.health-watchdog` launchd job calls a supported, tested CASS command surface again, or is deliberately replaced by an installed, documented, uninstallable equivalent.
- Done means: `cass watchdog run` dispatches from the verified binary, the health-watchdog plist points at the verified command path, and a launchd smoke proves the job no longer exits with argument-parse failure.
- Not done: spec 016 proves `com.cass.index-watch` while leaving the health watchdog in a background `Could not parse arguments` loop with no tracked repair path.

Closed spec 007 / bead `coding_agent_session_search-2efx` implemented the built-in watchdog command surface, including `cass watchdog run`, `install`, and `uninstall`.
Spec 016 recovery found the runtime surface had regressed:

- Live `com.cass.health-watchdog.plist` invokes `/Users/dalecarman/.local/bin/cass watchdog run`.
- `launchctl print gui/$(id -u)/com.cass.health-watchdog` reports the service loaded but not running with last exit code `2`.
- At issue creation, both the installed binary and the spec 016 release candidate returned exit `2` with `Could not parse arguments` for `watchdog run --help`.
- Historical `cass-watchdog.log` contains repeated `Could not parse arguments` lines.

Current local status: the source/debug binary and approval-gated release
candidate now dispatch `cass watchdog run --help`, but the installed binary and
launchd service have not been changed or smoke-tested.

Spec 016 intentionally treats this as nonblocking unless it interferes with the required `com.cass.index-watch` proof. This issue tracks the follow-up repair so the regression is not lost.

## Requirements

1. Preserve spec 016 as the live session-ingestion recovery owner; this issue must not block live promotion unless health-watchdog demonstrably prevents `com.cass.index-watch` from staying loaded or indexing.
2. Determine why `cass watchdog run` disappeared or stopped dispatching despite spec 007 being closed.
3. Restore or replace the command surface in the smallest durable way that fits current CLI/capabilities/robot-docs contracts.
4. Keep launchd install/uninstall behavior explicit and documented; background automation must have an uninstall path.
5. Add regression coverage that fails on the current `Could not parse arguments` surface and passes with the repair.
6. Avoid new `rusqlite` code.

## Constraint

- Do not delete or rewrite user data, live CASS archives, or launchd plists during issue/spec creation.
- Do not wire a new health-watchdog mechanism inside spec 016 without explicit scope expansion.
- Do not rely on a local-only binary or uncommitted script as the durable fix.
- Do not treat `com.cass.health-watchdog` repair as a substitute for proving `com.cass.index-watch` live ingestion.

## Acceptance Criteria

1. `cass watchdog run --help` or an equivalent documented command dispatches from the verified build instead of returning `Could not parse arguments`.
2. `cass capabilities --json` / robot docs expose the intended watchdog surface when applicable.
3. `com.cass.health-watchdog.plist` points at the verified installed command or the approved replacement path.
4. A launchd smoke captures `launchctl print`, last exit code, stdout/stderr/log evidence, and proves the argument-parse failure is gone.
5. Tests cover CLI dispatch and the selected install/uninstall path.
6. Spec 016 watcher proof remains separately satisfied by `com.cass.index-watch`.

## Out of Scope

- Promoting the spec 016 shadow DB/index to live CASS.
- Repairing live DB freelist corruption.
- Implementing a new watcher architecture.
- Replacing `com.cass.index-watch`.

## Selected Shape

Regression follow-up for the spec 007 command surface. First compare closed spec 007 artifacts against current `src/lib.rs`, `src/watchdog.rs`, capabilities, and release binary behavior; then repair the smallest missing dispatch/capability/install seam with focused tests and launchd smoke proof.
