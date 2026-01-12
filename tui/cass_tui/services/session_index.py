from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
import json
import subprocess
import os


@dataclass
class Session:
    source_path: Path
    agent: str
    workspace: str | None
    title: str | None
    started_at: datetime | None
    message_count: int
    
    @property
    def age_seconds(self) -> float:
        if self.started_at is None:
            return float("inf")
        return (datetime.now() - self.started_at).total_seconds()
    
    @property
    def display_name(self) -> str:
        if self.title:
            return self.title[:60]
        return self.source_path.name[:60]


@dataclass
class AgentSessions:
    agent: str
    sessions: list[Session] = field(default_factory=list)
    
    @property
    def count(self) -> int:
        return len(self.sessions)


class SessionIndex:
    KNOWN_AGENTS = (
        "claude_code",
        "codex",
        "cursor",
        "gemini",
        "opencode",
        "amp",
        "cline",
        "aider",
        "chatgpt",
        "pi_agent",
    )
    
    def __init__(self, workspace: Path | None = None, filter_by_workspace: bool = False):
        self.workspace = workspace or Path.cwd()
        self.filter_by_workspace = filter_by_workspace
        self._cache: dict[str, AgentSessions] = {}
    
    def _run_cass_search(self, *args: str) -> dict | None:
        try:
            result = subprocess.run(
                ["cass", *args, "--json"],
                capture_output=True,
                text=True,
                timeout=30,
            )
            if result.returncode != 0:
                return None
            parsed = json.loads(result.stdout)
            if isinstance(parsed, dict):
                return parsed
            return None
        except (subprocess.TimeoutExpired, json.JSONDecodeError, FileNotFoundError):
            return None
    
    def get_sessions_for_agent(self, agent: str) -> list[Session]:
        args = ["search", "", "--agent", agent, "--limit", "100"]
        if self.filter_by_workspace:
            args.extend(["--workspace", str(self.workspace)])
        data = self._run_cass_search(*args)
        
        if not data or "hits" not in data:
            return []
        
        sessions: list[Session] = []
        seen_paths: set[str] = set()
        
        for hit in data.get("hits", []):
            source_path = hit.get("source_path", "")
            if source_path in seen_paths:
                continue
            seen_paths.add(source_path)
            
            started_at = None
            if ts := hit.get("created_at"):
                try:
                    if isinstance(ts, int):
                        started_at = datetime.fromtimestamp(ts / 1000)
                    else:
                        started_at = datetime.fromisoformat(ts.replace("Z", "+00:00"))
                except (ValueError, OSError):
                    pass
            
            sessions.append(Session(
                source_path=Path(source_path),
                agent=agent,
                workspace=hit.get("workspace"),
                title=hit.get("title"),
                started_at=started_at,
                message_count=1,
            ))
        
        sessions.sort(key=lambda s: s.started_at or datetime.min, reverse=True)
        return sessions
    
    def get_all_agents_with_sessions(self) -> list[AgentSessions]:
        result: list[AgentSessions] = []
        
        for agent in self.KNOWN_AGENTS:
            sessions = self.get_sessions_for_agent(agent)
            if sessions:
                result.append(AgentSessions(agent=agent, sessions=sessions))
        
        result.sort(key=lambda a: a.count, reverse=True)
        return result
    
    def get_stats(self) -> dict:
        data = self._run_cass_search("stats")
        return data if data else {}
    
    def refresh(self) -> None:
        self._cache.clear()
