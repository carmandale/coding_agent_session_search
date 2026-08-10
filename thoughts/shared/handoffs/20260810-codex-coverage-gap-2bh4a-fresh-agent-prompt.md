---
generation: 1
parent-session: 268f9f88-0042-4fd7-b013-c9736ec41246
next-action-class: executable
---

# Continue: fix coding_agent_session_search-codex-coverage-gap-2bh4a

Part 1 (failure semantics) is written, compiling, and its regression test passes.
Part 2 (recovery) is not started. The proof work for part 1 is not finished.

**Destructive and external-write approvals expired with the ending session and do not
transfer.** In particular: no mass reindex against the live archive, no full rebuild, no
file deletion, no `git push` without re-establishing that it is wanted (the parent's push
was blocked by the permission classifier; the branch is local only).

---

## Original goal and authorization, VERBATIM (2026-08-10)

> Fix coding_agent_session_search-codex-coverage-gap-2bh4a in this repo. Run 'br show coding_agent_session_search-codex-coverage-gap-2bh4a' first — a measurement session already established the full mechanism with code paths, so do NOT re-diagnose. Read its comments and build on them.
>
> The bug in one line: on 2026-06-01 the codex connector aborted mid-scan, the error was caught and logged at warn level, the partial output was kept, and the watermark advanced anyway — so the index believes it is complete while 3,186 files sit unindexed and unreported.
>
> TWO changes, in this order.
>
> 1. FAILURE SEMANTICS (the important one). A connector scan error must not leave the index claiming completeness. The measurement identified the sites: src/indexer/mod.rs:10675 (scan_with_callback flushes partial output on mid-stream Err, logs 'local scan failed', sends IndexMessage::ScanError) and src/indexer/mod.rs:11147-11165 (consumer stores it on ConnectorStats, logs streaming_scan_error, comment reads 'Continue processing'). Either hold the watermark for that connector, or record a durable per-connector coverage floor that survives restarts and is reported by 'cass health' and 'cass stats'. Do not decide this by preference — read how the watermark is consumed and pick whichever cannot silently regress; state which you chose and why in the bead. Continuing the overall run after one connector fails is correct and should stay; the defect is claiming success afterward.
>
> 2. RECOVERY. Build a path-scoped re-scan that can reindex a bounded set of files without a full rebuild. BUILD AND VERIFY IT ON COPIES ONLY. Do NOT execute a mass reindex against the live archive at '~/Library/Application Support/com.coding-agent-search.coding-agent-search/agent_search.db' — that is Dale's call and he has not made it. Report the exact command you would run and your measured estimate of how long it takes and how much it writes.
>
> Scope: this bead only. The flat-layout gap (codex-flat-layout-undiscovered-kfaid) and the pi_agent workspace gap (pi-agent-missing-workspaces-le8s1) are separate beads with different causes — do not fix them here, and say so if you find yourself tempted.
>
> Constraints. Read AGENTS.md first: this repo forbids file deletion and destructive git without explicit written permission, stricter than the global default. Delete nothing. Do not upgrade br. Live archive is READ-ONLY ('file:...?mode=ro'); it is byte-identical at its 2026-08-04 mtime and must stay that way. Do not run a full rebuild — the 2026-06-01 one is what caused this and took the corpus down for a day. 'cass status --json' hangs on this archive (bead status-json-hang-nvq59), so use 'cass stats --json' instead.
>
> Done means: a regression test that fails against today's code — an aborted connector scan must not produce an index that reports itself complete. Run the relevant tests and report real output. Claim the bead in_progress, record what you changed and why on it, 'br sync --flush-only', then commit code and the tracked .beads/issues.jsonl and push. If you cannot finish, checkpoint with what is unverified and the exact next command rather than leaving a dirty tree.

---

## Where the work lives

- **Worktree (cwd for all of this):**
  `/Users/dalecarman/dev/coding_agent_session_search/.claude/worktrees/codex-coverage-gap-2bh4a`
- **Branch:** `worktree-codex-coverage-gap-2bh4a`, based on `main` at `73f94568`.
- **Commits so far (local only — never pushed):**
  - `e3ed01f0` fix(indexer): an aborted connector scan can no longer claim complete coverage
  - `e4eb5a5b` beads: claim the bead and record the failure-semantics decision
