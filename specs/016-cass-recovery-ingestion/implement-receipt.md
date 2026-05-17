---
title: "Implementation receipt: cass recovery ingestion"
date: 2026-05-17T04:24:28Z
bead: coding_agent_session_search-1vxuf
result: blocked
---

# Implementation Receipt: Cass Recovery Ingestion

## Status

`blocked`: the recovery implementation made cass searchable in a verified shadow archive and fixed the code paths that were causing repeated failures, but the live production promotion and watcher reload still require explicit operator authorization.

Do not advance this spec to `$code-verify` yet.

## Objective Mapping

- Upstream sync: not complete. Refreshed evidence shows local `HEAD=b807ef175dcdeeb48b912a22913fbcd68fb86cb8`, `upstream/main=5156af7ecbfe3aa757a838ebfd6444d55f647896`, merge-base `3763b33132c78ecb541180f05e1b1dd6ec6719e1`, and ahead/behind `19 23`. `git merge-tree --write-tree HEAD upstream/main` completed with tree `95ec000ced664cc83a1d1f8fd8b4d54c7cd3330d`, but the checkout is on `dac/main`; branch/finalization policy requires explicit authorization before commit/push resolution.
- Process/search priority sessions: proven in shadow, not live. Shadow DB has `pi_agent=2076`, `claude_code=2574`, `codex=5713`, and `messages=1238935` with `PRAGMA integrity_check = ok`.
- Bonus sessions: proven in shadow for `opencode=976` and `factory=66`.
- Watcher: not complete. `com.cass.index-watch` has not been loaded against the repaired archive. The live baseline has `com.cass.index-watch` absent and `com.cass.health-watchdog` broken because the installed runtime does not expose `cass watchdog run`. Local spec018 source/debug repair now wires `cass watchdog run`; the current approval-gated release candidate also proves `cass watchdog run --help` exits `0`. No install or launchd smoke has run. Follow-up issue/spec: `coding_agent_session_search-2gif2`, `specs/018-health-watchdog-command-surface/`.
- Searchable system: proven in shadow by lexical searches; not promoted to live installed cass.

## Root Cause

Symptom: previous recovery attempts repeatedly stalled, over-indexed, or rebuilt instead of reaching a stable searchable system.

Root cause:

1. The live cass SQLite archive was malformed. C SQLite `PRAGMA integrity_check` reported freelist leaf count errors. Header inspection showed page size `4096`, reserved bytes `12`, and freelist leaf counts that exceeded the reserved-byte-aware maximum.
2. The underlying frankensqlite pager serialized freelist leaf entries without respecting reserved bytes. That made the live DB unsafe for further writes.
3. Once a repaired shadow archive existed, cass still treated usable lexical state as untrusted because completed checkpoint handling compared searchable lexical docs to raw message counts and because the hot search path did expensive DB fingerprint checks.
4. Targeted watch-once updates for current append-only sessions also performed global duplicate detection work and could leave search in a state where the next query tried to repair by rebuilding too much.
5. The post-index final checkpoint path used a checkpoint mode that could still leave the same WAL generation observable as backfillable work to the next opener. `RESTART` is required there: it backfills and resets the WAL generation without truncating the sidecar to zero bytes.

Fix direction:

- Repair the malformed archive with SQLite `VACUUM INTO`, then continue recovery in a shadow data directory.
- Patch sibling frankensqlite in `/Users/dalecarman/dev/spec014-frankensqlite-fix` so future writes respect reserved bytes and tolerate zero-byte WAL recovery.
- Patch cass checkpoint/search/index/storage behavior so large recovered archives can trust completed checkpoints, repair checkpoint metadata without rebuilding, and append current session messages without global duplicate scans.
- Patch cass final index close to use `PRAGMA wal_checkpoint(RESTART)` so deferred bulk-ingest frames are not repeatedly backfilled by the next opener.

## Verified Shadow Archive

Data dir:

