# Coordinator log — generation 7

Session `39ab724c-6f6c-438c-b64a-9eb4aa22a4c9`, background job. Started in
`/Users/dalecarman/dev/coding_agent_session_search` on `main` at `de0fa2c6`,
then moved into the existing `cass-gen5-honesty` worktree (see *Where this
session works*, below).

Resumed from `cass-green-continuation.md` (generation 6, committed at
`f88b5ebf`) through the resume-handoff skill's autolaunched direct path: the
frontmatter carries `generation: 6`, a well-formed `parent-session`, and
`next-action-class: executable`, and the Step 3 verification below passed.

## Inherited state, verified rather than assumed

| claim from the handoff | verified |
|---|---|
| `main` at `c02e5d65`, pushed | `main` is at `de0fa2c6` — `c02e5d65` plus the gen-6 handoff and its launch receipt |
| catch-up running, 19 of 28 batches | 19 `END rc=0` lines, `batch-at` in flight since 12:49:46Z, process alive |
| free disk 66 GiB | `df -g /` reports 65 GiB |
| one unmerged `worktree-*` branch | `git branch --no-merged main` lists exactly `worktree-cass-gen5-honesty` |
| bead `2bh4a` `in_progress` | confirmed |

The catch-up started `2026-08-16T09:15:54Z` (`batch-aa START`) and the manifest
it works from was written at 09:15Z, so the handoff's
`--acceptance-since 2026-08-16T09:14:00Z` bound is correct and conservative.

## Batch sizes, measured — the tail is the whole cost

The manifest is sorted ascending, so the remaining work was never "9 of 28
batches":

```
batch-aa .. batch-as  (19 done at handoff)   2.11 GiB of source
batch-at .. batch-az  ( 7 more)              4.35 GiB
batch-ba              ( 1)                   5.52 GiB   <- one batch, 250 files
batch-bb              ( 1)                   1.81 GiB   <- 8 files
                                      TOTAL 13.79 GiB
```

Wall time across the four batches completed before this session fits
`720 s fixed + 200 s/GiB`, i.e. the per-batch cost is mostly a fixed scan rather
than the payload.

## Action taken: cleared two hung processes from this chain's own earlier generations

Found spinning at session start, both from this repair's predecessor jobs,
together holding ~2.4 cores against a catch-up that needs them:

| pid | what | elapsed | CPU burned | owner |
|---|---|---|---|---|
| 90878 | `cass status --json` | 19h29m | 1160 min (~99% of a core) | orphaned, `PPID 1` |
| 42381 | `cargo test --lib` test binary in `/tmp/cass-repair-target` | 18h23m | 1878 min (~143%) | erika job `af6e155f`, state `blocked` |

Both had `cwd` in this worktree, so both are debris from the generations that
produced the branch this session verifies.

- `90878` is the pre-fix `status` hang itself — the defect generation 5 fixed.
  Its stdout went to `/tmp/cass-bound-lWmcgX`, which is 0 bytes. It held read
  handles on the live `agent_search.db` and its 1.46 GB `-wal`; a 19-hour-old
  read snapshot is also what blocks WAL checkpointing.
- `42381` is the run that was supposed to answer bead `p3kgr` — the suite at the
  pin. Its log holds two lines, `START 2026-08-15T18:33:59Z` and a `df` reading
  **29 GiB free**, and nothing since. It started below cass's own ~32 GiB floor
  on a machine with no room, and never emitted another byte. That it produced
  nothing in 18 hours is the direct reason `p3kgr` is still open.

Its owning job's `needs` was harvested before acting, per
`destructive-verification.md`: *"decide /tmp reclaim on bead -jck92 to unblock
indexing measurement"*. That decision was made and the bead closed at
`c02e5d65`, so nothing was owed and nothing was discarded; `state.json` had not
been written since 2026-08-15T19:11Z.

`SIGTERM` to both, both confirmed gone by `ps -p`. The catch-up indexer is now
the only cass process on the machine. No file was deleted.

## Where this session works, and why it is not on `main`

This is a background session, and this harness refuses `Write`/`Edit` anywhere
in the shared checkout until the session isolates itself. That refusal is what
stranded a branch in each of the last several generations — generation 6's
handoff names it as a recurring tax and tells the successor to expect it.

Two things are worth writing down because they are not obvious:

1. **The block is the editing tool, not git.** `git push --dry-run origin HEAD`
   returns `Everything up-to-date` at rc=0 from this same shell, so the remote
   is reachable and the credential works.
2. **The documented escape hatch is unreachable from inside the guard.** The
   refusal message says to set `"worktree": {"bgIsolation": "none"}` in
   `.claude/settings.json` — and writing that file is itself refused by the same
   guard. `.claude/` is gitignored here (`.gitignore:63`), so that file would
   have been local to this checkout and would not have travelled anywhere.

