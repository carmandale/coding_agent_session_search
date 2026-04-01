---
title: "Sync fork to upstream HEAD; minimize local delta to watchdog + version"
date: 2026-03-31
bead: coding_agent_session_search-2n2u
---

<!-- issue:complete:v1 | harness: pi/claude-sonnet-4-6 | date: 2026-04-01T09:59:29Z -->

# Spec 011 — Full Upstream Sync

## Intent

Be fully in sync with upstream (`Dicklesworthstone/coding_agent_session_search`). Local
changes should be as minimal as possible — only what is needed to (a) run cass locally and
(b) provide the freshest possible index for agent use.

This is not a feature spec. It is a maintenance sync. After this spec, the fork should look
like upstream with a small, clearly-documented patch on top.

---

## Background: Why We're Here

**Spec 008** ("frankensqlite migration") was a partial upstream sync. It brought in some
upstream changes but left the fork in an inconsistent state:

- 67 upstream source files are missing from our tree (analytics, daemon, html_export,
  semantic search extras, 7 native connectors, etc.)
- Our Cargo.toml was left at version `0.1.55` while the upstream-built binary reported
  `0.2.5` → confusing "which binary is running" for weeks
- Schema version mismatch: upstream writes schema v14; our source only knew v8. Running
  `dev-install.sh` would have installed a binary that couldn't read its own database.
- The fork accumulated local bridges (fad_adapter.rs) to an external crate that upstream
  later replaced with native connectors.

The version was bumped to `0.2.6` in a 2026-03-31 session as an emergency fix, but the
underlying delta remains.

---

## Current Delta (as of 2026-03-31)

### Files in upstream that our fork is missing (67 src/ files):

| Category | Count | Key items |
|----------|-------|-----------|
| Native connectors | 7 | clawdbot, copilot, copilot_cli, kimi, qwen, openclaw, vibe |
| Analytics module | 6 | src/analytics/{bucketing,derive,mod,query,types,validate}.rs |
| Daemon (ML model serving) | 7 | src/daemon/{client,core,mod,models,protocol,resource,worker}.rs |
| HTML export | 7 | src/html_export/ |
| Semantic search extras | 7 | asset_state, ann_index, two_tier_search, reranker*, embedder_registry, daemon_client |
| Pages UI | 10+ | archive_config, attachments, config_input, confirmation, docs, errors, password, patterns, preview, profiles, summary, verify |
| Pages assets (JS) | 7 | attachments.js, password-strength.js, router.js, settings.js, share.js, stats.js, storage.js |
| UI components | 5 | app.rs, export_modal.rs, style_system.rs, theme.rs, trace.rs, ftui_adapter.rs |
| Indexer extras | 2 | redact_secrets.rs, semantic.rs |
| Other | 4 | bakeoff.rs, cass-pages-perf-bundle.rs, ftui_harness.rs, tui_asciicast.rs |

### Schema version: our fork = v8, upstream = v14 (migrations v9-v14 missing)

### Files in our fork NOT in upstream (fork-only additions):

| File | Purpose | Disposition |
|------|---------|-------------|
| `src/watchdog.rs` | launchd plist management, `cass watchdog` CLI subcommand | **KEEP** — local deployment need, upstream has no equivalent |
| `src/connectors/fad_adapter.rs` | Bridge to external FAD crate for copilot/clawdbot/openclaw/vibe | **DROP** — upstream has native connectors |
| `src/connectors/codebuff.rs` | Dead code (unregistered per spec 009) | **DROP** — never needed |
| `src/ui/sessions.rs` | Sessions TUI detail view (bead `3kt`) | **EVALUATE** — keep if it doesn't conflict |
| `src/ui/components/message_render.rs` | Message rendering improvement | **EVALUATE** — keep if it doesn't conflict |
| Cargo.toml `[patch]` for frankensqlite | Resolves FAD crate's internal dep | **DROP** with fad_adapter |
| Cargo dep: `franken_agent_detection` | External FAD crate | **DROP** with fad_adapter |

---

## Requirements

### R1 — Upstream parity

After this spec, `git diff upstream/main -- src/` should show ONLY the intentionally
retained local additions. Every file upstream has, we have. Every migration upstream has,
we have.

### R2 — Minimal local delta

The local patch on top of upstream consists of exactly:

1. **`src/watchdog.rs`** — the `cass watchdog` subcommand that manages the launchd plist
   (`com.cass.index-watch.plist`) for keeping the background watcher alive on macOS.
   Upstream has no launchd/background-process management at all.

2. **`src/lib.rs` watchdog wiring** — the 5 sites in `src/lib.rs` that wire `watchdog.rs`
   into the CLI: `pub mod watchdog`, `Commands::Watchdog` variant, dispatch arm, health
   status reporting (plist_installed), and subcommand string mapping.

3. **`Cargo.toml` fork identity** — `version = "0.2.6"` (or bumped above upstream),
   `repository = "https://github.com/carmandale/coding_agent_session_search"`.

4. **Optionally**: `src/ui/sessions.rs` and `src/ui/components/message_render.rs` if they
   don't conflict with upstream's UI changes and represent genuine improvements. If they
   cause merge friction, drop them.

