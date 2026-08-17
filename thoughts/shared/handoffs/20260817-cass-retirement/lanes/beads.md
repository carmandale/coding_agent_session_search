# cass retirement — beads sweep (discovery only, read-only)

Lane: beads. Date: 2026-08-17. Read-only: no bead was closed, updated, deferred, created, or flushed; no cass binary was run.

Goal served: Dale requires "no open beads" and "nothing that would potentially resurrect it." This log establishes the exact, complete set of beads that must be closed, across every tracker on this machine, plus the findings that should outlive the tool.

---

## Headline

| question | answer |
|---|---|
| Open/in_progress beads in the **main** cass tracker | **44** (38 open + 6 in_progress) |
| Open beads that exist **only outside** the main tracker | **4** |
| **True total that must be accounted for** | **48** |
| Is the JSONL export stale? | **No.** Zero field drift; the only divergence is a tombstone `br list` excludes by design |
| Cross-tracker bead dependency edges | **Zero.** All 3,042 edges are internal to the cass tracker |
| Beads in other trackers genuinely about cass | **6** (1 close/rewrite, 3 rewrite-not-close, 2 leave-open) |
| Beads that look like cass but are **not** | **4** — see the frankensqlite warning below |

### The single most important thing in this log

**br 0.2.22 is itself built on frankensqlite/fsqlite 0.1.19.** The binary embeds `fsqlite-func-0.1.19`, `fsqlite-ext-fts5-0.1.19`, `fsqlite-vfs-0.1.19`, `fsqlite-ext-icu-0.1.19`, and reports `FrankenSQLite 0.1.0 (compatible with SQLite 3.52.0)`. Verified by `strings /Users/dalecarman/.local/bin/br`. 172 `.beads` directories on this machine carry br's `beads.db-fsqlite-ns-gate` / `beads.db-fsqlite-ns-use` sidecars.

**Retiring cass must not retire frankensqlite.** Any sweep that deletes "frankensqlite" or "fsqlite" artifacts on the theory that they are cass residue would damage the beads fleet Dale uses in every repo. Several cass beads are frankensqlite defect reports that apply to br and should be re-filed rather than discarded.

---

## 1. Main cass tracker census

File: `/Users/dalecarman/dev/coding_agent_session_search/.beads/issues.jsonl`
1,929 lines, 1,929 unique ids, 0 malformed, 0 duplicate ids, 6,245,130 bytes, mtime 2026-08-17T15:00:44Z.

| status | count |
|---|---|
| closed | 1,884 |
| open | 38 |
| in_progress | 6 |
| tombstone | 1 |

Parsed line-by-line with python, not `br`.

### Tombstone (already deleted — no action)

`coding_agent_session_search-3wam` — line 674, "Issue 1", priority 2, `deleted_at` 2026-01-27T17:26:45Z, `deleted_by` ubuntu, `delete_reason` "delete". This is the one record `br list` excludes, which fully explains the 1929/1928 count difference.

### The 6 in_progress

| id | p | title |
|---|---|---|
| `coding_agent_session_search-p3kgr` | 0 | cass cannot index at all: one lexical-prep aggregate over messages takes >20 min in frankensqlite, 0.03s in stock sqlite |
| `coding_agent_session_search-1vxuf` | 2 | Recover cass session ingestion and watcher |
| `coding_agent_session_search-2d37b` | 2 | Index chipbot symlink sessions under clawdbot/Pi roots |
| `coding_agent_session_search-2gif2` | 2 | Repair health-watchdog command surface regression |
| `coding_agent_session_search-373b1` | 2 | pi_agent watch-once stalls after first chunk with 22GB RSS memset loop |
| `coding_agent_session_search-81z91` | 2 | watch-once scan materializes entire corpus before persist, blocks pi-agent historical backfill |

### The 38 open