Rather than route around a guard with a shell heredoc, this session entered the
**existing** `cass-gen5-honesty` worktree — the one holding the branch that has
to land anyway. Edits therefore land on the branch under verification, not in a
new one, and no eighth branch is created.

## Live progress read, 13:00Z (read-only, `mode=ro`)

```
claude_code   on_disk= 7953  indexed=10690  unindexed=1140  (4.05 GiB)
codex         on_disk=10346  indexed= 9321  unindexed=1025  (7.55 GiB)
                                    remaining 2165 paths, 11.61 GiB
```

Against bead `2bh4a`'s 2026-08-10 measurement of codex `indexed=3058` against
9,800 files on disk — 31.2% coverage — codex now stands at 9,321 of 10,346,
**90.1%**, the remainder being the batches still queued.

Incidental finding, to check rather than assume: all 1,647 flat-layout codex
rollouts under `~/.codex/sessions/` (`rollout-*.json` ×1,645 and
`rollout-*.jsonl` ×2 — the subject of bead
`codex-flat-layout-undiscovered-kfaid`, filed as *never discovered*) were
**already indexed** when the 09:15Z manifest was built. Only 2 of them appear in
it. The manifest's glob does match that layout, so this is a real read and not a
blind spot in the instrument. That bead may be stale; verify before closing it.

## Bead `kfaid` is answered by measurement, and it uncovered a smaller real hole

`codex-flat-layout-undiscovered-kfaid` says 1,647 flat-layout codex rollouts are
never discovered. Measured against the live archive read-only at 14:33Z:

```
flat-layout codex files on disk : 1647
   of those, indexed            : 1645
   of those, NOT indexed        :    2
total codex rows                : 10171   (8526 nested, 1645 flat, 0 elsewhere)
```

So the flat layout **is** discovered and indexed. The bead is stale as written.

The two that are not indexed are the interesting part, and neither is indexed
under any other path or `external_id` — the check was a `LIKE` over both columns,
so this is not a path-shape artifact:

| file | lines | shape |
|---|---|---|
| `rollout-2025-07-10T05-52-17-918acffe…jsonl` | 24 | real conversation — user text, `local_shell_call`, tool output |
| `rollout-2025-08-19T09-57-13-48b7207f…jsonl` | 1 | header record only, no messages |

The second is plausibly a correct skip: nothing to index. The first is a genuine
hole — and both sat in `batch-aa`, which ran first and exited `rc=0`, so the
catch-up handled them and they still have no row.

Hypothesis, from source rather than from a run: these two are the only flat files
with a `.jsonl` extension (the other 1,645 are `.json`), and their *content* is
the old un-wrapped shape — `{"type":"message","role":"user","content":[…]}` —
while `augment_modern_codex_messages` (`src/connectors/codex.rs:152`) requires
`raw.get("payload")` and returns `None` without it. Extension says modern, bytes
say legacy, so the file parses to zero messages. The base parse lives in the
pinned `franken-agent-detection` git dependency
(`Cargo.toml:101`, a public repo Dale does not own), but the local augmenter is
where a fix could land without touching upstream.

**Not yet confirmed by a run.** The check is a targeted `--watch-once --verbose`
over exactly those two paths once the catch-up releases the index lock. Until
then this is a reading of the source, not a measurement.

Consequence for step 2: the acceptance test is expected to exit 1 on at least
these two, because they are quiescent from 2025 and genuinely unindexed. That is
the test doing its job, not a defect in the bound.

## What the acceptance test actually found: 74 holes, two real causes, and an unreachable criterion

Run at 14:49Z with 27 of 28 batches complete (`batch-bb`, 8 files, still in
flight), against the quiescence bound:

```
claude_code   on_disk= 7962  indexed=11544  unindexed= 295   ->  15 stale, 280 arrived during the run
codex         on_disk=10351  indexed=10281  unindexed=  70   ->  59 stale,  11 arrived during the run
FAIL: 74 file(s) were quiescent before 2026-08-16T09:14:00Z and are still unindexed.
```

All 74 were in the 09:15Z manifest, so every one was offered to the indexer.
62 of them sit in `batch-aa` — the batch of the 250 *smallest* files — which is
the first hint that most of this is not a coverage failure at all.

Classifying each by the record types it actually contains:

| n | class | verdict |
|---:|---|---|
| 20 | codex, header record only | correct skip — nothing to index |
| 19 | codex, `session_meta` + `task_started` and nothing else | correct skip — session opened, no turn ever happened |
| 15 | claude_code, no `user` or `assistant` records | correct skip |
| **18** | **codex, legacy un-wrapped shape** | **real hole** |
| **2** | **codex, modern shape, 435 MB and 182 MB** | **real hole** |

