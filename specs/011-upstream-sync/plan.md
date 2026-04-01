---
title: "Plan: Sync fork to upstream HEAD with GJ version string"
date: 2026-03-31
bead: coding_agent_session_search-2n2u
---

<!-- plan:complete:v1 | harness: pi/claude-sonnet-4-6 | date: 2026-04-01T10:30:00Z -->

# Plan — Spec 011: Full Upstream Sync

**Collaborators**: FastNova (planner) · YoungNova (crew-challenger/claude-opus-4-6)  
**Transcript**: `planning-transcript.md` in this directory

---

## Goal

After this plan executes: `cass --version` prints `cass 0.2.7-gj.1`. The diff against upstream HEAD shows only: `src/watchdog.rs` (new file), 6 surgical sites in `src/lib.rs`, and the Cargo.toml fork identity block. Nothing else is local.

---

## Architecture of the Change

### What "copy-forward" means here

The fork has no common git ancestor with upstream. We cannot `git merge`. Instead:

```
git checkout upstream/main -- src/ tests/ benches/ Cargo.toml rust-toolchain.toml build.rs
```

This replaces our tree with upstream's verbatim. Then we restore our watchdog and apply the minimal local patch on top.

### The local patch (everything we add back)

| What | Why | How |
|------|-----|-----|
| `src/watchdog.rs` | launchd plist management for macOS; upstream has nothing like it | `git checkout HEAD -- src/watchdog.rs` |
| 6 sites in `src/lib.rs` | Wire watchdog into upstream's CLI | Manual edits described below |
| `libc = "*"` in Cargo.toml | watchdog.rs uses libc::kill, libc::flock, etc. | Add to [dependencies] |
| `version = "0.2.7-gj.1"` in Cargo.toml | Unmistakably GJ, above upstream v0.2.5 | Edit |
| `repository = "https://github.com/carmandale/coding_agent_session_search"` | Correct fork URL | Edit |
| Path dep → git dep conversions | We don't have sibling repos; git deps achieve same result | Edit |

### What we DROP (local additions that become unnecessary)

| File | Reason |
|------|--------|
| `src/connectors/fad_adapter.rs` | Upstream has native clawdbot/copilot/copilot_cli/kimi/qwen/openclaw/vibe connectors |
| `src/connectors/codebuff.rs` | Dead code, never registered |
| `src/ui/sessions.rs` | Uses ratatui; upstream's UI uses ftui. Bead 3kt deferred. |
| `src/ui/components/message_render.rs` | Uses ratatui; upstream's UI uses ftui. Deferred. |
| Cargo `[patch]` for asupersync | No longer needed (path dep → git dep) |
| Cargo `franken-agent-detection` dep pointing to rev de450843 | Replaced by upstream's version |

---

## Version String Decision

**`version = "0.2.7-gj.1"`** → output: `cass 0.2.7-gj.1`

Reasoning:
- Upstream is at `0.2.5`. Using `0.2.6-gj.1` is wrong: SemVer pre-release sorts LOWER than release (`0.2.6-gj.1 < 0.2.6`). If upstream bumps to 0.2.6, our version would appear older.
- `0.2.7-gj.1`: clearly above upstream's 0.2.5 AND any immediate 0.2.6 bump. The `-gj.1` is unambiguous.
- Convention: when we re-sync to a future upstream version N, our version becomes `N+minor-gj.1`.

---

## Dependency Remapping

Upstream uses path deps to sibling repos. We replace all with git+rev deps:

| Upstream (path) | Our replacement (git) |
|-----------------|----------------------|
| `asupersync = { path = "../asupersync", features = ["test-internals", "tls-native-roots"] }` | `asupersync = { git = "https://github.com/Dicklesworthstone/asupersync", rev = "95476b32", features = ["test-internals", "tls-native-roots"] }` |
| `frankensqlite = { path = "../frankensqlite/crates/fsqlite", package = "fsqlite", features = ["fts5"] }` | `frankensqlite = { git = "https://github.com/Dicklesworthstone/frankensqlite", rev = "92a9a0fa", package = "fsqlite", features = ["fts5"] }` |
| `franken-agent-detection = { path = "../franken_agent_detection", features = [...] }` | `franken-agent-detection = { git = "https://github.com/Dicklesworthstone/franken_agent_detection", rev = "de450843", features = ["connectors", "cursor", "chatgpt", "opencode", "crush"] }` |
| `fsqlite-types = { path = "../frankensqlite/crates/fsqlite-types" }` (dev-dep) | `fsqlite-types = { git = "https://github.com/Dicklesworthstone/frankensqlite", rev = "92a9a0fa", package = "fsqlite-types" }` |

Also **remove** the `[patch."https://github.com/Dicklesworthstone/asupersync"]` section entirely — it was redirecting asupersync's internal git deps to local paths, which we don't need.

**Add** `libc = "*"` to `[dependencies]` — upstream does not include this, but `src/watchdog.rs` requires it for `libc::kill`, `libc::flock`, `libc::getuid`, `libc::SIGTERM`, `libc::SIGKILL`, `libc::ESRCH`, `libc::EPERM`, `libc::LOCK_EX`, `libc::LOCK_NB`.

---

## The 6 Watchdog Wiring Sites in `src/lib.rs`

After `git checkout upstream/main -- src/lib.rs` (which gives us the 18,929-line upstream version), apply these 6 changes:

### Site 1 — Module declaration (top of file)

