# Lane: bound-lifecycle — the deploy blocker (bead 1a7mk)

Read-only grounding lane. Only write is this file. Repo HEAD at lane start: `37d52925`
(`beads(nvq59): the status --json hang is a 20 GB raw-mirror walk`).

No cargo was run. No index/doctor/models command was run. Two read-only cass commands were
run against the **installed pre-fix binary** (`cass triage --json`, `cass health --json`) and
two stock-`sqlite3` reads with `mode=ro&immutable=1`. Every claim below is labelled
**MEASURED** or **INFERRED**.

---

## Headline

**Bead 1a7mk's stated root cause is contradicted by measurement.** The bead says "the cost is
entirely in frankensqlite's open/close lifecycle, which the bound does not cover." Measured
today on the live 7.93 GB archive, the *entire* frankensqlite open → FTS-validate → two meta
reads → close lifecycle costs **40 ms**. It is not the cost.

The only new database work commit `e3ed01f0` adds to the three hanging surfaces is **one
`SELECT value FROM meta WHERE key = ?1` that returns zero rows on this archive.** On `triage`
and `stats` that query is the *entire* delta — those surfaces gained no open and no close.
Bounding the open/read/close lifecycle therefore cannot be the fix: it would fix `health` and
leave `triage`, `status` and `stats` hanging.

---

## 1. The code, verbatim

### 1.1 `read_connector_scan_floors_bounded` — src/lib.rs:15092-15108

```rust
15092  /// How long `cass health` will wait to open the database purely to answer the
15093  /// coverage question. Health is a fast surface, so it prefers reporting
15094  /// `checked: false` over blocking on a contended archive.
15095  const HEALTH_COVERAGE_OPEN_TIMEOUT: Duration = Duration::from_secs(2);
15096
15097  /// Read the coverage floors through a short bounded open, for surfaces whose
15098  /// state probe elided the database open entirely (`cass health`).
15099  fn read_connector_scan_floors_bounded(
15100      db_path: &Path,
15101      timeout: Duration,
15102  ) -> Option<BTreeMap<String, i64>> {
15103      let conn =
15104          open_franken_cli_read_db(db_path.to_path_buf(), "connector-coverage", timeout).ok()?;
15105      let floors = read_connector_scan_floors(&conn);
15106      let _ = close_franken_cli_read_db(conn, db_path, "connector-coverage");
15107      Some(floors)
15108  }
```

`HEALTH_COVERAGE_OPEN_TIMEOUT` has exactly **two** occurrences in the tree: its definition at
15095 and its single use at 65457. MEASURED (`git grep -n HEALTH_COVERAGE_OPEN_TIMEOUT`).

### 1.2 The word "bounded" is wrong in three places, not one

The bead names one (read and close are unbounded). There are three. MEASURED from source:

| layer | what `timeout` actually does there |
|---|---|
| `open_franken_cli_read_db` (lib.rs:14066) | passes `busy_timeout` to `open_franken_readonly_storage_with_timeout`, then `PRAGMA busy_timeout = <ms>`. Not a wall clock. |
| `open_franken_readonly_storage_with_timeout` (storage/sqlite.rs:637-665) | `timeout` bounds only the **retry loop between attempts**; a single `FrankenStorage::open_readonly(path)` call that blocks internally never consults `deadline`. |
| `FrankenStorage::open_readonly` (storage/sqlite.rs:4172-4174) | **ignores the caller's timeout entirely** — hardcodes `DOCTOR_MUTATION_DB_OPEN_LOCK_TIMEOUT` (storage/sqlite.rs:42 = **30 s**) for its lock guard. So a 2 s "bound" can legally wait 30 s inside the open. |
| `read_connector_scan_floors` (lib.rs:15077) | no timeout parameter at all. |
| `close_franken_cli_read_db` (lib.rs:14224) | no timeout parameter at all. |

Same shape as the retry-vs-block confusion in `franken_query_row_map_retry`
(lib.rs:14259-14288): `CLI_DB_QUERY_RETRY_TIMEOUT` (lib.rs:15066 = 10 s) bounds *retries of
retryable errors*, not one blocking `query_row_map`.

