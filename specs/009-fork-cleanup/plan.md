---
title: "Fork cleanup: drop codebuff/doctor, bump FAD to main, restore crush"
date: 2026-03-29
bead: coding_agent_session_search-hhm0
---

<!-- Codex Review: APPROVED after 3 rounds | model: gpt-5.3-codex | date: 2026-03-29 -->
<!-- Status: REVISED -->
<!-- Revisions: re-baselined stale doctor/shim work; preserved generic doctor reconciliation; made crush adapter-backed and mandatory; moved watch-state migration before Codebuff removal; tightened checkout-local verification -->
<!-- plan:complete:v1 | harness: pi/claude-sonnet-4-6 | date: 2026-03-29T12:02:22Z -->

# Plan — Spec 009: Fork Addition Cleanup

## What

Re-baseline spec 009 against the current checkout instead of the stale shaping snapshot.
In the live tree, `src/doctor.rs`, `pub mod doctor;`, and `ConnectorExt`/`scan_with_callback`
call sites are already absent, so the real implementation scope is narrower:

1. remove Codebuff from the live connector registry
2. bump `franken-agent-detection` to `de450843` with the `crush` feature and patch its
   `fsqlite` path dependency
3. restore/enable Crush in a way that matches this repo's local `Connector` abstraction
4. make watch-state loading tolerant of removed connector keys
5. preserve the existing generic `cass doctor` reconciliation flow

Current baseline:
- `src/doctor.rs` is already absent
- `cargo test --lib` is already green: `1179 passed; 0 failed; 0 ignored`
- verification must use the checkout-local binary (`target/debug/cass`), not the installed
  `cass` on PATH, which reports a newer connector set than this source tree

Baseline proof:

