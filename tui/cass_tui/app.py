from pathlib import Path
from textual import work
from textual.app import App, ComposeResult
from textual.binding import Binding
from textual.containers import Horizontal, Vertical, VerticalScroll
from textual.reactive import reactive
from textual.widgets import Footer, Header, RichLog, Static, ListView, ListItem, Label, Rule
from textual.worker import get_current_worker

from cass_tui.services import SessionIndex, Session, AgentSessions


AGENT_COLORS = {
    "claude_code": "magenta",
    "codex": "green",
    "cursor": "cyan",
    "gemini": "blue",
    "opencode": "yellow",
    "amp": "red",
    "cline": "bright_magenta",
    "aider": "bright_green",
    "chatgpt": "bright_cyan",
    "pi_agent": "bright_blue",
}


def agent_color(agent: str) -> str:
    return AGENT_COLORS.get(agent, "white")


class AgentListItem(ListItem):
    def __init__(self, agent_sessions: AgentSessions) -> None:
        super().__init__()
        self.agent_sessions = agent_sessions
    
    def compose(self) -> ComposeResult:
        agent = self.agent_sessions.agent
        count = self.agent_sessions.count
        color = agent_color(agent)
        yield Label(f"[{color}]●[/{color}] {agent} [dim]({count})[/dim]")


class SessionListItem(ListItem):
    def __init__(self, session: Session) -> None:
        super().__init__()
        self.session = session
    
    def compose(self) -> ComposeResult:
        age_seconds = self.session.age_seconds
        # Handle infinity or very large values
        if age_seconds == float('inf') or age_seconds > 365 * 86400 * 100:  # > 100 years
            age_str = "?"
            date_str = ""
        else:
            age = int(age_seconds)
            if age < 60:
                age_str = f"{age}s"
            elif age < 3600:
                age_str = f"{age // 60}m"
            elif age < 86400:
                age_str = f"{age // 3600}h"
            else:
                age_str = f"{age // 86400}d"
            
            # Format date/time
            if self.session.started_at:
                date_str = self.session.started_at.strftime("%m/%d %H:%M")
            else:
                date_str = ""
        
        color = agent_color(self.session.agent)
        name = self.session.display_name
        # Show: name (date time) age
        if date_str:
            yield Label(f"[{color}]●[/{color}] {name}\n  [dim]{date_str} ({age_str} ago)[/dim]")
        else:
            yield Label(f"[{color}]●[/{color}] {name} [dim]({age_str})[/dim]")


class SessionDetailPanel(Static):
    def __init__(self, id: str | None = None) -> None:
        super().__init__(id=id)
        self._session: Session | None = None
    
    def compose(self) -> ComposeResult:
        yield Static("[dim]Select a session to view details[/dim]", id="detail-content")
    
    def show_session(self, session: Session) -> None:
        self._session = session
        content = self.query_one("#detail-content", Static)
        
        color = agent_color(session.agent)
        lines = [
            f"[bold {color}]{session.agent.upper()}[/bold {color}]",
            "",
            f"[bold]Title:[/bold] {session.title or 'Untitled'}",
            f"[bold]Path:[/bold] {session.source_path}",
        ]
        
        if session.workspace:
            lines.append(f"[bold]Workspace:[/bold] {session.workspace}")
        
        if session.started_at:
            lines.append(f"[bold]Started:[/bold] {session.started_at.strftime('%Y-%m-%d %H:%M:%S')}")
        
        lines.append("")
        lines.append("[dim]Press Enter to open in editor[/dim]")
        
        content.update("\n".join(lines))


class AgentSessionsPanel(Static):
    def __init__(self, session_index: SessionIndex, id: str | None = None) -> None:
        super().__init__(id=id)
        self.session_index = session_index
        self._agents: list[AgentSessions] = []
    
    def compose(self) -> ComposeResult:
        yield Static("[bold cyan]Agents[/bold cyan]", id="agents-header")
        yield ListView(id="agent-list")
    
    def refresh_agents(self) -> None:
        self._agents = self.session_index.get_all_agents_with_sessions()
        agent_list = self.query_one("#agent-list", ListView)
        agent_list.clear()
        
        if not self._agents:
            agent_list.append(ListItem(Label("[dim]No sessions found[/dim]")))
        else:
            for agent_sessions in self._agents:
                agent_list.append(AgentListItem(agent_sessions))
    
    @property
    def agents(self) -> list[AgentSessions]:
        return self._agents