### 1.3 What each function actually does

`open_franken_cli_read_db` (lib.rs:14066-14125): existence check → try
`open_franken_readonly_storage_with_timeout` → on failure try
`open_franken_raw_readonly_connection_with_timeout` → `PRAGMA busy_timeout` + `PRAGMA
query_only = 1`. Both open paths reach `open_franken_with_flags(path,
SQLITE_OPEN_READ_ONLY)` = fsqlite `compat::open_with_flags` →
`open_read_only_connection` → `Connection::open_schema_only(path)`. fsqlite's own doc
(fsqlite-0.1.5/src/compat/flags.rs:184-190): *"table/index/view/trigger definitions are loaded
but no row data is read into the in-memory MemDatabase … makes opening even multi-gigabyte
databases near-instantaneous."*

`read_connector_scan_floors` (lib.rs:15077-15090):

```rust
15077  fn read_connector_scan_floors(conn: &frankensqlite::Connection) -> BTreeMap<String, i64> {
15078      use frankensqlite::compat::{ParamValue, RowExt};
15079
15080      franken_query_row_map_retry(
15081          conn,
15082          "SELECT value FROM meta WHERE key = ?1",
15083          &[ParamValue::from(
15084              crate::storage::sqlite::CONNECTOR_SCAN_FLOORS_META_KEY,
15085          )],
15086          |r| r.get_typed::<String>(0),
15087      )
15088      .map(|raw| crate::storage::sqlite::parse_connector_scan_floors(&raw))
15089      .unwrap_or_default()
15090  }
```

`close_franken_cli_read_db` (lib.rs:14224-14239): `conn.close_in_place()`, falling back to
`close_best_effort_in_place()` on error. fsqlite `close_in_place` →
`close_internal(false, true)` with `checkpoint_on_close = true`
(fsqlite-core-0.1.17/src/connection.rs:14271, 14291), but the passive WAL checkpoint is
guarded by `!self.pager.is_readonly()` (connection.rs ~14383), so a read-only handle does not
checkpoint. Independently confirmed cheap by the 40 ms live measurement in §4.

**Which one blocks?** On the evidence below: **none of open, read-of-a-present-key, or close.**
The block is specific to the *new* query. See §5.

---

## 2. Blast radius — every caller

MEASURED (`git grep -n`, HEAD `37d52925`).

`read_connector_scan_floors_bounded` — **one** call site:

```
src/lib.rs:65455    let connector_scan_floors = connector_coverage_floors_from_state(&state).or_else(|| {
src/lib.rs:65456        db_exists
src/lib.rs:65457            .then(|| read_connector_scan_floors_bounded(&db_path, HEALTH_COVERAGE_OPEN_TIMEOUT))
src/lib.rs:65458            .flatten()
src/lib.rs:65459    });
```

That is `run_health`, and only when the state envelope reports coverage unchecked — which it
always does for health, because `state_meta_json_for_health` (lib.rs:15754-15768) passes
`skip_db_open = true`. Confirmed live: the installed binary's `cass health --json` reports
`state.database.open_skipped = true` (§4.3).

`read_connector_scan_floors(&conn)` — **three** call sites, all on an already-open connection:

| site | function | surfaces reached |
|---|---|---|
| lib.rs:15283 | `probe_state_db` | `run_status` (64697-64698), `run_triage` (65176-65177), `cass search --robot-meta` (20502), `refresh_state_database_counts_if_needed` (16497) |
| lib.rs:23747 | `run_stats` | `cass stats`, `cass stats --json` |
| lib.rs:15105 | `read_connector_scan_floors_bounded` | `cass health` |

So the affected surfaces are **health, status, triage, stats, and robot-meta search** — five,
not three. Plain `cass search` never reaches it, which matches the bead's measured "search
unaffected".

`cass doctor` does **not** reach it: its readiness snapshot at lib.rs:69091-69099 passes
`skip_db_open = true`. So doctor's own hang is a separate bug and bead nvq59 stays separate.
INFERRED from the call graph; not measured against a doctor run (forbidden this lane).

