---
generation: 1
parent-session: a91c2501-1830-4d3d-9430-3c9afe08a63c
next-action-class: executable
---

# Continuation — cass is unblocked and the backfill is running

## The goal and authorization, verbatim (Dale, 2026-08-14)

> /my-way fix cass to completion and 100% green working state and completely up to date or tell me why it can't or /grill-me with any questions.

Sent mid-work the same day, as a correction to the work in flight:

> make sure that you are looking at the recent (last 2 weeks) work on cass and not regressing

And on 2026-08-15, the question that produced the breakthrough:

> should we make a local fork of frankensqlite and fix it?

**Destructive and external-write approvals expired with the parent session and do
not transfer.** You do NOT have approval to: delete any file (this repo's AGENTS.md
RULE 1 forbids it outright, including files you create), force-push, rewrite history,
file anything on a public third-party repo, or run `cass sources agents exclude`.

## THE HEADLINE — read this first

**cass is no longer wedged, no fork is needed, and the backfill is running right now.**

The blocker was `cleanup_orphan_fk_rows` in the watch_startup preflight. cass#265
already shipped an operator escape hatch for exactly this, and it is present in the
deployed 0.6.9 binary:

```bash
CASS_SKIP_PREFLIGHT_CLEANUP_ORPHAN_FK_ROWS=1
```

Measured on the live archive 2026-08-15: with the var set, `cass index --watch-once`
reached `phase_code 1` in **1,335 ms** and completed `rc=0`. Without it, the same
command sat in `phase="preparing"` past a 20-minute bound. The database went 12,722 →
12,723 conversations — the first thing cass had indexed since 2026-07-16.

**Skipping the sweep is free on this archive.** `message_metrics`, `token_usage`,
`snippets` and `conversation_tags` all have **zero** orphan rows. The sweep that
wedges cass has nothing to clean. Verify that yourself before extending the bypass to
any other machine.

## What is running RIGHT NOW — do not start a second one

A backfill of 4,895 codex rollouts / 13.12 GB, launched under `nohup caffeinate -is`
so it survives the parent session ending.

- Script: `<scratch>/backfill.sh` (session scratchpad; copy preserved in the log below)
- Log: `<scratch>/backfill/run.log`
- Manifest: `<scratch>/cass-hole-manifest.txt` — 4,895 paths, smallest file first
- 20 batches of 250, each `--watch-once` with the skip var set
- Progress at handoff: **12,835 conversations** and climbing (from 12,722)

`<scratch>` = `/private/tmp/claude-501/-Users-dalecarman--agent-config/a91c2501-1830-4d3d-9430-3c9afe08a63c/scratchpad`.
macOS reaps `/private/tmp` after a few days — if the scratchpad is gone, rebuild the
manifest with the script in `lanes/backfill-mechanics.md` §9 Step 2 and re-run. Batches
are idempotent, so re-running completed ones is a no-op.

Check it with:

```bash
rg -c 'END rc=' <scratch>/backfill/run.log          # batches finished, of 20
sqlite3 "file:$HOME/Library/Application Support/com.coding-agent-search.coding-agent-search/agent_search.db?mode=ro" \
  'SELECT count(*) FROM conversations;'
```

**Two things to watch.** The single 2.57 GB rollout is deliberately LAST (smallest-first
ordering) — beads `-373b1` and `-2rtk7` record watch-once OOM/quarantine on large files,
so if anything dies it will be there and it is not a surprise. And disk was 133 GB free
at launch against a 150 GB floor that is already breached; the backfill is estimated at
24–30 GB. Watch it, and do not free space by deleting anything without asking Dale.

## The exact next action

1. Confirm the backfill is still alive and advancing (commands above). If it died,
   read `run.log` for the failing batch and re-run `backfill.sh` — it resumes.
2. When it finishes, run the catch-up for everything newer than 2026-07-16 — this is
   the ~2,342 codex + ~7,796 claude files that are merely stale, on the safer streaming
   producer path:
   ```bash
   CASS_SKIP_PREFLIGHT_CLEANUP_ORPHAN_FK_ROWS=1 \
     cass index --json --progress-interval-ms 60000
   ```
3. Then prove it. **`connector_coverage.complete` is NOT a valid acceptance signal**
   — the coverage floor is forward-looking and `meta` holds no floor rows, so it
   reports `complete: true` over the hole. Count indexed sessions against the on-disk
   corpus instead.

## Correction to the record — believe this, not the earlier claim

