---
generation: 12
parent-session: a91c2501-1830-4d3d-9430-3c9afe08a63c
next-action-class: executable
---

# Generation 12 — cass is not broken; the SQLite engine it is pinned to cannot run a GROUP BY

## The goal and authorization, verbatim

Dale, 2026-08-14:

> /my-way fix cass to completion and 100% green working state and completely up to date or tell me why it can't or /grill-me with any questions.

Mid-work correction, same day:

> make sure that you are looking at the recent (last 2 weeks) work on cass and not regressing

Dale, 2026-08-15:

> your usage is good now. finish this to completion

Dale, 2026-08-16:

> if your senior dev recommendation is to delete those 4 stale directories do it. if that is what progresses us toward the goal. and I would prefer you just do that and only stop when it would break the pipeline or running sessions or if there is a true blocker or ambiguity or conflict rather than sitting here for something that I am going to just ask your recommendation on and agree to

And the message that produced this generation's finding — Dale, 2026-08-16:

> there has been SO MUCH WORK done on cass and it feels like you agents are all wondering around blind folded in a mine field in a lava flow. can I PLEASE have an agent like Opus 5 on high thinking....oh, that's what you are... not run around incircles playing whac-a-mole? what the heck?!

**No approval carries forward.** You may not file anything upstream, delete any
file, force-push, rewrite history, change repo visibility, or run
`cass sources agents exclude` (which would destroy 3,877 conversations that
exist nowhere else). The two approvals granted on 2026-08-16 are spent: the disk
reclaim is done, and the upstream filing was investigated and correctly not
exercised.

## The finding

`cass search` hangs because **fsqlite cannot execute the lexical-rebuild
planning query at this archive's size.** Not a cass bug, not corruption, not the
catch-up's fault.

The hang is one named step: `plan_lexical_shards`
(`src/indexer/mod.rs:18366`, entering
`plan_lexical_rebuild_shards_from_storage_with_settings` at `:9127`). The prep
profile prints *after* each step, and the live run printed
`load_checkpoint_state` and nothing since.

| step | control (12,722 conv, 3.98 GB) | prod (27,441 conv, 22 GB) |
|---|---|---|
| `open_readonly` | 0 ms | 0 ms |
| `prepare_db_state_deferred_fingerprint` | 0 ms | 0 ms |
| `load_checkpoint_state` | 0 ms | 0 ms |
| **`plan_lexical_shards`** | **4,202 ms** | **7h26m+, never returned** |
| `restart_from_zero_reset` | 0 ms | not reached |
| `start_packet_producer` | 0 ms | not reached |
| `persist_initial_checkpoint` | 10 ms | not reached |

Source: `/tmp/cass-ibuuh-probe/{control,prod}.err`, session `ibuuh`'s probe with
`CASS_PREP_PROFILE=1`. The control run went on to publish a valid generation
manifest (`gen-000001a00bb58d3e-content-v1:12722`, 579,776 docs). **The rebuild
works at 12,722 conversations and stops working at 27,441.**

## Stock SQLite does the same work in 77 milliseconds

`sqlite3` 3.54.0, read-only, against the live 22 GB archive:

```
row counts (27441 conv / 2335514 msg / 27441 tail_state)      33 ms
covered_sql LEFT JOIN                                          30 ms
SELECT conversation_id, COUNT(*) FROM messages
  GROUP BY conversation_id ORDER BY conversation_id ASC        77 ms
```

Same three against the 3.98 GB control: 21-23 ms.

So fsqlite 0.1.5 is ~200x slower at control size, and at prod size the ratio is
at least 347,000x. 2.16x the conversations bought at least 6,300x the time.

## It is grinding, not deadlocked

`ps -M -p 38174`: **434m35s of CPU over 446m elapsed**, 97.4% utilization, one
thread at 99.7%, RSS flat at 25 MB. Executing, not blocked on a lock, not
leaking. Two `read-witness cap reached on cursor ... cap=16384` warnings
(root_page 72 and 14) place it inside fsqlite's per-page snapshot-isolation
witness tracking during a scan.

This distinction matters and was worth measuring: a deadlock and a 350,000x
slowdown are different defects with different fixes.

## What this rules out

- **The archive is not corrupt.** Stock SQLite reads every table, correct counts,
  milliseconds.
- **cass's rebuild logic is not wrong.** Identical code path completes on the
  control and publishes a valid manifest.
- **The catch-up did not break anything.** It produced a healthy 27,441-
  conversation archive that merely crossed the size where the pinned engine stops
  finishing.
- **Not a missing index.** `messages` carries `UNIQUE (conversation_id, idx)` →
  `sqlite_autoindex_messages_1`, covering the GROUP BY's leading column. Stock
  SQLite uses it. `conversation_tail_state` has no index at all, but its LEFT
  JOIN is 30 ms in stock.

