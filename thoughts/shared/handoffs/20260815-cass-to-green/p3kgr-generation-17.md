---
generation: 17
parent-session: 64dacc01-d2f4-4eec-ae68-83e86ae092d9
next-action-class: executable
---

# Generation 17 — `cass status` is FIXED and proven on the real archive. `cass doctor` is not, and its cause is now named.

## The goal and authorization, verbatim

Dale, 2026-08-14:

> /my-way fix cass to completion and 100% green working state and completely up to date or tell me why it can't or /grill-me with any questions.

Dale, 2026-08-15:

> your usage is good now. finish this to completion

Dale, 2026-08-16:

> if your senior dev recommendation is to delete those 4 stale directories do it. if that is what progresses us toward the goal. and I would prefer you just do that and only stop when it would break the pipeline or running sessions or if there is a true blocker or ambiguity or conflict rather than sitting here for something that I am going to just ask your recommendation on and agree to

Dale, 2026-08-16:

> there has been SO MUCH WORK done on cass and it feels like you agents are all wondering around blind folded in a mine field in a lava flow. can I PLEASE have an agent like Opus 5 on high thinking....oh, that's what you are... not run around incircles playing whac-a-mole? what the heck?!

**Destructive and external-write approvals expired with the ending session and do
not transfer to you.** You may not file anything upstream on
frankensqlite/asupersync, delete any file, force-push, rewrite history, merge to
`main`, change repo visibility, or run `cass sources agents exclude` (that last
would destroy 3,877 conversations existing nowhere else). Dale's standing rule:
**never delete a file without express permission**, including files you created.

## What generation 17 landed

Two commits on `worktree-cass-p3kgr-gen13`, pushed: `46d74410` (the fix) and
`0ba4f688` (beads).

Bead `-zumve` is **closed**. `cass status --json` no longer hangs on a large
archive, measured on Dale's real 23 GB specimen rather than inferred:

| | BEFORE (gen-16 binary) | AFTER |
|---|---|---|
| `cass status --json --robot-meta` on the 23 GB archive | **no result in 240 s, 0 bytes** | **rc=0 in 6 s, 21,659 bytes** |

Both binaries marker-verified, never by timestamp: the BEFORE carries neither
new string literal and matches its recorded sha256 `01896686…`; the AFTER
carries both and hashes `5adf9957785af9f3d4fd91dff2a13cafca99ffe3e77e59599ac51edf7d5644a2`.

The AFTER output is honest rather than merely fast — `archive_coverage_state:
"not_checked"`, `counts_skipped: true`, and a recommended action naming `cass
doctor`. It declines the expensive work and says it declined.

### The root cause was three levels deep, and the bead named only the third

1. `run_status` collects archive coverage inline, which runs doctor's
   whole-archive raw-mirror backfill census.
2. That census was gated by `status_raw_mirror_scan_too_large`, which counts
   raw-mirror **manifests**. That gate is correct for the mirror walk it was
   written for (bead `-nvq59`). But **one boolean gated two costs with different
   drivers, and only the mirror was ever measured** — so a data dir with an empty
   mirror pointing at a huge database passed the gate and ran the census anyway.
   An absent manifest directory returns "not too large", so the worst case passes
   most easily. Confirmed by construction: the acceptance data dir has no
   `manifests/` directory at all.
3. Inside the census, **two** terms are unbounded in the archive rather than the
   mirror, and neither is gated by `apply`.

The bead named only the correlated `COUNT(*)`. The second term — every
candidate's live provider session file read in full and single-threaded
BLAKE3-hashed, whose only reader is the comparison against an existing
raw-mirror blob hash, so on an empty mirror every byte is discarded — was found
by a parallel review lane and would have been missed by a SQL-only fix.

### Three edits, all `src/lib.rs`