| id | p | title |
|---|---|---|
| `coding_agent_session_search-ibuuh.29.1` | 0 | Eliminate the single-core "preparing" plateau by making authoritative rebuild prep fully streaming and phase-explicit |
| `coding_agent_session_search-1a7mk` | 1 | cass health, triage and stats hang on the live archive after the coverage-floor fix — the 2s bound covers only the DB open |
| `coding_agent_session_search-759l7` | 1 | Hand-rolled spin-wait on a std mpsc channel deadlocks under block_on at asupersync 0.3.4 |
| `coding_agent_session_search-g0eyv` | 1 | Codex conversations in the archive are missing their tool messages — 6,452 of 10,283 carry zero tool rows; reindex is owed after 9531315d |
| `coding_agent_session_search-guided-ops-repro-trust-5u82n` | 1 | Epic: Guided operations, repro capsules, and trust-scored knowledge surfaces |
| `coding_agent_session_search-guided-ops-repro-trust-5u82n.1` | 1 | Add intent-to-command planner for guided safe workflows |
| `coding_agent_session_search-guided-ops-repro-trust-5u82n.13` | 1 | Add integrated guided-ops golden and e2e gate |
| `coding_agent_session_search-guided-ops-repro-trust-5u82n.2` | 1 | Generate redacted repro capsules for failures and search hits |
| `coding_agent_session_search-guided-ops-repro-trust-5u82n.3` | 1 | Trust-score search results and answer packs with provenance signals |
| `coding_agent_session_search-guided-ops-repro-trust-5u82n.5` | 1 | Preview privacy exposure before indexing exporting or support capture |
| `coding_agent_session_search-hd4u5` | 1 | FTS write gate 'AND rootpage > 0' silently disables all FTS maintenance under fsqlite >= 0.1.17 |
| `coding_agent_session_search-iapqz` | 1 | git object store has 16 broken links (8 missing trees, 6 blobs, 2 commits); unbounded history traversal dies |
| `coding_agent_session_search-jy8v8` | 1 | index-run.lock is machine-global and unscoped to --db/--data-dir: a probe against a scratch DB wedges production search |
| `coding_agent_session_search-move-bundle-stale-hot-journal-gtfx5` | 1 | Stale rollback journal can be replayed into a freshly created database at the same path |
| `coding_agent_session_search-oh96l` | 1 | Epic: Swarm operations cockpit and evidence broker |
| `coding_agent_session_search-pfar8` | 1 | cass mirror prune deletes exactly the irreplaceable blobs — pinning is by recency, never by upstream absence |
| `coding_agent_session_search-swarm-coordination-intelligence-gnrxb` | 1 | Epic: Swarm coordination intelligence and proof-debt control plane |
| `coding_agent_session_search-swarm-coordination-intelligence-gnrxb.10` | 1 | Add integrated coordination-intelligence golden and e2e gate |
| `coding_agent_session_search-xybl9` | 1 | Sidecar allowlist misses fsqlite's -fsqlite-ns-gate/-fsqlite-ns-use family (reopens the #236 orphan-amplification class) |
| `coding_agent_session_search-export-temp-sidecar-orphans-gd0dm` | 2 | Pages export orphans temp-database sidecars on the success path and on a failed replace |
| `coding_agent_session_search-guided-ops-repro-trust-5u82n.10` | 2 | Add workflow macro registry for repeatable operator journeys |
| `coding_agent_session_search-guided-ops-repro-trust-5u82n.4` | 2 | Extract durable lessons and decisions from closed sessions |
| `coding_agent_session_search-guided-ops-repro-trust-5u82n.6` | 2 | Build first-run source onboarding and readiness wizard |
| `coding_agent_session_search-guided-ops-repro-trust-5u82n.7` | 2 | Create search quality evaluation harness with qrels and drift reports |
| `coding_agent_session_search-guided-ops-repro-trust-5u82n.8` | 2 | Verify release distribution channels and installer parity |
| `coding_agent_session_search-k69vx` | 2 | cass status exits 0 while printing 'Database: Exists, but could not be opened' and its own probe over-bound |
| `coding_agent_session_search-mgw1o` | 2 | Report upstream: fsqlite 0.1.19 contentless FTS5 reports a stale COUNT(*) on the appending connection |
| `coding_agent_session_search-n62wn` | 2 | No behavioural test on sources agents exclude — the branch that consumes --purge-indexed-data is unguarded (mutant Q2 survives) |
| `coding_agent_session_search-ns-sidecar-transport-bricks-db-1mgjd` | 2 | Never transport the fsqlite namespace sidecars: a lost mode bit bricks the database permanently |
| `coding_agent_session_search-pi-agent-missing-workspaces-le8s1` | 2 | pi_agent: 41 whole workspace directories have zero indexed rows (199 files) |
| `coding_agent_session_search-swarm-coordination-intelligence-gnrxb.5` | 2 | Add workflow outcome analytics for skills commands proof gates and closures |
| `coding_agent_session_search-swarm-coordination-intelligence-gnrxb.7` | 2 | Optimize bead-scoped context packs under token budgets |
| `coding_agent_session_search-swarm-coordination-intelligence-gnrxb.9` | 2 | Replay real swarm histories into coordination-intelligence fixtures |
| `coding_agent_session_search-2hrs` | 3 | Spike: detect tantivy opens-but-spins corruption on startup |
| `coding_agent_session_search-6t64c` | 3 | Six local branches have broken history (missing git objects); poisons all-branch walks |
| `coding_agent_session_search-d907f` | 3 | frankensqlite_ext_fts5 stores column values even for contentless tables |
| `coding_agent_session_search-guided-ops-repro-trust-5u82n.11` | 3 | Plan resource what-if scheduling for indexing backfill and exports |
| `coding_agent_session_search-guided-ops-repro-trust-5u82n.12` | 3 | Add local operations dashboard for guided workflows and trust signals |

