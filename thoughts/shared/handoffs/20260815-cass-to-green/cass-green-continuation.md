---
generation: 1
parent-session: a91c2501-1830-4d3d-9430-3c9afe08a63c
next-action-class: executable
---

# Continuation — cass is green and deployed; the catch-up is the only long pole left

## The goal and authorization, verbatim

Dale, 2026-08-14:

> /my-way fix cass to completion and 100% green working state and completely up to date or tell me why it can't or /grill-me with any questions.

Sent mid-work the same day, as a correction to the work in flight:

> make sure that you are looking at the recent (last 2 weeks) work on cass and not regressing

Dale, 2026-08-15:

> should we make a local fork of frankensqlite and fix it?

And the standing instruction for this session, Dale, 2026-08-15:

> your usage is good now. finish this to completion

**Destructive and external-write approvals expired with the parent session and do
not transfer.** You do NOT have approval to: delete any file (this repo's AGENTS.md
RULE 1 forbids it outright, including files you created yourself), force-push,
rewrite history, change repo visibility, file anything on a public third-party
repo, or run `cass sources agents exclude` — that last one would destroy 3,877
conversations that exist nowhere else on Earth.

## THE HEADLINE

**Both test suites are green, the coverage fix is built and deployed and measured,
six beads are closed, and everything is pushed to `main` and `master`.** The only
thing still running is the catch-up indexer, which is mechanical and unattended.

| | state |
|---|---|
| `cargo test --lib` | **5,131 passed, 0 failed** (rc=0) at 82f316a7 |
| `cargo test --test golden_robot_json` | **37 passed, 0 failed** — was 9 red |
| deployed binary | `db29a26eafbfb3091a2f3c34`, built `--release` from `main` |
| `cass health --json` | **3.05s**, was no-return at 75s |
| conversations indexed | 12,722 → **18,796** and climbing |
| beads closed | a4xe1, tutfy, 1a7mk, c7yaw, gxw32, t61zi |

`main` is at `e9327ef2`, pushed to both `main` and `master`.

## What is running RIGHT NOW — do not start a second one

The generation-3 catch-up: 9,823 files / 14.03 GiB across `claude_code` and
`codex`, in 40 batches of 250 via path-scoped `--watch-once`.

- Script: `thoughts/shared/handoffs/20260815-cass-to-green/catchup-run.sh` (tracked)
- Work dir, log, manifest, batches: `~/.cass-catchup/`
- Disk guard: `<scratch>/catchup-guard.sh`, stops it between batches below 25 GB free
- Progress at handoff: **5 of 40 batches**, 18,796 conversations, 53 GB free
- Rate: roughly 10.5 min per batch, so **about 6 hours remaining**

```bash
rg -c 'END rc=' ~/.cass-catchup/run.log            # batches finished, of 40
tail -1 ~/.cass-catchup/guard.log                  # disk guard's last reading
pgrep -f catchup-run.sh >/dev/null && echo ALIVE || echo STOPPED
sqlite3 "file:$HOME/Library/Application Support/com.coding-agent-search.coding-agent-search/agent_search.db?mode=ro" \
  'SELECT count(*) FROM conversations;'
```

If it died, re-run `catchup-run.sh` — batches are idempotent and resumable via
`explicit_watch_once_root_unchanged_after_last_index`, so completed ones are
no-ops. If the guard stopped it for disk, **do not free space by deleting
anything**; bead `-jck92` records 61 GB of stale cargo targets awaiting Dale's
approval, and that approval is his to give.

## The exact next action

1. **Watch the catch-up to completion** (~6h). Then re-run the manifest builder to
   confirm the hole is closed — this is the acceptance test:
   ```bash
   python3 thoughts/shared/handoffs/20260815-cass-to-green/catchup-manifest.py /tmp/verify-manifest.txt
   ```
   It prints `on_disk`, `indexed` and `unindexed` per connector. **Unindexed should
   reach 0 for both.** It will NOT: `claude_code` shows `indexed` far above the
   number of files still on disk because 3,877 source files are gone. That is
   expected and is bead `-qtn0e`, not a failure.

2. **Count sessions, do not trust `connector_coverage.complete`.** The coverage
   floor is forward-looking and the archive has no `connector_scan_floors` meta
   row, so the field is structurally incapable of reporting this hole. The
   set-diff above is the honest signal.

3. **Then close `-2bh4a`** (the codex coverage gap) with the final counts.

## What was done this session, and where the proof is

