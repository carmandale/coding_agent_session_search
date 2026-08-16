---
generation: 5
parent-session: a91c2501-1830-4d3d-9430-3c9afe08a63c
next-action-class: executable
---

# Continuation — the code work is done and landed; the catch-up and the disk are what remain

## The goal and authorization, verbatim

Dale, 2026-08-14:

> /my-way fix cass to completion and 100% green working state and completely up to date or tell me why it can't or /grill-me with any questions.

Sent mid-work the same day, as a correction to the work in flight:

> make sure that you are looking at the recent (last 2 weeks) work on cass and not regressing

Dale, 2026-08-15:

> should we make a local fork of frankensqlite and fix it?

> your usage is good now. finish this to completion

Dale, 2026-08-16:

> give me the /tldr of what was wrong and where we stand today

**Destructive and external-write approvals expired with the parent session and do
not transfer.** You do NOT have approval to: delete any file (this repo's AGENTS.md
RULE 1 forbids it outright, including files you created yourself), force-push,
rewrite history, change repo visibility, file anything on the public
`Dicklesworthstone/frankensqlite`, or run `cass sources agents exclude` — that
last one would destroy 3,877 conversations that exist nowhere else on Earth.

**In particular: the 116 GiB of stale cargo targets in `/tmp` are NOT yours to
delete.** Bead `jck92` records them and the ask is with Dale. Three separate
agent chains have now been blocked by that disk and all three correctly declined
to act on it alone. Do not be the fourth that decides differently.

## THE HEADLINE

**Every code defect in this repair is fixed, verified and on `main`.** `main` is
at `f463ef57`, pushed to both `main` and `master`. What is left is one unattended
indexing run and one question for Dale.

| | state |
|---|---|
| `cargo test --lib` | **5,137 passed, 0 failed** |
| `cargo test --test golden_robot_json` | **37 passed, 0 failed** — was 9 red |
| `cass health --json` | **3.05s** — was no return at 75s |
| `cass triage --json` | **5s** — was no return at 75s |
| `cass status --json` | **6s** — was no return at 75s |
| conversations indexed | 12,722 → **21,443 and climbing** |
| deployed binary | `49fbba6e3789c252`, built from the tree now merged at `505f5bf2` |

## What is running RIGHT NOW — do not start a second one

The generation-3 catch-up, resumed 2026-08-16T09:14Z: **6,758 files in 28 batches**
of 250, via path-scoped `--watch-once`.

- Script: `thoughts/shared/handoffs/20260815-cass-to-green/catchup-run.sh` (tracked)
- Work dir, log, manifest, batches: `~/.cass-catchup/`
- Disk floor: `CASS_CATCHUP_FLOOR_GB=40`, deliberately **above** cass's own ~32 GiB
  requirement — the previous run set it to 25 and cass hit its own wall first
- Progress at handoff: **3 of 28**, 21,443 conversations, 52 GiB free
- Rate: roughly 10 minutes per batch, so **about 4 hours remaining**

```bash
grep -c 'END rc=' ~/.cass-catchup/run.log            # batches finished, of 28
grep 'catchup STOPPED\|catchup done' ~/.cass-catchup/run.log
pgrep -f catchup-run.sh >/dev/null && echo ALIVE || echo STOPPED
sqlite3 "file:$HOME/Library/Application Support/com.coding-agent-search.coding-agent-search/agent_search.db?mode=ro" \
  'SELECT count(*) FROM conversations;'
```

**A finished run and a stopped run are now distinguishable, and were not before.**
Exit 14 is cass refusing to *start* because free disk is below its requirement.
It is a precondition, not a property of the batch, so every remaining batch fails
the same way and fails instantly — on 2026-08-15 that burned 27 invocations in
two seconds and the loop wrote `=== catchup done ===` over a run that had indexed
nothing since batch 13 of 40. `37aea3f5` makes rc=14 fail fast with its own
marker. If you see that marker: nothing is corrupted, cass refused to start
rather than risk a partial commit, the error is retryable, completed batches
re-run as no-ops. Free space — **by asking Dale, not by deleting** — and re-run.

## The exact next action

1. **Watch the catch-up to completion** (~4h). Then run the acceptance test:
   ```bash
   python3 thoughts/shared/handoffs/20260815-cass-to-green/catchup-manifest.py /tmp/verify-manifest.txt
   ```
   It prints `on_disk`, `indexed` and `unindexed` per connector. **`unindexed`
   should reach 0 for both.** `claude_code` will still show `indexed` far above
   the files on disk, because 3,877 source files are gone. That is expected, is
   bead `qtn0e`, and is not a failure.

