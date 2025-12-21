//! Sessions TUI - Shows all agent sessions in current repository grouped by agent.

use anyhow::Result;
use chrono::{DateTime, Utc};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use std::io;
use std::path::{Path, PathBuf};

use crate::storage::sqlite::SqliteStorage;

/// Focus state for the TUI
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Focus {
    Search,
    Agents,
    Sessions,
}

#[derive(Clone, Debug)]
struct SessionInfo {
    #[allow(dead_code)] // Used for future session detail view (fetch_messages)
    id: i64,
    agent_slug: String,
    title: Option<String>,
    source_path: PathBuf,
    started_at: Option<i64>,
    message_count: usize,
}

struct SessionsState {
    all_sessions: Vec<SessionInfo>,
    sessions_by_agent: Vec<(String, Vec<SessionInfo>)>,
    selected_agent: usize,
    selected_session: ListState,
    current_workspace: Option<PathBuf>,
    search_query: String,
    focus: Focus,
}

impl SessionsState {
    fn new(sessions: Vec<SessionInfo>, current_workspace: Option<PathBuf>) -> Self {
        let sessions_by_agent = Self::group_sessions(&sessions);

        let mut state = ListState::default();
        if !sessions_by_agent.is_empty()
            && !sessions_by_agent
                .first()
                .map(|(_, s)| s.is_empty())
                .unwrap_or(true)
        {
            state.select(Some(0));
        }

        Self {
            all_sessions: sessions,
            sessions_by_agent,
            selected_agent: 0,
            selected_session: state,
            current_workspace,
            search_query: String::new(),
            focus: Focus::Search,
        }
    }

    fn group_sessions(sessions: &[SessionInfo]) -> Vec<(String, Vec<SessionInfo>)> {
        let mut by_agent: std::collections::BTreeMap<String, Vec<SessionInfo>> =
            std::collections::BTreeMap::new();
        for session in sessions {
            by_agent
                .entry(session.agent_slug.clone())
                .or_default()
                .push(session.clone());
        }
        by_agent.into_iter().collect()
    }

    fn apply_filter(&mut self) {
        let query = self.search_query.to_lowercase();
        let filtered: Vec<SessionInfo> = if query.is_empty() {
            self.all_sessions.clone()
        } else {
            self.all_sessions
                .iter()
                .filter(|s| {
                    let title_match = s
                        .title
                        .as_ref()
                        .map(|t| t.to_lowercase().contains(&query))
                        .unwrap_or(false);
                    let agent_match = s.agent_slug.to_lowercase().contains(&query);
                    let date_match = s
                        .started_at
                        .and_then(|ts| DateTime::<Utc>::from_timestamp(ts, 0))
                        .map(|dt| dt.format("%Y-%m-%d").to_string().contains(&query))
                        .unwrap_or(false);
                    title_match || agent_match || date_match
                })
                .cloned()
                .collect()
        };

        self.sessions_by_agent = Self::group_sessions(&filtered);
        self.selected_agent = 0;
        self.selected_session = ListState::default();
        if !self.sessions_by_agent.is_empty()
            && !self
                .sessions_by_agent
                .first()
                .map(|(_, s)| s.is_empty())
                .unwrap_or(true)
        {
            self.selected_session.select(Some(0));
        }
    }

    fn next_agent(&mut self) {
        if self.sessions_by_agent.is_empty() {
            return;
        }
        self.selected_agent = (self.selected_agent + 1) % self.sessions_by_agent.len();
        self.selected_session.select(Some(0));
    }

    fn prev_agent(&mut self) {
        if self.sessions_by_agent.is_empty() {
            return;
        }
        self.selected_agent = if self.selected_agent == 0 {
            self.sessions_by_agent.len() - 1
        } else {
            self.selected_agent - 1
        };
        self.selected_session.select(Some(0));
    }

    fn next_session(&mut self) {
        if let Some((_, sessions)) = self.sessions_by_agent.get(self.selected_agent) {
            if sessions.is_empty() {
                return;
            }
            let i = match self.selected_session.selected() {
                Some(i) => (i + 1) % sessions.len(),
                None => 0,
            };
            self.selected_session.select(Some(i));
        }
    }

