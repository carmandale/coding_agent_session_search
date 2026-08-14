# Lane: backfill-mechanics

Read-only grounding lane for the cass repair. Owner: this lane is the only writer of this file.
Date: 2026-08-14. Repo HEAD at start: `37d52925` (`beads(nvq59): the status --json hang is a 20 GB raw-mirror walk`).

Every claim below is marked **MEASURED** (I ran it and read the output, or I read the source at the
cited file:line) or **INFERRED**. Nothing was built, indexed, installed, or mutated. The live archive
was opened only through `file:...?mode=ro`.

---

## 0. Headline

**The two codex gaps are one defect, and the recovery is one operation, not two.**

Bead `-kfaid` says 1,647 flat-layout rollouts "are never discovered" and hypothesises that
`CodexConnector::discover_source_files` only walks `YYYY/MM/DD/`. That hypothesis is **refuted at
source** on the currently pinned FAD rev, and refuted again by walk-order arithmetic. The flat files
were skipped for exactly the reason the nested tail was skipped: the 2026-06-01 scan aborted at
sorted walk position ~1,554 of 10,290, and every flat file sits at position ≥ 8,643.

**The recovery is safe to run today on the installed pre-fix binary, with two changes to the command
the parent bead recommends.** Pass *file* paths, not day-directories — day-directories mint
non-canonical `external_id`s that will duplicate on any future scan, and they defeat the built-in
resume-skip. And take a `VACUUM INTO` backup first, because 3,877 indexed Claude Code conversations
exist **only** inside the archive; their source files are gone from disk.

---

## 1. Beads read in full

`-codex-coverage-gap-2bh4a` (P0, in_progress), `-codex-flat-layout-undiscovered-kfaid` (P1, open),
`-81z91` (P2, in_progress), `-373b1` (P2, in_progress), `-1vxuf` (P2, in_progress), `-2d37b` (P2,
in_progress), `-1a7mk` (P1, open), `-2rtk7` (closed, spec 017).

### Is the recovery path known-safe, or does it have open defects filed against it?

**It has two open beads filed against `--watch-once`, and neither applies to the recovery as I
specify it below.** That is a source-level finding, not an optimistic reading.

`-81z91` and `-373b1` both describe `cass index --watch-once ~/.pi/agent/sessions` reaching
9.55 GB RSS and wedging at `current=0` before persisting anything. Both name the same two
unbounded decisions, and both are still true at HEAD (**MEASURED**, I re-read them):

- `src/indexer/mod.rs:20901` — `let mut convs = match conn.scan(&ctx)` returns the **whole root** as
  one `Vec<NormalizedConversation>` before the ingest phase begins.
- `src/indexer/mod.rs:20986-20990` — for an explicit watch-once the ingest chunk is forced to the
  entire conversation count:
  ```rust
  let ingest_chunk_size = if explicit_watch_once { conv_count.max(1) } else { watch_ingest_chunk_size() };
  ```
  `CASS_WATCH_INGEST_CHUNK_SIZE` is read only in the `else` branch, so setting it does nothing here.

**What both beads missed is that "the whole root" means one trigger, and the caller controls how many
triggers there are.** `src/indexer/mod.rs:20804` is `for (kind, root, min_ts, max_ts) in triggers` —
scan, provenance, ingest, commit, then the next trigger. And `classify_paths`
(`src/indexer/mod.rs:21513-21593`) keys its batch map on the **explicit path itself** when
`prefer_explicit_paths` is set:

```rust
// src/indexer/mod.rs:21541-21548
let scan_path = if prefer_explicit_paths { p.clone() } else { root.path.clone() };
let mut scan_root = root.clone();
scan_root.path = scan_path.clone();
let key = (*kind, scan_path);
```

So `--watch-once <one root containing 2,077 files>` is **one** trigger holding the whole corpus —
that is the 81z91/373b1 shape. `--watch-once <N file paths>` is **N** triggers of one file each.
Peak working set is bounded by the largest *single trigger*, not by the total. **MEASURED at
source.** This is why the parent bead's 117-file / 5-day-dir fixture completed in 26.8 min while the
pi corpus wedges: 5 bounded triggers versus 1 unbounded one.

`-2d37b` (chipbot symlink) is irrelevant to this lane — the corpus it describes no longer exists on
disk (triage 2026-08-10, cannot reproduce).

`-1vxuf` is three jobs under one number and its own triage says so. The half that touches this lane
is "the watcher is not installed" — `launchctl` has no cass job, so nothing keeps the index current
after the backfill lands. That is a separate deliverable, not a blocker for the run.

---

## 2. Verifying the napkin's numbers