2. **Count sessions; do NOT trust `connector_coverage.complete`.** The coverage
   floor is forward-looking and the archive has no `connector_scan_floors` meta
   row, so that field is structurally incapable of reporting this hole. The
   set-diff above is the honest signal.

3. **Then close `2bh4a`** (currently `in_progress`) with the final counts.

## What was done, and where the proof is

**Landed two stranded chains.** Generations 2-4 could not push `main` — their
background harness forbids it — and neither could generation 5. Merged the first
at `82f316a7` (bead `t61zi`) and the second at `505f5bf2`. Both were harness
artifacts, not chosen workflows. **Expect this again**: a background session in
this repo cannot land its own work, so check for unmerged `worktree-*` branches
before assuming `main` is current.

**Fixed and PROVED the coverage hang (`1a7mk`).** The bound now covers the whole
read — open, query and close on a worker thread with one `recv_timeout` — instead
of the open alone, where SQLite had downgraded it to a `PRAGMA busy_timeout` that
bounds lock waits and not queries. Measured against the preserved pre-fix binary
in the same run under the same load: `health` 75.78s timeout → **3.05s**, with
`api-version` at 1.03s as a live positive control.

**Generation 5 fixed the same defect class on the remaining surfaces
(`nao4q`, `nvq59`, `0gzok`, `ddkwa`, `a59ou`, `sgvg3`).** `probe_state_db` now
runs on a worker with one `recv_timeout`, and on expiry reports the probe failed
with counts elided rather than returning `StateDbSnapshot::default()`, whose zero
counts alongside `counts_skipped: false` were the lie. Verified at that exact
tree in an isolated clone before merging (5,137 / 37 green), and measured on the
deployed binary: `triage` 75.85s timeout → **5s**, `status` → **6s**.

**Both detectors are mutation-proven, not merely present.** Re-ran the exact
mutants the beads describe: the `gxw32` mutant (per-connector floor → global min)
is killed by `each_connector_scans_from_its_own_coverage_floor`, and restoring
`.unwrap_or_default()` is killed by `failed_coverage_read_is_unknown_and_never_complete`.
Both previously passed all 5,127 tests. Mutants reverted; tree byte-clean.

**Answered `qtn0e` by census.** `--force-rebuild` cannot delete a conversation
row. Four `DELETE FROM conversations` exist in `src/`, three are `#[cfg(test)]`,
and the one reachable statement is behind `cass sources agents exclude`. Also
checked for runtime-built SQL (none) and both `DROP TABLE conversations` sites
(a copy-first migration, and test code). The bead stays OPEN because the hazard
it names is real and unchanged.

**Answered `p3kgr` — do NOT fork frankensqlite.** The engine defect behind the
whole honesty family has a name: fsqlite 0.1.5 cannot reload a populated
`WITHOUT ROWID` table into MemDatabase, so the coverage read failed and cass
rendered that failure as `"complete": true`. Upstream fixed it. Generation 6 ran
a controlled A/B on one specimen with both controls firing — 0.1.5 emits the
`not yet supported` WARN and reports `UNKNOWN`; 0.1.14 emits no WARN and reports
`complete`; identical row counts from both, so both read the data correctly and
the only thing that changed is whether the read finished.

**Goldens green.** `a4xe1` by per-file classification rather than regeneration
(`45d93234`), `tutfy` by folding host-derived blocks with sibling-key context and
deriving `status_shape` from the normalized payload (`d853ee50`). The Linux CI
leg is **reasoned, not observed** — no Linux host was available. Watch those five
cases on the next CI run.

## Open, with what is known

- **`p3kgr` — the frankensqlite pin bump, BLOCKED ON DISK.** Commit `cd1089a8`
  on `worktree-cass-gen5-honesty` moves fsqlite `0.1.5` → `=0.1.14`,
  fsqlite-types to `=0.1.14`, and asupersync `0.3.2` → `=0.3.4`, together. The
  `=` pins are load-bearing: from 0.1.15 on, fsqlite-types needs asupersync
  ≥0.3.5 → sysinfo 0.39 → rustc 1.95, and a caret range silently resolves to
  0.1.19 and fails with an unrelated-looking rustc error. It **builds** (binary
  `572ae86d`) and it has the A/B above. Its **test suite has never run**, and
  that needs a full dependency rebuild, which needs disk. This is the only piece
  of the repair that is not finished, and it is gated on Dale.