**19 of the 44 are aspirational feature scope, never built** — three epics from May 2026 plus their open children: `guided-ops-repro-trust-5u82n` + 12 children, `swarm-coordination-intelligence-gnrxb` + 4 children, `oh96l` (all 11 of its children are already closed). These carry no measured findings and should close as won't-do. The remaining 25 are real defects and tasks.

---

## 2. Live database cross-check — the export is fresh, no flush owed

```
br --db <cass>/.beads/beads.db list --status all --limit 5000 --json
  -> object with keys: has_more, issues, limit, offset, total
  -> total 1928, has_more false, 1928 items
  -> status counts: closed 1884, in_progress 6, open 38

br --db <cass>/.beads/beads.db ready --limit 5000 --json
  -> bare array, 26 items
br ... ready --limit 5000 --include-deferred --json
  -> 26 items (identical id set — nothing is deferred)
```

Id-set diff rather than a count comparison:

- In JSONL, not in DB: exactly `coding_agent_session_search-3wam` (the tombstone).
- In DB, not in JSONL: none.
- Field drift on `status`/`priority`/`title`/`updated_at`/`closed_at` across all 1,928 shared ids: **0 rows.**

So the 1929 vs 1928 gap is fully explained and **no flush is owed before retirement.** Note the corollary: closing the 44 will mutate the database, so a `br sync --flush-only` **is** owed after the closes and must land in the same commit as the JSONL.

`br ready` = 26 against 38 open: the 12 missing are blocked by open dependencies, not deferred (`--include-deferred` returns the same 26).

---

## 3. Beads that exist only OUTSIDE the main tracker — 4 open, invisible to `br ready`

There are 15 `issues.jsonl` files inside the cass repo. Six `.claude/worktrees/*` trackers, their six nested `recovery_20260509T004736Z` snapshots, a `.git/beads-worktrees/beads-sync` tracker, and the main pair. Most hold stale copies, but **four beads exist in no tracker other than a worktree's**, so closing main's 44 would leave them behind:

| id | status | p | title | only in |
|---|---|---|---|---|
| `coding_agent_session_search-gf1f0` | open | 0 | cass doctor: three more unbounded whole-archive collectors after the integrity probe (48.6 GB of blob hashing, uncapped) | `.claude/worktrees/cass-p3kgr-gen13/.beads/issues.jsonl` |
| `coding_agent_session_search-lj72p` | in_progress | 0 | cass doctor never returns on a large archive: unbounded full-database PRAGMA integrity_check | `.claude/worktrees/cass-p3kgr-gen13/.beads/issues.jsonl` |
| `coding_agent_session_search-iekel` | open | 1 | APPROVAL NEEDED: delete 13 GiB of killed-rebuild index shards in job c3b442f9 scratch (disk at 49 GiB vs 150 GiB floor) | `.claude/worktrees/cass-p3kgr-gen13/.beads/issues.jsonl` |
| `coding_agent_session_search-bzu1` | open | 1 | fix: Improve watcher reliability to prevent lost sessions | `.git/beads-worktrees/beads-sync/.beads/issues.jsonl` |

`gf1f0` was created 2026-08-17T12:42:16Z and `lj72p` last updated 12:43:09Z — the final writes to that worktree, three hours before main's 15:00:44Z flush. They were never flushed back. No live process holds the worktree (`ps` shows no cass or cass-worktree process; the only match was this retirement's own lane-log monitor), so the divergence is settled rather than in flight.

`bzu1` comes from the March 2026 `beads-sync` worktree, which holds 90 ids absent from main under an older id spelling (`-1oh`, `-2yg2`, `-rob.*`, `-tst.*`). 89 of those are closed; `bzu1` is the only open one. This is the residue of the "two divergent trackers for five months" incident that `.agent-config-beads-agreement-check-8fyh` was filed about.

Other worktree trackers differ from main only in *status* (they predate later closes), not in id set, except `cass-p3kgr-gen13` above.

Also checked and clean:
- `.beads/.br_history/` — 100 historical export files, 581 MB. **Zero ids absent from main** (positive control: a known main id does appear in the recent history files, so the scan was live).
- `tests/.beads/` and the six `worktrees/*/tests/.beads/` — empty scaffolds holding only `.gitignore`, `config.yaml`, `metadata.json`. No beads.
- `.beads/.br_recovery/` (69 MB), `.beads/.pre-migration-backup/` (15 MB), `.beads/recovery_20260509T004736Z/` (5.3 MB) — bead-state residue, retired with the repo.

---

## 4. Other trackers on this machine

305 `issues.jsonl` files under `~/dev`, `~/.agent-config`, `~/Documents`, `~/projects` (`~/Pictures`, `~/Desktop`, `~/Downloads`, `~/Library` deliberately not searched). 15 cass-owned, 119 live non-cass canonical, 171 worktree/agent-state snapshot copies.

Searched for: `coding_agent_session_search`, `coding[-_]agent[-_]search`, `frankensqlite`, `\bfsqlite\b`, word-boundary `cass`, `com.coding-agent-search`, `agent_search.db`, `.cass-catchup`, `share/cass`, `backups/cass`.

### Genuinely about cass — action needed

| tracker | bead | status | p | action |
|---|---|---|---|---|
| `~/.agent-config/.beads/issues.jsonl:575` | `.agent-config-mpsd` | open | 1 | **Close or rewrite.** "cass-*-target cargo caches are a TMPDIR family spec 279 does not reach (88 GB live)". Its whole subject is cass build residue that the retirement deletes. Its own measurement warns the caches were not all idle: `cass-repair-target` had 510 open handles feeding two live cargo test runs at measurement time. |
| `~/.agent-config/.beads/issues.jsonl:646` | `.agent-config-u2y6` | in_progress | 1 | **Rewrite, do not close.** "disk-janitor authority v3: Trash (24h age guard), cass fixture family, stale DeviceSupport" — one of three families is cass. The Trash and DeviceSupport halves are independent and still wanted. |
| `~/.agent-config/.beads/issues.jsonl:528` | `.agent-config-io58` | open | 2 | **Rewrite, do not close.** `--report-only` preview for the same three families, one of which is leaked cass fixtures. |
| `~/.agent-config/.beads/issues.jsonl:27` | `.agent-config-14bq` | in_progress | 1 | **Leave open, rewrite one clause.** "Repair and manage mini SSD backup" names `SSD-2/SSD-1-mirror/cass-mirror/agent_search.db` (6.4 GB) with no counterpart on SSD-1. That is an **off-machine cass data copy on an external SSD** and is not in the retirement inventory. After retirement it no longer needs preserving, which simplifies the bead. |
| `~/.agent-config/.beads/issues.jsonl:428` | `.agent-config-bn34` | in_progress | 1 | **Leave open.** The defect (per-entry unknown-hog tripwire never sums unmatched entries) is generic; cass fixtures are only the motivating example. Its line "Real cass index lives at `~/.local/share/cass`" goes stale on retirement. |
| `~/.agent-config/.beads/issues.jsonl:426` | `.agent-config-beads-agreement-check-8fyh` | open | 1 | **Leave open, but note a coupling.** The weekly DB-vs-JSONL agreement check is generic; cass is its motivating incident. If the check enumerates repos, it will keep trying to read the retired cass tracker and report a permanent disagreement — the retired repo needs excluding when the check is built. |