The repo napkin (`napkin.md`, `## Today`) carries five figures. Verdict on each:

| Napkin claim | Verdict | Basis |
|---|---|---|
| 117 hole files / 369.8 MB took 26.8 min | **MEASURED** (by the prior session, not by me) | Bead `-2bh4a` comment 2026-08-10 20:19 records `1609 s`, release binary, quiet machine, `env -i` + `CODEX_HOME` + `--data-dir` isolation |
| grew the data dir 874 MB (2.4x amplification) | **MEASURED** (prior session) | same comment |
| ~12-15 h for the 3,186-file tail | **EXTRAPOLATED**, linear, and the bead says so itself ("Treat both as lower bounds") | 34.1x by bytes → 15.2 h; 27.2x by count → 12.2 h |
| ~24-30 GB added | **EXTRAPOLATED** from the same 2.4x | same |
| stall detector fires 4x during a healthy run that still exits 0 | **MEASURED** (prior session) as an observation; the "harmless" half is now **MEASURED BY ME at source** | see §5 |

**My correction to the extrapolation.** The napkin extrapolates from 3,186 files / 12.6 GB. Measured
today, the true hole is **4,895 files / 13.12 GB** (§3), because the flat-layout 1,647 belong in the
same run. Re-extrapolating on the same linear basis: **15.9 h by bytes, 18.7 h by count.** Still a
lower bound, for two reasons the napkin already names and one it does not: the fixture ran against a
near-empty data dir, while the real run ingests into a 7.93 GB archive with a 1.18 GB lexical index —
and that difference is the entire subject of spec 017.

---

## 3. The hole, measured today against live state

Probe: `os.walk` over `~/.codex/sessions` for `rollout-*.{json,jsonl}`, differenced against
`conversations.source_path where agent_id=3`, read through `file:...?mode=ro`. **MEASURED.**

```
codex rollout files on disk: 10289        (was 9,800 on 2026-08-10 — corpus is growing)
absent from index:            7231   22.03 GB
  true hole (mtime <= last_indexed_at 1784200805044 = 2026-07-16T11:20:05Z):
                              4895   13.12 GB
      flat-layout             1647    0.50 GB
      nested                  3248   12.62 GB
          pre-rebuild tail    3227   12.62 GB   (mtime <= 2026-06-01T11:21:27Z)
          post-rebuild residue  21
  staleness (mtime > last_indexed_at):
                              2336    8.91 GB
```

`4,895 / 13.12 GB` reproduces bead `-2bh4a`'s figure exactly, four days later. The nested tail
spans **119 distinct day directories** (the bead said 108; my window is slightly wider).

Largest day directories in the tail, by bytes: `2026/04/18` 2607.6 MB, `2026/02/22` 1172.3 MB,
`2026/05/18` 660.2 MB, `2026/02/12` 628.7 MB, `2026/05/15` 614.2 MB.

**The single most dangerous file in the run** (**MEASURED**):

```
2570.9 MB  /Users/dalecarman/.codex/sessions/2026/04/18/rollout-2026-04-18T08-52-28-019da0dd-43ca-7b02-bf75-a139709340f9.jsonl
```

That is 3.7x larger than the 690.7 MB the parent bead cites as "the largest successfully indexed
codex file", and it is *inside the hole*. At the 6.5x scan amplification `-81z91` measured
(9.55 GB RSS from a 1.45 GB corpus), that one file is an **~17 GB working set**. Machine has 128 GiB
RAM / 16 cores (`hw.memsize 137438953472`, `hw.ncpu 16`, `Mac16,5` — **MEASURED**), so it fits — but
swap is currently `total 16384M used 15082M free 1301M` (**MEASURED**), i.e. the machine is already
under memory pressure from concurrent agent sessions. Schedule accordingly.

---

## 4. The flat-layout question — `-kfaid` is refuted, and it collapses into `-2bh4a`