Upstream has (lines 1-19):
```rust
pub mod analytics;
pub mod bakeoff;
pub mod bookmarks;
pub mod connectors;
// ...
pub mod update_check;
```

Add **after** `pub mod update_check;`:
```rust
pub mod watchdog;
```

### Site 2 — Commands enum variant

In `pub enum Commands { ... }`, after the last existing variant and before the closing `}`, add:
```rust
/// Watchdog: monitor and manage the watcher daemon
Watchdog {
    command: Option<watchdog::WatchdogCommand>,
},
```

Find anchor: grep for the last variant of `Commands` before `}` — use `git show upstream/main:src/lib.rs | grep -n "^}" | head -20` to find the enum closing brace.

### Site 3 — Dispatch arm

In the main `match command { ... }` block, add alongside other command arms:
```rust
Commands::Watchdog { command } => {
    crate::watchdog::run_watchdog_command(command).map_err(|e| CliError {
        code: 9,
        kind: "watchdog".to_string(),
        message: format!("{e}"),
        hint: None,
        retryable: false,
    })
}
```

Find anchor: the match block is the large dispatch in the main entry function. Look for `Commands::Index { ... } =>` as a nearby anchor.

### Site 4 — Health JSON "watchdog" block (surgical — in state_meta_json)

**Upstream's `state_meta_json` (line 4570) is ~200 lines** and builds a complex JSON structure including `"index"`, `"database"`, `"semantic"`, `"rebuild"`, `"fingerprint"`, `"pending"`, and `"_meta"` blocks. 

Add the `"watchdog"` block **between the `"pending"` block and the `"_meta"` block**:
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

This block has no dependencies on upstream's local variables — it only uses `dirs::home_dir()` which is already a dep in upstream's Cargo.toml.

Find anchor: `grep -n '"_meta"' src/lib.rs` after the checkout — insert the watchdog block immediately before that line.

### Site 5 — Subcommand string mapping

Somewhere in lib.rs there's a function that maps `Some(Commands::X { .. }) => "x".to_string()`. Add:
```rust
Some(Commands::Watchdog { .. }) => "watchdog".to_string(),
```

Find anchor: `grep -n "Commands::Index.*=> " src/lib.rs` — add the watchdog case in the same match.

### Site 6 — watchdog.rs test: fix `state_meta_json` call signature

After the copy-forward, upstream's `state_meta_json` takes **4 arguments** (adds `allow_db_open: bool`). Our watchdog.rs test at line 941 calls it with 3:

```rust
// Before (broken after upstream sync):
let state = crate::state_meta_json(dir.path(), &db_path, 1800);

// After (correct):
let state = crate::state_meta_json(dir.path(), &db_path, 1800, true);
```

This is in `src/watchdog.rs`, not `src/lib.rs` — but it's a wiring dependency between watchdog and lib.

---

## Build Environment Notes

### Toolchain change
Upstream uses `channel = "stable"` in `rust-toolchain.toml`. We've been using `nightly`. After copying upstream's toolchain file, builds use stable Rust (>= 1.85 required for edition 2024). Our watchdog.rs has no nightly features — verified.

### Vendored OpenSSL (plan for build time)
Upstream has `openssl = { version = "*", features = ["vendored"] }`. This compiles OpenSSL from source on first build, adding 2-5 minutes to a clean build. Requires: a C compiler and Perl installed (macOS ships both via Xcode tools). This is a one-time cost.

### Build command
```bash
~/.cargo/bin/cargo build --release
```
(NOT `cargo install --path .` — that fails due to a Homebrew toolchain issue. See napkin.)

### Deploy command
```bash
cp ./target/release/cass ~/.cargo/bin/cass
xattr -d com.apple.quarantine ~/.cargo/bin/cass 2>/dev/null || true
```

---

## Verification After Deploy

```bash
# 1. Version is unmistakably GJ
cass --version
# Expected: cass 0.2.7-gj.1

# 2. Git diff shows only our additions
git diff upstream/main --name-only -- src/
# Expected: src/lib.rs, src/watchdog.rs

git diff upstream/main --stat -- Cargo.toml
# Expected: ~10 line changes (version, repo, path→git deps, libc)

# 3. Health is good after watcher restart
sleep 120
cass health --json | python3 -c "import sys,json; d=json.load(sys.stdin); print('healthy:', d['healthy'], '| watchdog:', d.get('state',{}).get('watchdog',{}))"
# Expected: healthy: True | watchdog: {'watcher_plist_installed': True, 'plist_installed': True}

# 4. No crash loop
grep "LockBusy\|unsupported schema" ~/Library/Logs/cass-index-watch.log | tail -3
# Expected: empty (all entries pre-date deployment)
```

---

## Requirement Traceability

| Requirement | Change | Where |
|-------------|--------|-------|
| R1 — Upstream parity | `git checkout upstream/main -- src/ ...` | Step 2 |
| R2 — Minimal local delta | Exactly: watchdog.rs + 6 lib.rs sites + Cargo.toml changes | Steps 2-4 |
| R3 — Self-contained watchdog | libc dep added; no upstream src files modified except lib.rs | Cargo.toml + lib.rs |
| R4 — Build and deploy | stable Rust, cargo build --release, cp deploy | Step 5 |
| R5 — Watcher healthy | Schema migrations v9-v14 from upstream; watcher restarts clean | Post-deploy |
| User ask — GJ version | `version = "0.2.7-gj.1"` in Cargo.toml | Cargo.toml edit |
