# pi_agent watch-once ingest stalls in `_platform_memset` loop with 22 GB RSS

## Objective

Drive `specs/014-pi-agent-memset-stall/` through the canonical Dale Codex Workflow until the upstream PR is open and `cass index --watch-once ~/.pi/agent/sessions` completes the user's pi-agent historical backfill (≥ 1,970 conversations = 95 % of the 2,073 jsonl corpus) on the v0.4.7 binary + PR #233 chunk-size fix, with peak RSS staying under 8 GB.

<!-- REVISED 2026-05-15 during /goal-prep: corpus threshold corrected from ≥ 2,500 → ≥ 1,970 to match spec.md acceptance #1 amendment landed during /codex-plan and /codex-review (real corpus is 2,073 jsonl, not the earlier ≥ 2,800 estimate). -->

Spec, evidence table, selected shape, and acceptance criteria live in `specs/014-pi-agent-memset-stall/spec.md` and are the source of truth for *what* and *why*. This board owns *how* the workflow moves through `$codex-plan → $codex-review → $codex-implement → $code-verify → $finalize → upstream PR`.

The first three workflow commands are done (T001 `$codex-plan` → T002 `$codex-review` APPROVED, 5 rounds). The active task is **T003 `$codex-implement`**.

## Original Request

> "/issue for pi_agent stall (~10 min) — captures findings (PID, RSS pattern, that it was a different signature from spec-013) before they get stale" — user, 2026-05-15

Anchor quotes from the broader /goal context that drove the spec:

> "the goal is to be in sync with upstream and running properly, capturing all sessions and cass working." — user, 2026-05-13
> "claude code and codex are top priorities, with pi-agent next and then opencode." — user, 2026-05-14

## Intake Summary

- **Input shape**: `existing_plan` — spec 014 has a complete Selected Shape (Phase 1 Localise / Phase 2 Fix / Phase 3 Verify + upstream).
- **Audience**: Dale (fork maintainer) and upstream `Dicklesworthstone/coding_agent_session_search`.
- **Authority**: `requested` — user invoked `/issue` and `/goalbuddy` on the spec.
- **Proof type**: `artifact` — symbolised `sample` before/after + `success: true` run receipt with `conversations >= 1,970` and peak RSS < 8 GB.
- **Completion proof**: `cass index --watch-once ~/.pi/agent/sessions --json --no-progress-events` returns `success: true` with `conversations >= 1,970` (95 % of 2,073 jsonl), peak RSS during the run stays under 8 GB, post-fix `sample` shows the `_platform_memset` hot frame is gone, and the upstream PR is open.
- **Likely misfire**: skipping `$codex-review` between `$codex-plan` and `$codex-implement`, or letting `$codex-implement` ship a fix before profiling actually identifies the allocation site. Spec 014 + the canonical workflow guard both of these.
- **Blind spots considered**:
  - Release binary frames are stripped; `$codex-implement` will need a `profiling` build to symbolicate.
  - The bad allocation may live inside `franken-agent-detection`; if so the fix lands as a cap at the cass ingest boundary, not by vendoring (see Constraints).
  - The 22 GB RSS may be cumulative across `Vec<NormalizedConversation>` rather than one pathological session — fix shape may shift between candidate (a) and (b).
  - The stall watchdog didn't fire because the lock heartbeat kept it satisfied. That's a separate follow-up issue, out of scope here.
- **Existing plan facts** (recorded in `state.yaml.goal.intake.existing_plan_facts`):
  - `$issue` is satisfied (spec.md + bead exist with `issue:complete:v1`).
  - `$codex-shape` is satisfied via the explicit `shape-skip:` reason in `specs/014-pi-agent-memset-stall/log.md`.
  - Phase 1/2/3 selected shape is preserved verbatim from spec 014.
  - Post-PR #233 binary already cleared claude_code/codex/opencode without stall — pi is the lone holdout.

## Goal Kind

