---
title: "Clean up fork additions: drop codebuff/DoctorConnector, bump FAD to main, restore crush"
date: 2026-03-29
bead: coding_agent_session_search-hhm0
---

<!-- Codex Review: APPROVED after 3 rounds | model: gpt-5.3-codex | date: 2026-03-29 -->
<!-- Status: REVISED -->
<!-- Revisions: clarified stale doctor/shim assumptions as already satisfied; preserved generic doctor reconciliation; made crush restoration adapter-backed; added watch-state compatibility and checkout-local verification requirements -->
<!-- issue:complete:v1 | harness: pi/claude-sonnet-4-6 | date: 2026-03-29T11:07:28Z -->

# Spec 009 — Fork Addition Cleanup

## Context

After the upstream v0.2.5 sync (spec 008), we re-evaluated every file that differs
from upstream. The user confirmed codebuff is not needed. Auditing the rest:

## What we're changing and why

### DROP: `src/connectors/codebuff.rs`
Not needed. User confirmed.

### ALREADY SATISFIED: doctor/shim cleanup
The older shaping snapshot assumed `src/doctor.rs`, `pub mod doctor;`, and a
`ConnectorExt` shim still existed in this checkout. They do not. This spec now
treats doctor/shim cleanup as already satisfied before implementation of spec 009
begins, and focuses the live code changes on Codebuff removal, the FAD bump,
Crush restoration, and watch-state compatibility.

### BUMP: FAD dep → `rev = "de450843"` with `crush` feature
FAD main has crush. Our Cargo.toml already has frankensqlite as a git dep.
FAD declares frankensqlite as `path = "../frankensqlite/..."` — a path dep that
only resolves in their private monorepo.

Fix: Cargo `[patch]` section redirects FAD's internal frankensqlite path dep to
our git dep:

```toml
[patch."https://github.com/Dicklesworthstone/franken_agent_detection"]
frankensqlite = { git = "https://github.com/Dicklesworthstone/frankensqlite",
                  rev = "92a9a0fa", package = "fsqlite" }
```

This is the standard Cargo mechanism for exactly this scenario.

### RESTORE: `src/connectors/crush.rs`
Upstream has it. We dropped it in spec 008 because FAD v0.1.3 lacked crush.
With FAD bumped to main (crush feature enabled, frankensqlite resolved via patch),
`src/connectors/crush.rs` must be restored as a **local wrapper module** and wired
through this repo's adapter-backed `Connector` integration, not as a bare upstream
re-export that bypasses the local trait.

## What stays

- `src/watchdog.rs` — launchd watchdog subcommand. Upstream has nothing like it.
- SIGTERM/heartbeat/PID in `src/indexer/mod.rs` — upstream has none of this.
  Required for graceful watcher shutdown and watchdog liveness detection.
- Watchdog wiring in `src/lib.rs` (5 sites) — required for `cass watchdog` CLI.
- The generic `cass doctor` reconciliation loop in `src/lib.rs` — keep it. Codebuff
  should disappear from reconciliation because it is removed from connector factories,
  not because the shared doctor feature is deleted.

## Acceptance criteria

- [ ] Codebuff removed from the live connector registry and all runtime references removed
- [ ] `src/connectors/codebuff.rs` deleted only after explicit written approval, per repo rules
- [ ] doctor/shim cleanup remains satisfied in the live checkout and is not reintroduced
- [ ] generic `cass doctor` reconciliation preserved, and Codebuff no longer appears because factories no longer return it
- [ ] FAD dep bumped to `rev = "de450843"`, `crush` feature enabled
- [ ] `[patch]` section added to redirect FAD's frankensqlite path dep
- [ ] `src/connectors/crush.rs` restored as a local wrapper and Crush enabled via adapter-backed integration
- [ ] watch-state loading tolerates removed connector keys without resetting live timestamps
- [ ] `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --lib` all pass
- [ ] checkout-local `target/debug/cass watchdog run` smoke works under sandboxed `CASS_DATA_DIR` using documented exit codes
