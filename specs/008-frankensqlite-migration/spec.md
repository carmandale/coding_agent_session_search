---
title: "Migrate storage to frankensqlite + frankensearch (upstream v0.2.4 perf)"
date: 2026-03-27
bead: coding_agent_session_search-3iqk
---

<!-- issue:complete:v1 | harness: unknown | date: 2026-03-27T10:36:12Z -->
<!-- Codex Review: APPROVED after 5 rounds | model: gpt-5.3-codex | date: 2026-03-27 -->
<!-- Status: UNCHANGED — spec predates shaping; plan reflects the shaped outcome -->
<!-- Note: plan.md supersedes spec.md where they conflict (full upstream sync vs selective cherry-pick) -->

# Spec 008 — Frankensqlite + Frankensearch Migration

## Problem

Our fork is on schema v8 with `rusqlite` (standard SQLite via C FFI) and `tantivy`
(pure-Rust FTS). Upstream (Dicklesworthstone, v0.2.4) has migrated to:

- **`frankensqlite`** — ground-up Rust reimplementation of SQLite with concurrent
  writers, MVCC, and information-theoretic durability. Key wins for cass: concurrent
  reader + writer connections without WAL lock contention; the 8.8 GB DB currently
  serialises all writes through a single lock.
- **`frankensearch`** — two-tier hybrid search: sub-millisecond BM25 via Tantivy,
  quality-refined rankings via MiniLM-L6-v2 vectors, Reciprocal Rank Fusion.
  Directly replaces our 12 open P1 semantic-search beads with a maintained library.

Both repos are **public on GitHub** (`Dicklesworthstone/frankensqlite`,
`Dicklesworthstone/frankensearch`). Upstream uses `path = "../..."` deps because they
live in a monorepo sibling; we can consume them as `git` deps instead.

## Goals

1. Replace `rusqlite` with `frankensqlite` in `src/storage/sqlite.rs`.
2. Replace `tantivy` (FTS) with `frankensearch` lexical+semantic pipeline.
3. Migrate schema v8 → v13 safely, preserving all 24K+ existing conversations.
4. Close or supersede the 12 open semantic-search P1 beads (`vmet`, `7tsm`, `8q8f`,
   `vwxq`, `tn4t`, `mwsa`, `9vjh`, `rzrv`, `wsfj`, `vh6q`, `44pw`, `795s`).

## Out of Scope

- **`frankentui`** — keep `ratatui`. The TUI rewrite is massive and unrelated to
  indexing performance.
- **`asupersync`** — upstream uses it only for the ML models command; we can keep
  `tokio::task::spawn_blocking`.
- **Historical salvage toolkit** — upstream-specific operational concern for their
  production DB. Not relevant to our setup.
- **Upstream connector rewrites** — we keep our own connector stack. New FAD connectors
  (crush, kimi, copilot_cli, qwen) are a separate bead.

## Dependency Plan

```toml
# Replace in Cargo.toml:

# REMOVE:
rusqlite = { version = "*", features = ["bundled", "modern_sqlite"] }
tantivy = "*"

# ADD:
frankensqlite = { git = "https://github.com/Dicklesworthstone/frankensqlite", rev = "0d5df2b0", package = "fsqlite", features = ["fts5"] }
frankensearch = { git = "https://github.com/Dicklesworthstone/frankensearch", rev = "a7bcddb1", default-features = false, features = ["hash", "lexical", "ann", "fastembed-reranker"] }
```

Revs to pin at start of implementation; bump as needed.

## Architecture

### Storage layer (`src/storage/sqlite.rs`)

Current: `rusqlite::Connection` wrapped in `Arc<Mutex<_>>` with a reader pool
(parking_lot) and token-based writer limit.

Target: `frankensqlite::Connection` (FrankenConnection) with the same
reader-pool + writer-token pattern. The frankensqlite API is rusqlite-compatible
at the statement/row level; upstream provides a `compat` module that re-exports
familiar types (`ConnectionExt`, `Transaction`, `Row`, etc.). The migration is
largely mechanical — swap import paths, replace `rusqlite::params![]` with
`fparams![]`, replace `rusqlite::Error` with `frankensqlite::FrankenError`.