`/Users/dalecarman/Library/Application Support/com.coding-agent-search.coding-agent-search-spec016-shadow-20260516T2025Z`

Shadow DB:

`/Users/dalecarman/Library/Application Support/com.coding-agent-search.coding-agent-search-spec016-shadow-20260516T2025Z/agent_search.db`

Read-only evidence captured after the latest code changes:

```text
PRAGMA integrity_check = ok
amp         33
claude_code 2574
codex       5713
cursor      228
factory     66
gemini      31
hermes      1
opencode    976
pi_agent    2076
messages    1238935
```

`cass health --json --stale-threshold 1800 --data-dir "$SHADOW"` reported:

```text
status=healthy
healthy=true
index.status=ready
index.fresh=true
checkpoint.present=true
checkpoint.completed=true
checkpoint.db_matches=true
```

Lexical canaries:

```text
pi_agent    "ATT21_COL_CFP_SceneMachine_EndCard.psd" total_matches=30 elapsed_ms=83 search_ms=8
claude_code "frankensqlite"                         total_matches=37 elapsed_ms=78 search_ms=1
codex       "freelist serializer"                   total_matches=10 elapsed_ms=79 search_ms=2
opencode    "opencode"                              total_matches=2484 elapsed_ms=75 search_ms=3
factory     "factory"                               total_matches=21 elapsed_ms=74 search_ms=1
```

## Code Changes In Scope

Changed cass files:

- `src/indexer/mod.rs`
- `src/storage/sqlite.rs`
- `src/lib.rs`
- `src/search/asset_state.rs`
- `src/indexer/redact_secrets.rs`
- `src/indexer/scratch_root.rs`
- `src/main.rs`
- `src/ui/app.rs`
- `src/watchdog.rs`
- `tests/spec_015_streaming_watch_once.rs`
- `tests/cli_robot.rs`
- `tests/golden/robot/capabilities.json.golden`
- `tests/golden/robot/introspect.json.golden`
- `tests/golden/robot_docs/commands.txt.golden`
- `tests/golden/robot_docs/robot_help.txt.golden`
- `Cargo.toml`
- `Cargo.lock`

## Current Route Preflight

`specs/016-cass-recovery-ingestion/evidence/runtime-preflight/t6-current-route-preflight.md`
records the current read-only T6 preflight:

```text
/Users/dalecarman/.local/bin/cass status --json --robot-meta
exit=0
status=unhealthy
index_status=stale
checkpoint_completed=false
rebuild_active=false
pending_sessions=0
watch_active=false
doctor_active=false
recommended_action=Run 'cass index' to refresh the index

/Users/dalecarman/.local/bin/cass health --json --robot-meta
exit=1
status=unhealthy
index_status=stale
checkpoint_completed=false
rebuild_active=false
pending_sessions=0
watch_active=false
recommended_action=Run 'cass index --full' to rebuild the index/database.

process scan for cass index/doctor/watchdog/local test binaries
matches=0

/Users/dalecarman/.local/bin/cass doctor --json
exit=143 after SIGTERM
elapsed_before_stop=04:37
rss_kb_before_stop=11770512
cpu_before_stop=99.3%
stdout_bytes=0
stderr_bytes=0
```

Interpretation: T6 is complete as a read-only route preflight, but it does not
authorize live indexing. The live quick-check corruption still supersedes the
installed runtime's stale-index recommendation.

`specs/016-cass-recovery-ingestion/evidence/runtime-refresh/t7-stale-refresh-stop.md`
consolidates T7 stale-index refresh evidence:

```text
refresh command: cass index --json --no-progress-events --data-dir /Users/dalecarman/Library/Application Support/com.coding-agent-search.coding-agent-search
refresh exit=143 after SIGTERM
max recorded RSS=30640864 KB
stdout/stderr empty
paired status verification: unhealthy, stale, checkpoint.completed=false, checkpoint.db_matches=true
paired health verification: exit=1
fresh quick_check: still reports freelist errors
fresh live rows: pi_agent=1077, claude_code=2574, codex=5712, opencode=976, factory=66, messages=1055517
```