    fn prev_session(&mut self) {
        if let Some((_, sessions)) = self.sessions_by_agent.get(self.selected_agent) {
            if sessions.is_empty() {
                return;
            }
            let i = match self.selected_session.selected() {
                Some(i) => {
                    if i == 0 {
                        sessions.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.selected_session.select(Some(i));
        }
    }

    fn get_selected_session(&self) -> Option<&SessionInfo> {
        let (_, sessions) = self.sessions_by_agent.get(self.selected_agent)?;
        let idx = self.selected_session.selected()?;
        sessions.get(idx)
    }
}

pub fn run_sessions_tui(workspace: Option<&Path>, _data_dir: &Path, db_path: &Path) -> Result<()> {
    // Get current workspace if not provided
    let current_workspace = workspace.map(PathBuf::from).or_else(|| {
        std::env::current_dir()
            .ok()
            .and_then(|p| p.canonicalize().ok())
    });

    // Load sessions from database - query by workspace at SQL level for efficiency
    let storage = SqliteStorage::open(db_path)?;
    let conversations_with_counts =
        storage.list_conversations_for_workspace(current_workspace.as_deref(), 10000, 0)?;

    let sessions: Vec<SessionInfo> = conversations_with_counts
        .into_iter()
        .map(|(conv, msg_count)| SessionInfo {
            id: conv.id.unwrap_or(0),
            agent_slug: conv.agent_slug,
            title: conv.title,
            source_path: conv.source_path,
            started_at: conv.started_at,
            message_count: msg_count,
        })
        .collect();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = SessionsState::new(sessions, current_workspace);

    // Main event loop
    loop {
        terminal.draw(|f| render_ui(f, &mut state))?;

        if let Event::Key(key) = event::read()? {
            match state.focus {
                Focus::Search => match (key.code, key.modifiers) {
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,
                    (KeyCode::Esc, _) => {
                        if state.search_query.is_empty() {
                            break;
                        } else {
                            state.search_query.clear();
                            state.apply_filter();
                        }
                    }
                    (KeyCode::Tab, _) | (KeyCode::Down, _) => {
                        state.focus = Focus::Agents;
                    }
                    (KeyCode::Enter, _) => {
                        state.focus = Focus::Sessions;
                    }
                    (KeyCode::Backspace, _) => {
                        state.search_query.pop();
                        state.apply_filter();
                    }
                    (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                        state.search_query.push(c);
                        state.apply_filter();
                    }
                    _ => {}
                },
                Focus::Agents => match (key.code, key.modifiers) {
                    (KeyCode::Char('q'), _)
                    | (KeyCode::Esc, _)
                    | (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,
                    (KeyCode::Tab, _) | (KeyCode::Right, _) => {
                        state.focus = Focus::Sessions;
                    }
                    (KeyCode::BackTab, _) => {
                        state.focus = Focus::Search;
                    }
                    (KeyCode::Char('/'), _) => {
                        state.focus = Focus::Search;
                    }
                    (KeyCode::Down, _) | (KeyCode::Char('j'), _) => {
                        state.next_agent();
                    }
                    (KeyCode::Up, _) | (KeyCode::Char('k'), _) => {
                        state.prev_agent();
                    }
                    (KeyCode::Enter, _) => {
                        state.focus = Focus::Sessions;
                    }
                    _ => {}
                },
                Focus::Sessions => match (key.code, key.modifiers) {
                    (KeyCode::Char('q'), _)
                    | (KeyCode::Esc, _)
                    | (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,
                    (KeyCode::Tab, _) => {
                        state.focus = Focus::Search;
                    }
                    (KeyCode::BackTab, _) | (KeyCode::Left, _) => {
                        state.focus = Focus::Agents;
                    }
                    (KeyCode::Char('/'), _) => {
                        state.focus = Focus::Search;
                    }
                    (KeyCode::Down, _) | (KeyCode::Char('j'), _) => {
                        state.next_session();
                    }
                    (KeyCode::Up, _) | (KeyCode::Char('k'), _) => {
                        state.prev_session();
                    }
                    (KeyCode::Enter, _) => {
                        if let Some(session) = state.get_selected_session() {
                            let source_path = session.source_path.clone();
                            disable_raw_mode()?;
                            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

                            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "less".into());
                            let status = std::process::Command::new(&editor)
                                .arg(&source_path)
                                .status();

                            enable_raw_mode()?;
                            execute!(io::stdout(), EnterAlternateScreen)?;
                            terminal.clear()?;

                            if let Err(e) = status {
                                eprintln!("Failed to open editor: {e}");
                            }
                        }
                    }
                    _ => {}
                },
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(())
}

fn render_ui(f: &mut Frame, state: &mut SessionsState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search bar
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Help
        ])
        .split(f.area());

    // Search bar
    let search_style = if state.focus == Focus::Search {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Gray)
    };
    let search_text = if state.search_query.is_empty() {
        if state.focus == Focus::Search {
            "Type to filter sessions...".to_string()
        } else {
            "Press / to search".to_string()
        }
    } else {
        state.search_query.clone()
    };
    let search_block = Paragraph::new(search_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Search")
                .border_style(search_style),
        )
        .style(
            if state.search_query.is_empty() && state.focus != Focus::Search {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            },
        );
    f.render_widget(search_block, chunks[0]);

    // Title bar
    let title = if let Some(ref ws) = state.current_workspace {
        format!("Agent Sessions - Workspace: {}", ws.display())
    } else {
        "Agent Sessions - All Workspaces".to_string()
    };

    let session_count = state
        .sessions_by_agent
        .iter()
        .map(|(_, s)| s.len())
        .sum::<usize>();
    let title_with_count = format!("{} ({} sessions)", title, session_count);

    let title_block = Paragraph::new(title_with_count)
        .block(Block::default().borders(Borders::ALL).title("Sessions"))
        .style(Style::default().fg(Color::Cyan));
    f.render_widget(title_block, chunks[1]);

    // Content area - split into agents and sessions
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[2]);

    // Agent list (left panel)
    let agent_border_style = if state.focus == Focus::Agents {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };
    let agent_items: Vec<ListItem> = state
        .sessions_by_agent
        .iter()
        .enumerate()
        .map(|(idx, (agent, sessions))| {
            let prefix = if idx == state.selected_agent {
                "> "
            } else {
                "  "
            };
            let text = format!("{}{} ({})", prefix, agent, sessions.len());
            let style = if idx == state.selected_agent {
                Style::default().fg(Color::Yellow).bold()
            } else {
                Style::default()
            };
            ListItem::new(text).style(style)
        })
        .collect();

    let agent_list = List::new(agent_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Agents")
            .border_style(agent_border_style),
    );
    f.render_widget(agent_list, content_chunks[0]);

    // Session list (right panel)
    let session_border_style = if state.focus == Focus::Sessions {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };
    if let Some((agent, sessions)) = state.sessions_by_agent.get(state.selected_agent) {
        let session_items: Vec<ListItem> = sessions
            .iter()
            .map(|session| {
                let time_str = session
                    .started_at
                    .and_then(|ts| DateTime::<Utc>::from_timestamp(ts, 0))
                    .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "Unknown".to_string());

                let title = session
                    .title
                    .as_deref()
                    .unwrap_or("(untitled)")
                    .chars()
                    .take(50)
                    .collect::<String>();

                let text = format!("{} | {} msgs | {}", time_str, session.message_count, title);
                ListItem::new(text)
            })
            .collect();

        let session_list = List::new(session_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("{} Sessions", agent))
                    .border_style(session_border_style),
            )
            .highlight_style(Style::default().fg(Color::Black).bg(Color::Yellow))
            .highlight_symbol(">> ");

        f.render_stateful_widget(session_list, content_chunks[1], &mut state.selected_session);
    } else {
        let empty_msg = if state.search_query.is_empty() {
            "No sessions found"
        } else {
            "No sessions match your search"
        };
        let empty_block = Paragraph::new(empty_msg)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Sessions")
                    .border_style(session_border_style),
            )
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(empty_block, content_chunks[1]);
    }

    let help_text = "/: Search | Tab: Switch Focus | ↑↓/j/k: Navigate | Enter: Open | q/Esc: Quit";
    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .style(Style::default().fg(Color::Gray));
    f.render_widget(help, chunks[3]);
}
