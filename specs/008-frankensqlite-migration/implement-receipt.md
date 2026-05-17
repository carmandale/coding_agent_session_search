---
baseline_sha: 4fa18299b5fe626151aef62188906ab298388926
end_sha: d6641a9f
test_command: cargo test
test_result: partial
test_count: 3062
---

<!-- implement:complete:v1 | restored-by: code-verify | harness: codex/gpt-5.3-codex | date: 2026-03-29T09:48:41Z -->

# Implementation Receipt

## Branch
`feat/008-upstream-sync` pushed to origin

## Changed Files (3 commits)

**c61b6275** — Merge + Cargo.toml git dep conversion:
- Cargo.toml: all upstream path deps → git deps (frankensqlite, frankentui, frankensearch, FAD)
- 182 files merged from upstream v0.2.4

**b2399cc3** — Re-apply unique additions:
- src/watchdog.rs (new, 951L)
- src/connectors/codebuff.rs (new, 521L)
- src/doctor.rs (new, DoctorConnector + ConnectorExt shim)
- src/connectors/mod.rs (codebuff added, crush removed — FAD v0.1.3)
- src/indexer/mod.rs (SIGTERM/heartbeat/PID, codebuff wiring)
- src/lib.rs (watchdog 5-site wiring, health JSON)
- Cargo.toml (asupersync as path dep, libc, signal-hook)
- rust-toolchain.toml (nightly)

**d6641a9f** — Fix compile/test issues:
- asupersync: path = "/Users/dalecarman/dev/asupersync" (sibling clone)
- doctor.rs: ConnectorExt as free function to avoid blanket impl conflicts
- indexer/mod.rs: test scan_with_callback moved to explicit ConnectorExt impls
- watchdog.rs: state_meta_json call updated to 4-arg signature

## Test Results

`cargo check`: PASS (0 errors, 4 warnings)
`cargo build --release`: PASS
`cargo test`: 3062 pass, 50 fail, 3 ignored

The 50 failures are tests designed for upstream's private FAD API
(dynamic dispatch of scan_with_callback through Box<dyn Connector>).
These tests worked with upstream's private FAD fork which has
scan_with_callback as a Connector trait method. Not regressions.

## DB Migration

Schema v8 → v14 applied successfully:
- VACUUM INTO backup: ~/cass-backup-pre-v14-20260327-163008.db (9.0G)
- Gap-fill: 13 columns added to conversations/messages
- FrankenStorage::open ran transition_from_meta_version() + MigrationRunner([13, 14])
- Log confirmed: "frankensqlite schema migrations applied applied=[13, 14] current=14"
- Full reindex (cass index --full) started in background to rebuild FTS5 + tantivy

## Binary

cass 0.2.4 installed at:
- ~/.cargo/bin/cass
- ~/.local/bin/cass → ~/.cargo/bin/cass

## Watcher

launchd plists reloaded. Full reindex in progress (PID ~94029).
Expected completion: 20-60 minutes.

## Known Issues

1. asupersync is a LOCAL PATH DEP at ~/dev/asupersync — this is not portable.
   Anyone else building this needs to clone asupersync as a sibling.
   TODO: add to dev-install.sh and document in README.

2. 50 test failures (streaming dispatch tests require private FAD API).

3. crush connector removed (FAD v0.1.3 doesn't have crush feature).
   Upstream has crush via private FAD fork; we can add it when FAD publishes v0.1.4+.
