# What the 8 forward-line failures are, and what moving the fsqlite pin costs

Bead: `coding_agent_session_search-759l7`
Branch: `worktree-cass-759l7-spin-wait`
Written by generation 13; evidence trail in `agent-log-gen13.md` beside this file.

---

## The short version

Moving cass forward from **fsqlite 0.1.5 / asupersync 0.3.2 / rustc 1.94** to
**fsqlite 0.1.19 / asupersync 0.3.10 / rustc 1.99** breaks 8 tests. None of them
is data corruption, and none is a case of the newer library returning wrong
answers. Five are two-line fixes. The real cost is one small test
re-adjudication that retires a documented recovery path, plus one upstream
report worth filing whatever you decide.

The headline number is smaller than "8 failures" sounds:

| what | how much |
|---|---|
| trivial edits (literal strings, one assertion, two allow-list entries) | **5 of 8 failures**, 3 files |
| small test re-adjudication, and it needs a written decision | **2 of 8 failures**, 1 file |
| *(the eighth is covered below once its lane lands)* | |

## The controlled differential

Both binaries were verified by grepping them for version markers, never by
timestamp — this repo has a recorded incident of a stale binary reporting a
false green.

| | shipping | forward |
|---|---|---|
| markers found in the test binary | `fsqlite-core-0.1.5`, `asupersync-0.3.2` | `fsqlite-core-0.1.19`, `asupersync-0.3.10` |
| rustc | 1.94.0-nightly (f52090008) | 1.99.0-nightly (969b803cb) |
| the 8 tests, each run alone | **8 pass** | **8 fail** |

Every forward failure was reproduced in isolation, so none of them is
contention with the indexing job that has been running against the live
database for the last seven hours. Three sibling `salvage_historical_databases_*`
tests pass on the forward line, so the salvage failures are specific rather than
a broken fixture.

---

## The eight, one row each

*(verdict column filled in from the adversarial verifiers; see the detail
sections for the reasoning behind each)*

| # | failure | what it actually is | blocks the pin | effort |
|---|---|---|---|---|
| 1 | `dependency_drift::…manifest_pin_reads_git_and_registry_dependency_specs` | the experiment's own pin move, asserted against | no | trivial |
| 2 | `pages::encrypt::…key_slot_id_for_len_rejects_overflow` | rustc/std changed a message; nothing to do with fsqlite | no | trivial |
| 3 | `storage::sqlite::…salvage_historical_databases_imports_backups_once_and_merges_overlap` | fsqlite's new namespace sidecar counted as a database bundle | yes, but two strings | trivial |
| 4 | `storage::sqlite::…salvage_historical_databases_skips_unreadable_quarantined_bundles` | same cause as 3 | yes, same fix | trivial |
| 5 | `storage::sqlite::…franken_storage_open_repairs_duplicate_fts_messages_schema_rows` | a repair path for historically-damaged databases is now unreachable | yes | small |
| 6 | `storage::sqlite::…rebuild_fts_via_rusqlite_cleans_duplicate_legacy_schema_rows` | same cause as 5 | yes, same fix | small |
| 7 | `indexer::…full_run_fallback_fts_repair_skips_rebuild_when_fts_is_already_healthy` | *(pending)* | | |
| 8 | `storage::sqlite::…ensure_fts_consistency_via_rusqlite_catches_up_missing_rows` | *(pending)* | | |

---

## 1 and 2 — not really about the pin at all

**`dependency_drift`** parses cass's own checked-in `Cargo.toml` and asserts the
version strings equal literals hardcoded in the test
(`src/dependency_drift.rs:869` and `:882`, `"0.1.5"` and `"0.3.2"`). The
experiment moved the manifest and left the literals alone, so the test did
exactly its job. Fix: update the two literals in lockstep whenever the pin
actually moves.

**`pages::encrypt`** pins an error message whose second half belongs to std.
`key_slot_id_for_len` (`src/pages/encrypt.rs:298-306`) interpolates
`u8::try_from`'s error with `{}`, and rustc 1.99 rerouted `TryFromIntError`
onto the same descriptions `ParseIntError` already used. An executed probe
settles it — the same three-line program compiled by each toolchain prints
`out of range integral type conversion attempted` under 1.94 and `number too
large to fit in target type` under 1.99 — and `strings` on each shipped
`libcore` agrees: the old literal is gone from 1.99 entirely.

Worth stating plainly: **this one fires on any move to rustc 1.99, with or
without the fsqlite pin.** Fix: stop asserting equality over std's half of the
message at `src/pages/encrypt.rs:1825`.