1. `status_archive_scan_too_large` — the twin of the manifest gate on the
   dimension it does not measure. One `stat` against a 256 MiB default, the same
   figure `STATUS_COUNT_SCAN_MAX_DB_BYTES` already applies to a plain `COUNT(*)`.
   Overridable via `CASS_STATUS_COVERAGE_MAX_ARCHIVE_DB_BYTES` so the gate is
   testable at fixture scale. An unreadable database reads as too large — the
   same failure direction as the mirror gate.
2. The live-source hash is now conditional on the candidate having mirror
   evidence to compare against. The `stat` still happens; only the read is
   skipped.
3. The per-conversation message count comes from `conversations.last_message_idx
   + 1`, the archive's own cache, which this codebase already treats as the
   canonical count (`lexical_rebuild_message_count_from_tail_idx`,
   `src/storage/sqlite.rs:3705`). Rows whose cache is empty fall back to
   **bounded** point probes capped at 64 — same shape and figure as
   `LEXICAL_REBUILD_FOOTPRINT_POINT_TAIL_FALLBACK_LIMIT` — and above the cap the
   report says how many rows it could not count rather than passing a default off
   as a measurement.

### Do NOT follow the -zumve bead's own suggested fix (b)

It recommended one grouped `SELECT conversation_id, COUNT(*) FROM messages GROUP
BY conversation_id`. **That is the verbatim statement `26932422` removed from the
lexical rebuild** — 77 ms in stock sqlite3, 2h28m on fsqlite 0.1.19, never
finished in over 10 hours on the shipping 0.1.5 pin. It is the shape bead
`p3kgr` was filed about. Its fallback half was also impossible: `conversations`
has no `message_count` column on any schema version. The bead has been corrected
in its close reason so the next reader is not pointed at the mine.

### Proof

- **Suite**: `cargo test --lib` **5154 passed / 0 failed / 3 ignored** — identical
  to the recorded generation-16 baseline, zero regressions. `cli_doctor` 54/0
  (51 + 3 new), `cli_status`, `cli_robot`, `cli_diag` all green.
- **Mutants** (`~/.claude-accounts/katherine/jobs/64dacc01/tmp/mutants-final.log`,
  runner at `.../mutants.py`) — six mutants, **six PASS**, each removing exactly
  one behaviour a new test claims to pin:

  | mutant | red |
  |---|---|
  | drop the archive term from the gate | the new over-cap test only |
  | gate refuses every archive | the pre-existing under-cap test only (the other direction) |
  | correlated COUNT first, ignoring the cache | the tail-cache test |
  | drop the bounded point probe | the tail-cache test **and** the pre-existing backfill test |
  | hash unconditionally | the no-evidence test |
  | **negative control**: invert the hash condition | the pre-existing evidence-hit test **and** the new one |

  Two of my initial expectation lists were wrong, not the tests — read the
  observed dict, not the verdict word, if you re-run it.

## THE NEXT DEFECT IS ALREADY NAMED — bead `-lj72p` (P0), filed this session

**`cass doctor --json` still does not return on the 23 GB archive**, and none of
the three edits touches the reason. Measured this session: zero bytes, no result
in 300 s, with the fixed binary. The pre-fix binary is the same at 180 s, so it
predates the fix.

A sampled stack (1,663 samples, main thread, 100% on-CPU,
`~/.claude-accounts/katherine/jobs/64dacc01/tmp/doctor-sample.sample`) names it:

```
run_doctor_impl
 -> fsqlite_core::Connection::query_row_with_params -> execute_pragma
   -> pragma_maintenance::pragma_integrity_check_rows
     -> validate_database_integrity -> validate_schema_btrees_in_txn
       -> walk_integrity_btree_pages          (recursive)
         -> TransactionHandle::get_page -> PagerInner::read_page_copy
           -> ShardedPageCache::evict_any
             -> S3FifoEvictionTracker::build_model      1,258 of 1,663
```