Precedent path already in the repo, for contrast: `open_franken_cli_read_db_with_hard_timeout`
has three callers, all inside `cass doctor` (lib.rs:68339, 68484, 69386).

---

## 3. Bead 1a7mk — what it says, and whether the source supports it

Read from `.beads/issues.jsonl` (`br` not invoked; the JSONL is the tracked export).

Quoted verbatim:

> But src/lib.rs:15084 read_connector_scan_floors_bounded passes that timeout to
> open_franken_cli_read_db ONLY. The read_connector_scan_floors(&conn) and the
> close_franken_cli_read_db(conn, ..) that follow it are unbounded. So the stated
> 2s ceiling does not hold and 2s becomes >90s.
>
> The meta query is NOT the cost: stock sqlite3 with mode=ro&immutable=1 reads
> that table in 0.000s against the same live archive. The cass cost is entirely in
> frankensqlite's open/close lifecycle, which the bound does not cover.

(The bead's line numbers are its own snapshot: 15080/15082/15084/65440. At HEAD they are
15095/15098/15104/65455. Same code.)

**Supported by source:** the read and the close take no timeout. Confirmed above. The bead
undercounts — the *open* is not hard-bounded either, and its inner lock guard ignores the
caller's 2 s in favour of a hardcoded 30 s.

**Contradicted by measurement:** "the cost is entirely in frankensqlite's open/close
lifecycle." §4.2 measures that lifecycle at 40 ms on the live archive today. And the bead's own
`triage` row makes the same point against itself: `cass triage --json` went 0.09 s → >45 s while
gaining **no open and no close** — its coverage read happens on the connection `probe_state_db`
had already opened, and did already close, pre-fix. A lifecycle bound cannot explain a
regression on a surface whose lifecycle did not change.

**Supported and worth keeping:** the bead's `cass api-version` control (0.371 s OLD → 0.011 s
FIX) rules out the fix binary being a debug build, which would otherwise explain everything.
And its 6/6 alternating-trial comment rules out intermittency and archive contention.

**Attribution is clean — I tried to break it and could not.** I expected the June-1 specimen to
be confounded by ten weeks of commits. It is not. MEASURED:

```
$ git log --oneline --since=2026-06-01 --until=2026-08-11 -- src/ | wc -l
       2
$ git log --oneline --format='%h %ad %s' --date=short --since=2026-06-01 --until=2026-08-11 -- src/
193d2ad6 2026-08-10 fix(indexer): satisfy clippy and rustfmt for the coverage-floor change
e3ed01f0 2026-08-10 fix(indexer): an aborted connector scan can no longer claim complete coverage
$ git log --oneline --since=2026-06-01 --until=2026-08-11 -- Cargo.lock Cargo.toml
(no output)
```

Two commits, both the coverage-floor change, and no dependency movement. `193d2ad6` is
rustfmt/clippy only (68 changed lines, all reflow — `git show 193d2ad6`). So the OLD-vs-FIX
delta really is `e3ed01f0` and nothing else.

---

## 4. Live measurements (read-only)

### 4.1 The archive's `meta` table

```
$ DB="$HOME/Library/Application Support/com.coding-agent-search.coding-agent-search/agent_search.db"
$ URI="file:${DB// /%20}?mode=ro&immutable=1"
$ perl -e 'alarm shift; exec @ARGV' 60 /usr/bin/time -p /usr/bin/sqlite3 "$URI" "SELECT count(*) FROM meta;"
3
real 0.01

$ perl -e 'alarm shift; exec @ARGV' 60 /usr/bin/sqlite3 "$URI" "SELECT key, substr(value,1,80) FROM meta;"
schema_version|20
last_scan_ts|1784196225836
last_indexed_at|1784200805044

$ perl -e 'alarm shift; exec @ARGV' 60 /usr/bin/sqlite3 "$URI" "SELECT sql FROM sqlite_master WHERE name='meta';"
CREATE TABLE IF NOT EXISTS meta ("key" TEXT PRIMARY KEY, value TEXT NOT NULL)

$ perl -e 'alarm shift; exec @ARGV' 60 /usr/bin/time -p /usr/bin/sqlite3 "$URI" \
    "SELECT value FROM meta WHERE key = 'connector_scan_floors';"
real 0.02
```

