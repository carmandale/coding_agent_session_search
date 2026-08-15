# Lane: gen3-coverage-sweep

Read-only lane. Owner writes only this file. Worktree
`/Users/dalecarman/dev/coding_agent_session_search/.claude/worktrees/cass-to-green-c6bfb589`,
branch `worktree-cass-to-green-c6bfb589`, HEAD at start `9d4814d2` (`git log --oneline -5`).

Subject: establish the COMPLETE set of connector-coverage-floor readers, and sweep for
sibling swallows of a coverage/floor/watermark failure.

Nothing was built, no tests were run, no cargo invocation of any kind was run. Every claim
below is a source read or a verbatim `rg` result. Where I did not verify something I say so
in the sentence that makes the claim.

---

## 0. Instrument notes (so the negatives below can be trusted)

- All sweeps were scoped to `src/`, `tests/`, and (once) `benches/`. `thoughts/` matches were
  excluded after the first pass — they are prior lanes' prose, not code.
- Every glob was written literally (zsh does not expand a glob held in a variable).
- The `rg --stats` on `connector_scan_floor` reported `3508 files searched`, so the walker
  did traverse the tree rather than returning early.
- Positive control for the "no hits in tests/" claim below: `rg --files tests | wc -l` = **576**,
  so the tests directory is non-empty and readable; `rg -c 'connector_coverage' tests/` exits
  **1** (no match) rather than erroring.

---

## 1. The complete set of readers of the connector scan floors

### 1.1 The key and the parser (one definition each)

```
src/storage/sqlite.rs:60:pub const CONNECTOR_SCAN_FLOORS_META_KEY: &str = "connector_scan_floors";
src/storage/sqlite.rs:65:pub fn parse_connector_scan_floors(raw: &str) -> BTreeMap<String, i64> {
```

**Every** use of the constant, in `src/` + `tests/` + `benches/`
(`rg -n "CONNECTOR_SCAN_FLOORS_META_KEY|\"connector_scan_floors\"|'connector_scan_floors'" src/ tests/ benches/`):

| file:line | role |
|---|---|
| `src/storage/sqlite.rs:60` | the definition |
| `src/storage/sqlite.rs:6989` | doc-comment reference |
| `src/storage/sqlite.rs:6994` | **read** — `get_connector_scan_floors` |
| `src/storage/sqlite.rs:7008` | write (DELETE, empty map) |
| `src/storage/sqlite.rs:7016` | write (INSERT OR REPLACE) |
| `src/lib.rs:15114` | **read** — `read_connector_scan_floors` |
| `src/lib.rs:15774` | test fixture INSERT (`recorded_floor_reads_as_checked_and_incomplete`) |

There is **no other use of the constant and no use of the literal string anywhere in code.**
The literal appears nowhere outside `src/storage/sqlite.rs:60`.

I also checked for a dynamically constructed key — `rg -n 'format!\("connector|concat.*connector_scan|"connector_scan' src/`
returns only four `tracing` event-name strings (`src/indexer/mod.rs:10740,10748,10772,10778`)
and the constant itself. **No dynamically built floor key exists.**

And I checked whether a *generic* meta helper could be called with the key: there are
parameterized `SELECT value FROM meta WHERE key = ?1` sites at
`src/storage/sqlite.rs:8302, 8334, 9989, 10001, 10012, 10024, 20186, 20294, 25156` and
`src/search/query.rs:11411`, but since the constant has exactly the seven uses tabulated
above, **none of those generic helpers is ever invoked with the floors key.**

### 1.2 The four functions that read the floors

`rg -ni "connector_scan_floor" src/` — full result set, definitions only:

| # | reader | definition | what it returns on failure |
|---|---|---|---|
| R1 | `FrankenStorage::get_connector_scan_floors` | `src/storage/sqlite.rs:6991-7002` | `Err(e)` |
| R2 | `read_connector_scan_floors` | `src/lib.rs:15107-15130` | `None` |
| R3 | `read_connector_scan_floors_bounded` | `src/lib.rs:15161-15190` | `None` |
| R4 | `read_connector_scan_floors_fresh` | `src/indexer/mod.rs:10696-10721` | **`BTreeMap::new()`** |

