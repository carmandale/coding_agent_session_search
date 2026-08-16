use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde_json::Value;
use tracing::warn;

use super::{
    Connector, DetectionResult, DiscoveredSourceFile, NormalizedConversation, NormalizedMessage,
    ScanContext, parse_timestamp, reindex_messages,
};

const MAX_INDEXED_TOOL_OUTPUT_CHARS: usize = 128 * 1024;

pub struct CodexConnector {
    inner: franken_agent_detection::CodexConnector,
}

impl Default for CodexConnector {
    fn default() -> Self {
        Self::new()
    }
}

impl CodexConnector {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: franken_agent_detection::CodexConnector::new(),
        }
    }
}

impl Connector for CodexConnector {
    fn detect(&self) -> DetectionResult {
        self.inner.detect()
    }

    fn scan(&self, ctx: &ScanContext) -> Result<Vec<NormalizedConversation>> {
        let mut conversations = self.inner.scan(ctx)?;
        for conversation in &mut conversations {
            augment_modern_codex_messages(conversation);
        }
        let parsed: HashSet<PathBuf> = conversations
            .iter()
            .map(|conversation| conversation.source_path.clone())
            .collect();
        let mut recovered = Vec::new();
        recover_rollouts_the_base_parser_dropped(&self.inner, ctx, &parsed, &mut |conversation| {
            recovered.push(conversation);
            Ok(())
        })?;
        conversations.extend(recovered);
        Ok(conversations)
    }

    fn supports_streaming_scan(&self) -> bool {
        self.inner.supports_streaming_scan()
    }

    fn discover_source_files(&self, ctx: &ScanContext) -> Result<Vec<DiscoveredSourceFile>> {
        self.inner.discover_source_files(ctx)
    }

    fn scan_with_callback(
        &self,
        ctx: &ScanContext,
        on_conversation: &mut dyn FnMut(NormalizedConversation) -> Result<()>,
    ) -> Result<()> {
        let mut parsed: HashSet<PathBuf> = HashSet::new();
        self.inner.scan_with_callback(ctx, &mut |mut conversation| {
            augment_modern_codex_messages(&mut conversation);
            parsed.insert(conversation.source_path.clone());
            on_conversation(conversation)
        })?;
        recover_rollouts_the_base_parser_dropped(&self.inner, ctx, &parsed, on_conversation)
    }
}

/// Emit a conversation for every discovered rollout the base parser produced
/// nothing for, and say so plainly when one cannot be recovered.
///
/// `franken_agent_detection`'s `.jsonl` arm matches on the record *envelope* —
/// `session_meta` / `response_item` / `event_msg`, each requiring a `payload` —
/// and drops everything else through a catch-all. Rollouts written before that
/// envelope existed carry the Responses-API item at the top level instead
/// (`{"type":"message","role":"user","content":[…]}`), so every record falls
/// through, `messages` stays empty, and the file is skipped by a `continue`
/// *before* `on_conversation` is ever called. No conversation reaches this
/// crate, which is why augmenting is not enough to rescue one (bead 1pzs3).
///
/// Discovery and the scan share the base parser's traversal, dedupe and
/// `since_ts` filter, so "discovered but never emitted" is exactly the set the
/// base parser dropped. A skip may be correct — a session stub really does hold
/// nothing — but it has to be counted and named rather than swallowed, which is
/// the honesty half of the same defect (bead 9fnbr).
fn recover_rollouts_the_base_parser_dropped(
    inner: &franken_agent_detection::CodexConnector,
    ctx: &ScanContext,
    parsed: &HashSet<PathBuf>,
    on_conversation: &mut dyn FnMut(NormalizedConversation) -> Result<()>,
) -> Result<()> {
    let discovered = match inner.discover_source_files(ctx) {
        Ok(discovered) => discovered,
        Err(error) => {
            warn!(
                error = %error,
                "codex source discovery failed; rollouts the base parser dropped cannot be checked",
            );
            return Ok(());
        }
    };

    for source in discovered {
        if parsed.contains(&source.source_path) {
            continue;
        }
        let Some(conversation) =
            pre_envelope_conversation(&source.scan_root, &source.source_path)
        else {
            warn!(
                source_path = %source.source_path.display(),
                "codex rollout yielded no conversation and holds no recoverable records; skipping",
            );
            continue;
        };
        warn!(
            source_path = %conversation.source_path.display(),
            messages = conversation.messages.len(),
            "recovered a codex rollout in the pre-envelope record shape the base parser drops",
        );
        on_conversation(conversation)?;
    }

    Ok(())
}

