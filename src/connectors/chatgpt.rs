//! Connector for ChatGPT desktop app conversation history.
//!
//! ChatGPT stores conversations in:
//! - macOS: ~/Library/Application Support/com.openai.chat/
//!
//! Conversation storage versions:
//! - v1 (legacy): Plain JSON files in conversations-{uuid}/ (unencrypted)
//! - v2: Encrypted files in conversations-v2-{uuid}/ (uses keychain)
//! - v3: Encrypted files in conversations-v3-{uuid}/ (uses keychain)
//!
//! NOTE: v2/v3 files are encrypted using a key stored in macOS Keychain.
//! This connector can only read unencrypted v1 files. Encrypted versions
//! would require user authorization to access the keychain.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde_json::Value;
use walkdir::WalkDir;

use crate::connectors::{
    Connector, DetectionResult, NormalizedConversation, NormalizedMessage, ScanContext,
};

pub struct ChatGptConnector;

impl Default for ChatGptConnector {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatGptConnector {
    pub fn new() -> Self {
        Self
    }

    /// Get the ChatGPT app support directory
    pub fn app_support_dir() -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            dirs::home_dir().map(|h| h.join("Library/Application Support/com.openai.chat"))
        }
        #[cfg(not(target_os = "macos"))]
        {
            // ChatGPT desktop is currently macOS only
            None
        }
    }

    /// Find conversation directories (both encrypted and unencrypted)
    fn find_conversation_dirs(base: &PathBuf) -> Vec<(PathBuf, bool)> {
        let mut dirs = Vec::new();

        if !base.exists() {
            return dirs;
        }

        for entry in fs::read_dir(base).into_iter().flatten().flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // Check for conversation directories
            if name.starts_with("conversations-") {
                // v1 (unencrypted) or v2/v3 (encrypted)
                let is_encrypted = name.contains("-v2-") || name.contains("-v3-");
                dirs.push((path, is_encrypted));
            }
        }

        dirs
    }

    /// Parse a conversation file (JSON or data format)
    fn parse_conversation_file(
        path: &PathBuf,
        since_ts: Option<i64>,
    ) -> Result<Option<NormalizedConversation>> {
        let content =
            fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;

        let val: Value = serde_json::from_str(&content)
            .with_context(|| format!("parse JSON from {}", path.display()))?;

        let mut messages = Vec::new();
        let mut started_at = None;
        let mut ended_at = None;

        // Extract conversation ID
        let conv_id = val
            .get("id")
            .or_else(|| val.get("conversation_id"))
            .and_then(|v| v.as_str())
            .or_else(|| path.file_stem().and_then(|s| s.to_str()))
            .map(String::from);

        // Extract title
        let title = val.get("title").and_then(|v| v.as_str()).map(String::from);

        // Parse messages from mapping structure (ChatGPT format)
        if let Some(mapping) = val.get("mapping").and_then(|v| v.as_object()) {
            // Collect messages with their parent info for ordering
            let mut msg_nodes: Vec<(Option<String>, String, &Value)> = Vec::new();

            for (node_id, node) in mapping {
                if let Some(msg) = node.get("message") {
                    let parent = node
                        .get("parent")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    msg_nodes.push((parent, node_id.clone(), msg));
                }
            }

            // Simple ordering: sort by create_time if available
            msg_nodes.sort_by(|a, b| {
                let ts_a =
                    a.2.get("create_time")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                let ts_b =
                    b.2.get("create_time")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                ts_a.partial_cmp(&ts_b).unwrap_or(std::cmp::Ordering::Equal)
            });

            for (_, _, msg) in msg_nodes {
                // Get role
                let role = msg
                    .get("author")
                    .and_then(|a| a.get("role"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("assistant");

                // Skip system messages
                if role == "system" {
                    continue;
                }

                // Get content
                let content_val = msg.get("content");
                let content_str = if let Some(parts) = content_val
                    .and_then(|c| c.get("parts"))
                    .and_then(|p| p.as_array())
                {
                    parts
                        .iter()
                        .filter_map(|p| p.as_str())
                        .collect::<Vec<_>>()
                        .join("\n")
                } else if let Some(text) = content_val
                    .and_then(|c| c.get("text"))
                    .and_then(|t| t.as_str())
                {
                    text.to_string()
                } else {
                    continue;
                };

                if content_str.trim().is_empty() {
                    continue;
                }

                // Get timestamp (ChatGPT uses float seconds)
                let created_at = msg
                    .get("create_time")
                    .and_then(|v| v.as_f64())
                    .map(|ts| (ts * 1000.0) as i64);

                // Apply since_ts filter
                if let (Some(since), Some(ts)) = (since_ts, created_at)
                    && ts <= since
                {
                    continue;
                }

                if started_at.is_none() {
                    started_at = created_at;
                }
                ended_at = created_at;

                // Get model info
                let model = msg
                    .get("metadata")
                    .and_then(|m| m.get("model_slug"))
                    .and_then(|v| v.as_str())
                    .map(String::from);

                messages.push(NormalizedMessage {
                    idx: messages.len() as i64,
                    role: role.to_string(),
                    author: model,
                    created_at,
                    content: content_str,
                    extra: msg.clone(),
                    snippets: Vec::new(),
                });
            }
        }

        // Also try simple messages array format
        if messages.is_empty()
            && let Some(msgs) = val.get("messages").and_then(|v| v.as_array())
        {
            for item in msgs {
                let role = item
                    .get("role")
                    .and_then(|v| v.as_str())
                    .unwrap_or("assistant");

                if role == "system" {
                    continue;
                }

                let content = item.get("content").and_then(|v| v.as_str()).unwrap_or("");

                if content.trim().is_empty() {
                    continue;
                }

                let created_at = item
                    .get("timestamp")
                    .or_else(|| item.get("create_time"))
                    .and_then(crate::connectors::parse_timestamp);

                if let (Some(since), Some(ts)) = (since_ts, created_at)
                    && ts <= since
                {
                    continue;
                }

                if started_at.is_none() {
                    started_at = created_at;
                }
                ended_at = created_at;

                messages.push(NormalizedMessage {
                    idx: messages.len() as i64,
                    role: role.to_string(),
                    author: None,
                    created_at,
                    content: content.to_string(),
                    extra: item.clone(),
                    snippets: Vec::new(),
                });
            }
        }

        if messages.is_empty() {
            return Ok(None);
        }

        Ok(Some(NormalizedConversation {
            agent_slug: "chatgpt".to_string(),
            external_id: conv_id,
            title,
            workspace: None, // ChatGPT doesn't have workspace concept
            source_path: path.clone(),
            started_at,
            ended_at,
            metadata: serde_json::json!({
                "source": "chatgpt_desktop",
                "model": val.get("model").and_then(|v| v.as_str()),
            }),
            messages,
        }))
    }
}

impl Connector for ChatGptConnector {
    fn detect(&self) -> DetectionResult {
        if let Some(base) = Self::app_support_dir()
            && base.exists()
        {
            let conv_dirs = Self::find_conversation_dirs(&base);
            if !conv_dirs.is_empty() {
                let encrypted_count = conv_dirs.iter().filter(|(_, enc)| *enc).count();
                let unencrypted_count = conv_dirs.len() - encrypted_count;

                let mut evidence = vec![format!("found ChatGPT at {}", base.display())];

                if unencrypted_count > 0 {
                    evidence.push(format!(
                        "{} unencrypted conversation dir(s) (readable)",
                        unencrypted_count
                    ));
                }
                if encrypted_count > 0 {
                    evidence.push(format!(
                        "{} encrypted conversation dir(s) (v2/v3, requires keychain)",
                        encrypted_count
                    ));
                }

                return DetectionResult {
                    detected: true,
                    evidence,
                };
            }
        }
        DetectionResult::not_found()
    }

    fn scan(&self, ctx: &ScanContext) -> Result<Vec<NormalizedConversation>> {
        // Determine base directory
        let base = if ctx
            .data_root
            .file_name()
            .is_some_and(|n| n.to_str().unwrap_or("").contains("openai"))
            || ctx.data_root.join("conversations-").exists()
        {
            ctx.data_root.clone()
        } else if let Some(default_base) = Self::app_support_dir() {
            default_base
        } else {
            return Ok(Vec::new());
        };

        if !base.exists() {
            return Ok(Vec::new());
        }

        let conv_dirs = Self::find_conversation_dirs(&base);
        let mut all_convs = Vec::new();

        for (dir_path, is_encrypted) in conv_dirs {
            if is_encrypted {
                // Skip encrypted directories with a warning
                tracing::debug!(
                    path = %dir_path.display(),
                    "chatgpt skipping encrypted conversation directory (v2/v3)"
                );
                continue;
            }

            // Walk through unencrypted conversation files
            for entry in WalkDir::new(&dir_path).max_depth(1).into_iter().flatten() {
                if !entry.file_type().is_file() {
                    continue;
                }

                let path = entry.path();
                let ext = path.extension().and_then(|s| s.to_str());

                // Look for JSON or data files
                if ext != Some("json") && ext != Some("data") {
                    continue;
                }

                // Skip files not modified since last scan
                if !crate::connectors::file_modified_since(path, ctx.since_ts) {
                    continue;
                }

                match Self::parse_conversation_file(&path.to_path_buf(), ctx.since_ts) {
                    Ok(Some(conv)) => {
                        tracing::debug!(
                            path = %path.display(),
                            messages = conv.messages.len(),
                            "chatgpt extracted conversation"
                        );
                        all_convs.push(conv);
                    }
                    Ok(None) => {
                        tracing::debug!(
                            path = %path.display(),
                            "chatgpt no messages in conversation"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            path = %path.display(),
                            error = %e,
                            "chatgpt failed to parse conversation"
                        );
                    }
                }
            }
        }

        Ok(all_convs)
    }
}