```text
$ cargo test --lib
test result: ok. 1179 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Implementation steps

### Step 0 — Reconcile the spec/plan contract before calling implementation complete

This spec carries stale acceptance text from an older branch snapshot. Before implementation is
called complete, writeback must update the acceptance language so it matches the live codebase:

- replace "lib.rs codebuff reconciliation block removed" with "generic reconciliation preserved;
  Codebuff removed from factories so doctor no longer includes it"
- replace the crush criterion with a concrete one: "`src/connectors/crush.rs` restored as a local
  wrapper and Crush enabled through adapter-backed integration"
- note explicitly that `doctor.rs` / `pub mod doctor` / `ConnectorExt` cleanup was already
  satisfied in the current checkout before spec 009 implementation began

No requirement should remain in the final spec if the plan intentionally does the opposite.

### Step 1 — Cargo + dependency update
- Change the FAD dependency from `rev = "5b0eb1a"` to `rev = "de450843"` and enable
  the `crush` feature.
- Add a `[patch]` section that redirects FAD's internal `fsqlite` path dependency to
  the existing git source used by this repo:

```toml
[patch."https://github.com/Dicklesworthstone/franken_agent_detection"]
frankensqlite = { git = "https://github.com/Dicklesworthstone/frankensqlite", rev = "92a9a0fa", package = "fsqlite" }
```

Run `cargo check --all-targets` immediately after the dependency update to catch API or
resolution drift before connector wiring changes. Regenerate and review `Cargo.lock` as part
of this step so the git-source delta is explicit.

### Step 2 — Re-baseline stale doctor/shim assumptions as already satisfied

Do not schedule fresh code changes for `src/doctor.rs`, `pub mod doctor;`, or
`ConnectorExt` migration in this checkout. Instead:

- record that these artifacts are already absent from live source
- remove them from the implementation plan/tasks as action items
- carry the note forward into writeback so the spec directory explains that the plan was
  corrected to the live codebase, not implemented against a stale snapshot

### Step 3 — Make watch-state loading tolerant before connector removal

There is no `WatchState` struct to edit in this checkout. The live code deserializes
`watch_state.json` straight into `HashMap<ConnectorKind, i64>`, which means removing a
connector can break deserialization if old files still contain that key.

Replace `load_watch_state()` with tolerant object parsing first:

- parse raw JSON into `serde_json::Value`
- iterate object keys manually
- map recognized keys to `ConnectorKind`
- ignore unknown or removed connector keys instead of failing the whole load
- explicitly cover likely historical removed-key spellings in tests (`Codebuff`, and any short
  alias forms if found during fixture review)
- keep `save_watch_state()` as the single writer so newly written files normalize to the
  current key set

Regression tests:

- legacy JSON containing a removed connector key plus current keys still loads the current keys
- current save/load round-trip behavior remains unchanged

This step must land before `ConnectorKind::Codebuff` is removed so there is no transient window
where an intermediate build can zero out watch-state timestamps on legacy files.

### Step 4 — Remove Codebuff from the live registry only

Update only the places where Codebuff still exists in live source:

- `src/connectors/mod.rs` — remove `pub mod codebuff;`
- `src/indexer/mod.rs` import block — remove `codebuff::CodebuffConnector`
- `src/indexer/mod.rs::get_connector_factories()` — remove the `"codebuff"` factory entry
- `src/indexer/mod.rs::ConnectorKind::from_slug()` — remove `"codebuff"`
- `src/indexer/mod.rs::ConnectorKind::create_connector()` — remove the `Self::Codebuff` branch
- `src/indexer/mod.rs::ConnectorKind` — remove the `Codebuff` variant

Then run `rg -n "codebuff|Codebuff" README.md src tests Cargo.toml Cargo.lock` to confirm
no live runtime references remain outside historical/spec artifacts.

`src/connectors/codebuff.rs` should be treated as a separate file-deletion step because
this repo forbids deleting files without explicit written user permission. The implementation
plan should therefore:

- proceed with Codebuff fully detached from the build/registry
- request explicit permission before deleting `src/connectors/codebuff.rs`
- perform the delete only after that permission is granted

### Step 5 — Restore `src/connectors/crush.rs` and wire Crush through this repo's local abstraction

`src/connectors/crush.rs` is a required deliverable, not an optional cleanup. However, it cannot
be just the upstream bare re-export: in this fork, indexer factories produce `Box<dyn Connector + Send>`
from the repo-local trait, and FAD-backed connectors are adapted through `src/connectors/fad_adapter.rs`.

Implementation shape:

- create/restore `src/connectors/crush.rs` as a local wrapper module
- extend `src/connectors/fad_adapter.rs` to import FAD's `CrushConnector`
- add `pub fn crush() -> Box<dyn Connector + Send>` implemented via `FadAdapter::new(...)`
- have the wrapper module delegate to the adapter-backed integration rather than claiming the
  upstream re-export alone is sufficient
- wire the new constructor/module into `src/connectors/mod.rs`, `src/indexer/mod.rs`, and
  `ConnectorKind` + slug mapping for `crush`

### Step 6 — Preserve the existing generic doctor reconciliation feature

Do not remove the reconciliation loop in `src/lib.rs`. The current code is a generic
per-connector feature from spec 004, not a dead codebuff-only block.

Instead, verify that Codebuff removal naturally shrinks the reconciliation surface because
`get_connector_factories()` no longer returns Codebuff. If tests are missing, add one focused
regression that proves the reconciliation path still works for the remaining connectors after
registry cleanup.

### Step 7 — Verification in this checkout only

Use checkout-local commands and artifacts, not the installed `cass` on PATH:

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test --lib
```

Additional targeted proof:

- watch-state legacy-key regression test(s)
- crush factory/registry test(s)
- doctor reconciliation regression test (if existing coverage does not already prove registry-based behavior)
- checkout-local CLI smoke using the built binary, not PATH `cass`:

```bash
CASS_DATA_DIR=/tmp/cass-watchdog-<run-id> target/debug/cass watchdog run
```

The smoke should exercise the command path in a sandboxed data dir and confirm it exits with a
documented watchdog status without panicking. Because `default_data_dir()` honors `CASS_DATA_DIR`,
this can be done without touching the real user state. Automation must treat the documented
watchdog exit codes as valid outcomes for this smoke:

- `0` = healthy or already locked
- `1` = stale watcher restarted
- `2` = watcher not running

Only an undocumented exit code, panic, or CLI crash should fail this smoke.

If the dependency bump makes `cargo test --lib` fail, stop and investigate the actual breakage.
Do not add `#[ignore]` as a shortcut; the current baseline is already green.

### Step 8 — Final diff, commit, and cleanup

- confirm the resulting live diff matches the intentional current-fork delta rather than the
  stale doctor/shim deletions from the earlier plan
- if file deletion approval was granted, delete `src/connectors/codebuff.rs` in the same change set;
  otherwise leave it detached and note the remaining cleanup precondition explicitly
