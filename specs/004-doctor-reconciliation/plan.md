<!-- Codex Review: APPROVED after 4 rounds | model: gpt-5.3-codex | date: 2026-03-07 -->
<!-- Status: REVISED -->
<!-- Revisions: R1 per-connector filters audited against actual scan() code, default impl removed (required method), signed delta, configurable threshold, notes schema, test plan, canonical registry reuse. R2 Codex filter corrected to rollout-* in sessions/, Amp filter expanded to all 4 match paths. R3 stale references in Files Changed table fixed, "fail" removed from status enum. -->
---
title: "Implementation plan: cass doctor reconciliation"
date: 2026-03-07
bead: coding_agent_session_search-2gxp
---

# Plan: Disk-vs-DB Reconciliation in `cass doctor`

## Architecture Decision: Trait Extension vs. Standalone Logic

**Chosen: Add `count_disk_files()` method to `Connector` trait — NO usable default impl.**

The `Connector` trait currently has two methods: `detect()` and `scan()`. We need a lightweight way to count files on disk without parsing them.

Every connector has unique file-selection logic — a generic "count all files in root" default would produce wrong counts for all connectors. Therefore `count_disk_files()` is a **required method** (no default body), ensuring each connector explicitly defines its counted unit.

### Per-connector file-counting specification

Each implementation must mirror the exact file-matching logic from `scan()`:

| Connector | Counted unit | File filter | Notes |
|-----------|-------------|-------------|-------|
| **claude_code** | Session files | `.jsonl`, `.json`, `.claude` extensions | Walk all subdirs including projects/subagents |
| **gemini** | Session files | Delegate to `Self::session_files(root).len()` | Already has exact enumeration |
| **factory** | JSONL files | `.jsonl` extension, skip `.settings.json` | |
| **codex** | Rollout files | `rollout-*.jsonl` and `rollout-*.json` in `sessions/` dir (via `Self::rollout_files()`) | |
| **pi_agent** | Session files | Delegate to `Self::session_files(root).len()` | Already has exact enumeration |
| **chatgpt** | Export files | `.json` and `.data` extensions | |
| **aider** | History files | Filename == `.aider.chat.history.md` only | Bounded: checks CWD + env override only, no recursive scan |
| **amp** | Log files | `is_amp_log_file()` — `.json` files with stems containing `thread`, `conversation`, or `chat`; or `T-{uuid}.json` format; or any `.json` under a `threads/` parent directory | |
| **cline** | Task dirs with messages | Dirs containing `ui_messages.json` or `api_conversation_history.json` | Count = task dirs, not individual JSON files |
| **codebuff** | Workspace dirs with chat | Dirs containing `chat-messages.json` | Count = workspace dirs, not JSON files |
| **opencode** | Session files | `session/{projectID}/{sessionID}.json` | Walk session/ subdir only |
| **cursor** | **Non-comparable** | SQLite `state.vscdb` stores N conversations per DB | `status: "skip"`, `notes: "Cursor uses SQLite; DB-to-conversation mapping is not 1:1 with files"` |

### Cursor special handling

Cursor stores conversations inside SQLite databases (one DB = many conversations). Disk file count (number of `.vscdb` files) is not comparable to DB conversation count. The reconciliation check reports `status: "skip"` with an explanatory note. This is architecturally honest — reporting a false "balanced" or false "gap" would be worse.

## Implementation Approach

### 1. Extend `Connector` trait

```rust
pub trait Connector {
    fn detect(&self) -> DetectionResult;
    fn scan(&self, ctx: &ScanContext) -> Result<Vec<NormalizedConversation>>;

    /// Count session files on disk without parsing.
    /// Used by `cass doctor` for reconciliation.
    /// Must mirror the file-selection logic of `scan()`.
    /// Returns `None` if disk-file counting is not meaningful for this connector
    /// (e.g., Cursor uses SQLite where 1 file ≠ 1 conversation).
    fn count_disk_files(&self) -> Option<usize>;
}
```

Return type is `Option<usize>`:
- `Some(n)` — connector counted n files on disk
- `None` — counting is not meaningful (Cursor), reported as `status: "skip"`