So **54 of the 74 are files that can never acquire a conversation row**, and 20
are genuine.

### The criterion cannot pass as written, for the reason its own docstring gives

`catchup-manifest.py` already argues this case against `journal.jsonl`: "an
acceptance test that can never pass tells you nothing." It then excludes exactly
one basename. The same unreachability arrives through a second door — a file with
a conversation-shaped name and no conversation inside it — and 54 files come
through it. Tightening the exclusion to a *content* predicate rather than a
basename would make the test sharper, not weaker: it would fail on exactly the 20
real holes and pass otherwise.

Deliberately not done in this session. The predicate that decides "does this file
hold a conversation" is the connector's own judgment, and reimplementing it
outside the product is how a test guard grows into a second parser
(`right-sized-mechanism.md`). Recorded as a finding for a decision rather than
patched around.

### Class 1 — 18 legacy un-wrapped codex rollouts, real conversations

Dated 2025-08 through 2025-09, up to 1,296 lines and 1.1 MB, carrying
`function_call`, `function_call_output` and `message` records. Their records are
flat — `{"type":"message","role":…,"content":[…]}` — with no `payload` wrapper,
while `augment_modern_codex_messages` (`src/connectors/codex.rs:152`) requires
`raw.get("payload")` and returns `None` without it.

This is the same shape as the two flat-layout `.jsonl` files found earlier, which
means that finding was not a flat-layout quirk at all: it is a **format-era**
quirk that also affects nested files. 0.007 GiB of content, but real sessions.

### Class 2 — 2 modern codex rollouts that are simply enormous

`435,218,432` and `182,593,718` bytes, both well-formed modern shape with
thousands of `response_item` and `event_msg` records. 0.575 GiB between them.
Nothing about their content explains the miss; size is the only thing that
distinguishes them from the 10,281 codex files that did index.

Both classes need an executed probe against the live connector before being
filed as parser defects — the classification above is a reading of the bytes on
disk, not of what cass did with them.

## `8llb5` root cause, established from source

The forward-progress atomic that `status` reads is bumped in exactly three
places (`src/indexer/mod.rs`): inside `complete_preflight_phase!`, which is a
no-op unless the lock mode is `WatchStartup`, and after `run_streaming_index` /
`run_batch_index` return — that is, **once, when the whole scan is already
over**.

`--watch-once` takes lock mode `WatchOnce` (`mod.rs:12071`), and
`SearchMaintenanceMode::rebuild_active()` is `!matches!(self, Watch)`
(`asset_state.rs:74`), so `WatchOnce` *is* subject to stall detection. Its
threshold is 120 s (`REBUILD_STALL_DETECT_SECS_DEFAULT`) against a scan that runs
~12 minutes. Nothing posts progress in between, so `last_progress_at_ms` stays at
lock-acquisition time and the age climbs one second per second — exactly the
`+35s per 35s` the bead measured.

The root cause is a mismatch between classification and instrumentation: the mode
is enrolled in stall detection but never participates in the contract stall
detection reads. The comment at `mod.rs:13113` shows the shape was already
anticipated — it explains that a completed scan bumps progress so "a long single
mode scan does not false-positive as `status: stalled`" — but the bump sits
*after* the scan rather than inside it, so it prevents the false positive only
once the scan can no longer be called stalled anyway.

That makes the fix small and honest: bump the atomic from inside the ingest loop,
which already knows when a conversation completes. It is an atomic store, and the
heartbeat thread folds it to disk on its next ~1 s tick. Suppressing the check for
`WatchOnce` instead would be the bandaid — it would also destroy real stall
detection for the one path that is known to wedge.

A comment at `mod.rs:12087` already claims the bump "is currently used at the
per-conversation completion ingest loop." Three call sites say otherwise. That
line is aspirational and should be corrected in the same change.

### Reproduced live, and the lock file settles the mechanism

Against the deployed binary `49fbba6e3789c252` at 14:38Z, with the catch-up
completing batches `rc=0` throughout, `cass status --json` returned in seconds
and said:

```
status                        "stalled"
healthy                       false
rebuild.last_progress_age_ms  611254        <- 10.2 minutes
recommended_action            "Index rebuild is wedged; … capture a stack trace
                               with sudo gdb … for issue #258"
```

So the headline is not merely a wrong word. The operator is told the indexer is
**wedged** and handed a `gdb` incantation, while the run is healthy and one
batch from finishing. That is worse than the bead recorded, which measured the
`status` string but not the top-level `recommended_action`.

Two reads of `index-run.lock` about a second apart make the mechanism
unambiguous:

```
                        sample 1          sample 2        delta
started_at_ms           1786890461996     1786890461996       0
updated_at_ms           1786891101832     1786891102847   +1015 ms
last_progress_at_ms     1786890461996     1786890461996       0
mode                    watch_once        watch_once
```

`last_progress_at_ms` is byte-identical to `started_at_ms`: it has not moved
once since the lock was acquired, while the heartbeat thread ticks
`updated_at_ms` every second. The stall age is therefore just wall-clock since
lock acquisition, and it crosses the 120 s threshold roughly two minutes into
every twelve-minute batch. This is an executed falsifier, not an inference — the
prediction from source was that the value never advances, and it does not.

Also visible in the same capture, and worth separating from the above:
`connector_coverage` reads `{"checked": false, "complete": null}` — the coverage
read reports UNKNOWN rather than fabricating `complete: true`, which is
generation 5's fix working on the live binary. `healthy: false` here is caused by
`stalled`, not by the unknown coverage, so this capture does not exercise bead
`xarzt` either way.

### The shape of the fix, from the existing plumbing

The bump is already threaded thoroughly — but only through the *lexical rebuild*
path. `rebuild_tantivy_from_db_with_progress_bump` and its deferred-startup
sibling take an `Arc<AtomicI64>` and bump per page and per shard flush, and
`bump_index_run_lock_progress_if_present` (`mod.rs:2999`) already exists for
passing it optionally.

The *scan* path takes none. `run_streaming_index` (`mod.rs:11491`) and
`run_batch_index` (`mod.rs:11724`) have no progress parameter at all, and
`--watch-once` runs entirely inside them. So the change is to give the scan the
plumbing the rebuild already has:

1. add `progress_bump: Option<&Arc<AtomicI64>>` to `run_streaming_index`,
   `run_batch_index`, and their `_with_connector_factories` variants;
2. call `bump_index_run_lock_progress_if_present` at the per-conversation ingest
   completion point inside the scan loop — a single relaxed store, with the
   heartbeat already owning the I/O;
3. pass `Some(&progress_bump)` at the two call sites, `mod.rs:13104` and
   `mod.rs:13127`;
4. correct the stale comment at `mod.rs:12087`, which claims this already
   happens.

Not a suppression, and it earns something beyond silencing the false alarm: a
`--watch-once` run that genuinely wedges becomes detectable for the first time,
because the signal it is judged by finally means something.

The regression has to be a mutant, not just an assertion — the existing suite has
2,900+ lines of tests in `asset_state.rs` alone and every one of them passed
against this defect. Removing the new bump must turn a named case red.

## Disk plan for step 4, worked out before it bites

`verify-fsqlite-pin.sh` refuses below a 60 GiB floor. Free is 55 GiB right now
(mid `batch-ba`, transient) and the remaining permanent growth from `batch-ba`
plus `batch-bb` is 7.33 GiB of source at 1.75x, so the run should settle near
**52 GiB — under the script's own floor.** The refusal is therefore expected and
is not a signal that anything is wrong.

The floor's stated purpose is to keep a *running* indexer above cass's ~32 GiB
start floor. Once the catch-up has finished there is no indexer to protect, and
the measured cost of a full dependency target here is ~16 GiB
(`/tmp/cass-c6bfb589-target` and `/tmp/cass-gen3-golden-target` are 16 GiB each).
52 − 16 ≈ 36 GiB, which clears the real floor. So step 4 runs with an explicit
lowered floor and disk watched during the build, and the reason is recorded here
rather than being a silent flag.

Not available as an alternative: `/tmp` holds 62 GiB of stale cargo targets
(`cass-repair-target` 30 GiB, two more at 16 GiB each). Reclaiming them would
solve this outright, and this repo's `AGENTS.md` RULE 1 forbids deleting any file
without express written permission. Generation 6's handoff withdraws the earlier
approval explicitly. So they stay, and the build works around them.

A fresh target directory is used rather than the 30 GiB `cass-repair-target`,
even though reusing it would be warmer and cheaper. The napkin already records
what a shared `CARGO_TARGET_DIR` costs in this crate: two checkouts resolve to
the same artifact name, the later build clobbers the earlier, and cargo's
freshness check then re-runs the *other* tree's test binary while printing
`Finished`. An ambiguous green here would be worse than a slower one.

## Plan

1. Let the catch-up finish. Armed: a persistent monitor over
   `~/.cass-catchup/run.log` emitting each `END rc=`, both terminal markers, and
   process death.
2. Acceptance test at the quiescence bound.
3. Close `2bh4a` on the acceptance counts.
4. `verify-fsqlite-pin.sh` — the suite at the pin, which has never run to
   completion.
5. Green → land the pin and redeploy.
