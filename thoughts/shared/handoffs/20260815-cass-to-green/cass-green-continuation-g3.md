---
generation: 3
parent-session: 09a898c8-0a9e-4665-804e-37fb7e9ac7b1
next-action-class: executable
---

# Continuation — the honesty family is closed and deployed; the frankensqlite pin bump is the next real experiment

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
destroy 3,877 conversations that exist nowhere else on Earth. The `/tmp` cargo
reclaim on bead `-jck92` is a deletion and needs Dale's explicit written
permission; it has not been given.

## THE HEADLINE — the deploy experiment ran and the prediction held

`cass status --json` and `cass triage --json` both **return** now. They were one
defect, not two.

Measured on the live 7.9 GB archive with the codex catch-up running so the load
matches the earlier readings, each bounded at 90s:

| command | deployed `82f316a7` | gen5 `463f2649` |
|---|---|---|
| `triage --json` | TIMEOUT 75.84s / 99.26s | **rc=0, 5.17s** |
| `status --json` | **TIMEOUT 99.26s** | **rc=0, 5.13s** |
| `health --json` | 3.05s rc=1 | 2.15s rc=1, unchanged |

All four honesty fixes are now observed live rather than only unit-tested:
`database.open_error` names the 5000ms bound that fired, `conversation_count` and
`message_count` are JSON null beside `counts_skipped: true`,
`connector_coverage` is `{"checked": false, "complete": null}`, and `status` is
`"stalled"` with `healthy: false` — a failed probe degrades the verdict instead
of printing good news.

**Closed on that evidence: `-nvq59`, `-nao4q`, `-ddkwa`, `-a59ou`.**

## What else landed this session

- **`-0gzok` part 1** (commit `d4004fbd`). `last_scan_ts` is now
  `Option<Option<i64>>` — the same "None means did not check" convention
  `connector_scan_floors` documents. Making that expressible needed one SQL
  change: `query_row_map` returns `Err` both when a read fails and when the key
  is absent, so the plain `SELECT` could not tell them apart at all; wrapping the
  lookup in a scalar subquery makes the statement always yield exactly one row.
  That the scalar subquery works on frankensqlite 0.1.5 is **measured** by the
  new test, not assumed. Mutant run and reverted: restoring the collapsing read
  fails the new test alone with `left: None, right: Some(None)`.
  **`-0gzok` is now fully fixed and can be closed** — part 2 landed in `0cf37f0c`
  and is observed live above; nobody has closed the bead yet.
- **`-sgvg3`** (the commit after `d4004fbd`). `DoctorCoverageSummary` and
  `DoctorSourceAuthorityCoverageDelta` gained
  `archive_conversation_count_unknown`, set from the `db_query_error` signal that
  already existed. The gate now blocks on an unknown baseline the way the
  candidate side already blocks on `None`; gate evidence prints
  `archive-conversation-count=unknown` instead of a fabricated 0; and
  `doctor_candidate_build_should_run` no longer treats
  `raw_mirror_links_minus_archive` as meaningful when the archive side of that
  subtraction was never measured. Both fields carry `skip_serializing_if`, so
  every golden is byte-unchanged.

## The exact next action

**Run the frankensqlite 0.1.17 pin-bump experiment.** It is the one experiment
that could retire `-p3kgr` (P0, and the reason the archive cannot advance by any
supported route), and this session established that it is a version bump rather
than the fork Dale asked about.

The full answer to Dale's fork question, with its evidence and its proof
boundary, is committed at
`thoughts/shared/handoffs/20260815-cass-to-green/lanes/gen5-frankensqlite-fork-answer.md`.
Read it first. The short version: `ExistsValueSet` is 0 occurrences in
`fsqlite-core-0.1.5/src/connection.rs` and 8 in `0.1.17`'s — same file, same
instrument, so the zero is a real absence — and 0.1.17 is **already in the local
cargo registry cache**, so this needs no network.

```bash
cd /Users/dalecarman/dev/coding_agent_session_search/.claude/worktrees/cass-gen5-honesty
export PATH="$HOME/.rustup/toolchains/nightly-aarch64-apple-darwin/bin:$PATH"
export CARGO_TARGET_DIR=/tmp/cass-repair-target      # WARM — do not make a new one, see the disk section
# Cargo.toml:45 is the pin: frankensqlite = { version = "0.1.5", package = "fsqlite", ... }
# Note fsqlite-vfs already resolves at 0.1.6 while the rest of the family is
# 0.1.5, so this may not be a single-number edit.
cargo update -p fsqlite --precise 0.1.17   # or edit the pin, then:
cargo check --lib
```

Then the falsifier that actually matters, because the incremental path is
already unwedged and the FULL REBUILD path is not:

```bash
# against a THROWAWAY copy, never the live archive
CASS_SKIP_PREFLIGHT_CLEANUP_ORPHAN_FK_ROWS=1 cass index --full --force-rebuild --json --verbose
```

Baseline to beat: that command sat in `phase=preparing` for a full 30-minute
bound and never advanced (rc=143 at the watchdog), *with the skip var set*. If it
still wedges, the pin bump is not the answer and the leading suspect is the
cass-side unconditional `GROUP BY` over all 580,374 messages in
`raise_lexical_rebuild_footprints_to_exact_message_counts`
(`src/storage/sqlite.rs:7456`) — read where the fallback WARN actually fires with
`--json --verbose` rather than guessing.

**Do not bump the pin on `main` or deploy it without that measurement.** A green
`cargo check` proves nothing about the wedge.

## THE BLOCKER — two of them, and both are Dale's

