//! Local Crush connector wrapper.
//!
//! The actual parsing/detection logic comes from `franken-agent-detection`, but
//! this wrapper keeps Crush participating in cass through the repo-local
//! `Connector` trait and module layout.

use anyhow::Result;

use crate::connectors::{Connector, DetectionResult, NormalizedConversation, ScanContext};

pub struct CrushConnector {
    inner: Box<dyn Connector + Send>,
}

impl Default for CrushConnector {
    fn default() -> Self {
        Self::new()
    }
}

impl CrushConnector {
    pub fn new() -> Self {
        Self {
            inner: crate::connectors::fad_adapter::crush(),
        }
    }
}

impl Connector for CrushConnector {
    fn detect(&self) -> DetectionResult {
        self.inner.detect()
    }

    fn scan(&self, ctx: &ScanContext) -> Result<Vec<NormalizedConversation>> {
        self.inner.scan(ctx)
    }

    fn count_disk_files(&self) -> Option<usize> {
        self.inner.count_disk_files()
    }

    fn reconciliation_notes(&self) -> Option<String> {
        self.inner.reconciliation_notes()
    }
}