---

## 3 and 4 — fsqlite now leaves a file behind, and cass counts it as a database

fsqlite 0.1.19 added `fsqlite-vfs/src/namespace.rs`, which does not exist in
0.1.5. It writes two persistent sidecars beside every opened database —
`-fsqlite-ns-gate` (0 bytes) and `-fsqlite-ns-use` (40 bytes) — and its own
module doc says they are "deliberately never unlinked". They are created
*before* the file is validated, and on the read-only path too, so **even a file
that turns out not to be a database gets a permanent 40-byte record written
next to it.** Stock SQLite leaves nothing behind in that case.

cass's historical-bundle discovery takes any file in the data directory whose
name starts with `agent_search.corrupt.` or `agent_search.db.backup.`, skipping
a hardcoded list of sidecar suffixes that knows only `-wal`, `-shm` and three
Windows lock names (`src/storage/sqlite.rs:3008-3017`). So when salvage probes a
quarantined `agent_search.corrupt.<ts>`, fsqlite drops
`agent_search.corrupt.<ts>-fsqlite-ns-use` beside it — which matches the prefix,
is not in the skip list, and at 40 bytes survives the `total_bytes > 0` filter.
The next discovery counts it as a bundle.

Nothing bad is imported from it. Real SQLite rejects the 40-byte record at exit
26, which drives cass's own "zero recovered rows" bail, so the bundle is skipped
with a warning and no row reaches search results. Two costs remain, and they are
why this still has to be fixed before the pin moves:

- **It feeds itself.** Probing the phantom creates *its* sidecar, which also
  matches the prefix, so `bundles_considered` grows by one on every salvage run,
  unbounded, with two wasted `sqlite3` subprocesses each time.
- **Backup retention can be diluted.** `is_backup_root_name` deliberately does
  not use the sidecar skip list, and `cleanup_old_backups` keeps only
  `MAX_BACKUPS = 3` newest-first — so freshly-mtimed junk can occupy retention
  slots and prune real backups. *This one was reasoned from source and not
  executed; treat it as a follow-up to size, not a verified defect.*

**Fix: two strings** — add `"-fsqlite-ns-gate"` and `"-fsqlite-ns-use"` to
`SIDECAR_SUFFIXES` at `src/storage/sqlite.rs:3009-3015` and update the doc
comment above it. That restores both assertions exactly. Do not relax the tests
to the new numbers, and do not filter on size — 40 bytes is upstream's constant
to change.

**Already live on your machine.** Your real data directory already holds
`agent_search.db-fsqlite-ns-gate` (0 bytes) and `agent_search.db-fsqlite-ns-use`
(40 bytes), so a 0.1.19-family binary has already run against the 23 GB
database. Those two are harmless — they sit beside `agent_search.db`, which
matches neither salvage prefix. The phantom only appears once a
`agent_search.corrupt.*` or `*.backup.*` file exists, which is exactly what
`cass doctor`'s repair path creates.

---

## 5 and 6 — the one that costs something real

Under 0.1.19, opening a database whose `sqlite_master` holds a **duplicate
legacy `fts_messages` row** fails outright:

```
database disk image is malformed: FTS5 table `fts_messages`
is missing required content shadow table `fts_messages_content`
```

**Why 0.1.5 accepted it.** The check itself is byte-identical in both versions.
What changed is the rootpage fsqlite writes for its own FTS5 catalog rows, and
upstream inverted its own assertion in the same-named test: 0.1.5 asserts
`root_page > 0`, 0.1.19 asserts `root_page == 0` with the reason "must use a
stock-compatible rootpage=0 catalog row". Under 0.1.5 the healthy table had a
positive rootpage, so it masked the duplicate through a
`shadowed_by_materialized` skip and the bad row was never validated. Under
0.1.19 both rows are rootpage=0, the mask is gone, and the legacy row reaches
the check. **0.1.5's tolerance was an accident of a non-stock representation,
not a designed repair affordance.**

**Real SQLite rejects this file harder than fsqlite does.** Executed against
sqlite3 3.54.0: after injecting the duplicate row, *every* statement fails with
`malformed database schema (fts_messages) - table fts_messages already exists`,
including a plain `SELECT` on an unrelated table. An isolating probe injecting a
byte-identical *contentless* duplicate produces the same error, proving the
rejection is about the duplicate name rather than the missing shadow table. On
this fixture 0.1.19 is the more correct of the two.

