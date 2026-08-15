---
generation: 3
parent-session: 29dd053b-e4a3-4e71-89d6-a599d8c5e157
next-action-class: executable
---

# Continuation — the code is done and committed; deploy, re-measure, then catch up

This supersedes the generation-2 version of this file, which is preserved at
commit `ec1ab2a7`. Generation 3's work is committed at `b0a631b4`.

## The goal and authorization, verbatim (Dale, 2026-08-14)

> /my-way fix cass to completion and 100% green working state and completely up to date or tell me why it can't or /grill-me with any questions.

Sent mid-work the same day, as a correction to the work in flight:

> make sure that you are looking at the recent (last 2 weeks) work on cass and not regressing

And on 2026-08-15, the question that produced the escape-hatch breakthrough:

> should we make a local fork of frankensqlite and fix it?

**Destructive and external-write approvals expired with the parent session and do
not transfer.** You do NOT have approval to: delete any file (including files you
create), force-push, rewrite history, merge to `main`, file anything on a public
third-party repo, or run `cass sources agents exclude`.

## WHERE THE WORK LIVES

- worktree: `.claude/worktrees/cass-to-green-c6bfb589`
- branch: `worktree-cass-to-green-c6bfb589`, pushed, HEAD `b0a631b4`
- `main` is untouched at `74a72233`. Landing is Dale's call — bead
  `coding_agent_session_search-land-cass-to-green-branch-t61zi` carries the rule
  conflict. Do not merge unilaterally.

`br` does NOT work inside the worktree (`.beads/beads.db` is gitignored). Run
every `br` command from `/Users/dalecarman/dev/coding_agent_session_search`.

## State of the running backfill — DO NOT START A SECOND ONE

Alive at handoff, `nohup caffeinate -is bash .../backfill.sh`, **12 of 20
batches, 15,905 conversations**. Batches are idempotent; if it died, re-run
`backfill.sh` and it resumes.

```bash
SCRATCH=/private/tmp/claude-501/-Users-dalecarman--agent-config/a91c2501-1830-4d3d-9430-3c9afe08a63c/scratchpad
grep -c '^=== batch-.* END' "$SCRATCH/backfill/run.log"      # of 20
sqlite3 -readonly "$HOME/Library/Application Support/com.coding-agent-search.coding-agent-search/agent_search.db" \
  'SELECT count(*) FROM conversations;'
pgrep -f 'scratchpad/backfill.sh' || echo "DIED — read run.log"
```

**Deploy is blocked on this finishing, and the reason is mechanical:**
`backfill.sh:38` re-invokes `"$BIN" index --watch-once` where
`BIN=/Users/dalecarman/.local/bin/cass`, once per batch inside the loop. An
atomic rename at that path mid-run changes the binary the *next* batch executes,
silently splitting one backfill across two binaries.

## The exact next action

**Wait for 20/20, then deploy and re-measure.** A release binary built from this
branch was started at 12:03:52Z into `/tmp/cass-c6bfb589-target`; check it
finished and rebuild if not:

```bash
export PATH="$HOME/.rustup/toolchains/nightly-aarch64-apple-darwin/bin:$PATH"
cd /Users/dalecarman/dev/coding_agent_session_search/.claude/worktrees/cass-to-green-c6bfb589
CARGO_TARGET_DIR=/tmp/cass-c6bfb589-target cargo build --release -j 6
```

Then, only after 20/20:

1. Preserve the current binary under a dated name — do NOT delete it:
   `cp ~/.local/bin/cass ~/.local/bin/cass.pre-1a7mk-fix-20260815`
2. Deploy by **atomic rename, never `cp` over the live path** — bead `-1a7mk`
   records SIGKILL from a stale signature cache even though `codesign` reports
   the bytes valid:
   ```bash
   cp /tmp/cass-c6bfb589-target/release/cass ~/.local/bin/cass.new
   mv -f ~/.local/bin/cass.new ~/.local/bin/cass
   ```
