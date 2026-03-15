---
title: "chore: Evaluate fork sync — fast-forward main to upstream"
date: 2026-03-15
bead: coding_agent_session_search-3fir
---

# Fork Sync Evaluation

## Situation

Our fork (`carmandale/coding_agent_session_search`) is **51 commits behind**
upstream (`Dicklesworthstone/coding_agent_session_search`) with **zero divergent
commits** on main. Fork main can be fast-forwarded cleanly.

However, our active branch `fix/watcher-cpu-spin` was based on the old main
and conflicts with 21 source files that upstream changed.

The watcher fix is **already deployed and running** — the binary is installed,
the watchdog is deployed, CPU is at 0%. The question is whether to sync with
upstream or stay diverged.

## What Upstream Changed (51 commits)

### Major themes

| Theme | Commits | Impact |
|-------|---------|--------|
| **FrankenSQLite migration** | ~15 | Replaced rusqlite with frankensqlite (custom SQLite fork with BEGIN CONCURRENT). Major refactor of storage layer. |
| **FrankenSearch/FrankenTUI** | ~5 | Replaced tantivy search and ratatui TUI with custom forks. API changes throughout. |
| **Release pipeline hardening** | ~10 | Pin frankensqlite revisions, static OpenSSL, glibc floor, checksum verification |
| **New connectors** | 3 | Copilot CLI, Kimi Code, Qwen Code (re-export stubs) |
| **Security** | 1 | Redact secrets from tool-result content before DB insert |
| **Incremental semantic embeddings** | 1 | Embedding in watch mode (relevant to our watcher work) |
| **Colorblind theme** | 2 | Accessibility preset for deuteranopia/protanopia |
| **Bug fixes** | ~10 | FTS5 registration, doctor OOM, export modal keys, resize logging, etc. |

### Conflict analysis

**21 files overlap** between upstream changes and our branch. The critical ones:

| File | Upstream changes | Our changes | Conflict severity |
|------|-----------------|-------------|-------------------|
| `src/indexer/mod.rs` | +716/-109 (semantic embeddings, frankensqlite) | +87 (SIGTERM, heartbeat, slow-scan logging) | **High** — massive upstream rewrite |
| `src/lib.rs` | +426/-266 (frankensqlite, frankentui, API changes) | +60 (named threads) | **High** — widespread refactor |
| `src/search/query.rs` | +353/-126 (frankensearch integration) | +3 (named test thread) | **Low** — our change is trivial |
| `src/connectors/mod.rs` | New connectors, franken_agent_detection | No direct changes from us | **Medium** — structural |
| `src/storage/sqlite.rs` | frankensqlite migration | None from us | **Medium** — we don't touch storage |

**Files we changed that upstream did NOT touch (no conflict):**

| File | Our changes | Safe? |
|------|-------------|-------|
| `src/connectors/pi_agent.rs` | detect() root fix, scan() is_pi_path, max_depth, 6 tests | ✅ Clean |
| `src/update_check.rs` | Named thread | ✅ Clean |
| `src/ui/tui.rs` | Named thread | ✅ Clean |
| `src/ui/data.rs` | Named test threads | ✅ Clean |
| `scripts/watchdog.sh` | New file | ✅ Clean |
| `Cargo.toml` | strip=debuginfo, signal-hook | ⚠️ Upstream changed deps too |

## Risk Assessment

### If we sync (rebase onto upstream main)

**Effort:** High. The `indexer/mod.rs` rebase alone is ~800 lines of upstream
changes vs our ~90 lines. Many are frankensqlite API changes (`rusqlite::Connection`
→ `frankensqlite` equivalents) that are mechanical but tedious. `lib.rs` has
~700 lines of changes on both sides.

**Risk:** Moderate. Our changes (SIGTERM handler, heartbeat, named threads) are
conceptually orthogonal to upstream's changes (frankensqlite, semantic embeddings).
The conflicts are structural (same lines changed) but not semantic (different
concerns). A careful rebase should preserve both.

**Benefit:**
- Get frankensqlite (BEGIN CONCURRENT — better write performance)
- Get secret redaction (security fix)
- Get incremental semantic embeddings (relevant to watcher)
- Get 3 new connectors (Copilot CLI, Kimi, Qwen)
- Get bug fixes (FTS5, doctor OOM, export modal)
- Stay current with upstream for future updates

### If we stay diverged

**Effort:** Zero now. Growing over time.

**Risk:** Low now. Every upstream release increases the eventual merge cost.
If upstream refactors connectors or the indexer further, our pi_agent.rs
and indexer changes become harder to port.

**Benefit:**
- The fix is deployed and working today
- No risk of introducing regressions from upstream changes
- Can cherry-pick individual upstream fixes if needed

## FrankenSQLite Deep Dive

