# Lane: verify-coverage-floor-regression (adversarial verifier)

Subject: lane `coverage-floor-regression` (bead `coding_agent_session_search-1a7mk`).
Task: try to REFUTE. Open every cited file:line; hunt filtered-probe-stated-unfiltered,
negatives from instruments never shown capable of a positive, drifted citations, and
claims promoted from another document rather than measured.

Repo root: `/Users/dalecarman/dev/coding_agent_session_search/.claude/worktrees/cass-to-green-c6bfb589`
HEAD: `74a72233`. Read-only lane; the only file written is this log.

---

## 0. Environment fact that governs every line number below

`src/lib.rs` is **dirty in this worktree right now**:

```
$ git status --short
 M src/lib.rs
?? thoughts/shared/handoffs/20260815-cass-to-green/

$ git diff --stat HEAD
 src/lib.rs | 95 ++++++++++++++++++++++++++++++++++++++++++++++++++++++--------
 1 file changed, 83 insertions(+), 12 deletions(-)
```

The diff is a sibling session **implementing this lane's recommendation** — it rewrites
`read_connector_scan_floors` to return `Option<BTreeMap<..>>` via `.optional()`, and
rewrites `read_connector_scan_floors_bounded` to run open+read+close on a worker behind
`rx.recv_timeout(timeout)`. So the file moved under me mid-verification: my first
`rg -n` returned `open_franken_cli_read_db_with_hard_timeout` at 68438/68583/69485,
and an `awk` two minutes later showed different content at 68438.

**Everything below is therefore verified against the committed blob** via
`git show 74a72233:src/lib.rs | awk '...'`, which is the state the lane examined.
Against the working tree the numbers are already stale.

---

## 1. Finding 1 (line drift) — CONFIRMED, exact

```
$ rg -n 'HEALTH_COVERAGE_OPEN_TIMEOUT|read_connector_scan_floors' src/
src/lib.rs:15091 / 15109 / 15113 / 15119 / 15297 / 23761 / 65530
src/indexer/mod.rs:10696 / 11568 / 11737 / 33527 / 33567      <-- see §4
```

Against `git show 74a72233:src/lib.rs`, every construct the lane named is where it said:

| construct | line | verified |
|---|---|---|
| `fn read_connector_scan_floors` | 15091 | yes |
| `.unwrap_or_default()` | 15103 | yes |
| `const HEALTH_COVERAGE_OPEN_TIMEOUT` | 15109 | yes |
| `fn read_connector_scan_floors_bounded` | 15113-15122 | yes |
| `fn connector_coverage_json` | 15126 | yes |
| `fn connector_coverage_state_json` | 15149-15159 | yes (lane wrote 15145-15158; that range covers the doc comment it quotes) |
| `probe_state_db` call | 15297 | yes |
| `run_stats` call | 23761 | yes |
| `run_health` call | 65530 (block 65528-65532) | yes |

Bead claim spot-checked: `rg -o '65440|15080|15084' .beads/issues.jsonl` returns one
each — the bead does carry the stale trio the lane says it does.

## 2. Finding 2 (busy/retry budget, not wall clock) — CONFIRMED

- `src/lib.rs:14066-14125` — parameter is literally named `busy_timeout` (14069);
  `PRAGMA busy_timeout = {ms}` is issued at **14120-14121, after the connection
  already exists**.
- `src/storage/sqlite.rs:637-665` — `let deadline = Instant::now() + timeout` (645),
  and the deadline is read **only inside** `Err(err) if retryable_franken_anyhow(&err)`
  (650-661). A single blocking `FrankenStorage::open_readonly(path)` (648) is unbounded.
- `src/storage/sqlite.rs:4172-4174` — `pub fn open_readonly(path)` discards the caller's
  timeout and hands `DOCTOR_MUTATION_DB_OPEN_LOCK_TIMEOUT` to
  `open_readonly_with_doctor_lock_timeout`. Const is 30s at `sqlite.rs:42`.

**Nuance the lane did not state (not a refutation).** The 30s override applies to the
*first* attempt path only. The fallback `open_franken_raw_readonly_connection_with_timeout`
(`sqlite.rs:701-737`) **does** pass the caller's `timeout` to
`acquire_doctor_mutation_db_open_guard` at 713. So "already overridden by a hard-coded
30s one layer down" is true of the primary path, not of both.