- commit only after the full verification bundle passes

## Requirement traceability

| Req | Steps |
|-----|-------|
| Contract alignment | 0 (spec acceptance criteria reconciled during writeback) |
| R0 Remove codebuff | 4 (live registry removal), 8 (file delete only after explicit permission) |
| R1+R2 Bump FAD + support crush | 1 (Cargo/dependency change), 5 (adapter-backed crush integration) |
| R3 Restore crush.rs | 5 (mandatory local wrapper module + adapter-backed wiring) |
| R4 Keep tests green | 7 (`cargo test --lib` stays green; no ignored-test shortcut) |
| R5 Keep watchdog/SIGTERM | 7 (checkout-local watchdog CLI smoke), no watchdog code removal |
| R6/R7 doctor cleanup | 2 (already satisfied in live source; no new code action) |
| Watch-state forward-compat | 3 (tolerant loader + regression tests before Codebuff removal) |

## Source Context (Current Checkout Observations)

These are concrete observations from `/Users/dalecarman/dev/coding_agent_session_search` at review time.

### Files present / missing

- `src/connectors/codebuff.rs` exists.
- `src/connectors/crush.rs` does not exist.
- `src/doctor.rs` does not exist.
- current branch is `feat/007-watchdog-subcommand`
- `cargo test --lib` in this checkout is currently green: `1179 passed; 0 failed; 0 ignored`

### Cargo.toml

```toml
39: franken-agent-detection = { git = "https://github.com/Dicklesworthstone/franken_agent_detection", rev = "5b0eb1a", features = ["connectors"] }
```

There is currently no `[patch."https://github.com/Dicklesworthstone/franken_agent_detection"]` section.

### `src/connectors/mod.rs`

```rust
176-183:
pub mod aider;
pub mod amp;
pub mod chatgpt;
pub mod claude_code;
pub mod cline;
pub mod codebuff;
pub mod codex;
pub mod cursor;
```

### `src/indexer/mod.rs`

```rust
12-18:
use crate::connectors::{
    Connector, ScanRoot, aider::AiderConnector, amp::AmpConnector, chatgpt::ChatGptConnector,
    claude_code::ClaudeCodeConnector, cline::ClineConnector, codebuff::CodebuffConnector,
    codex::CodexConnector, cursor::CursorConnector, factory::FactoryConnector,
    gemini::GeminiConnector, opencode::OpenCodeConnector, pi_agent::PiAgentConnector,
};
```

```rust
777-794:
pub fn get_connector_factories() -> Vec<(&'static str, fn() -> Box<dyn Connector + Send>)> {
    vec![
        ("codex", || Box::new(CodexConnector::new())),
        ("cline", || Box::new(ClineConnector::new())),
        ("gemini", || Box::new(GeminiConnector::new())),
        ("claude", || Box::new(ClaudeCodeConnector::new())),
        ("opencode", || Box::new(OpenCodeConnector::new())),
        ("amp", || Box::new(AmpConnector::new())),
        ("aider", || Box::new(AiderConnector::new())),
        ("cursor", || Box::new(CursorConnector::new())),
        ("chatgpt", || Box::new(ChatGptConnector::new())),
        ("pi_agent", || Box::new(PiAgentConnector::new())),
        ("factory", || Box::new(FactoryConnector::new())),
        ("codebuff", || Box::new(CodebuffConnector::new())),
        ("copilot", crate::connectors::fad_adapter::copilot),
        ("clawdbot", crate::connectors::fad_adapter::clawdbot),
        ("openclaw", crate::connectors::fad_adapter::openclaw),
        ("vibe", crate::connectors::fad_adapter::vibe),
    ]
}
```

```rust
827-849:
impl ConnectorKind {
    fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "codex" => Some(Self::Codex),
            "cline" => Some(Self::Cline),
            "gemini" => Some(Self::Gemini),
            "claude" => Some(Self::Claude),
            "amp" => Some(Self::Amp),
            "opencode" => Some(Self::OpenCode),
            "aider" => Some(Self::Aider),
            "cursor" => Some(Self::Cursor),
            "chatgpt" => Some(Self::ChatGpt),
            "pi_agent" => Some(Self::PiAgent),
            "factory" => Some(Self::Factory),
            "codebuff" => Some(Self::Codebuff),
            "copilot" => Some(Self::Copilot),
            "clawdbot" => Some(Self::Clawdbot),
            "openclaw" => Some(Self::OpenClaw),
            "vibe" => Some(Self::Vibe),
```

