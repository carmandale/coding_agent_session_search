---
title: "Tasks: pi_agent watch-once memset stall"
date: 2026-05-15
bead: coding_agent_session_search-373b1
---

<!-- plan:complete:v1 | harness: unknown | date: 2026-05-15T16:23:04Z -->
<!-- Codex Review: APPROVED after 5 rounds | model: gpt-5.3-codex | date: 2026-05-15 | trust_level: full | round_records: .codex-round-91909777/, .codex-round-7f3ae320/, .codex-round-11ebb7d2/, .codex-round-8f680d33/, .codex-round-a8204eb3/ | Status: REVISED -->

# Tasks: pi_agent watch-once memset stall

Ordered checklist for the implementer. Plan source of truth lives in [`plan.md`](plan.md); spec is [`spec.md`](spec.md). Tasks are grouped under H2 headers — `/codex-implement` chunks at group boundaries for per-chunk Codex validation.


## Group A — Phase 1: Profile and localise

- [x] T1: Build a profiling-symbol binary: `cargo build --profile profiling --bin cass` (profile already defined in Cargo.toml with `debug = true, strip = false`).
- [x] T2: Stop launchd watcher (`launchctl bootout gui/$(id -u)/com.cass.index-watch`). Reproduce the stall: `./target/profiling/cass index --watch-once ~/.pi/agent/sessions --json --no-progress-events > /tmp/pi-stall-repro.log 2>&1 &`. Wait until the indexer has been at 99 % CPU for at least 5 minutes with the DB row count frozen.
- [x] T3: Capture symbolised stacks: `sample <pid> 10 -wait > /tmp/pi-stall-sample.txt`. Identify the top Rust frame inside the `_platform_memset` call chain. Also run `lsof -p <pid> | grep jsonl` to identify the open file (if a single conversation is wedged) and `vmmap -summary <pid>` to characterise the 22 GB allocation (heap vs anon vs mapped).
- [x] T4: Write `specs/014-pi-agent-memset-stall/notes/T4-profile-evidence.md` with: pid, elapsed, peak RSS, top three Rust frames with file:line, current open jsonl, top-three byte sources from `vmmap -summary`. Kill the indexer process. Reload the launchd watcher.
- [⚠ ESCALATED] T5: Decision blocked — profile evidence does NOT match plan.md's binding decision tree. Allocation site is `frankensqlite_ext_fts5::Fts5Table::snapshot_state`, an external crate outside the C1–C5 candidate space. See `notes/T5-candidate-decision.md` for the four follow-up paths (D1 upstream fsqlite fix, D2 vtab savepoint knob, D3 deferred-lexical-updates cass-side, D4 deeper spike). User decision required before any further work.

## Group B — Caller-preservation analysis

- [ ] T6: Read the five caller sites Phase A identified (`src/model/conversation_packet.rs:513`, `:668`; `src/indexer/mod.rs:18763`; `src/storage/sqlite.rs:10309`; `src/pages/export.rs:351`). For each, record what `extra_json` fields it reads, whether the read is conditional on agent slug, and whether it has a fallback when fields are absent. Output: `specs/014-pi-agent-memset-stall/notes/T6-extra-callers.md`.
- [ ] T7: Cross-reference T6 with the existing codex compactor (`compact_large_connector_extras` and `compact_indexer_message_extra` in `src/indexer/mod.rs:17540+`). Confirm whether pi compaction needs to preserve `cass.token_usage` in addition to the codex set (`cass.model`, `cass.attachments`). Document the pi-specific `cass.*` envelope in T6's note.
- [ ] T8: Draft the caller-preservation contract as a single unit test outline (not yet implemented). Place it in T6's note. Test must assert: (a) compaction emits the documented envelope, (b) packet hash on compacted form is stable, (c) redaction logic in `indexer/mod.rs:18763` produces unchanged shape.

## Group C — Implement chosen candidate

