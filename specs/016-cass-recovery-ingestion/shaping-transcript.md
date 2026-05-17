<!-- shape:complete:v1 | harness: unknown | date: 2026-05-16T12:53:41Z -->

# Shaping Transcript: cass session ingestion recovery

Review ID: `c3c97350`
Challenger session: `019e30c2-a5aa-7003-8e40-c56e99670907`
Command: `$codex-shape specs/016-cass-recovery-ingestion/`

## Inputs

- User goal: be in sync with upstream, process sessions into searchable cass, prioritize Pi Agent / Claude Code / Codex, keep OpenCode/factory/others as bonus, and set up a watcher.
- Spec: `specs/016-cass-recovery-ingestion/spec.md`
- Existing partial lane: `specs/015-watch-once-streaming-scan/`
- Goal board: `docs/goals/cass-session-ingestion-recovery/state.yaml`
- Live evidence: git upstream/origin state, `cass health --json`, `cass stats --json`, raw priority-agent source counts, and launchd cass job list.

## Phase A: Requirements

Settled requirements after challenge:

| Req | Summary |
|-----|---------|
| R0 | Completion is one current live proof packet from a frozen pre-mutation evidence snapshot. |
| R1 | Upstream sync is a hard pass/fail gate with ancestor and no-net-reversion proof. |
| R2 | Priority-agent identity math must come from a frozen corpus manifest. |
| R3 | Priority-agent historical coverage must reach at least 95% of parseable discovered identities, with every missing identity accounted. |
| R4 | Identity integrity and idempotency must include duplicate checks, source mapping, newest/oldest/random samples, and safe repeat-run proof. |
| R5 | Fresh lexical retrieval must be proven from ground-truth source strings; semantic is optional. |
| R6 | Watcher proof must cover same binary, launchctl reload, incremental path, 120-second search probe, and delayed durability. |
| R7 | Failure handling must cover stale index, missing watcher, lock/busy, Pi OOM/stall, and WAL/FTS corruption. |
| R8 | Evidence must be repeatable and scoped, with exact commands and repo verification if code changed. |
| R9 | Spec 015 must be closed, superseded, or routed as subordinate evidence. |

Challenge findings incorporated:

- Define a hard upstream gate instead of "compare and maybe incorporate."
- Use identity denominators, not raw count vibes.
- Tie freshness to source identity and final ingest window.
- Require watcher durability, not just plist/process existence.
- Require same-binary proof and full-index/watcher-path parity.
- Route spec 015 explicitly so it cannot remain the apparent owner.

## Phase B: Candidate Shapes

| Shape | Decision |
|-------|----------|
| A: Finish Spec 015 First | Rejected. Repeats the Pi-centric partial lane and delays upstream, Claude/Codex proof, and watcher durability. |
| B: Upstream-First Normal Recovery | Rejected. Health/stats/search surfaces alone cannot prove frozen identity coverage, integrity, or idempotency. |
| C: Manifest-Led Priority Recovery | Viable but weaker than E because it does not explicitly isolate priority load. |
| D: Watcher-First Forward Capture | Rejected. Defers historical recovery, which is the core user outcome. |
| E: Priority-Scoped Manifest Recovery | Selected. Passes requirements with the smallest operational blast radius. |
| F: Shadow-Data-Dir Cutover Recovery | Fallback only if live in-place mutation is unsafe. Mainline use adds cutover risk. |
| G: Root-Cause-First Canary Replay | Useful as an E sub-slice if blockers recur, but final proof still collapses into E. |

Challenge findings incorporated:

- A collapses into a failed repeat of spec 015 under product acceptance.
- B and C need manifest-level identity proof; B alone is a proxy.
- E was added/strengthened as the priority-scoped route.
- F and G remain fallback/sub-slice routes, not the main shape.

## Phase C: Fit Check And Breadboard

Fit result: Shape E passes R0-R9; C, F, and G are viable in limited ways, while A, B, and D fail core acceptance.

Breadboard affordances:

- Frozen priority corpus manifest.
- Upstream incorporation gate.
- Same-binary deployment proof.
- Priority-scoped ingestion driver.
- Reversible scope-toggle contract.
- Route policy artifact with thresholds, stop conditions, fallback routes, and re-entry criteria.
- Identity/integrity/idempotency verifier.
- Fresh lexical retrieval verifier.
- Watcher reload and durability proof.
- Spec ownership closeout.

Challenge findings incorporated:

- Priority scoping must be reversible and must not become a permanent disabled-connector state.
- Route policy needed exact retry/stop/re-entry semantics.
- One vertical searchable identity proof must happen before broad recovery.
- Safe unique-string probes must avoid secrets and common text.

## Phase D: Terminal Approval

First terminal review returned `VERDICT: REVISE`.

Minimum required fixes:

- Carry the exact R4 sampling floor into the final shape: at least 10 identities per priority agent, or all if fewer.
- Carry the R8 code-change verification floor into the final shape.
- Promote exact route thresholds before live mutation from a planning note to a mandatory gate.

Those fixes were applied to the final shape.

Second terminal review:

> Selected shape is ready to become the spec shaping decision.
> R0-R9 check: no blocking mismatch remains.
> Breadboard risk check: no load-bearing hand-wave remains that would force `/codex-plan` into the wrong work.
> Follow-up masking check: no blocker is being improperly deferred.
>
> VERDICT: APPROVED

## Final Selected Shape

**Priority-Scoped Manifest Recovery**.

The plan must freeze the priority corpus, incorporate upstream, prove the same binary, use reversible priority scoping, prove one vertical searchable identity before broad recovery, run broad priority historical recovery under exact route thresholds, verify identity/integrity/idempotency/search, restore full configured scope, prove watcher durability, and route spec 015.

## Temp Artifacts

The live shaping run used:

- `/tmp/codex-shape-r-c3c97350.md`
- `/tmp/codex-shape-s-c3c97350.md`
- `/tmp/codex-shape-fitcheck-c3c97350.md`
- `/tmp/codex-shape-final-c3c97350.md`
- `/tmp/codex-shape-review-c3c97350.md`

These were intentionally left in place because this repo forbids file deletion without explicit permission.
