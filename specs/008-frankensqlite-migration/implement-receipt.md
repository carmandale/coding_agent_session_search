---
baseline_sha: 4fa18299b5fe626151aef62188906ab298388926
end_sha: b2399cc3
test_command: cargo check
test_result: fail
test_count: 0
---

<!-- implement:complete:v1 | harness: pi/claude-sonnet-4-6 | date: 2026-03-27T18:30:34Z -->

# Implementation Receipt

## Changed Files (worktree: /tmp/cass-merge-base, branch: feat/008-upstream-sync)

**Phase 2 — Merge commit (c61b6275):**
- Cargo.toml: all path deps converted to git deps
- Cargo.lock: updated
- 182 upstream files merged

**Phase 3 — Our additions (b2399cc3):**
- src/watchdog.rs (new)
- src/connectors/codebuff.rs (new)
- src/doctor.rs (new — DoctorConnector + ConnectorExt compatibility shims)
- src/connectors/mod.rs (codebuff added, crush removed)
- src/connectors/crush.rs (removed — FAD v0.1.3 lacks crush feature)
- src/indexer/mod.rs (SIGTERM/heartbeat/PID/codebuff wiring)
- src/lib.rs (watchdog 5-site wiring, health JSON fix)
- Cargo.toml (frankensqlite→92a9a0fa, FAD→v0.1.3 tag, signal-hook added)
- rust-toolchain.toml (stable→nightly)

## Test Output Summary

**cargo check** — BLOCKED. Build fails due to asupersync API incompatibility.

Root cause: upstream's code (update_check.rs, model_download.rs, ui/app.rs, search/query.rs)
uses asupersync HTTP/runtime APIs (HttpClient::builder, Runtime::current_handle, Cx::now,
http::h1::MultipartForm, Response::bytes/text, etc.) that don't exist in any public git
revision of asupersync that also compiles with Rust nightly 1.94.0.

Available options:
- asupersync d72f93e: has all used APIs BUT has a broken test fixture causing Cargo parse
  warning AND some asupersync internal code fails nightly type checking
- asupersync ce6bfc28 (latest): fixture fixed BUT has `ref mut` pattern nightly error (lab.rs:803)
- asupersync 7b0dae0f: ref mut fixed BUT has 18 different nightly errors in Result method calls

**The fundamental issue:** upstream develops asupersync, frankensqlite, and cass in a private
monorepo with path deps. The public git revisions of asupersync have not been stabilized for
external git dep consumption with nightly 1.94.0.

## Known Remaining Compile Errors (beyond asupersync)

1. `count_disk_files` removed from Connector impl → moved to DoctorConnector (FIXED)
2. `scan_with_callback`/`supports_streaming_scan` missing from FAD v0.1.3 → polyfilled via ConnectorExt shim in doctor.rs (FIXED)
3. `serde_json::json!` macro can't contain let bindings → health JSON watchdog field restructured (FIXED)
4. asupersync API compatibility → BLOCKED (see above)

## Next Steps to Unblock Build

Option A (recommended): Clone asupersync as a local sibling path dep:
```bash
cd ~/dev  # or wherever cass lives
git clone https://github.com/Dicklesworthstone/asupersync
```
Then in Cargo.toml replace git dep with:
```toml
asupersync = { path = "../asupersync" }
```

Option B: Replace asupersync usage with tokio equivalents in:
- src/update_check.rs (heavy user)
- src/model_download.rs (2 uses)
- src/ui/app.rs (2 uses)
- src/lib.rs (1 use — spawn_blocking)

Option C: Wait for asupersync to release a version compatible with nightly 1.94.0.
