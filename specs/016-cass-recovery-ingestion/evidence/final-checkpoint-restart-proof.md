---
title: "Final checkpoint restart proof"
date: 2026-05-17T01:18:03Z
bead: coding_agent_session_search-1vxuf
---

# Final Checkpoint Restart Proof

## Symptom

The checkpoint-focused verifier failed after `close_storage_after_index`:

```text
test indexer::tests::close_storage_after_index_checkpointing_close_does_not_leave_backfillable_wal_frames ... FAILED
left: Integer(97)
right: Integer(0)
```

That meant a second `PRAGMA wal_checkpoint(FULL)` could still backfill frames after the index close path had supposedly finished its final checkpoint.

## Root Cause

`close_storage_after_index` closed the indexing storage handle and then ran a final `PRAGMA wal_checkpoint(FULL)`. A `FULL` checkpoint can backfill frames, but it does not guarantee a reset of the WAL generation. After deferred bulk ingest, the next opener could therefore still observe the same generation as backfillable work.

The earlier `TRUNCATE` shape avoided that but could create a zero-byte WAL sidecar, which this recovery has already proven is risky with older frankensqlite refresh paths. The right final-close operation is `RESTART`: backfill and reset the WAL generation without truncating the sidecar to zero bytes.

## Change

`src/indexer/mod.rs` now runs:

```text
PRAGMA wal_checkpoint(RESTART);
```

for the final post-index checkpoint.

## Verification

```text
env CARGO_TARGET_DIR=/tmp/cass-check-target $HOME/.cargo/bin/cargo test close_storage_after_index_checkpointing_close_does_not_leave_backfillable_wal_frames --lib -- --nocapture
result: pass, 1 passed
```

```text
env CARGO_TARGET_DIR=/tmp/cass-check-target $HOME/.cargo/bin/cargo test checkpoint --lib
result: pass, 58 passed
```

```text
$HOME/.cargo/bin/cargo fmt --check
result: pass

env CARGO_TARGET_DIR=/tmp/cass-check-target $HOME/.cargo/bin/cargo check --all-targets
result: pass

env CARGO_TARGET_DIR=/tmp/cass-check-target $HOME/.cargo/bin/cargo clippy --all-targets -- -D warnings
result: pass
```

`rch` was unavailable in this session, so the verification used the direct `$HOME/.cargo/bin/cargo` fallback with the same target directory.

## Release Candidate Refresh

```text
env CARGO_TARGET_DIR=/tmp/cass-release-target $HOME/.cargo/bin/cargo build --release --bin cass
result: pass
path: /tmp/cass-release-target/release/cass
version: cass 0.4.7
sha256: 077674c65899936a79885d24cf141e1ac05632e5bd201958a1a6a992fda20594
```

Shadow canaries with the refreshed release candidate:

```text
health --stale-threshold 86400: healthy=true, state.index.status=ready, checkpoint.completed=true, checkpoint.db_matches=true
pi_agent    "ATT21_COL_CFP_SceneMachine_EndCard.psd" total_matches=30   elapsed_ms=44 search_ms=20
claude_code "frankensqlite"                         total_matches=37   elapsed_ms=25 search_ms=1
codex       "freelist serializer"                   total_matches=10   elapsed_ms=56 search_ms=29
opencode    "opencode"                              total_matches=2484 elapsed_ms=57 search_ms=32
factory     "factory"                               total_matches=21   elapsed_ms=41 search_ms=19
```

## Verification Refresh

After removing the direct panic/unreachable markers from the changed indexer/storage/UI test paths, the release candidate was rebuilt again:

```text
env CARGO_TARGET_DIR=/tmp/cass-release-target $HOME/.cargo/bin/cargo build --release --bin cass
result: pass
path: /tmp/cass-release-target/release/cass
version: cass 0.4.7
sha256: db3dbb0a9652bc5cadfa9a7d824da13a529d9cd2ad6ad85dc169a0760b0a7f1c
```

Shadow canaries with the latest release candidate:

```text
health --stale-threshold 86400: healthy=true, state.index.status=ready, checkpoint.completed=true, checkpoint.db_matches=true
pi_agent    "ATT21_COL_CFP_SceneMachine_EndCard.psd" total_matches=30   elapsed_ms=24
claude_code "frankensqlite"                         total_matches=37   elapsed_ms=26
codex       "freelist serializer"                   total_matches=10   elapsed_ms=25
opencode    "opencode"                              total_matches=2484 elapsed_ms=24
factory     "factory"                               total_matches=21   elapsed_ms=24
```

## Remaining Blocker

```text
ubs src/indexer/mod.rs
result: pass
summary: 0 critical, 5962 warnings, 2052 info
```

The indexer-only UBS criticals are cleared.

## Changed-File Verifier Refresh

After cleaning the remaining changed-file UBS criticals in `src/lib.rs`,
`src/ui/app.rs`, and `src/indexer/redact_secrets.rs`, the verifier floor was
rerun:

```text
$HOME/.cargo/bin/cargo fmt --check
result: pass

env CARGO_TARGET_DIR=/tmp/cass-check-target $HOME/.cargo/bin/cargo check --all-targets
result: pass

env CARGO_TARGET_DIR=/tmp/cass-check-target $HOME/.cargo/bin/cargo clippy --all-targets -- -D warnings
result: pass

env CARGO_TARGET_DIR=/tmp/cass-check-target $HOME/.cargo/bin/cargo test memoizing_redactor_quarantined_entries_fall_through_to_direct_redaction --lib
result: pass, 1 passed

env CARGO_TARGET_DIR=/tmp/cass-check-target $HOME/.cargo/bin/cargo test state_save --lib
result: pass, 9 passed

env CARGO_TARGET_DIR=/tmp/cass-check-target $HOME/.cargo/bin/cargo test global_robot_format_overrides_subcommand_json_format --lib
result: pass, 1 passed

env CARGO_TARGET_DIR=/tmp/cass-check-target $HOME/.cargo/bin/cargo test search_without_mode_uses_hybrid_preferred_default_intent --lib
result: pass, 1 passed

env CARGO_TARGET_DIR=/tmp/cass-check-target $HOME/.cargo/bin/cargo test refresh_catchup_flags_are_opt_in_for_search_and_tui --lib
result: pass, 1 passed
```

Changed-file UBS now exits successfully for the local preflight command:

```text
git diff --name-only -- '*.rs' Cargo.toml Cargo.lock | tr '\n' '\0' | xargs -0 ubs --format=json --jsonl-summary-only
result: pass
summary: 0 critical, 19148 warnings, 10752 info, 9 files
```

Release candidate was rebuilt after the cleanup:

```text
env CARGO_TARGET_DIR=/tmp/cass-release-target $HOME/.cargo/bin/cargo build --release
result: pass
path: /tmp/cass-release-target/release/cass
version: cass 0.4.7
sha256: 423e2e4c2920ec74a38a5cb4af1f00de362a4a82e493d342b4891179f4955ada
```

Shadow canaries with the latest release candidate:

```text
health --stale-threshold 86400: healthy=true, state.index.status=ready, checkpoint.completed=true, checkpoint.db_matches=true
pi_agent    "ATT21_COL_CFP_SceneMachine_EndCard.psd" total_matches=30   elapsed_ms=24 search_ms=1
claude_code "frankensqlite"                         total_matches=37   elapsed_ms=23 search_ms=0
codex       "freelist serializer"                   total_matches=10   elapsed_ms=23 search_ms=0
opencode    "opencode"                              total_matches=2484 elapsed_ms=23 search_ms=0
factory     "factory"                               total_matches=21   elapsed_ms=23 search_ms=0
```

Remaining caveat: the local changed-file UBS command is clear for criticals and exits zero, but the repository CI policy says `ubs --ci --fail-on-warning` blocks on warnings. The changed-file warning inventory remains large and still needs an explicit baseline/policy decision before final merge claims.