### Looks like cass but is NOT — leave alone

These matched only on `fsqlite`, and the sidecars they describe are written by **br**, not cass:

| tracker | bead | status | p | why not cass |
|---|---|---|---|---|
| `~/.agent-config/.beads/issues.jsonl:522` | `.agent-config-i7mm` | open | 2 | About `.beads/beads.db-fsqlite-ns-gate` / `-ns-use` leaking in migrated repos. br's own runtime artifacts. |
| `~/.agent-config/.beads/issues.jsonl:689` | `.agent-config-wxns` | open | 1 | About orphan `beads.db-fsqlite-ns-*` sidecars during beads repair. br's own. |
| `~/dev/groove-sight-platform/.beads/issues.jsonl:542` | `gsp-wy0q` | open | 0 | slack-rep writer has no `beads.db`, two orphan `beads.db-fsqlite-ns-*` sidecars. br's own. |

### Already gone — no action

| tracker | bead | status | note |
|---|---|---|---|
| `~/dev/orchestrator/.beads/issues.jsonl:468` | `orchestrator-nqq.2` | tombstone | "Search cass history for past logging work" — closed and deleted 2025-12-18. A record of *using* cass, not of depending on it. |
| `~/dev/orchestrator/.worktrees/pfizer-18kx/.beads/issues.jsonl:431` | `orchestrator-nqq.2` | tombstone | Stale worktree copy of the same. |

### Closed-only mentions (informational, no action)

`~/.agent-config/.beads/issues.jsonl` — `.agent-config-34m`, `.agent-config-pyw`, `.agent-config-tldraw-offline-collision-oxig`. `~/dev/agent-observer/.beads/issues.jsonl` — `obs-kap`. `~/dev/destructive_command_guard/.beads/issues.jsonl` — `bd-24nx`, `bd-2ikd`, `bd-2l3u`, `bd-hg8t`.

### The replacement repo already exists

`~/dev/groove-session-search` was created today, 2026-08-17T15:32:44Z, with the standard foundation doc set and one open bead: `gss-v5v` (p1) "Run the repo-foundation interview: settle the Core Promise, naming ledger, and first vertical slice; fill the foundation placeholders". Its three cass mentions are all in `thoughts/shared/handoffs/20260817-session-search-foundation/lanes/*.md` and all *forbid* running cass ("Do not run `cass`", "No corpus copies"). This is the natural destination for the carry-forward beads and is **not** a resurrection vector.

---

## 5. Dependency edges — nothing crosses a tracker boundary

The `dependencies` field is present on 1,182 of the 1,929 records, holding 3,042 edges. Each entry is `{issue_id, depends_on_id, type, created_at, created_by, metadata, thread_id}`; `issue_id` is always the hosting bead, so no edge is duplicated from the far side.

- **Foreign edges (an endpoint outside the cass id namespace): 0**
- **Dangling edges (a cass-prefixed endpoint absent from the tracker): 0**
- Edge types in play: `blocks`, `parent-child`

