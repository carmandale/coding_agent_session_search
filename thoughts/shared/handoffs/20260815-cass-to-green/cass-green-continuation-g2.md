---
generation: 2
parent-session: 036c5f98-d2cb-4747-b689-cd4bfd68fa92
next-action-class: executable
---

# Continuation — four honesty fixes are green and pushed; the next move is build, deploy, measure

## The goal and authorization, verbatim

Dale, 2026-08-14:

> /my-way fix cass to completion and 100% green working state and completely up to date or tell me why it can't or /grill-me with any questions.

Sent mid-work the same day, as a correction to the work in flight:

> make sure that you are looking at the recent (last 2 weeks) work on cass and not regressing

Dale, 2026-08-15:

> should we make a local fork of frankensqlite and fix it?

And the standing instruction, Dale, 2026-08-15:

> your usage is good now. finish this to completion

**Destructive and external-write approvals expired with the parent session and do
not transfer.** You do NOT have approval to: delete any file (this repo's
`AGENTS.md` RULE 1 forbids it outright, including files you created yourself),
force-push, rewrite history, change repo visibility, file anything on a public
third-party repo, or run `cass sources agents exclude` — that last one would
destroy 3,877 conversations that exist nowhere else on Earth.

## THE HEADLINE

Four defects in one family are fixed, tested, mutation-proven, and pushed.
`cargo test --lib` is **5,134 passed, 0 failed** (5,131 baseline + 3 new tests).

Two things the parent session got WRONG and measured its way out of — read these
before trusting any earlier note:

1. **`cass status --json` is NOT fixed by deploying.** A survey lane concluded
   the `nvq59` fix had already landed at `447d97fe` and only the stale binary
   was at fault. `git merge-base --is-ancestor 447d97fe 82f316a7` is true, so the
   gate IS in the deployed binary — and a bounded run measured
   `rc=TIMEOUT 99.26s bytes=0` anyway. The lane's fallback theory (that the
   gate's flat `read_dir` misses nested manifests) was also checked and is
   FALSE: `raw-mirror/v1/manifests/` holds flat
   `doctor-raw-mirror-manifest-id-v1-*.json` files, which is exactly what the
   gate counts. So the gate fires and something else still hangs.
2. **The most likely remaining cause is the defect this session just fixed.**
   `probe_state_db` was unbounded on every path, not only triage's. That makes
   "build this branch, deploy, re-measure status" a real experiment with a
   falsifiable prediction, not a chore.

## The exact next action

Build the branch, deploy by atomic rename, and re-measure the three commands
against the live archive. The bound helper is written and working:
`~/.claude-accounts/erika/jobs/036c5f98/tmp/bound.sh <seconds> <binary> <args...>`
(there is no `timeout`/`gtimeout` on this machine; it needs
`zmodload zsh/datetime`).

```bash
cd /Users/dalecarman/dev/coding_agent_session_search/.claude/worktrees/cass-gen5-honesty
export PATH="$HOME/.rustup/toolchains/nightly-aarch64-apple-darwin/bin:$PATH"
export CARGO_TARGET_DIR=/tmp/cass-repair-target      # ALREADY WARM — do not make a new one, see the disk section
cargo build --release
# preserve first, then atomic rename — never cp over the live path (stale
# signature cache gives SIGKILL). Preserved binaries live beside it; nothing
# has ever been deleted.
cp ~/.local/bin/cass ~/.local/bin/cass.pre-gen5-$(date -u +%Y%m%d-%H%M%S)
mv /tmp/cass-repair-target/release/cass ~/.local/bin/cass.new && mv ~/.local/bin/cass.new ~/.local/bin/cass
```

Then, with the catch-up still running so the load matches the earlier readings:

```bash
zsh ~/.claude-accounts/erika/jobs/036c5f98/tmp/bound.sh 90 ~/.local/bin/cass triage --json
zsh ~/.claude-accounts/erika/jobs/036c5f98/tmp/bound.sh 90 ~/.local/bin/cass status --json
zsh ~/.claude-accounts/erika/jobs/036c5f98/tmp/bound.sh 90 ~/.local/bin/cass health --json
```

Baselines to compare against, all measured on the live archive:

| command | deployed `db29a26e` | prediction for this branch |
|---|---|---|
| `triage --json` | TIMEOUT 75.84s / 99.26s | returns; `counts` null, `status` degraded |
| `status --json` | **TIMEOUT 99.26s** | returns — this is the open question |
| `health --json` | 3.05s rc=1 | unchanged |

If `status --json` returns, `nvq59` closes on this evidence and `nao4q` closes
with it. **If it still times out, do not guess** — run it with `--verbose`
(robot mode hard-codes the log filter to `error` and ignores `RUST_LOG`,
`src/lib.rs:5769-5775`) and read where it actually sits, the way `nao4q` was
pinned.

