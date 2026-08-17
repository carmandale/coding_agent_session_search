# What the 8 forward-line failures are, and what moving the fsqlite pin costs

Bead: `coding_agent_session_search-759l7`
Branch: `worktree-cass-759l7-spin-wait`
Written by generation 13; evidence trail in `agent-log-gen13.md` beside this file.

---

## The short version

Moving cass forward from **fsqlite 0.1.5 / asupersync 0.3.2 / rustc 1.94** to
**fsqlite 0.1.19 / asupersync 0.3.10 / rustc 1.99** breaks 8 tests. None of them
is data corruption, and none makes search return a wrong answer. Six of the eight
are cass asserting something about the old library's private behavior that the
new library legitimately corrected — in every one of those cases 0.1.19 now
matches stock SQLite and 0.1.5 did not.

Two findings changed after this document was first written, and both are stated
in full rather than folded in, because the superseded versions were reported to
the operator first:

- **One failure (row 7) is a production defect, not a test artifact.** cass
  detects its FTS table with `rootpage > 0`, which is false on stock SQLite. Under
  0.1.19 that gate goes false and **every db-resident FTS write on the ordinary
  insert path silently stops** — ten call sites in `insert_conversation_tree`.
  Bounded: Tantivy is authoritative and this is the fallback index, so search
  results are unaffected. But it is real, it reaches the live database, and it
  is the single most important line in this document.
- **One failure (row 8) is a genuine fsqlite 0.1.19 regression** — the only one.
  A transient row undercount after reopen; no data loss, `MATCH` always correct,
  self-corrects on the next open. It costs a full rebuild where an incremental
  catch-up would have done. Fixable cass-side or waitable upstream.

Against that, the item previously flagged as the most expensive got **cheaper**:
a verifier refuted the claim that rows 5 and 6 destroy a recovery path. They
don't — two real damaged databases were measured and both still open fine under
0.1.19. Those tests block because their fixtures stopped resembling real damage,
which is a re-adjudication rather than a rescue.

| what | how much |
|---|---|
| trivial edits (literal strings, one assertion, two allow-list entries) | **4 of 8 failures**, 3 files |
| small test re-adjudication toward the real-world shape, needs a written decision | **2 of 8 failures**, 1 file |
| production gate fix — stop asking `rootpage > 0` | **1 of 8**, and the one that matters |
| upstream regression — fix locally or wait for fsqlite | **1 of 8**, optional |

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

Every classification below was handed to an adversarial verifier told to refute
it and to default to "refuted" if it could not independently confirm the
load-bearing claim. **Seven verdicts returned. Six confirmed; one refuted.** The
group that could genuinely block got three verifiers with different lenses
rather than one, because the failure mode there is data-loss shaped — and the
third of those three is the one that fired.

Two verifiers are worth singling out.

The first confirmed more than it was asked to: the `fts-shadow-table` lane had
flagged its rootpage mechanism as an "honest gap" — deduced from source rather
than executed — and the verifier closed it with a fourth line of evidence the
finder did not have, then found a real specimen on disk
(`~/Desktop/cass-backups-parked/agent_search.db.backup.3963…`, `fts_messages` at
rootpage 74) proving cass-on-older-fsqlite really did materialize with a
positive rootpage.

The second **refuted the reachability story for rows 5 and 6** — on reasoning,
not on label. The classification and "blocks the pin" both survive; the reason
they block is now close to the opposite of what the finder argued, and the fix
is cheaper and lower-risk as a result. That correction is written into the 5-and-6
section below rather than quietly folded in, because the superseded version was
reported to the operator before the verdict landed.

| # | failure | what it actually is | blocks the pin | effort |
|---|---|---|---|---|
| 1 | `dependency_drift::…manifest_pin_reads_git_and_registry_dependency_specs` | the experiment's own pin move, asserted against | no | trivial |
| 2 | `pages::encrypt::…key_slot_id_for_len_rejects_overflow` | rustc/std changed a message; nothing to do with fsqlite | no | trivial |
| 3 | `storage::sqlite::…salvage_historical_databases_imports_backups_once_and_merges_overlap` | fsqlite's new namespace sidecar counted as a database bundle | yes, but two strings | trivial |
| 4 | `storage::sqlite::…salvage_historical_databases_skips_unreadable_quarantined_bundles` | same cause as 3 | yes, same fix | trivial |
| 5 | `storage::sqlite::…franken_storage_open_repairs_duplicate_fts_messages_schema_rows` | the fixture's damaged shape stops reproducing the real-world one it was written to protect | yes | small |
| 6 | `storage::sqlite::…rebuild_fts_via_rusqlite_cleans_duplicate_legacy_schema_rows` | same cause as 5 | yes, same fix | small |
| 7 | `indexer::…full_run_fallback_fts_repair_skips_rebuild_when_fts_is_already_healthy` | **cass's own `rootpage > 0` gate goes false, silently disabling FTS writes on the ordinary insert path** | yes | small |
| 8 | `storage::sqlite::…ensure_fts_consistency_via_rusqlite_catches_up_missing_rows` | a real fsqlite 0.1.19 regression — transient undercount, no data loss | yes | small, or wait for upstream |

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