- [ ] T9: Implement the candidate selected in T5. Write boundary is constrained by the candidate:
  - **C1**: `src/connectors/pi_agent.rs` (replace re-export with wrapper struct), `src/indexer/mod.rs` (call-site change), possibly new file `src/connectors/pi_agent_compact.rs` for the wrapper logic.
  - **C2**: this spec cycle does not ship C2 alone — see T9a for the C2-in-cycle coordination path. cass diff is empty if C2 lands and the dep rev is bumped within the cycle; if not, T9a pivots to C4 (cass-owned streaming parser) — no downgrade-and-defer to C3.
  - **C3**: `src/indexer/mod.rs` lines 17561–17574 (broaden the gate). `compact_indexer_message_extra` extended to preserve `cass.token_usage` if missing from current codex set.
  - **C4**: `src/connectors/pi_agent.rs` replaced with a streaming parser; substantial new code in `src/connectors/pi_streaming.rs` or similar. **Discover-source parity required**: the cass-owned parser must implement `discover_source_files()` returning the same `Vec<PathBuf>` set FAD's pi connector returns at [`pi_agent.rs:557`](https://github.com/Dicklesworthstone/franken_agent_detection/), otherwise `capture_connector_sources_before_parse()` at `src/indexer/mod.rs:17209` will see empty input and raw-mirror linkage will silently break. Add a directory-root raw-mirror regression test as part of T13.
  - **C5**: extend `ingest_watch_batch_with_oom_split()` at `src/indexer/mod.rs:15579` with a pre-flight byte estimate; if batch estimate > threshold (default 1 GB, env `CASS_WATCH_BATCH_MEMORY_BUDGET_BYTES`), recursively halve before calling `ingest_batch_with_semantic_delta`. **The estimator must be non-allocating**: walking `message.content.len()` (String byte length, free) plus a recursive descent over `message.extra: &Value` that accumulates byte length via match arms (Number/String/Bool/Null → 0–32 bytes constant; Array → recurse + brackets; Object → recurse + braces + key.len()). Do NOT call `serde_json::to_vec`/`to_string` on `message.extra` to size it — that would allocate the very bytes we are trying to avoid (Codex /codex-review round 2 finding). Single-conv quarantine path reuses existing `record_watch_poison_conversation`.
- [ ] T9a (only if T5 selected C2): coordinate upstream FAD PR — draft on `franken_agent_detection`, get reviewed, merged, tag a release. Bump `Cargo.toml` pin to the fixed rev. If the FAD PR cannot merge within this spec cycle's window, do NOT downgrade-and-defer (that violates the binding in-cycle rule in plan.md ## Architecture). Instead, escalate to the user to either widen the spec cycle to accommodate the upstream landing, or pivot to C4 (cass-owned streaming parser) as the in-cycle fix path. T9 is replanned and re-executed with the new candidate.
- [ ] T10: Run the full check suite required by repo conventions (AGENTS.md:202–214, AGENTS.md:219–234):
  - `rch exec -- env CARGO_TARGET_DIR=/tmp/cass-check-target cargo check --all-targets`
  - `rch exec -- env CARGO_TARGET_DIR=/tmp/cass-check-target cargo clippy --all-targets -- -D warnings`
  - `rch exec -- env CARGO_TARGET_DIR=/tmp/cass-check-target cargo fmt --check`
  - `cargo build --release` (the package is `coding-agent-search`, not `cass` — see `Cargo.toml:1`; `cargo` resolves the workspace binary correctly without `-p`).
  - **UBS pre-merge gate** (AGENTS.md:219–234): `ubs $(git diff --name-only --cached)` before commit; CI runs `ubs --ci --fail-on-warning` on changed files.

  Commit on `dac/main` (authorized via prior `/allow-branch` for this session — the spec-014 work is part of the goal session that opted into the branch; see `napkin.md` / global §2.10 if checking). Focused commit message mirroring PR #233 style.

## Group D — Regression test