### 2. Reuse canonical connector registry

Doctor reuses `indexer::get_connector_factories()` to instantiate connectors, avoiding duplication:

```rust
use crate::indexer::get_connector_factories;

// Slug mapping: get_connector_factories() uses short names ("claude"),
// but DB agents table uses agent_slug ("claude_code").
// Build a mapping from factory key to DB slug.
let slug_map: HashMap<&str, &str> = [
    ("claude", "claude_code"),
    // All others match: "codex" -> "codex", "gemini" -> "gemini", etc.
].into_iter().collect();

for (factory_key, factory_fn) in get_connector_factories() {
    let connector = factory_fn();
    let db_slug = slug_map.get(factory_key).unwrap_or(&factory_key);
    // ... reconcile
}
```

### 3. Signed delta and threshold

Delta is `i64`, not unsigned:

```rust
struct ReconResult {
    agent: String,       // DB slug
    disk_files: Option<usize>,  // None = non-comparable (Cursor)
    db_entries: usize,
    delta: Option<i64>,  // disk - db; None if disk_files is None
    status: String,      // "pass", "warn", "skip"
    notes: Option<String>,
}
```

Status derivation:
- `disk_files == None` → `"skip"` (non-comparable connector)
- `delta == 0` → `"pass"`
- `0 < delta <= threshold` → `"warn"`
- `delta > threshold` → `"warn"` with `above_threshold: true` flag in JSON (reconciliation does not affect exit code in this PR; all findings are diagnostic-only)
- `delta < 0` → `"warn"` with note "DB has more entries than disk files (possible orphaned DB entries)"

**Threshold source:** New CLI arg `--reconciliation-threshold N` on the Doctor command (default: 10). Included in JSON `_meta`.

**Exit code:** Reconciliation findings do NOT escalate doctor exit code in this PR. They are purely diagnostic. This prevents false-positive failures from intentional skips while we measure real-world rates.

### 4. Intentional-skip notes

Each connector can provide a static `reconciliation_notes()` method:

```rust
pub trait Connector {
    // ... existing methods ...
    fn count_disk_files(&self) -> Option<usize>;

    /// Optional contextual notes for reconciliation output.
    /// E.g., "10 Factory stubs are session_start-only by design"
    fn reconciliation_notes(&self) -> Option<String> {
        None  // default: no notes
    }
}
```

Known notes:
- **Factory**: "Some files contain only session_start with no messages — these are intentionally skipped"
- **Claude Code**: "Progress-only subagent files with no user/assistant content are intentionally skipped"
- **Cursor**: "Cursor uses SQLite databases; file count is not comparable to conversation count"

### 5. DB query helper

```rust
fn db_count_for_agent(conn: &rusqlite::Connection, slug: &str) -> usize {
    conn.query_row(
        "SELECT COUNT(*) FROM conversations c JOIN agents a ON c.agent_id = a.id WHERE a.slug = ?1",
        [slug],
        |r| r.get::<_, i64>(0),
    ).unwrap_or(0).max(0) as usize
}
```

Error handling: if the query fails (DB locked, table missing), return 0 and add a note "DB query failed" to the result. Do not panic or propagate the error — reconciliation is best-effort diagnostics.

### 6. Add reconciliation to `run_doctor()`

After existing check #7 (session directories), insert check #8. Only runs if `db_ok` is true (no point reconciling against a broken DB):

```rust
// 8. Per-connector disk-vs-DB reconciliation
if db_ok {
    let recon_start = Instant::now();
    let mut recon_results = Vec::new();
    let conn = rusqlite::Connection::open_with_flags(
        &db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    );
    // ... (see Section 2 for iteration logic)
    let recon_elapsed_ms = recon_start.elapsed().as_millis() as u64;
}
```

Uses a **read-only** SQLite connection to prevent any accidental writes during diagnostics.

### 7. Output integration

**JSON:** Add `reconciliation` key to existing payload:

```json
{
  "reconciliation": {
    "balanced": true,
    "elapsed_ms": 1234,
    "threshold": 10,
    "connectors": [ ... ]
  }
}
```

**Human:** Add to check list:

```
✓ reconciliation: 11 connectors balanced, 1 skipped (1234ms)
```

Or with gaps:

```
⚠ reconciliation: 2 connectors have gaps, 1 skipped (1234ms)
    gemini: 41 on disk, 28 in DB (delta: +13)
    factory: 76 on disk, 66 in DB (delta: +10) — Some files contain only session_start with no messages
    cursor: skipped — Cursor uses SQLite databases; file count is not comparable to conversation count
```

### 8. Performance bounds

- `count_disk_files()` for all 12 connectors: ~1-2s (stat-only, no file reads)
- Aider is bounded: only checks 1-2 known paths, no recursive walk
- DB queries: <50ms (12 COUNT queries on indexed table with read-only connection)
- Total overhead to existing `cass doctor`: ~2s

### 9. Test plan

Tests live in `src/lib.rs` (integration-level) and per-connector files (unit-level).

**Unit tests (per connector):**
- Each `count_disk_files()` override: create temp fixture dir matching connector's pattern, verify count matches
- Cursor returns `None`
- Empty/missing root returns 0

**Integration tests (in `run_doctor` or separate test module):**
- Balanced state: mock DB with counts matching disk → `balanced: true`, all `"pass"`
- Gap detected: mock DB with fewer entries than disk → `balanced: false`, connector shows `"warn"`
- Negative delta: DB has more than disk → `"warn"` with orphaned-entries note
- Connector not found on disk: `detect()` returns false → `disk_files: 0`, `status: "pass"` (nothing on disk, nothing in DB)
- Cursor skip: verify `status: "skip"` in output
- JSON format: parse JSON output, verify `reconciliation` key exists with expected schema
- Threshold: verify `delta > threshold` produces `"warn"` with `above_threshold: true` in JSON

## Files Changed

| File | Change |
|------|--------|
| `src/connectors/mod.rs` | Add `count_disk_files() -> Option<usize>` and `reconciliation_notes() -> Option<String>` to `Connector` trait |
| `src/connectors/claude_code.rs` | Impl `count_disk_files()` — count `.jsonl`/`.json`/`.claude` files |
| `src/connectors/gemini.rs` | Impl — `Some(Self::session_files(root).len())` |
| `src/connectors/pi_agent.rs` | Impl — `Some(Self::session_files(root).len())` |
| `src/connectors/factory.rs` | Impl — count `.jsonl`, skip `.settings.json` |
| `src/connectors/codex.rs` | Impl — delegate to `Self::rollout_files(root).len()` |
| `src/connectors/chatgpt.rs` | Impl — count `.json`/`.data` files |
| `src/connectors/aider.rs` | Impl — count `.aider.chat.history.md` files (bounded, no recursive walk) |
| `src/connectors/amp.rs` | Impl — count files passing `is_amp_log_file()` |
| `src/connectors/cline.rs` | Impl — count task dirs with `ui_messages.json` or `api_conversation_history.json` |
| `src/connectors/codebuff.rs` | Impl — count dirs with `chat-messages.json` |
| `src/connectors/opencode.rs` | Impl — count `session/{pid}/{sid}.json` files |
| `src/connectors/cursor.rs` | Impl — return `None` (non-comparable) |
| `src/lib.rs` | Add reconciliation check #8 to `run_doctor()`, JSON/human output, `--reconciliation-threshold` CLI arg |

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| False positives from intentionally skipped files | Delta labeled "unindexed" not "missing"; `reconciliation_notes()` provides context; reconciliation does not affect exit code |
| Connector root detection failure masks real gaps | If `detect()` returns false, report `disk_files: 0`; if no agent in DB either, `status: "pass"` (consistent nothing) |
| Performance regression on large installs | `count_disk_files()` is stat-only; Aider bounded; timing reported in JSON `elapsed_ms` |
| DB locked during reconciliation | Use read-only connection; tolerate query failure with fallback to 0 + note |
| Slug mapping drift between `get_connector_factories()` and DB `agents.slug` | Single slug-map constant; test validates all factory keys map to valid DB slugs |
| New connectors added without `count_disk_files()` impl | Required trait method (no default) → compile error forces author to implement |