Verified with a positive control (`coding_agent_session_search-002 --blocks--> coding_agent_session_search-001`, both endpoints resolving) because an earlier pass of this same measurement used the wrong key names (`depends_on` instead of `depends_on_id`) and reported a meaningless zero over `None` targets. The zeros above are from the corrected extractor.

**Consequence: closing all 48 cass beads cannot orphan, unblock, or silently alter any bead in any other tracker.** There is no cross-repo bead coupling to sequence around.

Internal blocking, for close ordering: 10 of the 44 have at least one non-closed blocker. All are inside the two aspirational epics — `guided-ops-repro-trust-5u82n.{1,2,3,6,7,10,12,13}` and `swarm-coordination-intelligence-gnrxb.{9,10}`. The parent-child edges put all 12 open `5u82n` children under the open `5u82n` epic, all 4 open `gnrxb` children under the open `gnrxb` epic, `ibuuh.29.1` under closed `ibuuh.29`, and `oh96l`'s 11 already-closed children under the open `oh96l` epic.

`br` carries a `HAS_DEPENDENTS` error code. If a close is refused on that ground, close children before their parents and blocked beads before their blockers — the epics last.

---

## 6. Recommended close reason

One uniform reason, greppable, naming the decision and its evidence:

```
wont-do: cass retired 2026-08-17. Dale approved retirement after the five-lane viability
assessment found the tool structurally non-viable
(thoughts/shared/handoffs/20260817-cass-viability-assessment/). The tool, its binaries, data
and repo are gone; this is not deferred work. Findings worth keeping were carried forward —
see thoughts/shared/handoffs/20260817-cass-retirement/lanes/beads.md
```

For the 15 carry-forward beads in section 7, append one line naming the destination:

```
Finding carried forward to <destination repo/bead>.
```

Two beads should **not** get a boilerplate close:

- `coding_agent_session_search-mgw1o` — it explicitly says "NEEDS DALE'S APPROVAL BEFORE FILING" for an upstream frankensqlite report. Closing it silently discards a decision Dale reserved for himself.
- `coding_agent_session_search-iekel` — it is an open approval request to delete 13 GiB, and that 13 GiB is still on disk. It should be resolved (approved and reclaimed, or explicitly declined), not closed as won't-do.

---

## 7. Carry forward to the replacement — deliverable

The replacement should inherit what cass measured. Nine items belong in `~/dev/groove-session-search`; five are frankensqlite defects that belong with br; one belongs to asupersync.

### To the replacement (`~/dev/groove-session-search`, tracker `gss-*`)

