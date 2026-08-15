# Lane: coverage-floor regression (bead coding_agent_session_search-1a7mk)

Read-only grounding lane. Repo root
`/Users/dalecarman/dev/coding_agent_session_search/.claude/worktrees/cass-to-green-c6bfb589`,
branch `worktree-cass-to-green-c6bfb589`, commit `74a72233`.
Date 2026-08-15. Only write from this lane is this file.

---

## 0. Bead retrieval

`br show coding_agent_session_search-1a7mk` **failed** in this worktree:

```
Error: Sync conflict: Refusing storage open because pending sync-merge state
could not be inspected under database-family authority: ... the authorized
database is missing
```

`.beads/beads.db` does not exist in the worktree (only `issues.jsonl`, 6.0 MB).
Read the bead from the tracked JSONL export instead, via a `json.loads` filter on
`id == "coding_agent_session_search-1a7mk"`. Got the full record including the
one comment (the 6/6 alternating-trial determinism test). Nothing was lost; the
bead body and comment are complete.

---

## 1. Line numbers at 74a72233 — all three drifted

`rg -n 'HEALTH_COVERAGE_OPEN_TIMEOUT|read_connector_scan_floors' src/`

| what | bead / handoff said | actual at 74a72233 |
|---|---|---|
| `run_health` coverage read | 65440 (bead) / 65457 (handoff) | **src/lib.rs:65528-65532** |
| `probe_state_db` coverage read | 15283 (handoff) | **src/lib.rs:15297** |
| `run_stats` coverage read | 23747 (handoff) | **src/lib.rs:23761** |
| `HEALTH_COVERAGE_OPEN_TIMEOUT` const | 15080 | **src/lib.rs:15109** |
| `read_connector_scan_floors_bounded` | 15084 | **src/lib.rs:15113-15122** |
| "prefers reporting `checked: false`" comment | 15078 | **src/lib.rs:15106-15108** |
| "elided the database open entirely" comment | 15082 | **src/lib.rs:15111-15112** |
| `.unwrap_or_default()` defect | (unnumbered) | **src/lib.rs:15103** |

Drift is roughly +14 to +29 lines; every named construct still exists and still
has the shape the bead describes. `src/lib.rs` is 91,646 lines.

---

## 2. Why the 2s bound covers only the DB open — and does not even cover that

The bead's framing is right in substance and understates the defect. There are
**four** distinct gaps, stacked, and the file already contains the correct
primitive for the one that matters.

### 2.1 The bound is passed to the open call only

```rust
// src/lib.rs:15113-15122
fn read_connector_scan_floors_bounded(
    db_path: &Path,
    timeout: Duration,
) -> Option<BTreeMap<String, i64>> {
    let conn =
        open_franken_cli_read_db(db_path.to_path_buf(), "connector-coverage", timeout).ok()?;
    let floors = read_connector_scan_floors(&conn);          // 15119 — no bound
    let _ = close_franken_cli_read_db(conn, db_path, "connector-coverage"); // 15120 — no bound
    Some(floors)
}
```

Lines 15119 and 15120 receive no deadline of any kind.

### 2.2 The parameter is a *busy* timeout, not a wall-clock bound

`open_franken_cli_read_db` (src/lib.rs:14066-14125) names its third parameter
`busy_timeout`. It is used for exactly two things:

- passed on to `open_franken_readonly_storage_with_timeout` / the raw fallback
  (14084-14094);
- after the connection already exists, set as a PRAGMA:
  ```rust
  // src/lib.rs:14120-14122
  let timeout_ms = busy_timeout.as_millis().clamp(1, u128::from(u32::MAX));
  let _ = conn.execute(&format!("PRAGMA busy_timeout = {timeout_ms};"));
  let _ = conn.execute("PRAGMA query_only = 1;");
  ```

Nothing here bounds wall-clock time.

### 2.3 The retry deadline is only consulted *between* attempts

