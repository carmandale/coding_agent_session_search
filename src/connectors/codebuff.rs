//! Codebuff (manicode) connector for cass.
//!
//! Codebuff stores sessions in `~/.config/manicode/projects/{project}/chats/{timestamp}/`
//! with conversation data in `chat-messages.json`.

use std::path::PathBuf;

use anyhow::Result;
use serde_json::Value;
use walkdir::WalkDir;

use crate::connectors::{
    Connector, DetectionResult, NormalizedConversation, NormalizedMessage, ScanContext,
};

pub struct CodebuffConnector;

impl Default for CodebuffConnector {
    fn default() -> Self {
        Self::new()
    }
}

impl CodebuffConnector {
    pub fn new() -> Self {
        Self
    }

    /// Returns the Codebuff config root directory.
    /// Codebuff uses the "manicode" name internally and stores data in ~/.config/manicode
    /// on all platforms (Linux-style XDG path, even on macOS).
    fn config_root() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".config/manicode")
    }

    /// Returns the projects directory where chat sessions are stored.
    fn projects_root() -> PathBuf {
        Self::config_root().join("projects")
    }

    pub fn candidate_roots() -> Vec<PathBuf> {
        vec![Self::projects_root()]
    }
}

impl Connector for CodebuffConnector {
    fn detect(&self) -> DetectionResult {
        let evidence: Vec<String> = Self::candidate_roots()
            .into_iter()
            .filter(|r| r.exists())
            .map(|r| format!("found {}", r.display()))
            .collect();

        if evidence.is_empty() {
            DetectionResult::not_found()
        } else {
            DetectionResult {
                detected: true,
                evidence,
                root_paths: vec![],
            }
        }
    }

    fn scan(&self, ctx: &ScanContext) -> Result<Vec<NormalizedConversation>> {
        let mut convs = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        // Allow tests to override via ctx.data_dir
        let data_root = &ctx.data_dir;
        let roots = if data_root
            .file_name()
            .is_some_and(|n| n.to_str().unwrap_or("").contains("manicode"))
            || std::fs::read_dir(data_root)
                .map(|mut d| {
                    d.any(|e| {
                        e.ok().is_some_and(|e| {
                            e.path()
                                .file_name()
                                .is_some_and(|n| n == "chat-messages.json")
                        })
                    })
                })
                .unwrap_or(false)
        {
            vec![data_root.clone()]
        } else {
            Self::candidate_roots()
        };

        for root in roots {
            if !root.exists() {
                continue;
            }

            for entry in WalkDir::new(&root).into_iter().flatten() {
                if !entry.file_type().is_file() {
                    continue;
                }
                let path = entry.path();

                // Only process chat-messages.json files
                if path.file_name().and_then(|n| n.to_str()) != Some("chat-messages.json") {
                    continue;
                }

                // Skip files not modified since last scan (incremental indexing)
                if !crate::connectors::file_modified_since(path, ctx.since_ts) {
                    continue;
                }

                let text = match std::fs::read_to_string(path) {
                    Ok(t) => t,
                    Err(_) => continue,
                };

                let val: Value = match serde_json::from_str(&text) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                if let Some(messages) = extract_messages(&val) {
                    if messages.is_empty() {
                        continue;
                    }

                    // Extract project name and session timestamp from path
                    // Path format: projects/{project}/chats/{timestamp}/chat-messages.json
                    let (project_name, session_id) = extract_path_info(path);

                    let workspace = infer_workspace(&val, path);

                    let title = project_name.clone().or_else(|| {
                        messages
                            .first()
                            .and_then(|m| m.content.lines().next())
                            .map(|s| s.chars().take(80).collect())
                    });

                    let external_id = session_id.clone().or_else(|| {
                        path.parent()
                            .and_then(|p| p.file_name())
                            .and_then(|s| s.to_str())
                            .map(std::string::ToString::to_string)
                    });

                    let key = external_id.clone().map_or_else(
                        || format!("codebuff:{}", path.display()),
                        |id| format!("codebuff:{id}"),
                    );

                    if seen_ids.insert(key) {
                        convs.push(NormalizedConversation {
                            agent_slug: "codebuff".into(),
                            external_id,
                            title,
                            workspace,
                            source_path: path.to_path_buf(),
                            started_at: messages.first().and_then(|m| m.created_at),
                            ended_at: messages.last().and_then(|m| m.created_at),
                            metadata: val.clone(),
                            messages,
                        });
                        tracing::info!(
                            target: "connector::codebuff",
                            source = %path.display(),
                            messages = convs.last().map_or(0, |c| c.messages.len()),
                            since_ts = ctx.since_ts,
                            "codebuff_scan"
                        );
                    }
                }
            }
        }

        Ok(convs)
    }
}

