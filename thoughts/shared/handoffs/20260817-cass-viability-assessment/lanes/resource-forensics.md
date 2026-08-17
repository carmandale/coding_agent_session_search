# Lane: resource-forensics — where the bytes and the gigabytes of RAM go

Date: 2026-08-17 (09:45–10:00 CDT). Read-only lane; no cass invocations were made; every
sqlite3 read was bounded (background+kill runner — gtimeout is not installed on this Mac,
verified `command -v gtimeout timeout` empty). PID 75534 was not touched.

All sizes are `du` GiB unless bytes are given. MEASURED = command output captured this
session. INFERRED = mechanism read from source or deduced from timestamps.

---

## 1. The 69G in `.claude/worktrees` is 98.6% cargo build artifacts

MEASURED — `du -sh` per worktree, `git worktree list` confirms all six are registered:

| worktree | total | target/ | target breakdown | last build write | files changed <24h |
|---|---|---|---|---|---|
| codex-coverage-gap-2bh4a | 30G | 30G | debug 28G + release 2.3G | **Aug 10 13:40** | **0** |
| cass-759l7-spin-wait | 22G | 21G | debug 21G | Aug 16 19:42 | 3,568 |
| cass-p3kgr-gen13 | 17G | 17G | debug 14G + release 2.2G | **Aug 17 07:13** | 3,559 |
| cass-to-green-c6bfb589 | 97M | none | — | — | 0 |
| cass-nvq59-status-hang | 97M | none | — | — | 0 |
| cass-gen5-honesty | 97M | none | — | — | 3 (handoff logs) |

- Total: 69.3G, of which **68G is three `target/` directories** and ~1.3G is six source
  checkouts (each ~97M).