## 3. Finding 3 (nothing bounds query/close) — **PARTIALLY REFUTED**

The close half is exactly right. `close_franken_cli_read_db`
(`src/lib.rs:14224-14239`) calls `conn.close_in_place()` with no deadline, no retry,
no timeout of any kind.

The query half is **false as stated**. `read_connector_scan_floors` calls
`franken_query_row_map_retry`, which carries its own deadline:

```
14259  fn franken_query_row_map_retry<T, F>(
14268      let deadline = std::time::Instant::now() + CLI_DB_QUERY_RETRY_TIMEOUT;
14270      loop {
14271          match conn.query_row_map(sql, params, |row| map(row)) {
14273              Err(err) if retryable_franken_error(&err) => {
14275                  if now >= deadline { return Err(err); }
...
15080  const CLI_DB_QUERY_RETRY_TIMEOUT: Duration = Duration::from_secs(10);
```

So the query carries a **10s retry budget**, not "no bound of any kind". Its *shape*
is the same defect the lane correctly diagnosed for the open (deadline read only in
the retryable arm ⇒ a single blocking attempt is unbounded), so the substance —
no wall-clock bound anywhere in `read_connector_scan_floors_bounded` — survives.
But an implementer told "no bound of any kind" will not know a separate 10s budget
is in play. Note the in-flight sibling fix states this correctly in its own doc
comment ("The query carried its own separate `CLI_DB_QUERY_RETRY_TIMEOUT`, and
`close_franken_cli_read_db` … was bounded by nothing at all").

Rest of Finding 3 CONFIRMED: `open_franken_cli_read_db_with_hard_timeout`
(14127-14148) → `receive_franken_cli_read_db_open_result_with_hard_timeout`
(14193-14222) → `rx.recv_timeout(timeout)` at 14199. Its three call sites in the
committed blob are **68412, 68557, 69459**; 65530 is not among them. Exact.

## 4. Finding 4 (`.unwrap_or_default()` false green) — CONFIRMED, but the
   single-source count is **wrong**, and that changes the fix scope

Confirmed as stated: 15103 collapses `Err` into an empty map; `connector_coverage_json`
(15126-15131) renders empty as `"checked": true, "complete": true`.

**The lane says "one read, two implementations". There are three.**

`src/indexer/mod.rs:10696-10721` is a third implementation, and it is **equally lossy**:

```
10696  fn read_connector_scan_floors_fresh(db_path: &Path) -> BTreeMap<String, i64> {
10700      let storage = match FrankenStorage::open_readonly(db_path) {
10702          Err(error) => {
10703              tracing::warn!(... "could not open the archive to read connector scan
10707               coverage floors; previously failed connectors will not be widened this run");
10709              return BTreeMap::new();
10712      let floors = storage.get_connector_scan_floors().unwrap_or_else(|error| {
10717          BTreeMap::new()
10718      });
```

Consumers: `src/indexer/mod.rs:11568` and `:11737` (both feed
`ConnectorScanCoverage::new`). It also opens via the unbounded
`FrankenStorage::open_readonly` from §2. Its failure mode differs from the readiness
surfaces' — an empty map here means a previously-failed connector silently is not
widened this run, rather than a false "coverage complete" report — but it is the same
swallow of the same read, and the lane's recommendation (A+B, both scoped to
`src/lib.rs`) would leave it in place. "Every readiness surface calls the lossy one"
is true; "two implementations" is not.

Storage's correct read confirmed at `src/storage/sqlite.rs:6991-7002` — `.optional()`
at **6997**, the three arms at **6998-7000** (the lane cited 6994-6998, which starts
at the `fparams!` line and stops one arm in).

**Citation defect, repeated in the Recommended action.** The lane places run_stats'
text branch at `23831-23851`. At 74a72233 that range is
`return output_structured_value(payload, fmt);` (23831) through the by_source table
print (23851). The branch that prints the quoted string is **23860-23880**:

```
23860      if connector_scan_floors.is_empty() {
23861          println!("Scan Coverage: complete (no connector scan has aborted)");
23862      } else {
```

The verbatim string is real and exactly as quoted; the range is off by ~29 lines. An
implementer following the recommendation would edit the wrong block.

