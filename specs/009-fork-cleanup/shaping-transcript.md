---
shaping: true
---

<!-- shape:complete:v1 | harness: pi/claude-sonnet-4-6 | date: 2026-03-29T11:46:00Z -->

# Shaping Transcript — Spec 009

**Driver:** FastRaven (pi/claude-sonnet-4-6)
**Challenger:** BrightYak (crew-challenger, claude-opus-4-6)
**Date:** 2026-03-29

---

## Round 1: Challenger verifies facts, driver proposes R0–R7

**BrightYak verified independently:**
- feat/008-upstream-sync has 10 files differing from upstream/main (7 src/ + Cargo.lock, Cargo.toml, rust-toolchain.toml)
- `doctor.rs` exists with DoctorConnector + ConnectorExt shim
- FAD pinned at tag v0.1.3 (not yet bumped)
- ConnectorExt actively used in indexer/mod.rs (import line 37, call sites 1290+1342, test struct impls 5666/5704/5740)
- frankensqlite exists as git dep at rev 92a9a0fa
- No [patch] section yet in Cargo.toml

**Initial requirements proposed (R0–R7):**
R0: Remove codebuff
R1: Remove ConnectorExt shim from doctor.rs
R2: Bump FAD to main (de450843) for native scan_with_callback
R3: Restore crush.rs
R4: Add Cargo [patch] for FAD's frankensqlite path dep
R5: cargo test 0 failures
R6: Keep watchdog/SIGTERM/heartbeat
R7: DoctorConnector — undecided

---

## Round 2: Challenger raises six challenges (all accepted)

**C1: R1+R2 are atomically coupled — can't remove shim without bumping FAD, and vice versa.**
Evidence: indexer/mod.rs line 37 imports ConnectorExt; upstream line 1286 calls conn.scan_with_callback() directly.
Resolution: Explicit coupling constraint added; mandated as one commit.

**C2: R1 scope severely undercounted — it's 8 migration sites, not "remove a shim."**
Evidence: grep showed 1 import + 2 call sites + 3 test struct impl blocks in indexer/mod.rs.
Each call site changes from `connector_scan_with_callback(&*conn, ...)` to `conn.scan_with_callback(...)`.
Each test struct impl must move from separate `impl ConnectorExt` blocks into `impl Connector` blocks.
Resolution: R1 reframed as "Migrate from ConnectorExt shim to native Connector::scan_with_callback" with 8 sites enumerated.

**C3: [patch] mechanism for out-of-repo path dep — untested, needed a spike.**
Evidence: FAD's frankensqlite is `path = "../frankensqlite/crates/fsqlite"` — outside FAD's repo root. When Cargo fetches FAD as a git dep, the `../` path doesn't exist. [patch] for this edge case was unconfirmed.
Resolution: Driver ran spike immediately. Result: **[patch] works.** `cargo check` passes cleanly after adding:
```toml
[patch."https://github.com/Dicklesworthstone/franken_agent_detection"]
frankensqlite = { git = "https://github.com/Dicklesworthstone/frankensqlite", rev = "92a9a0fa", package = "fsqlite" }
```
C3 closed — not a risk.

**C4: R5 conflates two independent failure classes.**
Evidence: 55 failing tests = ~30 streaming dispatch (scan_with_callback not on Connector trait) + ~25 analytics/indexer (newly broken after v0.2.5 merge, unrelated to our changes).
Making "0 failures" block merge holds spec 009 hostage to unrelated upstream bugs.
Resolution: R5 split into R4 (streaming tests must pass after FAD bump) + best-effort for analytics (root-cause, file bead, add #[ignore] if upstream bug).

**C5: R7 should be DELETE, not undecided.**
Evidence: DoctorConnector has exactly 1 implementation (CodebuffConnector, being deleted). After R0, 0 implementations, 0 callers. Keeping empty trait is textbook YAGNI.
Resolution: R7 = DELETE doctor.rs entirely.

**C6: Missing requirement — pub mod doctor removal from lib.rs.**
Evidence: lib.rs line 19 has `pub mod doctor;`. Deleting doctor.rs without removing this breaks cargo check.
Resolution: Added as explicit acceptance criterion (R7/R-new).

---

## Final Requirements (approved)

| ID | Requirement | Status |
|----|-------------|--------|
| R0 | Remove codebuff: connector file, pub mod in connectors/mod.rs, import/factory/AgentKind in indexer/mod.rs, reconciliation block + pub mod in lib.rs | Core goal |
| R1 | Migrate ConnectorExt shim → native Connector::scan_with_callback in indexer/mod.rs: 1 import, 2 call sites, 3 test struct impl blocks | Core goal |
| R2 | Bump FAD to rev=de450843 (main) with crush feature + [patch] section for frankensqlite | Must-have |
| R3 | Restore crush.rs from upstream | Must-have |
| R4 | Streaming dispatch tests (~30) pass after FAD bump. Analytics failures (~25) root-caused; upstream bugs get bead + #[ignore] | Must-have |
| R5 | Keep watchdog.rs, SIGTERM/heartbeat in indexer, watchdog wiring in lib.rs | Must-have |
| R6 | Delete doctor.rs entirely | Must-have |
| R7 | Remove pub mod doctor from lib.rs | Must-have |

R1+R2 are atomically coupled — one commit.

---

## Fit Check

| Req | A (single atomic commit) | B (two commits) | C (three commits) |
|-----|--------------------------|-----------------|-------------------|
| R0 | ✅ | ✅ | ✅ |
| R1+R2 atomic | ✅ | ✅ | ❌ — Cargo.toml only commit leaves build in broken state |
| R3 | ✅ | ✅ | ✅ |
| R4 streaming tests | ✅ | ✅ | ✅ |
| R5 keep watchdog | ✅ | ✅ | ✅ |
| R6+R7 delete doctor | ✅ | ✅ | ✅ |
| Clean bisectable history | ✅ intermediate states are not meaningful | ⚠️ commit 1 has dead code | ❌ commit 1 is broken |

Notes:
- B fails because commit 1 (FAD + indexer migration) leaves codebuff/doctor as live dead code
- C fails R1+R2 atomicity: Cargo.toml-only commit 1 leaves indexer calling non-existent ConnectorExt shim

---

## Selected Shape: A — Single atomic commit

**Rationale:** This is one logical operation. Intermediate states in B and C are either broken or meaningless. Diff is bounded (~640 lines removed, ~35 added/modified — 90% deletions).

**Implementation order within the commit** (verify in stages, commit atomically):
1. Cargo.toml: FAD rev bump, crush feature, [patch] section
2. connectors/mod.rs: swap crush for codebuff
3. crush.rs: restore from upstream
4. codebuff.rs: delete
5. indexer/mod.rs: migrate ConnectorExt (import + 2 call sites + 3 test impls → impl Connector blocks)
6. ▶ Run `cargo test` here — confirm ~30 streaming tests now pass before proceeding
7. doctor.rs: delete
8. lib.rs: remove pub mod doctor, remove codebuff reconciliation block (~67 lines)

**Implementation notes from BrightYak:**
- Note 1: Run `cargo test` after step 6 (before step 7–8) to confirm streaming tests fixed. If FAD de450843 has API surprises, catch them before codebuff/doctor cleanup.
- Note 2: CrushConnector is NOT wired into indexer scan loop in upstream — restoring crush.rs matches upstream structure but the connector is dormant on both sides. Do NOT wire it into scan loop as part of spec 009.
