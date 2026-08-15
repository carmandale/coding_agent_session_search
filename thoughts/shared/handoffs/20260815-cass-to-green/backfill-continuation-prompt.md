---
generation: 2
parent-session: c6bfb589-e0c3-4bb9-97b4-04c75f2a043d
next-action-class: executable
---

# Continuation — the coverage fix is written and committed; the backfill is still running

## The goal and authorization, verbatim (Dale, 2026-08-14)

> /my-way fix cass to completion and 100% green working state and completely up to date or tell me why it can't or /grill-me with any questions.

Sent mid-work the same day, as a correction to the work in flight:

> make sure that you are looking at the recent (last 2 weeks) work on cass and not regressing

And on 2026-08-15, the question that produced the escape-hatch breakthrough:

> should we make a local fork of frankensqlite and fix it?

**Destructive and external-write approvals expired with the parent session and do
not transfer.** You do NOT have approval to: delete any file (this repo's
AGENTS.md RULE 1 forbids it outright, including files you create), force-push,
rewrite history, merge to `main`, file anything on a public third-party repo, or
run `cass sources agents exclude`.

## WHERE THE WORK LIVES — read this before anything else

This generation ran as a background job, whose harness rejects edits in the
shared checkout. **The work is on a branch in a worktree, not on `main`:**

- worktree: `.claude/worktrees/cass-to-green-c6bfb589`
- branch: `worktree-cass-to-green-c6bfb589`
- commits: `8dcd245b` (the fix), plus the evidence commit after it

`main` is untouched at `74a72233`. Landing to `main` is Dale's call — the repo's
AGENTS.md says all work happens on `main` and never on branches, while the
background-job harness forbids pushing `main` and required the worktree. Those
two rules conflict and only Dale can settle it. Do not merge unilaterally.

Note: `br` does NOT work inside the worktree — `.beads/beads.db` is gitignored so
only the tracked JSONL is there. Run every `br` command from
`/Users/dalecarman/dev/coding_agent_session_search`.

## State of the running backfill — DO NOT START A SECOND ONE

Still alive, `nohup caffeinate -is bash .../backfill.sh`, relaunched
2026-08-15T10:53:41Z. At handoff: **7 of 20 batches, 14,430 conversations**
(from 12,722 before the escape hatch), disk 112 GB free. Roughly 4-5 min per
early batch; later batches hold larger files and the single 2.57 GB rollout is
deliberately last, so expect it to slow down.

```bash
SCRATCH=/private/tmp/claude-501/-Users-dalecarman--agent-config/a91c2501-1830-4d3d-9430-3c9afe08a63c/scratchpad
grep -c '^=== batch-.* END' "$SCRATCH/backfill/run.log"      # of 20
sqlite3 "file:$HOME/Library/Application Support/com.coding-agent-search.coding-agent-search/agent_search.db?mode=ro" \
  'SELECT count(*) FROM conversations;'
pgrep -f 'scratchpad/backfill.sh' || echo "DIED — read run.log"
```

Batches are idempotent; if it died, re-run `backfill.sh` and it resumes.

## The exact next action

**Deploy the fix and re-measure the four surfaces that hang.** The fix is written
and unit-tested but has never run against the live archive, which is the only
place the bug reproduces.

1. Wait for the backfill to reach 20/20 (or confirm it is safe to build alongside
   — building is CPU-only, but do NOT overwrite the live binary while it runs).
2. Build a release binary from the branch:
   ```bash
   export PATH="$HOME/.rustup/toolchains/nightly-aarch64-apple-darwin/bin:$PATH"
   cd /Users/dalecarman/dev/coding_agent_session_search/.claude/worktrees/cass-to-green-c6bfb589
   CARGO_TARGET_DIR=/tmp/cass-c6bfb589-target cargo build --release
   ```
3. Deploy with an **atomic rename, never `cp` over the live path** — bead
   `-1a7mk` records that overwriting in place gives SIGKILL from a stale
   signature cache even though `codesign` reports the bytes valid. Preserve the
   current binary under a dated name first (do not delete it).
4. Re-measure. This is the acceptance test for `-1a7mk` and it must be run with a
   real timeout, because `gtimeout`/`timeout` are NOT installed on this machine:
   ```bash
   python3 - <<'PY'
   import subprocess, time
   BIN='/Users/dalecarman/.local/bin/cass'
   for args in (['status','--json'],['stats'],['health'],['triage']):
       t0=time.time()
       try:
           p=subprocess.run([BIN]+args,capture_output=True,text=True,timeout=150)
           print(f"{' '.join(args):16s} EXIT={p.returncode} in {time.time()-t0:.1f}s", flush=True)
       except subprocess.TimeoutExpired:
           print(f"{' '.join(args):16s} HANG", flush=True)
   PY
   ```
   Pre-fix baseline, measured 2026-08-15T11:05Z on the live archive: **all four
   HANG at 150s.** Anything that returns is the fix working. Print with
   `flush=True` — the parent lost ten minutes to block-buffered output.

## What is DONE

- **`-1a7mk` fixed in code** (`8dcd245b`). Two defects in one 30-line window:
  the declared 2s bound covered only the open (and only as a `PRAGMA
  busy_timeout`, a lock bound not a work bound) while `close_in_place()` on a
  7.7 GB archive was unbounded; and `.unwrap_or_default()` mapped a *failed*
  coverage read to an empty map, which `connector_coverage_json` renders as
  `"complete": true`. `cargo check --all-targets` exit 0, zero warnings; four new
  tests green; **the mutant restoring `.unwrap_or_default()` turns
  `failed_coverage_read_is_unknown_and_never_complete` red**, so the assertion is
  load-bearing.
