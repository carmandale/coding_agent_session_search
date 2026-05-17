---
title: "feat: Sync cass sessions to Mac Mini for OpenClaw"
type: feat
status: active
date: 2026-03-05
bead: coding_agent_session_search-9wz3
---

# Plan: Sync cass Sessions to Mac Mini

## Approach

Push the cass SQLite database and tantivy search index from this MacBook to the Mac Mini via rsync, triggered by a macOS launchd agent on a nightly schedule. A wrapper script handles reachability checks and logging. cass is installed on the Mini for OpenClaw to query.

## Architecture

```
┌─────────────────────────────┐         ┌──────────────────────────────┐
│  MacBook (source of truth)  │   SSH   │  Mac Mini (mini-ts)          │
│                             │ ──────► │                              │
│  ~/.claude/projects/        │  rsync  │  ~/cass-mirror/              │
│  ~/.codex/sessions/         │         │    agent_search.db           │
│  ~/.pi/agent/sessions/      │         │    index/                    │
│  ... (all agent dirs)       │         │                              │
│           │                 │         │  cass --db ~/cass-mirror/... │
│           ▼                 │         │           ▲                  │
│  cass index (local)         │         │           │                  │
│           │                 │         │  OpenClaw queries cass       │
│           ▼                 │         │                              │
│  agent_search.db + index/   │         │                              │
│  (~/Library/App Support/    │         │                              │
│   com.coding-agent-search/) │         │                              │
└─────────────────────────────┘         └──────────────────────────────┘
```

**Data flow**: Raw sessions → cass indexes locally → DB syncs to mini → OpenClaw queries synced DB

## Key Decisions

### Why push DB, not raw sessions?

- The DB is **6.2 GB** vs **~23 GB** of raw session files across 7+ agent directories
- rsync on SQLite with WAL mode is efficient — delta transfers for append-heavy workloads
- No need to maintain a list of agent directories on the sync side
- New agents are automatically included (cass indexes them, DB contains everything)

### Why launchd on MacBook, not cron on mini?

- MacBook already has `mini-ts` in SSH config — known, stable connection
- Mini would need reverse SSH to MacBook, whose IP/hostname can change
- MacBook is the data owner — push model is simpler than pull

### Why not cass sources sync?

- `cass sources sync` is designed for pulling raw files from remotes and re-indexing locally
- It requires SSH from the mini back to the MacBook (reverse direction)
- Syncing the pre-built DB is simpler and faster

### rsync + SQLite safety

- We sync a **checkpoint** copy: `PRAGMA wal_checkpoint(TRUNCATE)` before rsync to ensure the DB file is self-contained (no WAL dependency)
- Alternatively, use `.backup` command to create a clean copy, then rsync that
- The tantivy index directory is safe to rsync (read-only on the remote side)

## Components

### 1. Install cass on Mac Mini

```bash
ssh mini-ts "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
ssh mini-ts "source ~/.cargo/env && cargo install coding-agent-search"
ssh mini-ts "mkdir -p ~/cass-mirror/index"
```

Verify: `ssh mini-ts "cass --version"`

### 2. Sync Wrapper Script

Location: `~/.local/bin/cass-sync-to-mini.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail

LOG="$HOME/Library/Logs/cass-sync-to-mini.log"
REMOTE="mini-ts"
REMOTE_DIR="cass-mirror"
LOCAL_DB_DIR="$HOME/Library/Application Support/com.coding-agent-search.coding-agent-search"
TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')

log() { echo "[$TIMESTAMP] $*" >> "$LOG"; }

# Reachability check (timeout 5 seconds)
if ! ssh -o ConnectTimeout=5 -o BatchMode=yes "$REMOTE" "echo ok" &>/dev/null; then
    log "SKIP: $REMOTE unreachable"
    exit 0
fi

log "START: syncing to $REMOTE"

# Checkpoint WAL to ensure DB is self-contained
sqlite3 "$LOCAL_DB_DIR/agent_search.db" "PRAGMA wal_checkpoint(TRUNCATE);" 2>/dev/null || true

# Sync database
rsync -az --info=progress2 \
    "$LOCAL_DB_DIR/agent_search.db" \
    "$REMOTE:~/$REMOTE_DIR/agent_search.db"

# Sync search index
rsync -az --delete \
    "$LOCAL_DB_DIR/index/v6/" \
    "$REMOTE:~/$REMOTE_DIR/index/v6/"

log "DONE: sync complete"
```

### 3. launchd Plist

Location: `~/Library/LaunchAgents/com.cass.sync-to-mini.plist`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.cass.sync-to-mini</string>
    <key>ProgramArguments</key>
    <array>
        <string>/bin/bash</string>
        <string>-l</string>
        <string>-c</string>
        <string>~/.local/bin/cass-sync-to-mini.sh</string>
    </array>
    <key>StartCalendarInterval</key>
    <dict>
        <key>Hour</key>
        <integer>2</integer>
        <key>Minute</key>
        <integer>0</integer>
    </dict>
    <key>StandardOutPath</key>
    <string>/tmp/cass-sync-to-mini.stdout.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/cass-sync-to-mini.stderr.log</string>
</dict>
</plist>
```

Runs at **2:00 AM** daily — late enough that both machines are likely on the same network.

### 4. OpenClaw Integration

OpenClaw on the mini queries with:
```bash
cass --db ~/cass-mirror/agent_search.db search "query" --robot --limit 10
```

Or set an environment variable / alias:
```bash
# In ~/.zshrc on mini:
export CASS_DB="$HOME/cass-mirror/agent_search.db"
alias cass='cass --db "$CASS_DB"'
```

## Risks & Mitigations (Updated Post-Implementation)

| Risk | Mitigation |
|------|------------|
| SQLite corruption during rsync | WAL checkpoint before sync ensures self-contained DB |
| Apple rsync 3.4.1 deflate bug | **Hit this!** rsync crashes on delta transfer of 6GB+ files. Using `scp -C` instead (~6 min full transfer, reliable) |
| FTS corruption from mid-write sync | 2 AM schedule avoids active writes. If needed, `cass index --full` locally first |
| Mini disk space | 6.2 GB DB + 2.1 GB index = ~8.3 GB — Mini has 246 GB free |
| launchd DNS resolution | Tailscale MagicDNS works in launchd via SSH config alias; also has Tailscale IP fallback |
| launchd doesn't fire (lid closed) | macOS wakes for launchd calendar events; also can run manually with `launchctl start com.cass.sync-to-mini` |
| SSH auth | Uses Tailscale "none" auth — no keys needed |
| cass version mismatch | MacBook: v0.1.55, Mini: v0.1.64 — DB format compatible |