/// Rebuild the conversation a `.jsonl` rollout should have produced, from the
/// same per-record parser the augmenter uses.
///
/// Nothing new is invented at message level — [`modern_codex_message`] reads
/// both record shapes. What this reproduces is the *envelope*, matching the base
/// parser's own rules so a recovered conversation is indistinguishable from one
/// it emitted itself: same external ID, same title rule, same time bounds from
/// whatever timestamps the file records. Message order is file order, which is
/// what the base parser does for a rollout whose records carry no timestamps.
fn pre_envelope_conversation(
    scan_root: &Path,
    source_path: &Path,
) -> Option<NormalizedConversation> {
    if source_path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_none_or(|ext| !ext.eq_ignore_ascii_case("jsonl"))
    {
        return None;
    }

    let file = File::open(source_path).ok()?;
    let mut messages = Vec::new();
    let mut started_at = None;
    let mut ended_at = None;

    for line in BufReader::new(file).lines().map_while(Result::ok) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(raw) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        // The session header is the only timestamped record a pre-envelope
        // rollout carries, and it is not a message. Widening the bounds from
        // every timestamp in the file is what the base parser does for
        // `session_meta`, whose record is likewise not a message.
        widen_time_bounds(
            &mut started_at,
            &mut ended_at,
            raw.get("timestamp").and_then(parse_timestamp),
        );
        if let Some(message) = modern_codex_message(&raw) {
            messages.push(message);
        }
    }

    if messages.is_empty() {
        return None;
    }
    reindex_messages(&mut messages);

    Some(NormalizedConversation {
        agent_slug: "codex".to_string(),
        external_id: pre_envelope_external_id(scan_root, source_path),
        title: rollout_title(&messages),
        workspace: None,
        source_path: source_path.to_path_buf(),
        started_at,
        ended_at,
        metadata: serde_json::json!({"source": "rollout", "record_shape": "pre_envelope"}),
        messages,
    })
}

fn widen_time_bounds(started_at: &mut Option<i64>, ended_at: &mut Option<i64>, ts: Option<i64>) {
    if let Some(ts) = ts {
        *started_at = Some(started_at.map_or(ts, |current: i64| current.min(ts)));
        *ended_at = Some(ended_at.map_or(ts, |current: i64| current.max(ts)));
    }
}

/// The base parser's title rule: the first line of the first user turn, capped
/// at 100 characters, falling back to the first line of the first turn.
fn rollout_title(messages: &[NormalizedMessage]) -> Option<String> {
    messages
        .iter()
        .find(|message| message.role == "user")
        .map(|message| {
            message
                .content
                .lines()
                .next()
                .unwrap_or(&message.content)
                .chars()
                .take(100)
                .collect::<String>()
        })
        .or_else(|| {
            messages
                .first()
                .and_then(|message| message.content.lines().next())
                .map(|line| line.chars().take(100).collect())
        })
}

/// The base parser's external ID: the rollout's path relative to the sessions
/// directory with the extension dropped, falling back to the bare file stem.
fn pre_envelope_external_id(scan_root: &Path, source_path: &Path) -> Option<String> {
    let sessions_dir = sessions_dir_for(scan_root, source_path);
    source_path
        .strip_prefix(&sessions_dir)
        .ok()
        .and_then(|relative| {
            relative
                .with_extension("")
                .to_str()
                .map(std::string::ToString::to_string)
        })
        .or_else(|| {
            source_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map(std::string::ToString::to_string)
        })
}