R3 is a wrapper over R2. R4 is a wrapper over R1. So there are **two distinct SQL reads**
(`src/storage/sqlite.rs:6993` and `src/lib.rs:15112`) reached through **four** entry points.
The previous generation's "two implementations" counted the SQL reads; the adversarial
verifier's "third" is R4, which is a distinct *entry point with its own error policy*.
**There is no fifth.**

### 1.3 Every call site

R1 (`get_connector_scan_floors`) — 4 production + 8 test:

| file:line | caller | note |
|---|---|---|
| `src/storage/sqlite.rs:7027` | `record_connector_scan_floor` | read-modify-write, `?` propagates |
| `src/storage/sqlite.rs:7042` | `clear_connector_scan_floor` | read-modify-write, `?` propagates |
| `src/indexer/mod.rs:10712` | **R4** | error swallowed — see §2 |
| `src/storage/sqlite.rs:23553,23556,23564,23572,23576,23581` | unit test `connector_scan_floors_round_trip_and_clear` | |

R2 (`read_connector_scan_floors`) — 3 production + 4 test:

| file:line | surface reached | note |
|---|---|---|
| `src/lib.rs:15368` | `probe_state_db` → `state_meta_json` → `cass status`, `cass triage`, `cass health` state envelope | assigned straight through as `Option`, comment at `15365-15367` |
| `src/lib.rs:23970` | `run_stats` → `cass stats` / `cass stats --json` | |
| `src/lib.rs:15171` | inside **R3** | |
| `src/lib.rs:15712,15743,15747,15780` | unit tests in `mod connector_coverage_honesty_tests` | |

R3 (`read_connector_scan_floors_bounded`) — **1** production call site + 1 test:

```
src/lib.rs:65743            .then(|| read_connector_scan_floors_bounded(&db_path, HEALTH_COVERAGE_OPEN_TIMEOUT))
src/lib.rs:15808        let floors = read_connector_scan_floors_bounded(&db_path, Duration::from_nanos(1));   // test
```

R4 (`read_connector_scan_floors_fresh`) — **2** production call sites + 2 test:

```
src/indexer/mod.rs:11568        read_connector_scan_floors_fresh(&opts.db_path),   // streaming scan path, ConnectorScanCoverage::new
src/indexer/mod.rs:11737        read_connector_scan_floors_fresh(&opts.db_path),   // parallel/batch scan path, ConnectorScanCoverage::new
src/indexer/mod.rs:33527, 33567                                                     // tests
```

`ConnectorScanCoverage` (`src/indexer/mod.rs:10641`) is the only consumer of R4's map. Its
own call sites: `11087` (param), `11566`, `11735` (construction), and ten
`ConnectorScanCoverage::default()` uses in tests (`34508, 34557, 34625, 34682, 35108, 35347,
35452, 35531, 35578, 35716`).

---

## 2. Classification of each reader

### Axis (a) — is a failed read discriminated, or swallowed into an empty/default map?
### Axis (b) — is the whole operation (open + read + close) under a wall-clock bound?

