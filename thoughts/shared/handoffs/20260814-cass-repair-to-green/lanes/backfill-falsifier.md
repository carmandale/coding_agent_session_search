# Lane — backfill runbook Steps 1–2, executed

Owner: agent-config session `a91c2501` (coordinator), 2026-08-14.
Runs on the **installed pre-fix binary** `/Users/dalecarman/.local/bin/cass`,
sha256 `3d044227…`, verified byte-identical to `cass.pre-coverage-floor-20260601`
immediately before the run. `cass 0.6.9`. No `CASS_*` env set.

This lane executes `lanes/backfill-mechanics.md` §9 Steps 1 and 2 and reports what
came back. **The headline is negative: the backfill as designed is blocked.**

---

## Step 2 — the manifest reconciles exactly

Built read-only from `~/.codex/sessions` differenced against
`conversations.source_path` for `agent_id=3`, excluding anything with
`mtime > last_indexed_at` (1784200805044):

```
4895 files 13.12 GB
  indexed-already: 3058   newer-than-LAST (step 4's job): 2342
  smallest: 344 bytes        rollout-2025-04-16-fef501d1-….json
  largest : 2570897520 bytes 2026/04/18/rollout-2026-04-18T08-52-28-….jsonl
```

The runbook predicted `4895 files 13.12 GB` and said a materially smaller number
would mean someone had already run part of this. It matches to the file. **Nobody
has run any of it**, and the 3,058 already-indexed codex rows are the same 3,058
the grounding lanes counted.

Manifest written to the session scratchpad rather than `/tmp` (agent instructions
prefer it); path is not load-bearing.

## Step 1 — the falsifier reproduces `-81z91` Run B

One file. The **smallest** file — 344 bytes.

```
cass index --watch-once <one 344-byte file> --json --progress-interval-ms 5000
```

It never left `phase="preparing"`. `phase_code` stayed 0, `total` 0,
`discovered_agents` 0, for the entire run. `stall_detected` fired at its 120 s
timer carrying an all-zero, all-null payload — it is a bare timer with no
diagnostic content, so it names nothing.

`index-run.lock.meta` is the more honest instrument, and it says the run never
started:

```
job_kind=lexical_refresh
started_at_ms      =1786743008046
last_progress_at_ms=1786743008046      <-- identical, never advanced
updated_at_ms      =1786743327824      <-- heartbeat keeps ticking
```

The heartbeat advances while progress does not, which is exactly the shape that
makes this look alive to any watcher that reads `updated_at_ms`.

**This is the runbook's own stop condition.** Its rule was: 30–120 s startup means
the plan holds; beyond ~15 minutes with no phase advance means Run B is reproduced
against the live archive and the whole backfill is blocked behind it. That is what
happened, on the cheapest possible input.

## What it is NOT — three hypotheses, each tested and killed

Recorded because each one is the obvious next guess and each is wrong.

**1. Not the raw-mirror walk** (the `-nvq59` defect reached by a third entry
point). `lsof` across three probes, six seconds apart: **zero** raw-mirror file
descriptors, at any point. The only files open are the DB, its WAL, and the run
lock. The 21 GB / 251,208-file mirror is untouched by this path.

**2. Not a missing index on the startup query.**
`explicit_watch_once_root_unchanged_after_last_index`
(`src/indexer/mod.rs:21158-21201`) filters `conversations` on
`(source_id, source_path)`, and the known unique index is on
`(source_id, agent_id, external_id)` — a different column set, which looked like a
full table scan per trigger path. It is not: `idx_conversations_source_path`
exists and `EXPLAIN QUERY PLAN` confirms
`SEARCH conversations USING INDEX idx_conversations_source_path`.

**3. Not the freelist.** The live DB is **38.7 % free pages** — 748,333 of
1,935,327, about 3 GB — with `auto_vacuum=0`, so freed pages accumulate forever.
Read-only surfaces (triage/health, 40 ms) never allocate pages while an index run
opens read-write, which made a freelist walk a strong candidate.

Killed by differential: the identical binary, on the identical file, against the
**VACUUMed copy with `freelist_count = 0`**, spins the same way — four minutes in
`preparing` with no advance. Same content in both (12,722 conversations, 580,374
messages), only the freelist differs.

Both specimens stat'd at run time, per `.claude/rules/instrument-labels.md`:

| specimen | bytes | freelist | behavior |
|---|---|---|---|
| live `agent_search.db` | 7,927,099,392 | 748,333 | spins, no advance |
| scratch copy of the VACUUM backup | 3,984,084,992 | 0 | spins, no advance |

## What it actually looks like

- **CPU-bound, single core.** `STAT=R`, ~100 % of one core, no I/O in flight.
- **~6 GB resident regardless of database size.** The live run and the 3.7 GB
  vacuumed run both climb to ≈6.0 GB. RSS therefore does not track the file, and it
  cannot be the SQLite page cache — `cache_size` is the default 2,000 pages (8 MB).
  Both databases hold *identical content*, which is the thing that matches: this is
  the corpus being materialized in memory, roughly 6 GB for 580,374 messages, to
  index one 344-byte file.