- **Beads DB is NOT in the worktree** (`.beads/*.db` is gitignored). Run br with an
  explicit db path: `br <cmd> --db /Users/dalecarman/dev/coding_agent_session_search/.beads/beads.db`.
  br auto-flushes to the shared checkout's `.beads/issues.jsonl`; copy it into the
  worktree (`cp /Users/dalecarman/dev/coding_agent_session_search/.beads/issues.jsonl .beads/issues.jsonl`)
  before committing it here. Verify the diff only touches this bead first.
- **Scratch dir used by the parent:** `/Users/dalecarman/.claude-accounts/katherine/jobs/268f9f88/tmp`
  (holds `commit-msg-1.txt`, `bead-comment-1.md`). It disappears with the parent job — do
  not depend on it.

## DONE

**Part 1, failure semantics — code complete, `cargo check --all-targets` clean.**

Chose a **durable per-connector coverage floor**, not holding the watermark. The full
rationale is already posted as a bead comment (`br show <bead>`, comment beginning
"FIX part 1 of 2"). One-line version: `last_scan_ts` is a single global `meta` row shared
by every connector AND it is advanced every commit interval *while the scan is still
running*, so by the time `IndexMessage::ScanError` arrives the advance is already
committed and only a global rollback could undo it — which would force every healthy
connector into the full rescan that caused the 2026-06-01 outage.

Files changed in `e3ed01f0`:
- `src/storage/sqlite.rs` — `CONNECTOR_SCAN_FLOORS_META_KEY` (JSON in the existing `meta`
  k/v table, no schema migration), `get_connector_scan_floors`,
  `record_connector_scan_floor` (lowering-only), `clear_connector_scan_floor`,
  `parse_connector_scan_floors`, `connector_scan_since_ts`.
- `src/indexer/mod.rs` — `ConnectorScanCoverage`, `read_connector_scan_floors_fresh`,
  `record_connector_scan_floor` / `clear_connector_scan_floor` helpers; per-connector
  `since_ts` in both the streaming and batch scan paths; record on `ScanError` and on the
  batch path's scan `Err` (which previously dropped the error entirely); clear on a clean
  discovered scan.
- `src/lib.rs` — `connector_coverage` in `cass stats --json`, `cass status`, `cass health`
  (+ human-readable output). `checked` and `complete` are deliberately distinct so a
  surface that never opened the DB can never read as clean. Incomplete coverage makes
  health `degraded`, not `healthy`.

Non-obvious thing already measured, do not re-derive: a floor committed through an
ephemeral writer **during** a scan is invisible to the long-lived handle that started
that scan (stale MVCC snapshot); a fresh open sees it immediately. Hence
`read_connector_scan_floors_fresh`. Without it the widening silently does not happen.

Tests written and **passing with the fix**:
- `src/indexer/mod.rs::aborted_connector_scan_does_not_leave_the_index_claiming_complete_coverage`
- `src/storage/sqlite.rs::connector_scan_floors_round_trip_and_clear`
- `src/storage/sqlite.rs::connector_scan_since_ts_lowers_to_the_floor`
- `src/storage/sqlite.rs::parse_connector_scan_floors_tolerates_junk`

Measured output: `cargo test --lib aborted_connector_scan` →
`test result: ok. 1 passed; 0 failed; ... finished in 1.48s`; the three storage tests
passed in the same run as `connector_scan`.

## PENDING

1. **Prove the regression test is red against pre-change HEAD.** Required by the goal
   ("fails against today's code") and by AGENTS.md's discovered-red-suite rule, which
   demands an isolated temporary detached clone *outside the repo* rather than a branch
   or worktree. The test's behavioural half uses only pre-existing API, so it ports:
   the `assert_eq!(coverage_fixture_conversation_count(&storage), 3, ...)` after pass 2
   is the assertion that must go red (pre-change it stays at 1, because pass 2's
   `since_ts` filters the two unread rollouts out by mtime). Drop the two floor
   assertions and the `read_connector_scan_floors_fresh` calls when porting — that API
   does not exist at `73f94568`. Report the real before/after output.