| reader | (a) discriminated? | deciding line | (b) whole-op wall-clock bound? | deciding line |
|---|---|---|---|---|
| **R1** `get_connector_scan_floors` | **YES** | `src/storage/sqlite.rs:7000` — `Err(e) => Err(e.into())`, after `.optional()` at `6997` separates `Ok(None)` (absent row) from a real error | **N/A inside the fn / NO overall** — the fn owns no open or close; its single statement is `self.conn.query_row_map` (`6992`), with no retry deadline and no timeout parameter. The only lock bound is the caller's `PRAGMA busy_timeout = 5000` from `src/storage/sqlite.rs:4304`, which bounds *lock contention*, not work. |
| **R2** `read_connector_scan_floors` | **YES** | `src/lib.rs:15122-15128` — `Err(err) => { warn!(...); None }`; `Ok(None) => Some(BTreeMap::new())` at `15121` keeps "absent row" separate from "read failed" | **NO** | takes `conn`, does not open or close. Its query goes through `franken_query_row_map_retry` (`src/lib.rs:14259`), whose `deadline` at `14268` (`CLI_DB_QUERY_RETRY_TIMEOUT` = 10s, `src/lib.rs:15080`) bounds only the **retry loop**: a single `conn.query_row_map` call at `14271` that never returns is never interrupted. |
| **R3** `read_connector_scan_floors_bounded` | **YES** | `src/lib.rs:15179-15188` — `Timeout => { warn!(...); None }`, `Disconnected => None`; and the inner `.ok().and_then(...)` at `15169-15174` propagates R2's `None` | **YES** | `src/lib.rs:15167` spawns the worker; `src/lib.rs:15178` `match rx.recv_timeout(timeout)` is a single wait covering open (`15168`) + read (`15171`) + close (`15172`). Documented ceiling at `15157-15160`: on expiry the worker is orphaned holding the connection. Note the same `timeout` value is passed *both* as the open's busy timeout (`15168`) and as the total budget (`15178`), so the whole unit gets one 2s budget, not 2s per step. |
| **R4** `read_connector_scan_floors_fresh` | **NO — both failure modes collapse to an empty map** | `src/indexer/mod.rs:10709` — `return BTreeMap::new();` on open failure, and `src/indexer/mod.rs:10712` — `.unwrap_or_else(\|error\| { warn; BTreeMap::new() })` on read failure. Both warn, neither is distinguishable by the caller. | **NO** | `FrankenStorage::open_readonly` (`src/storage/sqlite.rs:4172`) has no timeout parameter; its only bound is `acquire_doctor_mutation_db_open_guard(path, DOCTOR_MUTATION_DB_OPEN_LOCK_TIMEOUT)` at `4182`, i.e. 30s waiting for the *doctor mutation lock* (`src/storage/sqlite.rs:42`), not for the open. The close at `src/indexer/mod.rs:10719` is `let _ = storage.close();` → `FrankenStorage::close` (`src/storage/sqlite.rs:4190-4196`) → `conn.close()`, unbounded — the same `close_in_place`-class step that `src/lib.rs:15145-15148` records as the >150s step on the 7.7 GB live archive. |

### Call-site classification (the surfaces)

| surface | reader | (a) at the surface | (b) at the surface |
|---|---|---|---|
| `cass status` / `cass triage` state envelope (`state_meta_json`) | R2 via `probe_state_db` | discriminated — `snapshot.connector_scan_floors` is `Option`, `src/lib.rs:15309` and `15368`; rendered by `connector_coverage_state_json` at `src/lib.rs:16438` | **NO** — `probe_state_db` passes `timeout` to `open_franken_cli_read_db` alone (`src/lib.rs:15327`); the floors read at `15368` and the close have no wall-clock bound. `STATE_DB_OPEN_TIMEOUT` = 5s (`src/lib.rs:15064`); the other caller at `src/lib.rs:16720` passes 30s. |
| `cass stats` / `cass stats --json` | R2 directly | discriminated — `src/lib.rs:24069/24073/24088` is a real three-way branch, and the `else` prints `Scan Coverage: UNKNOWN — the coverage read did not complete` (`24091`) | **NO** — `open_franken_cli_read_db(db_path, "stats", Duration::from_secs(30))` at `src/lib.rs:23834` is a busy timeout on the open only |
| `cass health` | R3 | discriminated — `src/lib.rs:65741-65745`, `.flatten()` keeps `None` as `None` | **YES** — the only bounded surface |
| `cass index` scan-window widening | R4 | **NOT discriminated** | **NO** |