## 5. Finding 5 (meta key absent) — CONFIRMED on the key; the "not the cost"
   half is a cross-engine inference stated unfiltered

Re-measured, read-only:

```
$ sqlite3 "file:.../agent_search.db?mode=ro" "SELECT key FROM meta ORDER BY key;"
last_indexed_at
last_scan_ts
schema_version
rc=0
$ sqlite3 "file:.../agent_search.db?mode=ro" "SELECT count(*) FROM meta;"
3          <-- positive control: the instrument reads real rows, and 3 == 3 keys
```

No `connector_scan_floors` row. So today's `connector_coverage` block is produced by
`Err(QueryReturnedNoRows) -> unwrap_or_default() -> empty -> complete: true`.
`QueryReturnedNoRows` is not retryable: `src/storage/sqlite.rs:739-750` lists the
retryable variants (Busy, BusyRecovery, BusySnapshot, DatabaseLocked, LockFailed,
WriteConflict, SerializationFailure) and 752-760 the retryable message substrings;
"query returned no rows" matches neither. Confirmed.

Two defects:

- **Wrong crate version.** The lane cites `fsqlite-error-0.1.17/src/lib.rs:74-76`.
  `Cargo.lock:2350-2354` pins **fsqlite-error 0.1.5**. Both versions sit in the
  registry cache, which is how the wrong one got read. The content is byte-identical
  at those lines in 0.1.5 (`74` doc, `75` `#[error("query returned no rows")]`,
  `76` `QueryReturnedNoRows`), so the substance survives on luck, not on method.
- **"the meta query is NOT the cost" is stated about cass but measured on a
  different engine.** The 0.064s `time sqlite3` figure measures **system SQLite**;
  cass runs frankensqlite. Finding 6's implication says the opposite about cass
  ("The blocking operation is executing `SELECT value FROM meta WHERE key = ?1`").
  The two implications are reconcilable only by "the cost is frankensqlite-specific",
  which is exactly what Finding 7 honestly labels UNVERIFIED — so Finding 5's
  implication should carry that qualifier and does not.

## 6. Finding 6 (triage attribution) — CONFIRMED

- `git diff 559b2329 419437e6 -- src/lib.rs` (249 insertions): the only added DB
  operation on triage's path is `snapshot.connector_scan_floors =
  Some(read_connector_scan_floors(&conn));` inside `probe_state_db`, plus helpers.
  Both commits resolve (`559b2329` beads flush; `419437e6` the coverage-floor merge).
- `state_meta_json_for_status` at **15784-15790**, passing `false` in the
  skip_db_open position — exact, and the neighbouring doc comment says
  "status / diag pass false (they explicitly want the open-success signal)".
- `refresh_state_database_counts_if_needed` — the **function** is at 16479, but the
  lane's cited **16505-16509** is precisely the short-circuit it describes:
  `let needs_refresh = !current_counts_skipped && (...)` / `if !needs_refresh ||
  !db_path.exists() { return; }`. Correct citation, not drift.
- `STATUS_COUNT_SCAN_MAX_DB_BYTES = 256 * 1024 * 1024` at **15065**. Exact.
- `15283` / `15291` do use literal keys with `params![]`; `15297` is the only
  parameterized meta read on these paths. Exact.

## 7. Finding 7 (frankensqlite mechanism UNVERIFIED) — honest label, one wrong hop

- `fsqlite-0.1.5/src/compat/connection.rs:67-74` — `query_row_map` calls
  `self.query_row_with_params(sql, &values)` at 72. Exact.
- The lane then writes "→ `Connection::query_with_params`
  (fsqlite-core-0.1.5/src/connection.rs:13160)". **The actual callee is
  `Connection::query_row_with_params` at 13272**, a distinct function; 13160 is
  `query_with_params`, which is not on this path.
- The endpoint claim survives: 13272's body reaches
  `execute_statement_after_background_status(statement.as_ref(), Some(params))` at
  **13299**, with a prepared-statement branch at **13294**
  (`query_prepared_row_after_background_status(&prepared, Some(params))`) the lane
  did not mention. `Connection::query` is at 13133 as cited.
- The finding is already labelled UNVERIFIED and its implication ("do not put a
  frankensqlite mechanism in the fix commit message") is the right call regardless.

## 8. Finding 8 (no coverage test) — headline CONFIRMED, test inventory **REFUTED**

Negative probe, with the positive control the parent asked for:

```
$ rg -c 'connector_coverage|connector_scan_floors|coverage_floor' tests/
rc=1                                   <-- no matches, 229 entries in tests/

