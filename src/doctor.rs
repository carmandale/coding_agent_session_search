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
/// `supports_streaming_scan` that exist in upstream's private FAD fork
/// but not in the public v0.1.3 release.
///
/// Not blanket-implemented over Connector to avoid conflicts.
/// Call via `connector_scan_with_callback(conn, ctx, cb)` free function
/// OR implement explicitly for types that need custom streaming behavior.
pub trait ConnectorExt {
    fn scan_with_callback(
        &self,
        ctx: &ScanContext,
        on_conversation: &mut dyn FnMut(NormalizedConversation) -> anyhow::Result<()>,
    ) -> anyhow::Result<()>;

    fn supports_streaming_scan(&self) -> bool {
        false
    }
}

/// Default scan_with_callback implementation via scan() for any Connector.
/// Used as a free function by the indexer when a connector doesn't override ConnectorExt.
pub fn connector_scan_with_callback<C: Connector + ?Sized>(
    conn: &C,
    ctx: &ScanContext,
    on_conversation: &mut dyn FnMut(NormalizedConversation) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let conversations = conn.scan(ctx)?;
    for conv in conversations {
        on_conversation(conv)?;
    }
    Ok(())
}