1. **`p3kgr` (p0, in_progress) — the reason the replacement uses stock SQLite.** One lexical-prep aggregate over messages takes >20 min in frankensqlite and 0.03s in stock sqlite. Measured on shipped cass 0.6.9 (sha256 `3d044227…`). Both `cass index` and `cass index --watch-once` on a single 344-byte file sat in `phase="preparing"` past a 20-minute bound, with `last_progress_at_ms` byte-identical to `started_at`.
2. **`g0eyv` (p1) — codex session-file format fact.** 6,452 of 10,283 codex conversations in the archive carry zero tool rows. Two code paths disagreed on **every** modern codex rollout: on one real file, full scan produced 387 messages with zero tool rows where watch-once produced 1,297 — 30% of the file. The replacement must parse codex tool messages and should assert per-file message counts.
3. **`2d37b` (p2) — a whole corpus root with a format quirk.** `~/.pi/agent/sessions/--clawdbot-chip--` is a symlink to `~/.clawdbot/agents/main/sessions` holding 2,098 JSONL sessions with UUID-only filenames in a nested Pi-style message format. Indexing that path produced 0 conversations / 0 messages while a normal Pi control file produced 1 / 6. The replacement must follow symlinked roots and handle UUID-only names.
4. **`pi-agent-missing-workspaces-le8s1` (p2) — corpus size and a coverage-shape lesson.** pi_agent has 2,077 session files on disk, 1,876 indexed (90.3%). Of the 201 absent, 199 sit inside 41 workspace directories with zero indexed rows while 175 other directories are fully indexed. A per-directory coverage assertion catches this shape; a global percentage does not.
5. **`move-bundle-stale-hot-journal-gtfx5` (p1) — data loss that applies to stock SQLite too.** A `<db>-journal` left behind at a canonical path is a candidate for replay into a different, freshly created database at that path. Measured against **stock sqlite3 3.54.0**: a journal built from one database was played into a different populated database at the same path and destroyed it. Any bundle move in the replacement must carry or delete `-journal`.
6. **`jy8v8` (p1) — a design constraint.** A machine-global `index-run.lock` keyed to the *default* data dir rather than the invocation's `--db` let a scratch-DB probe acquire the production lock, write 6.6 GB of temp into the production data dir, and fail every default-db search on the machine with exit 7 for 5+ hours while heartbeating zero progress. Lock and temp paths must key to the database actually in use.
7. **`pfar8` (p1) — data preservation, and an ordering hazard for this retirement.** The raw mirror holds byte-complete, blake3-verified copies of **all 3,877 source-absent claude_code conversations** (1.87 GB of blobs, 0 missing). Those conversations exist nowhere else. The prune pins by recency rather than by upstream absence, so it would delete exactly the irreplaceable ones. **Dale should decide about these 3,877 before the 77 GB production data dir is deleted.**
8. **`k69vx` (p2) — an exit-code lesson.** `cass status` took 5.09s at ~5.35 GiB peak, printed "state database probe exceeded its 5000ms bound", "Database: Exists, but could not be opened", "Semantic: Status: error… db unavailable" — and exited 0. Agents reading the exit code saw success over a self-reported failure.
9. **`gf1f0` (p0) + `lj72p` (p0), both main-invisible — archive scale, and the unbounded-collector class.** `agent_search.db` measured 23,313,477,632 bytes (87x over a 256 MiB gate) and `raw-mirror/v1/manifests` held 147,844 files (289x over). A full-database `PRAGMA integrity_check` with no size bound emitted zero bytes in 300 s; three further whole-archive collectors sit ungated after it (~48.6 GB of blob hashing). Corpus scale for sizing the replacement: ~27,441 conversations / ~2.3M messages in a 23 GB probe archive. Every whole-archive collector needs a size gate.

### To agent-config / br — frankensqlite defects br also runs

br 0.2.22 embeds fsqlite 0.1.19, so these are live defects in software Dale uses in every repo, not cass trivia.

10. **`ns-sidecar-transport-bricks-db-1mgjd` (p2) — highest value of the five.** Never transport the `-fsqlite-ns-*` sidecars: the identity record's *content* self-heals, but the sidecar file's inode metadata does not, and a lost mode bit bricks the database permanently. **172 `.beads` directories on this machine carry these files, and `.agent-config-14bq` is an active bead about backing up and mirroring databases.** These two should be connected.
11. **`hd4u5` (p1).** The FTS write gate `AND rootpage > 0` silently disables all FTS maintenance under fsqlite >= 0.1.17. br uses fsqlite 0.1.19 FTS5.
12. **`xybl9` (p1).** A sidecar allowlist misses the `-fsqlite-ns-gate` / `-fsqlite-ns-use` family, reopening the orphan-amplification class on macOS and Linux. Pairs directly with `.agent-config-i7mm`.
13. **`d907f` (p3).** `frankensqlite_ext_fts5` stores column values even for contentless tables, against stock FTS5 semantics for `content=''`.
14. **`mgw1o` (p2) — needs Dale's decision, not a close.** An unsent upstream report: fsqlite 0.1.19 contentless FTS5 returns a stale `COUNT(*)` on the appending connection (1 / `[2]` against 2 / `[1,2]` on both fsqlite 0.1.5 and stock sqlite3 3.54.0; a fresh process on the same file reads correctly, so on-disk state is fine). The bead explicitly parks the report for Dale's approval.

