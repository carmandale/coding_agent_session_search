---
title: "feat: Sync cass sessions to Mac Mini for OpenClaw"
date: 2026-03-05
bead: coding_agent_session_search-9wz3
---

# Tasks: Sync cass Sessions to Mac Mini

## Phase 1: Prerequisites

- [x] **T1: Verify SSH to mini-ts** — `ssh mini-ts "echo ok"` works (Tailscale, "none" auth)
- [x] **T2: Install Rust on Mac Mini** — SKIPPED: cass already installed via Homebrew
- [x] **T3: Install cass on Mac Mini** — Already present: cass v0.1.64 via Homebrew
- [x] **T4: Verify cass on Mini** — v0.1.64, has its own 14 conversations already indexed
- [x] **T5: Create mirror directory on Mini** — `~/cass-mirror/index/` created

## Phase 2: Sync Script

- [x] **T6: Create wrapper script** — `~/.local/bin/cass-sync-to-mini.sh`
  - Reachability: tries `mini-ts` alias, falls back to Tailscale IP `100.111.229.26`
  - WAL checkpoint before sync
  - Uses `scp -C` for DB (Apple rsync 3.4.1 has deflate bug with large files)
  - Uses `rsync -a` for index directory (small files, no issues)
  - Logging to `~/Library/Logs/cass-sync-to-mini.log`
- [x] **T7: Make executable** — done
- [x] **T8: Test manual run** — Initial sync: 4m18s. DB + index confirmed on Mini.
- [x] **T9: Verify cass query on Mini** — 10,655 conversations, 407K+ messages searchable

## Phase 3: Automation

- [x] **T10: Create launchd plist** — `~/Library/LaunchAgents/com.cass.sync-to-mini.plist` (2:00 AM daily)
- [x] **T11: Load launchd agent** — loaded and running
- [x] **T12: Test launchd trigger** — `launchctl start` confirmed working (7 min sync via scp)

## Phase 4: OpenClaw Integration

- [x] **T13: Set up cass alias on Mini** — Added `CASS_MACBOOK_DB` env var + `cass-macbook` alias to `~/.zshrc`
- [x] **T14: Verify OpenClaw can query** — `cass --db ~/cass-mirror/agent_search.db search "query" --robot` returns results

## Phase 5: Validation

- [x] **T15: Test network-unavailable graceful skip** — Logs "SKIP" and exits 0
- [x] **T16: Test incremental sync** — scp re-transfer with --whole-file takes ~6 min (acceptable for 2 AM)
- [x] **T17: Verify agent coverage** — All 9 agents searchable (codex, pi_agent, claude_code, opencode, cursor, factory, gemini, codebuff, amp)

## Known Issues

- **FTS corruption on mid-write sync**: If the DB is synced while cass is actively writing, minor FTS index corruption can occur. The 2 AM schedule avoids this since indexing is quiet at night. If needed, run `cass index --full` locally first.
- **Apple rsync 3.4.1 bug**: Cannot use rsync delta transfer on 6GB+ files (deflate crash). Using scp instead. Transfer is ~6 min (full file each time) but acceptable for nightly automation.