**The one operational consequence of R4's classification, stated plainly:** R4's map is not
rendered to any operator surface. It is consumed only by `ConnectorScanCoverage::new`
(`src/indexer/mod.rs:11566`, `11735`). An empty map there means
`connector_scan_since_ts(run_since_ts, None)` returns the unlowered run-wide watermark
(`src/storage/sqlite.rs:86`), i.e. **the widening silently does not happen** and the recorded
coverage hole is never re-read. It cannot make a surface *print* "complete"; it makes the
condition that would print "incomplete" persist forever while the index quietly never
repairs it. Note also `has_floor` (`10681`) returns false for every connector, so
`clear_connector_scan_floor` is never called (`11890`) — a floor recorded before the failing
read is neither widened nor cleared.

---

## 3. Wider sweep for the same defect shape

Method: a windowed scan (python, ±3 lines) over every `.rs` file under `src/` for
`unwrap_or_default()`, `unwrap_or_else(|_| ...)`, `unwrap_or_else(|x| { ... })`,
`unwrap_or(BTreeMap/HashMap...)`, `.ok()?`, `if let Ok(`, `let _ =`, `.ok().` where the
window mentions coverage / floor / watermark / last_scan_ts / last_indexed_at / complete /
scanned / backfill / pending_session / missing / total. **109 windows matched.** Most are
noise: `let _ = app.update(CassMsg::…Completed)` in `src/ui/app.rs` test code (about 40),
mutex-lock `if let Ok(...)` guards, and SQL-fragment `.unwrap_or_default()`.

Reported below are only those where **a failure is converted into a definite, successful, or
complete answer.**

### 3.1 CONFIRMED — failure becomes good news

| file:line | swallowed expression | what a caller wrongly concludes |
|---|---|---|
| `src/indexer/mod.rs:10709` + `:10712` | `return BTreeMap::new();` / `.unwrap_or_else(\|error\| { …; BTreeMap::new() })` | "no connector has a recorded coverage floor", so the next scan does not widen its window back over the hole a previous failure opened, and never clears the floor. Already known (R4); listed here for completeness of the shape. |
| `src/lib.rs:15363` | `snapshot.last_scan_ts = franken_query_row_map_retry(…, "SELECT value FROM meta WHERE key = 'last_scan_ts'", …).ok().and_then(…)` | A failed **watermark** read becomes `None`, and the staleness rule at `src/lib.rs:16207` is `last_scan_ts.is_some_and(…)`. `None` therefore skips the branch that sets `assets.lexical.status = "stale"` / `fresh = false` (`16214-16221`). **An unreadable watermark is reported as "not stale."** This is the same shape as the floors bug, on the sibling meta key, on the same code path (`probe_state_db`), and it survived `8dcd245b`. |
| `src/lib.rs:15376` and `src/lib.rs:15381` | `.unwrap_or(0)` on `SELECT COUNT(*) FROM conversations` / `FROM messages` | A failed count is reported as a definite `0` while `counts_skipped` stays **false** (set only from `!include_counts` at `15323`). The code's own comment at `src/lib.rs:16076-16079` says "Reporting counts_skipped=false alongside message_count=0 would be a lie" — but it guards only the *skip-open* branch, not the query-failure branch three screens above. |
| `src/lib.rs:23873` and `src/lib.rs:23877` | `.unwrap_or(0)` on the same two counts in `run_stats` | `cass stats --json` emits `"conversations": 0, "messages": 0` (`src/lib.rs:24006-24007`) as definite figures when the query failed. The fallback at `23879+` (`fresh_franken_count_retry`) also ends in `.unwrap_or(0)`. Direction note, honestly: `0` reads as *bad* news for the archive, so this is "a definite answer where the truth is unknown" rather than "good news". |
| `src/lib.rs:32315-32317` | `Err(err) => build_doctor_source_inventory_report(data_dir, true, Some(err.message), **Vec::new()**, detected_roots)` | An unreadable archive yields `total_indexed_conversations = 0` (accumulated at `src/lib.rs:32154`), which becomes `coverage_summary.archive_conversation_count` (`src/lib.rs:36328`) and then the **baseline** of the doctor promotion coverage gate (`src/lib.rs:36372-36378`). With baseline 0, `conversation_delta = candidate − 0 ≥ 0`, so neither blocking branch at `src/lib.rs:36424-36440` fires, `promote_allowed = blocking_reasons.is_empty()` is **true** (`36471`), and `lifecycle_status` is written as `"completed"` (`src/lib.rs:38752-38763`). The gate function receives no signal at all that the baseline was unreadable — `db_query_error` is not among its inputs. **Boundary: I did not trace whether the separate `archive_db_unreadable` critical finding raised at `src/lib.rs:34630-34641` blocks promotion further downstream.** The shape is established as far as the manifest's `lifecycle_status`; whether an operator-facing promote refuses later is unverified by me. |
| `src/indexer/mod.rs:20289` | `let Ok(contents) = fs::read_to_string(&path) else { continue; };` in `conversation_ingest_quarantine_summary` | An existing-but-unreadable quarantine poison file is skipped, so `quarantined_conversations` and `recent_quarantined_conversations` undercount. Those drive the health/status warnings at `src/lib.rs:65089` / `65755` and the circuit breaker; an unreadable quarantine file reads as **no quarantined conversations**. (The path is still listed in `quarantine_files` at `20288`, so the evidence exists in the JSON — but the counts that drive the verdict do not reflect it.) |