**A superseded claim, kept visible because it was reported before it was
checked.** The finding lane argued — and I relayed to the operator — that the
repair "becomes unreachable": `FrankenStorage::open` opens the connection as its
first act, `DatabaseCorrupt` is not retryable, so a case cass self-heals today
becomes a hard failure needing a 23 GB rebuild. **The third verifier refuted
that, with executed evidence, on three points.**

1. *The claim that every `writable_schema` site is test-gated is false.* Two
   production paths shell out to the real `sqlite3` binary with
   `PRAGMA writable_schema=ON`: `probe_historical_bundle_via_sqlite3_metadata`
   (`sqlite.rs:2180-2196`) and `scrub_staged_derived_fts_metadata_via_sqlite3`
   (`sqlite.rs:2479-2498`). Neither is `#[cfg(test)]`.
2. *The cited range refutes itself.* `sqlite.rs:2141-2142` — inside the very
   range quoted for "the probe also opens through fsqlite" — is
   `let Ok(conn) = open_historical_bundle_readonly(root_path) else { return probe_historical_bundle_via_sqlite3_metadata(root_path).unwrap_or_default(); };`
   That is the fallback for a bundle fsqlite cannot open, with a second at 2174.
   The verifier executed cass's production probe SQL against a duplicate-row
   database it built: `13 / 2 / 1` at rc=0 — the exact `Some(2)` the ranker keys
   on. Healing is wired too: `ensure_seeded_canonical_fts_consistency`
   (`sqlite.rs:2539-2570`) catches this error class and runs the scrub. The
   verifier executed that scrub: rc=0, duplicate removed, integrity check clean.
   So "probed" and "healed" are both false; only "opened via fsqlite" is true,
   and that is a recognized, handled branch.
3. *The decisive one — the fixture is not what the wild contains.* The
   `shadowed_by_materialized` mask is built by filtering **the file's own**
   `sqlite_master` rows on `root_page_num > 0`, and that code is byte-identical
   between 0.1.5 and 0.1.19. Two genuinely damaged cass databases were measured
   on this machine (`~/Desktop/cass-backups-parked/agent_search.db.pre-rebuild-bak`,
   10.5 GB, and `backups/agent_search.db.1774667229.failed-baseline-seed.bak`,
   11.3 GB): both have `COUNT(*) WHERE name='fts_messages'` = 2, both carry a
   positive-rootpage materialized twin (2137238 / 2367531), and **both carry
   `fts_messages_content`** (rootpage 23 and 17). So under 0.1.19 they are still
   masked, still skipped, still openable. The failing fixture has neither
   property — it needs both rows at rootpage 0 *and* a contentless canonical
   table so `_content` never exists — and cass cannot produce that: its two
   production `writable_schema` paths only DELETE rows, and stock SQLite refuses
   a second `CREATE VIRTUAL TABLE` outright.

**So rows 5 and 6 block for the opposite reason.** Not because a production
recovery capability is destroyed, but because the fixtures no longer reproduce
the real-world damage they exist to protect against. That makes the fix cheaper
and the risk lower than the superseded version implied.

**Can your database hit this?** Not as it stands. Measured read-only against the
live 23 GB file: 71 objects in `sqlite_master`, schema version 20, **zero**
objects whose name contains "fts", no virtual tables of any kind, no duplicate
names. Positive and negative controls both fired. That is not a mid-rebuild
artifact either — `MIGRATION_V14` drops `fts_messages` and its own comment says
the contentless table is "recreated lazily after open() only when the
frankensqlite FTS consistency check finds it missing or malformed". The primary
index is Tantivy, on disk with 211 entries. A schema-20 database with no FTS
objects is exactly what the code predicts.

Nor does it cost you the historical databases. That was the superseded reading —
"any pre-V14 backup or salvage bundle a user still holds" — and the two real
specimens above are exactly those files. They do not take this path under
0.1.19. cass's own commit `e4796ba6` attributes the duplicate state to
"interrupted migrations or concurrent schema operations", and it is right that
the state is production-reachable; what is *not* production-reachable is the
fixture's particular variant of it.

**Fix: re-adjudicate the two fixtures toward the real specimen shape.** Rebuild
each around what the wild actually contains — a materialized twin at positive
rootpage with `_content` present — so the test again exercises the recovery path
on a state that can occur, and add a case pinning the new behavior. Two notes on
scope: this must not be a silent re-arm, because the shape being changed is a
documented recovery path and the record has to say why; and the previously
proposed "add a rusqlite pre-flight" option should be **dropped outright** —
cass already ships that capability in production through the `sqlite3` binary at
`sqlite.rs:2479-2498`, so it was never a new-dependency question.