```rust
851-873:
    fn create_connector(&self) -> Box<dyn Connector + Send> {
        match self {
            Self::Codex => Box::new(CodexConnector::new()),
            Self::Cline => Box::new(ClineConnector::new()),
            Self::Gemini => Box::new(GeminiConnector::new()),
            Self::Claude => Box::new(ClaudeCodeConnector::new()),
            Self::Amp => Box::new(AmpConnector::new()),
            Self::OpenCode => Box::new(OpenCodeConnector::new()),
            Self::Aider => Box::new(AiderConnector::new()),
            Self::Cursor => Box::new(CursorConnector::new()),
            Self::ChatGpt => Box::new(ChatGptConnector::new()),
            Self::PiAgent => Box::new(PiAgentConnector::new()),
            Self::Factory => Box::new(FactoryConnector::new()),
            Self::Codebuff => Box::new(CodebuffConnector::new()),
            Self::Copilot => crate::connectors::fad_adapter::copilot(),
            Self::Clawdbot => crate::connectors::fad_adapter::clawdbot(),
            Self::OpenClaw => crate::connectors::fad_adapter::openclaw(),
            Self::Vibe => crate::connectors::fad_adapter::vibe(),
```

```rust
1262-1299:
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ConnectorKind {
    Codex,
    Cline,
    Gemini,
    Claude,
    Amp,
    OpenCode,
    Aider,
    Cursor,
    ChatGpt,
    PiAgent,
    Factory,
    Codebuff,
    Copilot,
    Clawdbot,
    OpenClaw,
    Vibe,
}

fn state_path(data_dir: &Path) -> PathBuf {
    data_dir.join("watch_state.json")
}

fn load_watch_state(data_dir: &Path) -> HashMap<ConnectorKind, i64> {
    let path = state_path(data_dir);
    if let Ok(bytes) = fs::read(&path)
        && let Ok(map) = serde_json::from_slice(&bytes)
    {
        return map;
    }
    HashMap::new()
}
```

No `ConnectorExt`, `connector_scan_with_callback`, or `DoctorConnector` symbols were found in
`src/indexer/mod.rs` or `src/lib.rs`.

### `src/connectors/fad_adapter.rs`

```rust
1-6:
//! Adapter layer for `franken-agent-detection` (FAD) connectors.
//!
//! Bridges FAD's 2-method `Connector` trait to our 4-method trait, converting
//! FAD types to our structurally-identical types. This lets us use FAD
//! connectors for agents we don't have in-tree (copilot, clawdbot, openclaw,
//! vibe) without rewriting connector logic.
```

```rust
121-140:
impl<T: fad::Connector + Send> Connector for FadAdapter<T> {
    fn detect(&self) -> DetectionResult {
        convert_detection(self.inner.detect())
    }

    fn scan(&self, ctx: &ScanContext) -> Result<Vec<NormalizedConversation>> {
        let fad_ctx = convert_scan_context(ctx);
        let fad_convs = self.inner.scan(&fad_ctx)?;
        Ok(fad_convs.into_iter().map(convert_conversation).collect())
    }

    fn count_disk_files(&self) -> Option<usize> {
        None
    }

    fn reconciliation_notes(&self) -> Option<String> {
        None
    }
}
```

### `src/lib.rs`

```rust
1-12:
pub mod bookmarks;
pub mod connectors;
pub mod encryption;
pub mod export;
pub mod indexer;
pub mod model;
pub mod pages;
pub mod search;
pub mod sources;
pub mod storage;
pub mod ui;
pub mod update_check;
pub mod watchdog;
```

`rg -n "Disk-vs-DB reconciliation|codebuff|DoctorConnector|pub mod doctor" src/lib.rs`
returned no matches.

### Verification context

- `default_data_dir()` honors `CASS_DATA_DIR`, so watchdog CLI smoke tests can be sandboxed:

```rust
8168-8177:
pub fn default_data_dir() -> PathBuf {
    if let Ok(dir) = dotenvy::var("CASS_DATA_DIR") {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
```

- The installed `cass` on PATH reports `crate_version = "0.2.5"` and a connector list that does
  not match this source tree, so final verification must use the checkout-local binary
  (`target/debug/cass`) rather than PATH `cass`.