FrankenSQLite is NOT just "a different SQLite binding." It's a **ground-up
Rust reimplementation of SQLite** with two major architectural innovations:

1. **MVCC Concurrent Writers** — Replaces SQLite's single-writer lock with
   page-level Multi-Version Concurrency Control. Multiple writers commit
   simultaneously as long as they touch different pages. Serializable
   Snapshot Isolation (SSI) prevents write skew. This directly addresses
   cass's write bottleneck during indexing.

2. **RaptorQ Self-Healing Storage** — RFC 6330 fountain codes infused into
   every persistent layer. WAL frames carry repair symbols for self-healing
   after torn writes. This is relevant to our CPU spin bug — if tantivy
   corruption from dirty kills is the root cause, frankensqlite's
   self-healing could prevent it at the storage layer.

### How upstream uses it in cass

Upstream has introduced a **dual-storage architecture**:

- `SqliteStorage` — the existing rusqlite backend (still used by the indexer)
- `FrankenStorage` — new frankensqlite backend with a `FrankenConnectionManager`
  that provides a reader pool (4 connections) + concurrent writer pool
  (up to available_parallelism connections)

The migration is **in-progress, not complete**:
- The indexer (`mod.rs`) still uses `SqliteStorage` (rusqlite)
- The connection manager and concurrent writers exist but are used in
  benchmarks, search, and vector index — not yet the hot indexing path
- Both `rusqlite` and `frankensqlite` are in Cargo.toml simultaneously

### What BEGIN CONCURRENT means for cass

The indexer currently serializes all writes through a single `SqliteStorage`
behind a `Mutex`. With `FrankenConnectionManager.concurrent_writer()`, the
indexer could parallelize connector ingestion — each connector writes to its
own concurrent writer connection, and frankensqlite's page-level MVCC
resolves conflicts at commit time.

This would directly fix the "one connector blocks all others" bottleneck
and make the 30-minute full scan faster (parallel connector writes instead
of sequential).

### Current status caveat

The README has this important note:
> "The current runnable engine is already real, but still hybrid.
> Compatibility mode over standard SQLite files is the live runtime path
> today; Native mode / ECS sections describe the longer-term design."

So frankensqlite works today for SQLite-compatible operations, but some
advanced features are still in progress.

## Updated Recommendation

**This changes the calculus.** FrankenSQLite's concurrent writers are
directly relevant to the watcher performance problem — not just a nice-to-have.

### Arguments for syncing now

1. **Concurrent writers** could eliminate the "one slow connector blocks
   everything" bottleneck that's causing the OpenCode 100% CPU scan
   (115K files, 10 GB) to block all other connectors.
2. **Self-healing storage** could mitigate the tantivy corruption risk
   from dirty kills (defense in depth with our SIGTERM handler).
3. **Incremental semantic embeddings in watch mode** — directly builds on
   our watcher work.
4. **Secret redaction** — security fix we should have.
5. **3 new connectors** (Copilot CLI, Kimi, Qwen) — more agent coverage.
6. The longer we wait, the harder the rebase gets.

### Arguments for waiting

1. FrankenSQLite is a **ground-up reimplementation** of SQLite by a single
   developer. It's ambitious but young. The upstream commit history shows
   ~10 "pin frankensqlite to X (fixes Y compile errors)" commits — it's
   still stabilizing.
2. The indexer hasn't actually switched to concurrent writers yet — it
   still uses `SqliteStorage`. We'd be syncing to get the infrastructure
   without the payoff in the hot path.
3. 21 file conflicts is still a half-day of careful work.

### Recommendation

**Sync, but not urgently.** The concurrent writer infrastructure is worth
having. Schedule it as a focused session:

1. Fast-forward fork main to upstream main (clean fast-forward)
2. Rebase `fix/watcher-cpu-spin` onto new main
3. Resolve conflicts (the functions are the same, just larger)
4. Re-test, rebuild, redeploy
5. Estimated effort: 4-6 hours

The key files that need conflict resolution:
- `src/indexer/mod.rs` — our SIGTERM/heartbeat/slow-scan vs upstream's
  semantic embeddings and new connectors (HIGH effort, same functions)
- `src/lib.rs` — our named threads vs upstream's frankentui/API changes
  (HIGH effort, widespread)
- `Cargo.toml` — our signal-hook + strip vs upstream's franken deps
  (LOW effort, additive)
- 18 other files — mostly upstream-only changes that auto-resolve

## Acceptance Criteria

- [x] Decision documented: sync — frankensqlite concurrent writers are worth having
- [x] Napkin updated with fork ownership notes
- [ ] Fork main fast-forwarded to upstream main
- [ ] `fix/watcher-cpu-spin` rebased onto new main (21 file conflicts)
- [ ] All tests pass after rebase
- [ ] Release binary rebuilt and installed
- [ ] Watcher restarted and verified healthy
