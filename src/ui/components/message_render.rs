//! Shared message rendering utilities for conversation detail views.
//!
//! Provides functions for rendering conversation messages with beautiful formatting,
//! including code blocks, tool calls, JSON pretty-printing, and search term highlighting.

use chrono::{DateTime, Utc};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::model::types::MessageRole;
use crate::ui::components::theme::ThemePalette;
use crate::ui::data::ConversationView;

/// Formats a timestamp as an absolute string with date and time in UTC.
pub fn format_absolute_time(timestamp_ms: i64) -> String {
    DateTime::<Utc>::from_timestamp_millis(timestamp_ms).map_or_else(
        || "unknown".to_string(),
        |dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
    )
}

/// Highlight occurrences of a query term within text.
/// Returns styled spans with matches highlighted using the palette's highlight style.
pub fn highlight_spans_owned(
    text: &str,
    query: &str,
    palette: ThemePalette,
    base: Style,
) -> Vec<Span<'static>> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    if query.trim().is_empty() {
        spans.push(Span::styled(text.to_string(), base));
        return spans;
    }

    let lower = text.to_lowercase();
    let q = query.to_lowercase();

    // If Unicode casefolding changes byte lengths (e.g., ß -> ss), fall back to
    // case-sensitive matching to avoid slicing errors.
    if lower.len() != text.len() || q.len() != query.len() {
        let mut remaining = text;
        while let Some(pos) = remaining.find(query) {
            if pos > 0 {
                spans.push(Span::styled(remaining[..pos].to_string(), base));
            }
            let end = pos + query.len();
            spans.push(Span::styled(
                remaining[pos..end].to_string(),
                base.patch(palette.highlight_style()),
            ));
            remaining = &remaining[end..];
        }
        if !remaining.is_empty() {
            spans.push(Span::styled(remaining.to_string(), base));
        }
        return spans;
    }
    let mut idx = 0;
    while let Some(pos) = lower[idx..].find(&q) {
        let start = idx + pos;
        if start > idx {
            spans.push(Span::styled(text[idx..start].to_string(), base));
        }
        let end = start + q.len();
        spans.push(Span::styled(
            text[start..end].to_string(),
            base.patch(palette.highlight_style()),
        ));
        idx = end;
    }
    if idx < text.len() {
        spans.push(Span::styled(text[idx..].to_string(), base));
    }
    spans
}

/// Render a single line with light-weight inline markdown (bold/italic/`code`) and
/// search-term highlighting. Keeps everything ASCII-friendly for predictable widths.
pub fn render_inline_markdown_line(
    line: &str,
    query: &str,
    palette: ThemePalette,
    base: Style,
) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut rest = line;

    while !rest.is_empty() {
        if let Some(content) = rest.strip_prefix("**")
            && let Some(end) = content.find("**")
        {
            let (bold_text, tail) = content.split_at(end);
            let highlighted =
                highlight_spans_owned(bold_text, query, palette, base.add_modifier(Modifier::BOLD));
            spans.extend(highlighted);
            rest = tail.trim_start_matches('*');
            continue;
        }

        if let Some(content) = rest.strip_prefix('`')
            && let Some(end) = content.find('`')
        {
            let (code_text, tail) = content.split_at(end);
            let highlighted = highlight_spans_owned(
                code_text,
                query,
                palette,
                base.bg(palette.surface).fg(palette.accent_alt),
            );
            spans.extend(highlighted);
            rest = &tail[1..]; // skip closing backtick
            continue;
        }

        if let Some(content) = rest.strip_prefix('*')
            && !content.starts_with('*')
            && let Some(end) = content.find('*')
        {
            let (ital_text, tail) = content.split_at(end);
            let highlighted = highlight_spans_owned(
                ital_text,
                query,
                palette,
                base.add_modifier(Modifier::ITALIC),
            );
            spans.extend(highlighted);
            rest = tail.trim_start_matches('*');
            continue;
        }

        // Plain chunk until next special token
        let next_special = rest.find(['*', '`']).unwrap_or(rest.len());

        if next_special == 0 {
            // Avoid infinite loop on stray marker; emit literally and advance
            if let Some((ch, tail)) = rest.chars().next().map(|c| (c, &rest[c.len_utf8()..])) {
                spans.extend(highlight_spans_owned(&ch.to_string(), query, palette, base));
                rest = tail;
                continue;
            }
        }

        let (plain, tail) = rest.split_at(next_special);
        spans.extend(highlight_spans_owned(plain, query, palette, base));
        rest = tail;
    }

    Line::from(spans)
}