- **`-qtn0e` answered.** `cass index --force-rebuild` does NOT delete
  conversation rows. Four `DELETE FROM conversations` in the tree, three
  `#[cfg(test)]`, the one reachable is the agent purge. No dynamic
  `DELETE FROM {`. Whole-archive replacement via `promote_staged_historical_seed`
  is double-gated on a zero-conversation archive. Live `meta.schema_version` = 20
  = `CURRENT_SCHEMA_VERSION`, so no migration runs. **`cass sources agents
  exclude claude_code` IS the destruction path** — 4,050 rows, 3,877 irreplaceable.
- **The four frankensqlite claims verified** (survived adversarial review). No
  fork. Upstream fixed the engine defect in 0.1.11; cass pins 0.1.5. Keep
  `parity_cert` ON. Footnote: five published releases between 0.1.5 and 0.1.11,
  not six.
- **`-a4xe1` fully diagnosed, not yet applied** (see below).
- Escape-hatch premise re-verified with positive controls; `cass search` works.

## What is PENDING, in priority order

1. **Deploy + re-measure** — the exact next action above.
2. **Apply the golden fix for `-a4xe1`.** Fully worked out in
   `lanes/golden-robot-json.md` §7a, and the `.actual` files are already written
   by the test run. All nine failures are stale goldens or macOS host drift;
   **none is a code regression.** Do NOT run `UPDATE_GOLDENS=1` on macOS — it
   bakes macOS topology into a contract CI checks on Linux. Instead:
   - replace 4 wholesale from `.actual` (diff is purely the added
     `connector_coverage` block, trailing-byte parity checked): `health.json`,
     `health_shape.json`, `stats_full_payload.json`, `stats_full_payload_shape.json`
   - take **only** the `connector_coverage` hunk for 3: `status_shape.json`,
     `status_quarantine.json`, `status_quarantine_full.json`
   - do **not** touch `diag.json`, `diag_quarantine.json`
   Regenerate `.actual` first (`cargo test --test golden_robot_json`) — the fix in
   `8dcd245b` changes `run_stats` to the honest `connector_coverage_state_json`,
   so the stats goldens must be taken post-fix, not from the lane's pre-fix run.
3. **Fix the THIRD copy of the coverage read.** The adversarial verifier found
   what the lane and the fix both missed: `read_connector_scan_floors_fresh` at
   `src/indexer/mod.rs:10696-10721` performs the same read a third time and
   swallows failure identically (`BTreeMap::new()` at 10709 and 10712-10718),
   consumed at 11568 and 11737. The committed fix is scoped to `src/lib.rs` only.
4. **`-gxw32`** — the per-connector test. `lanes/coverage-floor-test.md` §5 names
   the location and the exact mutant at `src/indexer/mod.rs:10657`.
5. **Catch-up index** after the backfill, for the ~7,796 stale claude + ~2,342
   stale codex files:
   ```bash
   CASS_SKIP_PREFLIGHT_CLEANUP_ORPHAN_FK_ROWS=1 cass index --json --progress-interval-ms 60000
   ```
6. **Prove coverage by counting**, never by reading `connector_coverage.complete`.
   That field is structurally incapable of reporting this hole: `meta` holds only
   `last_indexed_at`, `last_scan_ts`, `schema_version` — there is no
   `connector_scan_floors` row at all, so an empty map renders `complete: true`
   over an archive missing thousands of conversations.

## Two things only Dale can decide — ask, do not act

1. **68 GB of reclaimable disk.** `/tmp` holds 81.69 GiB of cass cargo target
   dirs from prior agent sessions (largest `/tmp/cass-nvq59-target`, 32.15 GiB).
   Excluding this chain's active `/tmp/cass-c6bfb589-target` (13.21 GiB), ~68 GB
   is pure build cache whose loss costs one rebuild — and reclaiming it puts the
   machine above its 150 GB floor for the first time in days. RULE 1 forbids
   deleting it without his express written permission.
2. **Landing the branch to `main`**, per the rule conflict above.

## Do not repeat these

- `gtimeout`/`timeout` are not installed. Use the python `subprocess.timeout`
  form above, with `flush=True`.
- `--json` sets robot mode, which hard-codes the log filter to `error` and
  IGNORES `RUST_LOG` (`src/lib.rs:5769-5775`). Add `--verbose`.
- Never share a `CARGO_TARGET_DIR` between checkouts of this crate.
- `ps -o etime` is `MM:SS` at short durations. This session briefly read `03:57`
  as four hours and nearly condemned a healthy backfill.
- Do not trust `cass --version`'s git sha alone — bead `ff3d7125` files a known
  vergen gap. Identify a binary by sha256 against the preserved dated copies.

## Evidence

`thoughts/shared/handoffs/20260815-cass-to-green/` on branch
`worktree-cass-to-green-c6bfb589` — `agent-log.md` (coordinator, carries the
deployment correction and the measured tables), six grounding lanes, five
adversarial verifier logs. Prior generation:
`thoughts/shared/handoffs/20260814-cass-repair-to-green/`. Backup:
`~/backups/cass/agent_search-20260814-vacuum.db`, 3.98 GB, verified.
