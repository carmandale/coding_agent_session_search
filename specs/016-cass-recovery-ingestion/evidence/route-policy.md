---
title: "Spec 016 route policy"
date: 2026-05-16
bead: coding_agent_session_search-1vxuf
---

# Route Policy

This file gates live recovery work for spec 016. No broad indexing mutation starts
until this policy exists and the current `cass status --json` output satisfies
the relevant entry criteria.

## Shared Command Rules

- Use the installed user-facing cass binary unless a receipt explicitly switches
  to a tested checkout build: `$(which cass)`.
- Never run bare `cass`; use `--json`, `--robot`, or a noninteractive command.
- Never delete index data, raw mirrors, quarantines, session files, plists, or
  locks. If a stale artifact appears to be the blocker, record it and stop.
- Do not run concurrent writers. Before any indexing mutation, capture:
  - `cass status --json`
  - `cass health --json`
  - `ps -axo pid,ppid,rss,etime,command | rg 'cass index|cass doctor|cass watchdog'`
- A process is recovery-owned only when it was started by this spec and its
  command/output file is recorded under `evidence/`.

## Stale Index

Entry condition:

- `cass health --json` or `cass status --json` reports stale lexical readiness
  and does not report an active rebuild, watch run, doctor repair, lock/busy
  state, or live index writer.

Command:

- Run exactly the first `refresh-lexical-index` command recommended by
  `cass health --json` or `cass status --json`.
- Save stdout, stderr, and exit code under `evidence/runtime-refresh/`.
- Immediately run the paired health/status verification command and save it.

Retry limit:

- One refresh attempt. No loop.

Stop condition:

- If the verification output is still stale, or if the command exits nonzero,
  record the JSON and command output in `implement-receipt.md` before choosing a
  different route.

Re-entry:

- Continue only after status reports no active rebuild/lock and the next planned
  mutation is a canary or a documented fallback.

## Missing Watcher

Entry condition:

- `launchctl print gui/$(id -u)/com.cass.index-watch` exits nonzero or the
  printed service is not running.

Command:

- Inspect the existing plist:
  `~/Library/LaunchAgents/com.cass.index-watch.plist`.
- If the plist is present and points at the installed cass binary, run:
  `launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/com.cass.index-watch.plist`.
- Verify with `launchctl print`, `launchctl list`, `ps`, `cass status --json`,
  and a watcher search probe.

Retry limit:

- One bootstrap/reload cycle for the unchanged plist.
- One targeted non-destructive plist/binary-path fix if the first cycle proves a
  concrete mismatch.

Stop condition:

- Stop if launchd rejects the service after the targeted fix, if the plist would
  need deletion/replacement, or if the installed binary cannot run
  `cass index --watch` noninteractively.

Re-entry:

- Continue only after `com.cass.index-watch` is loaded and a priority-agent probe
  becomes searchable.

## Index Lock Or Busy State

Entry condition:

- `cass status --json`, `cass health --json`, command stderr, or logs report an
  index lock, database busy state, active rebuild, active watch, or active index
  run.

Command:

- Capture `cass status --json`.
- Capture `ps -axo pid,ppid,rss,etime,command | rg 'cass index|cass doctor|cass watchdog'`.
- If status or lock metadata names a PID, prove whether that PID is alive with
  `ps -p <pid> -o pid,ppid,rss,etime,command`.

Retry limit:

- One bounded wait of 120 seconds when the owner PID is alive and making progress.
- No manual lock deletion.

Stop condition:

- Stop if the owner is stale but cass exposes no safe repair command, if two
  consecutive status samples show no progress, or if the same lock blocks after
  the single bounded wait.

Re-entry:

- Continue only after `cass status --json` reports no active writer or the
  selected route explicitly becomes a read-only diagnostic path.

## Doctor Mutation Lock

Entry condition:

- `cass status --json` reports `doctor_summary.active_repair.active=true` or an
  equivalent active repair field.

Command:

- Capture full `cass status --json`.
- Capture process evidence for `cass doctor`.
- Do not start `cass index`, `cass doctor --fix`, or watch-once recovery while
  the repair is active.

