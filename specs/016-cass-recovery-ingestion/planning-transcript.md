<!-- plan:complete:v1 | harness: unknown | date: 2026-05-16T13:47:37Z -->

# Codex Plan Transcript: cass recovery ingestion

Review ID: `6853697e`
Codex session: `019e30e2-5375-7d82-ba42-5b546d6b123f`
Command: `$codex-plan specs/016-cass-recovery-ingestion/`

## Phase A (Research Challenge) — Round 1

Protocol check passed: I did **not** find task breakdown labels, implementation sequencing, final-plan prose, or file-by-file change lists in [context](/tmp/codex-plan-research-6853697e.md), so Phase A can proceed.

**Blast Radius You May Be Underestimating**
- `watchdog` wiring is not just a local fix: adding/changing a CLI subcommand impacts `capabilities`/`robot-docs` contract tests and goldens (see [src/lib.rs](/Users/dalecarman/dev/coding_agent_session_search/src/lib.rs) and `tests/cli_robot.rs` golden checks).
- You have two launchd surfaces in play (`com.cass.index-watch`, `com.cass.health-watchdog`), and right now watchdog is configured to call a command the installed binary doesn’t expose. That mismatch can keep failing in the background while ingestion “looks fixed.”
- Spec ownership/state blast radius is real: [spec 015 state](/Users/dalecarman/dev/coding_agent_session_search/docs/goals/watch-once-streaming-scan/state.yaml) is still active while [spec 016](/Users/dalecarman/dev/coding_agent_session_search/specs/016-cass-recovery-ingestion/spec.md) requires explicit routing/supersession.
- There is an active doctor mutation lock in current health/status output; your recovery flow currently treats this as noise, but it can invalidate mutation timing and interpretation.

**Load-Bearing Assumptions Still Unverified**
- “Existing CLI is enough” is unproven for watcher repair: `watchdog` is absent from current command surface while [src/watchdog.rs](/Users/dalecarman/dev/coding_agent_session_search/src/watchdog.rs) is untracked.
- Raw file count ≠ accountable conversation identity. Your direction says this, but doesn’t define the canonical identity rule for files that collapse via source-path/start-time fallback or merge heuristics.
- DB provenance uniqueness is not universal: when `external_id` is missing, identity fallback behavior can merge/split differently than manifest-level counting implies.
- “Priority-only routing won’t perturb global behavior” is unproven while launchd/job config and agent-scoping settings remain mutable surfaces.

**Spec Requirements Not Fully Covered by Current Direction**
- Acceptance requires 3 real lexical probe strings per priority agent; your direction mentions sampling identities, not query-level lexical proof obligations.
- Acceptance requires no completion while recovery state board has required queued/active tasks; your direction doesn’t explicitly include board-state closure as a hard gate.
- Acceptance requires exact upstream ancestor/blocker proof; your direction says upstream is load-bearing, but does not yet define a hard stop condition if merge is blocked on current branch/policy constraints.
- If code changes are made, spec requires full Rust + UBS verifier floor; your direction treats code as “only if needed” but doesn’t carry verification obligations as mandatory once touched.

**Stronger Alternative You Didn’t Fully Explore**
- Use a strict **runtime-first canary route** before code changes:
1. Prove stale-index recovery + priority-agent lexical retrieval using only shipped surfaces and recommended commands.
2. Prove raw→DB→search reconciliation with explicit accounting math and quarantine reasons.
3. Only if watcher proof fails due command-surface mismatch, do a minimal CLI/runtime parity patch (including contract/golden updates), then re-prove with same binary/plist/process args.
- This reduces risk of expanding into command-contract churn before proving the core ingestion/search problem is actually unsolved by current runtime behavior.

**Risks Introduced by Your Proposed Direction**
- Early watchdog wiring can balloon into contract/test regeneration work and delay core recovery.
- Temporary agent scoping can create false success if restoration checks are weak.
- Long broad watch-once runs against huge Codex/Claude files can produce quarantine-heavy outcomes that look like progress while failing coverage thresholds.
- A single “frozen manifest” taken too early can drift under live session churn, making reconciliation math non-repeatable unless you define snapshot windows and recapture rules.
- Upstream sync plus live recovery in one motion can mix root causes (code drift vs runtime deployment mismatch), making failure attribution harder.

## Phase B (Plan Review) — Round 1

VERDICT: REVISE

Findings:

1. High: The draft made health-watchdog parse-error clearance a stricter blocker than the spec requires. The spec requires `com.cass.index-watch` durability and priority-agent searchability; health-watchdog is a durability risk but should block only if it interferes with the required watcher proof.
2. High: The draft said to integrate `src/watchdog.rs` only if compatible, but did not define a complete fallback path when it is incompatible.
3. High: The draft deployed the runtime binary before the full verifier floor. Verification must gate deployment.
4. Medium: The draft captured launchd logs and search evidence without an explicit redact/sanitize checkpoint before committing artifacts.
5. Medium: The draft defined route policy but did not hard-gate later mutation tasks on the policy existing and being satisfied.

Reviewer sign-off details:

- Shape comparison gate passed.
- Plan Sanity Evidence gate passed.
- Source-verified assumptions: current CLI exposes `index --watch-once` but no `watchdog`; watch-once has Pi-specific behavior and limited path hinting; provenance uniqueness needs source-path reconciliation when `external_id` is null.

## Phase B (Plan Review) — Round 2

VERDICT: APPROVED

Reviewer result:

- Completeness: pass. The plan covers upstream ancestor/blocker proof, priority raw-to-DB reconciliation, three lexical probes per priority agent, live watcher proof, spec 015 routing, exact command evidence, and GoalBuddy completion gating.
- Correctness: pass. The runtime-first path is now gated by route-policy and verifier-before-deploy.
- Risks: residual lock contention, OOM quarantine, and corruption risks are documented.
- Missing steps: no blocking gaps found; incompatible `src/watchdog.rs` fallback is explicit.
- Security: acceptable for planning; evidence hygiene/redaction is explicit.
- R0 Shape Comparison gate: pass.
- Plan Sanity Evidence gate: pass.

## Phase B North-Star Check — Round 1

Outcome: HALT

The north-star checker found no banana drift, but flagged two plan-quality risks:

- Bloat risk: conditional `watchdog` CLI integration is valid only if watcher durability cannot be proven without it; if treated as default, it risks over-engineering relative to the user's main outcome.
- Ambiguity risk: branch-policy conflict handling said to stop if unauthorized branch blocks commit/push, but did not define the concrete resolution step.

Revision made:

- Plan/tasks now state the default path is direct `com.cass.index-watch` proof and `src/watchdog.rs` is not inspected or integrated on the happy path.
- Plan/tasks now define the final branch-resolution step: capture branch status, list intended staged files, and ask the user to authorize committing/pushing from the current branch or a non-destructive move back to `main`.

## Phase B North-Star Check — Round 2

Outcome: HALT

The checker again found no banana drift, but still flagged:

- Bloat risk: even conditional watchdog CLI wiring plus capabilities/robot-docs/golden churn could overtake the user's goal unless direct watcher proof first fails for a command-surface reason.
- Ambiguity risk: upstream divergence numbers were baseline snapshots and not explicitly timestamp/refresh anchored.

Revision made:

- Removed watchdog CLI wiring from this recovery path. Direct `com.cass.index-watch` proof is the repair route; a separate command-surface gap becomes a concrete blocker or follow-up issue unless the user explicitly authorizes separate watchdog work.
- Marked upstream/count numbers as baseline evidence only and made the T1 refresh the operative implementation state.

## Phase B North-Star Check — Round 3

Outcome: HALT

The checker again found no banana drift, but still flagged:

- Bloat risk: the evidence and watcher sections were too operationally dense for plan.md.
- Ambiguity risk: the health-watchdog blocker condition was still judgment-based.

Revision made:

- Compressed evidence and runtime details in plan.md, leaving command-heavy proof steps to tasks/route-policy.
- Made health-watchdog blocking testable: it blocks only when evidence shows it unloaded, killed, or otherwise prevented `com.cass.index-watch` from staying running or indexing the probe.

## Phase B North-Star Check — Round 4

Outcome: HALT

The checker found bloat resolved, but flagged:

- Potential under-delivery: a watcher command-surface blocker/follow-up path could miss the user's explicit working-watcher intent.
- Ambiguity: non-priority "all sessions" completion semantics were not measurable.

Revision made:

- A working `com.cass.index-watch` is now required; if direct reload/probe fails, this recovery fixes the smallest non-destructive cause instead of deferring it.
- Non-priority completion now requires full configured scope restored, no undocumented disabled agents, no count regression for OpenCode/factory/other connectors against the refreshed baseline, and no non-priority connector blocking priority indexing or watcher operation.

## Phase B North-Star Check — Round 5

Outcome: HALT

The checker flagged:

- Banana risk: OpenCode/factory/non-priority sessions had no explicit lexical proof bar.
- Bloat/ambiguity: lock/OOM/stall mechanics were too procedural and referenced thresholds that lived outside the plan.

Revision made:

- Added one safe lexical proof for OpenCode and factory when the refreshed baseline has indexed rows for those agents.
- Compressed lock/OOM/stall mechanics in plan.md and left numeric thresholds to `evidence/route-policy.md`, which must exist before mutation.

## Phase B North-Star Check — Round 6

Outcome: BOOTSTRAP

Final drift flags:

- BANANA: none
- BLOAT: none
- AMBIGUOUS: none

The checker confirmed the plan is aligned with the user's three source requests: live failure diagnosis, upstream sync plus priority-agent searchability plus watcher durability, and tracked GoalBuddy/spec ownership.

## Phase B (Plan Review) — Final Delta

VERDICT: APPROVED

Reviewer result:

- Completeness: pass.
- Correctness: pass.
- Security: acceptable for planning.
- Missing steps: no blocking gaps.
- Final deltas accepted: watchdog command work is opt-in only with user authorization, the working `com.cass.index-watch` outcome remains required, and non-priority connectors now have measurable no-regression plus OpenCode/factory lexical proof.

## Phase B — Boundary SHA: b807ef17 — Snapshot: boundary-B-b807ef17-b3c5942f.md
## Phase B — North-Star Check — Round 1 — BOOTSTRAP
