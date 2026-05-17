---
shaping: true
title: "feat: Sync cass sessions to Mac Mini for OpenClaw"
type: feat
status: active
date: 2026-03-05
bead: coding_agent_session_search-9wz3
---

# Sync cass Sessions to Mac Mini for OpenClaw

## Source

> I want to get the cass sessions to the mac mini so that openclaw can have access to them. We can always `ssh mini-ts` when we need to. We are on the same network at night. I want this to at a minimum, sync daily. We can probably install cass on the mini.

## Problem

OpenClaw runs on the Mac Mini (`mini-ts`) but all coding agent session data lives on this MacBook. OpenClaw has no way to search past agent sessions, losing valuable context that could improve its responses.

## Outcome

OpenClaw can run `cass search --robot "query"` on the Mac Mini and get results from all 10,651+ conversations across all agents (Claude Code, Codex, Pi Agent, OpenCode, Cursor, Gemini, Factory, etc.) with data refreshed at least daily.

---

## Requirements (R)

| ID | Requirement | Status |
|----|-------------|--------|
| R0 | OpenClaw on the Mac Mini can search cass-indexed sessions from this MacBook | Core goal |
| R1 | Sync happens automatically at least once daily | Must-have |
| R2 | Works when machines are on the same network (evenings/nights) | Must-have |
| R3 | Gracefully handles network unavailability (no errors/alerts when mini is unreachable) | Must-have |
| R4 | Incremental — doesn't re-transfer the full ~8 GB every time | Must-have |
| R5 | OpenClaw can query via `cass search --robot` or equivalent on the mini | Must-have |
| R6 | Minimal ongoing maintenance (set-and-forget after initial setup) | Nice-to-have |
| R7 | New agent types (amp, factory, etc.) automatically picked up without config changes | Nice-to-have |

## Selected Shape: A — Push DB from MacBook

A launchd job on the MacBook rsyncs the cass SQLite database and search index to the Mac Mini nightly. cass on the Mini reads the synced DB directly.

| Part | Mechanism |
|------|-----------|
| A1 | Install cass on Mac Mini via `cargo install` |
| A2 | launchd plist on MacBook: runs nightly wrapper script |
| A3 | Wrapper script: reachability check → rsync DB + index → log result |
| A4 | OpenClaw queries with `cass --db <synced-path> search --robot` |

## Fit Check: R × A

| Req | Requirement | Status | A |
|-----|-------------|--------|---|
| R0 | OpenClaw on Mac Mini can search cass sessions | Core goal | ✅ |
| R1 | Sync happens automatically at least once daily | Must-have | ✅ |
| R2 | Works when on same network (evenings/nights) | Must-have | ✅ |
| R3 | Gracefully handles network unavailability | Must-have | ✅ |
| R4 | Incremental transfer | Must-have | ✅ |
| R5 | OpenClaw queries via `cass search --robot` | Must-have | ✅ |
| R6 | Minimal ongoing maintenance | Nice-to-have | ✅ |
| R7 | New agent types auto-discovered | Nice-to-have | ✅ |

## Acceptance Scenarios

### AS-1: Daily Sync Succeeds

**Given** both machines are on the same network  
**When** the launchd job fires at the scheduled time  
**Then** the cass DB and index are rsync'd to the Mini, and `cass --db <path> search "test" --robot --limit 1` on the Mini returns results.

### AS-2: Network Unavailable

**Given** the Mac Mini is unreachable (e.g., different networks)  
**When** the launchd job fires  
**Then** the wrapper logs "mini-ts unreachable, skipping" and exits 0 (no error alerts, no hung processes).

### AS-3: Incremental Sync

**Given** the DB was synced yesterday  
**When** ~200 new sessions were indexed today and the sync runs  
**Then** rsync transfers only the changed blocks (not the full 6+ GB), completing in under 2 minutes on LAN.

### AS-4: New Agent Type

**Given** a new agent connector (e.g., `amp`) starts generating sessions  
**When** cass indexes them locally and the nightly sync runs  
**Then** the new agent's sessions are searchable on the Mini without any config changes.

### AS-5: OpenClaw Query

**Given** the DB has been synced to the Mini  
**When** OpenClaw runs `cass --db ~/cass-mirror/agent_search.db search "authentication" --robot --limit 5`  
**Then** it receives JSON results with session paths, agents, and content.
