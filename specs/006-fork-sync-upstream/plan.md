---
title: "plan: Fork sync — Shape D hybrid FAD port"
date: 2026-03-15
bead: coding_agent_session_search-3fir
---

# Implementation Plan: Shape D — Hybrid FAD Port

## Overview

Add `franken-agent-detection` (FAD) as a git dependency to gain 4 new
connectors: **copilot** (GitHub Copilot Chat), **clawdbot**, **openclaw**,
and **vibe** (Mistral). Keep all existing in-tree connectors unchanged,
including our pi_agent with the spec-005 watcher fix.

## Architecture

### Current state

Our `src/connectors/mod.rs` defines:
- `Connector` trait (4 methods: `detect`, `scan`, `count_disk_files`, `reconciliation_notes`)
- `NormalizedConversation`, `NormalizedMessage`, `DetectionResult`, `ScanContext` types
- 12 in-tree connector implementations

### Target state

- FAD added as dependency (rev `5b0eb1a`, features `["connectors"]`)
- FAD's dep chain is clean: anyhow, serde_json, walkdir, tracing, dotenvy, bloomfilter, once_cell — all already in our Cargo.toml except bloomfilter (lightweight)
- 4 adapter structs that wrap FAD's 2-method `Connector` with our 4-method trait
- `get_connector_factories()` grows from 12 to 16 entries
- `ConnectorKind` enum grows by 4 variants

### Type bridge: Adapter pattern

FAD's types are field-identical to ours but they're different Rust types
from different crates. The adapter pattern converts:

```rust
/// Adapter: wraps a FAD connector to implement our 4-method Connector trait.
struct FadAdapter<T: franken_agent_detection::Connector + Send>(T);

impl<T: franken_agent_detection::Connector + Send> Connector for FadAdapter<T> {
    fn detect(&self) -> DetectionResult {
        let fad_result = self.0.detect();
        DetectionResult {
            detected: fad_result.detected,
            evidence: fad_result.evidence,
            root_paths: fad_result.root_paths,
        }
    }

    fn scan(&self, ctx: &ScanContext) -> Result<Vec<NormalizedConversation>> {
        let fad_ctx = franken_agent_detection::ScanContext {
            data_dir: ctx.data_dir.clone(),
            scan_roots: ctx.scan_roots.iter().map(|r| /* convert ScanRoot */).collect(),
            since_ts: ctx.since_ts,
        };
        let fad_convs = self.0.scan(&fad_ctx)?;
        Ok(fad_convs.into_iter().map(convert_conversation).collect())
    }

    fn count_disk_files(&self) -> Option<usize> { None }
    fn reconciliation_notes(&self) -> Option<String> { None }
}
```

The `convert_conversation` and `convert_message` functions are mechanical
field-by-field copies since the types are structurally identical.

### Registration

**File:** `src/indexer/mod.rs`

Add to `get_connector_factories()` (line 776):
```rust
("copilot", || Box::new(FadAdapter(fad::CopilotConnector::new()))),
("clawdbot", || Box::new(FadAdapter(fad::ClawdbotConnector::new()))),
("openclaw", || Box::new(FadAdapter(fad::OpenClawConnector::new()))),
("vibe", || Box::new(FadAdapter(fad::VibeConnector::new()))),
```

Add to `ConnectorKind` enum:
```rust
Copilot,
Clawdbot,
OpenClaw,
Vibe,
```

Add to `ConnectorKind::from_slug()` and `create_connector()` match arms.

### ScanRoot conversion

FAD's `ScanRoot` may differ from ours. Our `ScanRoot` has:
- `path: PathBuf`
- `origin: Origin`
- `platform: Option<Platform>`
- `workspace_rewrites: Vec<WorkspaceRewrite>`
- `rewrite_trie: OnceCell<PathTrie>`

FAD's `ScanRoot` may be simpler. The adapter needs to convert our
`ScanRoot` to FAD's format. If FAD's `ScanContext` only needs `data_dir`
and `since_ts` for these connectors (which are simple local-only scans),
the `scan_roots` conversion may be trivial.

## Files to modify

| File | Change | Lines |
|------|--------|-------|
| `Cargo.toml` | Add `franken-agent-detection` git dep | ~2 |
| `src/connectors/mod.rs` | Add `pub mod fad_adapter;` | ~1 |
| `src/connectors/fad_adapter.rs` | New file: adapter + convert functions | ~150 |
| `src/indexer/mod.rs` | Register 4 new connectors in factory + enum | ~30 |

**Total: ~180 lines of new code, 1 new file.**

## Verification

- `cargo check` — compiles with new FAD dependency
- `cargo test --lib` — all existing tests pass
- `cargo clippy --all-targets -- -D warnings` — clean
- Manual: run `cass index --full` and verify copilot/clawdbot/openclaw/vibe sessions appear in `cass stats`