/// Mirror of the base parser's sessions-directory resolution.
///
/// Discovery records the scan root each file came from. When the root *is* the
/// rollout file — the shape `--watch-once <path>` produces — the base parser
/// walks the file's ancestors for a `sessions` directory and otherwise treats
/// the file's own parent as the root. Getting this wrong would give a recovered
/// conversation a different external ID from the one the base parser assigns,
/// which is how a later re-scan would insert a duplicate instead of an update.
fn sessions_dir_for(scan_root: &Path, source_path: &Path) -> PathBuf {
    if scan_root == source_path {
        if let Some(sessions) = source_path.ancestors().find(|ancestor| {
            ancestor.file_name().and_then(|name| name.to_str()) == Some("sessions")
        }) {
            return sessions.to_path_buf();
        }
        let home = source_path.parent().unwrap_or(scan_root);
        return sessions_dir_under(home);
    }
    sessions_dir_under(scan_root)
}

fn sessions_dir_under(home: &Path) -> PathBuf {
    let sessions = home.join("sessions");
    if sessions.exists() {
        sessions
    } else {
        home.to_path_buf()
    }
}

fn augment_modern_codex_messages(conversation: &mut NormalizedConversation) {
    if conversation
        .source_path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_none_or(|ext| !ext.eq_ignore_ascii_case("jsonl"))
    {
        return;
    }

    let Ok(file) = File::open(&conversation.source_path) else {
        return;
    };

    let mut seen_messages: HashSet<ModernCodexMessageSignature> = conversation
        .messages
        .iter()
        .map(modern_codex_message_signature)
        .collect();
    let mut seen_call_ids: HashSet<String> = conversation
        .messages
        .iter()
        .flat_map(modern_codex_message_call_ids)
        .collect();
    let mut seen_raw_entries: HashSet<[u8; 32]> = conversation
        .messages
        .iter()
        .map(|message| modern_codex_raw_signature(&message.extra))
        .collect();
    let mut added = false;
    for (line_no_zero, line) in BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .enumerate()
    {
        let line_no = line_no_zero + 1;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let raw = match serde_json::from_str::<Value>(line) {
            Ok(value) => value,
            Err(parse_err) => {
                // Per gauntlet finding CONF-cass-003: surface malformed JSONL lines
                // to tracing so operators can correlate `cass diag` reports against
                // unreadable Codex rollout entries. The line is still dropped to
                // preserve resilience; the warning is purely diagnostic.
                warn!(
                    source_path = %conversation.source_path.display(),
                    line_no = line_no,
                    error = %parse_err,
                    "codex rollout JSONL line failed to parse; skipping",
                );
                continue;
            }
        };
        let raw_signature = modern_codex_raw_signature(&raw);
        if seen_raw_entries.contains(&raw_signature) {
            continue;
        }
        let Some(message) = modern_codex_message(&raw) else {
            continue;
        };
        if message_already_indexed(&seen_messages, &seen_call_ids, &message) {
            seen_raw_entries.insert(raw_signature);
            continue;
        }
        seen_messages.insert(modern_codex_message_signature(&message));
        seen_call_ids.extend(modern_codex_message_call_ids(&message));
        seen_raw_entries.insert(raw_signature);
        conversation.messages.push(message);
        added = true;
    }

    if added {
        conversation.messages.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.idx.cmp(&right.idx))
        });
        reindex_messages(&mut conversation.messages);
    }
}

/// Read one Codex rollout record, in either of the two shapes Codex has written.
///
/// Modern records wrap the Responses-API item in an envelope
/// (`{"type":"response_item","payload":{…}}`). Records written before that
/// envelope existed put the item at the top level with no `payload`, so the
/// record *is* the payload and [`response_item_message`] reads it directly —
/// which is why `reasoning` and `local_shell_call` are skipped in both shapes
/// without either being named here.
///
/// Measured 2026-08-16 across 8,707 `.jsonl` rollouts on this machine: 8,650
/// carry only envelope records, 17 only bare ones, 40 neither, and none carries
/// both. So the bare arm changes nothing for an existing conversation today; it
/// exists so a rollout that straddles a future format boundary keeps the half
/// the envelope arm cannot read (beads 1pzs3, 9fnbr).
fn modern_codex_message(raw: &Value) -> Option<NormalizedMessage> {
    let entry_type = raw.get("type").and_then(Value::as_str)?;
    let created_at = raw.get("timestamp").and_then(parse_timestamp);

    match raw.get("payload") {
        Some(payload) => match entry_type {
            "response_item" => response_item_message(payload, created_at, raw),
            "event_msg" => event_message(payload, created_at, raw),
            _ => None,
        },
        None => matches!(
            entry_type,
            "message" | "function_call" | "function_call_output"
        )
        .then(|| response_item_message(raw, created_at, raw))
        .flatten(),
    }
}