**Landed the stranded chain.** Generations 2-4 had done excellent work on
`worktree-cass-to-green-c6bfb589` and could not push `main` — their background
harness forbids it. Merged at `82f316a7` from the shared checkout, which is what
bead `-t61zi` was asking about. That branch was a harness artifact, not a chosen
workflow.

**Fixed and PROVED the coverage hang (`-1a7mk`).** The bound now covers the whole
read — open, query and close on a worker thread with one `recv_timeout` — instead
of the open alone, where it was only a `PRAGMA busy_timeout`. A failed read no
longer renders as `"complete": true`. Measured against the preserved pre-fix
binary in the same run under the same load:

```
                 OLD 5b3344fd     NEW db29a26e
api-version      1.03s rc=0       (control, proves the instrument is alive)
health --json    TIMEOUT 75.78s   3.05s rc=1
triage --json    TIMEOUT 75.85s   TIMEOUT 75.84s
```

**Both detectors are mutation-proven, not merely present.** Re-ran the exact
mutants the beads describe: the `gxw32` mutant (per-connector floor → global min)
is now killed by `each_connector_scans_from_its_own_coverage_floor`, and restoring
`.unwrap_or_default()` is killed by
`failed_coverage_read_is_unknown_and_never_complete`. Both previously passed all
5,127 tests. Mutants reverted; tree byte-clean.

**Answered `-qtn0e` by census.** `--force-rebuild` cannot delete a conversation
row. Four `DELETE FROM conversations` exist in `src/`, three are `#[cfg(test)]`,
and the one reachable statement is behind `cass sources agents exclude`. Also
checked for runtime-built SQL (none) and both `DROP TABLE conversations` sites
(a copy-first migration, and test code). The bead stays OPEN because the hazard
it names is real and unchanged.

**Goldens green.** `-a4xe1` by per-file classification rather than regeneration
(`45d93234`), `-tutfy` by folding host-derived blocks with sibling-key context and
deriving `status_shape` from the normalized payload (`d853ee50`).

## Open, with what is known

- **`-nao4q` (NEW, P1)** — `cass triage --json` still never returns. It hangs
  identically on BOTH binaries, so it is not a regression from the coverage fix.
  Its cost is a full b-tree descent from `probe_state_db(.., include_counts=true)`
  at `src/lib.rs:16720`, whose 30s bounds only the open — the same defect class as
  `1a7mk` on a different surface. The fix pattern is already in the file. This is
  the highest-value remaining code fix.
- **`-qtn0e`** — answered, hazard stands.
- **`-2bh4a`** — close it when the catch-up finishes and the counts prove it.
- **`-jck92`** — 61 GB of stale cargo targets in `/tmp`. Needs Dale's deletion
  approval. Do not act on it unilaterally.
- The mini as a source (`cass sources list` is `total: 0`) and scheduling/freshness
  are Dale's decisions, not bugs to fix.
- The Linux CI leg for the goldens is **reasoned, not observed** — no Linux host
  was available. Watch those five cases on the next CI run.

## Environment facts that cost real time

1. The build needs nightly, and an absolute path to nightly cargo is NOT enough:
   ```bash
   export PATH="$HOME/.rustup/toolchains/nightly-aarch64-apple-darwin/bin:$PATH"
   export CARGO_TARGET_DIR=/tmp/cass-repair-target      # debug/test, warm
   export CARGO_TARGET_DIR=/tmp/cass-c6bfb589-target    # release, warm
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
5. There is no `timeout`/`gtimeout` on this machine. Bound a command with
   background + poll + kill.
6. Indexing requires `CASS_SKIP_PREFLIGHT_CLEANUP_ORPHAN_FK_ROWS=1`, and it is
   free here — all four child tables have zero orphan rows. Without it every entry
   point wedges in `phase="preparing"`.
7. **A plain `cass index` is the wrong tool AND wedges.** It skips files older than
   the watermark and then advances the watermark past them, closing the door
   permanently. Path-scoped `--watch-once` is what the catch-up uses and why.

## Evidence

`thoughts/shared/handoffs/20260815-cass-to-green/` — coordinator logs for
generations 2-4 and seventeen lane logs, including `rebuild-safety.md` (the census
behind the `qtn0e` answer) and `golden-robot-json.md` (the per-file golden
classification). Backup: `~/backups/cass/agent_search-20260814-vacuum.db`, 3.98 GB,
verified.

`<scratch>` = `/private/tmp/claude-501/-Users-dalecarman--agent-config/a91c2501-1830-4d3d-9430-3c9afe08a63c/scratchpad`.
macOS reaps `/private/tmp` after a few days; everything load-bearing is committed.