- **`jck92` — the disk.** 116 GiB of cargo targets in `/tmp` against ~52 GiB
  free, while cass refuses to index below ~32 GiB. Four are 85+ hours old and
  worth 25 GiB: `cass-il0e9-check-target`, `cass-il0e9-test-target`,
  `cass-ubs-drift-test-target`, `cass-il0e9-release-target`. **Dale's call.**
- **`8llb5` (NEW, P1)** — `cass status` reports `"stalled"` for the whole of a
  healthy `--watch-once` run and `triage` advises restarting the watcher.
  Measured 2026-08-16: the staleness counter climbs +35s per 35s of wall clock
  and resets each batch, while that process completes rc=0. The `--watch-once`
  path holds the lock and refreshes its heartbeat but never posts forward
  progress. Honesty family inverted — bad news from a healthy run instead of
  good news from a failed read. Not bisected; likely long-standing.
- **`qtn0e`** — answered, hazard stands.
- **`2bh4a`** — `in_progress`; close it when the catch-up finishes and the
  counts prove it.
- **`b6xc3`** — doctor states a failed archive query as measured fact. Same
  family, filed by generation 5.
- The mini as a source (`cass sources list` is `total: 0`) and scheduling /
  freshness are Dale's decisions, not bugs to fix.

## Environment facts that cost real time

1. The build needs nightly, and an absolute path to nightly cargo is NOT enough:
   ```bash
   export PATH="$HOME/.rustup/toolchains/nightly-aarch64-apple-darwin/bin:$PATH"
   export CARGO_TARGET_DIR=/tmp/cass-repair-target      # warm; debug + test
   ```
2. **`cargo test --lib` depends on the INSTALLED binary.** `src/sources/probe.rs`'s
   `real_probe_*` tests shell out to `cass health --json`. While health hung, the
   suite hung — eleven child processes blocked at 0% CPU for twelve minutes,
   looking exactly like a compiler problem. Deploying the fix cleared it. If the
   suite ever hangs there again, measure the installed binary first.
3. `--json` sets robot mode, which hard-codes the log filter to `error` and
   **ignores `RUST_LOG`** (`src/lib.rs:5769-5775`). Add `--verbose` or you will
   conclude an instrument is silent when it is only suppressed.
4. Deploy by **atomic rename**, never `cp` over the live path — overwriting in
   place gives SIGKILL from a stale signature cache. Preserved binaries live
   beside it as `~/.local/bin/cass.*`; nothing has been deleted.
5. There is no `timeout`/`gtimeout` on this machine. `<scratch>/bound.sh` wraps
   background + poll + kill and prints `rc=N in Ns`.
6. Indexing requires `CASS_SKIP_PREFLIGHT_CLEANUP_ORPHAN_FK_ROWS=1`, and it is
   free here — all four child tables have zero orphan rows. Without it every entry
   point wedges in `phase="preparing"`.
7. **A plain `cass index` is the wrong tool AND wedges.** It skips files older than
   the watermark and then advances the watermark past them, closing the door
   permanently. Path-scoped `--watch-once` is what the catch-up uses and why.
8. **To test a branch without disturbing `main` or a running indexer, clone
   locally.** `git clone --local` hardlinks objects, so a 3 GB repo costs ~0 GiB,
   and pointing `CARGO_TARGET_DIR` at the warm target made a full lib+golden run
   cost 2 GiB and four minutes. That is how `505f5bf2` was verified before it
   landed, and it is much cheaper than it looks.

## Evidence

`thoughts/shared/handoffs/20260815-cass-to-green/` — coordinator logs for
generations 2-6 and their lane logs, including `rebuild-safety.md` (the census
behind the `qtn0e` answer), `golden-robot-json.md` (the per-file golden
classification), `lanes/gen5-frankensqlite-fork-answer.md` and
`agent-log-gen6-pin-bump.md` (the A/B that answered `p3kgr`).
Backup: `~/backups/cass/agent_search-20260814-vacuum.db`, 3.98 GB, verified.

`<scratch>` = `/private/tmp/claude-501/-Users-dalecarman--agent-config/a91c2501-1830-4d3d-9430-3c9afe08a63c/scratchpad`.
macOS reaps `/private/tmp` after a few days; everything load-bearing is committed.