fn response_item_message(
    payload: &Value,
    created_at: Option<i64>,
    raw: &Value,
) -> Option<NormalizedMessage> {
    match payload.get("type").and_then(Value::as_str) {
        Some("message") | None => {
            let content = payload.get("content").and_then(flatten_modern_content)?;
            let role = payload
                .get("role")
                .and_then(Value::as_str)
                .unwrap_or("agent")
                .to_string();
            Some(normalized_message(
                role,
                None,
                created_at,
                content,
                raw.clone(),
                payload.get("content").map_or_else(
                    Vec::new,
                    franken_agent_detection::extract_invocations_from_content_blocks,
                ),
            ))
        }
        Some("function_call") => {
            let tool_name = payload
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let arguments = payload.get("arguments").cloned();
            let content = tool_call_content(tool_name, arguments.as_ref());
            let call_id = payload
                .get("call_id")
                .or_else(|| payload.get("id"))
                .and_then(Value::as_str)
                .map(str::to_string);
            Some(normalized_message(
                "assistant".to_string(),
                None,
                created_at,
                content,
                raw.clone(),
                vec![franken_agent_detection::NormalizedInvocation {
                    kind: "tool".to_string(),
                    name: tool_name.to_string(),
                    raw_name: None,
                    call_id,
                    arguments: arguments.and_then(normalize_invocation_arguments),
                }],
            ))
        }
        Some("function_call_output") => {
            let output = payload.get("output").and_then(Value::as_str)?;
            let call_id = payload.get("call_id").and_then(Value::as_str);
            Some(normalized_message(
                "tool".to_string(),
                None,
                created_at,
                tool_output_content(call_id, output),
                raw.clone(),
                Vec::new(),
            ))
        }
        _ => None,
    }
}

fn event_message(
    payload: &Value,
    created_at: Option<i64>,
    raw: &Value,
) -> Option<NormalizedMessage> {
    match payload.get("type").and_then(Value::as_str) {
        Some("agent_message") => {
            let content = payload
                .get("message")
                .or_else(|| payload.get("text"))
                .and_then(Value::as_str)?
                .trim()
                .to_string();
            non_empty_message("assistant".to_string(), None, created_at, content, raw)
        }
        Some("tool_result") => {
            let output = payload
                .get("output")
                .or_else(|| payload.get("result"))
                .and_then(Value::as_str)?;
            let call_id = payload
                .get("call_id")
                .or_else(|| payload.get("id"))
                .and_then(Value::as_str);
            Some(normalized_message(
                "tool".to_string(),
                None,
                created_at,
                tool_output_content(call_id, output),
                raw.clone(),
                Vec::new(),
            ))
        }
        _ => None,
    }
}

fn normalized_message(
    role: String,
    author: Option<String>,
    created_at: Option<i64>,
    content: String,
    extra: Value,
    invocations: Vec<franken_agent_detection::NormalizedInvocation>,
) -> NormalizedMessage {
    NormalizedMessage {
        idx: 0,
        role,
        author,
        created_at,
        content,
        extra,
        invocations,
        snippets: Vec::new(),
    }
}

fn non_empty_message(
    role: String,
    author: Option<String>,
    created_at: Option<i64>,
    content: String,
    raw: &Value,
) -> Option<NormalizedMessage> {
    (!content.trim().is_empty())
        .then(|| normalized_message(role, author, created_at, content, raw.clone(), Vec::new()))
}

fn flatten_modern_content(content: &Value) -> Option<String> {
    if let Some(text) = content
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
    {
        return Some(text.to_string());
    }

    let mut parts = Vec::new();
    for item in content.as_array()? {
        let text = modern_content_part_text(item);

        let text = text.trim();
        if !text.is_empty() {
            parts.push(text.to_string());
        }
    }

    (!parts.is_empty()).then(|| parts.join("\n"))
}

