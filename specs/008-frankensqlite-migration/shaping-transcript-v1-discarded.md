---
shaping: true
---

<!-- shape:complete:v1 | harness: pi/claude-sonnet-4-6 | date: 2026-03-27T15:26:16Z -->

# Shaping Transcript — Spec 008

**Driver:** FastRaven (pi/claude-sonnet-4-6)
**Challenger:** NiceBear (crew-challenger)
**Date:** 2026-03-27
**Session:** frankensearch + FAD migration

---

## Round 1: Challenger reviews spec, finds four defects

**NiceBear verified:**

1. **rusqlite surface area is larger than spec implied.** Spec said "largely mechanical — swap import paths." Actual count: 63 rusqlite references across 12 files (src/bookmarks.rs, src/connectors/cursor.rs, src/indexer/mod.rs, src/lib.rs, src/pages/analytics.rs, src/pages/export.rs, src/pages/secret_scan.rs, src/pages/size.rs, src/search/query.rs, src/search/vector_index.rs, src/storage/sqlite.rs, src/watchdog.rs).

2. **Search layer is 14,775 lines**, not a thin adapter: query.rs alone is 7,720 lines.

3. **No `Arc<Mutex>` or reader pool in our storage.** Spec was describing upstream's architecture, not ours. Our actual storage: `struct SqliteStorage { conn: Connection }` — single connection, no concurrency wrapper.

4. **`daily_stats` already exists in our schema v8** — spec's migration table claiming v11 adds it was wrong.

---

## Round 2: Driver proposes requirements R0–R8

**Initial requirements proposed.** Key: R0 included frankentui as part of "full franken ecosystem." R8 (frankentui) marked Undecided.

---

## Round 3: Challenger challenges requirements

**Five challenges raised:**

1. **R0 contradicts spec's Out of Scope** — spec already excluded frankentui; R8 re-introduced it. frankentui = 41K-line Elm-architecture rewrite, verified by checking upstream app.rs. Evidence: our TUI is 11,782 lines (tui.rs 9,262 + components 2,520); upstream rewrote to 41K around frankentui. No relationship to storage/search migration. **R8 should be Out of scope.**

2. **R2 "equivalent results" is untestable** — needs concrete overlap metric (≥80% top-10 overlap).

3. **Duplicate fastembed/ort risk** — we have direct `fastembed` dep; frankensearch bundles its own. Two copies of ort could break build. Needs explicit dep deconfliction requirement.

4. **Missing rollback path requirement** — spec mentioned it in Constraints but no R captured it. Add R9 (rollback via VACUUM INTO snapshot).

5. **R5 needs verification mechanism** — `src/watchdog.rs` has rusqlite ref; "unaffected" needs concrete test (`cass watchdog run`).

**Driver accepted all five challenges.**

---

## Round 4: Driver proposes revised R0–R9 and four shapes

**Updated requirements:**

| ID | Requirement | Status |
|----|-------------|--------|
| R0 | Migrate backend to frankensqlite + frankensearch + updated FAD connectors. frankentui deferred. | Core goal |
| R1 | Live 8.8GB DB (schema v8, 24K+ convs) survives with zero data loss | Must-have |
| R2 | Top-10 results ≥80% overlap pre/post migration | Must-have |
| R3 | Semantic search works; no duplicate ort/fastembed runtime | Must-have |
| R4 | All deps as git deps at pinned revs | Must-have |
| R5 | Watcher, watchdog compile and pass `cass watchdog run` | Must-have |
| R6 | 12 P1 semantic beads closed/superseded | Must-have |
| R7 | Build on Rust nightly; no asupersync in our code | Must-have |
| R8 | Rollback path: VACUUM INTO snapshot before schema change | Must-have |
| R9 | frankensqlite: only included if concrete perf benefit demonstrated | Undecided |

**Four shapes proposed:**
- **A**: Big-bang (frankensearch + frankensqlite + FAD, one PR)
- **B**: Staged (frankensearch PR1, frankensqlite PR2)
- **C**: Dropped after round 2 analysis
- **D**: Frankensearch only (frankensqlite deferred indefinitely)

---

## Round 5: Challenger challenges shapes

**Four challenges on shapes:**

1. **All shapes assumed search layer is tantivy-only — it's a DUAL index.** Driver verified: query.rs uses both tantivy (primary, line 1972: "Tantivy is the primary high-performance engine") AND rusqlite FTS5 as fallback (`search_sqlite()` function, `sqlite: Option<Connection>` field, `fts_messages MATCH ?` queries). Shapes B and D that said "replace tantivy" also need to address the FTS5 fallback path. Driver correction: FTS5 path is a fallback (not co-primary), so frankensearch replacing tantivy means FTS5 path can be dropped entirely, making B and D MORE viable than challenger suggested.

2. **Shape A's "single migration runner pass" is wrong for our schema.** Our v8 already has `daily_stats`. Upstream's v9-v13 migrations assume their v8 baseline. We need a CUSTOM migration path, not a blind application of upstream's runner. Requires a spike.

3. **63 rusqlite refs across 12 files, not 8** — shapes undercount the scope.

4. **frankensqlite value prop challenge** — our storage is single-connection (`struct SqliteStorage { conn: Connection }`). No `Arc<Mutex>`. No reader pool. No concurrent writers. frankensqlite's headline feature doesn't help us. Concrete benefit = upstream alignment + pure Rust. Risk = untested reimplementation on 8.8GB. Shape D deserves stronger framing as the right default.

