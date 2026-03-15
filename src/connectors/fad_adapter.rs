//! Adapter layer for `franken-agent-detection` (FAD) connectors.
//!
//! Bridges FAD's 2-method `Connector` trait to our 4-method trait, converting
//! FAD types to our structurally-identical types. This lets us use FAD
//! connectors for agents we don't have in-tree (copilot, clawdbot, openclaw,
//! vibe) without rewriting connector logic.
//!
//! See: specs/006-fork-sync-upstream

use super::{
    Connector, DetectionResult, NormalizedConversation, NormalizedMessage, NormalizedSnippet,
    ScanContext,
};
use anyhow::Result;

/// Namespace alias for FAD types to avoid confusion with our identically-named types.
mod fad {
    pub use franken_agent_detection::connectors::Connector;
    pub use franken_agent_detection::{
        ClawdbotConnector, CopilotConnector, NormalizedConversation, NormalizedMessage,
        NormalizedSnippet, OpenClawConnector, ScanContext, ScanRoot, VibeConnector,
    };
}

// ---------------------------------------------------------------------------
// Type conversion: FAD → ours (field-by-field copy)
// ---------------------------------------------------------------------------

fn convert_snippet(s: fad::NormalizedSnippet) -> NormalizedSnippet {
    NormalizedSnippet {
        file_path: s.file_path,
        start_line: s.start_line,
        end_line: s.end_line,
        language: s.language,
        snippet_text: s.snippet_text,
    }
}

fn convert_message(m: fad::NormalizedMessage) -> NormalizedMessage {
    NormalizedMessage {
        idx: m.idx,
        role: m.role,
        author: m.author,
        created_at: m.created_at,
        content: m.content,
        extra: m.extra,
        snippets: m.snippets.into_iter().map(convert_snippet).collect(),
    }
}

fn convert_conversation(c: fad::NormalizedConversation) -> NormalizedConversation {
    NormalizedConversation {
        agent_slug: c.agent_slug,
        external_id: c.external_id,
        title: c.title,
        workspace: c.workspace,
        source_path: c.source_path,
        started_at: c.started_at,
        ended_at: c.ended_at,
        metadata: c.metadata,
        messages: c.messages.into_iter().map(convert_message).collect(),
    }
}

fn convert_scan_context(ctx: &ScanContext) -> fad::ScanContext {
    // For FAD connectors (copilot, clawdbot, openclaw, vibe), these are
    // local-only connectors that use default detection. We pass through
    // data_dir and since_ts; scan_roots are converted to FAD's ScanRoot.
    let scan_roots = ctx
        .scan_roots
        .iter()
        .map(|r| {
            if r.origin.is_local() {
                fad::ScanRoot::local(r.path.clone())
            } else {
                let fad_origin = franken_agent_detection::Origin {
                    source_id: r.origin.source_id.clone(),
                    kind: match r.origin.kind {
                        crate::sources::provenance::SourceKind::Local => {
                            franken_agent_detection::SourceKind::Local
                        }
                        crate::sources::provenance::SourceKind::Ssh => {
                            franken_agent_detection::SourceKind::Ssh
                        }
                    },
                    host: r.origin.host.clone(),
                };
                let fad_platform = r.platform.map(|p| match p {
                    crate::connectors::Platform::Macos => franken_agent_detection::Platform::Macos,
                    crate::connectors::Platform::Linux => franken_agent_detection::Platform::Linux,
                    crate::connectors::Platform::Windows => {
                        franken_agent_detection::Platform::Windows
                    }
                });
                fad::ScanRoot::remote(r.path.clone(), fad_origin, fad_platform)
            }
        })
        .collect();

    fad::ScanContext {
        data_dir: ctx.data_dir.clone(),
        scan_roots,
        since_ts: ctx.since_ts,
    }
}

fn convert_detection(d: franken_agent_detection::DetectionResult) -> DetectionResult {
    DetectionResult {
        detected: d.detected,
        evidence: d.evidence,
        root_paths: d.root_paths,
    }
}

// ---------------------------------------------------------------------------
// Generic adapter: wraps any FAD connector as our Connector trait
// ---------------------------------------------------------------------------

/// Wraps a FAD connector to implement our 4-method `Connector` trait.
/// `count_disk_files()` and `reconciliation_notes()` return `None` since
/// FAD's trait doesn't have these methods.
pub struct FadAdapter<T: fad::Connector + Send> {
    inner: T,
}

impl<T: fad::Connector + Send> FadAdapter<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T: fad::Connector + Send> Connector for FadAdapter<T> {
    fn detect(&self) -> DetectionResult {
        convert_detection(self.inner.detect())
    }

    fn scan(&self, ctx: &ScanContext) -> Result<Vec<NormalizedConversation>> {
        let fad_ctx = convert_scan_context(ctx);
        let fad_convs = self.inner.scan(&fad_ctx)?;
        Ok(fad_convs.into_iter().map(convert_conversation).collect())
    }

    fn count_disk_files(&self) -> Option<usize> {
        None
    }

    fn reconciliation_notes(&self) -> Option<String> {
        None
    }
}

// ---------------------------------------------------------------------------
// Convenience constructors for the 4 new FAD connectors
// ---------------------------------------------------------------------------

pub fn copilot() -> Box<dyn Connector + Send> {
    Box::new(FadAdapter::new(fad::CopilotConnector::new()))
}

pub fn clawdbot() -> Box<dyn Connector + Send> {
    Box::new(FadAdapter::new(fad::ClawdbotConnector::new()))
}

pub fn openclaw() -> Box<dyn Connector + Send> {
    Box::new(FadAdapter::new(fad::OpenClawConnector::new()))
}

pub fn vibe() -> Box<dyn Connector + Send> {
    Box::new(FadAdapter::new(fad::VibeConnector::new()))
}
