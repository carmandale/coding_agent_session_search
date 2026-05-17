---
title: "recover cass session ingestion and watcher"
date: 2026-05-16
bead: coding_agent_session_search-1vxuf
---

## Source (verbatim)

> "I don't know if you can look at the recent claude code sessions but they have been a complete failure. $ground and look at recent sessions, specs, and commits and see if you understand what is going on, what the purpose and intent is and what needs to happen" - user, 2026-05-16

> "my goal was clear, I thought. be in sync with upstream. process all sessions and allow them to be searchable. sessions that matter are pi-agent, claude code, an codex, with opencode, factory and others being bonus sessions. then set up a watcher so that all sessions are processed and part of the searchable system." - user, 2026-05-16

> "$goalbuddy set a new goal to solve this. create a new $issue if necessary." - user, 2026-05-16

## Problem

The current cass state does not satisfy the user's actual goal, even though recent workflow artifacts and commits make it look like progress happened.

Live baseline captured during issue creation:

- `git fetch upstream main` shows `upstream/main` at `c5d7be3b585a38546759cb5331401b9ad1ac06ba`; local `HEAD` is `b807ef175dcdeeb48b912a22913fbcd68fb86cb8`, `19` commits ahead and `12` behind upstream.
- `cass health --json` is healthy for lexical search, but semantic search is missing and correctly falling back to lexical. Semantic absence is not the blocker.
- `cass stats --json` reports `9,657` conversations and `903,256` messages. Agent counts include `codex=5,712`, `claude_code=2,574`, `opencode=976`, `factory=66`, and only `pi_agent=36`.
- Raw source counts show `~/.pi/agent/sessions` has `2,076` jsonl files, `~/.claude/projects` has `2,557` jsonl files, `~/.codex/sessions` has `4,187` jsonl files, OpenCode has more than `115,000` json part files, and factory has `656` json/jsonl files.
- `launchctl list` shows `com.cass.health-watchdog` and `com.cass.sync-to-mini`, but no `com.cass.index-watch`; forward capture is not active.
- `docs/goals/watch-once-streaming-scan/state.yaml` still has spec 015 active at T003, with code verification and finalize queued. `specs/015-watch-once-streaming-scan/tasks.md` still leaves full-corpus verification, watcher reload, and no-regression proof unchecked.

The root cause is not that the original intent was unclear. The root cause is that the work split the goal across narrower specs and then treated stage artifacts or partial implementation commits as meaningful delivery without requiring one live acceptance packet for upstream sync, priority-agent ingestion/searchability, and watcher operation.

## Requirements

1. Bring the fork into a deliberate upstream state: `upstream/main` must be fetched, compared, and either incorporated into the working branch or explicitly documented as intentionally not incorporated with a blocking reason.
2. Preserve the user's fork work while reconciling upstream. Do not discard local commits or dirty files, and do not create a new branch unless the user explicitly approves it.
3. Make the priority session classes searchable from the real cass installation: Pi Agent, Claude Code, and Codex. OpenCode, factory, and other connectors are bonus coverage and must not block the priority path unless they are actively causing corruption or preventing the priority agents from indexing.
4. Treat raw-source-to-DB reconciliation as a required truth surface. For each priority agent, record raw discovered source count, DB conversation count, indexed/searchable proof, and any quarantined/skipped files with reasons.
5. Fix the Pi Agent historical gap. The current `pi_agent=36` DB count is not acceptable against `2,076` raw jsonl files.
6. Keep lexical search as the required search surface. Semantic search remains opt-in and must not be used as a completion blocker unless the user explicitly requests semantic model installation.
7. Install or repair the launchd watcher path so `com.cass.index-watch` is loaded, running, and processing new sessions through the same shipped cass binary the user will keep using.
8. Prove watcher behavior from a new-session or modified-session probe, not from plist existence alone.
9. Preserve existing workflow safety: no destructive recovery, no file deletion without explicit user permission, no new `rusqlite` code, and no bare interactive `cass`.
10. If code changes are required, verify with the repo-required Rust checks and focused tests before claiming completion.

## Constraint

- No destructive git or filesystem operations. Do not use `git reset --hard`, `git clean`, `rm -rf`, or any file deletion.
- Do not overwrite `.env` or credentials. Do not expose secret values in receipts or chat.
- New SQLite code must use frankensqlite APIs, not new `rusqlite` usage.
- Do not push to upstream. Upstream is read-only for this work; pushes go only to the user's fork after verification.
- Do not call bare `cass`; use `--json`, `--robot`, or an explicitly non-interactive subcommand.
- Do not auto-download semantic models. Lexical fallback is valid when reported truthfully.
- Do not let spec 015's implementation state substitute for this spec's product-level acceptance.
- Existing unrelated dirty files are target-scoped noise, not a reason to stop. Pause only if a file in this spec's active write scope is dirty or changes unexpectedly.

## Acceptance Criteria

1. Upstream sync is resolved: `git merge-base --is-ancestor upstream/main HEAD` succeeds, or the final receipt records a concrete blocker explaining why upstream could not be incorporated yet. The final receipt includes the exact upstream SHA, local HEAD SHA, and ahead/behind counts.
2. Priority-agent ingestion is resolved:
   - Pi Agent: at least `95%` of raw discovered Pi jsonl sessions are represented in cass DB/search, or each missing file is accounted for by quarantine/skip evidence. With the issue-time count of `2,076`, the target is at least `1,973` accounted-for Pi sessions.
   - Claude Code: recent and historical Claude Code sessions are represented and searchable, with raw-vs-DB reconciliation recorded.
   - Codex: recent and historical Codex sessions are represented and searchable, with raw-vs-DB reconciliation recorded.