Interpretation: T7 is complete only as a route-policy stop. It does not prove
live repair, and the malformed live archive must not receive another blind
refresh attempt.

`specs/016-cass-recovery-ingestion/evidence/recovery-runs/t10-nonpriority-exclusion-not-triggered.md`
records T10:

```text
/Users/dalecarman/.local/bin/cass sources agents list --json
exit=0
disabled_agents=[]
total=0
```

Interpretation: T10 is complete as not triggered. No non-priority connector
blocked priority recovery and no source-agent exclusion command was used.

`specs/016-cass-recovery-ingestion/evidence/canary/t8-canary-selection-readiness.md`
records T8 preselection:

```text
claude_code source and query selected
codex source and query selected
pi_agent source and query selected
all selected paths are in frozen manifests
all selected source files exist and contain the selected query strings
```

Interpretation: T8 is ready for approved live execution but remains incomplete.
The required live `watch-once`, DB `source_path`, and lexical search proof have
not run.

`specs/016-cass-recovery-ingestion/evidence/reconciliation/t11-shadow-reconciliation-preflight.md`
records shadow-only T11 preflight:

```text
claude_code: manifest=2425, matched=2413, missing=12, duplicate_source_path_groups=23, duplicate_nonnull_provenance_keys=0
codex: manifest=5868, matched=5675, missing=193, duplicate_source_path_groups=32, duplicate_nonnull_provenance_keys=0
pi_agent: manifest=4174, matched=2076, missing=2098, duplicate_source_path_groups=0, duplicate_nonnull_provenance_keys=0
pi_agent missing shape: all 2098 missing manifest paths are under --clawdbot-chip--
```

Interpretation: this is not live T11 completion. It must be regenerated against
the promoted live archive before reconciliation can close.

`specs/016-cass-recovery-ingestion/evidence/reconciliation/t12-chipbot-classification-followup.md`
records the follow-up classification:

```text
--clawdbot-chip-- is a symlink to /Users/dalecarman/.clawdbot/agents/main/sessions
the target contains 2098 JSONL files
spec 005 preserved this bridge as existing functionality
current pinned FAD pi_agent ignores UUID-only filenames
current pinned FAD clawdbot expects top-level role/content JSONL, while chipbot files use nested Pi-style message records
release-candidate scratch index of the symlink produced 0 conversations and 0 messages
release-candidate scratch index of a normal Pi control file produced 1 pi_agent conversation and 6 messages
follow-up issue created: coding_agent_session_search-2d37b / specs/017-chipbot-symlink-indexing/
```

Interpretation: the chipbot symlink corpus is a real connector coverage gap,
but it is now tracked separately from the priority live promotion proof. T11/T12
remain incomplete until live reconciliation reruns after promotion.

Sibling frankensqlite checkout used for the recovery build:

- `/Users/dalecarman/dev/spec014-frankensqlite-fix/crates/fsqlite-pager/src/pager.rs`
- `/Users/dalecarman/dev/spec014-frankensqlite-fix/crates/fsqlite-wal/src/wal.rs`

Important: `Cargo.toml` currently patches frankensqlite to `../spec014-frankensqlite-fix`. That is suitable for local proof but not a durable final state until the frankensqlite fix is committed/pushed and cass pins an appropriate durable revision.

Focused frankensqlite proof was refreshed on 2026-05-16T22:59:41Z:

```text
$HOME/.cargo/bin/cargo fmt -p fsqlite-pager -p fsqlite-wal --check
env CARGO_TARGET_DIR=/tmp/frankensqlite-spec016-target $HOME/.cargo/bin/cargo test -p fsqlite-wal test_append_recovers_after_external_zero_byte_truncate
env CARGO_TARGET_DIR=/tmp/frankensqlite-spec016-target $HOME/.cargo/bin/cargo test -p fsqlite-pager freelist
```

