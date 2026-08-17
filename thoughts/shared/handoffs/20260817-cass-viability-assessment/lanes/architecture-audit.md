# Lane: architecture-audit — is cass right-sized for searching ~37.5G of local session JSONL?

Date: 2026-08-17. Read-only lane; no cass invocations were needed (all evidence from source, git, Cargo metadata, and directory listings). PID 75534 untouched. No writes to production data.

## Bottom line

The job is: scan ~37.5G of local JSONL, normalize, index, search, browse. The mechanism is 393,912 lines of Rust across 161 files (≈218k non-test), 823 locked dependencies including a from-scratch pure-Rust SQLite reimplementation, a bespoke async runtime, a bespoke TUI framework, and a forked search engine — plus 77G of derived state (2x the corpus) storing the same text three times and indexing it twice. The component that most violates right-sizing (the pinned fsqlite engine) is precisely the one that took search down in production: stock SQLite runs the hanging GROUP BY in 77ms against the same 22G archive where fsqlite 0.1.5 burned 434 CPU-minutes without returning. There IS a sound small core, and the strongest evidence is cass's own record: the SQLite archive is stock-compatible and the code already contains a working stock-shaped FTS5 search path.

## 1. Inventory

### README claims (README.md, 3,079 lines / 144KB)

- "Unified, high-performance TUI to index and search your local coding agent history" across 20 agent harnesses (Codex, Claude Code, Gemini CLI, Cline, Cursor, ChatGPT, Aider, Copilot, ...).
- "Instant Search (Sub-60ms Latency)", search-as-you-type, edge n-gram prefix indexing "trading disk space for O(1) lookup speed" (README:211-216).
- Optional local semantic search (fastembed ONNX, 3 embedders, HNSW vector index).
- Beyond search: remote multi-machine sync over SSH/SFTP, encrypted HTML export, "cass pages" (encrypted static-site publishing incl. a 2,102-line Cloudflare deploy module), swarm coordination surfaces (`swarm status`, `work-packet`, `lint`, `dependency-drift`), a daemon, analytics with charts, a doctor/diag/triage surface, golden-file-pinned robot JSON API.
- Reality check measured by the coordinator today: `cass stats --json` ran >3.5 min to 5.2GB RSS without returning; a sibling probe search has run 4h48m+. The sub-60ms claim and the live behavior are describing different tools.

### Scale

- src/: 161 .rs files, **393,912 LOC** total. tests/: 225 files, **158,616 LOC** more.
- **≈44.7% of src is inline test code**: 176,076 lines inside `#[cfg(test)]` blocks (brace-count heuristic; ±small % noise from braces in strings), 5,156 inline `#[test]` fns. Non-test src ≈ **217,836 LOC**.
- Repo age: first commit **2025-11-20** → 2026-08-17 (~9 months), 4,261 commits. That is ~1,450 src LOC/day sustained.

### Top 15 files (total LOC)

| file | LOC | note |
|---|---|---|
| src/lib.rs | 92,719 | god-file: full CLI (28 top-level subcommands + 10 sub-enums) + every `run_*` handler + 439 inline tests; 52,007 non-test |
| src/ui/app.rs | 47,458 | TUI app; 1,022 inline test fns across 28 cfg(test) blocks |
| src/indexer/mod.rs | 46,654 | indexing pipeline, single file |
| src/storage/sqlite.rs | 26,207 | storage layer, single file |
| src/search/query.rs | 20,872 | query engine, single file |
| src/analytics/query.rs | 7,265 | analytics |
| src/ui/style_system.rs | 4,806 | TUI styling |
| src/indexer/semantic.rs | 4,301 | embeddings |
| src/search/asset_state.rs | 4,094 | derived-asset state machine |
| src/indexer/lexical_generation.rs | 4,066 | tantivy generation mgmt |
| src/sources/sync.rs | 3,913 | remote SSH/SFTP sync |
| src/ui/analytics_charts.rs | 3,710 | TUI charts |
| src/search/model_download.rs | 3,414 | model fetching |
| src/search/pack_planner.rs | 3,385 | context-pack planner |
| src/pages/verify.rs | 3,167 | pages publishing verification |

## 2. Core vs periphery (non-test LOC, ≈217.8k total)

Core = scan JSONL → normalize → index → store → search → TUI.

| bucket | non-test LOC | share |
|---|---|---|
| **Core pipeline** (connectors 1,079 + indexer 32,492 + storage 14,109 + search 23,805 + model 1,606 + ui 33,595 + main 289) | **106,975** | 49% |
| **lib.rs** (mixed: CLI + handlers for both core and peripheral commands) | **52,007** | 24% |
| **Periphery** | **58,854** | 27% |