MEASURED: `meta` holds **3 rows** and **does not contain `connector_scan_floors`**
(`CONNECTOR_SCAN_FLOORS_META_KEY = "connector_scan_floors"`, storage/sqlite.rs:60). So on this
archive the new query returns **zero rows**, while the two pre-existing meta reads
(`last_scan_ts`, `last_indexed_at`) each return exactly one. Stock sqlite answers the exact
query in 0.02 s wall / 0.00 s cpu against the 7.93 GB file — the bead's claim replicates.

### 4.2 The full frankensqlite lifecycle on the live archive: 40 ms

```
$ perl -e 'alarm shift; exec @ARGV' 60 /usr/bin/time -p \
    /Users/dalecarman/.local/bin/cass triage --json > triage.json
rc=0
real 0.04
user 0.01
sys 0.01
```

`triage.json` → `readiness.database`:

```json
{ "exists": true, "opened": true, "conversations": null, "messages": null,
  "open_error": null, "open_retryable": false,
  "counts_skipped": true, "open_skipped": false }
```

This is the load-bearing measurement. `opened: true` with `open_skipped: false` and
`open_error: null` proves `probe_state_db` (lib.rs:15230-15324) really executed, on the live
7.93 GB archive, in 40 ms total wall clock:

1. `open_franken_cli_read_db(..., STATE_DB_OPEN_TIMEOUT = 5 s)` — a real schema-only readonly open,
2. `validate_fts_messages_integrity_for_connection`,
3. `SELECT value FROM meta WHERE key = 'last_indexed_at'` via `franken_query_row_map_retry`,
4. `SELECT value FROM meta WHERE key = 'last_scan_ts'` via the same helper,
5. `close_franken_cli_read_db` → `close_in_place`.

`counts_skipped: true` confirms no `COUNT(*)` ran (7.93 GB > `STATUS_COUNT_SCAN_MAX_DB_BYTES`
= 256 MB, lib.rs:15065/15832-15836), and `refresh_state_database_counts_if_needed`
(lib.rs:16491-16495) short-circuits because `counts_skipped` is true.

**The open is cheap. The close is cheap. A literal-key meta read through
`franken_query_row_map_retry` is cheap. All three, today, on the specimen archive.**

### 4.3 Health elides the open, as the fix's own comment says

```
$ perl -e 'alarm shift; exec @ARGV' 60 /usr/bin/time -p \
    /Users/dalecarman/.local/bin/cass health --json > health.json
rc=1
real 0.05
```

`health.json` → `state.database`:

```json
{ "exists": true, "opened": true, "conversations": null, "messages": null,
  "open_error": null, "open_retryable": false,
  "counts_skipped": true, "open_skipped": true }
```

`open_skipped: true`. `connector_coverage` is absent from this binary's output, as expected —
the installed binary is pre-fix (`cass 0.6.9`, sha256 `3d044227..`). Health level `unhealthy`,
exit 1, consistent with the bead's OLD row.

### 4.4 Refuted hypothesis: the 10 s retry loop

I expected the zero-row error to be misclassified as retryable, making
`franken_query_row_map_retry` spin for `CLI_DB_QUERY_RETRY_TIMEOUT` = 10 s. It is not.
MEASURED from source: fsqlite renders the error as `"query returned no rows"`
(fsqlite-error-0.1.17/src/lib.rs:74-76, `#[error("query returned no rows")]`), and cass's
classifier `retryable_storage_error_message` (storage/sqlite.rs:752-760) matches only
`busy | locked | locking | contention | temporarily unavailable | would block`. No match, and
`QueryReturnedNoRows` is not in the `retryable_franken_error` variant list
(storage/sqlite.rs:739-750). The helper returns on the first attempt. Hypothesis dead.

### 4.5 Refuted hypothesis: `?1` takes a different execution engine