2. **Broader tests.** At minimum `cargo test --lib indexer::`, `cargo test --lib storage::`,
   and the CLI JSON surfaces that could be affected by the new `connector_coverage` key
   and by health flipping to `degraded`: `tests/cli_robot.rs`, `tests/cli_*`, `tests/e2e_*`.
   Expect possible fallout in health/status contract tests. Note: this repo has a known
   pre-existing red band from frankensqlite FTS5 differences (napkin, spec 011) — classify
   any failure as REPRODUCED / NOT REPRODUCED / UNAVAILABLE against `73f94568`, never
   silently "pre-existing".
3. **`cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`.** Not yet run.
   Use `"$HOME/.cargo/bin/cargo"` — Homebrew cargo drifts behind the toolchain here.
4. **Part 2, recovery.** Largely already exists in the product; verify rather than build
   (AGENTS.md §13 / global §13 — do not add a second mechanism). Established by reading
   the source this session:
   - `cass index --watch-once <path>[,<path>...]` is the path-scoped re-scan.
     `reindex_paths_with_semantic_delta` sets `since_ts = None` when
     `explicit_watch_once` is true (src/indexer/mod.rs, the `let since_ts = if force_full
     || explicit_watch_once { None }` line), so targeted paths **bypass the mtime
     watermark entirely** — exactly what the hole files need.
   - `explicit_watch_once_connector_hint` matches the `.codex/sessions` path pair, so
     day-directories under `~/.codex/sessions/YYYY/MM/DD/` classify as Codex.
   - `explicit_watch_once_root_unchanged_after_last_index` returns false for directories
     and for files absent from `conversations`, so hole files are never skipped.
   - A `targeted_watch_once_only_run` does **not** advance `last_scan_ts`
     (`targeted_watch_once_only_run` branch before `persist_final_index_run_metadata`).
   What is left: build a **copy** of a slice of the archive (copy the DB and a bounded set
   of codex rollouts into a temp data dir — the live archive stays read-only and
   untouched), run the targeted command against the copy, confirm the previously-absent
   sessions land, and **measure** wall time and bytes written. Then report the exact
   command you would run against the live archive and the extrapolated estimate for the
   3,186-file tail. Do **not** run it against the live archive.
   Watch for: argv length if paths are passed per-file (3,186 paths will not fit) — prefer
   the ~110 day-directories. Also confirm whether one invocation can take many paths or
   whether it should be batched.
5. **Land it.** `br sync --flush-only`, commit the code and the tracked
   `.beads/issues.jsonl`, and push. **The parent's `git push` was denied by the permission
   classifier** — if it is denied again, do not work around it: say so plainly, leave the
   commits on the branch, and tell Dale the exact command to run. Note the repo works on
   `main` (AGENTS.md and global §2.10); this work is on a worktree branch only because
   background-session isolation required it, so landing on `main` is Dale's call.

## Constraints that still bind

- **Delete nothing.** AGENTS.md RULE 1 — no file deletion without written permission, not
  even files you created. No `git reset --hard`, `git clean -fd`, `rm -rf`.
- **Live archive is read-only.** `~/Library/Application Support/com.coding-agent-search.coding-agent-search/agent_search.db`
  must stay byte-identical at its 2026-08-04 mtime. Open with `file:...?mode=ro`.
- **No full rebuild. No mass reindex against the live archive.**
- **Do not upgrade br.**
- `cass status --json` hangs on the live archive (bead `status-json-hang-nvq59`) — use
  `cass stats --json`.
- Scope is this bead only. `codex-flat-layout-undiscovered-kfaid` and
  `pi-agent-missing-workspaces-le8s1` are separate. Tempting datum, recorded and **not
  acted on**: the FAD crate is now checked out at
  `~/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/b62d859/`, so -kfaid's
  previously-unprovable discovery hypothesis is now readable at source level. Leave it.
- No `rusqlite` in new code — frankensqlite only.
- Never `git add -A`; stage by name and bound the commit with `git commit -- <paths>`.

## Exact next action

From `/Users/dalecarman/dev/coding_agent_session_search/.claude/worktrees/codex-coverage-gap-2bh4a`,
run the two verification commands that are cheap and unblock everything else:

```bash
"$HOME/.cargo/bin/cargo" clippy --all-targets -- -D warnings
"$HOME/.cargo/bin/cargo" fmt --check
```

Then pending item 1 (the measured red against `73f94568` in a temporary detached clone
outside the repo), then item 2, then item 4.