Periphery breakdown (non-test): pages 21,251 (encrypted static-site publishing: wizard, key management, Cloudflare deploy, bundle, verify), sources 9,533 (multi-machine remote sync), html_export 5,999, analytics 5,656, daemon 2,778, doctor family 2,655 (doctor + runs + chokepoint + undo + robot_docs), raw_mirror 1,797, perf_evidence 1,229, update_check 920, dependency_drift 798, bakeoff 769, evidence_bundle 681, crash_replay 678, topology_budget 623, swarm_status 487, tui_asciicast 473, export 401, bookmarks 398, query_cost_planner 339, ftui_harness 313, explainability 309, bin 278, policy_registry 276, encryption 213.

Several peripheral modules describe themselves as speculative or process-serving (their own `//!` docs):
- swarm_status.rs — "Fixtureable source adapters for the **planned** `cass swarm status` surface."
- explainability.rs — "Layered explanation cards for robot-visible controller decisions."
- policy_registry.rs — "Data-only registry for runtime controller policies."
- topology_budget.rs — "Topology-aware advisory budgets for large indexing hosts."
- crash_replay.rs — "Deterministic crash/replay harness for state-machine proof tests."
- perf_evidence.rs — "Stable evidence records for performance experiments and control-plane decisions."

And the "core" half is itself inflated: indexer carries responsiveness.rs (2,435 — the stall detector that per the napkin fires 4x during a healthy 26.8-min run), refresh_ledger.rs (2,451), lexical_generation.rs (4,066); search carries pack_planner (3,385), model_download (3,414), asset_state (4,094), semantic_manifest (2,299); ui carries style_system (4,806) and analytics_charts (3,710). Connectors are genuinely small in-repo (1,079) because format detection/parsing lives in the external `franken-agent-detection` crate.

## 3. Storage design: the same text lives three times and is indexed twice

Measured production data dir = 77G against a 37.5G corpus. From source:

1. **raw-mirror/ 46G** (`src/raw_mirror.rs`, 2,850 LOC): a blake3 content-addressed blob store (`raw-mirror/v1/{blobs,manifests,tmp}`, manifest kind `cass_raw_session_mirror_v1`). `capture_discovered_source_file_before_parse` (src/indexer/mod.rs:22246-22290, call sites 22265/22425/22468 + lib.rs:36295) copies **every discovered source file into the mirror before the connector even parses it**. There is **no disable knob** — the only env is `CASS_RAW_MIRROR_SIZE_WARN_THRESHOLD_BYTES` (a warn threshold, lib.rs:30543) and the only reclaim is the manual, audited `cass mirror prune` (dry-run by default, `--apply` required, 7d safety hold-down). So the 46G copy is **inherent to the current design**, not an option. It exceeds the 37.5G corpus because the mirror retains captures of sessions the agent harnesses have since rotated/deleted (the README's doctor section frames raw-mirror blobs as preserved "source evidence" / sole-copy protection).
2. **agent_search.db 22-23G** (frankensqlite): the declared **source of truth**, an append-only log (README:2203-2208 — messages are inserted, never updated; conversations accumulate). Schema: `conversations`, `messages` (full `content TEXT NOT NULL` + `extra_json` + `extra_bin`), `snippets`, plus analytics tables (`daily_stats`, `token_usage`, `token_daily_stats`, `model_pricing`), job tables (`embedding_jobs`), tail-state and external-lookup tables. It is 22G because it holds a **second full normalized copy of every message body**, plus a **contentless FTS5 inverted index** (`fts_messages`, `content=''`, porter tokenizer — storage/sqlite.rs:1162-1179) over content/title/agent/workspace/source_path.
3. **index/ 9.5G**: a real tantivy index, reached through the author's `frankensearch` crate (`frankensearch::lexical::{CassTantivyIndex, tantivy_crate, ...}` — src/search/tantivy.rs imports; tantivy appears in Cargo.lock via frankensearch). This is the "derived speed layer" with edge n-gram prefix indexing for search-as-you-type.
4. Optional semantic vectors (`vector_index/index-<embedder>.fsvi`, HNSW) when models are installed.

**Why both a 22G DB and a 9.5G tantivy index:** the README's asset contract says SQLite is authoritative and all search assets are derived/rebuildable; tantivy is the required fast lexical path; the in-DB FTS5 is a **fallback** engaged when tantivy is absent or non-authoritative (`search_sqlite_fts5`, query.rs:7006, engaged at query.rs:3713; the code logs "tantivy is authoritative when available"). So cass maintains **two overlapping lexical search engines over the same text** — an inverted index inside SQLite and a tantivy index beside it — plus the raw blobs, plus the normalized copy. Total: text stored 3x (raw blob, messages.content, source corpus) and indexed 2x (+vectors).

## 4. The frankensqlite pin

- Cargo.toml: `frankensqlite = { version = "0.1.5", package = "fsqlite", features = ["fts5"] }` — **fsqlite is a from-scratch pure-Rust SQLite reimplementation** by the same author (Dicklesworthstone), pinned at 0.1.5 (Cargo.lock confirms 0.1.5 from crates.io; registry cache shows the line already reaches 0.3.4). README:2085 gives the rationale: `BEGIN CONCURRENT` MVCC multi-writer transactions (used: storage/sqlite.rs:941, 3959 — a concurrent writer pool). README:2418: prebuilt Linux binaries require glibc 2.38+ "to access newer kernel features used by the frankensqlite storage engine."
- The sidecar files `agent_search.db-fsqlite-ns-gate` / `-ns-use` are **fsqlite's own VFS namespace lock files**: `GATE_SUFFIX = "-fsqlite-ns-gate"`, `USE_SUFFIX = "-fsqlite-ns-use"` in fsqlite-vfs `src/namespace.rs` (cargo registry). The ns-use file carries an `FSQLNS01` magic header. They are engine-private coordination state, alongside ordinary -wal/-shm.
- The migration was total: git history shows a "Major FrankenStorage rewrite" plus module-by-module rusqlite→frankensqlite migrations (commits e5789a7f, 89c1a0fb, 8b114ac3, 6657c980, f501868d). rusqlite survives only as a dev-dependency "for C-SQLite interop fixtures in tests" (Cargo.toml comment).
- **The realized risk (p3kgr handoff chain, commit 5a919924, 2026-08-15):** "The search hang is one step, `plan_lexical_shards`, and **stock SQLite does its work in 77 ms against the same live 22 GB archive where fsqlite 0.1.5 has now burned 434 minutes of CPU without returning**." A control archive (12,722 conversations) completes the identical code path in 4,202 ms — cass's logic is correct, the data is healthy; "the archive simply crossed the size where the pinned engine stops finishing." Commit 5d1718a3 adds: a query-phase-only re-run returned nothing in 980s (16m20s) before an external SIGTERM — a lower bound, not a runtime. That is a **>300,000x regression on one GROUP BY** attributable purely to the engine.
- Version hygiene around the pin is treacherous: fsqlite 0.1.5 resolves fsqlite-core forward to 0.1.19, "so a version-range probe is not a control" (same handoff). cass builds clean against fsqlite 0.1.19 on the installed nightly, but that bump "does not fix the performance at control scale."
- The pin is one instance of a **whole-stack pattern**: cass's load-bearing dependencies are largely same-author rewrites of standard infrastructure — `asupersync` 0.3.2 (async runtime + HTTP client in place of tokio/reqwest), `ftui`/frankentui (TUI framework, git-pinned rev), `frankensearch` (search engine wrapping tantivy, git-pinned rev), `franken-agent-detection` (connectors, git-pinned rev), `toon`/tru, `frankensqlite`. 823 packages in Cargo.lock. Every one of these forfeits the ecosystem's testing surface, and the storage engine — the worst place to accept that trade — is where it failed.

## 5. Verdict: right-sized-mechanism test

**The mechanism is dramatically larger than the problem, at three levels.**

1. **Dependency level.** The problem is "search local JSONL files." The mechanism includes a reimplementation of SQLite (0.1.x maturity) chosen for multi-writer MVCC — a property whose value to a single-user local search tool is marginal, and whose cost was the production outage above. Stock SQLite's 77ms on the same query, against the same 22G file, is the direct measurement that the bespoke engine solves a problem cass does not have while failing the one it does.
2. **Storage level.** 37.5G of source becomes 77G of derived state: an unconditional 46G raw mirror (no off switch), a 22G append-only normalized copy with an FTS5 index inside it, and a second 9.5G lexical engine beside it. Two lexical indexes over the same text is redundancy by design, justified as "derived speed layer" — but the fallback path (FTS5) is the one stock engines execute in milliseconds.
3. **Module level.** ≈59k non-test LOC of periphery (Cloudflare publishing, swarm surfaces, evidence bundles, crash replay harnesses, topology budgets, explainability cards, policy registries, bakeoff, asciicast recording, analytics charting) against a core need a few thousand lines could serve. Several peripheral modules self-describe as "planned" surfaces or exist to produce evidence about cass's own development process. The test mass (≈335k lines across inline + tests/) exceeds the product code (~218k) — process rigor lavished on a shape that fails its headline job (README: sub-60ms; measured: minutes-to-hours, or never).

**Is there a sound small core? Yes — and cass's own record proves it is extractable.**

- The data model is sane and boring: `conversations`/`messages`/`snippets` + contentless FTS5. Nothing about it needs fsqlite.
- The 22G archive is **stock-SQLite compatible**: the p3kgr handoff ran stock SQLite against "the same live 22 GB archive" (77ms); frankensqlite explicitly interoperates with stock-SQLite databases (storage/sqlite.rs:1156-1161 handles loading stock DBs); rusqlite fixtures exist in-tree.
- A working stock-shaped search path already exists in the code (`search_sqlite_fts5` — rank + hydrate over `fts_messages`).
- Format detection/normalization for 20 harnesses is already factored into a separate crate (`franken-agent-detection`); the in-repo connector glue is only ~1.1k non-test LOC.

A right-sized cass is roughly: walk source trees → normalize (existing connector crate or plain serde) → rusqlite (bundled C SQLite) with the **same schema and FTS5** → the existing FTS5 query path → a thin TUI or plain CLI output. That is plausibly 5-10k LOC and one battle-tested storage engine, with zero raw mirror by default (or opt-in), no second lexical engine, no bespoke runtime. The 46G mirror, the tantivy layer, semantic search, remote sync, and pages publishing are all separable options — none is load-bearing for "find that prompt from two weeks ago."

**Structural vs incidental:**
- *Structural (design-inherent):* the disk quadrupling (unconditional pre-parse raw mirror + append-only DB + dual lexical indexes are deliberate design decisions, documented in the README's asset contract); the whole-stack bespoke-dependency posture; the periphery mass (it is the project's stated surface area, golden-file-pinned).
- *Structural in practice though incidental in principle:* the fsqlite engine choice. Swapping engines is conceptually a seam, but the 26k-line storage layer is woven with fparams! macros, franken compat gates, franken-specific FTS rebuild metadata keys, ns sidecar semantics, and a BEGIN CONCURRENT writer pool — migrating back to rusqlite is a project, not a patch. (The data migrates trivially; the code does not.)
- *Incidental (fixable bugs/rot):* the specific GROUP BY performance wall (an engine defect at scale); the stall detector false-firing on healthy runs; the god-file organization (92k-line lib.rs, 47k-line app.rs) — enormous but mechanical to split.

## Evidence trail

- Module LOC: `fd -e rs . src -x wc -l` (clean count: 161 files, 393,912). Per-module non-test split: brace-count heuristic over `#[cfg(test)]` blocks (labeled ESTIMATE; total 176,076 test lines, 44.7%).
- Raw mirror capture: src/indexer/mod.rs:22246-22290 (`capture_discovered_source_file_before_parse` → `raw_mirror::capture_source_file`), call sites 22265/22425/22468, lib.rs:36295; no disable flag found (`CASS_RAW_MIRROR_SIZE_WARN_THRESHOLD_BYTES` warn-only, lib.rs:30543); prune: lib.rs:1331-1366 (`MirrorCommand::Prune`, dry-run default).
- FTS5: storage/sqlite.rs:1156-1189 (contentless fts_messages, porter); fallback engagement: search/query.rs:3680-3725; implementation query.rs:7006.
- Tantivy via frankensearch: src/search/tantivy.rs:17-25 imports; Cargo.lock tantivy entries under frankensearch.
- fsqlite pin + rationale: Cargo.toml; README.md:2081-2085, 2203, 2418, 2936; Cargo.lock fsqlite 0.1.5; sidecars: fsqlite-vfs-0.1.17/src/namespace.rs:31-32 (GATE_SUFFIX/USE_SUFFIX).
- GROUP BY story: commits 5a919924 ("cass is fine — the pinned SQLite engine cannot run the GROUP BY", full body quoted above), 5d1718a3 (16m20s query phase), dcbb2c52 (rebuild fixed, 57s), 8e4e0241 (RED verdict collapsed / pin ceiling never real).
- Repo age/commits: `git log --reverse` (2025-11-20) → 2026-08-17, `rev-list --count` = 4,261.
- Production dir listing (read-only): agent_search.db 23G, raw-mirror 46G (blobs/manifests/tmp), ns-gate 0B, ns-use 40B (`FSQLNS01` header).
- Probe hygiene note: two early probes in this lane used `rg -rn` (the `-r` replace trap from the global rules); their outputs were recognized as fabricated and discarded — every finding above traces to a clean re-run.