Key additions from upstream's storage layer:
- **Lazy connection** (`LazyFrankenDb`): defers DB open until first use, cutting
  startup latency for commands that don't need the DB.
- **Schema v13**: adds `daily_stats`, `search_history`, `bookmarks` tables and
  FTS5 consistency columns. Migrations v9–v13 must be ported.
- **Preflight repair**: detects read-only open failure and attempts repair before
  hard-failing (relevant for the 8.8 GB DB on a cold start).

### Search layer (`src/search/`)

Current: `tantivy` for lexical FTS; our own CVVI vector index (planned, not built);
hash embedder as fallback; FastEmbed ML (planned).

Target: `frankensearch` unifies all of this:
- Lexical tier: Tantivy BM25 (same underlying engine, exposed via frankensearch API)
- Semantic tier: FastEmbed MiniLM-L6-v2 via `fastembed-reranker` feature
- Fusion: built-in RRF implementation
- Vector index: FSVI format (replaces our planned CVVI)
- Hash embedder: built-in FNV-1a fallback

This means `src/search/tantivy.rs`, `src/search/embedder.rs`,
`src/search/hash_embedder.rs`, `src/search/vector_index.rs`,
`src/search/fastembed_embedder.rs` can all be replaced or thinned to adapters
over frankensearch.

## Schema Migration Path

| Version | Change |
|---------|--------|
| v8 (current) | Base: conversations, messages, snippets, agents, meta |
| v9 | Add `search_history` table |
| v10 | Add `bookmarks` table |
| v11 | Add `daily_stats` table |
| v12 | FTS5 consistency repair columns |
| v13 | Fast-forward marker + frankensqlite internal schema |

Migration must be idempotent and tested against a copy of the real 8.8 GB DB
before being run in production. A `VACUUM INTO` snapshot before migration is
mandatory.

## Acceptance Criteria

- [ ] `cargo build --release` succeeds with frankensqlite + frankensearch deps
- [ ] Existing 24,340 conversations are readable after schema migration
- [ ] `cass health --json` reports `healthy: true`
- [ ] `cass search "test" --robot` returns results in < 100ms (vs current ~300ms)
- [ ] Semantic search (`--mode semantic`) works with hash embedder fallback
- [ ] Semantic search works with MiniLM model when model files are present
- [ ] `cass index --full` completes without error on the live 8.8 GB DB
- [ ] Watcher continues to function after migration
- [ ] `cass watchdog run` reports healthy
- [ ] P1 semantic search beads (`vmet`, `7tsm`, `8q8f`, `vwxq`, `tn4t`, `mwsa`,
      `9vjh`, `rzrv`) are closed or explicitly superseded

## Constraints & Risks

**Risk: frankensqlite is a ground-up reimplementation.** It is not battle-tested
at the scale of SQLite. Our 8.8 GB DB is non-trivial. Mitigation: keep a `VACUUM
INTO` snapshot before migration; maintain a rollback path to rusqlite for at least
one release.

**Risk: schema v8 → v13 on 8.8 GB.** Must be tested offline first. The migration
runner in frankensqlite (`MigrationRunner`) must be verified to handle our existing
data without corruption.

**Risk: frankensearch API stability.** Pinned to rev `a7bcddb1`. Upstream may break
the API. Mitigation: pin rev, only bump intentionally.

**Risk: concurrent writer improvement may not materialise.** Our watcher is
single-threaded for writes; the main concurrency win is reader/writer overlap during
TUI use. Profile before and after.

**Risk: asupersync.** Upstream uses it for `spawn_blocking` in the models command.
We avoid this dep by using `tokio::task::spawn_blocking` directly, which is
equivalent.

## Reference

- upstream: `Dicklesworthstone/coding_agent_session_search` @ `5fe06f0c` (v0.2.4)
- frankensqlite: `Dicklesworthstone/frankensqlite` @ `0d5df2b0`
- frankensearch: `Dicklesworthstone/frankensearch` @ `a7bcddb1`
- our current schema: `src/storage/sqlite.rs` line 224 (`CURRENT_SCHEMA_VERSION = 8`)
- upstream schema: v13 (`src/storage/sqlite.rs` in upstream/main)
