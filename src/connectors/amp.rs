//! Connector for Amp (Sourcegraph) session logs — DISABLED.
//!
//! Amp scanning is disabled because:
//!  - No sessions since 2026-01-10 (83+ days)
//!  - Re-parsed 33 conversations / 264 messages every watcher cycle
//!    for zero new data, contributing to the WAL write load that triggers
//!    the frankensqlite MVCC FK mismatch (issue 26of).
//!
//! Re-enable by restoring `pub use franken_agent_detection::AmpConnector`
//! and removing this stub.

use crate::connectors::{Connector, DetectionResult, NormalizedConversation, ScanContext};

/// Stub connector — always reports not-found and returns no conversations.
pub struct AmpConnector;

impl AmpConnector {
    pub fn new() -> Self {
        Self
    }
}

impl Connector for AmpConnector {
    fn detect(&self) -> DetectionResult {
        DetectionResult::not_found()
    }

    fn scan(&self, _ctx: &ScanContext) -> anyhow::Result<Vec<NormalizedConversation>> {
        Ok(Vec::new())
    }
}
