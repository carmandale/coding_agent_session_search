---
planning: true
---
# Planning Transcript — Spec 009

**Driver:** WildCastle (pi/claude-sonnet-4-6)
**Challenger:** TrueViper (crew-challenger, claude-opus-4-6)
**Date:** 2026-03-29

## Challenges raised (all accepted)

**C1 — Site count wrong (HIGH):** Step 4 labeled "8 sites" but enumerated 10. R0 and R1 sites conflated. Fixed: split into 4a (R1: 6 ConnectorExt sites) and 4b (R0: 4 Codebuff removal sites).

**C2 — WatchState silent data loss (HIGH):** `WatchState` has `#[serde(deny_unknown_fields)]`. After removing `Codebuff` variant, any user with `"bf"` in watch_state.json loses ALL connector timestamps silently (not just codebuff's). Fix: remove `deny_unknown_fields` from WatchState (step 4c).

**C3 — Fork diff file list wrong (MEDIUM):** Plan said connectors/mod.rs stays different. After codebuff→crush swap it matches upstream exactly. Actual post-009 diff: watchdog.rs, indexer/mod.rs, lib.rs, Cargo.toml ([patch] section).

**C4 — supports_streaming_scan omission (LOW):** FAD main has default impl for this method; test structs only need scan_with_callback. Documented explicitly.

**C5 — Traceability table gap (LOW):** CodebuffConnector import removal at line 30 is R0, not R1. Fixed in table.