- **It does move.** The live run's RSS fell 5.72 GB → 1.77 GB at about eight
  minutes and CPU kept accumulating, so it completed one stage and freed it. It is
  grinding, not deadlocked. Nothing had advanced the public phase by 15 minutes.
- **A full lexical rebuild is not the explanation.** `.lexical-refresh-ledger.json`
  records the last full rebuild of this whole corpus at **12,438 ms** — 12 seconds
  for 12,722 conversations / 579,776 messages — and the generation manifest reads
  `build_state: validated`, `publish_state: published`. Whatever costs minutes here
  is not the rebuild that costs twelve seconds.

## Root cause — found, and it is in frankensqlite

The falsifier was only supposed to answer yes/no. It also produced the mechanism,
because a **closed** bead named the instrument that finds it.

`coding_agent_session_search-ibuuh.29.1` — *"Eliminate the single-core 'preparing'
plateau"*, P0, **closed 2026-04-22**. Its closing note cites `CASS_PREP_PROFILE`
and says *"Startup timing test proves bounded first-batch delivery."* That proof
was a timing **test**. On the live archive the plateau is not eliminated, which is
the same fixture-too-small false green already recorded against the latency test.
**This bead should be reopened.**

`CASS_PREP_PROFILE=1` alone emits nothing, and that silence is not evidence:
`--json` sets robot mode, which hard-codes the filter to `EnvFilter::new("error")`
and **ignores `RUST_LOG` entirely** (`src/lib.rs:5769-5775`). `--verbose` overrides
it even in robot mode. With `--json --verbose` the run talks.

What it says, one WARN line, whose `trace_id=49 decision_id=98` is exactly the
trace of the b-tree descent:

```
WARN statement: execute_statement_dispatch: using in-memory fallback path
  while parity-cert mode is enabled
  backend_kind="mem" mode="parity_cert" strict_reject=false
  decision_reason="correlated_exists_fallback" statement_kind="select"
```

Immediately before it: `vdbe.order_by.index_bypass table=messages
index=sqlite_autoindex_messages_1 covering=true`. Immediately after it: a
**sequential page-by-page b-tree descent** — `issued best-effort btree prefetch
hint page_number=936707, 936708, 936709 …` — climbing toward the 972,677-page end
of the file. One statement, walking essentially the whole database.

So frankensqlite declines its normal backend for this statement, falls back to an
**in-memory** path, materializes the table (the ~3.6–6 GB resident), and walks the
b-tree page by page.

**The statement is a full aggregate over every message**, run unconditionally
during lexical-rebuild prep by
`raise_lexical_rebuild_footprints_to_exact_message_counts`
(`src/storage/sqlite.rs:7456`), reached from
`list_conversation_footprints_for_lexical_rebuild` (`:7306`, line 7383):

```sql
SELECT conversation_id, COUNT(*) AS message_count
FROM messages
GROUP BY conversation_id
ORDER BY conversation_id ASC
```

The contrast is the finding. Same file, same query, same index:

| engine | time |
|---|---|
| stock `sqlite3`, `SCAN messages USING COVERING INDEX sqlite_autoindex_messages_1` | **0.03 s** |
| frankensqlite, in the shipped binary | **> 20 min, did not finish** |

Both engines *choose the same covering index* — frankensqlite's own
`index_bypass` line names it. It then does not use it.

Two honest limits on this attribution. The engine's `decision_reason` string reads
`correlated_exists_fallback` while the cass statement above is a `GROUP BY`
aggregate, so either that label is the dispatcher's category rather than a literal
description, or a second statement is involved; separating those needs one more
probe inside frankensqlite. And `parity_cert` appears **nowhere in cass's source**
— it is frankensqlite's own internal mode, not a knob cass sets or can unset.

Per this repo's AGENTS.md the sanctioned response to a frankensqlite defect is a
targeted reproducer filed against frankensqlite, not a bypass in cass. There is
also a cass-side question worth asking independently: this aggregate refines
already-estimated footprints into *exact* counts, unconditionally, at startup —
`.claude/rules/right-sized-mechanism.md` is the lens for whether a full pass over
580,374 messages is the right size for that job.

## Consequence for the goal

The backfill cannot be run by the route the runbook specifies, and the blocker is
not disk, not usage, and not authorization — all three of which were cleared first.
Batching does not rescue it: the runbook's own model was `batches × startup`, and
startup does not terminate. Twenty batches of a cost that never completes is still
never.

`--watch-once` is the only path that takes explicit file paths, and explicit file
paths are mandatory here — day-directories mint non-canonical `external_id`s that
silently duplicate against `idx_conversations_provenance` on any later scan.

The remaining candidate is plain `cass index`, which uses the streaming producer
(`conn.scan_with_callback`) rather than watch-once's whole-root `Vec`, and which
the runbook already designates as a different and safer code path. It has not been
tested against the hole. It also does not obviously reach these files: they exist
precisely *because* the global watermark advanced past them, which is the defect
the coverage floor was built to fix — so the two remaining paths to a current index
are (a) the streaming producer if it can be pointed at them, or (b) the
coverage-floor fix, which is itself blocked on the still-unexplained regression.

**Filed, not resolved.** The falsifier bought the answer the runbook said it would
buy, and the answer is no.
