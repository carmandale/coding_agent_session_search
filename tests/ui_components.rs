use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use coding_agent_search::ui::components::theme::ThemePalette;
use coding_agent_search::ui::components::widgets::search_bar;
use coding_agent_search::ui::data::InputMode;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Tabs;

#[test]
fn search_bar_tips_include_clear_hotkeys() {
    let palette = ThemePalette::dark();
    let widget = search_bar(
        "test",
        palette,
        InputMode::Query,
        "standard",
        vec![Span::raw("[agent:codex] ")],
    );
    let rect = Rect::new(0, 0, 100, 4);
    let mut buf = Buffer::empty(rect);
    widget.render(rect, &mut buf);

    let lines: Vec<String> = (0..rect.height)
        .map(|y| {
            (0..rect.width)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<Vec<_>>()
                .join("")
        })
        .collect();
    let joined = lines.join("\n");
    eprintln!("bar={joined}");
    // Simplified tips line now shows F1 help, F3-F5 filters, and Ctrl+Del
    assert!(joined.contains("F1"));
    assert!(joined.contains("help"));
    assert!(joined.contains("F3"));
    assert!(joined.contains("agent"));
    assert!(joined.contains("F5"));
    assert!(joined.contains("time"));
    assert!(joined.contains("Ctrl+Del"));
    assert!(joined.contains("clear"));
}

#[test]
fn filter_pills_render_selected_filters() {
    let palette = ThemePalette::dark();
    let chips = vec![
        Span::styled(
            "[agent:codex] ",
            Style::default()
                .fg(palette.accent_alt)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("[ws:/ws/demo] ", Style::default().fg(palette.accent_alt)),
        Span::styled(
            "[time:Some(100)->Some(200)] ",
            Style::default().fg(palette.accent_alt),
        ),
    ];

    let widget = search_bar("test", palette, InputMode::Query, "standard", chips);
    let rect = Rect::new(0, 0, 100, 4);
    let mut buf = Buffer::empty(rect);
    widget.render(rect, &mut buf);
    let lines: Vec<String> = (0..rect.height)
        .map(|y| {
            (0..rect.width)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<Vec<_>>()
                .join("")
        })
        .collect();
    let joined = lines.join("\n");
    assert!(joined.contains("[agent:codex]"));
    assert!(joined.contains("[ws:/ws/demo]"));
    assert!(joined.contains("[time:Some(100)->Some(200)]"));
}

#[test]
fn detail_tabs_labels_present() {
    let palette = ThemePalette::dark();
    let tabs = ["Messages", "Snippets", "Raw"];
    let tab_titles: Vec<Line> = tabs
        .iter()
        .map(|t| Line::from(Span::styled(*t, palette.title())))
        .collect();
    let widget = Tabs::new(tab_titles);

    let mut buf = Buffer::empty(Rect::new(0, 0, 40, 1));
    widget.render(Rect::new(0, 0, 40, 1), &mut buf);
    let line: String = (0..40).map(|x| buf[(x, 0)].symbol().to_string()).collect();
    eprintln!("tabs={line}");
    assert!(line.contains("Messages"));
    assert!(line.contains("Snippets"));
    assert!(line.contains("Raw"));
}

fn rel_luminance(c: Color) -> f64 {
    let (r, g, b) = match c {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Black => (0, 0, 0),
        Color::White => (255, 255, 255),
        Color::Indexed(i) => {
            let v = i as f64 / 255.0;
            let scaled = (v * 255.0).round() as u8;
            (scaled, scaled, scaled)
        }
        _ => (128, 128, 128),
    };

    let to_linear = |v: u8| {
        let srgb = v as f64 / 255.0;
        if srgb <= 0.03928 {
            srgb / 12.92
        } else {
            ((srgb + 0.055) / 1.055).powf(2.4)
        }
    };

    0.2126 * to_linear(r) + 0.7152 * to_linear(g) + 0.0722 * to_linear(b)
}

fn contrast_ratio(a: Color, b: Color) -> f64 {
    let la = rel_luminance(a);
    let lb = rel_luminance(b);
    let (bright, dark) = if la > lb { (la, lb) } else { (lb, la) };
    (bright + 0.05) / (dark + 0.05)
}

/// Ensure agent pane themes remain visually distinct and legible.
#[test]
fn agent_pane_colors_are_distinct_and_legible() {
    use coding_agent_search::ui::components::theme::PaneTheme;

    let agents = vec![
        "codex",
        "claude_code",
        "cline",
        "gemini",
        "amp",
        "aider",
        "cursor",
        "chatgpt",
        "opencode",
    ];

    let panes: Vec<(String, PaneTheme)> = agents
        .iter()
        .map(|a| (a.to_string(), ThemePalette::agent_pane(a)))
        .collect();

    // Text and accent should be readable over background
    for (name, pane) in &panes {
        let fg_ratio = contrast_ratio(pane.bg, pane.fg);
        let accent_ratio = contrast_ratio(pane.bg, pane.accent);
        assert!(
            fg_ratio >= 3.0,
            "fg contrast for {name} too low: {fg_ratio:.2}"
        );
        assert!(
            accent_ratio >= 3.0,
            "accent contrast for {name} too low: {accent_ratio:.2}"
        );
    }

    // Backgrounds should be clearly distinct between agents (avoid near-duplicates)
    let mut min_distance = f64::MAX;
    for i in 0..panes.len() {
        for j in (i + 1)..panes.len() {
            let bg_a = panes[i].1.bg;
            let bg_b = panes[j].1.bg;
            let (ra, ga, ba) = match bg_a {
                Color::Rgb(r, g, b) => (r as f64, g as f64, b as f64),
                Color::Black => (0.0, 0.0, 0.0),
                Color::White => (255.0, 255.0, 255.0),
                Color::Indexed(v) => {
                    let p = v as f64;
                    (p, p, p)
                }
                _ => (128.0, 128.0, 128.0),
            };
            let (rb, gb, bb) = match bg_b {
                Color::Rgb(r, g, b) => (r as f64, g as f64, b as f64),
                Color::Black => (0.0, 0.0, 0.0),
                Color::White => (255.0, 255.0, 255.0),
                Color::Indexed(v) => {
                    let p = v as f64;
                    (p, p, p)
                }
                _ => (128.0, 128.0, 128.0),
            };
            let dist = ((ra - rb).powi(2) + (ga - gb).powi(2) + (ba - bb).powi(2)).sqrt();
            min_distance = min_distance.min(dist);
        }
    }

    assert!(
        min_distance >= 25.0,
        "agent backgrounds too similar (min distance {min_distance:.2})"
    );
}
