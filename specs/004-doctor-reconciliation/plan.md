---
title: "Implementation plan: cass doctor reconciliation"
date: 2026-03-07
bead: coding_agent_session_search-2gxp
---

# Plan: Disk-vs-DB Reconciliation in `cass doctor`

## Architecture Decision: Trait Extension vs. Standalone Logic

**Chosen: Add `count_files()` method to `Connector` trait with default impl.**

The `Connector` trait currently has two methods: `detect()` and `scan()`. We need a lightweight way to count files on disk without parsing them. Options considered:

| Approach | Pros | Cons |
|----------|------|------|
| **A: New `count_files(&self) -> usize` trait method** | Clean, per-connector, testable | 12 impls (but most can share a default) |
| B: Standalone function that re-walks roots | No trait change | Duplicates root/file-pattern logic from each connector |
| C: Dry-run `scan()` that counts instead of parsing | Reuses existing logic exactly | `scan()` does heavy parsing work we don't need; too slow |

Approach A wins because:
- Each connector already knows its root path and file patterns
- A default implementation using `detect()` + `WalkDir` covers most connectors
- Connectors with custom file-matching (Gemini's `session_files()`, Cursor's SQLite) can override
- The method is fast (stat-only, no parsing) and independently testable

## Implementation Approach

### 1. Extend `Connector` trait

```rust
pub trait Connector {
    fn detect(&self) -> DetectionResult;
    fn scan(&self, ctx: &ScanContext) -> Result<Vec<NormalizedConversation>>;

    /// Count session files on disk without parsing.
    /// Used by `cass doctor` for reconciliation.
    /// Default: walks detected roots counting files matching extension filter.
    fn count_disk_files(&self) -> usize {
        let det = self.detect();
        if !det.detected {
            return 0;
        }
        // Default: count all files in root_paths
        det.root_paths.iter().map(|root| {
            WalkDir::new(root).into_iter()
                .flatten()
                .filter(|e| e.file_type().is_file())
                .count()
        }).sum()
    }
}
```

Connectors that need custom logic override this:
- **Claude Code**: Count `.jsonl` and `.json` files, skip `.settings.json`
- **Gemini**: Use `session_files()` already implemented
- **Cursor**: Count SQLite workspace DBs
- **Factory**: Count `.jsonl` files, skip `.settings.json`
- **ChatGPT**: Count `.json` conversation files
- Others: Default `WalkDir` count is sufficient

### 2. Add reconciliation to `run_doctor()`

After the existing check #7 (session directories), add check #8:

```rust
// 8. Per-connector disk-vs-DB reconciliation
let connectors: Vec<(&str, Box<dyn Connector>)> = vec![
    ("claude_code", Box::new(ClaudeCodeConnector::new())),
    ("gemini", Box::new(GeminiConnector::new())),
    // ... all 12
];

let mut recon_results = Vec::new();
for (slug, connector) in &connectors {
    let disk_count = connector.count_disk_files();
    let db_count = db_count_for_agent(&conn, slug);
    let delta = disk_count.saturating_sub(db_count);
    recon_results.push(ReconResult { agent: slug, disk_files: disk_count, db_entries: db_count, delta });
}
```

### 3. DB query helper

```rust
fn db_count_for_agent(conn: &rusqlite::Connection, slug: &str) -> usize {
    conn.query_row(
        "SELECT COUNT(*) FROM conversations c JOIN agents a ON c.agent_id = a.id WHERE a.slug = ?1",
        [slug],
        |r| r.get::<_, i64>(0),
    ).unwrap_or(0) as usize
}
```

### 4. Output integration

- JSON: Add `reconciliation` key to the existing payload
- Human: Add reconciliation check to the check list, with per-connector detail on `warn`/`fail`
- The check inherits the existing `Check` struct pattern (name, status, message, fix_available)

### 5. Performance

Expected timing for ~10K sessions:
- `count_disk_files()` for all 12 connectors: ~1-2s (stat-only, no file reads)
- DB queries: <50ms (12 COUNT queries on indexed table)
- Total overhead to existing `cass doctor`: ~2s

## Files Changed

| File | Change |
|------|--------|
| `src/connectors/mod.rs` | Add `count_disk_files()` default method to `Connector` trait |
| `src/connectors/claude_code.rs` | Override `count_disk_files()` — count `.jsonl`/`.json`, skip settings |
| `src/connectors/gemini.rs` | Override — delegate to `session_files().len()` |
| `src/connectors/cursor.rs` | Override — count workspace SQLite DBs |
| `src/connectors/factory.rs` | Override — count `.jsonl`, skip `.settings.json` |
| `src/connectors/chatgpt.rs` | Override — count `.json` conversation files |
| `src/lib.rs` | Add reconciliation check to `run_doctor()`, add JSON output field |

Connectors not listed use the default `WalkDir` implementation (aider, amp, cline, codebuff, codex, opencode, pi_agent).

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| False positives from intentionally skipped files | Report delta as "unindexed" not "missing"; add notes field |
| Connector root detection failure masks real gaps | If `detect()` returns false, report `disk_files: 0, status: "skip"` |
| Performance regression on large installs | `count_disk_files()` is stat-only; add timing to JSON output |
| DB locked during reconciliation | Use read-only connection; tolerate query failure gracefully |