An **unbounded full-database `PRAGMA integrity_check`**. It walks every b-tree
page, and fsqlite 0.1.5's pager makes that superlinear rather than merely long —
`evict_any` rebuilds the whole S3-FIFO model on each eviction, 76% of samples.
Same pager pathology already recorded under `-zumve`, reached by a different
caller.

**This is your exact next action: fix `-lj72p`.** Read the bead — it carries the
full stack, the code location (`src/lib.rs:25431-25441`, but confirm which pragma
call under `run_doctor_impl` actually blocks before editing; the existing
`DOCTOR_DATABASE_INTEGRITY_DIAGNOSTIC_LIMIT` bounds output rows, not the walk),
and three candidate fixes in minimalism-ladder order. The pattern to follow is
the one that just shipped: `status_archive_scan_too_large` plus
`STATUS_COVERAGE_MAX_ARCHIVE_DB_BYTES_DEFAULT` in `src/lib.rs`.

Note the loop this closes: `cass status` now tells the operator to "Run 'cass
doctor --json'". That command is currently unusable at this scale. Status is
honest and fast; the surface it points at is not.

The acceptance is a one-liner once it is fixed — reuse
`~/.claude-accounts/katherine/jobs/64dacc01/tmp/acceptance.sh`, which already
symlinks the specimen, bounds every run, and prints the verdicts.

## Still open, and owed to Dale

1. **Nothing is on `main`, and the landing recipe was corrected mid-session.**
   `26932422` (rebuild guard, 2h28m → 57s), `15f9af64` (the p3kgr query fix),
   `46d74410` and `0ba4f688` are all on `worktree-cass-p3kgr-gen13`, pushed.
   Background jobs cannot push `main`. **The branch does NOT fast-forward** —
   `origin/main` (`5d1718a3`) carries one commit this branch lacks, touching only
   `.beads/issues.jsonl`; merge-base is `10575de2`. The recipe is an ordinary
   merge, then regenerate the export from the database (`br sync --import-only
   --rebuild` then `br sync --flush-only`) rather than hand-resolving the JSONL,
   then confirm the beads are present before pushing. (Correction supplied by the
   gen-15 sibling session as `38cd5d35`.)
2. **A sibling job has burned 16+ hours on a pre-fix binary** — pid 38174,
   account `george`, `cass index --force-rebuild` via `~/.local/bin/cass`, which
   predates `26932422`. Do not kill another session's job. Installing the fixed
   binary is the remedy.
3. **`-iekel`: 13 GiB of dead index shards need deleting and no agent may.**
   `~/.claude-accounts/katherine/jobs/c3b442f9/tmp/acceptance-data/index/` holds
   partial Tantivy shards from two killed runs, incomplete by construction,
   nothing depends on them. Free space is ~43 GiB against a 150 GiB floor. The
   command is in the bead. **Do not extend it to `/tmp/fsq-probe-data/prod.db`** —
   that specimen is protected and is the only large archive available for
   measuring this class of defect.
4. **Three files carry unrelated `cargo fmt` churn I did not commit** —
   `src/connectors/codex.rs`, `src/connectors/mod.rs`,
   `tests/golden_robot_json.rs`. `cargo fmt` reformatted them; they were already
   unformatted on `HEAD` and I never edited them, so §4.2 says name them rather
   than sweep them. Restore with `git checkout -- <the three paths>` (dcg blocks
   it and its `allow-once` needs an interactive confirm no background job can
   give), or land them as a separate formatting commit. My own two files are
   `fmt --check` and `clippy` clean.
5. **Two pre-existing clippy errors**, `src/connectors/mod.rs:237` and `:280`,
   both `type_complexity`. Not attributable to this change — my diff never touches
   `src/connectors/`, and clippy ran before I ran `cargo fmt`. `cargo clippy
   --all-targets -- -D warnings` is therefore red on this branch for reasons that
   predate it. Worth a bead if nobody owns it.
6. **The fsqlite pin move is optional, not required** — priced in
   `thoughts/shared/handoffs/20260816-759l7-spin-wait-gen13/pin-move-cost.md`.
   0.1.19 answers the catastrophic GROUP BY in 2h28m, so it is not a fix for this
   class.