```rust
// src/storage/sqlite.rs:637-665
pub(crate) fn open_franken_readonly_storage_with_timeout(path, timeout) -> Result<FrankenStorage> {
    let deadline = Instant::now() + timeout;
    loop {
        match FrankenStorage::open_readonly(path) {
            Ok(storage) => return Ok(storage),
            Err(err) if retryable_franken_anyhow(&err) => {
                let now = Instant::now();
                if now >= deadline { return Err(err); }   // ← only reached after an attempt RETURNS
                ...
            }
            Err(err) => return Err(err),
        }
    }
}
```

A single `FrankenStorage::open_readonly` call that blocks is unbounded. The
deadline is a *retry budget*, not a timeout.

### 2.4 `open_readonly` discards the caller's timeout entirely

```rust
// src/storage/sqlite.rs:4172-4174
pub fn open_readonly(path: &Path) -> Result<Self> {
    Self::open_readonly_with_doctor_lock_timeout(path, DOCTOR_MUTATION_DB_OPEN_LOCK_TIMEOUT)
}
```

`DOCTOR_MUTATION_DB_OPEN_LOCK_TIMEOUT = Duration::from_secs(30)`
(src/storage/sqlite.rs:42). So a caller asking for 2 s can spend up to 30 s in
`acquire_doctor_mutation_db_open_guard` (sqlite.rs:494-565) inside one attempt,
before the 2 s deadline is even looked at, and then fall through to
`open_franken_raw_readonly_connection_with_timeout` for another round.

### 2.5 The correct primitive exists in the same file and was not used

```rust
// src/lib.rs:14127-14148
fn open_franken_cli_read_db_with_hard_timeout(path, reason, timeout) -> CliResult<Connection> {
    ...
    let _open_worker = std::thread::spawn({ ... open_franken_cli_read_db(path, &reason, timeout) ... });
    receive_franken_cli_read_db_open_result_with_hard_timeout(rx, display_path, reason, timeout)
}
```

`receive_...` (14193-14222) uses `rx.recv_timeout(timeout)` — a genuine
wall-clock bound that returns a `DbOpen` `CliError` on expiry. Three call sites
use it (65530 is not among them):

```
src/lib.rs:68412, src/lib.rs:68557, src/lib.rs:69459
```

Even that primitive bounds only the open, so it is the pattern to reuse, not the
call to substitute.

### 2.6 The close does more than close

`close_franken_cli_read_db` (src/lib.rs:14224-14239) calls `conn.close_in_place()`.
In `fsqlite-core-0.1.17/src/connection.rs:14271-14273` that is
`close_internal(false, /*checkpoint_on_close=*/ true)`, and at 14386-14396 the
checkpoint fires when

```rust
if checkpoint_on_close
    && !self.pager.is_memory()
    && self.pager.journal_mode() == JournalMode::Wal
    && !self.pager.is_readonly()          // ← this guard should exclude our readonly conn
```

**Caveat, stated rather than glossed:** the `!is_readonly()` guard should suppress
the checkpoint on this read-only connection, so I am *not* claiming the checkpoint
is the cost. The structural point stands regardless: the close is an unbounded
teardown (transaction rollback, live-vtab registry restore, region teardown) with
no deadline over it.

---

## 3. The `.unwrap_or_default()` defect

```rust
// src/lib.rs:15091-15104
fn read_connector_scan_floors(conn: &frankensqlite::Connection) -> BTreeMap<String, i64> {
    franken_query_row_map_retry(
        conn,
        "SELECT value FROM meta WHERE key = ?1",
        &[ParamValue::from(crate::storage::sqlite::CONNECTOR_SCAN_FLOORS_META_KEY)],
        |r| r.get_typed::<String>(0),
    )
    .map(|raw| crate::storage::sqlite::parse_connector_scan_floors(&raw))
    .unwrap_or_default()          // ← line 15103
}
```

`.unwrap_or_default()` on a `Result<BTreeMap<String,i64>, FrankenError>` yields an
**empty map** for *every* error. Three distinct states collapse into one value:

1. the key is absent (no scan has ever recorded a floor) — `QueryReturnedNoRows`;
2. the read genuinely failed (corruption, type mismatch, parse error, or the
   10 s `CLI_DB_QUERY_RETRY_TIMEOUT` retry budget at src/lib.rs:14268 exhausted
   against a busy archive);