fn modern_content_part_text(item: &Value) -> String {
    if let Some(text) = item.as_str() {
        return text.to_string();
    }

    let item_type = item.get("type").and_then(Value::as_str);
    if matches!(
        item_type,
        None | Some("text") | Some("input_text") | Some("output_text")
    ) {
        return item
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
    }

    if item_type == Some("tool_use") {
        let tool_name = item
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let detail = item
            .get("input")
            .and_then(|input| {
                input
                    .get("description")
                    .or_else(|| input.get("file_path"))
                    .or_else(|| input.get("path"))
                    .or_else(|| input.get("command"))
            })
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        return if detail.is_empty() {
            format!("[Tool: {tool_name}]")
        } else {
            format!("[Tool: {tool_name} - {detail}]")
        };
    }

    String::new()
}

fn tool_call_content(tool_name: &str, arguments: Option<&Value>) -> String {
    let mut content = format!("[Tool: {tool_name}]");
    if let Some(arguments) = arguments.and_then(argument_text) {
        content.push('\n');
        content.push_str(&arguments);
    }
    content
}

fn tool_output_content(call_id: Option<&str>, output: &str) -> String {
    let label = call_id.map_or_else(
        || "[Tool output]".to_string(),
        |id| format!("[Tool output: {id}]"),
    );
    let output = truncate_tool_output(output.trim());
    if output.is_empty() {
        label
    } else {
        format!("{label}\n{output}")
    }
}

fn argument_text(arguments: &Value) -> Option<String> {
    let text = match arguments {
        Value::String(text) => text.trim().to_string(),
        other => serde_json::to_string(other).ok()?,
    };
    (!text.is_empty()).then_some(text)
}

fn normalize_invocation_arguments(arguments: Value) -> Option<Value> {
    match arguments {
        Value::String(text) => serde_json::from_str(&text)
            .ok()
            .or_else(|| (!text.trim().is_empty()).then_some(Value::String(text))),
        Value::Null => None,
        other => Some(other),
    }
}

fn truncate_tool_output(output: &str) -> String {
    let mut truncated = String::new();
    let mut chars = output.chars();
    for _ in 0..MAX_INDEXED_TOOL_OUTPUT_CHARS {
        let Some(ch) = chars.next() else {
            return output.to_string();
        };
        truncated.push(ch);
    }
    let omitted = chars.count();
    truncated.push_str(&format!(
        "\n[truncated {omitted} additional chars from tool output]"
    ));
    truncated
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ModernCodexMessageSignature {
    role: String,
    author: Option<String>,
    created_at: Option<i64>,
    content_hash: [u8; 32],
}

fn modern_codex_message_signature(message: &NormalizedMessage) -> ModernCodexMessageSignature {
    ModernCodexMessageSignature {
        role: message.role.clone(),
        author: message.author.clone(),
        created_at: message.created_at,
        content_hash: *blake3::hash(message.content.as_bytes()).as_bytes(),
    }
}

fn modern_codex_raw_signature(raw: &Value) -> [u8; 32] {
    let mut bytes = Vec::new();
    if serde_json::to_writer(&mut bytes, raw).is_err() {
        bytes.extend_from_slice(raw.to_string().as_bytes());
    }
    *blake3::hash(&bytes).as_bytes()
}

fn modern_codex_message_call_ids(message: &NormalizedMessage) -> impl Iterator<Item = String> + '_ {
    message
        .invocations
        .iter()
        .filter_map(|invocation| invocation.call_id.clone())
}