7. **`-hd4u5`** (the `rootpage > 0` FTS gate) is a production behaviour change
   three sessions have judged Dale's call. Untouched.
8. **Two follow-ups from generation 16, still unacted**:
   `src/search/model_manager.rs:390` computes the lexical fingerprint a second
   time on every default search (~40 ms of redundancy); and no test covers
   appending a message to an *existing* conversation, the one change that moves
   only the message half of the fingerprint.
9. **The fsqlite pager defect is worth reporting upstream** — an O(cache) model
   rebuild per eviction is what turns every unbounded page walk in this repo into
   unbounded wall time rather than a slow one. Two independent sampled stacks now
   show it. Filing upstream is not authorized.
10. **`collect_doctor_source_inventory` was not audited.** It sits behind the same
    status gate (now bounded as a side effect) and runs `GROUP BY 1,2,3,4,5` over
    `conversations` — 27k rows, not 2.3M, so first-order fine, but it is the same
    missed-sibling shape that produced `-zumve` and `-nvq59`'s second defect.
    Assume a third exists until someone reads them.

## Environment facts that cost real time

1. `rustup`/`cargo` are NOT on PATH — `export PATH="$HOME/.cargo/bin:$PATH"` first
   and confirm `rustc --version` reads `1.94.0-nightly (f52090008 2025-12-10)`.
   A bare `cargo` is Homebrew stable 1.96 and dies with `E0554`.
2. The worktree's own `./target` is warm — do NOT set `CARGO_TARGET_DIR` for
   tests. `/tmp/cass-fix-target` (7.7 GB) is the warm **release** tree; reuse it
   for binaries. `~/.claude-accounts/katherine/jobs/64dacc01/tmp/build-cass.sh`
   builds and marker-verifies in one step.
3. `cargo check --lib` does NOT type-check `#[cfg(test)]` or `tests/` code.
4. `cargo fmt` with no path argument reformats the WHOLE repo, including files
   you never touched. Pass paths, or check `git diff --stat` before staging.
5. dcg blocks `git checkout -- <path>`, and `dcg allow-once` prompts
   interactively, which a background job cannot answer. Plan around it rather
   than reaching for a workaround.
6. Disk is ~43 GiB against a 150 GiB janitor floor. `cass status` and `cass
   doctor` (without `--fix`) write nothing of size; `cass search` and `cass index`
   write tens of GiB. Generation 16's 13 GiB came from letting a `cass search`
   run to a 240 s wall clock. Bound every run and kill on the measured step.
7. `cass doctor` without `--fix` cannot mutate — `fix_can_mutate = fix && …`
   (`src/lib.rs:69310`). The specimen's size and mtime were byte-identical either
   side of four bounded runs this session.
8. Peer sessions answer within minutes and it is cheap — `ListAgents` then
   `SendMessage`. The gen-15 sibling caught a wrong landing recipe in my handoff
   before I acted on it.

## State

- Branch `worktree-cass-p3kgr-gen13` at `0ba4f688`, pushed. Working tree carries
  only the three `cargo fmt`-churned files above and ignored `.agent-state/`.
- `main` / `origin/main` at `5d1718a3`. The branch does not fast-forward — see
  item 1.
- Open beads: `p3kgr` (P0), **`lj72p` (P0, new — your next action)**, `759l7`,
  `9fnbr`, `qtn0e`, `hd4u5`, `xybl9`, `iekel`. `zumve` is closed.
- Evidence for everything above lives in
  `~/.claude-accounts/katherine/jobs/64dacc01/tmp/`: `acceptance.log` (the four
  bounded runs), `doctor-sample.sample` (the integrity-check stack),
  `mutants-final.log`, `lib-suite.log`, `gates.log`, `build-cass.log` (the marker
  guard). That directory dies with the job — copy anything you need to keep.
