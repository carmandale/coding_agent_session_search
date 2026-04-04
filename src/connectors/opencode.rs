//! Connector for OpenCode session logs — DISABLED.
//!
//! OpenCode scanning is disabled because:
//!  - No sessions since 2026-01-17 (2.5 months)
//!  - Its ~3 second SQLite scan was always the last connector to finish,
//!    triggering a frankensqlite OOM (issue 26of) in the cleanup phase
//!    and crash-looping the watcher every 2 minutes.
//!
//! Re-enable by restoring "opencode" in the FAD features (Cargo.toml)
//! and un-commenting `pub mod opencode` in connectors/mod.rs.

use crate::connectors::{Connector, DetectionResult, NormalizedConversation, ScanContext};

/// Stub connector — always reports not-found and returns no conversations.
pub struct OpenCodeConnector;

impl OpenCodeConnector {
    pub fn new() -> Self {
        Self
    }
}

impl Connector for OpenCodeConnector {
    fn detect(&self) -> DetectionResult {
        DetectionResult::not_found()
    }

    fn scan(&self, _ctx: &ScanContext) -> anyhow::Result<Vec<NormalizedConversation>> {
        Ok(Vec::new())
    }
}