3. floors were read and there are none.

**What it silently swallows is the whole point of the feature.** Downstream:

```rust
// src/lib.rs:15126-15131
fn connector_coverage_json(floors: &BTreeMap<String, i64>) -> serde_json::Value {
    serde_json::json!({
        "checked": true,
        "complete": floors.is_empty(),
        ...
```

So a *failed* coverage read is rendered as `checked: true, complete: true`. On
`cass stats` (src/lib.rs:23761 → 23777 for JSON, 23831-23851 for text) that prints:

```
Scan Coverage: complete (no connector scan has aborted)
```

That is precisely the false-green the parent bead
`coding_agent_session_search-codex-coverage-gap-2bh4a` was created to eliminate.
The fix reintroduces its own failure mode through the error path.

### 3.1 It is a single-source defect: one read, two implementations, one correct

The storage layer got this right in the same commit:

```rust
// src/storage/sqlite.rs:6989-6999 (FrankenStorage::get_connector_scan_floors)
match result.optional() {
    Ok(Some(raw)) => Ok(parse_connector_scan_floors(&raw)),
    Ok(None) => Ok(BTreeMap::new()),
    Err(e) => Err(e.into()),
}
```

`.optional()` separates "no row" from "error" and propagates the error. The CLI
copy at 15091 does the same read against the same key with the same SQL and
throws that distinction away. Two implementations of one operation; the lossy one
is the one every readiness surface calls.

### 3.2 Today, every surface takes the error path

Read-only probe of the live archive:

```
$ sqlite3 "file:/Users/dalecarman/Library/Application Support/com.coding-agent-search.coding-agent-search/agent_search.db?mode=ro" "SELECT key FROM meta ORDER BY key;"
last_indexed_at
last_scan_ts
schema_version
```

`connector_scan_floors` is **absent**. So `franken_query_row_map_retry` returns
`FrankenError::QueryReturnedNoRows` ("query returned no rows",
`fsqlite-error-0.1.17/src/lib.rs:74-76`), which is *not* in the retryable set
(`retryable_franken_error`, src/storage/sqlite.rs:739-750, and its message
contains none of busy/locked/locking/contention/temporarily unavailable/would
block). It returns immediately and `unwrap_or_default()` fires. Every
`connector_coverage` block cass emits today is produced by the error path, not by
a successful read.

Timing of that same query through stock sqlite3, read-only, live archive:

```
$ time sqlite3 "file:...agent_search.db?mode=ro" "SELECT value FROM meta WHERE key = 'connector_scan_floors';"
sqlite3 ... 0.02s user 0.04s system 91% cpu 0.064 total
```

0.064 s wall, most of it process start. Confirms the bead's "the meta query is
NOT the cost."

---

## 4. Which surfaces regressed, and the clean differential

`cass health` (src/lib.rs:65528) is the surface the bead explains best: its state
probe deliberately elides the DB open —

```rust
// src/lib.rs:15871-15877
let db_snapshot = if skip_db_open && db_exists && db_is_regular_file {
    StateDbSnapshot { opened: true, counts_skipped: true, open_skipped: true, ..Default::default() }
```

and `state_meta_json_for_health` (15768-15782) passes `skip_db_open = true`. That
elision (bead `gi4oy`) is why health was 6 ms. The fix reintroduces an open at
65528-65532 to answer the coverage question.

**`cass triage` is the decisive row, and it does not fit "the new open is slow."**
Triage goes through `state_meta_json_for_status` (15784-15790, `skip_db_open =
false`) → `probe_state_db` → `open_franken_cli_read_db` at 15259 — the *same* open
function health now calls. Pre-fix triage was 0.09 s end to end. So a
frankensqlite read-only open of this 7.98 GB archive is ~0.09 s, not 90 s.

I eliminated the other candidate on triage's path with evidence:
`refresh_state_database_counts_if_needed` (src/lib.rs:16479-16511) would re-probe
with `include_counts = true` and run `COUNT(*)` over 580 k messages — the known
20-minute pathology from commit c8556771. It does not fire here:

```rust
// src/lib.rs:16505-16509
let needs_refresh = !current_counts_skipped
    && (!current_opened || current_conversations <= 0 || current_messages <= 0);
if !needs_refresh || !db_path.exists() { return; }
```

`include_counts` for a 7.98 GB DB is false (`STATUS_COUNT_SCAN_MAX_DB_BYTES =
256 MB`, src/lib.rs:15065; test at 15846-15850), so `counts_skipped` is true and
`needs_refresh` is false. Not the cause.

That leaves exactly **one** operation added to triage's path by 419437e6:
`read_connector_scan_floors(&conn)` at 15297. The same single operation is what
`run_stats` added at 23761, and what health's new bounded read performs at 15119.
All three regressed surfaces share it; `cass search` and `cass api-version` do not
call it and did not regress.

**Mechanism inside frankensqlite: UNVERIFIED by me.** The differential says the
cost is in executing `SELECT value FROM meta WHERE key = ?1` — note it is the only
*parameterized* meta read on these paths; the pre-existing ones at 15283 and
15291 use literal keys and `params![]`. `ConnectionExt::query_row_map` routes
params through `query_row_with_params`
(`fsqlite-0.1.5/src/compat/connection.rs:67-74`), which reaches
`Connection::query_with_params` (`fsqlite-core-0.1.5/src/connection.rs:13160`)
rather than `Connection::query` (:13133); both end at
`execute_statement_after_background_status`, differing only in `Some(params)` vs
`None`. Whether that difference selects a memdb/in-memory fallback is exactly the
shape commit c8556771 documented for a different statement ("stock sqlite answers
in 0.03s; frankensqlite takes it onto an in-memory fallback path and walks ~936k
b-tree pages"), but I did not trace it to a decision site and did not execute it.
The sibling lane `lanes/fsqlite-claims.md` owns that surface; its confirmed
`reload_memdb_from_pager_with_mode` full-hydration path is gated on
`!reject_mem_fallback`, and cass leaves parity_cert ON, so that particular path is
*not* the explanation.

**Live reproduction: UNAVAILABLE, by lane constraint.** The preserved specimens
exist (`~/.local/bin/cass.coverage-floor-fix-20260810`,
`~/.local/bin/cass.pre-coverage-floor-20260601`), but `ps` shows
`cass index --watch-once ... --json` running against the live archive (pid 68378)
plus another session's `cass stats` (pid 82693, 36 s elapsed / 18 s CPU). I did
not add a reader that the bead says blocks for 90 s while a backfill writes. The
bead's own 6/6 alternating-trial evidence already settles determinism.

---

## 5. Root-cause gate — asking why three times

**Symptom.** `cass health`, `cass health --json`, `cass triage --json` and
`cass stats` do not return on the live archive with a HEAD build; pre-fix they
returned in 15 ms / 20 ms / 90 ms / 26.9 s.

**Why 1 — why do they hang?** Each now performs a coverage read
(`read_connector_scan_floors`, directly or via `read_connector_scan_floors_bounded`)
that has no upper bound on how long it may take.

**Why 2 — why is there no upper bound, when the code declares one?** Because the
only value the author had to work with is a `Duration` that this codebase spends
as a *busy/retry budget*, not as a wall-clock deadline. It is checked between
retry attempts (sqlite.rs:651-653), set as a PRAGMA after the connection already
exists (lib.rs:14120-14121), and silently replaced with a hard-coded 30 s inside
`open_readonly` (sqlite.rs:4173). The query and the close are outside it
altogether (lib.rs:15119-15120).

**Why 3 — why did a careful author reach for that?** Because health's fast path
was designed *not to touch the database at all* (bead `gi4oy`, comment at
lib.rs:15851-15870), so answering the coverage question there required
reintroducing the very open that had been removed, and the only bounding tool in
easy reach was the `timeout` argument the neighbouring functions already took.
The hard-timeout primitive that actually holds
(`open_franken_cli_read_db_with_hard_timeout`, lib.rs:14127) sits ~1,000 lines
above in the same file and was not reached for.

**Why 4 — the one that makes the fix small.** `read_connector_scan_floors`
collapses failure into "no floors" (lib.rs:15103). Because the read has no way to
say *"I could not answer"*, the only design available was to make it always
answer — which forces a blocking read onto a surface whose whole contract is not
to block. Give the read a third state and the fast path is free to give up. The
machinery for that third state **already exists and is already wired**:
`connector_coverage_state_json` (lib.rs:15145-15158) renders `checked: false,
complete: null` for `None`, and its doc comment says the thing out loud —

> `checked: false` is not the same claim as `complete: true`, and collapsing the
> two is the whole shape of this bug.

The bug is that the author wrote that sentence and then, one screen earlier,
collapsed exactly those two states in the reader.

**Bandaid check.** Widening `HEALTH_COVERAGE_OPEN_TIMEOUT` from 2 s to any larger
number is a bandaid under §2.6: it adds no new information, leaves the query and
close unbounded, leaves `open_readonly`'s hard-coded 30 s overriding it, and would
break again the moment the archive is contended. Wrapping only the open in the
existing hard-timeout helper is *also* short of a fix — it fixes 2.3/2.4 and leaves
2.1 (query + close) open. Neither should be presented as a fix. A real fix is
available and is small, so no bandaid is needed here.

---

## 6. Smallest fix that eliminates the cause (not implemented)

Two changes, both inside `src/lib.rs`, no new files, no new abstraction, no
widened timeout.

**Fix A — bound the whole read, by hoisting the pattern already in the file.**
Rewrite `read_connector_scan_floors_bounded` (15113-15122) so the *entire*
open + query + close runs on a worker thread and the caller waits with a single
`recv_timeout(timeout)`, returning `None` on expiry. That is the same shape as
`open_franken_cli_read_db_with_hard_timeout` (14127-14148) plus
`receive_franken_cli_read_db_open_result_with_hard_timeout` (14193-14222), applied
one level up: the unit being bounded becomes "answer the coverage question"
instead of "open the database". Roughly ten lines. `None` already flows to
`checked: false` through `connector_coverage_state_json`, so the surfaces need no
change and the code finally does what its own doc comment at 15106-15108 promises.
(Known ceiling, worth a comment: on expiry the worker thread is orphaned holding
the connection. Acceptable for a short-lived CLI process; it is the same ceiling
the existing helper already accepts at 14138.)

**Fix B — stop collapsing failure into "complete".** Change
`read_connector_scan_floors` (15091) to return `Option<BTreeMap<String, i64>>`
(or `Result`), mapping `QueryReturnedNoRows` to `Some(empty)` and every other
error to `None` — i.e. adopt the discrimination `FrankenStorage::get_connector_scan_floors`
already performs with `.optional()` at sqlite.rs:6994-6998. Then:

- 15297 (`probe_state_db`) assigns the `Option` straight through — the field is
  *already* `Option<BTreeMap<String, i64>>` (15241) with a doc comment saying
  `None` means "did not check", so this is a one-token change;
- 23761 (`run_stats`) picks `connector_coverage_state_json` over
  `connector_coverage_json`, and its text branch (23831-23851) gains an
  "unknown" arm instead of printing "complete";
- 65528 (`run_health`) needs no change.

Fix B is the one that eliminates the *cause* named in Why 4; Fix A without Fix B
would bound the hang while leaving the false-green. Fix B without Fix A would keep
the hang. Both are needed, and together they are smaller than the code they
replace plus one branch.

---

## 7. What test would catch a regression, and what exists today

### 7.1 A latency contract exists — and could not catch this

`tests/spec_health_latency_contract.rs` asserts `latency_ms <= 150` on
`cass health --json` (three cases: empty data dir, initialized fixture, type
check). I verified the stopwatch genuinely spans the new read:

```
src/lib.rs:65446   let start = Instant::now();
src/lib.rs:65528   ... read_connector_scan_floors_bounded(&db_path, HEALTH_COVERAGE_OPEN_TIMEOUT)
src/lib.rs:65661   let latency_ms = start.elapsed().as_millis() as u64;
```

So the metric is honest — the test is not measuring the wrong thing. It fails to
catch the regression for a different reason: it runs against
`copy_search_demo_fixture` (a tiny demo archive), where the open and the meta read
are genuinely fast. This is the same false-green shape this repo already recorded
against `ibuuh.29.1` in commit c8556771 — *"the fixture was too small."*

### 7.2 No coverage-floor test exists at the CLI level at all

```
$ rg -n 'connector_coverage|connector_scan_floors|coverage_floor' tests/
(no output)
```

Zero hits across the whole `tests/` tree. The only tests the fix shipped are three
unit tests in `src/storage/sqlite.rs` (`connector_scan_floors_round_trip_and_clear`,
`connector_scan_since_ts_lowers_to_the_floor`,
`parse_connector_scan_floors_tolerates_junk`) — all against the *storage* API that
handles errors correctly, none against the CLI reader that does not, and none
touching health/triage/stats. This corroborates bead `-gxw32`.

### 7.3 What would actually catch it

Two tests, neither expensive, and the first is the one that matters:

1. **A wall-clock bound test with a blocked archive.** Hold the DB in a state the
   coverage read cannot complete against — the cheapest honest lever is the
   doctor mutation lock, since `acquire_doctor_mutation_db_open_guard`
   (sqlite.rs:494) is a real file lock any test can take with `fs2` — then assert
   `cass health --json` still returns inside a few seconds *and* reports
   `connector_coverage.checked == false`. This asserts the contract the doc
   comment states, fails today, and does not depend on archive size. Note the
   mutant discipline from `.claude/rules/no-vacuous-test-guards.md`: the assertion
   must be `checked == false` (an exact value), not `if let Some(cov) = ...`,
   or it passes vacuously when the field is missing.
2. **A false-green test for the reader.** With the `meta` row present but
   unreadable-as-text (e.g. an integer/blob value where a `String` get_typed
   fails), assert the surface reports `checked: false` rather than
   `complete: true`. That pins Fix B directly and is a pure unit test.

Neither exists. Both would have failed on 419437e6.

---

## 8. Summary of evidence status

| claim | status |
|---|---|
| The three call sites drifted; real locations are 65528 / 15297 / 23761 | CONFIRMED (rg + Read at 74a72233) |
| The 2s value is spent as a busy/retry budget, not a wall-clock bound | CONFIRMED (lib.rs:14066-14125, sqlite.rs:637-665) |
| `open_readonly` discards the caller's timeout for a hard-coded 30 s | CONFIRMED (sqlite.rs:4172-4174, :42) |
| Query (15119) and close (15120) carry no bound | CONFIRMED (lib.rs:15113-15122) |
| A hard-timeout primitive exists in-file and was not used at 65530 | CONFIRMED (lib.rs:14127-14148; users at 68412/68557/69459) |
| `.unwrap_or_default()` at 15103 turns a failed read into `complete: true` | CONFIRMED (lib.rs:15103 → 15126-15131 → 23761/23831) |
| The storage layer discriminates correctly with `.optional()` | CONFIRMED (sqlite.rs:6989-6999) |
| `connector_scan_floors` key is absent from the live archive | CONFIRMED (read-only sqlite3, 3 keys only) |
| The meta query itself is not the cost | CONFIRMED (stock sqlite3, 0.064 s wall) |
| `refresh_state_database_counts_if_needed` is not the triage cause | CONFIRMED (lib.rs:16505-16509 short-circuits) |
| `read_connector_scan_floors` is the only op added to all three paths | CONFIRMED (git diff 559b2329..419437e6) |
| The frankensqlite-internal reason that op blocks | UNVERIFIED — not traced to a decision site, not executed |
| Live reproduction of the hang at HEAD | UNAVAILABLE — backfill (pid 68378) writing the live archive |
| No CLI-level coverage test exists | CONFIRMED (`rg` over tests/, zero hits) |
| The health latency contract exists but uses too small a fixture | CONFIRMED (tests/spec_health_latency_contract.rs) |
