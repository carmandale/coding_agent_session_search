---
title: "Plan: Sync fork to upstream HEAD with GJ version string"
date: 2026-03-31
bead: coding_agent_session_search-2n2u
---
<!-- Codex Review: APPROVED after 4 rounds | model: gpt-5.3-codex | date: 2026-04-01 -->
<!-- Status: REVISED -->
<!-- Revisions: fixed dispatch arm type (kind static str not String, code 3 not 9, added ?; and #[command(subcommand)]); documented two-stage dispatch (outer OR pattern + inner match); added bare compiler gate commands (no pipe masking); added path-based watcher binary verification; added deployment rollback procedure; fixed smoke test from cass watchdog status → cass watchdog run -->
<!-- plan:complete:v1 | harness: pi/claude-sonnet-4-6 | date: 2026-04-01T10:30:00Z -->

# Plan — Spec 011: Full Upstream Sync

**Collaborators**: FastNova (planner) · YoungNova (crew-challenger/claude-opus-4-6)
**Codex review**: APPROVED after 4 rounds (gpt-5.3-codex)

---

## Goal

After this plan executes: `cass --version` prints `cass 0.2.7-gj.1`. The diff against upstream HEAD shows only `src/watchdog.rs`, 6 surgical sites in `src/lib.rs`, and the Cargo.toml fork identity block.

---

## Architecture of the Change

### What "copy-forward" means here

The fork has no common git ancestor with upstream. We cannot `git merge`. Instead:

```bash
git checkout upstream/main -- src/ tests/ benches/ Cargo.toml rust-toolchain.toml build.rs
```

This replaces our tree with upstream's verbatim. Then we restore our watchdog and apply the minimal local patch.

### Toolchain note

Upstream uses `channel = "stable"` in rust-toolchain.toml. Copying that file switches our builds to stable Rust. This is intentional and safe: watchdog.rs uses no nightly features (verified line-by-line). The spec R4 reference to "nightly toolchain" was written before discovering this; the copy-forward naturally resolves it.

### The local patch (everything we add back)

| What | Why | How |
|------|-----|-----|
| `src/watchdog.rs` | launchd plist management; upstream has nothing like it | `git checkout HEAD -- src/watchdog.rs` |
| 6 sites in `src/lib.rs` | Wire watchdog into upstream's two-stage CLI dispatch | Manual edits described below |
| `libc = "*"` in Cargo.toml | watchdog.rs uses 9 libc symbols (kill, flock, getuid, etc.) | Add to [dependencies] |
| `version = "0.2.7-gj.1"` | Unmistakably GJ, above upstream v0.2.5 in SemVer | Edit |
| `repository = "https://github.com/carmandale/coding_agent_session_search"` | Correct fork URL | Edit |
| 4 path dep → git dep conversions | Build environment lacks sibling repos upstream expects | Edit (required) |

### What we DROP

| File | Reason |
|------|--------|
| `src/connectors/fad_adapter.rs` | Upstream has native clawdbot/copilot/copilot_cli/kimi/qwen/openclaw/vibe |
| `src/connectors/codebuff.rs` | Dead code, never registered |
| `src/ui/sessions.rs` | Uses ratatui; upstream uses ftui. Bead 3kt deferred. |
| `src/ui/components/message_render.rs` | Uses ratatui; upstream uses ftui. |
| Cargo `[patch]` for asupersync | No longer needed (path dep → git dep) |

---

## Version String

**`version = "0.2.7-gj.1"`** → `cass 0.2.7-gj.1`

- SemVer pre-release: `0.2.7-gj.1 > 0.2.5` (upstream) ✓
- `-gj` is unmistakably Groove Jones
- Convention for future syncs: upstream version N → our version is `N+minor-gj.1`

---

## Dependency Remapping

Upstream uses path deps to sibling repos. Replace all with git+rev:

| Upstream (path) | Replacement (git+rev) |
|-----------------|----------------------|
| `asupersync = { path = "../asupersync", features = ["test-internals", "tls-native-roots"] }` | `asupersync = { git = "https://github.com/Dicklesworthstone/asupersync", rev = "95476b32", features = ["test-internals", "tls-native-roots"] }` |
| `frankensqlite = { path = "../frankensqlite/crates/fsqlite", package = "fsqlite", features = ["fts5"] }` | `frankensqlite = { git = "https://github.com/Dicklesworthstone/frankensqlite", rev = "92a9a0fa", package = "fsqlite", features = ["fts5"] }` |
| `franken-agent-detection = { path = "../franken_agent_detection", features = ["connectors", "cursor", "chatgpt", "opencode", "crush"] }` | `franken-agent-detection = { git = "https://github.com/Dicklesworthstone/franken_agent_detection", rev = "de450843", features = ["connectors", "cursor", "chatgpt", "opencode", "crush"] }` |
| `fsqlite-types = { path = "../frankensqlite/crates/fsqlite-types" }` (dev-dep) | `fsqlite-types = { git = "https://github.com/Dicklesworthstone/frankensqlite", rev = "92a9a0fa", package = "fsqlite-types" }` |

Remove the entire `[patch."https://github.com/Dicklesworthstone/asupersync"]` section.

Add `libc = "*"` to `[dependencies]` (watchdog.rs requires 9 libc symbols; upstream omits it).

---

## The 6 Watchdog Wiring Sites in `src/lib.rs`

**Critical architecture note**: Upstream's `run_cli` has a **two-stage dispatch**:

```
OUTER match &command {
  Commands::Tui { ... }   → TUI-specific tracing init + TUI launch
  Commands::Index | Commands::Search | ... | Commands::Analytics(..) => {
    non-TUI tracing init
    INNER match command {
      Commands::Index { ... } => { ... }
      Commands::Search { ... } => { ... }
      ... all non-TUI commands
    }
  }
}
```

Watchdog must be wired into **BOTH** the outer OR pattern AND the inner match. Missing either makes the command unreachable.

### Site 1 — Module declaration

Insert **after** `pub mod update_check;` (upstream lib.rs line ~19):
```rust
pub mod watchdog;
```

### Site 2 — Commands enum variant

Add as the last variant in `pub enum Commands { ... }` with `#[command(subcommand)]`:
```rust
/// Watchdog: monitor and manage the watcher daemon
Watchdog {
    #[command(subcommand)]
    command: Option<watchdog::WatchdogCommand>,
},
```
Note: `#[command(subcommand)]` is required — verified in current src/lib.rs:501.

### Site 3 — Outer dispatch pattern

Find the **outer** match arm pattern that lists non-TUI commands (around upstream lib.rs line ~2733):
```rust
Commands::Index { .. }
| Commands::Search { .. }
| ...
| Commands::Analytics(..) => {
```

Add `| Commands::Watchdog { .. }` to this OR pattern so upstream's tracing init fires for watchdog commands.

Anchor: `grep -n "Commands::Analytics" src/lib.rs` — add `| Commands::Watchdog { .. }` before the `=>` on the same arm.

### Site 4 — Inner dispatch arm (non-TUI match)

Inside the non-TUI branch's **inner** `match command { ... }`, add the watchdog arm alongside others (type-correct version verified against src/lib.rs:936-938, 2277-2284):

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

Key: `kind: "watchdog"` is `&'static str` (not `.to_string()`), `code: 3`, ends with `?;`.

Anchor: `grep -n "Commands::Index {" src/lib.rs | head -3` — add watchdog arm in the same inner match.

### Site 5 — Health JSON "watchdog" block

Upstream's `state_meta_json` (~line 4570) has `"pending"` and `"_meta"` blocks. Insert between them:

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

Anchor: `grep -n '"_meta"' src/lib.rs` — insert immediately before that line.

### Site 6 — watchdog.rs test: fix `state_meta_json` call signature

In `src/watchdog.rs` line ~941 (watchdog test):
```rust
// Before (breaks after copy-forward):
let state = crate::state_meta_json(dir.path(), &db_path, 1800);

// After (4-arg upstream signature):
let state = crate::state_meta_json(dir.path(), &db_path, 1800, true);
```

Also add `Some(Commands::Watchdog { .. }) => "watchdog".to_string()` to the subcommand string mapping function (find with `grep -n '"index"' src/lib.rs | grep "Commands::"`).

---

## Build Environment

### Toolchain
Upstream's rust-toolchain.toml uses `channel = "stable"`. After copy-forward, we use stable. Safe: watchdog.rs has no nightly features.

### First build
First `cargo build --release` includes vendored OpenSSL compilation (~2-5 minutes extra). Requires C compiler and Perl (macOS ships both via Xcode CLT). One-time cost.

### Build command
```bash
~/.cargo/bin/cargo build --release
```

---

## Deployment with Rollback

### Pre-deploy backup
```bash
cp ~/.cargo/bin/cass ~/.cargo/bin/cass.pre-011-backup
```

### Deploy sequence (order matters for watcher cutover)
```bash
# 1. Copy new binary
cp ./target/release/cass ~/.cargo/bin/cass
xattr -d com.apple.quarantine ~/.cargo/bin/cass 2>/dev/null || true

# 2. Verify new binary version BEFORE touching watcher
~/.local/bin/cass --version
# Must show: cass 0.2.7-gj.1

# 3. Reload launchd plist (kills old watcher, starts new one with new binary)
PLIST="$HOME/Library/LaunchAgents/com.cass.index-watch.plist"
launchctl unload "$PLIST" 2>/dev/null && sleep 1 && launchctl load "$PLIST"

# 4. Verify watcher PID is fresh and binary is correct version
sleep 3
WATCHER_PID=$(pgrep -f "cass index --watch" | head -1)
echo "Watcher PID: $WATCHER_PID"
WATCHER_PATH=$(ps -o args= -p "$WATCHER_PID" | awk '{print $1}')
"$WATCHER_PATH" --version   # Must show: cass 0.2.7-gj.1
```

### Rollback procedure
```bash
cp ~/.cargo/bin/cass.pre-011-backup ~/.cargo/bin/cass
PLIST="$HOME/Library/LaunchAgents/com.cass.index-watch.plist"
launchctl unload "$PLIST" 2>/dev/null && sleep 1 && launchctl load "$PLIST"
# Verify rollback:
cass --version    # Must NOT show 0.2.7-gj.1
cass health --json
```

---

## Verification

### Compiler gates (bare commands — exit code determines pass/fail)
```bash
# Each command must exit 0. If any fail, stop and fix before proceeding.
~/.cargo/bin/cargo check --all-targets
~/.cargo/bin/cargo clippy --all-targets -- -D warnings
~/.cargo/bin/cargo fmt --check
~/.cargo/bin/cargo test --lib
```

### Post-deploy verification
```bash
# 1. Version unmistakably GJ
cass --version
# Required output: cass 0.2.7-gj.1

# 2. Watchdog command is reachable (runtime smoke test)
cass watchdog run
# Must not print "error: unrecognized subcommand" or panic

# 3. Watcher is running the NEW binary (path verification)
WATCHER_PID=$(pgrep -f "cass index --watch" | head -1)
WATCHER_PATH=$(ps -o args= -p "$WATCHER_PID" | awk '{print $1}')
echo "Watcher binary: $WATCHER_PATH"
"$WATCHER_PATH" --version   # Must show: cass 0.2.7-gj.1

# 4. Diff shows only our additions
git diff upstream/main --name-only -- src/
# Required: src/lib.rs, src/watchdog.rs ONLY

# 5. Health check
sleep 180
cass health --json | python3 -c "
import sys, json
d = json.load(sys.stdin)
print('healthy:', d['healthy'])
print('watchdog:', d.get('state',{}).get('watchdog',{}))
"
# Required: healthy: True, both plist fields True

# 6. Crash loop check
grep "LockBusy\|drop_close" ~/Library/Logs/cass-index-watch.log | tail -3
# Required: all entries pre-date deployment timestamp

# 7. Incremental scan confirmed (spec R5)
grep "incremental_scan\|full_scan" ~/Library/Logs/cass-index-watch.log | tail -10
# Required: at least one incremental_scan after first full_scan
```

---

## Requirement Traceability

| Requirement | Change | Location |
|-------------|--------|----------|
| R1 — Upstream parity | `git checkout upstream/main -- src/ ...` | Phase 1 |
| R2 — Minimal local delta | watchdog.rs + 6 lib.rs sites + Cargo.toml | Phases 1-3 |
| R3 — Self-contained watchdog | libc dep added; only lib.rs wiring sites modified | Cargo.toml + lib.rs |
| R4 — Build/deploy (stable toolchain) | Upstream's toolchain copied; stable safe for watchdog.rs | Phase 1 + Phase 5 |
| R5 — Watcher healthy | Schema v9-v14 from upstream; incremental_scan verified | Post-deploy |
| User ask — GJ version | `version = "0.2.7-gj.1"` in Cargo.toml | Phase 2 |

---

## Source Code Anchors (verified in current src/lib.rs)

- Line 13: `pub mod watchdog;`
- Line 499-505: `Watchdog { #[command(subcommand)] command: Option<watchdog::WatchdogCommand>, }`
- Line ~2733: outer dispatch OR pattern where `Commands::Watchdog { .. }` must be added
- Lines 2277-2284: inner dispatch arm (`code: 3, kind: "watchdog"` static str, `?;`)
- Line 936-938: `CliError.kind` is `&'static str`
- Line 2456: `Some(Commands::Watchdog { .. }) => "watchdog".to_string()`
