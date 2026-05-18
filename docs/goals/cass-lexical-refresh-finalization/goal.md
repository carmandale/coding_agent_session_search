# Cass Lexical Refresh Finalization

## Objective

Get cass back to a clean, documented, usable state for Dale's recent and historical sessions by isolating and fixing the remaining large-corpus lexical refresh finalization failure, then verify live behavior and report upstream only with concise evidence.

## Original Request

"I agree with the next 4 steps you are proposing. $goalbuddy these and get them done"

## Intake Summary

- Input shape: `existing_plan`
- Source spec path: none yet; create a focused spec if diagnosis confirms the remaining bug.
- Source bead: none yet; create or claim the focused bead before patching.
- Execution lane: `dale_workflow`
- Execution lane reason: defaulted to Dale workflow; no explicit direct-goal opt-out phrase matched
- Audience: Dale, using cass from local infrastructure and upstream-compatible workflows.
- Authority: `approved`
- Proof type: `live_state`
- Prior failure signal: `true`
- Completion proof: live cass can finish or correctly finalize the large-corpus lexical refresh state without restarting into another OOM, recent sessions remain searchable, and any upstream comment/issue is factual and concise.
- Likely misfire: declaring victory because individual watch-once indexing/search works while `cass status` remains stale or the pending lexical refresh marker still forces unsafe rebuild loops.
- Blind spots considered: large historical corpus size, live index mutation risk, installed binary drift, upstream issue hygiene, and accidental local band-aid machinery.
- Existing plan facts: diagnose the remaining finalization bug; create the focused issue/spec; patch the rebuild/finalization path; verify live; comment upstream only after evidence is clean.
- Purpose source sections: prior senior-dev assessment and spec 017 evidence.

## Purpose Contract

- Confirmation: `user_confirmed`
- Tangible outcome: Dale can actually use cass against recent and historical sessions through documented or upstream-compatible behavior, without custom local watcher/index band-aids.
- Done proof: status, marker, index metadata, and search probes agree that the index is usable after the large-corpus refresh path; repository changes are tested, committed, and pushed; upstream is updated only with evidence that remains true after verification.
- False positives: a GoalBuddy board alone, a new spec alone, an isolated search hit alone, or a local patch that is not installed/proven against the live cass state.
- Required outcome checks:
  - OC1: Focused issue/spec records the remaining bug and separates it from upstream #239/#240.
  - OC2: Code change prevents completed DB-matching lexical refresh state from restarting into a full rebuild solely because marker/status metadata lagged.
  - OC3: Focused tests and Rust verification pass.
  - OC4: Live cass proof shows the May 17 search path still works and the refresh marker/status no longer misrepresent the completed state.
  - OC5: Upstream note is posted only if it is concise, true, and useful.

## Goal Kind

`recovery`

## Current Tranche

Complete the full remaining recovery loop: evidence, focused issue/spec, patch, verification, live proof, commit/push, and upstream note if warranted.

## Non-Negotiable Constraints

- Do not delete files or derived index directories.
- Do not introduce new `rusqlite` code.
- Do not run bare `cass`; use `--json` or robot-mode commands.
- Work on `main`, stage only owned files, and push `main` plus `main:master` after verified changes.
- Treat live cass state and installed binary changes as high-risk; make backups/proof explicit.
- Prefer clean upstream-compatible behavior over local band-aids.

## Stop Rule

Stop only when a final audit proves the full original outcome is complete, or when a specific blocker prevents safe local progress and is documented on the board.

## Canonical Board

Machine truth lives at:

`docs/goals/cass-lexical-refresh-finalization/state.yaml`

If this charter and `state.yaml` disagree, `state.yaml` wins for task status, active task, receipts, verification freshness, and completion truth.

## Run Command

```text
/goal Follow docs/goals/cass-lexical-refresh-finalization/goal.md.
```
