use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::Value;
use walkdir::WalkDir;

use crate::connectors::{
    Connector, DetectionResult, NormalizedConversation, NormalizedMessage, ScanContext,
};

pub struct GeminiConnector;
impl Default for GeminiConnector {
    fn default() -> Self {
        Self::new()
    }
}

impl GeminiConnector {
    pub fn new() -> Self {
        Self
    }

    fn root() -> PathBuf {
        std::env::var("GEMINI_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| dirs::home_dir().unwrap_or_default().join(".gemini/tmp"))
    }

    /// Find all session JSON files in the Gemini structure.
    /// Structure: ~/.gemini/tmp/<hash>/chats/session-*.json
    fn session_files(root: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        for entry in WalkDir::new(root).into_iter().flatten() {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            // Only process session-*.json files in chats/ directories
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.starts_with("session-") && name.ends_with(".json") {
                // Verify it's in a chats/ directory
                if path
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    == Some("chats")
                {
                    files.push(path.to_path_buf());
                }
            }
        }
        files
    }
}

impl Connector for GeminiConnector {
    fn detect(&self) -> DetectionResult {
        let root = Self::root();
        if root.exists() {
            DetectionResult {
                detected: true,
                evidence: vec![format!("found {}", root.display())],
            }
        } else {
            DetectionResult::not_found()
        }
    }

    fn scan(&self, ctx: &ScanContext) -> Result<Vec<NormalizedConversation>> {
        // Use data_root only if it looks like a Gemini directory (for testing)
        // Otherwise use the default root
        let root = if ctx
            .data_root
            .file_name()
            .map(|n| n.to_str().unwrap_or("").contains("gemini"))
            .unwrap_or(false)
            || ctx.data_root.join("chats").exists()
            || fs::read_dir(&ctx.data_root)
                .map(|mut d| {
                    d.any(|e| {
                        e.ok()
                            .map(|e| e.path().join("chats").exists())
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false)
        {
            ctx.data_root.clone()
        } else {
            Self::root()
        };

        if !root.exists() {
            return Ok(Vec::new());
        }

        let files = Self::session_files(&root);
        let mut convs = Vec::new();

        for file in files {
            let content = fs::read_to_string(&file)
                .with_context(|| format!("read session {}", file.display()))?;

            let val: Value = match serde_json::from_str(&content) {
                Ok(v) => v,
                Err(_) => continue,
            };

            // Extract session metadata
            let session_id = val
                .get("sessionId")
                .and_then(|v| v.as_str())
                .map(String::from);
            let project_hash = val
                .get("projectHash")
                .and_then(|v| v.as_str())
                .map(String::from);

            // Parse session-level timestamps
            let start_time = val
                .get("startTime")
                .and_then(crate::connectors::parse_timestamp);
            let last_updated = val
                .get("lastUpdated")
                .and_then(crate::connectors::parse_timestamp);

            // Parse messages array
            let Some(messages_arr) = val.get("messages").and_then(|m| m.as_array()) else {
                continue;
            };

            let mut messages = Vec::new();
            let mut started_at = start_time;
            let mut ended_at = last_updated;

            for (idx, item) in messages_arr.iter().enumerate() {
                // Role from "type" field - Gemini uses "user" and "model"
                let msg_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("model");
                let role = if msg_type == "model" {
                    "assistant"
                } else {
                    msg_type
                };

                // Parse timestamp using shared utility
                let created = item
                    .get("timestamp")
                    .and_then(crate::connectors::parse_timestamp);
                started_at = started_at.or(created);
                ended_at = created.or(ended_at);

                // Extract content using flatten_content for consistency
                let content_str = item
                    .get("content")
                    .map(crate::connectors::flatten_content)
                    .unwrap_or_default();

                // Skip entries with empty content
                if content_str.trim().is_empty() {
                    continue;
                }

                messages.push(NormalizedMessage {
                    idx: idx as i64,
                    role: role.to_string(),
                    author: None,
                    created_at: created,
                    content: content_str,
                    extra: item.clone(),
                    snippets: Vec::new(),
                });
            }

            if messages.is_empty() {
                continue;
            }

            // Extract title from first user message
            let title = messages
                .iter()
                .find(|m| m.role == "user")
                .map(|m| {
                    m.content
                        .lines()
                        .next()
                        .unwrap_or(&m.content)
                        .chars()
                        .take(100)
                        .collect::<String>()
                })
                .or_else(|| {
                    messages
                        .first()
                        .and_then(|m| m.content.lines().next())
                        .map(|s| s.chars().take(100).collect())
                });

            // Try to get workspace from parent directory structure
            // Structure: ~/.gemini/tmp/<hash>/chats/session-*.json
            // The <hash> directory might correspond to a project path
            let workspace = file
                .parent() // chats/
                .and_then(|p| p.parent()) // <hash>/
                .map(|p| p.to_path_buf());

            convs.push(NormalizedConversation {
                agent_slug: "gemini".into(),
                external_id: session_id
                    .or_else(|| file.file_stem().and_then(|s| s.to_str()).map(String::from)),
                title,
                workspace,
                source_path: file.clone(),
                started_at,
                ended_at,
                metadata: serde_json::json!({
                    "source": "gemini",
                    "project_hash": project_hash
                }),
                messages,
            });
        }

        Ok(convs)
    }
}
