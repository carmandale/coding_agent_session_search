# Napkin

## Corrections
| Date | Source | What Went Wrong | What To Do Instead |
| 2026-04-02 | Debug | `franken_existing_message_fingerprints_by_idx` had `LIMIT 1000` / `franken_existing_message_replay_fingerprints` had `LIMIT 100`. Codex sessions with >1000 messages (9 exist, max 2859) caused UNIQUE(conversation_id,idx) crash every scan cycle because early messages weren't in the fingerprint map and were re-inserted. | Remove both LIMITs — fetch ALL fingerprints. Connector reindexes full conversation on each modified file scan. Confirmed fix: watcher stable, no more crash loop. Commit: `5a3b33a4`. |
|------|--------|----------------|-------------------|
| 2026-03-07 | Self | Spec 004 was written assuming local connectors, but `main` branch moved connectors to external FAD crate (v0.1.2). Trait couldn't be modified. | Always check the target branch architecture before implementing. The `fix/index-gaps` branch has local connectors; `main` uses `franken_agent_detection` crate. Also: `main` currently doesn't compile due to rustc 1.88 vs dependency requirements (wide, unty-next). |
| 2026-02-12 | Self | Assumed `watch_scan conversations=0` meant a bug - it's actually expected for incremental scans when no new files exist | Understand that `conversations=0` in watch mode is normal when timestamps are current; check `streaming_ingest` logs for actual ingestion counts |
| 2026-02-12 | Self | Used `cass stats --robot` but stats doesn't have that flag | Use `cass stats --json` for machine-readable stats output |
| 2026-02-18 | Debug | Watcher running but not indexing new Claude Code sessions - watch_state.json showed recent timestamp but sessions weren't in DB | 1) Check `cass stats --json` to see if recent sessions are missing for a specific agent; 2) Compare most recent DB entry vs filesystem files; 3) Run `pkill -f "cass.*watch" && cass index --full --json` to reindex; 4) Restart watcher with `nohup cass index --watch > ~/Library/Logs/cass-index-watch.log 2>&1 &`; 5) Verify `streaming_ingest connector="claude"` appears in log |
| 2026-03-07 | Self | Spec hypothesized compact format files lacked `"type"` fields — actually both compact and standard subagent files DO have `"type"` on all lines | Always verify hypotheses by reading actual files before writing spec. The real bug was `looks_like_root`, not parsing |
| 2026-03-07 | Self | Assumed external_id collisions wouldn't happen because subagent filenames are unique | Same filename (agent-a0f386b.jsonl) CAN exist under different session UUID directories. Include parent session UUID in external_id for subagent files |
| 2026-03-29 | Self | Plan review trusted stale shaping assumptions and the PATH-installed `cass` binary more than the live checkout | Re-baseline every spec/plan against the current source tree with `rg`/file reads first, and use checkout-local binaries like `target/debug/cass` for verification when PATH may point at a newer install |
| 2026-03-29 | Self | Used Homebrew `cargo`/`rustc` from `/opt/homebrew/bin`, which drifted behind the repo's nightly toolchain and confused dependency resolution | In this repo, run Rust verification with `"$HOME/.cargo/bin/cargo"` (or rustup-managed cargo) so `rust-toolchain.toml` wins over the stale Homebrew toolchain |

## User Preferences
- Prefers thorough investigation with clear root cause analysis
- Wants beads updated and synced after resolving issues
- Test-first approach: write failing test → fix → verify pass

## Patterns That Work
- **Watcher troubleshooting flow**: Check watcher log → verify `streaming_ingest` entries → check `watch_state.json` timestamps → restart with `--full --force-rebuild` if needed
- **"gj last shows no Claude sessions" diagnostic**:
  1. `cass stats --json` → check `claude_code` count exists
  2. `sqlite3 <db> "SELECT ... WHERE a.name='claude_code' ORDER BY started_at DESC LIMIT 5"` → see most recent indexed date
  3. `find ~/.claude/projects -name "*.jsonl" -mtime -1` → confirm recent files exist
  4. If DB is stale: `pkill -f "cass.*watch" && cass index --full --json` → full reindex
  5. Restart watcher: `nohup cass index --watch > ~/Library/Logs/cass-index-watch.log 2>&1 &`