### 3.2 CONFIRMED-CLEAN — same shape, correct polarity (checked, not defects)

Recording these because a later reader will otherwise re-derive them:

- `src/indexer/mod.rs:13028` — `storage.get_last_scan_ts().unwrap_or(None)`. A failed watermark
  read becomes `None`, and `non_watch_scan_since_ts` (`src/indexer/mod.rs:11995-12006`) maps
  `None` to a **full scan**. Fails toward more work, not less coverage.
- `src/lib.rs:36424-36440` — `None => blocking_reasons.push("candidate conversation coverage is
  unknown and cannot be promoted")`. The doctor gate **fails closed on unknown** on the
  *candidate* side. This is the positive control for §3.1's last-but-one row: the same
  function gets it right for the candidate and wrong for the baseline, because the baseline
  arrives as a `usize` that cannot express "unknown".
- `src/lib.rs:46192-46200` — `.unwrap_or("unknown")` / `.unwrap_or(false)` reading
  `coverage_gate/status` and `coverage_gate/promote_allowed` from a manifest. Fails closed.
- `src/lib.rs:38456` — `live_conversations.unwrap_or_default()`. Feeds a `>` comparison whose
  failing branch (`38457-38461`) *refuses* the live-archive copy; a `0` there makes the
  refusal more likely, not less.
- The meta-wiping statements do **not** delete the floors row:
  `src/storage/sqlite.rs:2427-2432` (`clear_seeded_runtime_meta`) names `last_scan_ts`,
  `last_indexed_at`, `last_embedded_message_id` and the `historical_bundle_salvaged:%` prefix;
  `src/indexer/mod.rs:20743` names `last_scan_ts` only. So a wipe/rebuild clears the watermark
  and leaves the floors — the safe direction.

### 3.3 Deliberate tolerance that still renders "complete" — a judgment call for the coordinator

`src/storage/sqlite.rs:65-72`:

```rust
pub fn parse_connector_scan_floors(raw: &str) -> BTreeMap<String, i64> {
    let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(raw) else {
        return BTreeMap::new();
    };
```

A floors row that exists but holds malformed JSON (or a JSON array, or a scalar) returns an
**empty** map — which R1/R2 both hand back as `Ok(...)`/`Some(...)`, i.e. *checked*, and which
`connector_coverage_json` renders as `"complete": true` (`src/lib.rs:15197`). So a **corrupt
floors record is still reported as proven-complete coverage** after `8dcd245b`.