1. **Disk.** `/System/Volumes/Data` is at **31-32 GB free** and falling; the
   catch-up's own guard floor is 25 GB. `/tmp` holds ~100 GB of cass cargo target
   dirs, ~82 GB of it stale from worktrees with no live session.
   `/tmp/cass-repair-target` (~19 GB) is IN USE and must not be touched. The
   exact reclaim commands are recorded on bead
   `coding_agent_session_search-reclaim-tmp-cargo-targets-jck92`. **This is a
   deletion: RULE 1 means it needs Dale's explicit written permission.** Do not
   act on it without that. Until then the catch-up stops around batch 22 of 40 —
   correct guard behaviour, not a failure, and batches are idempotent and
   resumable.
2. **`-xarzt` is a product call, not a bug.** Should "could not check" degrade
   the one-word verdict? `cass health` prints `healthy` when the coverage read
   FAILED. This session deliberately did NOT decide it: the `-0gzok` part 1 fix
   surfaces the unknown through `status_reason` and leaves `fresh`/`healthy`
   alone, precisely because that flip is the same question. **Ask Dale.**

## Open, with what is known

- **`-0gzok`** — close it. Both parts are fixed and proven; see above.
- **`-p3kgr`** — the next action above. Two distinct wedges: incremental is fixed
  by `CASS_SKIP_PREFLIGHT_CLEANUP_ORPHAN_FK_ROWS=1`; full rebuild is not.
- **`-2bh4a`** — the codex catch-up. **Owned by a live peer session**
  (`coding_agent_session_search-cont-...-2bh4a-g1`). Do not compete with it.
  Acceptance is a set-diff, NOT `connector_coverage.complete` — the archive has
  no `connector_scan_floors` meta row, so that field is structurally incapable of
  reporting this hole:
  `python3 thoughts/shared/handoffs/20260815-cass-to-green/catchup-manifest.py /tmp/verify-manifest.txt`
- **`-qtn0e`** — the data-loss question is STILL UNANSWERED and the recorded
  "source-absent rows SURVIVED, delta 0" is a **null result**, not a pass: both
  attempts ended with the rebuild never running, so the instrument could not have
  detected a drop. Today's mitigation is luck — `--force-rebuild` cannot complete
  on an archive this size — and it stops being true the moment `-p3kgr` is fixed.
  So the next action above and this bead are coupled: fixing the full-rebuild
  path re-arms a destructive path that has never been proven safe.
- **`-sgvg3` follow-up** — three surfaces still state a failed query as measured
  fact but do not gate a write: the "archive database currently contains 0
  indexed conversation(s) and is authoritative" prose, the
  `archive-conversation-count=0` evidence on the coverage-risk surface, and
  `doctor_coverage_confidence_tier` returning `"no_archive_rows"` rather than
  `"unchecked"`. Deliberately left out of this session's commit rather than
  widening it.

## Remainders this session could not discharge

- **The merge to `main`.** This background harness rejects edits to the shared
  checkout until the session isolates, and then forbids pushing `main`. The work
  is safe on `origin/worktree-cass-gen5-honesty`. A shared-checkout session
  merges it, then `git push origin main:master` per this repo's `AGENTS.md`.
- **The comment fix at `Cargo.toml:244`.** It is a git-source `[patch]` table
  against a dependency that resolves from crates.io, so anyone following it finds
  their local checkout silently ignored. The working form is `[patch.crates-io]`.
  Not edited here to keep the sgvg3 commit clean, and because
  `.github/workflows/fresh-clone-build.yml` fails on a committed sibling-path
  patch — a local override belongs in an uncommitted `.cargo/config.toml`.

## Environment facts that cost real time

1. Build needs nightly on `PATH`; an absolute path to nightly cargo is not
   enough. Confirm the `Compiling coding-agent-search (<the path you mean>)` line
   — the napkin's guard against two checkouts sharing one target dir.
2. `cargo test --lib` shells out to the **installed** binary
   (`cass health --json`). Only health, so the suite does not hang.
3. `--json` sets robot mode, which hard-codes the log filter to `error` and
   ignores `RUST_LOG` (`src/lib.rs:5769-5775`). Add `--verbose`.
4. Deploy by **atomic rename**, never `cp` over the live path (stale signature
   cache gives SIGKILL). Preserve first; nothing has ever been deleted.
5. No `timeout`/`gtimeout`. Use
   `~/.claude-accounts/erika/jobs/036c5f98/tmp/bound.sh <seconds> <bin> <args>`.
6. Indexing requires `CASS_SKIP_PREFLIGHT_CLEANUP_ORPHAN_FK_ROWS=1`.
7. A plain `cass index` is the wrong tool and wedges. Path-scoped `--watch-once`
   is what the catch-up uses.
8. **`br` from inside a worktree fails.** Pass
   `br --db /Users/dalecarman/dev/coding_agent_session_search/.beads/beads.db`.
   `br` writes its flush to the SHARED checkout's `.beads/issues.jsonl`, so carry
   that file into the worktree before committing bead changes.
9. This harness refuses compound shell commands from a worktree-isolated session.
   Put multi-step shell work in a script file and run the script.
10. **Check `cusage` before any fan-out.** The signed-in account was at 100% of
    its weekly window this session, so this generation ran solo and consumed the
    committed lane logs as evidence instead of re-running surveys. At or above
    95%, do not launch lanes.

## Evidence

`thoughts/shared/handoffs/20260815-cass-to-green/agent-log-gen5-deploy.md` (this
session's coordinator log, including its proof boundary),
`lanes/gen5-frankensqlite-fork-answer.md`, and the five committed gen5 survey
lanes under `lanes/gen5-*.md` — the promote-gate and status-hang lanes are the
two worth reading in full. Backup:
`~/backups/cass/agent_search-20260814-vacuum.db`, 3.98 GB, verified.