Both the literal and the parameterised meta reads enter fsqlite through the same
`ConnectionExt::query_row_map` → `Connection::query_row_with_params`
(fsqlite-0.1.5/src/compat/connection.rs:67-73; fsqlite-core-0.1.17/src/connection.rs:14922),
because cass passes `params![]` for the literal ones. The prepared-fast-lane predicate
`ad_hoc_query_supports_prepared_reuse` (fsqlite-core connection.rs:15114-15117) is
`matches!(statement, Statement::Select(_)) && !prepared_select_requires_dispatch(..)` — the
same answer for both texts. So `?1` alone does not change engine path.

---

## 5. Mechanism — what the evidence forces

**MEASURED, by elimination:** `e3ed01f0` adds exactly one new database operation to `triage`,
`status` and `stats` (`git show e3ed01f0 -- src/lib.rs` — every other added hunk is JSON
assembly, verdict booleans, and `println!`):

```diff
@@ -15121,6 +15261,7 @@ fn probe_state_db(
     .and_then(|s| s.parse::<i64>().ok());
+    snapshot.connector_scan_floors = Some(read_connector_scan_floors(&conn));

@@ -23579,6 +23722,10 @@ fn run_stats(
+    let connector_scan_floors = read_connector_scan_floors(&conn);
```

and to `health`, that same call plus one open/close (`read_connector_scan_floors_bounded`).
Since the open, the close, and literal-key meta reads are measured at 40 ms combined, **the
added query is the only candidate left.** The bead's proposed fix — bound the lifecycle —
targets a component measured cheap and would not touch `triage`, `status` or `stats` at all.

**INFERRED, unresolved:** *why* that one query is expensive inside frankensqlite. Three
candidates survive, in descending order of fit:

1. **Zero rows.** It is the only meta read on these paths that matches nothing. `meta` has 3
   rows so a scan is trivial; a pathological no-match path in the prepared-row lane is the
   suspicion, not a finding.
2. **First-use materialisation.** A SQL text not previously executed on that connection may
   force schema/catalog or `cached_read_snapshot` materialisation
   (fsqlite-core connection.rs:14297) that the two already-warm literal reads did not — and
   this archive's schema includes FTS5 shadow tables over 580,374 messages.
3. **Bound-parameter planning.** Weakest: §4.5 shows the engine path is shared, and `cass
   search` (1.90 s in the bead) runs parameterised queries constantly without dying.

I could not separate these without executing frankensqlite, which needs a build (forbidden this
lane). There is no raw-SQL surface on the cass CLI to borrow — MEASURED, `rg -n 'Sql \{|Query
\{|raw_sql'` over `src/lib.rs` returns nothing.

**The falsifier, for whoever owns the build.** One test or scratch binary, one readonly
connection to a copy of the live archive, four timed calls:

| # | sql | params | rows |
|---|---|---|---|
| 1 | `SELECT value FROM meta WHERE key = 'last_scan_ts'` | `params![]` | 1 |
| 2 | `SELECT value FROM meta WHERE key = 'connector_scan_floors'` | `params![]` | 0 |
| 3 | `SELECT value FROM meta WHERE key = ?1` | `['last_scan_ts']` | 1 |
| 4 | `SELECT value FROM meta WHERE key = ?1` | `['connector_scan_floors']` | 0 — production shape |

1 vs 2 isolates zero-rows. 1 vs 3 isolates the bound parameter. Run 4 first and last on the
same connection to catch first-use materialisation. Each call could come out either way, so it
is a real instrument.

---

## 6. In-repo precedent

### 6.1 For bounding a blocking DB operation

Closest precedent, and it already exists: **`open_franken_cli_read_db_with_hard_timeout`**,
src/lib.rs:14127-14148, with its receiver split out at 14193-14222.

```rust
14137      let (tx, rx) = std::sync::mpsc::channel();
14138      let _open_worker = std::thread::spawn({
14141              let result = open_franken_cli_read_db(path, &reason, timeout)
14142                  .map(crate::storage::sqlite::SendFrankenConnection::new);
14143              let _ = tx.send(result);
14147      receive_franken_cli_read_db_open_result_with_hard_timeout(rx, display_path, reason, timeout)
```

