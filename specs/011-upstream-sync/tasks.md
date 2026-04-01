---
title: "Tasks: Sync fork to upstream HEAD with GJ version string"
date: 2026-03-31
bead: coding_agent_session_search-2n2u
---
<!-- Codex Review: APPROVED after 4 rounds | model: gpt-5.3-codex | date: 2026-04-01 -->
<!-- Status: REVISED -->
<!-- Revisions: added outer dispatch site (Site 3 — Commands::Watchdog in outer OR pattern); updated smoke test to cass watchdog run (not status); compiler gates now bare commands; added binary path verification step -->

<!-- plan:complete:v1 | harness: pi/claude-sonnet-4-6 | date: 2026-04-01T10:23:12Z -->

# Tasks — Spec 011: Full Upstream Sync

Read plan.md before starting. Every task here references decisions made there.

**Single guiding rule**: After this task list completes, `git diff upstream/main --name-only -- src/` shows only `src/lib.rs` and `src/watchdog.rs`.

---

## Phase 0 — Pre-flight

- [x] **T0.1** Confirm upstream is fetched and current:
  ```bash
  git fetch upstream
  git log --oneline upstream/main | head -5
  # Note the top commit SHA for the record
  ```

- [x] **T0.2** Check current watcher health so we know baseline:
  ```bash
  cass health --json | python3 -c "import sys,json; d=json.load(sys.stdin); print('pre-sync health:', d['healthy'])"
  ```

- [x] **T0.3** Back up `src/watchdog.rs` to a safe location:
  ```bash
  cp src/watchdog.rs /tmp/watchdog.rs.bak
  echo "backed up to /tmp/watchdog.rs.bak"
  ```

---

## Phase 1 — Copy Upstream Source

- [x] **T1.1** Copy upstream source, tests, Cargo files, and build script — then immediately restore our watchdog (single chain, no interruption):
  ```bash
  git checkout upstream/main -- src/ tests/ benches/ Cargo.toml rust-toolchain.toml build.rs \
    && git checkout HEAD -- src/watchdog.rs
  ```

  ⚠️ This DELETES `src/fad_adapter.rs`, `src/connectors/codebuff.rs`, `src/ui/sessions.rs`, `src/ui/components/message_render.rs` — that is intentional per the spec.

- [x] **T1.2** Restore local scripts (NOT replaced by upstream checkout):
  ```bash
  git checkout HEAD -- scripts/watchdog.sh dev-install.sh hooks/
  ```

- [x] **T1.3** Verify watchdog.rs is present and lib.rs is upstream's version:
  ```bash
  ls src/watchdog.rs && echo "watchdog present"
  wc -l src/lib.rs
  # Expected: ~18929 lines (upstream's size, not our 11752)
  ```

---

## Phase 2 — Fix Cargo.toml

All edits to `Cargo.toml` in one pass.

- [x] **T2.1** Set version and repository:
  - Change `version = "0.2.5"` → `version = "0.2.7-gj.1"`
  - Change `license-file = "LICENSE"` → `license = "MIT"`
  - Change `repository = "https://github.com/Dicklesworthstone/coding_agent_session_search"` → `repository = "https://github.com/carmandale/coding_agent_session_search"`

- [x] **T2.2** Replace path dep — `asupersync`:
  ```toml
  # Remove this line:
  asupersync = { path = "../asupersync", features = ["test-internals", "tls-native-roots"] }
  
  # Replace with:
  asupersync = { git = "https://github.com/Dicklesworthstone/asupersync", rev = "95476b32", features = ["test-internals", "tls-native-roots"] }
  ```

- [x] **T2.3** Replace path dep — `frankensqlite`:
  ```toml
  # Remove this line:
  frankensqlite = { path = "../frankensqlite/crates/fsqlite", package = "fsqlite", features = ["fts5"] }
  
  # Replace with:
  frankensqlite = { git = "https://github.com/Dicklesworthstone/frankensqlite", rev = "92a9a0fa", package = "fsqlite", features = ["fts5"] }
  ```

- [x] **T2.4** Replace path dep — `franken-agent-detection`:
  ```toml
  # Remove this line:
  franken-agent-detection = { path = "../franken_agent_detection", features = ["connectors", "cursor", "chatgpt", "opencode", "crush"] }
  
  # Replace with:
  franken-agent-detection = { git = "https://github.com/Dicklesworthstone/franken_agent_detection", rev = "de450843", features = ["connectors", "cursor", "chatgpt", "opencode", "crush"] }
  ```

- [x] **T2.5** Replace path dev-dep — `fsqlite-types`:
  ```toml
  # Remove this line (under [dev-dependencies]):
  fsqlite-types = { path = "../frankensqlite/crates/fsqlite-types" }
  
  # Replace with:
  fsqlite-types = { git = "https://github.com/Dicklesworthstone/frankensqlite", rev = "92a9a0fa", package = "fsqlite-types" }
  ```

