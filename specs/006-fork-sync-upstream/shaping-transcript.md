# Shaping Transcript: 006-fork-sync-upstream

**Date:** 2026-03-15
**Participants:** PureYak (proposer, pi/claude-sonnet-4-20250514), YoungYak (challenger, crew-challenger)
**Spec:** specs/006-fork-sync-upstream/spec.md
**Bead:** coding_agent_session_search-3fir

---

## Phase: Research (PureYak)

Initial spec described the problem as "fork main is 51 commits behind upstream." Deep research revealed the problem is fundamentally different:

1. **Our working branch diverged from upstream in December 2025** (commit 8e97d77a). It has 489 unique commits (though ~332 are AI agent churn — the real functional work is ~15-20 commits across specs 001-005).
2. **Upstream moved ALL connectors to an external crate** (`franken_agent_detection`). Every connector file is now a 3-5 line re-export stub. Our 14,329 lines of in-tree connector code doesn't exist on upstream anymore.
3. **Fork main is not our working codebase.** Our dev branch carries 489 commits never merged to main. Fork main tracks upstream but is unused.
4. **The running binary** is built from our dev branch, not fork main or upstream.

---

## Phase: Review (YoungYak)

Verified all claims. Corrected the 489-commit characterization:
- 332 (68%) are AI agent churn (version bumps, beads sync, docs)
- 106 (21%) are from upstream (merged Jan 2026)
- 51 (10%) are Dale's actual work
- Real unique functional work: ~15-20 commits

---

## Phase: Challenge Round 1 (YoungYak — 5 issues)

### Challenge 1: Missing Shape D — Hybrid Port
Add `franken-agent-detection` to our Cargo.toml as a dependency. Use FAD connectors for everything EXCEPT pi_agent (keep our superior implementation). Get new connectors without touching indexer/storage/TUI.

### Challenge 2: Shape A is catastrophically undercosted
Not "zero effort" — it's accepting permanent solo maintenance of 63K lines. Upstream replaced rusqlite, tantivy, AND ratatui with custom forks. Every future upstream fix targets franken crates we don't use.

### Challenge 3: FAD does NOT have our fixes
Checked actual FAD source:
- No `is_pi_path` exact home match (still uses substring heuristic)
- No `max_depth` on WalkDir (unbounded)
- No clawdbot-compatible `.jsonl` acceptance
- No `count_disk_files()` or `reconciliation_notes()` on trait
- Watcher fixes (SIGTERM, heartbeat, named threads) are indexer-level, N/A

### Challenge 4: Shape B is mis-framed
Not a cherry-pick — it's a re-architecture against FAD's 2-method trait (vs our 4-method), different path matching, different detection mechanism.

### Challenge 5: Requirements missing
Need to prioritize R1 (connectors), R2 (frankensqlite), R5 (time budget) before choosing.

---

## Phase: Revision (PureYak)

All five challenges accepted:
1. Shape D added as fourth option
2. Shape A re-costed as "deferred effort that compounds"
3. FAD gap confirmed as critical constraint
4. Shape B relabeled as "full re-architecture"
5. Requirements proposed with prioritization based on user signals

Shape E (extract our fixes as a crate) also proposed.

---

## Phase: Challenge Round 2 (YoungYak — 3 issues)

### Challenge 1: Shape D type collision
Both our codebase and FAD define the same types (`NormalizedConversation`, `Connector` trait, etc.) — structurally identical but different Rust types. Need adapter pattern to bridge: ~2-3 hours for 7-8 connectors.

### Challenge 2: Shape E is a distraction — drop it
Creates three connector implementations (in-tree, FAD, our crate). Nobody else consumes our crate. It's Shape A with packaging overhead.

### Challenge 3: R1/R2/R5 need resolution
Based on user signals:
- R1 (new connectors): Nice-to-have
- R2 (FrankenSQLite): Nice-to-have (upstream indexer doesn't use concurrent writers yet)
- R5 (≤8 hours): Leaning yes

---

## Phase: Final Revision (PureYak)

Shape E dropped. Requirements prioritized. Fit check formalized:

### Requirements (R)

| ID | Requirement | Status |
|----|-------------|--------|
| R0 | Watcher fix stays deployed and working | Core goal |
| R1 | New upstream connectors available | Nice-to-have |
| R2 | FrankenSQLite concurrent writers accessible | Nice-to-have |
| R3 | Maintenance burden doesn't grow indefinitely | Must-have |
| R4 | No regression in existing functionality | Must-have |
| R5 | Completable in ≤8 hours | Leaning yes |

### Fit Check

| Req | Requirement | Status | A | B | D |
|-----|-------------|--------|---|---|---|
| R0 | Watcher fix stays deployed and working | Core goal | ✅ | ❌ | ✅ |
| R1 | New upstream connectors available | Nice-to-have | ❌ | ✅ | ✅ |
| R2 | FrankenSQLite concurrent writers accessible | Nice-to-have | ❌ | ✅ | ❌ |
| R3 | Maintenance burden doesn't grow indefinitely | Must-have | ❌ | ✅ | ❌ |
| R4 | No regression in existing functionality | Must-have | ✅ | ❌ | ✅ |
| R5 | Completable in ≤8 hours | Leaning yes | ✅ | ❌ | ✅ |

Notes:
- B fails R0/R4 because R5 constrains the time to do it safely — B is deferred due to time, not architecture. With unlimited time, B passes everything.
- A fails R3 hard: all 63K lines diverge. D fails R3 soft: only ~40K lines diverge (connector divergence stops via FAD). D halves the divergence growth rate.
- No shape passes all must-have requirements. R3 fails for both A and D; only B passes R3 but fails R0, R4, R5.
- Shape C (contribute upstream) is the correct long-term R3 answer but requires upstream cooperation and is not session-sized work.

---

## Phase: Agreement

YoungYak confirmed with two nuances:
1. B is deferred due to time (R5), not architectural unsoundness — document this so B becomes viable when time allows
2. D's R3 failure is qualitatively different from A's — D halves the growth rate

### Shape selected: D (Hybrid Port)

Add `franken-agent-detection` as dependency. Use FAD connectors for everything except pi_agent. Adapter pattern bridges type differences (~2-3 hours). Keep our watcher fix, indexer, and storage as-is.

**Sequencing:** D now (pragmatic, session-sized), C later (contribute upstream for long-term sustainability).
