---
title: "chore: Fork sync — hybrid port with FAD connectors"
date: 2026-03-15
bead: coding_agent_session_search-3fir
shaping: true
---

# Fork Sync: Hybrid Port

## User Story

**As a user running cass from our fork,** I want access to new upstream
connectors (Copilot CLI, Kimi, Qwen) without rebasing 1,400+ commits or
risking my working watcher fix. I also want to slow the maintenance
divergence from upstream so this fork doesn't become unmaintainable.

## Problem

Our working codebase (`fix/watcher-cpu-spin` branch) diverged from upstream
in December 2025. It carries ~15-20 functional commits (pi_agent connector,
OpenCode rewrite, doctor reconciliation, watcher CPU fix) on top of a
codebase that's 1,400+ commits behind upstream.

Meanwhile, upstream made three architectural moves that widened the gap:
1. **Extracted all connectors** to `franken_agent_detection` (FAD) crate —
   every connector file is now a 3-5 line re-export stub
2. **Replaced rusqlite** with `frankensqlite` (concurrent-writer SQLite)
3. **Replaced ratatui/tantivy** with `frankentui`/`frankensearch`

Our fork has 14,329 lines of in-tree connector code (including the spec-005
watcher fix). Upstream has 134 lines of stubs. A full rebase is days of
work and risks breaking the deployed watcher fix.

## Constraints

- The spec-005 watcher fix is deployed and working — must not regress
- This is a side project — work must fit in ≤8 hours
- We don't push to upstream (fork only)
- The cass binary runs from our dev branch, not fork main

---

## Requirements (R)

| ID | Requirement | Status |
|----|-------------|--------|
| R0 | Watcher fix (spec 005) stays deployed and working | Core goal |
| R1 | New upstream connectors available (Copilot CLI, Kimi, Qwen) | Nice-to-have |
| R2 | FrankenSQLite concurrent writers accessible | Nice-to-have |
| R3 | Maintenance burden doesn't grow indefinitely | Must-have |
| R4 | No regression in existing functionality | Must-have |
| R5 | Completable in ≤8 hours | Leaning yes |

---

## A: Stay diverged (permanent fork)

Our branch is the product. Accept solo maintenance of 63K lines.

| Part | Mechanism |
|------|-----------|
| A1 | Continue building on fix/watcher-cpu-spin |
| A2 | Manually cherry-pick individual upstream fixes as needed |

---

## B: Full rebase onto upstream main

Re-implement our work against upstream's architecture (FAD, frankensqlite,
frankensearch, frankentui).

| Part | Mechanism | Flag |
|------|-----------|:----:|
| B1 | Fast-forward fork main to upstream main | |
| B2 | Re-implement spec-005 changes against FAD's pi_agent and upstream indexer | ⚠️ |
| B3 | Re-implement spec-004 doctor changes against FAD's 2-method Connector trait | ⚠️ |
| B4 | Resolve frankensqlite/frankensearch/frankentui API differences across ~1400 commits | ⚠️ |

**Note:** B is deferred due to time constraint (R5), not architectural
unsoundness. With unlimited time, B passes all requirements. It becomes
viable when a multi-day session can be allocated.

---

## D: Hybrid port — add FAD as dependency (selected)

Add `franken-agent-detection` to our Cargo.toml. Use FAD connectors for new
agents only. Keep our pi_agent (with watcher fix) and all existing
connectors in-tree.

| Part | Mechanism |
|------|-----------|
| D1 | Add `franken-agent-detection` git dependency to Cargo.toml |
| D2 | Write adapter layer: `From<fad::NormalizedConversation> for crate::NormalizedConversation` (or adapter structs implementing our 4-method Connector trait) |
| D3 | Register FAD connectors for agents we DON'T already have (Copilot CLI, Kimi, Qwen) |
| D4 | Keep all existing in-tree connectors unchanged (pi_agent, codex, claude, etc.) |
| D5 | Verify all existing tests pass with the new dependency |

**Type collision note:** Both our codebase and FAD define structurally
identical types (`NormalizedConversation`, `Connector` trait, etc.) but
they're different Rust types. The adapter pattern (D2) bridges this — wrap
each FAD connector in a struct implementing our 4-method trait. Estimated
~2-3 hours for the 3 new connectors.

---

## Fit Check

| Req | Requirement | Status | A | B | D |
|-----|-------------|--------|---|---|---|
| R0 | Watcher fix stays working | Core goal | ✅ | ❌ | ✅ |
| R1 | New connectors available | Nice-to-have | ❌ | ✅ | ✅ |
| R2 | FrankenSQLite accessible | Nice-to-have | ❌ | ✅ | ❌ |
| R3 | Maintenance burden bounded | Must-have | ❌ | ✅ | ❌ |
| R4 | No regression | Must-have | ✅ | ❌ | ✅ |
| R5 | ≤8 hours | Leaning yes | ✅ | ❌ | ✅ |

**Notes:**
- B fails R0/R4 because R5 constrains time for safe porting, not because
  B is architecturally wrong.
- A fails R3 hard: all 63K lines diverge, gap grows with every upstream commit.
- D fails R3 soft: only ~40K lines diverge (connector divergence stops via
  FAD). Halves the growth rate. Not solved, but meaningfully slowed.
- No shape passes all must-haves. R3 fails for A and D. B passes R3 but
  fails R0, R4, R5.
- Shape C (contribute upstream) is the correct long-term R3 answer but
  requires upstream maintainer cooperation and is not session-sized.

## Recommendation

**Shape D** is the pragmatic choice. It keeps the watcher fix safe (R0),
gets new connectors (R1), fits in a session (R5), and halves the
maintenance divergence rate (partial R3). FrankenSQLite (R2) is deferred —
upstream's indexer doesn't use concurrent writers yet anyway.

**Sequencing:** D now (pragmatic), B or C when time allows and
frankensqlite's indexer integration matures.

## Acceptance Criteria

- [x] Decision documented: Shape D selected
- [x] Napkin updated with fork ownership notes
- [ ] FAD dependency added to Cargo.toml
- [ ] Adapter layer for new connectors (Copilot CLI, Kimi, Qwen)
- [ ] All existing tests pass
- [ ] New connectors indexing sessions
- [ ] Release binary rebuilt and deployed