- [x] **T2.6** Remove the `[patch."https://github.com/Dicklesworthstone/asupersync"]` section entirely (including all 4 sub-entries: asupersync, franken-decision, franken-evidence, franken-kernel).

- [x] **T2.7** Add `libc = "*"` to `[dependencies]` (watchdog.rs requires it; upstream does not include it):
  ```toml
  libc = "*"  # required by src/watchdog.rs for PID management
  ```

- [x] **T2.8** Verify Cargo.toml has no remaining `path = "../"` references:
  ```bash
  grep 'path = "\.\.' Cargo.toml
  # Expected: no output
  ```

---

## Phase 3 — Apply Watchdog Wiring (6 Sites in src/lib.rs)

Edit `src/lib.rs` to wire in our watchdog subcommand. Apply all 6 sites.

**Key architecture note**: Upstream uses a TWO-STAGE dispatch. Site 3 (outer pattern) and Site 4 (inner arm) must BOTH be applied — missing either makes `cass watchdog` unreachable.

- [x] **T3.1** Site 1 — Module declaration. Find `pub mod update_check;` and add the line after it:
  ```bash
  grep -n "pub mod update_check" src/lib.rs
  ```
  Insert **after** that line:
  ```rust
  pub mod watchdog;
  ```

- [x] **T3.2** Site 2 — Commands enum variant. Find the `pub enum Commands {` closing brace and add a new variant before it:
  ```bash
  grep -n "^pub enum Commands" src/lib.rs
  ```
  Add as the last variant:
  ```rust
  /// Watchdog: monitor and manage the watcher daemon
  Watchdog {
      command: Option<watchdog::WatchdogCommand>,
  },
  ```

- [x] **T3.3** Site 3 — **OUTER** dispatch pattern (two-stage dispatch). Find the outer match arm for non-TUI commands:
  ```bash
  grep -n "Commands::Analytics" src/lib.rs
  ```
  Add `| Commands::Watchdog { .. }` to the OR pattern BEFORE the `=>`, e.g.:
  ```rust
  | Commands::Analytics(..)
  | Commands::Watchdog { .. } => {
  ```

- [x] **T3.4** Site 4 — **INNER** dispatch arm. Inside the non-TUI branch's inner `match command { ... }`, add:
  ```bash
  grep -n "Commands::Index {" src/lib.rs | head -3
  ```
  Add in the SAME inner match:
  ```rust
  Commands::Watchdog { command } => {
      crate::watchdog::run_watchdog_command(command).map_err(|e| CliError {
          code: 3,
          kind: "watchdog",
          message: format!("{e}"),
          hint: None,
          retryable: false,
      })?;
  }
  ```
  Note: `kind: "watchdog"` is `&'static str` (NOT `.to_string()`), `code: 3`, ends with `?;`

- [x] **T3.5** Site 5 — Health JSON watchdog block. Find the `"_meta":` block inside `state_meta_json`:
  ```bash
  grep -n '"_meta"' src/lib.rs | grep -v test | head -3
  ```
  Insert **immediately before** the `"_meta":` entry:
  ```rust
  "watchdog": {
      "watcher_plist_installed": dirs::home_dir()
          .map(|h| h.join("Library/LaunchAgents/com.cass.index-watch.plist").exists())
          .unwrap_or(false),
      "plist_installed": dirs::home_dir()
          .map(|h| h.join("Library/LaunchAgents/com.cass.health-watchdog.plist").exists())
          .unwrap_or(false),
  },
  ```

- [x] **T3.6** Site 6 — Subcommand string mapping. Find the match that maps commands to strings:
  ```bash
  grep -n '"index"\|"search"\|"tui"' src/lib.rs | grep "Commands::" | head -5
  ```
  Add alongside:
  ```rust
  Some(Commands::Watchdog { .. }) => "watchdog".to_string(),
  ```

- [x] **T3.7** Site 7 — Fix `state_meta_json` call in watchdog.rs tests (NOT in lib.rs — this is in our watchdog.rs):
  ```bash
  grep -n "state_meta_json" src/watchdog.rs
  ```
  Update line 941 (approximately):
  ```rust
  // Before:
  let state = crate::state_meta_json(dir.path(), &db_path, 1800);
  
  // After:
  let state = crate::state_meta_json(dir.path(), &db_path, 1800, true);
  ```

---

## Phase 4 — First Build Check

- [x] **T4.1** Compiler gates — each command must exit 0. If any fails, fix before continuing:
  ```bash
  ~/.cargo/bin/cargo check --all-targets
  ~/.cargo/bin/cargo clippy --all-targets -- -D warnings
  ~/.cargo/bin/cargo fmt --check
  ~/.cargo/bin/cargo test --lib
  ```