- [ ] T11: Synthesise a large-pi-conversation fixture in `tests/fixtures/pi_agent/sessions/--Users-dalecarman-test--/<date>_<uuid>.jsonl`. Use synthetic content (no real user data) sized to ~10 MB with high message count (500+) and large `toolCall.arguments` / `thinking` blobs. Document the fixture's intended trigger conditions in its sibling `README.md`.
- [ ] T12: Implement the caller-preservation contract unit test from T8 against the new fixture. Place in `src/indexer/mod.rs` test module near the existing codex compactor tests (line 34308+).
- [ ] T13: Add an integration test in `tests/connector_pi_agent.rs` that loads the fixture, runs ingest, and asserts: (a) peak RSS under a cap (configurable via env, default 1 GB for CI), (b) all synthesized messages survive in the DB, (c) raw-mirror manifest links to the fixture file.
- [ ] T13a: Raw-mirror reconstruction equivalence test. After T13's ingest writes the compacted rows, look up the raw-mirror blob for the fixture conversation via `RawMirrorDbLink`, re-read the blob bytes, re-parse with FAD's pi parser (same crate version pinned in `Cargo.toml`), compare the reconstructed `message.extra` Values structurally with the pre-compaction Values. Assert the lossy-but-recoverable contract: every dropped field is reconstructable from the raw blob; no message is structurally lost. Place near T13 in `tests/connector_pi_agent.rs`.
- [ ] T13b: PR #233 chunk-size regression test. New unit test in `src/indexer/mod.rs` test module asserting: (i) `watch_ingest_chunk_size()` honors `CASS_WATCH_INGEST_CHUNK_SIZE=8` env override and returns 8; (ii) given a synthetic 100-conv batch handed to a mock ingest path with chunk size 8, exactly 13 chunked ingest calls fire (not 1, not 100). This guards against the spec-013 single-chunk regression independent of compactor changes.
- [ ] T14: Run the test suite via `rch exec` per repo convention (AGENTS.md:343): `rch exec -- env CARGO_TARGET_DIR=/tmp/cass-test-target cargo test --release`. Confirm all existing tests + the new T12/T13/T13a/T13b tests pass. (The workspace package is `coding-agent-search`, not `cass`; `-p coding-agent-search` is acceptable but `cargo test` resolves correctly without it.)

## Group E — Full-corpus verification on user's machine