### To asupersync (`~/dev/asupersync`, its own live tracker)

15. **`759l7` (p1).** A hand-rolled spin-wait on a std mpsc channel deadlocks under `block_on` at asupersync 0.3.4 and works at 0.3.2 — 16 tests hang forever rather than failing. Three named call sites (`src/update_check.rs:852`, `src/search/model_download.rs:1022`, and one more). The call sites die with cass, but the asupersync behavior change between 0.3.2 and 0.3.4 is worth recording in the owning repo.

### Operational, for the retirement itself (not carry-forward)

16. **`iapqz` (p1) + `6t64c` (p3) — expect git operations in this repo to fail.** `git fsck --connectivity-only --no-dangling` reports 16 broken links (8 missing trees, 6 blobs, 2 commits); six local branches (`beads-sync`, `feat/007-watchdog-subcommand`, `feat/doctor-reconciliation-v2`, `fix/index-gaps`, `fix/watch-state-skip-prevention`, `fix/watcher-cpu-spin`) error on a full history walk. An unbounded `git log --all` dies, and the dirtiness close-check's all-branch unpushed-commit computation hits the read errors. If the plan includes archiving, tagging, or pushing before deletion, budget for this.
17. **`iekel` (p1, main-invisible) — 13 GiB still on disk, outside the known inventory.** `/Users/dalecarman/.claude-accounts/katherine/jobs/c3b442f9/tmp/acceptance-data/index` measured **13G just now**. Partial Tantivy shards from two `cass search` runs killed at 240 s on 2026-08-17; incomplete by construction, nothing depends on them. The bead is an open approval request because the creating session could not delete.

---

## 8. Verified negatives

Each of these is a true negative with a live positive control, not an empty result.

- **No launchd service invokes cass.** `launchctl list` names nothing matching cass (positive control: 613 rows, 543 matching `com.apple`). No plist filename matches `*cass*` or `*coding-agent*` across `~/Library/LaunchAgents` (33 plists), `/Library/LaunchAgents` (12), `/Library/LaunchDaemons` (22). No plist **body** references `coding-agent-search`, `/cass`, `share/cass`, `com.cass`, `cass index`, `cass watchdog`, or `agent_search` (positive control: the same instrument matched `ProgramArguments` in 26 files). Beads `-2gif2` and `-1vxuf` describe `com.cass.health-watchdog` as loaded-but-failing and `com.cass.index-watch` as absent; **both plists are already gone**, so those bead descriptions are stale on that point and the coordinator should not hunt for them. Only these two service names appear anywhere in the tracker (10 mentions of `com.cass.index-watch`, 5 of `com.cass.health-watchdog`).
- **No cass process is running.** `ps -Ao pid,etime,command` matched no cass or cass-worktree process; the single hit was this retirement's own lane-log monitor.
- **No bead ids hide in the history exports.** 100 files, 581 MB, zero ids absent from main.
- **No beads in the `tests/.beads` fixtures.** Seven such directories, all empty scaffolds.

### Instrument failures caught and corrected during this lane

Recorded because each produced a plausible false answer:

- `fd -H -I -t f -e plist ~/Library/LaunchAgents` returned **0** for a directory holding 33 plists. `-e plist` consumes its value, so the directory became the *regex pattern*. Cross-checked against `/bin/ls` and re-run as `fd … -e plist . <dir>`, which returns 33. The false zero would have published "no cass plists on disk" off a dead instrument.
- `fd -H -t f -e db` missed every `beads.db` because `.beads/.gitignore` ignores `*.db`; `-I` is required.
- The first dependency extractor read `depends_on` where the schema is `depends_on_id`, so "foreign edges: 0" and "dangling edges: 0" were computed over `None` and proved nothing. Re-run with a positive control on a known-resolving edge.
- The first cross-tracker scan split `fd` output on whitespace and died on `~/projects/Bryan Rigg/…`, aborting partway through 307 trackers with no error visible in the results. Re-run with `-0` and null splitting for full 305-tracker coverage.