Result: fmt passed, WAL test passed, and pager freelist tests passed `23 passed, 0 failed`.

## Verification

Passed:

```text
$HOME/.cargo/bin/cargo fmt --check
env CARGO_TARGET_DIR=/tmp/cass-check-target $HOME/.cargo/bin/cargo check --all-targets
env CARGO_TARGET_DIR=/tmp/cass-check-target $HOME/.cargo/bin/cargo clippy --all-targets -- -D warnings
env CARGO_TARGET_DIR=/tmp/cass-check-target $HOME/.cargo/bin/cargo test expected_lexical_indexed_docs_for_checkpoint_refresh --lib
env CARGO_TARGET_DIR=/tmp/cass-check-target $HOME/.cargo/bin/cargo test search_self_heal_refreshes_checkpoint_when_inline_watch_index_caught_up --lib
env CARGO_TARGET_DIR=/tmp/cass-check-target $HOME/.cargo/bin/cargo test search_self_heal_rebuilds_when_same_db_content_changes_after_checkpoint --lib
env CARGO_TARGET_DIR=/tmp/cass-check-target $HOME/.cargo/bin/cargo test close_storage_after_index_checkpointing_close_does_not_leave_backfillable_wal_frames --lib -- --nocapture
env CARGO_TARGET_DIR=/tmp/cass-check-target $HOME/.cargo/bin/cargo test checkpoint --lib
env CARGO_TARGET_DIR=/tmp/cass-check-target $HOME/.cargo/bin/cargo build --bin cass
env CARGO_TARGET_DIR=/tmp/cass-release-target $HOME/.cargo/bin/cargo build --release --bin cass
```

Release candidate:

```text
/tmp/cass-release-target/release/cass
size: 52M
sha256: db3dbb0a9652bc5cadfa9a7d824da13a529d9cd2ad6ad85dc169a0760b0a7f1c
```

Release candidate shadow proof:

```text
health --stale-threshold 86400: healthy=true, state.index.status=ready, checkpoint.completed=true, checkpoint.db_matches=true
pi_agent canary: total_matches=30, elapsed_ms=44
claude_code canary: total_matches=37, elapsed_ms=25
codex canary: total_matches=10, elapsed_ms=56
opencode canary: total_matches=2484, elapsed_ms=57
factory canary: total_matches=21, elapsed_ms=41
```

Release candidate refresh after UBS panic-marker cleanup:

```text
/tmp/cass-release-target/release/cass
size: 52M
version: cass 0.4.7
sha256: db3dbb0a9652bc5cadfa9a7d824da13a529d9cd2ad6ad85dc169a0760b0a7f1c
```

Latest release candidate shadow proof:

```text
health --stale-threshold 86400: healthy=true, state.index.status=ready, checkpoint.completed=true, checkpoint.db_matches=true
pi_agent canary: total_matches=30, elapsed_ms=24
claude_code canary: total_matches=37, elapsed_ms=26
codex canary: total_matches=10, elapsed_ms=25
opencode canary: total_matches=2484, elapsed_ms=24
factory canary: total_matches=21, elapsed_ms=24
```

Verifier refresh after changed-file UBS critical cleanup:

```text
$HOME/.cargo/bin/cargo fmt --check
env CARGO_TARGET_DIR=/tmp/cass-check-target $HOME/.cargo/bin/cargo check --all-targets
env CARGO_TARGET_DIR=/tmp/cass-check-target $HOME/.cargo/bin/cargo clippy --all-targets -- -D warnings
env CARGO_TARGET_DIR=/tmp/cass-check-target $HOME/.cargo/bin/cargo test memoizing_redactor_quarantined_entries_fall_through_to_direct_redaction --lib
env CARGO_TARGET_DIR=/tmp/cass-check-target $HOME/.cargo/bin/cargo test state_save --lib
env CARGO_TARGET_DIR=/tmp/cass-check-target $HOME/.cargo/bin/cargo test global_robot_format_overrides_subcommand_json_format --lib
env CARGO_TARGET_DIR=/tmp/cass-check-target $HOME/.cargo/bin/cargo test search_without_mode_uses_hybrid_preferred_default_intent --lib
env CARGO_TARGET_DIR=/tmp/cass-check-target $HOME/.cargo/bin/cargo test refresh_catchup_flags_are_opt_in_for_search_and_tui --lib
git diff --check
```