This is the question the lane brief called decisive ("if flat-layout rollouts are never DISCOVERED, a
re-scan may not find them at all — this decides whether the backfill is one operation or two").
**It is one operation.**

The FAD checkout is present and is the pinned rev (**MEASURED**):
`Cargo.toml:94` pins `rev = "b62d859709aa6f8e772759efa2c13da9e3c088c9"`; the checkout is
`~/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/b62d859/`.
`git log -L` on that dependency line says the rev was pinned **2026-05-20**, i.e. *before* the
2026-06-01 rebuild — so this is the code that ran.

**Discovery is not depth-limited.** `src/connectors/codex.rs:98-123`:

```rust
fn rollout_files(root: &Path) -> Vec<PathBuf> {
    let sessions = Self::sessions_dir(root);
    for entry in WalkDir::new(sessions).into_iter().flatten() {
        if entry.file_type().is_file() {
            let name = entry.file_name().to_str().unwrap_or("");
            if name.starts_with("rollout-")
                && entry.path().extension()... .is_some_and(|ext| {
                    ext.eq_ignore_ascii_case("jsonl") || ext.eq_ignore_ascii_case("json")
                })
            { out.push(entry.path().to_path_buf()); }
        }
    }
    out.sort();
    out
}
```

No `max_depth`. Both extensions accepted. The only filter is the `rollout-` filename prefix — and
**all 1,647 flat files carry it** (**MEASURED**: `fd -d 1 -t f '^rollout-' ~/.codex/sessions` → 1647,
of which 1645 `.json` + 2 `.jsonl`, which is exactly kfaid's own split).

**Parsing is not the gap either.** `src/connectors/codex.rs:614-640` has a dedicated `.json` branch
reading `val["session"]["cwd"]` and iterating `val["items"]`. I read a real flat file
(**MEASURED**): it is `{"session": {"timestamp","id","instructions"}, "items": [...]}` with content
blocks of `"type": "input_text"`. `flatten_content` → `extract_content_part`
(`src/connectors/utils.rs:206-217`) explicitly accepts `input_text`, and there is a unit test for it
(`utils.rs:448 flatten_content_input_text_block`).

**So why were they absent? Sorted walk order.** `rollout_files` sorts the collected paths, and
`scan_codex_with_callback` iterates that sorted Vec. `'2' (0x32) < 'r' (0x72)`, so every
`sessions/YYYY/...` path sorts before every `sessions/rollout-...` path. Simulated on the real
corpus in sorted order (**MEASURED**):

```
total sorted entries:                        10290
first flat file at sorted position:           8643   (rollout-2025-04-16-0a9c…json)
   preceding entry:                           2026/08/14/rollout-2026-08-14T15-52-43-…jsonl
2026/02/12 occupies sorted positions:         1549 .. 1574
last codex row from the 2026-06-01 scan block (id 7411):
   /Users/dalecarman/.codex/sessions/2026/02/12/rollout-2026-02-12T05-35-04-019c51a2-…jsonl
```

The scan died at sorted position ~1,554 of 10,290. **Every flat file is at position ≥ 8,643.** They
were never reached, so "never discovered" is true as an observation and false as a diagnosis. This
also explains, with no extra hypothesis, the two facts kfaid found puzzling: why the two flat
`.jsonl` are absent alongside the `.json` (they sort in the same block), and why the same full scan
happily ingested nested files with 2025-09 mtimes (position ~0-700, well before the abort).

One incidental correction: the parent bead floats an OOM on the 499.3 MB
`2026/02/12T22-31-52` rollout as the possible abort trigger. Walk order puts the abort at the file
starting `09:33:37`, several entries *before* that one. Whatever killed the connector, it was not
that file being reached.

**Consequence for the runbook:** the flat files need no separate mechanism, no FAD change, and no
second operation. They are 1,647 more paths in the same manifest — and the cheapest, safest 0.50 GB
in the whole run, which makes them a good first batch.

---

## 5. Spec 017 — is the OOM fixed, and would a long run hit it?

**Read:** `specs/017-watch-once-lexical-oom/{spec.md,log.md,workflow-state.md}`,
`specs/018-lexical-refresh-finalization/{spec.md,log.md}`, bead `-2rtk7` (CLOSED 2026-05-17),
`specs/016-cass-recovery-ingestion/` (**MEASURED: an empty shell — `evidence/logs/` and
`evidence/watcher-proof/`, both empty, no `spec.md`; nothing to resume from**).

**The OOM is not eliminated. It is made non-lossy, and the run continues.** That is the honest
answer, and it is better than "fixed" for our purposes.

Spec 017's selected shape is DB-first ingest: persist to SQLite, treat the inline lexical update as a
derived asset that may fail open. The shipped behaviour is at `src/indexer/mod.rs:21030-21056`
(**MEASURED at source**):

```rust
let lexical_update_deferred = chunk_outcome.batch_outcome.lexical_update_deferred;
if lexical_update_deferred {
    tracing::warn!(... "dropping uncommitted watch Tantivy writer after deferred lexical update");
    *t_index_guard = None;
} else {
    t_index_guard.as_mut().expect(...).commit()?;
}
if lexical_update_deferred {
    tracing::warn!("skipping watch last_indexed_at update after deferred lexical update so health/status report stale lexical assets");
}
```

So on a lexical OOM mid-run: the conversation **stays in SQLite**, that chunk's Tantivy writer is
dropped uncommitted, `last_indexed_at` is not bumped, a pending-refresh marker is written, and the
loop moves to the next trigger. **Nothing aborts.** An OOM nine hours in costs the *lexical* rows for
that one file, not the run.

A second escape hatch exists for the genuinely irreducible case: `ingest_watch_batch_with_oom_split`
recursively bisects an OOM'd batch and quarantines the poison conversation
(`streaming-ingest-out-of-memory`) rather than failing. Spec 018's live log records this happening in
production against the real archive — 3 then 7 quarantined codex sessions — with the run still
reporting `success=true` and health `ready`.

**Would a 12-24 h run on a 20 GB mirror / 7.93 GB DB hit it? Almost certainly yes, and that is
survivable.** Spec 017's own reproducer is precisely "small session, live-sized DB clone, lexical
update OOMs". With a 2.57 GB rollout in the manifest, expect at least one deferral and possibly a
quarantine. **The cost is a follow-up lexical refresh, not a lost run.**

**Therefore the backfill has a mandatory second phase**, and it is already specified and shipped:
spec 018's idempotent finalization. The documented command is the plain
`cass index --json --no-progress-events --data-dir <data dir>`, which takes the no-rebuild
finalization path when a completed DB-matching checkpoint exists. Spec 018's live log measures it at
`elapsed_ms=76023` on the first pass and 30 s afterwards against a ~7 GB archive
(`lexical_strategy=deferred_authoritative_db_rebuild`, then `incremental_inline`).

**One counter-signal I could not close.** `-81z91`'s Run B — watch-once against a *clone of the live
7.4 GB archive* — sat in `phase="preparing"`, `total=0`, for 12+ minutes and was SIGTERM'd at
15 m 12 s. It was never allowed to finish, so "the startup path against a live-sized archive
completes" is **UNVERIFIED**. Two things argue it is slow rather than wedged: spec 018's live runs
against that same archive completed in 30-76 s, and the fast-path branch for exactly this case exists
at `src/indexer/mod.rs:12398-12404` (`populated_explicit_watch_once_only`, which requires
`initial_canonical_sessions_before_salvage > 0` and takes a cheaper checkpoint probe). But Run B was
an explicit watch-once and *did* qualify for that branch and *still* took >12 min. **This is the
single largest unmeasured risk in the plan, and step 1 of the runbook is designed to measure it for
~60 s of cost.**

### The stall watchdog cannot kill the run — measured at source

`IndexStallWatchdog::observe` (`src/lib.rs:79059-79126`) computes a payload and **returns it**. It
never signals, never aborts, never touches the process. It also self-silences per phase
(`stall_reported_for_phase`) and resets on any phase change or on `current` advancing. Threshold is
`CASS_INDEX_STALL_DETECT_SECS`, default 120 s (`src/lib.rs:79024-79035`). The napkin's "fires 4x in
26.8 min and still exits 0" is consistent with that: each trigger toggles phase 1→2 and re-arms it.

**The thing that CAN wreck the run is a different mechanism with a similar name.** `StaleDetector` /
`StaleAction` (`src/indexer/mod.rs:505-585`, acted on at `20678-20711`) defaults to `Warn`, but:

```rust
"rebuild" | "auto" | "fix" => Self::Rebuild,
```

and `StaleAction::Rebuild` calls `callback(vec![], &roots, true)` → `reindex_paths(..., force_full = true)`
**over every root**. If `CASS_WATCH_STALE_ACTION` is set to `rebuild`/`auto`/`fix`, a long quiet
stretch mid-backfill turns into an unsolicited full rebuild of everything. Both this and the stall
threshold are read through `dotenvy::var`, which reads a `.env` file as well as the process
environment.

**MEASURED now:** no `CASS_*` variables are set in the shell, and there is no `.env` in the repo or
at `~/.env`. Re-check immediately before launching, and launch from a directory with no `.env`.

---

## 6. Two hazards in the recommended command that nobody has recorded

Bead `-2bh4a` part 2 recommends
`cass index --watch-once "$(paste -sd, /path/to/hole-day-dirs.txt)"` — 108 *day directories*.
**Do not run that form.** Both problems are in the connector's root handling, and both disappear if
you pass file paths instead.

### 6a. Day-directory roots mint non-canonical `external_id`s, and the archive's unique key is `external_id`

`scan_codex_with_callback` (`src/connectors/codex.rs:378-410`) derives the id by stripping a
`sessions_dir` prefix:

```rust
let sessions_dir = explicit_file.as_ref()
    .and_then(|path| CodexConnector::sessions_dir_for_explicit_file(path))
    .unwrap_or_else(|| CodexConnector::sessions_dir(&home));
...
let external_id = source_path.strip_prefix(&sessions_dir).ok().and_then(|rel| rel.with_extension("")...)
```

- Root is a **file** → `sessions_dir_for_explicit_file` walks ancestors for a component literally
  named `sessions` (`codex.rs:91-97`) → finds `~/.codex/sessions` → id `2026/02/12/rollout-…`.
- Root is a **day directory** → `sessions_dir(day)` is `day.join("sessions")`, which does not exist,
  so it returns the day dir itself (`codex.rs:69-76`) → id `rollout-…` with **no date prefix**.

And the archive's dedup key is the id, not the path (**MEASURED**, read out of the live schema):

```sql
CREATE UNIQUE INDEX idx_conversations_provenance ON conversations(source_id, agent_id, external_id)
```
confirmed against `src/storage/sqlite.rs:4844` and the lookup key builder at `sqlite.rs:6223`
(`"{}:{source_id}:{agent_id}:{}:{external_id}"`). There is no normalisation of `external_id`
anywhere in `src/indexer/mod.rs` (the only `format!`s that build one are test fixtures).

Every one of the 3,058 codex rows already in the archive uses the dated form (**MEASURED**:
`external_id LIKE '%/%'` → 3058, `NOT LIKE '%/%'` → 0). So a day-directory backfill would write
4,895 rows keyed differently from every neighbour, and the next ordinary scan rooted at `~/.codex`
would compute the dated id for the same file, find no conflict, and **insert a second row**. Silent
duplication of the entire recovery.

Passing file paths produces the canonical dated form and the hazard vanishes. Flat files are
unaffected either way (their canonical id has no slash to begin with).

### 6b. File paths are resumable; day directories are not

`explicit_watch_once_root_unchanged_after_last_index` (`src/indexer/mod.rs:21158-21201`,
**MEASURED**) returns "skip" only when **all three** hold: the root is a file; its mtime is
`<= last_indexed_at`; and a `conversations` row already exists for that exact `source_path`.
Anything that is not a file returns `Ok(false)` at line 21164.

So a re-run over file paths **automatically skips everything already landed** — the run is
idempotent and resumable at zero cost. A re-run over day directories re-scans and re-ingests all of
them. Given a 16-24 h run that may be interrupted, this is not a nicety.

### Both together

Use file paths. Manifest fits: comma-joining all 4,895 paths is **533,252 bytes** against
`ARG_MAX 1048576` (**MEASURED**), so even one invocation is legal — but batch anyway (§8), because
batches give checkpoints and because a batch is exactly what the skip logic makes free to retry.

---

## 7. Disk arithmetic, honestly

**MEASURED today** (`os.walk`, not `du | tail` — see the `xargs`/`du` trap in the global rules):

```
agent_search.db                        7,927,099,392  B   7.93 GB
  page_size 4096, 1,935,327 pages, freelist 748,333 pages
  → 3.065 GB free, 38.67 % of the file
raw-mirror/    251,208 files          21,326,374,995  B  21.33 GB
index/             242 files           1,183,983,558  B   1.18 GB
data dir TOTAL 251,456 files          30,437,498,985  B  30.44 GB

df /   →  3.6 Ti size, 153 GiB available     (the brief said 162 GiB — it has moved)
```

Projected growth for a 13.12 GB input at the measured 2.4x amplification: **~31.5 GB**, taking the
data dir to ~62 GB and leaving ~122 GiB free. Note the mirror is 1:1 with source bytes (the indexer
calls `attach_raw_mirror_capture` per conversation at `src/indexer/mod.rs:20939`), so ~13 GB of that
31.5 GB is raw mirror.

**Disk is not a constraint.** Do not do anything clever to free space first.

### Would a VACUUM help, and is it safe here?

`VACUUM` reclaims the freelist, so the ceiling is **~3.07 GB** — 2 % of free disk. It buys nothing
operationally.

It is *available*, and in one specific form it is worth running for a different reason.
`frankensqlite` supports `VACUUM INTO`, and cass already uses it: `src/storage/sqlite.rs:1457`
`conn.execute_compat("VACUUM INTO ?", ...)` inside `create_backup`, staged through
`.<name>.vacuum-in-progress` and renamed on success (`sqlite.rs:1404-1444`), with a documented
refusal to fall back to a raw WAL bundle copy under contention (`sqlite.rs:1412-1424`). That is a
**non-destructive** operation: it writes a new compacted file and never rewrites the live archive in
place. In-place `VACUUM` on a 7.93 GB frankensqlite file is not something I would run against the
only copy of 3,877 conversations, and there is no reason to.

**Recommendation: skip VACUUM as space reclamation; use `VACUUM INTO` as the pre-run backup (§8
step 0).** The backup lands at ~4.9 GB rather than 7.93 GB, which is a pleasant side effect and not
the point.

---

## 8. What else is missing, and where it lives

### Claude Code — and a data-loss hazard that outranks everything else in this lane

**MEASURED** against `~/.claude/projects` and the live archive:

```
claude jsonl on disk                     8,008    6.09 GB
indexed claude_code rows                 4,050
  ...whose source file NO LONGER EXISTS  3,877        <-- 95.7 %
files on disk absent from the index      7,835    5.95 GB
  of which mtime > last_indexed_at       7,796        (ordinary staleness)
  of which mtime <= last_indexed_at         39    0.005 GB
```

Claude Code rotates transcripts off disk. **For 3,877 conversations the cass archive is the only
surviving copy.** That makes `cass index --full`, `--force-rebuild`, and anything that clears
conversations a data-destruction event, not a slow command.

I checked what can actually delete rows in the shipped binary (**MEASURED**): the wholesale
`DELETE FROM conversations` at `src/indexer/mod.rs:20738` is inside `reset_storage`, which is
`#[cfg(test)]` and unreachable in the product — good. The reachable one is
`purge_agent_archive_data` (`src/storage/sqlite.rs:7121-7190`), which deletes **every conversation
for an agent**, reached from `purge_excluded_agent_archive_data` (`src/lib.rs:90425`) on the
`cass sources agents` exclusion path. **Never exclude `claude_code`.**

The Claude gap is 99.5 % ordinary staleness (7,796 of 7,835 files are newer than the last index run),
so it is the plain catch-up run's job, not the targeted backfill's.

**Null result worth recording so nobody else burns an hour on it.** `~/.claude-accounts/` holds nine
account directories (`chip, dale, dale.lock, erika, faith, george, george.lock, hilda, katherine`)
and my first pass measured 72,072 jsonl / 54.84 GB across them, with zero indexed — which looked like
a corpus five times the size of everything else. It is an artifact: every one reported *identically*
8,008 files / 6.09 GB, which is the signature of a symlink, and `readlink` confirms
`~/.claude-accounts/<acct>/projects -> /Users/dalecarman/.claude/projects` (**MEASURED**). There is no
additional Claude corpus. `os.walk` follows nothing by default but these are directory symlinks
resolved by the path itself.

### The Mac mini

**MEASURED** via `ssh mini-ts` (user `chipcarman`), read-only:

```
~/.codex/sessions   rollout-*     1,293 files     258 MB
~/.claude/projects  *.jsonl       3,585 files    1.20 GB
                    TOTAL         4,878 files    1.46 GB     <-- matches the brief exactly
cass on the mini    /opt/homebrew/bin/cass   version 0.6.23  (laptop runs 0.6.9)
mini data dir       .../com.coding-agent-search.coding-agent-search/ contains only index/ (Apr 22)
                    — no agent_search.db. The mini has never indexed anything.
```

Laptop archive `sources` table holds exactly one row, `('local','local',...)` (**MEASURED**) —
confirming no remote source is configured. The surface exists:
`cass sources {list,add,remove,doctor,sync,mappings,agents,discover,setup}`.

At 1.46 GB the mini is **~11 % of the codex hole** and is the cheapest coverage in the whole repair.
It is also independent of everything above. Do it after the local work lands, or in parallel by a
different lane — but do not let it be the reason the big run waits.

---

## 9. The runbook

Run it **on the laptop**. The corpus, the archive, and the only proven binary are all here; the mini
has an empty data dir and an unproven different version, and shipping 30 GB of archive there to save
nothing is strictly worse.

Run it **on the installed pre-fix binary**, `/Users/dalecarman/.local/bin/cass` (0.6.9,
sha256 `3d044227..`). Reasons, both **MEASURED at source**: a targeted watch-once never advances a
watermark (`src/indexer/mod.rs:21125` — every `save_watch_state_watermark` call in the trigger loop is
gated on `!explicit_watch_once`), so the recovery does not need the coverage-floor fix; and the fixed
binary adds an unbounded `FrankenStorage::open_readonly` of the live archive at index startup
(`read_connector_scan_floors_fresh`, `src/indexer/mod.rs:10696-10721`), which is the exact operation
bead `-1a7mk` measured as never returning on this 7.9 GB file (6/6 deterministic). Do not put an
unmeasured 15-hour run behind it.

### Step 0 — backup (mandatory, ~5-10 min)

Because of the 3,877 archive-only Claude conversations. Take it with the same mechanism cass itself
uses, into a path outside the data dir:

```bash
python3 - <<'PY'
# uses the same VACUUM INTO cass's create_backup uses; writes a NEW file, never touches the live one
PY
```
or simply let cass do it if a backup subcommand is wired; otherwise a plain `cp` of
`agent_search.db` + `-wal` + `-shm` while no indexer is running is acceptable (7.93 GB).
Verify the copy opens read-only and reports 12,722 conversations before proceeding.

### Step 1 — the 60-second falsifier (do this before anything long)

Nothing has ever run `--watch-once` against the **live** archive. `-81z91` Run B is the only
live-sized evidence and it was killed at 15 min in `phase="preparing"`. Buy that answer for one file:

```bash
cd "$HOME"                     # no .env on the path
env | rg '^CASS_' || echo "clean"
F=$(head -1 /tmp/cass-hole-manifest.txt)     # smallest file, see step 2
/usr/bin/time -l /Users/dalecarman/.local/bin/cass index --watch-once "$F" \
  --json --progress-interval-ms 5000 2>&1 | tail -40
```

**Read the elapsed time to first `phase_code=1`.** If startup is 30-120 s, the plan holds and step 3's
batch overhead is `batches × startup`. If it exceeds ~15 min with no phase advance, **stop** — you
have reproduced `-81z91` Run B against the live archive, the whole backfill is blocked behind it, and
that is the finding, not a failure.

### Step 2 — build the manifest (read-only, ~2 min)

Ordered **smallest file first**, so cheap batches prove the path and the 2.57 GB monster lands last:

```python
# read-only; writes only /tmp/cass-hole-manifest.txt
import os, sqlite3
sess = os.path.expanduser("~/.codex/sessions")
db   = os.path.expanduser("~/Library/Application Support/com.coding-agent-search.coding-agent-search/agent_search.db")
cur  = sqlite3.connect("file:"+db+"?mode=ro", uri=True, timeout=10).cursor()
cur.execute("select source_path from conversations where agent_id=3")
idx = set(r[0] for r in cur.fetchall())
LAST = 1784200805044                      # meta.last_indexed_at, 2026-07-16T11:20:05Z
rows = []
for dp,_,fs in os.walk(sess):
    for f in fs:
        if not f.startswith("rollout-"): continue
        if not (f.endswith(".json") or f.endswith(".jsonl")): continue
        p = os.path.join(dp,f)
        st = os.stat(p)
        if p in idx: continue
        if int(st.st_mtime*1000) > LAST: continue      # staleness is step 4's job
        rows.append((st.st_size, p))
rows.sort()
open("/tmp/cass-hole-manifest.txt","w").write("\n".join(p for _,p in rows)+"\n")
print(len(rows), "files", round(sum(s for s,_ in rows)/1e9,2), "GB")
```

Expected: `4895 files 13.12 GB`. If it prints materially fewer, someone has already run part of
this — reconcile before continuing.

### Step 3 — the backfill, in batches of 250 (16-24 h, expect longer)

```bash
mkdir -p /tmp/cass-backfill && cd /tmp/cass-backfill
split -l 250 /tmp/cass-hole-manifest.txt batch-      # 20 batches

nohup caffeinate -is bash -c '
  for b in /tmp/cass-backfill/batch-*; do
    echo "=== $b $(date -u +%FT%TZ) ==="
    /Users/dalecarman/.local/bin/cass index \
      --watch-once "$(paste -sd, "$b")" \
      --json --progress-interval-ms 60000
    echo "=== rc=$? $(date -u +%FT%TZ) ==="
  done
' > /tmp/cass-backfill/run.log 2>&1 &
echo $! > /tmp/cass-backfill/run.pid
```

`caffeinate -is` keeps the machine awake for a multi-hour run; `nohup` + `&` survives the terminal.
Keep progress events on at a 60 s interval — they are the only in-band progress signal, and
`--no-progress-events` would also silence the stall payloads you want to see and ignore.

Batches are free to retry: §6b's skip logic means a re-run of a completed batch does nothing.

### Step 4 — catch-up for everything newer than 2026-07-16

Separate operation, **different and safer code path**: plain `cass index` uses the streaming producer
(`conn.scan_with_callback`, `src/indexer/mod.rs:10832`/`10902`) with batched ingest, not watch-once's
whole-root `Vec`. This is the ~2,336 codex + ~7,796 claude files that are merely stale.

```bash
/Users/dalecarman/.local/bin/cass index --json --progress-interval-ms 60000
```

Run it **after** step 3. It is the same command spec 018 designates as the lexical-refresh
finalization path, so it doubles as step 5.

### Step 5 — finalize the lexical index

Expect step 3 to have deferred some lexical updates (§5). Re-run step 4's command until
`cass status --json` reports `index.status: ready` and no pending
`lexical-refresh-needed.json`. Spec 018's live evidence: 76 s first pass, ~30 s after.

Caveat: on the currently installed binary `cass status --json` and `cass doctor` never return
(bead `-nvq59` — they CPU-walk the 21.33 GB / 251,208-file raw mirror). Use plain `cass status`
(49 ms) and `cass health` until that is fixed.

### Step 6 — the mini (independent, ~1-2 h)

```bash
cass sources discover              # reads ~/.ssh/config
cass sources add …                 # mini-ts / chipcarman
cass sources doctor
cass sources sync
```
1.46 GB, 4,878 files. Cheapest coverage available. Does not need to wait for step 3.

### Step 7 — install the watcher

Otherwise the index goes stale again the day after it goes green. Currently `launchctl` has no cass
job and `-1vxuf`/`-2gif2` record that the fork-local `cass watchdog install` machinery was removed by
the 2026-05-17 upstream reset, so **there is no supported command that installs it**. That is a
decision for Dale, not a bug to fix in this run.

---

## 10. Watching a live run: real progress vs the false stall

The in-band signal is the NDJSON progress line's `current` (conversations ingested) and `total`.
`phase_code` 0 = preparing, 1 = scanning, 2 = indexing (`src/indexer/mod.rs:978`).

Do not trust that alone — use an **out-of-band** instrument that does not depend on the process being
honest:

```bash
# every 5 min, in a second shell
python3 -c "
import sqlite3,os,time
p=os.path.expanduser('~/Library/Application Support/com.coding-agent-search.coding-agent-search/agent_search.db')
c=sqlite3.connect('file:'+p+'?mode=ro',uri=True,timeout=10)
print(time.strftime('%H:%M:%S'), c.execute('select count(*) from conversations where agent_id=3').fetchone()[0])"
```

Baseline is **3,058**; the run is complete near **7,953**.

**A `stall_detected` event is not a stall.** It is a warning that fires once per phase after 120 s of
no `current` movement, it cannot abort anything (§5), and a single large rollout legitimately takes
longer than that to parse. The 2.57 GB file at the end of the manifest could sit silent for a long
time and be perfectly healthy.

### Stop the run if any of these

1. The codex conversation count has not moved in **60 min** *and* the process RSS is flat *and* no new
   files are appearing under `raw-mirror/`. That is three independent instruments agreeing.
2. RSS above **~60 GB**, or free swap at zero. One trigger should never need that; if it does, the
   whole-root path has somehow been taken.
3. Free disk below **30 GB**.
4. The conversation count **drops**, or `agent_search.db` shrinks. Abort immediately and restore the
   step-0 backup — nothing in this plan should ever delete a row.
5. Any log line mentioning a rebuild being triggered (`"stale state detected, triggering automatic
   full rebuild"`). Kill it, then check `CASS_WATCH_STALE_ACTION`.

### Do not, at any point

- `cass index --full` or `--force-rebuild` — 3,877 Claude conversations exist only in the archive.
- `cass sources agents` exclusions — that path deletes every row for the agent
  (`src/lib.rs:90425` → `src/storage/sqlite.rs:7174`).
- `cass mirror` pruning to free space. Disk is not the constraint, and the raw mirror is the evidence
  trail for everything just ingested.
- pass `~/.codex/sessions` or `~/.pi/agent/sessions` as a `--watch-once` root. That is the one shape
  measured to wedge.

---

## 11. Verdict

**Safe to run today, on the installed pre-fix binary, after step 0's backup and step 1's 60-second
falsifier.** It is not blocked behind `-81z91`/`-373b1`: those describe one unbounded root, and the
file-path manifest makes every trigger one file. It is not blocked behind `-1a7mk`: that regression
lives in the read surfaces of a binary we are deliberately not using. It is not blocked behind
`-kfaid`: that bead's premise is refuted and its 1,647 files are just the cheapest batch in the same
manifest.

The one thing that *could* block it is the unmeasured `phase="preparing"` startup against the live
archive. Step 1 buys that answer for a minute, and no long run should start before it.

Two corrections the parent bead's recommended command needs, both from source and neither previously
recorded: **file paths, not day directories** — for `external_id` canonicality against a unique index
keyed on it, and for the free resume-skip. And **a backup first**, because 95.7 % of the indexed
Claude corpus no longer exists anywhere else.

Estimate: **16-24 h** for step 3 on a quiet machine, ~31.5 GB added, treated as a lower bound.
