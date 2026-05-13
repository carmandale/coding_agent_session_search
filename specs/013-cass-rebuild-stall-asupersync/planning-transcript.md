<!-- plan:complete:v1 | harness: unknown | date: 2026-05-13T15:22:07Z -->

# Planning transcript — specs/013-cass-rebuild-stall-asupersync/

REVIEW_ID: 6f61efe1
CODEX_SESSION_ID: 019e21ca-fccf-7ec1-807c-54cccf4d33ee

## Phase A (Research Challenge) — Round 1

Codex Round 1 raised 8 challenges:
1. `--full --force-rebuild` may not backfill against a non-empty DB — canonical-only path at src/indexer/mod.rs:9675 / 10088 skips raw source rescan
2. "drained but not closed" hypothesis not supported by current channel ownership (parent tx dropped at 8224, batch sender borrows at 9140, producers send Done at 8585)
3. Proposed panic-close regression overlaps existing coverage at src/indexer/mod.rs:27750
4. Raw-mirror also runs on consumer path via persist_conversations_batched_with_raw_mirror_links() at 18659, sharing MANIFEST_UPDATE_LOCK
5. "25 GB raw mirror is reused" overstated; copy + hash runs before dedup check (raw_mirror.rs:141 / 415)
6. "Never started" is a log inference error — streaming_scan_complete fires at scan END (src/indexer/mod.rs:8576)
7. Watchdog payload missing R4 fields; gated to structured-output mode only
8. Spec naming "asupersync deadlock" contradicts research finding (asupersync is red herring)

Codex did not issue VERDICT in Phase A.

## Phase A (Research Challenge) — Round 2

Author rewrote /tmp/codex-plan-research-6f61efe1.md addressing all 8 challenges. Codex Round 2 stopped on negative-contract violation: "instrumentation first, root-cause ... second, fix third, regression test fourth" was sequencing; "Before fixing anything, add..." was sequencing; "only after the diagnosis thread converges" was ordering.

## Phase A (Research Challenge) — Round 3

Author rewrote the six-surface section to be flat (no first/second/etc enumeration). Codex Round 3 still found two surviving sequencing phrases: "Replacing this with a liveness-instrumentation-first approach" (line 12) and "instrumentation is a precondition" (line 96).

## Phase A (Research Challenge) — Round 4

Author rewrote those two phrases and split "Emission gating is the second problem / Escalation semantics is the third problem" into a flat three-concern bullet list. Codex Round 4 confirmed negative contract clean and provided substantive challenge:

1. A2 acceptance criterion conflict (spec says --full --force-rebuild must complete; research correctly says use plain --full)
2. R3 caveat unsurfaced: cass index --full will fight the live watcher for the index-run lock
3. O2 reconciliation should be sharper — the 1,648 conv WITH streaming_ingest events prove streaming did run
4. R4 "thread states" not satisfied by logical pipeline state alone; need OS-level capture
5. Faster diagnostic path: existing knobs (CASS_STREAMING_CONSUMER_COMBINE=0, CASS_STREAMING_INDEX=0, commit cadence) before new instrumentation
6. Missing fix candidates: reservation-before-send at 8293, flat-combine at 8624
7. Upstream commit 544402b9 (staged-merge zero-clamp) underweighted; matches shape of post-scan rebuild deadlock candidate
8. C4 launchd reconciliation: actual plist is boolean KeepAlive=true, not dictionary form; "exit code launchd treats as terminal" not currently available — sentinel-file/wrapper is the design

Author incorporated all 8 substantive points into the research file. Phase A approach feels settled; proceeded to Phase B.

## Phase B (Plan Draft Review) — Round 1

Author drafted /tmp/codex-plan-draft-6f61efe1.md with combined Spec + Plan + Tasks. Six surfaces from Phase A mapped to seven task groups (A through G). VERDICT: REVISE with six findings:

1. Plan Sanity Evidence — riskiest assumption doesn't match probe (probe tests recovery command; riskiest assumption claimed deadlock-is-in-cass)
2. A2 not actually resolved — plan promises in-scope defect fix but no task implements it
3. R4 thread states still under-specified
4. Recovery data safety incomplete (T3 only snapshots .db, not -wal/-shm)
5. Watch-state claim false (plan says cass index --full updates watch_state; code shows it updates DB last_scan_ts at 8870, watch_state.json only at 16013)
6. Wrapper/sentinel scope needs hardening (no uninstall path per §2.9)

## Phase B (Plan Draft Review) — Round 2

Author revised: (1) rewrote sanity evidence to align probe with riskiest assumption, (2) made --force-rebuild defect explicitly OUT of scope + matched in T1, (3) added two-layer R4 interpretation (logical default + OS-level opt-in via CASS_INDEX_STALL_CAPTURE_LLDB), (4) WAL-safe snapshot in T3 via PRAGMA wal_checkpoint(TRUNCATE) + clone-copy triplet, (5) added new Group H T35 for watch_state reconciliation, (6) sentinel lifecycle (T23), wrapper tests (T25), uninstaller (T27).

VERDICT: REVISE with five findings:
1. A2 still contradictory — spec text in draft says "in scope as defect" while T1 says out of scope
2. T35 not implementable — DB stores ONE global last_scan_ts (sqlite.rs:6244), not per-connector
3. A5 baseline verification uses relative-rev `dac/main^`, not exact SHA `29c3672a`
4. R4 thread-state semantics slippery; needs explicit spec amendment
5. Security/privacy under-covered (file modes, redaction for upstream sharing)

## Phase B (Plan Draft Review) — Round 3

Author revised: (1) made A2 spec-text and T1 consistent (both say short-circuit is OUT of scope), (2) rewrote T35 to use global last_scan_ts + delete-and-rederive alternative, (3) T28 now uses `git worktree add /tmp/cass-baseline-29c3672a 29c3672a` exact-SHA verification, (4) T8 made the two-layer R4 interpretation explicit + T1 amends spec.md to clarify, (5) added T27a covering file modes (0600/0700), redaction script, operator runbook.

VERDICT: APPROVED. Codex's verification:
- A2 consistently names cass index --full; short-circuit consistently out of scope
- Recovery command sanity evidence matches code at 9675 + 10234
- Watch-state reconciliation matches actual storage semantics at sqlite.rs:6244 + indexer/mod.rs:16138
- Baseline verification targets exact 29c3672a
- Security/privacy and uninstall coverage explicit
- R0 Shape Comparison gate passes
- Plan Sanity Evidence gate passes (after final rewrite to remove the word "None" which triggered PLACEHOLDER_RE)
- Non-blocking caution: T35's `cass --version` parse check is weak evidence; T36/T37 provide real runtime proof

## Phase B — Boundary SHA: 16d0a17d — Snapshot: boundary-B-16d0a17d-39ab06dc.md
## Phase B — North-Star Check — Round 1 — BOOTSTRAP
