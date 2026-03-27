//! DoctorConnector extension trait for disk-vs-DB reconciliation,
//! and compatibility shims for newer Connector trait methods not yet
//! in the public franken_agent_detection releases.

use crate::connectors::{Connector, ScanContext};
use crate::connectors::NormalizedConversation;

/// Extension trait for connectors that support disk-vs-DB reconciliation.
/// Implemented by connectors where we can count source files on disk.
pub trait DoctorConnector {
    /// Count how many session files exist on disk for this connector.
    /// Returns None if not applicable (e.g. FAD-backed connectors).
    fn count_disk_files(&self) -> Option<usize>;

    /// Optional contextual notes shown in doctor reconciliation output.
    fn reconciliation_notes(&self) -> Option<String> {
        None
    }
}

/// Compatibility extension trait — adds `scan_with_callback` and
/// `supports_streaming_scan` methods that exist in upstream's private FAD
/// fork but not in the public v0.1.3 release.
///
/// Implemented as a blanket impl over all `Connector + Sync` types.
/// Uses the standard `scan()` method internally (non-streaming fallback).
pub trait ConnectorExt: Connector {
    /// Streaming scan callback variant. Falls back to `scan()` + callback iteration.
    fn scan_with_callback(
        &self,
        ctx: &ScanContext,
        on_conversation: &mut dyn FnMut(NormalizedConversation) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        let conversations = self.scan(ctx)?;
        for conv in conversations {
            on_conversation(conv)?;
        }
        Ok(())
    }

    /// Returns true if this connector natively supports streaming scan.
    /// FAD public connectors all use buffered scan, so this returns false.
    fn supports_streaming_scan(&self) -> bool {
        false
    }
}

// Blanket impl for all FAD connectors
impl<T: Connector + ?Sized> ConnectorExt for T {}