3. Searchability is proven with real queries: at least three known strings from real source sessions for each priority agent return via `cass search ... --robot --mode lexical --fields minimal --robot-meta`.
4. Watcher is live and verified: `launchctl list` includes `com.cass.index-watch`; cass health/status reports no active rebuild, no stale lock, and watcher/forward capture truthfully; a new or modified session probe is indexed and searchable within the expected watcher window.
5. The recovery either completes, supersedes, or explicitly routes spec 015. There must be no ambiguous state where spec 015 still appears to be the path to done while this recovery spec is the active owner of the user's goal.
6. Verification artifacts include exact commands and exact outputs or summarized machine-readable fields for `git`, `cass health --json`, `cass stats --json`, launchd watcher state, priority-agent searches, and any code/test commands run.
7. No completion claim is made while `docs/goals/cass-session-ingestion-recovery/state.yaml` still has required queued or active tasks for this outcome.

## Out of Scope

- Semantic model installation or semantic backfill, unless the user explicitly requests it.
- TUI redesign, browser/E2E tests, or UI polish.
- Rewriting every connector. Non-priority connectors are bonus unless they block the priority agents or corrupt shared indexing state.
- Public PR creation, upstream pushes, force-pushes, or history rewrites.
- Destructive cleanup of old indexes, raw mirrors, quarantine files, or session data.
- A broad frankensqlite architecture rewrite unless live evidence proves it is the smallest necessary blocker for Pi Agent historical ingestion. If that happens, pause and create/route a separate upstream issue/spec before expanding scope.

## Selected Shape

Approved by `$codex-shape`: **Priority-Scoped Manifest Recovery**.

This spec remains the product-level owner of the user's goal. The recovery must not complete because spec 015 advances, because lexical health looks better, or because a one-shot watcher smoke passes. Completion requires one live evidence packet that proves upstream sync, priority-agent historical ingestion/searchability, and durable watcher capture together.

The selected shape:

1. Freeze a Pi Agent, Claude Code, and Codex corpus manifest before any ingestion or watcher mutation.
2. Incorporate `upstream/main` until `git merge-base --is-ancestor upstream/main HEAD` passes, then review upstream-touched indexer, watcher, storage, and search surfaces for no-net-reversion.
3. Build/deploy and prove the same cass binary under test with `git rev-parse`, `which cass`, `cass --version`, and a binary hash when deployed.
4. Capture recovery scope baseline with `cass sources agents list --json`. Prefer explicit priority roots through `cass index --watch-once ... --json --no-progress-events`; if temporary connector exclusions are unavoidable, use `--keep-indexed-data`, record before/after config, restore the prior config, and prove priority search still works after full scope is restored.
5. Before broad recovery, run one vertical proof: one manifest-selected Pi/Claude/Codex identity is ingested, indexed, searchable, and mapped back to the expected source path.
6. Before live mutation, write exact route commands, numeric thresholds, retry limits, stop conditions, and re-entry criteria for stale index, missing watcher, lock/busy, OOM/stall, WAL/FTS corruption, and verifier failures. No broad recovery starts until this gate exists.
7. Run broad priority-scoped historical recovery under those thresholds.
8. Verify identity coverage, no duplicate identity rows, source mapping, at least 10 identities per priority agent, or all if fewer, across newest/oldest/random buckets, idempotent repeat, and fresh lexical retrieval from safe unique probes.
9. Restore or confirm full configured scope before watcher proof.
10. Install/reload `com.cass.index-watch`; prove launchctl/process args match the same binary; prove a priority-agent incremental probe is searchable within 120 seconds; repeat launchd/process/search proof after at least 10 minutes or one watchdog interval, whichever is shorter.
11. If code changed, run `rch exec -- env CARGO_TARGET_DIR=/tmp/cass-check-target cargo check --all-targets`, `rch exec -- env CARGO_TARGET_DIR=/tmp/cass-check-target cargo clippy --all-targets -- -D warnings`, `rch exec -- env CARGO_TARGET_DIR=/tmp/cass-check-target cargo fmt --check`, UBS on changed files, and targeted tests for touched behavior. If any required verifier cannot run, record the exact blocker and do not claim completion.
12. Route spec 015 as closed, superseded, or subordinate evidence and produce the final evidence packet.

### Route Policy Minimums

- Stale index: run exactly one non-interactive refresh command chosen from `cass health --json` `recommended_action`; if still stale, capture health plus command output and either apply one planned repair or block.
- Missing watcher: run one planned install/reload cycle, then re-run `launchctl print`; if still absent, block watcher acceptance.
- Lock/busy: verify owner PID/command from lock metadata; never start a concurrent index; wait one bounded interval only if owner is alive; stale-owner handling must use an existing safe cass/doctor path, not file deletion.
- Pi OOM/stall: if one plan-defined interval has zero DB/search progress or RSS exceeds the plan ceiling, SIGTERM only the recovery-owned process and switch once to canary or DB/archive-first route; a second stall blocks completion.
- WAL/FTS corruption: stop live mutation, preserve evidence, and route once to shadow/canary or record an upstream/frankensqlite blocker without destructive cleanup.
- Verification failure: do not continue to watcher proof if identity, integrity, idempotency, or lexical retrieval fails.

### Plan Slice Order

1. Freeze manifest, incorporate upstream, and write route policy.
2. Same-binary deployment plus first vertical searchable identity proof.
3. Broad priority-scoped historical recovery with the reversible scope-toggle contract.
4. Identity, integrity, idempotency, and fresh lexical retrieval proof.
5. Watcher reload and durability proof after full scope is restored.
6. Spec 015 routing, final evidence packet, and workflow closeout.

See `shaping-transcript.md` for the challenge transcript and terminal `VERDICT: APPROVED`.

<!-- issue:complete:v1 -->