### R3 — Local additions are self-contained

The `watchdog.rs` addition must compile cleanly and not require upstream to change. It must
not modify any upstream source files beyond the 5 wiring sites in `lib.rs`.

### R4 — Build and deploy work

`cargo build --release` (using `~/.cargo/bin/cargo` for nightly toolchain) produces a
working binary. `./dev-install.sh` deploys it (or the cp-based equivalent if `cargo install`
continues to fail due to the Homebrew toolchain issue — see napkin).

### R5 — Watcher healthy after deploy

After deploy:
- `cass --version` shows our fork version (≥ 0.2.6, > upstream 0.2.5)
- `cass health --json` → `"healthy": true` within 5 minutes
- `grep "LockBusy\|drop_close" ~/Library/Logs/cass-index-watch.log | tail -3` — no new entries
- `grep "incremental_scan" ~/Library/Logs/cass-index-watch.log | tail -3` — shows incremental (not always full)

---

## Acceptance Criteria

- [ ] `git diff upstream/main --name-only -- src/` shows only: `watchdog.rs`, `lib.rs`
  (and optionally `ui/sessions.rs`, `ui/components/message_render.rs`)
- [ ] `git diff upstream/main -- Cargo.toml` shows only: version line, repository line,
  and removal of fad_adapter deps + [patch] section
- [ ] `~/.cargo/bin/cargo check --all-targets` — clean
- [ ] `~/.cargo/bin/cargo clippy --all-targets -- -D warnings` — clean
- [ ] `~/.cargo/bin/cargo test --lib` — all tests pass
- [ ] `cass --version` — shows 0.2.6 or higher
- [ ] `cass health --json` — `"healthy": true`
- [ ] Watcher runs continuously without LockBusy loop

---

## Constraints and Non-Negotiables

- **Do NOT** add local features that diverge from upstream's design
- **Do NOT** keep fad_adapter.rs — it was a bridge we no longer need
- **Do NOT** keep codebuff.rs — it was never registered and serves no purpose
- **Do NOT** use `upstream/main` as the merge base if it causes the watchdog subcommand
  to disappear — the watchdog is the deployment mechanism, not an optional feature
- **The watchdog IS the "provide freshest index" mechanism**: it manages the launchd plist
  that keeps `cass index --watch` alive 24/7. Without it, there's no automated indexing.

---

## Out of Scope

- Implementing new features from upstream's roadmap (semantic search, ML model download, etc.)
- Porting our spec 010 crash-loop fixes to upstream (they may have fixed it differently)
- Changing the launchd plist itself
- Windows or Linux support changes

---

## Implementation Notes

### Merge strategy

The fork history has no common ancestor with upstream (shallow clone from early divergence).
The recommended approach is **not** a git merge but a **copy-forward**:

1. Take upstream's current `src/`, `Cargo.toml`, tests, and docs verbatim
2. Apply the watchdog patch on top (diff from our current `src/watchdog.rs` + lib.rs wiring)
3. Resolve any conflicts in `lib.rs` (upstream's CLI structure vs our watchdog additions)
4. Adjust version and repository in Cargo.toml

This avoids trying to merge two divergent histories and produces a clean, auditable diff.

### The `lib.rs` wiring diff

The 5 sites in `lib.rs` that need to be preserved:

```rust
// 1. Module declaration (top of file)
pub mod watchdog;

// 2. CLI command variant
Commands::Watchdog {
    command: Option<watchdog::WatchdogCommand>,
},

// 3. Dispatch arm
Commands::Watchdog { command } => {
    crate::watchdog::run_watchdog_command(command).map_err(|e| CliError { ... })
},

// 4. Health status (plist check)
"watchdog": {
    "watcher_plist_installed": ...,
    "plist_installed": ...,
},

// 5. Subcommand string mapping
Some(Commands::Watchdog { .. }) => "watchdog".to_string(),
```

Upstream's `lib.rs` is ~11,000 lines and will have changed significantly. These 5 sites
will need to be re-applied to upstream's version after copy-forward.

### Cargo.toml changes

After taking upstream's Cargo.toml:
- Change `version` to `"0.2.6"` (or higher if appropriate)
- Change `repository` to `"https://github.com/carmandale/coding_agent_session_search"`
- Remove the `franken_agent_detection` dependency line
- Remove the `[patch."https://github.com/Dicklesworthstone/franken_agent_detection"]` section

### Database migration

After deploying the new binary:
- If the database is at v8 (from our current run), the new binary (with v9-v14 migrations)
  will migrate it automatically via `open_or_rebuild`
- If migration fails, a `--force-rebuild` full reindex from source JSONL files is safe
  (all agent conversation files are untouched)

---

## Notes

- Upstream actively developed: check `git log upstream/main --oneline | head -10` before
  starting to get the latest commits
- The upstream repo URL is `https://github.com/Dicklesworthstone/coding_agent_session_search`
- The `scripts/watchdog.sh` in our fork is a shell-script complement to `src/watchdog.rs` —
  keep it alongside watchdog.rs
- The `hooks/` directory contains git hooks; check if upstream has equivalent hooks before
  deciding to keep or discard