The parent session published a WRONG root cause and then corrected it. Bead
`-p3kgr`'s body names the GROUP BY aggregate in
`raise_lexical_rebuild_footprints_to_exact_message_counts` as the runaway statement.
**That is wrong**; read the bead's correction comment. The `correlated_exists_fallback`
branch requires a WHERE clause containing a correlated EXISTS, and the GROUP BY has no
WHERE at all. The real wedge is the orphan-FK probe at `src/storage/sqlite.rs:6002`,
and cass's own source already named that hang site at `src/indexer/mod.rs:2493`.

The lesson worth carrying: the answer was in this repo's own comments and beads the
whole time. Search the record before instrumenting the binary.

## The fork question — answered, no

Do not fork frankensqlite. It is 761,565 lines across 19 crates, and:

- Upstream **already fixed** this engine defect. Issue #117 names the identical
  `correlated_exists_fallback` string and was fixed in fsqlite **0.1.11**, six releases
  past the 0.1.5 cass pins. `ExistsValueSet` appears 0 times in 0.1.5 and 8 times in
  0.1.17, both of which are already in the local cargo registry cache.
- Upstream is very much alive: 23 releases since February, 0.3.1 published 2026-08-14,
  external bug reports typically closed in hours to days.
- `parity_cert` mode is default-on and CAN be disabled by a consumer, but doing so does
  not change routing and triggers full in-memory hydration of the 7.4 GB database. It is
  strictly worse. Do not reach for it.

**Caveat that must not be lost:** all of the above came from investigation lanes whose
*verification* lanes died on a usage limit. The escape-hatch result is directly measured
and solid. The upstream claims are UNVERIFIED — confirm before acting on them, especially
before proposing a version bump.

A pin bump 0.1.5 → 0.1.17 is the natural follow-up, but it is a separate, carefully
tested change: the live DB is 7.4 GB and is the only surviving copy of **6,752 of its
12,722 conversations**. Test any upgrade against a copy, never the live archive.

## Still open

- Beads `-1a7mk` (coverage-floor regression, three call sites at `src/lib.rs:65457`,
  `:15283`, `:23747`, plus the `.unwrap_or_default()` single-source defect) and the
  missing detectors — `connector_coverage` appears in ZERO test files.
- `-nvq59`'s `cass doctor` half is deliberately still open; the `status --json` half
  landed at `447d97fe` and is **not deployed** (HEAD still carries the coverage-floor
  regression, so a HEAD build reintroduces `-1a7mk`).
- `-a4xe1`: `golden_robot_json` 9-of-37 red on main since 2026-08-10. Do NOT regenerate
  goldens on macOS.
- `-qtn0e`, the data-loss hazard, is **unanswered**. The parent ran a full rebuild
  against a copy to find out whether a rebuild drops source-absent rows; it wedged in
  `preparing` for the full 40-minute bound and never rebuilt anything, so the
  "source-absent rows SURVIVED, delta 0" line in that log is a null result, not a pass.
  **Re-run it with the skip var set** — that is now a cheap experiment and it is the
  load-bearing question for whether `--force-rebuild` is ever safe here.
- The mini as a source (`cass sources list` is `total: 0`) and scheduling/freshness.
  There is no supported watcher-install command since the 2026-05-17 upstream reset;
  that is Dale's decision, not a bug to fix.

## Environment facts that cost an hour each

1. The build needs nightly, and calling nightly cargo by absolute path is NOT enough:
   ```bash
   export PATH="$HOME/.rustup/toolchains/nightly-aarch64-apple-darwin/bin:$PATH"
   CARGO_TARGET_DIR=/tmp/cass-repair-target cargo check --all-targets
   ```
2. Never share a `CARGO_TARGET_DIR` between checkouts of this crate.
3. `--json` sets robot mode, which hard-codes the log filter to `error` and **ignores
   `RUST_LOG`**. Add `--verbose` or you will conclude an instrument is silent when it
   is only suppressed (`src/lib.rs:5769-5775`).
4. Repo rules: no `rusqlite` in new code ever; work on `main`; after pushing `main` also
   `git push origin main:master`.

## Evidence

`thoughts/shared/handoffs/20260814-cass-repair-to-green/` — `agent-log.md`,
`lanes/backfill-falsifier.md` (the falsifier and the refuted hypotheses; its root-cause
section carries the wrong attribution corrected above), and the six grounding lanes.
Backup: `~/backups/cass/agent_search-20260814-vacuum.db`, 3.98 GB, verified.