`std::thread::spawn` + `std::sync::mpsc` + `rx.recv_timeout(timeout)`, a
`SendFrankenConnection` wrapper to carry the handle back, a `CliError` with
`kind = DbOpen, retryable = true` on timeout, and a unit test
(`cli_read_db_hard_timeout_reports_open_timeout`, lib.rs:66186-66204). Three callers, all in
`cass doctor` (68339 at 30 s, 68484, 69386). It also runs `sqlite_header_preflight_error`
(14150-14191) first, so a corrupt file fails fast without spending the timeout.

Older sibling, same idiom: **`LazyFrankenDb::get_with_timeout`**, storage/sqlite.rs:245-295
("Fix for #128"), also `std::thread::spawn` + `recv_timeout`.

**There is no precedent in this repo for bounding a whole open → query → close lifecycle.**
Both existing bounds cover the open only. Extending the idiom is straightforward and needs
*less* machinery than the existing one: run the whole lifecycle on the worker thread and send
back the `BTreeMap<String, i64>`, so the connection never crosses a thread boundary and
`SendFrankenConnection` is not needed at all.

### 6.2 asupersync — what it constrains

AGENTS.md:76-78: *"This project uses **asupersync** as its async runtime. It provides
`RuntimeBuilder`, `spawn_blocking`, `fs` ops, `net`, `signal`, and structured concurrency via
`Cx`."*

It does not constrain this fix. MEASURED: `asupersync::runtime::spawn_blocking` has exactly two
uses in the tree (src/lib.rs:7290, src/pages/preview.rs:431) and neither is on a CLI read path.
Every CLI DB read here is synchronous, and both existing timeout bounds use plain
`std::thread` + `std::sync::mpsc`. fsqlite's async API additionally requires "an asupersync
runtime with a blocking pool" (fsqlite-0.1.5/src/async_api.rs:112) which the CLI does not
stand up. Reaching for asupersync here would be new machinery against an existing precedent —
use `std::thread` + `recv_timeout`.

### 6.3 For reading a maybe-absent meta key

Two precedents, and `read_connector_scan_floors` uses neither:

1. **`.optional()`** — `FrankenStorage::get_connector_scan_floors`, storage/sqlite.rs:6991-7002.
   This is the *same read*, written correctly on the storage side:

   ```rust
   6992      let result: Result<String, _> = self.conn.query_row_map(
   6993          "SELECT value FROM meta WHERE key = ?1",
   6994          fparams![CONNECTOR_SCAN_FLOORS_META_KEY],
   6995          |row| row.get_typed(0),
   6996      );
   6997      match result.optional() {
   6998          Ok(Some(raw)) => Ok(parse_connector_scan_floors(&raw)),
   6999          Ok(None) => Ok(BTreeMap::new()),
   ```

   (`frankensqlite::compat::OptionalExtension as FrankenOptionalExtension`, imported at
   storage/sqlite.rs:10.)

2. **`COALESCE`** — storage/sqlite.rs:2171:
   `SELECT COALESCE((SELECT value FROM meta WHERE key = 'schema_version'), '');` — an existing
   in-repo way to make a maybe-absent meta read return exactly one row.

The CLI copy at lib.rs:15077 is a **duplicate of the storage reader** that drops the
`.optional()` handling. That is a single-source defect independent of the timing question, and
it is where the fix belongs.

### 6.4 AGENTS.md — does the repo sanction a plain sqlite path?

**No.** Read in full:

- **"Verified Standard SQLite File Reads"** (AGENTS.md:284-289) says the opposite of what the
  bead's sqlite3 measurement might suggest: *"`frankensqlite::Connection::open()` can open and
  read standard SQLite database files … **Do not add `rusqlite` just to read an existing SQLite
  file.** If a specific query shape fails against one of these files, treat it as a targeted
  engine/query bug and file a reproducer instead of assuming the file format is unsupported."*
- **"Known frankensqlite Differences"** (AGENTS.md:306-312) lists only file-format interop
  (readable both ways since rev `9cedb30b`) and `PRAGMA writable_schema` write support. Nothing
  about meta reads or open cost.
- **RULE NUMBER 2** (AGENTS.md:48): no rusqlite in new code, frankensqlite only.