fn message_already_indexed(
    seen_messages: &HashSet<ModernCodexMessageSignature>,
    seen_call_ids: &HashSet<String>,
    candidate: &NormalizedMessage,
) -> bool {
    seen_messages.contains(&modern_codex_message_signature(candidate))
        || candidate
            .invocations
            .iter()
            .filter_map(|invocation| invocation.call_id.as_deref())
            .any(|call_id| seen_call_ids.contains(call_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectors::ScanRoot;
    use std::fs;
    use tempfile::TempDir;

    /// `2025-08-20T13:20:47.060Z`, the session header timestamp used below.
    const HEADER_TS_MS: i64 = 1_755_696_047_060;

    /// A rollout in the shape Codex wrote before the record envelope existed:
    /// a session header, state markers, and bare Responses-API items at the top
    /// level with no `payload` and no per-record timestamp. `reasoning` and
    /// `local_shell_call` are present because the modern path skips both, and a
    /// recovered conversation must skip them too.
    const PRE_ENVELOPE_ROLLOUT: &str = r#"{"id":"c27a914d","timestamp":"2025-08-20T13:20:47.060Z","instructions":null,"git":{"branch":"main"}}
{"record_type":"state"}
{"type":"message","id":null,"role":"user","content":[{"type":"input_text","text":"index the pre-envelope rollouts\nsecond line of the same turn"}]}
{"type":"reasoning","id":"rs_1","summary":[],"encrypted_content":"opaque"}
{"type":"function_call","id":"fc_1","name":"shell","arguments":"{\"command\":[\"ls\"]}","call_id":"call_1"}
{"type":"function_call_output","call_id":"call_1","output":"README.md"}
{"type":"local_shell_call","id":"lsc_1","status":"completed","action":{"type":"exec"}}
{"type":"message","id":null,"role":"assistant","content":[{"type":"output_text","text":"listed the directory"}]}
"#;

    /// The modern envelope shape the base parser already handles.
    const MODERN_ROLLOUT: &str = r#"{"type":"response_item","timestamp":"2025-08-20T13:20:47.060Z","payload":{"role":"user","content":"modern turn"}}
{"type":"response_item","timestamp":"2025-08-20T13:25:00.000Z","payload":{"role":"assistant","content":"modern reply"}}
"#;

    /// A real session stub: envelope records that carry no message at all. The
    /// base parser is right to drop it and recovery must not manufacture one.
    const EMPTY_STUB_ROLLOUT: &str = r#"{"type":"session_meta","timestamp":"2025-08-20T13:20:47.060Z","payload":{"cwd":"/tmp/project"}}
{"record_type":"state"}
"#;

    /// A rollout straddling the format boundary. No file on this machine looks
    /// like this today (measured: 8,650 envelope-only, 17 bare-only, 40 empty,
    /// 0 mixed), but a future boundary will land mid-file the same way, and the
    /// base parser emits this one — so the bare half must arrive by augmenting
    /// the conversation it emitted, never as a second conversation.
    const MIXED_SHAPE_ROLLOUT: &str = r#"{"type":"response_item","timestamp":"2025-08-20T13:20:47.060Z","payload":{"role":"user","content":"enveloped turn"}}
{"type":"message","id":null,"role":"assistant","content":[{"type":"output_text","text":"bare turn in the same file"}]}
"#;

    /// Lay out `<dir>/.codex/sessions/2025/08/20/` and write each named rollout
    /// into it, returning the codex home the connector should be pointed at.
    fn codex_home_with(dir: &TempDir, rollouts: &[(&str, &str)]) -> PathBuf {
        let home = dir.path().join("home").join(".codex");
        let day = home.join("sessions").join("2025").join("08").join("20");
        fs::create_dir_all(&day).unwrap();
        for (name, body) in rollouts {
            fs::write(day.join(name), body).unwrap();
        }
        home
    }

    fn scan_streaming(ctx: &ScanContext) -> Vec<NormalizedConversation> {
        let connector = CodexConnector::new();
        let mut out = Vec::new();
        connector
            .scan_with_callback(ctx, &mut |conversation| {
                out.push(conversation);
                Ok(())
            })
            .unwrap();
        out
    }

    #[test]
    fn pre_envelope_rollout_is_recovered_with_the_base_parsers_envelope() {
        let dir = TempDir::new().unwrap();
        let home = codex_home_with(&dir, &[("rollout-pre-envelope.jsonl", PRE_ENVELOPE_ROLLOUT)]);
        let ctx = ScanContext::with_roots(
            dir.path().join("cass"),
            vec![ScanRoot::local(home.clone())],
            None,
        );

        let convs = scan_streaming(&ctx);
        assert_eq!(
            convs.len(),
            1,
            "the base parser drops this file entirely; recovery must emit it"
        );
        let conversation = &convs[0];

        assert_eq!(
            conversation.external_id.as_deref(),
            Some("2025/08/20/rollout-pre-envelope"),
            "external ID must match what the base parser would have assigned, \
             or a re-scan inserts a duplicate row instead of updating this one"
        );
        assert_eq!(conversation.agent_slug, "codex");
        assert_eq!(
            conversation.title.as_deref(),
            Some("index the pre-envelope rollouts"),
            "title is the first line of the first user turn"
        );
        assert_eq!(
            conversation.metadata.get("record_shape").and_then(Value::as_str),
            Some("pre_envelope"),
            "recovered rows must be identifiable in the archive"
        );

        // The four message-bearing records, in file order: user message,
        // function_call, function_call_output, assistant message. `reasoning`
        // and `local_shell_call` are skipped, which is what the modern path
        // does with the same item types.
        let roles: Vec<&str> = conversation
            .messages
            .iter()
            .map(|message| message.role.as_str())
            .collect();
        assert_eq!(roles, vec!["user", "assistant", "tool", "assistant"]);
        assert!(
            conversation.messages[0]
                .content
                .contains("index the pre-envelope rollouts"),
            "user turn content lost: {:?}",
            conversation.messages[0].content
        );
        assert_eq!(
            conversation.messages[1]
                .invocations
                .first()
                .and_then(|invocation| invocation.call_id.as_deref()),
            Some("call_1"),
            "the tool call must keep its call id so output pairs with it"
        );
        assert!(
            conversation.messages[2].content.contains("README.md"),
            "tool output lost: {:?}",
            conversation.messages[2].content
        );
        assert!(
            conversation.messages[3]
                .content
                .contains("listed the directory"),
            "assistant turn lost: {:?}",
            conversation.messages[3].content
        );
        let indices: Vec<i64> = conversation
            .messages
            .iter()
            .map(|message| message.idx)
            .collect();
        assert_eq!(indices, vec![0, 1, 2, 3], "messages must be reindexed");

        // The session header is the only timestamped record in the file.
        assert_eq!(conversation.started_at, Some(HEADER_TS_MS));
        assert_eq!(conversation.ended_at, Some(HEADER_TS_MS));
    }

    #[test]
    fn pre_envelope_recovery_keeps_the_sessions_relative_id_for_an_explicit_file_root() {
        // The shape `cass index --watch-once <path>` produces: the scan root is
        // the rollout file itself, not a directory.
        let dir = TempDir::new().unwrap();
        let home = codex_home_with(&dir, &[("rollout-pre-envelope.jsonl", PRE_ENVELOPE_ROLLOUT)]);
        let rollout = home
            .join("sessions")
            .join("2025")
            .join("08")
            .join("20")
            .join("rollout-pre-envelope.jsonl");
        let ctx = ScanContext::with_roots(
            rollout.clone(),
            vec![ScanRoot::local(rollout.clone())],
            None,
        );

        let convs = scan_streaming(&ctx);
        assert_eq!(convs.len(), 1);
        assert_eq!(
            convs[0].external_id.as_deref(),
            Some("2025/08/20/rollout-pre-envelope"),
            "an explicit file root must not collapse the ID to the bare stem"
        );
        assert_eq!(convs[0].source_path, rollout);
    }

    #[test]
    fn modern_rollout_is_emitted_once_and_never_recovered_on_top() {
        // Negative arm. Recovery runs over every discovered file; if it keyed
        // off anything looser than "the base parser emitted nothing for this
        // path" it would double every modern rollout in the archive.
        let dir = TempDir::new().unwrap();
        let home = codex_home_with(&dir, &[("rollout-modern.jsonl", MODERN_ROLLOUT)]);
        let ctx = ScanContext::with_roots(
            dir.path().join("cass"),
            vec![ScanRoot::local(home)],
            None,
        );

        let convs = scan_streaming(&ctx);
        assert_eq!(convs.len(), 1, "modern rollout emitted more than once");
        assert_eq!(convs[0].messages.len(), 2);
        assert_eq!(
            convs[0].metadata.get("record_shape"),
            None,
            "a base-parser conversation must not be relabelled as recovered"
        );
    }

    #[test]
    fn mixed_shape_rollout_arrives_as_one_conversation_carrying_both_halves() {
        let dir = TempDir::new().unwrap();
        let home = codex_home_with(&dir, &[("rollout-mixed.jsonl", MIXED_SHAPE_ROLLOUT)]);
        let ctx = ScanContext::with_roots(
            dir.path().join("cass"),
            vec![ScanRoot::local(home)],
            None,
        );

        let convs = scan_streaming(&ctx);
        assert_eq!(
            convs.len(),
            1,
            "a file the base parser emitted must not also be recovered as a second conversation"
        );
        let contents: Vec<&str> = convs[0]
            .messages
            .iter()
            .map(|message| message.content.as_str())
            .collect();
        assert!(
            contents.contains(&"enveloped turn"),
            "base-parser turn lost: {contents:?}"
        );
        assert!(
            contents.contains(&"bare turn in the same file"),
            "bare turn in an emitted file is still dropped: {contents:?}"
        );
    }

    #[test]
    fn session_stub_with_no_message_records_still_yields_nothing() {
        // Negative arm. The tempting wrong fix is to emit a conversation for
        // every dropped file, which would flood the archive with empty rows and
        // make the recovery count meaningless.
        let dir = TempDir::new().unwrap();
        let home = codex_home_with(&dir, &[("rollout-stub.jsonl", EMPTY_STUB_ROLLOUT)]);
        let ctx = ScanContext::with_roots(
            dir.path().join("cass"),
            vec![ScanRoot::local(home)],
            None,
        );

        assert!(
            scan_streaming(&ctx).is_empty(),
            "a stub holding no message records must stay skipped"
        );
    }

    #[test]
    fn mixed_archive_recovers_only_the_dropped_rollout() {
        let dir = TempDir::new().unwrap();
        let home = codex_home_with(
            &dir,
            &[
                ("rollout-modern.jsonl", MODERN_ROLLOUT),
                ("rollout-pre-envelope.jsonl", PRE_ENVELOPE_ROLLOUT),
                ("rollout-stub.jsonl", EMPTY_STUB_ROLLOUT),
            ],
        );
        let ctx = ScanContext::with_roots(
            dir.path().join("cass"),
            vec![ScanRoot::local(home)],
            None,
        );

        let convs = scan_streaming(&ctx);
        let mut ids: Vec<String> = convs
            .iter()
            .filter_map(|conversation| conversation.external_id.clone())
            .collect();
        ids.sort();
        assert_eq!(
            ids,
            vec![
                "2025/08/20/rollout-modern".to_string(),
                "2025/08/20/rollout-pre-envelope".to_string(),
            ],
        );

        // The batch path must agree with the streaming path.
        let batch = CodexConnector::new().scan(&ctx).unwrap();
        let mut batch_ids: Vec<String> = batch
            .iter()
            .filter_map(|conversation| conversation.external_id.clone())
            .collect();
        batch_ids.sort();
        assert_eq!(batch_ids, ids, "scan and scan_with_callback disagree");
    }

    fn message(content: &str, call_id: Option<&str>) -> NormalizedMessage {
        NormalizedMessage {
            idx: 0,
            role: "assistant".to_string(),
            author: None,
            created_at: Some(1_700_000_000_000),
            content: content.to_string(),
            extra: Value::Null,
            invocations: call_id
                .map(|call_id| {
                    vec![franken_agent_detection::NormalizedInvocation {
                        kind: "tool".to_string(),
                        name: "shell".to_string(),
                        raw_name: None,
                        call_id: Some(call_id.to_string()),
                        arguments: None,
                    }]
                })
                .unwrap_or_default(),
            snippets: Vec::new(),
        }
    }

    #[test]
    fn modern_codex_duplicate_detection_uses_precomputed_sets() {
        let existing = message("canonical response", Some("call-1"));
        let mut seen_messages = HashSet::from([modern_codex_message_signature(&existing)]);
        let mut seen_call_ids: HashSet<String> = modern_codex_message_call_ids(&existing).collect();

        assert!(message_already_indexed(
            &seen_messages,
            &seen_call_ids,
            &message("canonical response", None)
        ));
        assert!(message_already_indexed(
            &seen_messages,
            &seen_call_ids,
            &message("same tool call, changed wording", Some("call-1"))
        ));

        let fresh = message("fresh response", Some("call-2"));
        assert!(!message_already_indexed(
            &seen_messages,
            &seen_call_ids,
            &fresh
        ));
        seen_messages.insert(modern_codex_message_signature(&fresh));
        seen_call_ids.extend(modern_codex_message_call_ids(&fresh));
        assert!(message_already_indexed(
            &seen_messages,
            &seen_call_ids,
            &fresh
        ));
    }
}