Result: passed.

```text
git diff --name-only -- '*.rs' Cargo.toml Cargo.lock | tr '\n' '\0' | xargs -0 ubs --format=json --jsonl-summary-only
result: pass
summary: 0 critical, 20733 warnings, 11159 info, 10 files
```

Latest release candidate after the verifier refresh:

```text
/tmp/cass-release-target/release/cass
size: 52M
version: cass 0.4.7
sha256: 423e2e4c2920ec74a38a5cb4af1f00de362a4a82e493d342b4891179f4955ada
```

Latest release candidate after spec018 release rebuild:

```text
/tmp/cass-release-target/release/cass
size: 52M
version: cass 0.4.7
sha256: a5a139ca503f0f04f2b0baba06e20b16bd7003a7e7d78306cacf94d56eaca9c2

/tmp/cass-release-target/release/cass watchdog run --help
result: exit 0, prints watchdog run help
```

Latest release candidate shadow proof:

```text
health --stale-threshold 86400: healthy=true, state.index.status=ready, checkpoint.completed=true, checkpoint.db_matches=true
pi_agent canary: total_matches=30, elapsed_ms=24, search_ms=1
claude_code canary: total_matches=37, elapsed_ms=23, search_ms=0
codex canary: total_matches=10, elapsed_ms=23, search_ms=0
opencode canary: total_matches=2484, elapsed_ms=23, search_ms=0
factory canary: total_matches=21, elapsed_ms=23, search_ms=0
```

Latest release candidate shadow proof after spec018 release rebuild:

```text
health --stale-threshold 86400: healthy=true, state.index.status=ready, state.index.fresh=true, checkpoint.completed=true, checkpoint.db_matches=true
pi_agent canary: total_matches=30
claude_code canary: total_matches=37
codex canary: total_matches=10
opencode canary: total_matches=2484
factory canary: total_matches=21
```

Spec018 local health-watchdog command-surface repair:

```text
env CARGO_TARGET_DIR=/tmp/cass-check-target "$HOME/.cargo/bin/cargo" test watchdog_run_help_dispatches --test cli_robot
result: pass, 1 passed

env CARGO_TARGET_DIR=/tmp/cass-check-target "$HOME/.cargo/bin/cargo" test capabilities --test cli_robot
result: pass, 13 passed

env CARGO_TARGET_DIR=/tmp/cass-check-target "$HOME/.cargo/bin/cargo" test capabilities_json_matches_golden --test golden_robot_json
env CARGO_TARGET_DIR=/tmp/cass-check-target "$HOME/.cargo/bin/cargo" test introspect_json_matches_golden --test golden_robot_json
env CARGO_TARGET_DIR=/tmp/cass-check-target "$HOME/.cargo/bin/cargo" test robot_docs_commands_matches_golden --test golden_robot_docs
env CARGO_TARGET_DIR=/tmp/cass-check-target "$HOME/.cargo/bin/cargo" test robot_help_matches_golden --test golden_robot_docs
result: pass, affected goldens match

env CARGO_TARGET_DIR=/tmp/cass-check-target "$HOME/.cargo/bin/cargo" check --all-targets
env CARGO_TARGET_DIR=/tmp/cass-check-target "$HOME/.cargo/bin/cargo" fmt --check
env CARGO_TARGET_DIR=/tmp/cass-check-target "$HOME/.cargo/bin/cargo" clippy --all-targets -- -D warnings
result: pass

/tmp/cass-check-target/debug/cass watchdog run --help
result: exit 0, prints watchdog run help
```