3. Re-measure. `gtimeout`/`timeout` are NOT installed; use this, and keep
   `flush=True` — an earlier generation lost ten minutes to block buffering:
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

Pre-fix baseline, measured 2026-08-15T11:05Z on the live archive: **all four HANG
at 150s.**

### A falsifiable prediction to test against that measurement

Generation 3 predicts, from source, that **`8dcd245b` fixes `cass health` alone**
and that `status`, `triage` and `stats` still hang. `read_connector_scan_floors_bounded`
— the function `8dcd245b` bounded — has exactly one production call site,
`src/lib.rs:65743`, reached through `health`. The other three reach the archive
through `probe_state_db` (`src/lib.rs:15312`), whose close is
`close_franken_cli_read_db`, and that function calls `conn.close_in_place()` with
**no wall-clock bound** — the exact expensive step `8dcd245b` identified.

If all four return, this prediction is wrong; say so plainly. If three still hang,
the remaining fix is to bound `probe_state_db`'s close the way `8dcd245b` bounded
the coverage read. **Do not copy the worker-thread shape into the indexer** —
see the "do not repeat" list.

## What generation 3 DID (all committed at `b0a631b4`, pushed)

- **`-1a7mk` third copy fixed.** `read_connector_scan_floors_fresh`
  (`src/indexer/mod.rs`) is now `Option<BTreeMap<String, i64>>`, both failure arms
  log at `error!`. Three copies of one read finally agree. **It does not close the
  hole** — callers still substitute an empty map and run unwidened, deliberately.
- **Deliberately NOT bounded with a worker thread**, overturning the grounding
  lane on the adversarial verifier's evidence. See "do not repeat".
- **`-gxw32` test written and proven.** `each_connector_scans_from_its_own_coverage_floor`.
  Mutant `floors.get(name).copied()` → `floors.values().copied().min()` at
  `src/indexer/mod.rs:10657` turns it RED (`left: Some(100) right: Some(400)`).
- **Two more tests**, one of which closes a real gap: before it, *neither* failure
  arm of that function was exercised anywhere in the tree. Mutant restoring
  `Some(BTreeMap::new())` turns it RED (`left: Some({}) right: None`).
- **A vacuous assertion killed** — the existing
  `read_connector_scan_floors_fresh(&db_path).is_empty()` passed identically
  whether the floor was cleared or the read failed.
- `cargo check --all-targets` exit 0, zero warnings, and the log confirms it
  recompiled `coding-agent-search` rather than reporting a cached fingerprint.
- **Eight beads filed** from six lanes' findings (see below).
- **The catch-up runbook written**: `catchup-manifest.py` + `catchup-run.sh` in
  this directory, both validated by a dry run.

## What is PENDING, in priority order

1. **Deploy + re-measure** — the exact next action above.
2. **`-a4xe1` golden repair.** Lane `gen3-golden-diff.md` confirmed all nine of
   the previous generation's classifications with per-file diffs and trailing-byte
   parity. Regenerate `.actual` first
   (`CARGO_TARGET_DIR=/tmp/cass-gen3-golden-target cargo test --test golden_robot_json`),
   then:
   - replace wholesale from `.actual` (4, all verified to contain zero
     macOS-host bytes): `health.json`, `health_shape.json`,
     `stats_full_payload.json`, `stats_full_payload_shape.json`
   - take **only** the `connector_coverage` hunk for 3: `status_shape.json`,
     `status_quarantine.json`, `status_quarantine_full.json`
   - do **not** touch `diag.json`, `diag_quarantine.json`
   - **never** run `UPDATE_GOLDENS=1` on macOS — it bakes macOS topology into a
     contract CI checks on Linux.
3. **The catch-up index — much larger than the previous generation thought.**
   Measured 2026-08-15T11:42Z by set-diff of disk against
   `conversations.source_path`: **12,210 unindexed transcripts, 25.56 GiB**
   (`claude_code` 7,311 and `codex` 4,899). The running backfill covers **zero**
   Claude Code files. Run, only after the backfill is done and the binary deployed:
   ```bash
   thoughts/shared/handoffs/20260815-cass-to-green/catchup-run.sh
   ```
   It refuses to start while `backfill.sh` is alive, rebuilds the manifest itself,
   and batches 250 at a time. Expect several hours.