$ rg -c 'latency_ms' tests/            <-- POSITIVE CONTROL, same tool, same dir
tests/cli_dispatch_coverage.rs:1
tests/golden_robot_json.rs:2
tests/spec_health_latency_contract.rs:15
tests/lifecycle_matrix.rs:1
tests/e2e_two_tier_search.rs:6
tests/e2e_health.rs:2
tests/golden/robot_docs/schemas.txt.golden:1
tests/golden_readiness.rs:2
tests/e2e_tui_smoke_flows.rs:1
tests/golden/robot/health_shape.json.golden:1
rc=0
```

The instrument can produce a positive on this exact path. **No CLI-level coverage-floor
test exists.** CONFIRMED.

Latency contract honest-but-blind: CONFIRMED. `HEALTH_LATENCY_THRESHOLD_MS = 150`
(`tests/spec_health_latency_contract.rs:91`), fixture via `copy_search_demo_fixture`
(:65-84), and the stopwatch genuinely spans the new read — 65446 `Instant::now()`,
65530 the coverage read, 65661 `start.elapsed()`. Fixture size measured:
`tests/fixtures/search_demo_data` = **408K, 24 files**, against a ~7.98 GB live archive.

**Refuted:** "The fix shipped only three unit tests in src/storage/sqlite.rs, all
against the storage API that handles errors correctly."
`git diff 559b2329 419437e6 | rg -c '^\+\s*#\[test\]'` → **4**. The fourth is in
`src/indexer/mod.rs`:

```
+    #[test]
+    #[serial]
+    fn aborted_connector_scan_does_not_leave_the_index_claiming_complete_coverage() {
```

It is a behavioral coverage test, not a storage-API test — it asserts a durable floor
survives an aborted scan (`src/indexer/mod.rs:33527-33532`) and that a clean scan
clears it (:33566-33569). The lane's headline (no CLI-level test) is unaffected; its
inventory of what already exists is not.

## 9. Finding 9 (live repro UNAVAILABLE) — CONFIRMED, still true

Re-measured. Two cass writers live right now: pid **46340**
`cass index --watch-once … --json` (3:00 elapsed, 2:31 CPU) against the live archive,
and pid **87813** `cass index --full --force-rebuild --data-dir /private/tmp/…`
(21:48 elapsed, 19:27 CPU). Different pids from the lane's (68378/82693) — time has
passed; the conclusion is unchanged. Preserved specimens exist as claimed:

```
-rwxr-xr-x  51900976  Aug 10 20:37  ~/.local/bin/cass.coverage-floor-fix-20260810
-rwxr-xr-x  51834784  Jun  1 06:21  ~/.local/bin/cass.pre-coverage-floor-20260601
```

---

## Verdict

Headline survives. Recommended action (A) survives unchanged. Three stated claims are
false as written and one gap changes the fix scope:

1. Finding 3 — "the query … carries no bound of any kind" is false; it carries
   `CLI_DB_QUERY_RETRY_TIMEOUT` = 10s (src/lib.rs:14268, 15080). True of the close only.
2. Finding 4 — three implementations, not two; `read_connector_scan_floors_fresh`
   (src/indexer/mod.rs:10696-10721) is equally lossy and is outside the recommended
   fix's scope.
3. Finding 8 — four tests shipped, not three; the fourth
   (`aborted_connector_scan_does_not_leave_the_index_claiming_complete_coverage`,
   src/indexer/mod.rs) is a behavioral coverage test.
4. Citation defects: run_stats text branch is 23860-23880 not 23831-23851 (repeated in
   the Recommended action); fsqlite-error is pinned at 0.1.5 not 0.1.17
   (Cargo.lock:2350-2354); the compat hop lands on `query_row_with_params` (13272),
   not `query_with_params` (13160).
5. Finding 5's "the meta query is NOT the cost" was measured with the system `sqlite3`
   binary — a different engine from the one cass runs — and is stated without that
   qualifier while Finding 6 asserts the opposite about cass.
