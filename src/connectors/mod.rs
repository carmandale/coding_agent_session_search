//! Connectors for agent histories.
//!
//! All connector implementations live in `franken_agent_detection`.
//! This module provides re-export stubs for backward-compatible import paths.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

// Re-export normalized types and connector infrastructure from franken_agent_detection.
pub use franken_agent_detection::{
    Connector,
    DetectionResult,
    DiscoveredSourceFile,
    DiscoveredSourceRole,
    ExtractedTokenUsage,
    LOCAL_SOURCE_ID,
    ModelInfo,
    // Scan & provenance types
    NormalizedConversation,
    NormalizedMessage,
    NormalizedSnippet,
    Origin,
    PathMapping,
    // Connector infrastructure
    PathTrie,
    Platform,
    ScanContext,
    ScanRoot,
    SourceKind,
    TokenDataSource,
    WorkspaceCache,
    estimate_tokens_from_content,
    extract_claude_code_tokens,
    extract_codex_tokens,
    extract_tokens_for_agent,
    file_modified_since,
    flatten_content,
    franken_detection_for_connector,
    normalize_model,
    parse_timestamp,
    reindex_messages,
};

/// Result of a Codex scan-root preflight. The preflight replaces directory
/// roots with explicit rollout files while preserving each root's provenance
/// and workspace rewrite metadata.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct CodexScanPreflight {
    pub scan_roots: Vec<ScanRoot>,
    pub original_roots: usize,
    pub explicit_file_roots: usize,
    pub fallback_roots: usize,
}

/// Expand Codex directory roots into explicit rollout-file roots where doing so
/// preserves Codex's session-relative external IDs.
///
/// Parent directories that contain a `.codex` child fall back to the original
/// directory root: `franken_agent_detection` includes `.codex/sessions/...` in
/// the external ID from that shape, while explicit file roots make the ID
/// relative to `sessions/`. Unreadable or ambiguous roots similarly fall back
/// so the connector's existing behavior remains the source of truth.
#[doc(hidden)]
#[must_use]
pub fn preflight_codex_explicit_file_roots(
    roots: &[ScanRoot],
    since_ts: Option<i64>,
) -> CodexScanPreflight {
    let mut scan_roots = Vec::new();
    let mut explicit_file_roots = 0usize;
    let mut fallback_roots = 0usize;

    for root in roots {
        if root.path.is_file() {
            if is_codex_rollout_file(&root.path) {
                explicit_file_roots = explicit_file_roots.saturating_add(1);
            }
            scan_roots.push(root.clone());
            continue;
        }

        match codex_explicit_file_roots_for_root(root, since_ts) {
            Ok(expanded) => {
                explicit_file_roots = explicit_file_roots.saturating_add(expanded.len());
                scan_roots.extend(expanded);
            }
            Err(_) => {
                fallback_roots = fallback_roots.saturating_add(1);
                scan_roots.push(root.clone());
            }
        }
    }

    CodexScanPreflight {
        scan_roots,
        original_roots: roots.len(),
        explicit_file_roots,
        fallback_roots,
    }
}

fn codex_explicit_file_roots_for_root(
    root: &ScanRoot,
    since_ts: Option<i64>,
) -> io::Result<Vec<ScanRoot>> {
    if !is_under_codex_dir(&root.path) && root.path.join(".codex").exists() {
        return Err(io::Error::other(
            "parent codex roots keep directory scan to preserve external IDs",
        ));
    }

    let sessions = codex_sessions_dir(&root.path);
    if sessions == root.path
        && root
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .is_none_or(|name| name != "sessions")
    {
        return Err(io::Error::other(
            "roots without a sessions directory keep directory scan to preserve external IDs",
        ));
    }

    let files = collect_codex_rollout_files(&sessions, since_ts)?;

    Ok(files
        .into_iter()
        .map(|path| {
            let mut file_root = root.clone();
            file_root.path = path;
            file_root
        })
        .collect())
}

fn is_under_codex_dir(path: &Path) -> bool {
    path.ancestors().any(|ancestor| {
        ancestor
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == ".codex")
    })
}

fn codex_sessions_dir(home: &Path) -> PathBuf {
    let sessions = home.join("sessions");
    if sessions.exists() {
        sessions
    } else {
        home.to_path_buf()
    }
}

fn collect_codex_rollout_files(sessions: &Path, since_ts: Option<i64>) -> io::Result<Vec<PathBuf>> {
    if !sessions.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    let mut pending_dirs = vec![sessions.to_path_buf()];
    while let Some(dir) = pending_dirs.pop() {
        let mut entries = fs::read_dir(&dir)?.collect::<io::Result<Vec<_>>>()?;
        entries.sort_by_key(|entry| entry.path());
        for entry in entries {
            let file_type = entry.file_type()?;
            let path = entry.path();
            if file_type.is_dir() {
                pending_dirs.push(path);
            } else if file_type.is_file()
                && is_codex_rollout_file(&path)
                && file_modified_since(&path, since_ts)
            {
                files.push(path);
            }
        }
    }

    files.sort();
    files.dedup();
    Ok(files)
}

fn is_codex_rollout_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    name.starts_with("rollout-")
        && path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| {
                ext.eq_ignore_ascii_case("jsonl") || ext.eq_ignore_ascii_case("json")
            })
}