Retry limit:

- One bounded wait of 120 seconds when the doctor PID is alive.

Stop condition:

- Stop if the active repair remains after the bounded wait or if no owner PID can
  be proven safely.

Re-entry:

- Continue only after status shows no active doctor repair.

## OOM Or Stall

Entry condition:

- A recovery-owned indexing process exceeds the RSS ceiling, makes no DB/search
  progress across samples, or logs allocation/OOM/stall failures.

Thresholds:

- RSS ceiling: 24 GB resident memory for a recovery-owned process.
- Stall interval: 5 minutes without any increase in priority-agent DB
  `source_path` count and without new progress output.
- Process sample interval: 30 seconds during broad priority recovery.

Command:

- Capture `ps -o pid,ppid,rss,etime,command -p <pid>` samples.
- Capture a DB/source-path count sample before and after the stall interval.
- Send SIGTERM only to a recovery-owned process if it must be stopped.

Retry limit:

- One retry on the same agent after switching from broad root to smaller canary
  or chunked watch-once input.

Stop condition:

- A second stall or OOM on the same priority agent blocks completion and must be
  recorded with the exact path/agent/log evidence.

Re-entry:

- Continue only after the stalled recovery-owned process has exited and status
  reports no active writer.

## WAL, FTS, Or Corruption Error

Entry condition:

- cass reports WAL, FTS, SQLite/frankensqlite corruption, schema mismatch,
  malformed index, failed lexical publish, or quarantine-heavy validation errors.

Command:

- Capture the command output, `cass health --json`, `cass status --json`, and
  `cass doctor --json`.
- Do not delete or move the live database, WAL, index, quarantine, or raw mirror.

Retry limit:

- No blind retry. One alternate route is allowed only if it is read-only or uses
  a fresh shadow data dir without deleting live assets.

Stop condition:

- Stop if the priority-agent accounting threshold cannot be met without
  destructive cleanup or broad frankensqlite architecture work.

Re-entry:

- Continue only after a shadow/canary route proves searchability or the blocker
  is routed to a separate issue/spec.

## Verifier Failure

Entry condition:

- Any required identity, reconciliation, idempotency, lexical search, launchd, or
  Rust verifier command fails.

Command:

- Save exact command, stdout, stderr, exit code, and the smallest failing input.
- Fix the root cause if it is inside the approved spec scope.

Retry limit:

- Two implementation attempts per failure class.

Stop condition:

- After two failed attempts, stop and write the blocker with what changed and
  what evidence would unblock continuation.

Re-entry:

- Continue only after the failing verifier passes or the failure is explicitly
  documented as the final blocker.

## Branch-Policy Conflict

Entry condition:

- Current branch is not `main`, or upstream incorporation/final commit would
  require committing/pushing from an unauthorized branch.

Command:

- Capture `git status --short --branch`, `git rev-parse HEAD`,
  `git rev-parse upstream/main`, `git merge-base HEAD upstream/main`, and
  `git rev-list --left-right --count HEAD...upstream/main`.
- Continue read-only/runtime evidence work that does not require committing.

Retry limit:

- No branch creation.

Stop condition:

- Stop before final commit/push, or before a merge commit on the unauthorized
  branch, unless the user authorizes the current branch or a non-destructive move
  back to `main`.

Re-entry:

- Continue finalization only after branch authorization is explicit.

## Temporary Agent Scoping

Entry condition:

- A non-priority connector blocks Pi Agent, Claude Code, or Codex recovery with a
  measured lock, OOM, stall, corruption, or command failure.

Command:

- Capture `cass sources agents list --json` before any scope change.
- Use only:
  `cass sources agents exclude <agent> --keep-indexed-data`
  and later:
  `cass sources agents include <agent>`.
- Restore the previous state before watcher proof.

Retry limit:

- One temporary exclusion cycle per blocking non-priority agent.

Stop condition:

- Stop if exclusion would remove indexed data, leave configured agents disabled,
  or become a substitute for full-scope restoration.

Re-entry:

- Continue only after `cass sources agents list --json` proves no undocumented
  disabled agents remain and priority-agent search still works.