class CassTuiApp(App):
    CSS = """
    #main-container {
        height: 1fr;
    }
    
    #sidebar {
        width: 35;
        border-right: solid $primary;
    }
    
    #agents-panel {
        height: auto;
        max-height: 12;
        padding: 0 1;
        border-bottom: solid $primary-darken-2;
        background: $surface-darken-1;
    }
    
    #agents-header {
        height: 1;
        margin-bottom: 1;
    }
    
    #agent-list {
        height: auto;
        max-height: 8;
    }
    
    #sessions-header {
        height: 1;
        padding: 0 1;
        background: $surface-darken-2;
    }
    
    #session-list {
        height: 1fr;
    }
    
    #detail-container {
        width: 1fr;
    }
    
    #status-bar {
        height: 1;
        background: $primary;
        color: $text;
        padding: 0 1;
    }
    
    #detail-panel {
        height: 1fr;
        padding: 1 2;
    }
    
    #detail-content {
        height: 1fr;
    }
    
    ListView {
        background: $surface;
    }
    
    ListItem {
        padding: 0 1;
        height: auto;
    }
    
    ListItem:hover {
        background: $primary-darken-2;
    }
    
    #session-list ListItem {
        height: auto;
        min-height: 2;
    }
    """
    
    BINDINGS = [
        Binding("q", "quit", "Quit"),
        Binding("r", "refresh", "Refresh"),
        Binding("i", "reindex", "Re-index"),
        Binding("space", "open_session", "Open"),
        Binding("?", "help", "Help"),
    ]
    
    current_agent: reactive[str | None] = reactive(None)
    current_session: reactive[Session | None] = reactive(None)
    
    def __init__(self, workspace: Path | None = None) -> None:
        super().__init__()
        self.session_index = SessionIndex(workspace)
        self._agent_sessions: dict[str, AgentSessions] = {}
    
    def compose(self) -> ComposeResult:
        yield Header()
        
        with Horizontal(id="main-container"):
            with Vertical(id="sidebar"):
                yield AgentSessionsPanel(self.session_index, id="agents-panel")
                yield Static("[bold]Sessions[/bold]", id="sessions-header")
                yield ListView(id="session-list")
            
            with Vertical(id="detail-container"):
                yield Static("Loading...", id="status-bar")
                yield SessionDetailPanel(id="detail-panel")
        
        yield Footer()
    
    def on_mount(self) -> None:
        self.title = "CASS Sessions"
        self.sub_title = f"Workspace: {self.session_index.workspace}"
        self.refresh_data()
    
    @work(exclusive=True, thread=True)
    def refresh_data(self) -> None:
        agents_panel = self.query_one("#agents-panel", AgentSessionsPanel)
        self.call_from_thread(agents_panel.refresh_agents)
        self.call_from_thread(self._update_status)
        
        if agents_panel.agents:
            first_agent = agents_panel.agents[0].agent
            self.call_from_thread(self._set_current_agent, first_agent)
    
    def _set_current_agent(self, agent: str) -> None:
        self.current_agent = agent
    
    def _update_status(self) -> None:
        agents_panel = self.query_one("#agents-panel", AgentSessionsPanel)
        total_agents = len(agents_panel.agents)
        total_sessions = sum(a.count for a in agents_panel.agents)
        
        status_bar = self.query_one("#status-bar", Static)
        status_bar.update(
            f"[bold]CASS[/bold] │ {total_agents} agents │ {total_sessions} sessions"
        )
    
    def watch_current_agent(self, agent: str | None) -> None:
        if agent is None:
            return
        
        agents_panel = self.query_one("#agents-panel", AgentSessionsPanel)
        agent_sessions = next(
            (a for a in agents_panel.agents if a.agent == agent),
            None
        )
        
        if agent_sessions is None:
            return
        
        self._agent_sessions[agent] = agent_sessions
        
        session_list = self.query_one("#session-list", ListView)
        session_list.clear()
        
        for session in agent_sessions.sessions:
            session_list.append(SessionListItem(session))
        
        if agent_sessions.sessions:
            self.current_session = agent_sessions.sessions[0]
    
    def watch_current_session(self, session: Session | None) -> None:
        if session is None:
            return
        
        detail_panel = self.query_one("#detail-panel", SessionDetailPanel)
        detail_panel.show_session(session)
    
    def on_list_view_selected(self, event: ListView.Selected) -> None:
        if isinstance(event.item, AgentListItem):
            self.current_agent = event.item.agent_sessions.agent
        elif isinstance(event.item, SessionListItem):
            self.current_session = event.item.session
    
    def action_refresh(self) -> None:
        self.refresh_data()
        self.notify("Refreshing sessions...", title="Refresh")
    
    def action_open_session(self) -> None:
        if self.current_session is None:
            return
        
        import subprocess
        import shutil
        
        path = str(self.current_session.source_path)
        
        # Suspend the TUI and show the session in a pager
        with self.suspend():
            try:
                # Export to markdown
                export_result = subprocess.run(
                    ["cass", "export", path, "--format", "markdown"],
                    capture_output=True,
                    text=True,
                    timeout=30,
                )
                
                if export_result.returncode != 0:
                    print(f"Error exporting session: {export_result.stderr}")
                    input("Press Enter to continue...")
                    return
                
                markdown_content = export_result.stdout
                
                # Try glow first (nice markdown rendering), fall back to less
                if shutil.which("glow"):
                    pager = subprocess.Popen(
                        ["glow", "-p"],
                        stdin=subprocess.PIPE,
                        text=True,
                    )
                    pager.communicate(input=markdown_content)
                elif shutil.which("less"):
                    pager = subprocess.Popen(
                        ["less", "-R"],
                        stdin=subprocess.PIPE,
                        text=True,
                    )
                    pager.communicate(input=markdown_content)
                else:
                    # Fallback: just print and wait
                    print(markdown_content)
                    input("\nPress Enter to continue...")
                    
            except subprocess.TimeoutExpired:
                print("Export timed out")
                input("Press Enter to continue...")
            except FileNotFoundError:
                print("cass command not found")
                input("Press Enter to continue...")
    
    def action_help(self) -> None:
        self.notify(
            "q=quit  r=refresh  i=reindex  Space=open  ?=help",
            title="Keyboard Shortcuts",
        )
    
    @work(exclusive=True, thread=True)
    def action_reindex(self) -> None:
        """Run cass index --full to rebuild the search index."""
        import subprocess
        
        self.call_from_thread(
            self.notify,
            "Running cass index --full...",
            title="Re-indexing",
            timeout=10,
        )
        
        try:
            result = subprocess.run(
                ["cass", "index", "--full"],
                capture_output=True,
                text=True,
                timeout=300,  # 5 minute timeout
            )
            
            if result.returncode == 0:
                self.call_from_thread(
                    self.notify,
                    "Index rebuilt successfully! Refreshing...",
                    title="Re-index Complete",
                    severity="information",
                )
                # Refresh the data after reindexing
                self.call_from_thread(self.refresh_data)
            else:
                error_msg = result.stderr[:200] if result.stderr else "Unknown error"
                self.call_from_thread(
                    self.notify,
                    f"Re-index failed: {error_msg}",
                    title="Error",
                    severity="error",
                )
        except subprocess.TimeoutExpired:
            self.call_from_thread(
                self.notify,
                "Re-index timed out after 5 minutes",
                title="Timeout",
                severity="error",
            )
        except FileNotFoundError:
            self.call_from_thread(
                self.notify,
                "cass command not found. Is it installed?",
                title="Error",
                severity="error",
            )