- [ ] T15: Stop launchd watcher. WAL-checkpoint the DB. APFS-snapshot DB as `agent_search.db.PRE-PI-VERIFY-<date>`. Record pre-run pi state: `sqlite3 <db> "SELECT id, source_path FROM conversations c JOIN agents a ON c.agent_id=a.id WHERE a.slug='pi_agent' ORDER BY id;" > specs/014-pi-agent-memset-stall/notes/T15-pre-run-pi-rows.txt`. This is the no-data-loss baseline that T18 verifies.
- [ ] T15a: Install the fix-bearing binary to the shipping path. `cp -p target/release/cass ~/.local/bin/cass.real` (preserves the existing `~/.local/bin/cass` symlink that points at `cass.real`). Verify: `~/.local/bin/cass --version` reports 0.4.7, `stat -f "%m %N" ~/.local/bin/cass.real` mtime matches the just-built binary, `shasum -a 256 ~/.local/bin/cass.real target/release/cass` shows identical hashes. Without this step, T16 verifies the wrong binary — the spec's single-source-of-truth constraint (`spec.md:59`) is the load-bearing rationale.
- [ ] T16: Run `~/.local/bin/cass index --watch-once ~/.pi/agent/sessions --json --no-progress-events > /tmp/pi-verify-run.log 2>&1` against the user's full corpus. While running, sample RSS every 60 s into `specs/014-pi-agent-memset-stall/notes/T16-rss.txt`.
- [ ] T17: **Capture mid-run sample first** (acceptance #4 evidence requires a live process). While the indexer PID is still alive at >50 % CPU (between minute 5 and "process exits"), run `sample <pid> 10 -wait > specs/014-pi-agent-memset-stall/notes/T17-mid-run-sample.txt`. Assert `_platform_memset` is no longer the top frame on the active producer thread. Then, after the run completes (or fails), parse `/tmp/pi-verify-run.log` for `success`, `conversations`, `messages` and record the totals.
- [ ] T18: Verify acceptance criteria.
  - **#1 — count gate**: `sqlite3 <db> "SELECT COUNT(*) FROM conversations c JOIN agents a ON c.agent_id=a.id WHERE a.slug='pi_agent';"` must return ≥ 1,970.
  - **#2 — peak RSS**: take the max value from `specs/014-pi-agent-memset-stall/notes/T16-rss.txt`; must be < 8 GB (8,388,608 KB).
  - **#3 — chunk-size preserved**: spot-check by reading lock-file phase output during run, confirm the run cycles through bounded chunks.
  - **#4 — post-fix sample**: T17's `notes/T17-mid-run-sample.txt` must show `_platform_memset` is no longer the top frame on the active producer thread.
  - **#5 — message coverage**: spot-check 3 random pi conversations. Compare the DB's `messages` row count for each conversation against the *expected count from the actual FAD pi parser*, NOT a hand-rolled flattener. FAD's `flatten_message_content()` at `pi_agent.rs:156–164` recognises text blocks AND `thinking` blocks (rendered as `[Thinking] …`) AND `toolCall` blocks (rendered as `[Tool: name] …`) — a simplified script that only handles text undercounts the exact shapes this spec's fixture is designed to exercise (Codex /codex-review round 4 finding).

    **Required approach: small Rust harness using the pinned connector.**
    ```rust
    // tests/expected_pi_messages.rs (a separate test binary or example), gated on a feature so it doesn't pollute the default test set
    use franken_agent_detection::{PiAgentConnector, Connector};
    fn expected_message_count(jsonl_path: &Path) -> usize {
        let conn = PiAgentConnector::new();
        let convs = conn.scan(/* single-file ScanContext */).unwrap();
        convs.iter().map(|c| c.messages.len()).sum()
    }
    ```
    Call this harness once per spot-check conversation; assert DB count equals harness count. The harness uses the exact same `FAD` rev pinned in `Cargo.toml`, so the C4 streaming-parser variant (if selected in T5) gets its parity test against the real contract automatically. (Bonus: this same harness powers T13a's raw-mirror reconstruction equivalence test — single source of truth.)
  - **No data loss (spec.md:61 constraint)**: every id in `notes/T15-pre-run-pi-rows.txt` must still exist in `conversations` after the run. Run `comm -23 <(sort -n notes/T15-pre-run-pi-rows.txt | awk -F'|' '{print $1}') <(sqlite3 <db> "SELECT id FROM conversations c JOIN agents a ON c.agent_id=a.id WHERE a.slug='pi_agent' ORDER BY id;" | sort -n)` — output must be empty (no missing rows).
  - **Quarantine reconciliation — strict set-equality (no silent misses)**: build three source-path sets and assert the discovered set equals indexed ∪ quarantined exactly. Per spec.md:66, any skipped file MUST appear in quarantine.
    ```
    # Discovered (corpus): find ~/.pi/agent/sessions -name "*.jsonl" -type f | sort > /tmp/T18-discovered.txt
    # Indexed (DB): sqlite3 <db> "SELECT source_path FROM conversations c JOIN agents a ON c.agent_id=a.id WHERE a.slug='pi_agent';" | sort > /tmp/T18-indexed.txt
    # Quarantined (file may be absent — treat as empty set, not error):
    #   if [ -f <data_dir>/quarantine/watch_ingest_poison.jsonl ]; then
    #     jq -r '.source_path' <data_dir>/quarantine/watch_ingest_poison.jsonl | sort > /tmp/T18-quarantined.txt
    #   else
    #     : > /tmp/T18-quarantined.txt
    #   fi
    # Accounted set: sort -u /tmp/T18-indexed.txt /tmp/T18-quarantined.txt > /tmp/T18-accounted.txt
    # Unaccounted: comm -23 /tmp/T18-discovered.txt /tmp/T18-accounted.txt
    ```
    The unaccounted set MUST be empty. If any file is unaccounted, the run fails acceptance and the implementer must either re-run or explicitly classify each unaccounted path in the implement-receipt with a documented reason (file vanished mid-run, parser-aborted-without-quarantine bug, etc.) — never as "slack".
- [ ] T19: Reload launchd watcher. Confirm forward capture resumes via `launchctl list | grep cass` and `cass status`.

## Group F — Implementation provenance

- [ ] T20: Spot-check 3 random claude_code conversations and 3 random codex conversations in the DB after T18 to confirm no spec-013 regression. Write outcomes to `specs/014-pi-agent-memset-stall/notes/T20-no-regression.md`.
- [ ] T21: Write `specs/014-pi-agent-memset-stall/implement-receipt.md` with: candidate selected, files changed, test results, full-corpus verification numbers, raw-mirror linkage check.
- [ ] T22: Run `~/.agent-config/scripts/gate.sh record implement specs/014-pi-agent-memset-stall/` and `gate.sh verify implement specs/014-pi-agent-memset-stall/`. This is implementation provenance, not project closeout.

Out-of-scope-for-tasks-md (handled by the goalbuddy board sequence after `/code-verify` + `/finalize`):
- Bead closure → `/finalize`
- Final handoff artifact under `thoughts/shared/handoffs/` → `/finalize`
- Upstream PR opening on `Dicklesworthstone/coding_agent_session_search` → goalbuddy board T006 PM task, post-`/finalize`
- Commit + push to `origin/dac/main` of all spec-014 artifacts → `/finalize`