`existing_plan` — first active task is `$codex-implement` (T003), per the Workflow Resume Map. `$codex-plan` (T001) and `$codex-review` APPROVED (T002) are both done.

## Current Tranche

Move spec 014 from "shaped" to "shipped":

1. **`$codex-plan`** produces `plan.md`, `tasks.md`, `planning-transcript.md`, and the plan provenance sentinel.
2. **`$codex-review`** runs as the mandatory gate; iterate until Codex returns `VERDICT: APPROVED`.
3. **`$codex-implement`** does Phase 1 profiling, Phase 2 fix, and the focused regression test inside its own task list and write boundaries.
4. **`$code-verify`** independently checks all five spec-014 acceptance criteria.
5. **`$finalize`** closes the bead, writes the handoff under `thoughts/shared/handoffs/`, commits + pushes.
6. **Upstream PR** opens on `Dicklesworthstone/coding_agent_session_search` mirroring PR #233's body shape (Symptom / Repro / Root cause / Fix / Verification). If `$codex-implement` already opened it, T006 just records the URL.
7. **T999 Judge** audits the whole tranche before the goal can close.

Continuous-execution default applies: after each provenance sentinel lands, PM activates the next workflow command without stopping for unsolicited approval.

## Non-Negotiable Constraints

- **Never skip `$codex-review`.** The same-session Phase B approval inside `$codex-plan` does not substitute for the fresh independent review gate.
- **No external-crate patching from this repo.** If profiling points at `franken-agent-detection`, the fix lands as a cap on the cass ingest boundary or as an issue filed upstream against that crate. Do not vendor a patched fork without explicit user approval.
- **No destructive recovery.** Current `pi_agent=33` rows stay. Backfill is additive.
- **Single source-of-truth binary.** Whatever lands ships via `~/.local/bin/cass.real` and the launchd watcher daemon. No "pi-only" build flavour.
- **Upstreamable.** Diff must be small enough that the upstream PR matches PR #233 shape: one commit, ≤ ~50 LOC, no infrastructure additions, no internal-spec terminology in the commit message, no AI attribution.
- **Do not regress PR #233.** Chunk-size behaviour from commit `e429eaab` must remain. T999 spot-checks 3 random claude_code + 3 random codex conversations after the fix.
- **Watcher uptime.** `com.cass.index-watch` keeps running for forward capture during this work. Stop it only inside Worker/PM tasks that explicitly need the index-run lock; restart before the task closes.

## Stop Rule

Stop only when T999 audits `full_outcome_complete: true` with: all five spec-014 acceptance criteria met, bead closed, handoff written, upstream PR open, and no spec-013 regression.

Do not stop after `$codex-plan` — the plan is setup, not delivery.
Do not stop after `$codex-implement` — verification + upstream PR are part of the tranche.
Do not stop after the upstream PR opens if any acceptance criterion is unverified.

## Slice Sizing

Each `$codex-*` command is one PM slice end-to-end. Do not split `$codex-plan` into "draft plan" + "stress-test plan" — that's what the command's two-phase shape already does internally. Do not split `$codex-implement` into per-phase Worker tasks — its own `tasks.md` is the inner board.

If `$codex-implement` discovers the fix needs to touch `franken-agent-detection` (external crate), escalate to user before activating any change there — that is a scope change, not a slice-sizing decision.

## Canonical Board

`docs/goals/pi-agent-memset-stall/state.yaml` (machine truth)

`http://goalbuddy.localhost:41737/pi-agent-memset-stall/` (live UI)

## Run Command

```
/goal Follow docs/goals/pi-agent-memset-stall/goal.md.
```

## PM Loop

Standard PM loop: read charter, read state, validate intake, work the active workflow command, record the provenance sentinel and next-command pointer in the receipt, advance the board, audit at the final boundary only. Workflow command tasks own their own gate checks, artifacts, and write boundaries — PM does not pre-create plan.md/tasks.md by hand or duplicate `$codex-implement` with a generic Worker task.
