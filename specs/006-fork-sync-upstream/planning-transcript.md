# Planning Transcript: 006-fork-sync-upstream

**Date:** 2026-03-15
**Participants:** PureYak (proposer, pi/claude-sonnet-4-20250514), SwiftKnight (challenger, crew-challenger)
**Spec:** specs/006-fork-sync-upstream/spec.md
**Bead:** coding_agent_session_search-3fir

---

## Phase: Research (PureYak)

Deep codebase research revealed:

1. FAD types (NormalizedConversation, NormalizedMessage, etc.) are field-identical to ours
2. FAD Connector trait has 2 methods; ours has 4. Adapter needed for `count_disk_files()` and `reconciliation_notes()`
3. Registration is centralized in `get_connector_factories()` + `ConnectorKind` enum
4. FAD rev mismatch: upstream pins 5b0eb1a, we had cached 6b1f044

PureYak initially proposed copying connector files instead of depending on FAD ("skip adapter pattern entirely").

---

## Phase: Challenge (SwiftKnight — 3 issues)

### 🔴 SHOWSTOPPER: Wrong connector names
The spec named copilot_cli, kimi, qwen as target connectors. These don't exist in FAD at the pinned rev. The actual new connectors available: copilot, clawdbot, openclaw, vibe. All 4 verified present at rev 5b0eb1a.

### 🟡 Copy approach misses franken_detection_for_connector()
FAD connectors use internal helpers not available in our codebase. Copying would require rewriting detect() logic for each connector.

### 🟢 FAD dep chain is clean
Confirmed: FAD with `features = ["connectors"]` pulls only anyhow, serde_json, walkdir, tracing, dotenvy, bloomfilter, once_cell. No frankensqlite, no frankensearch, no frankentui.

---

## Phase: Revision (PureYak)

PureYak claimed we already had copilot, clawdbot, openclaw, vibe — proposing Shape A (stay diverged) since "no new connectors to gain."

---

## Phase: Challenge Round 2 (SwiftKnight)

### Factual correction: we do NOT have those connectors
Verified: none of copilot.rs, clawdbot.rs, openclaw.rs, vibe.rs exist on our branch. They exist only on upstream/main as FAD re-export stubs. PureYak confused upstream's files with our branch's files.

Shape D's R1 value is real — we'd gain 4 genuinely new connectors (2,585 lines of logic for agents we cannot index today).

---

## Phase: Agreement

Shape D confirmed with corrected targets:
1. Target connectors: copilot, clawdbot, openclaw, vibe (NOT copilot_cli, kimi, qwen)
2. Approach: depend on FAD (not copy), adapter pattern
3. FAD dep chain clean, pin to rev 5b0eb1a (verified)
4. ~180 lines of new code, 1 new file
5. Estimated 3-4 hours including verification