**Challenger also proposed Shape E** (keep FTS5, add frankensearch semantic tier). Driver rejected: creates THREE search tiers (tantivy + FTS5 + semantic) = worse complexity than two.

---

## Round 6: Driver verifies, proposes final shapes with corrections

**Driver verified:**
- `search_sqlite()` confirmed in query.rs — D2 removal target is concrete
- FTS5 write paths: 4 refs in indexer/mod.rs, 5 refs in storage/sqlite.rs — bounded scope
- cursor.rs opens Cursor's OWN SQLite DB (not ours) — isolated rusqlite consumer
- bookmarks.rs opens its own separate DB with own schema — isolated

**Shapes updated to include FTS5 write path removal (D3).**

---

## Round 7: Challenger raises independence question

**NiceBear's final challenge:** "Have you confirmed frankensearch can be consumed as a standalone git dep without frankensqlite? If frankensearch internally depends on frankensqlite, Shape D collapses."

**Driver verified:**
- `frankensearch-storage/Cargo.toml` has `fsqlite = "0.1.2"` — storage crate depends on frankensqlite
- BUT: `storage` is an OPTIONAL feature in the frankensearch facade crate
- Features `["hash", "lexical", "ann", "fastembed-reranker"]` do NOT include `storage`
- **Upstream's own Cargo.toml confirms this:** `frankensearch = { ..., features = ["hash", "lexical", "ann", "fastembed-reranker"] }` — no `storage` feature
- **Shape D is viable.** frankensearch is independently usable without frankensqlite.

**asupersync:** Direct dep of frankensearch facade (not feature-gated). Has `asupersync-tokio-compat` workspace member — designed to work alongside tokio, not replace it. Transitive dep is acceptable; we don't call asupersync APIs in our code.

---

## Final Fit Check

| Req | Requirement | Status | A | B | D |
|-----|-------------|--------|---|---|---|
| R0 | Replace tantivy+FTS5 with frankensearch; bump FAD for new connectors | Core goal | ✅ | ✅ | ✅ |
| R1 | 8.8GB DB, 24K+ conversations survive with zero data loss | Must-have | ❌ | ✅/⚠️ | ✅ |
| R2 | Top-10 results ≥80% overlap pre/post migration | Must-have | ✅ | ✅ | ✅ |
| R3 | Semantic search works; no duplicate ort/fastembed runtime | Must-have | ✅ | ✅ | ✅ |
| R4 | All deps as git deps at pinned revs | Must-have | ✅ | ✅ | ✅ |
| R5 | Watcher, watchdog compile and pass `cass watchdog run` | Must-have | ✅ | ✅ | ✅ |
| R6 | 12 P1 semantic beads closed/superseded | Must-have | ✅ | ✅ | ✅ |
| R7 | Build on Rust nightly; no asupersync in our code | Must-have | ✅ | ✅ | ✅ |
| R8 | Rollback path: VACUUM INTO snapshot before schema change | Must-have | ❌ | ✅ | ✅ |
| R9 | frankensqlite: only if concrete perf benefit demonstrated | Undecided | Assumes yes | Defers | ✅ Deferred |

**Notes:**
- A fails R1: MigrationRunner on diverged schema v8→v13 untested; no isolated rollback from big-bang
- A fails R8: big-bang makes rollback harder
- B ✅/⚠️ on R1: PR1 zero data risk; PR2 carries schema risk (isolated to its own PR)

---

## Selected Shape: D — Frankensearch + FAD, frankensqlite deferred

| Part | Mechanism |
|------|-----------|
| D1 | Replace tantivy dep with frankensearch `features = ["hash", "lexical", "ann", "fastembed-reranker"]` at pinned rev; rewrite query.rs to use frankensearch API — sub-tasks: (a) lexical, (b) semantic, (c) hybrid/RRF, (d) query parsing compat |
| D2 | Remove `search_sqlite()` function, `sqlite: Option<Connection>` field, and FTS5 read path from query.rs |
| D3 | Remove FTS5 write path from storage/sqlite.rs (`batch_insert_fts_messages`, `rebuild_fts`) and indexer/mod.rs; gate or remove `CREATE VIRTUAL TABLE fts_messages` from migration chain for fresh installs |
| D4 | Bump FAD git dep rev to pick up crush, kimi, copilot_cli, qwen connectors; remove direct fastembed dep |
| D5 | frankensqlite: separate spec required, with concrete perf justification (R9 = Deferred) |

**Rationale:** D satisfies R0-R8 with lowest risk. frankensearch is independently usable without frankensqlite (confirmed by upstream's own feature selection). frankensqlite deferred — our single-connection architecture has no demonstrated concurrent writer bottleneck. The real value of this migration is closing 12 P1 semantic beads and gaining a maintained search library; both achieved by D alone.

**Challenger approval:** NiceBear approved Shape D after verifying D2/D3 removal targets are concrete and bounded.

---

## Implementation Notes (from NiceBear, post-approval)

1. D3 should also drop or gate `fts_messages` virtual table from migration chain — stop writing AND stop creating on fresh installs
2. D1 (query.rs 7,720-line rewrite) is the main risk — split into sub-tasks: (a) lexical, (b) semantic, (c) hybrid/RRF, (d) query parsing compat layer

---

## Scope Clarification: frankentui

frankentui is explicitly OUT of scope for this spec. User said "all franken items" but:
- frankentui = 41K-line Elm-architecture TUI rewrite
- Zero relationship to storage/search migration
- Can be done independently in any order
- Warrants its own spec

A separate `/issue` should be created for frankentui migration when ready.