// Connector re-export stubs — each module file re-exports from FAD.
pub mod aider;
pub mod amp;
pub mod chatgpt;
pub mod claude_code;
pub mod clawdbot;
pub mod cline;
pub mod codex;
pub mod copilot;
pub mod copilot_cli;
pub mod crush;
pub mod cursor;
pub mod factory;
pub mod gemini;
pub mod hermes;
pub mod kimi;
pub mod openclaw;
pub mod opencode;
pub mod pi_agent;
pub mod qwen;
pub mod vibe;

/// franken's connector factory table, with the codex entry replaced by this
/// crate's wrapper.
///
/// Every connector module here except `codex` is a bare `pub use` of franken's
/// implementation. `codex::CodexConnector` is the one that adds behavior: it
/// recovers rollouts written in the pre-envelope record shape, which franken's
/// `.jsonl` arm drops before `on_conversation` is ever called (bead 1pzs3).
///
/// Without this substitution that recovery was reachable only through
/// `ConnectorKind::create_connector`, which serves `--watch-once`. Everything
/// that builds the archive — `run_streaming_index` and `run_batch_index`, i.e.
/// `cass index` and `cass index --full` — goes through this table instead, so
/// it constructed franken's connector and indexed none of those rollouts while
/// exiting 0. Measured on the deployed binary before this change: the same 17
/// real rollouts gave 0 conversations through `cass index` and 17 (2,859
/// messages) through `cass index --watch-once`.
pub fn get_connector_factories() -> Vec<(&'static str, fn() -> Box<dyn Connector + Send>)> {
    let mut factories = franken_agent_detection::get_connector_factories();
    for (name, factory) in &mut factories {
        if *name == "codex" {
            *factory = || Box::new(codex::CodexConnector::new()) as Box<dyn Connector + Send>;
        }
    }
    factories
}

#[cfg(test)]
mod connector_factory_tests {
    use super::*;
    use tempfile::TempDir;

    /// A rollout in the pre-envelope record shape: the Responses-API item sits
    /// at the top level instead of inside a `payload` envelope, so franken's
    /// `.jsonl` arm drops the whole file.
    const PRE_ENVELOPE_ROLLOUT: &str = r#"{"id":"c27a914d","timestamp":"2025-08-20T13:20:47.060Z","instructions":null,"git":{"branch":"main"}}
{"type":"message","id":null,"role":"user","content":[{"type":"input_text","text":"index the pre-envelope rollouts"}]}
{"type":"message","id":null,"role":"assistant","content":[{"type":"output_text","text":"listed the directory"}]}
"#;

    fn codex_home_with_pre_envelope(dir: &TempDir) -> PathBuf {
        let home = dir.path().join("home").join(".codex");
        let day = home.join("sessions").join("2025").join("08").join("20");
        fs::create_dir_all(&day).unwrap();
        fs::write(day.join("rollout-pre-envelope.jsonl"), PRE_ENVELOPE_ROLLOUT).unwrap();
        home
    }

    fn scan_count(connector: &(dyn Connector + Send), ctx: &ScanContext) -> usize {
        let mut count = 0usize;
        connector
            .scan_with_callback(ctx, &mut |_conversation| {
                count += 1;
                Ok(())
            })
            .unwrap();
        count
    }

    fn codex_factory_from(
        factories: &[(&'static str, fn() -> Box<dyn Connector + Send>)],
    ) -> fn() -> Box<dyn Connector + Send> {
        factories
            .iter()
            .find(|(name, _)| *name == "codex")
            .map(|(_, factory)| *factory)
            .expect("the factory table must carry a connector under the slug `codex`")
    }

    /// The property, not the mechanism: a codex connector built the way the
    /// archive-building scan builds one must recover a pre-envelope rollout.
    ///
    /// This is what `cass index` and `cass index --full` actually do, and it is
    /// the assertion that was missing when 89db6723 landed — the recovery was
    /// tested by driving our wrapper directly, which is the path the full scan
    /// does not take.
    #[test]
    fn codex_connector_from_the_factory_table_recovers_pre_envelope_rollouts() {
        let dir = TempDir::new().unwrap();
        let home = codex_home_with_pre_envelope(&dir);
        let ctx = ScanContext::with_roots(
            dir.path().join("cass"),
            vec![ScanRoot::local(home)],
            None,
        );

        let factories = get_connector_factories();
        let connector = codex_factory_from(&factories)();

        assert_eq!(
            scan_count(connector.as_ref(), &ctx),
            1,
            "the connector the full scan builds must recover this rollout; \
             getting 0 here is bead 1pzs3 reaching the archive again"
        );
    }

    /// Why the substitution above has to exist. If this ever goes red, franken
    /// has learned the pre-envelope shape upstream and the wrapper should be
    /// re-adjudicated rather than kept out of habit.
    #[test]
    fn frankens_own_codex_factory_still_drops_pre_envelope_rollouts() {
        let dir = TempDir::new().unwrap();
        let home = codex_home_with_pre_envelope(&dir);
        let ctx = ScanContext::with_roots(
            dir.path().join("cass"),
            vec![ScanRoot::local(home)],
            None,
        );

        let upstream = franken_agent_detection::get_connector_factories();
        let connector = codex_factory_from(&upstream)();

        assert_eq!(
            scan_count(connector.as_ref(), &ctx),
            0,
            "franken's connector is expected to drop this shape; if it no longer \
             does, this crate's CodexConnector substitution may be redundant"
        );
    }
}