## THE BLOCKER — the catch-up cannot finish without Dale

Measured, not extrapolated, and recorded as a comment on bead
`coding_agent_session_search-reclaim-tmp-cargo-targets-jck92`:

- guard start 16:05:55Z free = 67 GB; after batch 8 at 17:28Z free = 50 GB
- of that 17 GB, 3 GB was this session's cargo builds, so indexing costs
  **~1.8 GB/batch**
- 32 batches remain -> **~58 GB needed**, against **~25 GB** of headroom above
  the catch-up's own 25 GB disk-guard floor

So the run stops around **batch 22 of 40** and the guard halting it is correct
behaviour, not a failure. Batches are idempotent and resumable, so nothing is
lost — it simply cannot finish until space exists.

`/tmp` holds 100 GB of cass cargo target dirs; 82 GB of that is stale (seven
dirs, from worktrees with no live session — `ListAgents` shows none of 22 peers
is in this repo). `/tmp/cass-repair-target` (19 GB) is IN USE and must not be
touched. The exact commands are in the bead comment. **Do not act on it without
Dale's explicit written permission** — RULE 1. Let the catch-up run until the
guard stops it; that is the correct behaviour and it maximises progress.

## What was done, and where the proof is

Commit `0cf37f0c` on branch `worktree-cass-gen5-honesty`, pushed.

- **`-nao4q`** — `probe_state_db` now runs open + read + close on a worker with
  one `recv_timeout`, the same shape as `read_connector_scan_floors_bounded` one
  level up. The expiry path sets `open_retryable: false` **on purpose**:
  `run_status` computes `db_available = db_opened || (db_exists &&
  db_open_retryable)` and `healthy` never asks whether the probe completed, so
  `true` there would have printed `"status": "healthy"` for an archive that
  could not be read at all. A survey lane caught that, and bounding the read
  without it would have traded a visible hang for a silent lie.
- **`-0gzok`, the count half** — `state_db_count_or_unknown` keeps the existing
  fresh-connection retry and returns `None` when the count could not be
  obtained. `.unwrap_or(0)` had made a failed query indistinguishable from an
  empty table while `counts_skipped` stayed `false` beside it.
- **`-ddkwa`** — `parse_connector_scan_floors` returns `Option`. The per-entry
  tolerance is unchanged; only the whole-blob case became `None`.
- **`-a59ou`** — an unreadable quarantine file now lists its path, degrades the
  summary, and degrades `cass status`/`health` with it. The new field is
  `skip_serializing_if = "Vec::is_empty"`, so **every golden is byte-unchanged**.

Two mutants were run and reverted, each killing exactly its own case: the probe
running inline again fails the bound case with `counts_skipped=false`, and
restoring `.unwrap_or(0)` fails the count case with `left: Some(0)`.

Two tests were re-adjudicated with evidence rather than re-armed —
`probe_state_db_reads_meta_without_count_scan` (its timeout argument changed
meaning from a lock allowance to a hard wall-clock bound, so 250ms became the
5s production value) and `parse_connector_scan_floors_tolerates_junk` (renamed;
whole-blob junk is the behaviour `ddkwa` was filed against).

## Open, with what is known

- **`-sgvg3` (P0) is NOT the live hole its own text describes** — a survey lane
  settled the bead's stated open question and found the opposite mechanism.
  `archive_db_unreadable` (`src/lib.rs:34630-34643`) blocks nothing: it is only
  computed by `run_doctor_archive_scan_impl`/`run_doctor_archive_normalize_impl`,
  and `run_doctor_impl` never calls it. What actually refuses is `db_ok` — when
  the archive DB is readable, `candidate_promotion_candidate` is `None` and the
  promote action is never planned (`src/lib.rs:47925-47935`). So a transient
  inventory-query error alone cannot promote. The gate IS still fail-open when
  BOTH probes fail (they have different timeouts, 30s vs 1s), behind two-step
  approval and a pre-replacement backup. Root cause is representational:
  `DoctorCoverageSummary.archive_conversation_count` is a bare `usize` while the
  candidate side is `Option<usize>` and already fails closed. **Two live
  consequences worth fixing:** `raw_mirror_links_minus_archive`
  (`src/lib.rs:36989`) is computed against the fabricated 0 and feeds a trigger
  that makes `cass doctor --fix` STAGE a candidate (a real write) while the
  archive is fine; and the fail-closed candidate branch has **no regression
  test** — neither test passes `None`.