- Liveness: no running process has a path in any worktree (`ps auxww | rg worktrees` —
  only this assessment's own probes matched). codex-coverage-gap-2bh4a has had **zero
  file writes since Aug 10** → abandoned residue, 30G reclaimable with `cargo clean` (or
  worktree removal) at zero information loss. cass-p3kgr-gen13's debug tree was written
  **this morning 07:13** and p3kgr/gen13 is the newest handoff generation → treat as live
  until its session closes. cass-759l7-spin-wait last built Aug 16 19:42 (yesterday
  evening) — probably a paused/recent session; confirm before cleaning.
- A single full debug+release build of this crate costs **17–30G of target/**; every
  worktree that ever compiles pays it again. Nothing shares a target dir
  (no CARGO_TARGET_DIR consolidation), so N concurrent fix sessions ≈ N × ~20G.

Reclaim: 30G now (idle worktree), 38G more when the two active sessions close, all of it
rebuildable at CPU cost only.

## 2. Production data dir: 37.5G of sources becomes 77.5G derived — and every byte of the corpus is stored 2–3 times

MEASURED totals: raw-mirror 46G + agent_search.db 21.7GiB (23,313,477,632 bytes) +
index 9.5G = **77.5G**.

### 2a. raw-mirror (46.6G) — verbatim uncompressed byte-copies of every source file

- Structure: `v1/blobs/blake3/<xx>/<hash>.raw` + `v1/manifests/*.json`. MEASURED:
  **140,344 blobs (46G)** + **147,844 manifests (578M)**.
- Blobs are **verbatim, uncompressed** copies: 8 of 8 sampled large blobs matched their
  live original in both size and first 64KB (`file` says plain "JSON data"; no
  compression anywhere). Content-addressing dedupes identical files (147,844 manifests →
  140,344 unique blobs, ~5% dedup), nothing more.
- It mirrors MORE than the 37.5G claude+codex corpus: sampled manifest providers were
  opencode (from `~/.local/share`), codex, claude_code, claude, pi_agent, openclaw,
  cursor, amp. By sampled bytes codex dominates (~80%).
- So the first ~46G of "derived" data is a full second copy of every session file the
  machine has, kept forever with no compression. zstd on JSONL typically gets 5–10×;
  this store chose 1.0×.

### 2b. agent_search.db (21.7GiB) — 10.3G of message text + 3.5G analytics + 8.2GiB dead space

MEASURED via stock sqlite3 read-only (`dbstat WHERE aggregate=TRUE`, 110s bound —
completed):

- `messages` table: **10,562 MB** — 2,335,514 rows (max rowid) ≈ 4.5KB/row: the message
  text is stored a THIRD time (after the original and the raw-mirror blob), plus FTS.
- Analytics machinery: `message_metrics` + `token_usage` tables and their **13 indexes**
  total ≈ **2.9G** (idx_token_usage_timestamp 453M, idx_mm_workspace_hour 411M,
  idx_mm_hour 339M, idx_mm_agent_hour 321M, idx_mm_source_hour 299M …). Five separate
  hour/day rollup index families over per-message metrics.
- `conversations`: 47M (27,441 rows).
- **freelist_count = 2,141,121 pages of 4,096 bytes = 8.2GiB dead space — 37.6% of the
  file** (page_count 5,691,767). A vacuum backup dated 20260814 exists (see §4), so
  either the freelist regrew to 8.2GiB in three days of churn or the vacuum never
  landed. VACUUM would return ~8.2GiB (needs downtime + scratch space).

### 2c. index/ (9.5G) — only 2.3G is the live index; 6.6G is this morning's stalled job

MEASURED:

| entry | size | mtime |
|---|---|---|
| v8/ (live tantivy index, 34 segments) | 2.3G | files as recent as Aug 16 09:48 |
| cass-lexical-merge.YtoIyM | 4.2G | created **Aug 17 04:50:36** |
| cass-lexical-shards.4zftqB | 2.4G | created **Aug 17 04:50:35** |
| .lexical-publish-backups | 566M | Jul 22 |

The two temp dirs were created in the same minute PID 75534 started (04:50). The
production `index-run.lock` (MEASURED, read 09:48) says:
`pid=75534 … db_path=/private/tmp/fsq-probe-data/prod.db … job_kind=lexical_refresh,
phase=index, started_at_ms=1786960233009 (04:50:33), last_progress_at_ms=1786960307804
(04:51:47), updated_at_ms=1786978102946 (09:48:22)`.

Three facts fall out, the first two MEASURED, the third INFERRED:

1. A `cass … search` invocation spawned a background lexical_refresh **index job**.
2. That job has heartbeat for ~5h with **zero progress since 75 seconds after start**
   (updated_at ticks, last_progress frozen) — and `ps` shows 75534 at 292 CPU-minutes
   with only ~31MB RSS: a pure CPU spin, matching the repo's own "spin-wait" worktree
   name.
3. Although the process was pointed at `--db /tmp/fsq-probe-data/prod.db`, its lock and
   its 6.6G of shard/merge temp data landed in the **production** data dir — the temp
   and lock paths derive from the default data dir, not from `--db`. A probe against a
   /tmp copy is dirtying and locking production. (No other cass process existed at
   04:50 to attribute these artifacts to.)

Net: of 77.5G "derived", the honestly-live search assets are ≈ live DB pages (~13.5G) +
v8 (2.3G) ≈ 15.8G. The rest is a 46.6G uncompressed mirror of files that still exist at
their source, 8.2GiB of SQLite dead space, 6.6G of stalled-job temp, and 0.6G of backups.

## 3. Why `cass stats` reaches 5.2GB RSS

Code path (MEASURED in source; the binary run itself was the coordinator's measurement):

- `Commands::Stats` → `run_stats` (src/lib.rs:24196). It opens the 22G production DB via
  **frankensqlite** — crate `fsqlite` v0.1.5 from crates.io (Cargo.toml:45,
  Cargo.lock:2270), a from-scratch pure-Rust SQLite reimplementation at version 0.1.x —
  not C SQLite (rusqlite is also in the dep tree but this path doesn't use it).
- It then runs, over that engine: `SELECT COUNT(*) FROM messages` (10.3GB table,
  2.33M rows), `SELECT agent_id, COUNT(*) FROM conversations … GROUP BY` and
  `SELECT workspace_id, COUNT(*) … GROUP BY` (src/lib.rs:24244–24316, 24126–24144), plus
  a per-source `COUNT(DISTINCT c.id) … JOIN messages` when by-source output is on.
  The repo's own commit 5d1718a3 (bead p3kgr) records this engine class-failing exactly
  this shape: "the query phase is 16m20s with no result … the pinned SQLite engine
  cannot run the GROUP BY". INFERRED: the engine materializes scans in memory rather
  than streaming b-tree pages — multi-GB RSS for queries whose *results* are a few
  hundred bytes.
- `run_stats` also unconditionally calls `raw_mirror::storage_summary`
  (src/lib.rs:~24341 → src/raw_mirror.rs:73), which walks all 288,188 files for a byte
  total, then opens and JSON-parses **all 147,844 manifests** and stats all 140,344
  blobs, building a 140k-entry HashSet. A sibling lane already measured the same walk
  family: `cass status --json` plateaued at **3.95GB RSS** building 125,607 manifest
  report structs, then hashed 19.68GiB of blobs until killed at 15 minutes
  (thoughts/shared/handoffs/20260814-cass-repair-to-green/lanes/raw-mirror-walk.md:345-360,
  which also notes `cass doctor` reaches the same walk unconditionally at
  src/lib.rs:68743).
- Also on the path before any output: `validate_fts_messages_integrity_for_cli` probes
  `SELECT * FROM fts_messages LIMIT 0` through the same engine
  (src/storage/sqlite.rs:1189,1291).

So 5.2GB RSS on a "print counts" command is structural: a v0.1.5 hand-rolled SQL engine
scanning a 22G DB it cannot execute aggregates on, plus a full parse of 147k manifest
files, on every `stats` (and the same family on `status`/`doctor`).

### Crash record (MEASURED — CrashReporter + DiagnosticReports/Retired, read-only)

- Two CrashReporter plists: `cass_…plist` (Aug 15 13:31), `coding_agent_search-983a…plist`
  (Aug 16 12:42) — date markers only (240 bytes each).
- 9 retired .ips reports:
  - **Aug 16 12:41:20–12:42:43, five crashes in ~83s**: test binary
    `coding_agent_search-983a915ea0c0a592`, `EXC_BAD_ACCESS` / "Could not determine
    thread index for stack guard region" → Rust `stack_overflow::imp::signal_handler` →
    abort. Faulting frames sit in clap derive code for the `Cli` type — a **stack
    overflow during CLI parsing** in the test binary (the CLI enum lives in a 92,719-line
    lib.rs). Not OOM — stack exhaustion, five times in a row.
  - Aug 15 13:31 and Aug 13 13:42: `cass` SIGABRT (abort — panic/abort path; symbols not
    in the report).
  - Aug 10 20:37 (×2): `cass` SIGKILL "Code Signature Invalid" — the installed binary
    was overwritten in place while running (matches the ~/.local/bin deploy churn in §4).
- No jetsam/OOM kill reports found; the 5.2GB stats run was killed by the operator, not
  the kernel (128GB machine).

## 4. Other machine-wide cass residue

MEASURED:

- **/tmp (= /private/tmp): 101.8 GiB across 83 cass/fsq entries** (`du -sk` sum:
  106,706,932 KiB). Big ones: fsq-probe-data 29G (the DB copy PID 75534 is querying),
  cass-gen8-target 26G, cass-759l7-forward-target 12G, cass-8llb5-verify 9.9G,
  cass-fix-target 7.7G (holds the binary 75534 is executing), cass-ibuuh-probe 4.9G,
  cass-0119-test 3.0G, cass-0119-target 2.7G, cass-fix-data 2.3G, three fsq-probe
  targets ~2.9G. Every entry's mtime is Aug 16–17 — this is the current fix campaign's
  churn (mostly yet more cargo target dirs built in /tmp), not old junk. 36.7G
  (fsq-probe-data + cass-fix-target) is pinned by live PID 75534; the remaining ~65G has
  no visible owner process but was written within ~36h, so confirm session ownership
  before deleting. macOS's 3-day /tmp reaper will take these anyway once sessions stop
  regenerating them.
- **~/.local/bin: 9 cass binaries, 0.44 GiB** — `cass` (current, Aug 16 12:29) plus 8
  timestamped rollback copies (pre-gen5/gen10/gen11, pre-8llb5, nvq59-gate, pre-1a7mk,
  coverage-floor ×2). ~0.39 GiB is rollback residue.
- **~/backups/cass/agent_search-20260814-vacuum.db: 3.7G** (3,984,084,992 bytes,
  Aug 14 16:24) — pre/post-vacuum DB backup; prod freelist is nonetheless 8.2GiB today.
- Main repo `target/`: 2.7G; repo `.beads/`: 700M (tracker DB+JSONL, with 11–89M copies
  in each worktree). No LaunchAgents, no ~/Library/Caches entry.

## 5. The honest table

Sources it mines (not cass-owned): ~/.claude/projects 8.5G + ~/.codex/sessions 29G = 37.5G.

cass-attributable bytes on this machine, this morning:

| category | size | class |
|---|---|---:|
| Prod data: DB live pages (messages 10.3G, analytics+idx ~3.2G) | ~13.5G | live-needed |
| Prod data: tantivy v8 live index | 2.3G | live-needed |
| Prod data: raw-mirror (uncompressed verbatim copy of all sources) | 46.6G | by-design duplication — the product's choice, but 0% of it is unique data |
| Prod data: SQLite freelist (dead pages) | 8.2G | residue (VACUUM) |
| Prod data: stalled-job shard/merge temp (today 04:50) | 6.6G | residue once PID 75534 is dealt with |
| Prod data: lexical publish backups | 0.6G | residue |
| Worktree target/ dirs (3) | 68.0G | residue (30G idle since Aug 10; 38G owned by recent sessions) |
| Worktree sources (6 checkouts) | 1.3G | live-ish (2 sessions), rest removable |
| Main repo target/ | 2.7G | rebuildable |
| /tmp cass+fsq (83 entries) | 101.8G | residue; 36.7G pinned by PID 75534, ~65G unpinned |
| ~/backups/cass vacuum DB | 3.7G | operator-decision backup |
| ~/.local/bin rollback binaries (8 of 9) | 0.4G | residue |
| repo .beads tracker | 0.7G | live tracker |
| **Total cass-attributable** | **≈ 256G** | |
| of which live-needed for search to work | ≈ 18–20G | DB live + v8 + current binary + checkout |
| of which by-design duplication (raw-mirror) | 46.6G | product decision to re-store the corpus uncompressed |
| of which residue/reclaimable | ≈ 190G | ~100G without touching any live session (30G idle worktree target + ~65G unpinned /tmp + 3.7G backup + 0.4G binaries + 0.6G publish backups), the rest as sessions close (38G worktrees, 36.7G /tmp) + 8.2G VACUUM + 6.6G index temp |

Context numbers: the disk has 29Gi free; cass residue alone (~190G) is 6.5× the free
space; the machine is carrying **6.8×** the source corpus in cass-attributable bytes to
answer searches over it (256G / 37.5G), of which the actually-load-bearing search assets
are ~0.5× (18-20G / 37.5G).

## Notes for other lanes

- The `index-run.lock` cross-contamination (a /tmp-DB probe locking prod and writing
  6.6G into prod index/) means "read-only probe against a copy" is currently NOT
  read-only toward production. Any lane spawning cass against copied DBs inherits this.
- PID 75534's RSS is ~31MB — the 4h48m search hang is CPU spin, not memory. The 5.2GB
  class of problem (stats/status/doctor) and the spin class (search/GROUP BY through
  fsqlite) are two different defects.
- The five-in-83s Aug 16 stack-overflow crashes are in the TEST binary while parsing the
  CLI — the 92k-line lib.rs / giant clap enum is itself at a scale that breaks runtime
  stack limits under test.