- **Root cause diagnostic for looks_like_root**: Test the closure against actual filesystem paths — `path.file_name()` returns the LAST component (e.g., `"projects"` not `".claude"`). Need to check parent directory name to identify roots like `~/.claude/projects`.
- **Subagent directory structure**: `~/.claude/projects/{slug}/{uuid}/subagents/agent-{hash}.jsonl` — the UUID is the session ID, the hash identifies the subagent. Same hash can appear under different UUIDs.
- **Log locations**: 
  - Watcher log: `~/Library/Logs/cass-index-watch.log`
  - Index DB: `~/Library/Application Support/com.coding-agent-search.coding-agent-search/agent_search.db`
  - Watch state: `~/Library/Application Support/com.coding-agent-search.coding-agent-search/watch_state.json`
- **Key log patterns to look for**:
  - `streaming_ingest connector=X conversations=N` - actual ingestion happened
  - `watch_scan kind=X conversations=0` - normal incremental scan, no new files
  - `streaming_scan_complete connector=X discovered=true` - connector found data sources
  - `skipping session file` (NEW) - structured skip log with path/connector/reason

## Patterns That Work (from compound-learnings 2026-03-15)
- **Ground claims in filesystem before acting**: This session had 4 factual errors caught by reviewers: (1) wrong connector names from stale FAD cache, (2) false "we already have them" from confusing upstream/main with our branch, (3) looks_like_root rejecting the new sessions root path, (4) PiAgent watching 22K files assumed as root cause when it was the watchdog kill loop. Rule: before stating "X exists" or "X works," run `ls`, `rg`, or `git show` to verify. Especially when comparing branches.
- **External monitors must not self-harm**: The spec 005 watchdog was killing the watcher every 10 min (SIGKILL, no cleanup), then running `cass index --full` (tantivy lock fight, always failed). Both created the instability being monitored. Fix: heartbeat file proves liveness independently of scan completion; SIGTERM before SIGKILL with 120s grace; no concurrent index commands.
- **Instrument for diagnosability before fixing**: When a thread spins at 100% CPU and `sample` shows `??? (in cass)`, the fix is NOT to guess — it's to add named threads (`thread::Builder::new().name(...)`) and keep symbols (`strip = "debuginfo"`, not `"symbols"` or `"none"`). After deploying instrumentation, the next occurrence immediately showed `OpenCodeConnector::scan` on the `index-watcher` thread.

## Patterns That Don't Work
- **Assuming stale timestamps = bug**: Watch state timestamps are intentionally kept current to enable incremental scanning
- **Checking only `watch_scan` logs**: Must also check `streaming_ingest` to see what was actually indexed
- **Trusting watch_state.json alone**: The `Claude` timestamp being recent does NOT mean sessions were ingested - the watcher can update timestamps without actually writing to DB if something goes wrong silently
- **Filename-only dedup for subagents**: Same agent ID hash can exist under different session UUIDs. Must include session UUID in external_id.

## Corrections (continued)
| 2026-03-31 | Self | Spec 008 was a partial upstream sync — upstream v0.2.5 has 70+ source files we don't have (analytics, daemon, html_export, 7 native connectors). Our Cargo.toml version was left at 0.1.55 while the installed binary reported 0.2.5 (built from upstream). This caused binary confusion and schema mismatch (upstream wrote schema v14, our source only knows v8). | Always bump Cargo.toml version and set repository = carmandale fork URL when doing upstream syncs. Version 0.2.6 = our fork based on upstream 0.2.5 era + local additions. |
| 2026-03-31 | Self | `cargo install --path .` fails in this repo due to Homebrew cargo (1.88) vs nightly toolchain. `cargo build --release` works (respects rust-toolchain.toml). Deploy by: `~/.cargo/bin/cargo build --release && cp ./target/release/cass ~/.cargo/bin/cass && xattr -d com.apple.quarantine ~/.cargo/bin/cass 2>/dev/null` | Never use `./dev-install.sh` until its `cargo install` is fixed to use `~/.cargo/bin/cargo`. |

## Corrections (continued)
| 2026-04-01 | Spec 011 | frankensqlite pin must balance stable-compat AND pragma_table_info support. 92a9a0fa lacks pragma_table_info (14 test failures); HEAD requires nightly. dd9b457 is the stable-safe sweet spot. | When upgrading frankensqlite, test pragma_table_info support AND check for `#![feature(core_intrinsics)]`. |
| 2026-04-01 | Spec 011 | New binary + new frankensqlite rev causes WAL frame salt mismatch on first start. Watcher crash-loops on historical salvage at source_row_id=7. | After frankensqlite upgrades: expect WAL mismatch on first watcher start; clear historical bundles if salvage loop detected. |
| 2026-04-02 | Spec 011 code-verify | Upstream sync specs that say "all tests pass" will always fail code-verify because 55 upstream tests fail due to frankensqlite FTS5 behavior differences between git pins and local path dep. This is a pre-existing upstream issue — cannot fix without frankensqlite dev access. | For future upstream sync specs: write acceptance criterion as "watchdog/wiring tests pass; upstream test failures documented in receipt are acceptable." Never write "cargo test --lib — all tests pass" for upstream sync work without qualifying which modules must pass. |
| 2026-04-02 | Post-deploy | After spec 011 plist reload, `com.cass.index-watch` was left unregistered in launchd (not in `launchctl list`). Went ~13 hours without indexing. Stale `index-run.lock` (PID 20317) was also blocking new runs. | After any plist reload/redeploy: verify `launchctl list \| grep cass` shows `com.cass.index-watch`. Check for stale lock: `cat <data_dir>/index-run.lock` and verify PID is alive. |