4. **Prove coverage by counting**, never by reading `connector_coverage.complete`.
   The archive has **no `connector_scan_floors` meta row at all** — `meta` holds
   only `last_indexed_at`, `last_scan_ts`, `schema_version` — so an empty map
   renders `"complete": true` over an archive missing thousands of conversations.
   `catchup-manifest.py` is the honest instrument.

## Two things only Dale can decide — filed as beads, do not act

- `coding_agent_session_search-reclaim-tmp-cargo-targets-jck92` — 61 GB of stale
  `/tmp` cass target dirs. **Disk is now 89 GB and falling** (the release build
  took ~20 GB); the floor is 150 GB. Re-measure before acting.
- `coding_agent_session_search-land-cass-to-green-branch-t61zi` — landing this
  branch to `main`.

## Six more beads filed from the lanes

- `doctor-promote-gate-fails-open-sgvg3` (**P0**) — an unreadable archive gives
  the doctor promotion coverage gate a baseline of 0, so `promote_allowed` comes
  back `true`. Same tri-state collapse as `1a7mk`, on the archive that is the only
  surviving copy of 3,877 conversations.
- `probe-state-db-sibling-swallows-0gzok` (P1) — an unreadable `last_scan_ts`
  reads as "not stale"; a failed `COUNT` reads as a definite 0 with
  `counts_skipped: false`.
- `golden-robot-json-host-drift-tutfy` (P1) — five goldens bake Linux host values,
  so the suite is permanently red on macOS. **This is why `a4xe1` sat unnoticed:**
  the suite went 5 red → 9 red and nobody could see it.
- `parse-floors-unparseable-reads-compl-ddkwa`, `quarantine-unreadable-undercounts-a59ou`,
  `health-healthy-on-unknown-coverage-xarzt` (P2).

## Do not repeat these

- **Do not add a bounded worker thread to the indexer.** The adversarial verifier
  established that an orphaned worker holds `runtime_state` across a
  `std::hint::spin_loop()` in `region.rs`, and `register_connection` needs that
  same mutex — so the next ephemeral-writer acquisition blocks forever, with no
  timeout and no log, far from the cause. Fine in a short-lived CLI, not in a
  long-lived indexer. Full reasoning in `lanes/gen3-verify-third-copy.md`.
- **`cargo check` can report a cached fingerprint.** A check that prints
  `Finished` without a `Checking coding-agent-search` line has verified nothing
  about your edits. Grep the log for that line before believing a green result.
- **Never share a `CARGO_TARGET_DIR` between checkouts of this crate.**
- `--json` sets robot mode, which hard-codes the log filter to `error` and
  IGNORES `RUST_LOG` (`src/lib.rs:5769-5775`). Add `--verbose`.
- Do not trust `cass --version`'s git sha alone — bead `ff3d7125` files a known
  vergen gap. Identify a binary by sha256.
- `backfill.sh`'s own header comment claiming it "runs on the installed PRE-FIX
  binary" is **false**; its `run.log:4` records the post-fix sha `5b3344fd`. The
  `-1a7mk` regression is live on this machine right now, and deploying is what
  removes it.
- `ps -o etime` is `MM:SS` at short durations.
- `/usr/bin/trash` is not a way around the no-delete rule.

## Evidence

`thoughts/shared/handoffs/20260815-cass-to-green/` — `agent-log-gen3.md`
(coordinator, carries the measured tables and the ordering rationale), six
generation-3 lane logs (`gen3-*.md`), plus the generation-2 lanes. Prior
generation: `thoughts/shared/handoffs/20260814-cass-repair-to-green/`. Backup:
`~/backups/cass/agent_search-20260814-vacuum.db`, 3.98 GB, verified.