/// Extract messages from Codebuff chat-messages.json format.
///
/// Codebuff messages have structure:
/// ```json
/// [
///   {
///     "id": "ai-1766137957398-bf2216fd8cd09",
///     "variant": "ai",
///     "content": "...",
///     "blocks": [...],
///     "timestamp": "03:52 AM",
///     "metadata": {...}
///   }
/// ]
/// ```
fn extract_messages(val: &Value) -> Option<Vec<NormalizedMessage>> {
    let msgs = val.as_array()?;

    let mut out = Vec::new();
    for m in msgs {
        // Get role from "variant" field (ai/human)
        let variant = m
            .get("variant")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let role = match variant {
            "ai" => "assistant".to_string(),
            "human" => "user".to_string(),
            other => other.to_string(),
        };

        // Extract content from multiple sources
        let content = extract_content(m);

        if content.trim().is_empty() {
            continue;
        }

        // Parse timestamp - Codebuff uses various formats
        let created_at = parse_codebuff_timestamp(m);

        let author = m
            .get("author")
            .or_else(|| m.get("agentName"))
            .and_then(|v| v.as_str())
            .map(std::string::ToString::to_string);

        out.push(NormalizedMessage {
            idx: 0, // Will be re-assigned after collection
            role,
            author,
            created_at,
            content,
            extra: m.clone(),
            snippets: Vec::new(),
        });
    }

    // Re-assign indices to maintain sequential order
    for (i, msg) in out.iter_mut().enumerate() {
        msg.idx = i as i64;
    }

    if out.is_empty() { None } else { Some(out) }
}

/// Extract content from a Codebuff message.
///
/// Content can be in:
/// - Top-level "content" field
/// - Nested in "blocks" array with type "text"
/// - Tool calls in blocks with type "tool"
fn extract_content(msg: &Value) -> String {
    let mut parts = Vec::new();

    // Direct content field
    if let Some(content) = msg.get("content").and_then(|v| v.as_str())
        && !content.is_empty()
    {
        parts.push(content.to_string());
    }

    // Extract from blocks array
    if let Some(blocks) = msg.get("blocks").and_then(|b| b.as_array()) {
        for block in blocks {
            extract_block_content(block, &mut parts);
        }
    }

    parts.join("\n")
}

/// Recursively extract content from a block.
fn extract_block_content(block: &Value, parts: &mut Vec<String>) {
    let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match block_type {
        "text" => {
            if let Some(text) = block.get("content").and_then(|v| v.as_str())
                && !text.is_empty()
            {
                parts.push(text.to_string());
            }
        }
        "tool" => {
            // Include tool information for searchability
            let tool_name = block
                .get("toolName")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            // Include tool input for context (e.g., commands, file paths)
            if let Some(input) = block.get("input") {
                if let Some(command) = input.get("command").and_then(|v| v.as_str()) {
                    parts.push(format!("[Tool: {tool_name}] {command}"));
                } else if let Some(path) = input.get("path").and_then(|v| v.as_str()) {
                    parts.push(format!("[Tool: {tool_name}] {path}"));
                } else {
                    parts.push(format!("[Tool: {tool_name}]"));
                }
            }

            // Include tool output
            if let Some(output) = block.get("output").and_then(|v| v.as_str()) {
                // Truncate very long outputs safely (respecting UTF-8 boundaries)
                let truncated = if output.len() > 1000 {
                    // Find a valid character boundary at or before 1000 bytes
                    let mut end = 1000;
                    while !output.is_char_boundary(end) && end > 0 {
                        end -= 1;
                    }
                    format!("{}... [truncated]", &output[..end])
                } else {
                    output.to_string()
                };
                if !truncated.is_empty() {
                    parts.push(truncated);
                }
            }
        }
        "agent" => {
            // Agent blocks can contain nested blocks
            if let Some(agent_content) = block.get("content").and_then(|v| v.as_str())
                && !agent_content.is_empty()
            {
                parts.push(agent_content.to_string());
            }
            // Process nested blocks within agent block
            if let Some(nested_blocks) = block.get("blocks").and_then(|b| b.as_array()) {
                for nested in nested_blocks {
                    extract_block_content(nested, parts);
                }
            }
        }
        _ => {
            // For unknown block types, try to extract any content field
            if let Some(content) = block.get("content").and_then(|v| v.as_str())
                && !content.is_empty()
            {
                parts.push(content.to_string());
            }
        }
    }
}