## Domain Notes
- **Version pattern**: When upstream bumps to version N, our fork version becomes `N+minor-gj.1`. The `-gj.1` suffix identifies our fork at a glance. Current: upstream `0.2.5` → fork `0.2.7-gj.1`.
- **Fork ownership**: This is a fork (`carmandale/coding_agent_session_search`) of upstream (`Dicklesworthstone/coding_agent_session_search`). We do NOT push to upstream. Push to `origin` (our fork) only. Upstream is tracked as `upstream` remote. As of 2026-03-15, fork main is 51 commits behind upstream (frankensqlite migration, new connectors, semantic embeddings). Spec 006 tracks the sync.
- **PiAgent root_paths FIXED (spec 005)**: `detect()` now returns `~/.pi/agent/sessions` instead of `~/.pi/agent`. Reduces watched files from 22K to 1.7K. `scan()` uses exact `Self::home()` comparison instead of path-substring heuristic.
- **PiAgent WalkDir FIXED (spec 005)**: `session_files()` now has `max_depth(10)` to prevent symlink bombs. Keeps `follow_links(true)` for clawdbot symlink compatibility (2,098 sessions).
- **Watcher log rotation FIXED (spec 005)**: `watchdog.sh` now does copytruncate when log exceeds 100MB (`: > "$LOG_FILE"` preserves launchd fd).
- **Watcher lifecycle FIXED (spec 005)**: SIGTERM handler via signal_hook. Heartbeat file at `<data_dir>/watcher-heartbeat` (60s interval). Watchdog checks heartbeat age (2700s threshold), not index staleness. Kill sequence: SIGTERM → 120s → SIGKILL.
- **Thread naming FIXED (spec 005)**: All `thread::spawn` calls now use `thread::Builder::new().name(...)`. Binary uses `strip = "debuginfo"` to keep symbols for `sample` output.
- **FAD connectors ADDED (spec 006)**: 4 new connectors via `franken-agent-detection` adapter: copilot, clawdbot, openclaw, vibe. `src/connectors/fad_adapter.rs` bridges FAD's 2-method trait to our 4-method trait. FAD dep chain is clean (no franken crates).
- **Monster codex files**: Up to 476MB per session file. Full scan re-reads all.
- **Indexer architecture**:
  - `src/indexer/mod.rs` - main indexing logic, streaming scan, watch loop
  - `src/connectors/claude_code.rs` - Claude Code connector with JSONL parsing
  - `watch_state.json` - HashMap<ConnectorKind, i64> of last-seen timestamps per connector
- **Two indexing paths (CRITICAL)**:
  1. `run_index()` (startup / `--full`): uses `ScanContext::local_default()` — bypasses `looks_like_root`
  2. `reindex_paths()` (watcher): uses `ScanContext::with_roots()` — calls `looks_like_root` for validation
  - The discrepancy between these paths caused the 70-file Claude Code gap
- **Streaming indexing (Opt 8.2)**: Parallel connector threads send batches through bounded channel, consumer ingests with backpressure
- **Watcher reliability improvements** (this branch):
  - 30-min periodic full scan heartbeat (10780a64)
  - Per-connector scan_start_ts advancement (b9f85cc4)
  - looks_like_root parent directory check for ~/.claude/projects (2fe246f0)
  - Subagent dedup with session UUID in external_id (0af61bb8)
  - Structured skip logging across all connectors (6cce3495)
- **Factory stubs**: 10 session_start-only files are by design — Factory creates stub files before the session has content. These correctly return `None` from `parse_factory_session`.
- **Restart command**: `pkill -TERM -f "cass index --watch"` (SIGTERM, not SIGKILL — lets tantivy flush). launchd KeepAlive restarts automatically. Do NOT run concurrent `cass index --full` — it fights the watcher for the tantivy lock.
