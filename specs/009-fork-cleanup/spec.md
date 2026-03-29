---
title: "Clean up fork additions: drop codebuff/DoctorConnector, bump FAD to main, restore crush"
date: 2026-03-29
bead: coding_agent_session_search-hhm0
---

<!-- issue:complete:v1 | harness: pi/claude-sonnet-4-6 | date: 2026-03-29T11:07:28Z -->

# Spec 009 — Fork Addition Cleanup

## Context

After the upstream v0.2.5 sync (spec 008), we re-evaluated every file that differs
from upstream. The user confirmed codebuff is not needed. Auditing the rest:

## What we're changing and why

### DROP: `src/connectors/codebuff.rs`
Not needed. User confirmed.

### DROP: `DoctorConnector` trait from `src/doctor.rs`
The only implementation was `codebuff.rs`. With codebuff gone, the trait has zero
implementations and zero callers (except the dead reconciliation block in lib.rs).
Pure dead code.

### DROP: `ConnectorExt` shim from `src/doctor.rs`
This shim existed because FAD v0.1.3 lacked `scan_with_callback` on the `Connector`
trait. We polyfilled it with a free function and explicit test-struct impls.

FAD main (`de450843`) now has `scan_with_callback` natively on `Connector`:
```
pub trait Connector {
    fn scan_with_callback(...)
    fn supports_streaming_scan() -> bool
}
```
Bumping FAD makes the shim unnecessary AND fixes the 50 streaming dispatch test
failures (which failed because vtable dispatch couldn't find the method on the trait).

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
crush.rs can be restored.

## What stays

- `src/watchdog.rs` — launchd watchdog subcommand. Upstream has nothing like it.
- SIGTERM/heartbeat/PID in `src/indexer/mod.rs` — upstream has none of this.
  Required for graceful watcher shutdown and watchdog liveness detection.
- Watchdog wiring in `src/lib.rs` (5 sites) — required for `cass watchdog` CLI.
- `pub mod doctor;` in lib.rs — keep the module but it will just have the empty
  `DoctorConnector` trait stub for future use (or drop if truly empty).

## Acceptance criteria

- [ ] codebuff.rs deleted, all references removed
- [ ] DoctorConnector + ConnectorExt removed from doctor.rs (or doctor.rs deleted)
- [ ] lib.rs codebuff reconciliation block removed
- [ ] FAD dep bumped to `rev = "de450843"`, `crush` feature enabled
- [ ] `[patch]` section added to redirect FAD's frankensqlite path dep
- [ ] crush.rs restored from upstream
- [ ] `cargo check` clean
- [ ] `cargo test` passes with 0 failures (the 50 streaming tests should now pass)
- [ ] `cass watchdog run` still works