/// Parse timestamp from Codebuff message.
///
/// Codebuff uses various timestamp formats:
/// - Session directory name: "2025-12-19T09-07-21.203Z" (ISO-8601 with hyphens instead of colons)
/// - Message timestamp field: "03:52 AM" (12-hour time)
/// - Metadata timestamps: Unix milliseconds or ISO-8601
fn parse_codebuff_timestamp(msg: &Value) -> Option<i64> {
    // Try metadata timestamps first
    if let Some(metadata) = msg.get("metadata")
        && let Some(ts) = metadata
            .get("timestamp")
            .or_else(|| metadata.get("createdAt"))
            .or_else(|| metadata.get("created_at"))
        && let Some(parsed) = crate::connectors::parse_timestamp(ts)
    {
        return Some(parsed);
    }

    // Try direct timestamp fields
    if let Some(ts) = msg
        .get("timestamp")
        .or_else(|| msg.get("createdAt"))
        .or_else(|| msg.get("created_at"))
        && let Some(parsed) = crate::connectors::parse_timestamp(ts)
    {
        return Some(parsed);
    }

    // Try parsing from message ID (contains timestamp)
    // Format: "ai-1766137957398-bf2216fd8cd09" where middle part might be a timestamp
    if let Some(id) = msg.get("id").and_then(|v| v.as_str()) {
        let parts: Vec<&str> = id.split('-').collect();
        if parts.len() >= 2
            && let Ok(ts) = parts[1].parse::<i64>()
            // Check if it looks like milliseconds (13 digits, year 2020+)
            && ts > 1_577_836_800_000
            && ts < 2_000_000_000_000
        {
            return Some(ts);
        }
    }

    None
}

/// Extract project name and session ID from file path.
///
/// Path format: `projects/{project}/chats/{timestamp}/chat-messages.json`
fn extract_path_info(path: &std::path::Path) -> (Option<String>, Option<String>) {
    let components: Vec<_> = path.components().collect();
    let len = components.len();

    let mut project_name = None;
    let mut session_id = None;

    // Find "projects" in path and extract project name
    for (i, comp) in components.iter().enumerate() {
        if let std::path::Component::Normal(name) = comp {
            if name.to_str() == Some("projects")
                && i + 1 < len
                && let std::path::Component::Normal(proj) = components[i + 1]
            {
                project_name = proj.to_str().map(std::string::ToString::to_string);
            }
            if name.to_str() == Some("chats")
                && i + 1 < len
                && let std::path::Component::Normal(sess) = components[i + 1]
            {
                session_id = sess.to_str().map(std::string::ToString::to_string);
            }
        }
    }

    (project_name, session_id)
}

/// Infer workspace from message metadata or file path.
fn infer_workspace(val: &Value, path: &std::path::Path) -> Option<PathBuf> {
    // Try to find workspace in message metadata
    if let Some(msgs) = val.as_array() {
        for msg in msgs {
            if let Some(metadata) = msg.get("metadata")
                && let Some(run_state) = metadata.get("runState")
                && let Some(session_state) = run_state.get("sessionState")
                && let Some(file_context) = session_state.get("fileContext")
            {
                // Try projectRoot first, then cwd
                if let Some(root) = file_context.get("projectRoot").and_then(|v| v.as_str()) {
                    return Some(PathBuf::from(root));
                }
                if let Some(cwd) = file_context.get("cwd").and_then(|v| v.as_str()) {
                    return Some(PathBuf::from(cwd));
                }
            }
        }
    }

    // Fallback: try to infer from project name in path
    let (project_name, _) = extract_path_info(path);
    project_name.map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_path_info() {
        let path = PathBuf::from(
            "/Users/test/.config/manicode/projects/myproject/chats/2025-12-19T09-07-21.203Z/chat-messages.json",
        );
        let (project, session) = extract_path_info(&path);
        assert_eq!(project, Some("myproject".to_string()));
        assert_eq!(session, Some("2025-12-19T09-07-21.203Z".to_string()));
    }

    #[test]
    fn test_parse_codebuff_timestamp_from_id() {
        let msg = serde_json::json!({
            "id": "ai-1766137957398-bf2216fd8cd09",
            "variant": "ai"
        });
        let ts = parse_codebuff_timestamp(&msg);
        assert_eq!(ts, Some(1766137957398));
    }

    #[test]
    fn test_extract_content_with_tool_blocks() {
        let msg = serde_json::json!({
            "variant": "ai",
            "content": "",
            "blocks": [
                {
                    "type": "tool",
                    "toolName": "run_terminal_command",
                    "input": {
                        "command": "git status"
                    },
                    "output": "On branch main\nnothing to commit"
                },
                {
                    "type": "text",
                    "content": "The git status shows everything is clean."
                }
            ]
        });
        let content = extract_content(&msg);
        assert!(content.contains("git status"));
        assert!(content.contains("On branch main"));
        assert!(content.contains("everything is clean"));
    }
}
