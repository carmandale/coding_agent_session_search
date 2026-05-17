# Recover cass session ingestion and watcher

## Objective

Drive cass from the current partially working state to the user's actual intended state: synced deliberately with upstream, priority agent histories ingested and searchable, and the watcher running so new sessions stay searchable.

## Original Request

> "my goal was clear, I thought. be in sync with upstream. process all sessions and allow them to be searchable. sessions that matter are pi-agent, claude code, an codex, with opencode, factory and others being bonus sessions. then set up a watcher so that all sessions are processed and part of the searchable system." - user, 2026-05-16

> "$goalbuddy set a new goal to solve this. create a new $issue if necessary." - user, 2026-05-16

## Intake Summary

- Input shape: `recovery`
- Audience: Dale Carman, as cass user/operator
- Authority: `requested`
- Proof type: `metric`
- Completion proof: upstream is incorporated or explicitly blocked with evidence; Pi Agent, Claude Code, and Codex histories are indexed and searchable from live cass; `com.cass.index-watch` is loaded and proven by a new-session or modified-session ingestion probe.
- Likely misfire: completing spec artifacts, code commits, or narrow Pi watch-once changes while Pi remains under-indexed, upstream remains behind, or the watcher remains unloaded.
- Blind spots considered:
  - Existing spec 015 captures part of the problem but is not the full owner outcome.
  - `cass health --json` can be healthy for lexical search while the watcher is absent.
  - Semantic search missing is not a blocker because lexical fallback is the required surface.
  - Upstream sync can conflict with local cass/fork changes and must preserve the user's fork work.
- Existing plan facts:
  - `specs/016-cass-recovery-ingestion/spec.md` owns the recovery issue and bead `coding_agent_session_search-1vxuf`.
  - Issue-time live baseline: local `HEAD=b807ef175dcdeeb48b912a22913fbcd68fb86cb8`, `upstream/main=c5d7be3b585a38546759cb5331401b9ad1ac06ba`, `HEAD...upstream/main = 19 ahead / 12 behind`.
  - Issue-time cass stats: `codex=5,712`, `claude_code=2,574`, `pi_agent=36`, total `9,657` conversations.
  - Issue-time raw counts: Pi `2,076` jsonl files, Claude `2,557` jsonl files, Codex `4,187` jsonl files.
  - Issue-time launchd state: `com.cass.health-watchdog` and `com.cass.sync-to-mini` loaded; `com.cass.index-watch` absent.

## Goal Kind

`recovery`

## Current Tranche

Run the Dale Codex workflow for `specs/016-cass-recovery-ingestion/` from shaping through finalize. This tranche is complete only when the live cass installation proves the full owner outcome, not when planning or implementation artifacts exist.

## Non-Negotiable Constraints

- No destructive git or filesystem operations.
- No file deletion without explicit written permission.
- No new `rusqlite` code; use frankensqlite for new SQLite work.
- Do not run bare interactive `cass`; use `--json`, `--robot`, or non-interactive subcommands.
- Do not auto-download semantic models.
- Do not push to upstream; sync against upstream and push verified fork work only to origin.
- Preserve target-scoped dirty-file safety.
- Keep watcher and launchd side effects explicit in receipts.

## Stop Rule

Stop only when a final audit maps current live evidence back to the original user outcome and records `full_outcome_complete: true`.

Do not stop after `$codex-shape`, `$codex-plan`, `$codex-review`, or `$codex-implement`. Planning and implementation are setup. The outcome is live upstream state, live searchable sessions, and a live watcher.

## Slice Sizing

Use the Dale command workflow as the control spine. Each command is a PM task because each command owns its own gates, artifacts, and receipts.

## Canonical Board

Machine truth lives at:

`docs/goals/cass-session-ingestion-recovery/state.yaml`

If this charter and `state.yaml` disagree, `state.yaml` wins for task status, active task, receipts, verification freshness, and completion truth.

## Run Command

```text
/goal Follow docs/goals/cass-session-ingestion-recovery/goal.md.
```

## PM Loop

On every `/goal` continuation:

1. Read this charter.
2. Read `state.yaml`.
3. Re-check the intake: original request, input shape, authority, proof, blind spots, existing plan facts, and likely misfire.
4. Work only on the active board task.
5. Write a compact task receipt.
6. Update the board.
7. Continue to the next required command unless a real blocker is hit.
8. Finish only with a Judge/PM audit receipt that records `full_outcome_complete: true`.