So the sqlite3 timing in the bead is a **diagnostic, not a prescription**. And AGENTS.md:289
prescribes exactly what §5's falsifier does: treat it as a targeted engine/query bug and file a
reproducer.

---

## 7. Fix direction: "bound the whole lifecycle" vs "don't do the expensive thing"

**The evidence supports the second, and locates it precisely.**

*Bound the whole lifecycle* fails three ways. It fixes only `health` (the sole caller of the
bounded helper) and leaves `status`, `triage`, `stats` and robot-meta search hanging, because
their coverage read is inside `probe_state_db` / `run_stats` on a shared connection. It bounds
components measured at 40 ms while leaving the one measured-expensive operation inside the
bound. And it converts a wrong answer into a slow-then-degraded answer — symptom suppression.

*Don't do the expensive thing* is right, and there is exactly one place to do it:
**`read_connector_scan_floors`, src/lib.rs:15077-15090** — the single function all five
surfaces funnel through. One edit fixes all of them.

**Smallest architecture-correct change**, in order:

1. **src/lib.rs:15077-15090 — stop asking for a row that is not there.** Whichever of §5's
   candidates wins, the repair is the same shape and the repo already has it twice: adopt the
   storage-side `.optional()` handling (storage/sqlite.rs:6997), or the `COALESCE` idiom
   (storage/sqlite.rs:2171), so the query returns exactly one row and the absent-key case is a
   value rather than an error. Better still, **delete the duplicate and call
   `FrankenStorage::get_connector_scan_floors`** — one reader, one behaviour. Verify against
   the §5 falsifier before committing to the exact shape; do not guess which candidate it is.
2. **src/lib.rs:15099-15108 — make the name true, or change it.** Independently of timing, a
   function called `_bounded` taking a `timeout` that binds nothing is a trap for the next
   reader. Either route it through the existing
   `open_franken_cli_read_db_with_hard_timeout` idiom extended to carry the whole
   lifecycle on the worker thread (§6.1 — send back the `BTreeMap`, not the connection), or
   rename it and delete `HEALTH_COVERAGE_OPEN_TIMEOUT`. This is second, not first: with (1)
   done the read is a 3-row lookup on an already-cheap open, and a bound is belt-and-braces
   rather than the fix.
3. **Regression proof.** Bead `gxw32` already records that the lib suite cannot see a
   coverage-floor mutant because its fixture registers one connector. A timing assertion on a
   synthetic archive will not reproduce this either — the defect only appears at the live
   archive's scale. The honest verifier is the §5 four-call falsifier plus a live
   before/after on `health`, `triage`, `status` and `stats` with the binary named by its
   explicit preserved path and its sha256 printed alongside the timings, per 1a7mk's own
   comment.

**Do not deploy `e3ed01f0` as-is a second time.** Nothing in the tree at HEAD `37d52925` has
changed since the 2026-08-10 rollback: `read_connector_scan_floors_bounded` is byte-identical,
and the added `probe_state_db` / `run_stats` calls are untouched. Redeploying reproduces the
same 6/6 hang.

---

## 8. Corrections to the established facts handed to this lane

1. **"Root cause per 1a7mk: … passes the 2s timeout to open only; the subsequent read and close
   are unbounded."** True as far as it goes, and **not the root cause**. The open and close are
   measured at 40 ms combined on the live archive (§4.2). The unbounded read is where the cost
   is — and the same read is unbounded on four other surfaces that never touch the bounded
   helper.
2. **"the 2s HEALTH_COVERAGE_OPEN_TIMEOUT to open_franken_cli_read_db ONLY."** Also incomplete:
   the open is not hard-bounded either (§1.2), and `FrankenStorage::open_readonly` discards the
   caller's timeout for a hardcoded 30 s lock wait.
3. **Blast radius is five surfaces, not three** — health, status, triage, stats, and
   `cass search --robot-meta` (lib.rs:20502). `cass doctor` is *not* among them (§2).
4. **Attribution checked and upheld.** I expected the June-1 vs August-10 specimen comparison to
   be confounded and went looking; only two commits touched `src/` in that window and both are
   the coverage-floor change, with no dependency movement (§3).