**Worth filing upstream regardless of the pin decision**, and it is narrow: do
*not* report the duplicate case, where fsqlite is more correct than 0.1.5 was.
Report the **orphan** case, where fsqlite genuinely diverges from stock — given
a single rootpage-0 fts5 row whose shadow tables are absent, real SQLite opens
the database and fails lazily only on first access to that table, while fsqlite
fails eagerly during schema reload and makes the whole file unopenable. That
divergence stands on its own merits and is worth reporting; note that the
"and it would restore cass's repair reachability for free" argument attached to
it in the superseded version does not survive, since the reachability was never
lost.

---

## 7 and 8 — the one I would not have guessed

These two share a cause with 5 and 6 — the same `rootpage` correction — but they
land somewhere far more consequential, and only one of them is a test problem.

**7 is a production defect, not a test artifact.** cass decides whether an FTS
table exists with this, at `src/storage/sqlite.rs:4127-4131`:

```sql
SELECT COUNT(*) FROM sqlite_master
 WHERE name = 'fts_messages' AND rootpage > 0
```

Real SQLite writes `rootpage = 0` for a virtual table. fsqlite 0.1.5 wrote 2-3,
so the gate returned 1; 0.1.19 writes 0 like SQLite, so **the gate returns 0 and
cass concludes the FTS table is absent.** Every db-resident FTS write sits behind
it — `flush_pending_fts_entries` at `sqlite.rs:15292`, reached from ten call
sites inside the production `insert_conversation_tree` (9213, 9236, 9329, 9351,
9494, 9521, 9665, 9691, 9837, 9860). Under 0.1.19 those writes silently no-op on
every index run against the real database, and the fallback index only catches
up on the next `cass index --full`.

Measured, not read. A standalone probe was linked against both prebuilt rlibs
(a version/compiler mismatch fails loudly at `E0514`, so a silent mislink is
impossible):

| | `fts_messages` rootpage | cass's gate | shadow tables |
|---|---|---|---|
| fsqlite 0.1.5 | 3 | `1` | none |
| fsqlite 0.1.19 | 0 | `0` | all five |
| stock sqlite3 3.54.0 | 0 | `0` | all five |

The positive control ran in the same probe on the same databases: the identical
gate query against the ordinary `messages` table returns `1` on both pins, so the
zero is a real zero rather than a dead query. The gate's own stated rationale
(`sqlite.rs:1155-1160`, "FrankenSQLite skips virtual-table entries") was also
falsified directly — under 0.1.19 the rootpage-0 table accepted an INSERT,
answered `COUNT(*)`, and answered `MATCH`, on both the creating connection and a
fresh reopen. The discriminator is simply obsolete.

**How bad is it?** Bounded, and worth stating precisely. Tantivy is the
authoritative index; the SQLite FTS path is consulted only when Tantivy is
unavailable (`src/search/query.rs:3596`). So the failure degrades a fallback, not
primary search. The fix is to stop asking `rootpage > 0` and ask something that
is true on stock SQLite.

**8 is a genuine fsqlite 0.1.19 regression** — the only one in the eight, and I
was wrong to tell the operator there were none before this landed. In
`fsqlite-ext-fts5-0.1.19/src/lib.rs:7480`,
`hydrate_contentless_index_from_segments` ends by setting `self.shadow_rows =
None` without repopulating `self.documents`; `row_count()` at 7837 then falls
through to `self.documents.len()`, which holds only the newly inserted row. On a
contentless table with one persisted row, reopened, then given one catch-up
insert:

| | `COUNT(*)` | rowids |
|---|---|---|
| fsqlite 0.1.5 | 2 | 1, 2 |
| fsqlite 0.1.19 | **1** | 2 |
| stock sqlite3 3.54.0 | 2 | 1, 2 |

An ordinary table carried through the identical sequence returned 2 on both pins,
which is what proves the instrument. Here 0.1.5 agrees with upstream and 0.1.19
does not.

**It is not data loss, and I checked rather than assumed.** The undercount is
transient: reopen once more and `COUNT(*)` is 2 with both rowids, and `MATCH`
stayed correct even inside the bad window — so search never returns a wrong
answer. What it costs is efficiency. cass reads that count as `repaired_rows`
(`sqlite.rs:10176-10181`), sees it fall short, and takes the full-rebuild branch
at `sqlite.rs:10213-10217` instead of the incremental one. The end states are
equivalent by construction: `rebuild_fts_via_frankensqlite` recreates the table
and streams one row per indexable message, a superset of what the incremental
path would have written. So this one can be fixed cass-side, or simply waited out
upstream — it is the one item on the list that does not have to be your problem.

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