- [x] **T4.2** If any gate fails: read compiler output to understand the error; do NOT pipe through grep/head as that masks the exit code. Common expected issues:
  - Function signature changes between our fork and upstream
  - Types that moved modules in upstream
  - API changes in upgraded dependencies

---

## Phase 5 — Build and Deploy

- [x] **T5.1** Full release build (expect 2-5 extra minutes for vendored OpenSSL on first build):
  ```bash
  ~/.cargo/bin/cargo build --release 2>&1 | tail -5
  ```

- [x] **T5.2** Verify version:
  ```bash
  ./target/release/cass --version
  # Required: cass 0.2.7-gj.1
  ```

- [x] **T5.3** Deploy (macOS gatekeeper quarantine workaround):
  ```bash
  cp ./target/release/cass ~/.cargo/bin/cass
  xattr -d com.apple.quarantine ~/.cargo/bin/cass 2>/dev/null || true
  cass --version
  # Expected: cass 0.2.7-gj.1
  ```

- [x] **T5.4** Reload the watcher with new binary:
  ```bash
  PLIST="$HOME/Library/LaunchAgents/com.cass.index-watch.plist"
  launchctl unload "$PLIST" 2>/dev/null && sleep 1 && launchctl load "$PLIST"
  sleep 5
  WATCHER_PID=$(pgrep -f "cass index --watch" | head -1)
  WATCHER_PATH=$(ps -o args= -p "$WATCHER_PID" | awk '{print $1}')
  "$WATCHER_PATH" --version
  # Required: cass 0.2.7-gj.1
  ```

---

## Phase 6 — Verify

- [ ] **T6.1** Confirm GJ version is unmistakable:
  ```bash
  cass --version
  # Must show: cass 0.2.7-gj.1
  ```

- [ ] **T6.1b** Smoke test watchdog command is reachable:
  ```bash
  cass watchdog run
  # Must NOT print "error: unrecognized subcommand"
  ```

- [ ] **T6.2** Confirm git diff shows only our additions:
  ```bash
  git diff upstream/main --name-only -- src/
  # Expected: src/lib.rs and src/watchdog.rs ONLY
  
  git diff upstream/main --stat -- Cargo.toml
  # Expected: ~10-15 line delta (version, repo, path→git deps, libc add)
  ```

- [ ] **T6.3** Wait 3 minutes then check health:
  ```bash
  sleep 180
  cass health --json | python3 -c "
  import sys,json
  d = json.load(sys.stdin)
  print('healthy:', d['healthy'])
  print('watchdog:', d.get('state',{}).get('watchdog',{}))
  "
  # Expected: healthy: True, watchdog: {watcher_plist_installed: True, plist_installed: True}
  ```

- [ ] **T6.4** Confirm no crash loop:
  ```bash
  grep "LockBusy\|unsupported schema\|drop_close" ~/Library/Logs/cass-index-watch.log | tail -3
  # Expected: all entries pre-date this deployment
  ```

- [ ] **T6.5** Confirm search works:
  ```bash
  cass search "authentication" --robot --limit 3 | python3 -c "import sys,json; d=json.load(sys.stdin); print('hits:', len(d.get('hits',[])))"
  ```

---

## Phase 7 — Closeout

- [ ] **T7.1** Commit:
  ```bash
  git add -A
  git commit -m "feat: upstream sync to HEAD + 0.2.7-gj.1 (spec 011)

  - Copy upstream HEAD (Dicklesworthstone) verbatim
  - Local delta: src/watchdog.rs + 6 lib.rs wiring sites
  - Version: 0.2.5 (upstream) → 0.2.7-gj.1 (fork)
  - Cargo: path deps → git deps (asupersync rev 95476b32, frankensqlite rev 92a9a0fa)
  - Removes: fad_adapter.rs, codebuff.rs (upstream has native connectors)
  - Removes: sessions.rs, message_render.rs (ratatui → ftui migration, bead 3kt deferred)
  
  Refs: spec 011, bead coding_agent_session_search-2n2u"
  ```

- [ ] **T7.2** Push to origin:
  ```bash
  git push origin feat/007-watchdog-subcommand
  ```

- [ ] **T7.3** Close bead:
  ```bash
  br close coding_agent_session_search-2n2u --reason="Done: upstream synced to HEAD, version 0.2.7-gj.1 deployed, watcher healthy"
  br sync --flush-only
  ```

- [ ] **T7.4** Update napkin with version maintenance pattern:
  > When upstream bumps to version N, our fork version becomes N+minor-gj.1. The `-gj.1` suffix identifies our fork at a glance.