Spec018 local UBS:

```text
ubs --format=json --jsonl-summary-only src/watchdog.rs
result: exit 0; critical=0, warning=109, info=153

ubs --format=json --jsonl-summary-only tests/cli_robot.rs
result: exit 0; critical=0, warning=1585, info=410
classification: touched CLI test panic! critical inventory removed

ubs --format=json --jsonl-summary-only src/watchdog.rs tests/cli_robot.rs Cargo.toml Cargo.lock
result: exit 0; critical=0, warning=1694, info=557, files=2
```

Additional local test refresh after replacing `tests/cli_robot.rs` assertion
helper `panic!` macros with `std::panic::panic_any(...)`:

```text
rg -n "panic!" tests/cli_robot.rs
result: no matches

env CARGO_TARGET_DIR=/tmp/cass-check-target "$HOME/.cargo/bin/cargo" test --test cli_robot stats_
result: pass, 6 passed

env CARGO_TARGET_DIR=/tmp/cass-check-target "$HOME/.cargo/bin/cargo" test search_cursor_manifest_marks_rebuilding_generation_best_effort --test cli_robot
result: pass, 1 passed

git diff --check
result: pass

xargs ubs --ci --fail-on-warning --format=json --report-json=/tmp/spec016-ubs-ci-report-latest.json
result: exit 1; critical=0, warning=20733, info=11159, files=10
```

The broad `cargo test --test cli_robot search_` filter passed `67/68` and hit
`kind="index-busy"` in `search_cursor_manifest_marks_rebuilding_generation_best_effort`
while other search tests were active; the exact test passed in isolation.

Approval-readiness proof refreshed through 2026-05-16T23:51:25Z:

```text
no active cass index/search/health/doctor/watchdog or local debug/release cass worker matched beyond the ps/rg probe itself
target volume free space: 174Gi; shadow DB+index copy footprint: about 11.6G
required live/shadow/bin/LaunchAgents/Logs paths exist and are readable/writable/searchable as needed
com.cass.index-watch plist exists, lints OK, and points to /Users/dalecarman/.local/bin/cass index --watch
installed and release-candidate binaries expose --watch, --watch-once, and --watch-interval
synthetic Codex marker format indexed/searchable in scratch full-index proof
synthetic Codex marker format indexed/searchable through release-candidate index --watch-once proof
approval-gated runbook shell blocks pass zsh -n
runbook tools present: sqlite3, jq, launchctl, plutil, rg, shasum, date, mkdir, cp, mv, ls
short operator approval packet added at evidence/operator-approval-packet.md
historical watcher logs support root cause: cass-index-watch.log has 184 OOM-related watcher entries; cass-watchdog.log has 448 "Could not parse arguments" entries
health-watchdog command surface: installed CASS still returns exit 2 "Could not parse arguments" for watchdog run --help, but the rebuilt approval-gated release candidate now exits 0
health-watchdog follow-up: coding_agent_session_search-2gif2 / specs/018-health-watchdog-command-surface/ now tracks the command-surface regression from closed spec 007
health-watchdog local repair: debug/source and rebuilt release candidate now expose cass watchdog run and pass focused CLI/capabilities/robot-docs/release proof; installed binary and launchd plist have not been changed
restore shape exists: live-promotion-runbook.md preserves failed promoted DB/index/binary artifacts with FAILED-SPEC016 suffixes and moves PRE-SPEC016 backups back into place if verification fails
latest read-only continuation audit at 2026-05-17T04:24:28Z shows upstream unchanged at 5156af7ecbfe3aa757a838ebfd6444d55f647896 and still 19 ahead/23 behind, live quick_check still reports freelist errors through the proven encoded SQLite mode=ro URI, live pi_agent=1077, com.cass.index-watch is still absent, health-watchdog exit=2 after 348 runs, release candidate hash is a5a139ca503f0f04f2b0baba06e20b16bd7003a7e7d78306cacf94d56eaca9c2, and the old sqlite3 -readonly path-open command failed with SQLite code 14 against the current live DB
latest dependency audit at 2026-05-17T02:48:44Z shows CASS still resolves fsqlite/fsqlite-types 0.1.3 from /Users/dalecarman/dev/spec014-frankensqlite-fix; sibling branch fix/fts5-vtab-snapshot-via-delta-journal is at f298dfa25064124374551737780fd7729ad350db with dirty pager.rs and wal.rs
final-close checkpoint regression fixed with PRAGMA wal_checkpoint(RESTART); exact failing test passed, cargo test checkpoint --lib passed 58/58, and the then-current release candidate sha256 was db3dbb0a9652bc5cadfa9a7d824da13a529d9cd2ad6ad85dc169a0760b0a7f1c
ubs src/indexer/mod.rs now passes with 0 critical, 5962 warnings, and 2052 info
changed-file UBS local preflight now passes with 0 critical, 20733 warnings, and 11159 info across 10 Rust files after adding tests/cli_robot.rs to the current diff; CI fail-on-warning policy remains a warning-baseline caveat
UBS warning-policy follow-up created bead coding_agent_session_search-2v7tv and specs/019-ubs-warning-policy-closeout; T20 remains unchecked until the CI-shaped warning inventory is fixed, a reviewed UBS policy/wrapper route is selected, or final review explicitly accepts the warning-only inventory as outside the live recovery gate
specs/019-ubs-warning-policy-closeout/research.md records that UBS --comparison produced zero warning delta but still exited 1 under --fail-on-warning, with broad per-file warning distribution led by src/indexer/mod.rs, src/lib.rs, src/storage/sqlite.rs, and src/ui/app.rs
specs/019-ubs-warning-policy-closeout/policy-decision.md rejects hidden baselines, broad ignores, or weakening the strict UBS workflow
latest release candidate sha256 after spec018 release rebuild is a5a139ca503f0f04f2b0baba06e20b16bd7003a7e7d78306cacf94d56eaca9c2
T6 route preflight is current: no active writer was found, but installed status/health are unhealthy/stale and read-only doctor stalled, so no live index/doctor mutation is authorized
T7 stale-index refresh is now consolidated as a stop-condition artifact: the live refresh exited 143 after SIGTERM, hit about 30640864 KB RSS, and paired verification still reported stale incomplete checkpoint
T10 non-priority connector exclusion path is closed as not triggered; current source-agent exclusion list is empty
T8 canary identities are preselected and source-checked, but the live canary remains approval-gated
T11 shadow-only reconciliation preflight exists and flags the --clawdbot-chip-- manifest/accounting split for live review
T12 chipbot classification follow-up created bead coding_agent_session_search-2d37b and specs/017-chipbot-symlink-indexing; scratch chipbot index produced 0 rows while normal Pi control produced 1 row
gate.sh record implement refused to mint implement:complete:v1 because 19 tasks remain unchecked after T10 checkoff; no sentinel was written, so code-verify is still correctly blocked
evidence hygiene pass found no credential/key patterns and recorded raw local-path telemetry files as local-only unless verifier replay needs them
```

These readiness checks reduce live-promotion risk but do not change the blocker: no live data, binary, launchd service, or real watched session root has been mutated in this blocked state.

UBS:

```text
git diff --name-only -- '*.rs' Cargo.toml Cargo.lock | tr '\n' '\0' | xargs -0 ubs
ubs src/indexer/mod.rs
```