**But the repair really does become unreachable.** `FrankenStorage::open` opens
the fsqlite connection as its first act, before migrations and before
`ensure_search_fallback_fts_consistency`. Every production repair route goes
through that open, and `DatabaseCorrupt` is not retryable. So a case cass
currently self-heals becomes a hard failure whose operator remedy is the one
cass already writes down — back up and rebuild — for a 23 GB database.

**Can your database hit this?** Not as it stands. Measured read-only against the
live 23 GB file: 71 objects in `sqlite_master`, schema version 20, **zero**
objects whose name contains "fts", no virtual tables of any kind, no duplicate
names. Positive and negative controls both fired. That is not a mid-rebuild
artifact either — `MIGRATION_V14` drops `fts_messages` and its own comment says
the contentless table is "recreated lazily after open() only when the
frankensqlite FTS consistency check finds it missing or malformed". The primary
index is Tantivy, on disk with 211 entries. A schema-20 database with no FTS
objects is exactly what the code predicts.

What the pin costs is therefore **not your live database — it is the ability to
recover a database that reached the damaged state in the past**, including any
pre-V14 backup or salvage bundle still on disk. cass's own commit `e4796ba6`
describes that state as arising from "interrupted migrations or concurrent
schema operations", and the salvage ranker has a branch for it, so it is a state
cass has met in the field and built machinery for.

**Fix: re-adjudicate the two fixtures, and write down why.** Split each into a
test that keeps the real repair signal on a state 0.1.19 can open, plus a new
test pinning the new behavior. This is the one item that must not be a silent
re-arm — the coverage being retired is a documented recovery path, and the
record has to say so.

**Worth filing upstream regardless of the pin decision**, and it is narrow: do
*not* report the duplicate case, where fsqlite is more correct than 0.1.5 was.
Report the **orphan** case, where fsqlite genuinely diverges from stock — given
a single rootpage-0 fts5 row whose shadow tables are absent, real SQLite opens
the database and fails lazily only on first access to that table, while fsqlite
fails eagerly during schema reload and makes the whole file unopenable. Asking
for that validation to be deferred to first use would also restore cass's repair
reachability for free, because the repair drops and recreates the table without
ever querying it.

---

## The two decisions that are yours

### 1. Merging `1fc20dbb` to `main`

The 759l7 fix — three hand-rolled spin-waits replaced by awaiting the spawned
task's `JoinHandle` — is verified green on the shipping pin (5151 passed, 0
failed) and pushed on `worktree-cass-759l7-spin-wait`. **`main` is still at
`c4b3f955` and still carries the spin.** This session cannot merge: the
background-session harness forbids pushing to `main` and forbids merging.

This is independent of everything above. The fix is good on the pin you ship
today.

### 2. The toolchain, which is what actually gates the pin

Worth correcting a framing carried in earlier handoffs: this is not a ceiling so
much as a **stale toolchain**.

| fact | source |
|---|---|
| asupersync 0.3.9/0.3.10 depend on `sysinfo`; 0.3.2 does not | their `Cargo.toml` |
| sysinfo 0.39.6 declares `rust-version = "1.95"` | its `Cargo.toml` |
| the forward lock resolved sysinfo 0.39.6; the shipping lock has no sysinfo at all | both `Cargo.lock` |
| `rust-toolchain.toml` pins bare `nightly`, and the installed nightly is rustc **1.94.0-nightly, dated 2025-12-10** | the file, and `rustc --version` |

So the compiler this repo builds with is eight months old. Two ways forward,
with very different blast radii:

- `rustup update nightly` — clears it, and changes the compiler for **every repo
  on this machine at once**.
- pin `nightly-2026-08-10` in `rust-toolchain.toml` — clears it for this repo
  only, but still changes the compiler for **every other session and worktree in
  this repo at once**, which is why no session has done it unasked.

Neither is a cass code problem, and both are yours to call.

---

## What has not been measured, stated plainly

Every green and red number in this chain — 5151/0 shipping, 5143/8 forward — is
`cargo test --lib`. `Cargo.toml` declares 3 `[[test]]` targets and cargo
auto-discovers a further 209 top-level files under `tests/`. **None of that
surface has been run on either pin.**

It is also not cheap to run: the napkin records the e2e suite spawning 8
concurrent `cass index --full` against your real `~/.codex` and `~/.claude`
trees, isolating only the output, and not finishing in 90 minutes. So "100%
green" is true of the library suite and unmeasured beyond it. Closing that gap
is its own piece of work, and the first question in it is whether that suite
should be reading your real home directory at all.