- **`-0gzok` part 1 (`last_scan_ts`)** — unfixed. A failed read still renders as
  a NOT STALE index. Needs genuine tri-state; `Option<i64>` already spends `None`
  on "absent". The staleness lane established the ripple claim in the bead is
  **not** true today: zero goldens reach a successful DB open, so a type-only fix
  changes zero golden bytes.
- **`-xarzt`** — health prints `healthy` when the coverage read FAILED. Left
  alone deliberately: the bead itself says whether "could not check" should
  degrade the verdict is a product call. **Ask Dale.**
- **`-2bh4a`** — close it when the catch-up finishes and the set-diff proves it.
  Acceptance:
  `python3 thoughts/shared/handoffs/20260815-cass-to-green/catchup-manifest.py /tmp/verify-manifest.txt`.
  Count by set-diff, NOT `connector_coverage.complete` — the archive has no
  `connector_scan_floors` meta row, so that field is structurally incapable of
  reporting this hole.
- **`-qtn0e`** — answered by census last session; the hazard stands.
- **`-p3kgr` / Dale's frankensqlite question** — partly answered, and the answer
  is concrete. The repo's own `AGENTS.md` RULE 2 already mandates fixing
  frankensqlite rather than routing around it, and names
  `/data/projects/frankensqlite` — which **does not exist on this machine**.
  Dale already has a public fork at `carmandale/frankensqlite` (forked from
  `Dicklesworthstone/frankensqlite`, last pushed 2026-05-15). The wiring is
  wrong, though: `fsqlite`/`fsqlite-types` resolve from the **crates.io
  registry** (`Cargo.lock` has zero references to the git URL), while the
  commented-out override in `Cargo.toml:244-246` is
  `[patch."https://github.com/Dicklesworthstone/frankensqlite"]` — a **git-source**
  patch table, which does not apply to a registry dependency. Anyone following
  that comment would find their local checkout silently ignored. **Not yet
  verified by an executed probe** — that needs the fork cloned and
  `cargo metadata` run, and it needs `[patch.crates-io]` instead. Note the
  fresh-clone CI guard (`.github/workflows/fresh-clone-build.yml`) fails if any
  sibling-path `[patch]` block is committed.

## Remainders this session could not discharge

- **The merge to `main`.** This background harness rejects every edit to the
  shared checkout until the session isolates, and then forbids pushing `main` —
  the same artifact the parent's "Landed the stranded chain" section describes
  for generations 2-4. The work is safe on
  `origin/worktree-cass-gen5-honesty` at `0cf37f0c`. A shared-checkout session
  merges it, then `git push origin main:master` per this repo's `AGENTS.md`.
- **`.beads/issues.jsonl` in the SHARED checkout is dirty** — `br comments add`
  auto-flushed the jck92 measurement into it. That is a real record that should
  be committed; this session cannot commit it from inside a worktree.

## Environment facts that cost real time

1. Build needs nightly on `PATH`; an absolute path to nightly cargo is not
   enough. `CARGO_TARGET_DIR=/tmp/cass-repair-target` is warm — reuse it and
   confirm the `Compiling coding-agent-search (<the path you mean>)` line, which
   is the napkin's stated guard against two checkouts sharing one target dir.
2. `cargo test --lib` shells out to the **installed** binary
   (`src/sources/probe.rs` `real_probe_*` -> `cass health --json`). Only health,
   not triage — so the suite does not hang today.
3. `--json` sets robot mode, which hard-codes the log filter to `error` and
   ignores `RUST_LOG` (`src/lib.rs:5769-5775`). Add `--verbose`.
4. Deploy by **atomic rename**, never `cp` over the live path.
5. No `timeout`/`gtimeout`. Use `bound.sh` above.
6. Indexing requires `CASS_SKIP_PREFLIGHT_CLEANUP_ORPHAN_FK_ROWS=1`.
7. A plain `cass index` is the wrong tool and wedges — it skips files older than
   the watermark then advances the watermark past them. Path-scoped
   `--watch-once` is what the catch-up uses and why.
8. **`br` from inside a worktree fails** with a sync-conflict error, because
   `.beads/beads.db` is gitignored and lives only in the shared checkout. Pass
   `br --db /Users/dalecarman/dev/coding_agent_session_search/.beads/beads.db`.
9. This harness refuses compound shell commands from a worktree-isolated
   session ("too complex to verify that it stays inside the worktree"). Put
   multi-step shell work in a script file and run the script.

## Evidence

`thoughts/shared/handoffs/20260815-cass-to-green/gen5-coordinator.md` and the
five read-only survey lanes under `lanes/gen5-*.md`, all committed at `0cf37f0c`.
The promote-gate and status-hang lanes are the two worth reading in full.
Backup: `~/backups/cass/agent_search-20260814-vacuum.db`, 3.98 GB, verified.