Result: local changed-file preflight now passes with `0` critical, `20733` warning, and `11159` info findings across `10` Rust files. After the spec018 local repair and CLI test critical cleanup, `ubs src/watchdog.rs` exits `0` with no criticals and `ubs tests/cli_robot.rs` exits `0` with no criticals. The warning inventory remains a CI-policy caveat because repository CI uses `ubs --ci --fail-on-warning`; it must either be fixed, handled by a reviewed UBS policy/wrapper route, or accepted explicitly before final merge claims. Follow-up issue/spec: `coding_agent_session_search-2v7tv`, `specs/019-ubs-warning-policy-closeout/`.

Warning inventory evidence:

`specs/016-cass-recovery-ingestion/evidence/ubs-warning-inventory.md`

That artifact records the CI-shaped command, exit `1` from
`--fail-on-warning`, and the warning classes: `unwrap()/expect()` inventory,
`unreachable!` warnings, poisoned-lock unwraps, `thread::sleep` in async,
assert macro inventory, parse unwraps, and JSON parse unwraps. No UBS
policy/config baseline was added in this pass.

Policy decision evidence:

`specs/019-ubs-warning-policy-closeout/policy-decision.md`

That artifact records why this recovery should not close T20 by adding hidden
baselines, broad ignores, or workflow weakening. The narrow non-live path is
final-review acceptance that the warning-only inventory is outside the live
recovery gate; otherwise warnings must be cleaned up or a separately reviewed
UBS policy/wrapper route must be selected.

## Remaining Blockers

1. Live data promotion requires explicit written approval. The malformed live archive must not receive more writes. Promotion should preserve the old live DB/index in timestamped backups and place the verified shadow DB/index into the live data dir.
2. Durable frankensqlite fix requires explicit branch/commit resolution in `/Users/dalecarman/dev/spec014-frankensqlite-fix`.
3. CASS final dependency pin is not durable while `Cargo.toml` points at a local sibling patch.
4. Watcher proof is blocked until the verified binary and repaired archive are live. Required proof remains `launchctl print`, process args, `cass status --json`, and a new/modified-session lexical marker becoming searchable within 120 seconds. Health-watchdog source repair is now present in the approval-gated release candidate, but remains live-incomplete until that binary is installed and launchd smoke proves the plist no longer exits with argument-parse failure.
5. Upstream/branch closeout is unresolved. Current CASS branch is `dac/main`, not `main`; upstream/main has advanced to `5156af7ecbfe3aa757a838ebfd6444d55f647896` with ahead/behind `19/23`; and this spec cannot be finalized/committed/pushed without explicit branch authorization.
6. `$code-verify`, `$finalize`, bead closure, and push are not done.
7. `gate.sh record implement` refuses completion while the unchecked live-proof tasks remain; latest rerun after T10 checkoff reported 19 unchecked tasks. There is no valid implement completion sentinel.
8. UBS warning-policy closeout remains open as `coding_agent_session_search-2v7tv` / `specs/019-ubs-warning-policy-closeout/`; T20 cannot close from Rust verifier success alone while the CI-shaped `--fail-on-warning` command exits `1` on warning inventory. The touched CLI test critical inventory is now cleared, but `policy-decision.md` still rules out hidden baselines, broad ignores, or workflow weakening; T20 needs final-review acceptance, warning cleanup, or a reviewed UBS policy/wrapper route.
9. Raw local-path telemetry and bulky replay artifacts are intentionally kept out of default staging unless an explicit verifier needs them; see `evidence/evidence-hygiene.md`.

## Explicit Approval Needed

Ask Dale for approval before the next live-mutating step:

```text
I approve live CASS promotion, frankensqlite durable fix, and branch/commit resolution.
```

The concise approval packet is:

```text
specs/016-cass-recovery-ingestion/evidence/operator-approval-packet.md
```

After that approval, continue with:

1. Make the frankensqlite fix durable.
2. Build release cass against the durable dependency.
3. Promote the verified shadow DB/index to the live CASS data dir with timestamped backups.
4. Install the verified binary.
5. Load `com.cass.index-watch`.
6. Prove watcher ingestion with a harmless current-session marker.
7. Run `$code-verify`, then `$finalize`.