/// Parse message content and render with beautiful formatting.
/// Handles code blocks, tool calls, JSON, and highlights search terms.
pub fn parse_message_content(content: &str, query: &str, palette: ThemePalette) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut in_code_block = false;
    let mut code_lang: Option<String> = None;
    let mut code_buffer: Vec<String> = Vec::new();

    for line_text in content.lines() {
        let trimmed = line_text.trim_start();

        // Handle code block start/end
        if trimmed.starts_with("```") {
            if in_code_block {
                // End of code block - render buffered code
                in_code_block = false;
                if !code_buffer.is_empty() {
                    let lang_label = code_lang
                        .take()
                        .filter(|l| !l.is_empty())
                        .map(|l| format!(" {l}"))
                        .unwrap_or_default();
                    lines.push(Line::from(vec![
                        Span::styled("┌──", Style::default().fg(palette.hint)),
                        Span::styled(
                            lang_label,
                            Style::default()
                                .fg(palette.accent_alt)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));
                    for code_line in code_buffer.drain(..) {
                        lines.push(Line::from(vec![
                            Span::styled("│ ", Style::default().fg(palette.hint)),
                            Span::styled(
                                code_line,
                                Style::default().fg(palette.fg).bg(palette.surface),
                            ),
                        ]));
                    }
                    lines.push(Line::from(Span::styled(
                        "└──",
                        Style::default().fg(palette.hint),
                    )));
                }
            } else {
                // Start of code block - extract language (first word after ```)
                in_code_block = true;
                let lang_str = trimmed.trim_start_matches('`');
                code_lang = Some(lang_str.split_whitespace().next().unwrap_or("").to_string());
            }
            continue;
        }

        if in_code_block {
            code_buffer.push(line_text.to_string());
            continue;
        }

        // Handle tool call markers
        if trimmed.starts_with("[Tool:") || trimmed.starts_with("⚙️") {
            lines.push(Line::from(vec![
                Span::styled("  🔧 ", Style::default()),
                Span::styled(
                    line_text.trim().to_string(),
                    Style::default()
                        .fg(palette.tool)
                        .add_modifier(Modifier::ITALIC),
                ),
            ]));
            continue;
        }

        // Try to detect and format JSON objects on a single line
        if ((trimmed.starts_with('{') && trimmed.ends_with('}'))
            || (trimmed.starts_with('[') && trimmed.ends_with(']')))
            && let Ok(json_val) = serde_json::from_str::<serde_json::Value>(trimmed)
        {
            // Pretty print JSON
            if let Ok(pretty) = serde_json::to_string_pretty(&json_val) {
                lines.push(Line::from(Span::styled(
                    "  ┌── JSON",
                    Style::default().fg(palette.hint),
                )));
                for json_line in pretty.lines() {
                    lines.push(Line::from(vec![
                        Span::styled("  │ ", Style::default().fg(palette.hint)),
                        Span::styled(
                            json_line.to_string(),
                            Style::default().fg(palette.accent_alt),
                        ),
                    ]));
                }
                lines.push(Line::from(Span::styled(
                    "  └──",
                    Style::default().fg(palette.hint),
                )));
                continue;
            }
        }

        // Markdown-aware inline rendering with search highlight
        let mut base = Style::default();
        let mut content_body = line_text.to_string();
        let mut prefix = "  ".to_string();

        if trimmed.starts_with('#') {
            let hashes = trimmed.chars().take_while(|c| *c == '#').count();
            let after = trimmed[hashes..].trim_start();
            content_body = after.to_string();
            base = base
                .fg(palette.accent_alt)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
            prefix = format!("{} ", "#".repeat(hashes));
        } else if trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || trimmed.starts_with("+ ")
        {
            content_body = trimmed[2..].trim_start().to_string();
            prefix = " • ".to_string();
        } else if trimmed.starts_with('>') {
            content_body = trimmed.trim_start_matches('>').trim_start().to_string();
            prefix = " ❯ ".to_string();
            base = base.add_modifier(Modifier::ITALIC).fg(palette.hint);
        }

        let rendered =
            render_inline_markdown_line(&format!("{prefix}{content_body}"), query, palette, base);
        lines.push(rendered);
    }

    // Handle unclosed code block
    if in_code_block && !code_buffer.is_empty() {
        lines.push(Line::from(Span::styled(
            "┌── code",
            Style::default().fg(palette.hint),
        )));
        for code_line in code_buffer {
            lines.push(Line::from(vec![
                Span::styled("│ ", Style::default().fg(palette.hint)),
                Span::styled(
                    code_line,
                    Style::default().fg(palette.fg).bg(palette.surface),
                ),
            ]));
        }
        lines.push(Line::from(Span::styled(
            "└──",
            Style::default().fg(palette.hint),
        )));
    }

    lines
}

/// Render parsed content lines from a conversation for the detail modal.
/// Parses tool use, code blocks, and formats beautifully for human reading.
pub fn render_parsed_content(
    detail: &ConversationView,
    query: &str,
    palette: ThemePalette,
) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    // Header with conversation info
    if let Some(title) = &detail.convo.title {
        lines.push(Line::from(vec![
            Span::styled("📋 ", Style::default()),
            Span::styled(
                title.clone(),
                Style::default()
                    .fg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(""));
    }

    // Workspace info
    if let Some(ws) = &detail.workspace {
        lines.push(Line::from(vec![
            Span::styled("📁 Workspace: ", Style::default().fg(palette.hint)),
            Span::styled(
                ws.display_name
                    .clone()
                    .unwrap_or_else(|| ws.path.display().to_string()),
                Style::default().fg(palette.fg),
            ),
        ]));
        lines.push(Line::from(""));
    }

    // Time info
    if let Some(ts) = detail.convo.started_at {
        lines.push(Line::from(vec![
            Span::styled("🕐 Started: ", Style::default().fg(palette.hint)),
            Span::styled(
                format_absolute_time(ts),
                Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "─".repeat(60),
        Style::default().fg(palette.hint),
    )));
    lines.push(Line::from(""));

    // Render messages with beautiful formatting
    for msg in &detail.messages {
        let (role_icon, role_label, role_color) = match &msg.role {
            MessageRole::User => ("👤", "You", palette.user),
            MessageRole::Agent => ("🤖", "Assistant", palette.agent),
            MessageRole::Tool => ("🔧", "Tool", palette.tool),
            MessageRole::System => ("⚙️", "System", palette.system),
            MessageRole::Other(r) => ("📝", r.as_str(), palette.hint),
        };

        // Role header with timestamp
        let ts_text = msg
            .created_at
            .map(|t| format!(" · {}", format_absolute_time(t)))
            .unwrap_or_default();
        lines.push(Line::from(vec![
            Span::styled(format!("{role_icon} "), Style::default()),
            Span::styled(
                role_label.to_string(),
                Style::default().fg(role_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(ts_text, Style::default().fg(palette.hint)),
        ]));
        lines.push(Line::from(""));

        // Parse and render content
        let content = &msg.content;
        let parsed_lines = parse_message_content(content, query, palette);
        lines.extend(parsed_lines);
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "─".repeat(60),
            Style::default()
                .fg(palette.hint)
                .add_modifier(Modifier::DIM),
        )));
        lines.push(Line::from(""));
    }

    lines
}
