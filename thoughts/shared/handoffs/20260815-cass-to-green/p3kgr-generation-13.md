---
generation: 13
parent-session: a91c2501-1830-4d3d-9430-3c9afe08a63c
next-action-class: executable
---

# Generation 13 — the rebuild is fixed and proven; one blocker left in the query phase

## The goal and authorization, verbatim

Dale, 2026-08-14:

> /my-way fix cass to completion and 100% green working state and completely up to date or tell me why it can't or /grill-me with any questions.

Mid-work correction, same day:

> make sure that you are looking at the recent (last 2 weeks) work on cass and not regressing

Dale, 2026-08-15:

> your usage is good now. finish this to completion

Dale, 2026-08-16:

> if your senior dev recommendation is to delete those 4 stale directories do it. if that is what progresses us toward the goal. and I would prefer you just do that and only stop when it would break the pipeline or running sessions or if there is a true blocker or ambiguity or conflict rather than sitting here for something that I am going to just ask your recommendation on and agree to

Dale, 2026-08-16, the message that redirected this work:

> there has been SO MUCH WORK done on cass and it feels like you agents are all wondering around blind folded in a mine field in a lava flow. can I PLEASE have an agent like Opus 5 on high thinking....oh, that's what you are... not run around incircles playing whac-a-mole? what the heck?!

**No approval transfers to you.** Destructive and external-write approvals
expired with the ending session. You may not file anything upstream on
frankensqlite/asupersync, delete any file, force-push, rewrite history, change
repo visibility, or run `cass sources agents exclude` (that last would destroy
3,877 conversations existing nowhere else). Dale's standing rule: **never delete
a file without express permission**, including files you created.

## What is settled, measured, and NOT to be re-litigated

The search hang was never a cass corruption problem, never the catch-up's fault,
and never a missing index. Three distinct defects, in order of what matters.

### 1. The guard inversion in cass — FOUND AND FIXED, PROVEN

`src/storage/sqlite.rs`, `list_conversation_footprints_for_lexical_rebuild`
(:7405-7414). Its own doc comment says the exact-count scan is a fallback for
conversations *missing* tail metadata. The guard fired when at least one row
**had** it — the inverse — so a full `GROUP BY` over every message ran on every
healthy rebuild.

Measured on the live archive: **0** conversations missing tail metadata, and
**0 of 27,441** footprints the scan would change. It cannot change any, because
`last_message_idx + 1` is an upper bound and the raise only moves counts up.

The fix gates it behind one ungrouped `SELECT COUNT(*) FROM messages` (19 ms
measured) against the estimate sum already in memory:

```rust
if !every_footprint_was_missing_tail
    && self.lexical_rebuild_tail_estimates_understate_message_total(&footprints)?
{
    self.raise_lexical_rebuild_footprints_to_exact_message_counts(&mut footprints)?;
}
```

**Result, on the SHIPPING fsqlite 0.1.5 pin, no bump:**

```
lexical refresh ledger published total_duration_ms=57351 failed_phase=""
search lexical self-heal completed action="rebuilt-from-canonical-db" indexed_docs=2334366
```

**57.4 seconds** for the rebuild that had never completed. 159 shards, 2.3 GB
index. `plan_lexical_shards` cleared in ~6 seconds. Cross-check: the 12,722-
conversation control rebuild is 17.6 s wall, so 57 s at 4x the messages is the
right shape.

**The patched tree is `/tmp/cass-0119-test`** — verified byte-identical to
shipping `main` except this one change, with the shipping `Cargo.lock` restored.
Binary at `/tmp/cass-fix-target/release/cass`. **The change is NOT yet in the
repo.** Landing it is the main deliverable.

The existing test `list_conversation_footprints_for_lexical_rebuild_raises_stale_low_tail_cache`
(:20899) covers the staleness path and the guard preserves it by construction
(estimate 1, exact 3, fires) — **but it has not been executed. Run it.**

### 2. fsqlite cannot plan `GROUP BY` — REAL, UPSTREAM, UNFIXED IN EVERY RELEASE

Stock SQLite plans it as a covering-index scan and never touches the table:

```
EXPLAIN QUERY PLAN SELECT conversation_id, COUNT(*) FROM messages
  GROUP BY conversation_id ORDER BY conversation_id ASC;
`--SCAN messages USING COVERING INDEX sqlite_autoindex_messages_1
```

fsqlite evidently does not, so it reads every row's `content` — the whole 22 GB.

| `GROUP BY` over 2,335,514 rows | |
|---|---|
| stock sqlite3 3.54.0 | **77 ms** |
| fsqlite 0.1.19 | **8,927,989 ms** (2h28m48s, correct result) |
| fsqlite 0.1.5 | **>10h**, sibling's run never finished |

Discriminator: an **ungrouped** `COUNT(*)` on the same table is 19 ms under
fsqlite against stock's 9 ms. Scanning is fine; grouping fails to use the index.

A 40-line standalone reproduction exists at `/tmp/fsq-probe/src/main.rs` — this
is exactly the clean reproduction generation 11 said was missing. **Filing needs
Dale's explicit approval and is NOT inherited.**

### 3. Page-cache thrash in the query phase — DIAGNOSED, UNTESTED, THE OPEN BLOCKER

After the rebuild succeeded, the `cass search` query itself ran 3+ minutes
without returning, at ~3.4 cores with no I/O. `sample(1)` on the live process
(saved at `/tmp/cass-search-sample.txt`) puts every working frame in fsqlite's
pager, not lexical search:

```
S3Fifo::insert                          649
hash_one(PageNumber)                    586
S3Fifo::trim_ghosts                     554
S3FifoEvictionTracker::build_model      497
HashMap<PageNumber, EntryState>::insert 167
```

Hypothesis: cass sets `PRAGMA cache_size = -65536` (64 MB) at
`src/storage/sqlite.rs:4267` and `:4324`, and `-16384` (16 MB) for readers at
`:870`. Against a 22 GB database that is ~16,000 cached pages for structures
whose scans touch 972,677 — on a machine with 128 GB of RAM.

**`FSQLITE_CACHE_PAGES` is a registered module name, not an environment
variable** (`fsqlite-core-0.1.5/src/connection.rs:12946`), so this cannot be
tested by env var. It needs a code change to the PRAGMA.

That run was killed by session teardown before it returned, so **it is
inconclusive, not a failure.** A re-run against the already-built index is in
flight; read `/tmp/rerun.log`, `/tmp/rerun-search.out`, `/tmp/rerun-search.err`
before assuming anything.

## The exact next action

1. **Read `/tmp/rerun.log`.** It holds whether `cass search` returns against the
   already-built index at `/tmp/cass-fix-data` (2.3 GB, with
   `lexical-generation-manifest.json`, so the rebuild should be skipped).
   - Returns quickly → the whole thing works; go to 2.
   - Still slow → raise `cache_size` in the patched tree (try `-2097152`, 2 GB),
     rebuild, re-run. That is the falsifier for defect 3.
2. **Land the guard fix on `main`** — the patch is in `/tmp/cass-0119-test/src/storage/sqlite.rs`;
   `diff` it against the repo to extract exactly the one change. Run the existing
   footprint tests first (five of them near `src/storage/sqlite.rs:20683-21060`).
   Add a test that fails when the guard inverts again — there is none today.
   Commit by exact path, push `main` and `main:master`.
3. **Do not bump the pin to fix this.** cass builds clean against 0.1.19 on
   `nightly-2026-08-10` (rustc 1.99, rc=0, 6m30s) but 0.1.19 does not fix the
   query. The bump is a separate, non-urgent question.

## Environment facts that cost real time

1. **`rustup toolchain list` first.** Every prior handoff named
   `$HOME/.rustup/toolchains/nightly-aarch64-apple-darwin` as environment fact
   #1; that is a **1.94 nightly from December 2025**. `nightly-2026-08-10` is
   rustc 1.99 and has been installed the whole time. That single unchecked
   inherited line is most of the "whack-a-mole".
2. Bumping the pin needs `build.rs` `expected_version` (two sites, exact string
   compare) and `Cargo.toml` (spelled `0.1.19`, **no `=` prefix**) moved together.
3. `fsqlite = "0.1.5"` resolves `fsqlite-core` **forward to 0.1.19**. A
   version-range probe is not a control. Restore the repo's `Cargo.lock`.
4. `cass search` with no override writes to the **live** archive's index and takes
   its `index-run.lock`. Use `CASS_DATA_DIR=<scratch>` (`src/lib.rs:81214`) plus
   `--db`. Set `CASS_SKIP_UPDATE=1`.
5. Probe against **APFS clones** (`cp -c`) — `fsqlite::Connection::open` is
   read-write. The clones cost ~10 GB of real space, not zero.
6. fsqlite's API is not rusqlite's: `Statement::query()` takes no arguments and
   returns an eager `Vec<Row>`; `Row::get(usize) -> Option<&SqliteValue>`; the
   float variant is `Float`, not `Real`; `prepare()` rejects `EXPLAIN`.
7. Background jobs die with the session. Use `setsid nohup`.
8. Disk is under the janitor's 150 GB floor (76 GB free). The clones at
   `/tmp/fsq-probe-data` are ~27 GB. **Do not delete them without asking.**

## State

- `main` is green on fsqlite 0.1.5 and is still the shipping pin. Do not merge
  `worktree-cass-gen5-honesty`.
- Open beads: `p3kgr` (P0, carries all three defects as comments), `759l7`,
  `9fnbr`, `qtn0e`.
- `indexed_docs=2334366` is 1,148 short of 2,335,514 rows in `messages` —
  possibly the same population as `9fnbr`. Check, do not assume.
- A sibling session's 0.1.5 rebuild (pid 38174) was still grinding at 10h+.
  Check `claude agents` and `lsof` before assuming a lock or dirty file is yours.