## The pin ceiling was false in a second, dumber way

Already recorded: the rustc 1.95 barrier is transitive
(`fsqlite 0.1.19 → asupersync 0.3.10 → sysinfo 0.39.6`), not in fsqlite's own
manifest.

New: **`nightly-2026-08-10` is rustc 1.99.0-nightly and has been installed on
this machine the entire time.** Every handoff in this chain lists
`$HOME/.rustup/toolchains/nightly-aarch64-apple-darwin` as "environment fact
#1"; that resolves to a **1.94 nightly from December 2025**. Each generation used
it faithfully. Nobody ran `rustup toolchain list`.

**cass builds clean against fsqlite 0.1.19 on that toolchain** — `rc=0`, 6m30s,
release binary at `/tmp/cass-0119-target/release/cass`. Two guards must move
together, and both bit:

- `build.rs` `expected_version`, two sites (lines 56 and 74), exact string compare
- `Cargo.toml` lines 45 and 181, spelled `0.1.19` with **no `=` prefix** — the
  guard compares literally, so `=0.1.19` fails against expected `0.1.19`

## But the pin bump does NOT fix the performance

Controlled probe, `/tmp/fsq-probe/` — 40 lines, opens a database and runs the two
queries. Against an APFS clone of the control database:

| | stock sqlite3 | fsqlite 0.1.19 |
|---|---|---|
| open | — | 15 ms |
| Q1 `LEFT JOIN` | 23 ms | 104 ms |
| Q2 `GROUP BY` | 21 ms | **4,880 ms** |

fsqlite 0.1.5 spent 4,202 ms on the whole step; 0.1.19 spends 4,880 ms on these
two queries alone. **At control scale 0.1.19 is not an improvement.** Whether it
diverges favourably at prod scale was still running when this was written — see
"Open" below. Do not assume either way; read the log.

Note for anyone rerunning this: `fsqlite = "0.1.5"` resolves its own internals
forward (`fsqlite-pager`/`fsqlite-btree`/`fsqlite-mvcc` all came out at 0.1.19),
so a naive "0.1.5 vs 0.1.19" probe A/B is **not** clean. Check `Cargo.lock` for
`fsqlite-core`'s resolved version before quoting a control number.

## The exact next actions

1. **Read `/tmp/fsq-prod-0119.log`.** It holds the 0.1.19 prod-scale `GROUP BY`
   result. If it completed in minutes, the pin bump is the fix and the path is
   0.1.19 + bead `759l7`. If it did not, go to 2.
2. **Test upstream's current line.** A 0.3.4 probe is building at
   `/tmp/fsq-probe-034/`. 0.1.19 is old; upstream shipped through 0.3.4. If 0.3.4
   fixes the query, the pin target is 0.3.4, not 0.1.19.
3. **If no released fsqlite fixes it**, this is a genuine upstream performance
   defect with a 40-line standalone reproduction already written
   (`/tmp/fsq-probe/src/main.rs`) — which is exactly the clean reproduction the
   generation-11 handoff said was missing. **Filing needs Dale's approval; it is
   not inherited.**
4. **Bead `759l7` remains real** regardless: three hand-rolled spin-waits
   (`update_check.rs:852`, `search/model_download.rs:1022`,
   `pages/deploy_cloudflare.rs:843`) self-deadlock under asupersync 0.3.4+. The
   third has no test coverage and sits on a **CLI startup path**, so set
   `CASS_SKIP_UPDATE=1` when testing any 0.3.x-linked binary.

## Safety notes learned this generation

- `cass search` with no override writes to the **live** archive's index and takes
  its `index-run.lock`. Use `CASS_DATA_DIR=<scratch>` (`src/lib.rs:81214`) and/or
  `--db`. A run of mine did take that lock (it had gone genuinely stale, owner pid
  33179 absent) and was stopped before any staged shard published; live
  `index/` mtimes were verified unchanged afterward.
- Probe against **APFS clones** (`cp -c`), never the live file, since
  `fsqlite::Connection::open` is read-write. Clones of the 22 GB + 3.98 GB pair
  cost about 10 GB of real space here, not zero — watch the disk floor.
- fsqlite's API is not rusqlite's: `Statement::query()` takes no arguments and
  returns an eager `Vec<Row>`; `Row::get(usize) -> Option<&SqliteValue>`.

## State

- `main` is green on fsqlite 0.1.5 and is still the shipping pin. Do not merge
  `worktree-cass-gen5-honesty`.
- Open beads: `p3kgr` (P0, carries this root cause as a comment), `759l7`,
  `9fnbr`, `qtn0e`.
- Session `ibuuh` had a 7-hour probe live against the archive at the time of
  writing; check `claude agents` and `lsof` before assuming a lock or a dirty
  file is yours.