The doc comment at `src/storage/sqlite.rs:62-64` defends this deliberately ("failing the whole
read would hide the connectors that *are* reporting"). That argument holds for the per-entry
`filter_map` at line 70 — one bad connector entry should not sink the rest. It does **not**
hold for the whole-blob `else` at 66-68, where there are no other connectors to protect: the
row is unparseable, nothing is reporting, and the honest answer is unknown. This is the same
`Some(empty)`-vs-`None` collapse that `8dcd245b` fixed one layer up, still present one layer
down. I am flagging it, not calling it a regression: it predates the fix and the doc comment
shows it was a decision, not an oversight.

Also in the same family, lower realism: `connector_coverage_floors_from_state`
(`src/lib.rs:15231-15259`) returns `Some(floors)` whenever `checked` is true, with
`.unwrap_or_default()` at `15257` if the `floors` key is missing or not an array, and a
`filter_map` at `15249-15253` that silently drops any entry lacking `connector` or `floor_ts`.
A state envelope with `checked: true` and a malformed `floors` array therefore reads as
complete. The only producer is `connector_coverage_json`, which always writes both fields, so
this is reachable only through a corrupted or foreign envelope — I did not find a producer
that can emit it.

---

## 4. `connector_coverage_json` / `connector_coverage_state_json` after `8dcd291b`/`8dcd245b`

Read directly from source at HEAD `9d4814d2`.

**Q: does an EMPTY map still render `"complete": true`?** — **YES, and by design.**

```
src/lib.rs:15194  fn connector_coverage_json(floors: &BTreeMap<String, i64>) -> serde_json::Value {
src/lib.rs:15195      serde_json::json!({
src/lib.rs:15196          "checked": true,
src/lib.rs:15197          "complete": floors.is_empty(),
```

`connector_coverage_state_json` (`15217-15227`) is the only caller:

```
src/lib.rs:15219          Some(floors) => connector_coverage_json(floors),
src/lib.rs:15220          None => serde_json::json!({
src/lib.rs:15221              "checked": false,
src/lib.rs:15222              "complete": serde_json::Value::Null,
```

So the tri-state is intact and is exactly what the doc comment at `src/lib.rs:15091-15102`
promises: `Some(non-empty)` → checked+incomplete, `Some(empty)` → checked+complete,
`None` → `checked: false`, `complete: null`. What `8dcd245b` changed is that a **failed read**
now produces `None` instead of `Some(empty)`. An empty map still means complete — which is
correct when it came from `Ok(None)` (absent meta row, `src/lib.rs:15121`), and is the residual
hole described in §3.3 when it came from an unparseable row.

**Q: does anything else in the tree render a completeness verdict from a floor map?** — **NO.**

`rg -n "connector_coverage_json|connector_coverage_state_json|connector_coverage_warning|connector_coverage_floors_from_state|connector_coverage_recommended_action" src/ tests/ -g '*.rs'`
returns 26 lines. `connector_coverage_json` is **never** called outside
`connector_coverage_state_json` (`15219`) — the other two mentions (`15099`, `15727`) are prose.
`connector_coverage_state_json` has exactly four production call sites:

```
src/lib.rs:16438        "connector_coverage": connector_coverage_state_json(connector_scan_floors.as_ref()),   // state envelope (status/triage)
src/lib.rs:24015        "connector_coverage": connector_coverage_state_json(connector_scan_floors.as_ref()),   // cass stats --json
src/lib.rs:65308        "connector_coverage": connector_coverage_state_json(connector_scan_floors.as_ref()),   // cass status
src/lib.rs:65952        "connector_coverage": connector_coverage_state_json(connector_scan_floors.as_ref()),   // cass health
```

plus four test call sites (`15719, 15753, 15783, 15810`).

The two human-readable renderings are `src/lib.rs:24069-24093` (`cass stats` text) and
`src/lib.rs:66020-66033` (`cass health` text); both branch on the `Option` and the stats one
prints an explicit `UNKNOWN` line at `24091`. The boolean that feeds `healthy` /
`"degraded"` is `connector_coverage_incomplete`, computed identically at `src/lib.rs:65096-65098`
and `65746-65748` as `.is_some_and(|floors| !floors.is_empty())` — so `None` makes it **false**,
i.e. an unknown coverage read does **not** by itself make the surface degraded; it is reported
as `checked: false` and the health verdict is decided by the other terms.
That is a deliberate consequence of the design, not a bug I am claiming, but it is worth the
coordinator's eye: **`cass health` will still print "healthy" when the coverage read failed**,
with `connector_coverage.checked = false` sitting in the JSON. Verified by reading
`65771-65778` (`healthy = … && !connector_coverage_incomplete`) — nothing in that conjunction
tests `connector_scan_floors.is_none()`.

Other `"complete"` renderings in the tree, checked and excluded: `src/crash_replay.rs:1193/1288`
is a backup-manifest flag written by a test harness (`copy_wal_and_manifest`), unrelated to
floors; `src/analytics/types.rs:320/622` is `api_coverage_pct`, a token-attribution ratio;
`src/indexer/semantic.rs:219` `pub complete: bool` is semantic-embedding progress;
`src/sources/setup.rs:134-159` are wizard step flags.

---

## 5. Test coverage of the readers (context for the coordinator, not a finding)

`src/lib.rs:15687-15817` `mod connector_coverage_honesty_tests` has four tests, all against
**R2/R3 only**:

- `absent_floors_row_reads_as_checked_and_complete` (`15708`)
- `failed_coverage_read_is_unknown_and_never_complete` (`15731`) — has a real positive control
  at `15740-15745`
- `recorded_floor_reads_as_checked_and_incomplete` (`15768`)
- `expired_bound_reports_unknown_rather_than_complete` (`15795`) — asserts only
  `assert_ne!(rendered["complete"], true)` with a 1ns timeout, and has **no** positive control
  proving the bounded reader can return `Some` at a realistic timeout. It would pass against a
  `read_connector_scan_floors_bounded` that always returned `None`.

`src/storage/sqlite.rs:23548` and `:23599` cover R1 and the parser. **Nothing covers R4**
(`read_connector_scan_floors_fresh`) as a unit; `src/indexer/mod.rs:33527/33567` call it as an
*assertion instrument* inside a larger scan test, which means a silent-empty regression in R4
would make those assertions read the wrong thing rather than fail loudly at the read.

`rg -c 'connector_coverage' tests/` → no match (exit 1) across 576 files. **No integration
test pins any of this.**

---

## 6. What this search could NOT have found

Stated so the coordinator knows the boundary of the "complete set" claim.

1. **Dynamic construction of the meta key.** I grepped for `format!("connector`,
   `concat.*connector_scan`, and `"connector_scan`. A key assembled from pieces that share no
   contiguous literal with `connector_scan_floors` (e.g. `"connector" + "_scan_floors"`, or a
   key stored in a config file / env var / another meta row) would not appear. I have no
   evidence such a construction exists; I simply cannot exclude it by grep.
2. **Macro-generated readers.** I did not expand macros. `fparams!` and `params!` are used
   throughout, but I did not check whether any declarative or procedural macro *generates* a
   meta-key accessor. `cargo expand` was not run (out of scope for a read-only lane, and the
   crate is mid-backfill).
3. **Dynamic dispatch / trait objects.** All four readers are free functions or inherent
   methods and are resolved statically. But if a `dyn` trait somewhere exposes a
   `get_connector_scan_floors`-shaped method on an alternate storage backend, my grep on the
   concrete names would still have found the method name — unless the trait method is named
   differently and forwards. I checked no trait definitions.
4. **The `frankensqlite` crate itself.** Everything below `conn.query_row_map` / `close()` /
   `open_with_flags` is external (`src/lib.rs:44` re-exports it). Whether *those* have internal
   bounds is unverified; my axis-(b) answers describe only what this crate does.
5. **Other crates / binaries in the workspace.** I scoped to `src/`, `tests/`, `benches/` of
   this crate. `benches/` produced zero hits for the constant. I did not look at any sibling
   workspace member (I did not check whether one exists).
6. **Runtime behaviour of any kind.** No build, no test, no binary was run — per the lane's
   hard prohibitions. Every "would hang" / "would report complete" statement in this log is a
   source-reading inference about control flow, not a measurement.
7. **`.unwrap()` / `.expect()` panics.** My sweep targeted *silent* swallows. A coverage read
   that panics is loud and was not in scope, so I did not enumerate those.
8. **Swallows outside my topic window.** The §3 sweep required a coverage/floor/watermark/
   completeness word within ±3 lines. A swallow whose surrounding six lines use different
   vocabulary (e.g. a variable named `n` fed into a completeness verdict twenty lines later)
   would not be in the 109. The four floor readers were found by name, not by this sweep, so
   §1's completeness does not depend on the window.
9. **Git history.** I read HEAD only. I did not check whether a fifth reader existed and was
   deleted, or whether `8dcd245b` introduced anything I am attributing to earlier code.

---

## 7. Bottom line

- **The complete set is four entry points over two SQL reads.** R1
  `src/storage/sqlite.rs:6991`, R2 `src/lib.rs:15107`, R3 `src/lib.rs:15161` (wraps R2),
  R4 `src/indexer/mod.rs:10696` (wraps R1). There is no fourth SQL read and no fifth entry
  point; the constant has exactly seven uses and the literal exactly one.
- **R4 is the only reader that swallows, and the only one whose consumer is not an operator
  surface.** Its failure mode is "the hole is never repaired", not "the hole is reported as
  clean".
- **R1, R2, R4 are all unbounded; only R3 bounds the whole operation.** The `cass status`,
  `cass triage` and `cass stats` paths therefore still have an unbounded coverage read on the
  live archive — `8dcd245b` bounded `cass health` alone.
- **Two sibling swallows on the same code path as the fixed one:** the `last_scan_ts` `.ok()`
  at `src/lib.rs:15363` (a failed watermark read reads as "not stale"), and the two
  `.unwrap_or(0)` counts at `src/lib.rs:15376/15381` reported alongside `counts_skipped: false`.
- **One residual `Some(empty)` → `"complete": true`** path survives the fix: an unparseable
  floors row (`src/storage/sqlite.rs:66-68`).
- **One adjacent subsystem has the same shape with a worse consequence:** an unreadable archive
  gives the doctor promotion coverage gate a baseline of 0 and `promote_allowed: true`
  (`src/lib.rs:32315` → `36328` → `36424`). Boundary: downstream blocking unverified.


---

## 8. Corrections to this log (appended, not edited)

- §4 heading reads "after `8dcd291b`/`8dcd245b`". `8dcd291b` is a typo with no
  corresponding object; the only relevant commit is **`8dcd245b`** (`fix(coverage): bound the
  whole coverage read, and stop reporting a failed read as complete`). Read the heading as
  "after 8dcd245b".
- §3.1 cites the doctor source-inventory error branch as `src/lib.rs:32315-32317`. Re-read
  with line numbers: the `match query_result` opens at **32308**, the `Err(err) =>` arm at
  **32312**, and `Vec::new()` — the swallow itself — is at **32316**. The claim is unchanged.
- §3.1 line numbers for the `cass stats` counts confirmed by name-anchored grep:
  `src/lib.rs:23873` (conversations) and `src/lib.rs:23877` (messages), both `.unwrap_or(0)`;
  the open they run against is `src/lib.rs:23834`.
