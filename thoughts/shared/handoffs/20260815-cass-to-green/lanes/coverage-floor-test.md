# Lane: coverage-floor-test — bead coding_agent_session_search-gxw32

Read-only grounding lane. Worktree `cass-to-green-c6bfb589`, commit 74a72233.
Only file written by this lane: this log.

---

## 0. Bead text (verbatim, via `br --no-db show`)

`br show` failed in this worktree — the worktree has no `.beads/beads.db`:

```
$ br show coding_agent_session_search-gxw32
Error: Sync conflict: Refusing storage open because pending sync-merge state
could not be inspected under database-family authority
```

`br --no-db show coding_agent_session_search-gxw32` works and returned the full
body. Key claims to test:

1. "no test holds it there" — a mutant restoring the global watermark ships green.
2. Mutant M2 at `ConnectorScanCoverage::new`:
   `floors.get(name).copied()` → `floors.values().copied().min()`;
   `cargo test --lib` → 5124 passed, 0 failed, 3 ignored, RC=0.
3. Cause is the fixture: `src/indexer/mod.rs:33492` and `:33555` both register
   `vec![("codex", mtime_filtered_aborting_connector_factory)]` — ONE connector.

---

## 1. "connector_coverage appears in ZERO test files" — CONFIRMED, with counts

### Positive control first (proving the instrument can produce a hit)

```
$ rg -c "raw_mirror" tests/
tests/doctor_release_checklist.rs:1
tests/pages_pipeline_e2e.rs:8
tests/doctor_fixture_factory.rs:2
tests/util/doctor_e2e_runner.rs:22
tests/golden/robot_docs/schemas.txt.golden:93
tests/golden/robot/stats_full_payload.json.golden:1
...
rc=0
```

The same command shape over `tests/` finds hits, including inside golden files.

### The subject

```
$ rg -c "connector_coverage" tests/          ; rc=1   (zero matches)
$ rg -c "connector_scan_since_ts|ConnectorScanCoverage|connector_scan_floors|coverage_floor" tests/
                                             ; rc=1   (zero matches)
$ rg -c "incomplete_connectors|connector_coverage" tests/
                                             ; rc=1   (zero matches)
$ rg -c "connector_coverage" tests/golden/    ; rc=1   (zero matches)
```

### Repo-wide count (excluding `target/`)

```
$ rg -c "connector_coverage" -g '!target' .
./src/lib.rs:30
./thoughts/shared/handoffs/... (docs only, 8 files)
```

`connector_coverage` exists in exactly ONE source file (`src/lib.rs`, 30 lines)
and in zero test files.

### Are any of those 30 inside an inline `#[cfg(test)]` module?

Classifier: for hit line L, find the nearest preceding column-0 `#[cfg(test)]`
(T) and the nearest preceding column-0 `}` (C); L is inside a test module iff
T > C (top-level items close at column 0).

Positive control for the classifier:

```
'assert_eq!':              total=1182  inside_test_mod=1143  outside=39
'connector_coverage':      total=30    inside_test_mod=0     outside=30
'connector_scan_floors':   total=25    inside_test_mod=0     outside=25
'connector_scan_since_ts': total=0     (not referenced in lib.rs)
'ConnectorScanCoverage':   total=0     (not referenced in lib.rs)
```

`assert_eq!` lands 1143/1182 inside test modules, so the classifier can produce
a positive. `connector_coverage` is 0/30.

### What IS tested

`src/storage/sqlite.rs` `#[cfg(test)]`:

- `:23548 connector_scan_floors_round_trip_and_clear` — record/clear/persist.
- `:23585 connector_scan_since_ts_lowers_to_the_floor` — the pure two-argument
  function `connector_scan_since_ts(run, floor)`.
- `:23599 parse_connector_scan_floors_tolerates_junk`.

`src/indexer/mod.rs` `#[cfg(test)] mod tests`:

- `:33444 aborted_connector_scan_does_not_leave_the_index_claiming_complete_coverage`
  — the two-pass end-to-end fixture, **one connector**.

**The gap:** `ConnectorScanCoverage::new` — the code that decides WHICH
connector's floor a given connector gets — has zero direct coverage. The pure
selector is tested; the per-connector dispatch is not. `ConnectorScanCoverage`
is never named anywhere under `tests/`, and inside `src/indexer/mod.rs` it is
only ever constructed in test code as `ConnectorScanCoverage::default()`
(`:34508, :34557, :34625, :34682, :35108, :35347, :35452, :35531, :35578, :35716`)
— i.e. always with an empty floor map, which cannot distinguish the mutant.

---

## 2. What the coverage floor is, in plain English

Source: `src/storage/sqlite.rs:55-88`, `src/indexer/mod.rs:10620-10721`.

cass keeps ONE global watermark, `meta.last_scan_ts` — "the last time a run
finished." Every incremental run asks each connector for files modified after
that watermark. That is fine while every connector succeeds.

It breaks when one connector dies mid-scan. The run keeps going (a dead
connector must not abandon the others' work), the run finishes, and the global
watermark advances past everything — **including the files the dead connector
never opened.** Every later incremental run then filters those files out by
mtime, so the hole is permanent. That is the 2026-06-01 codex incident recorded
at `src/indexer/mod.rs:10635-10639`: 3,186 files.

The coverage floor is the repair. A floor is a per-connector entry in a single
JSON blob under `meta` key `connector_scan_floors`
(`src/storage/sqlite.rs:60`), of the form `{"codex": 1749000000000}`. It means:
"this connector is only *proven* to have read up to here."

- Written at the moment of failure, from the `since_ts` that run was using:
  `src/indexer/mod.rs:11328` (streaming path, on `IndexMessage::ScanError`) and
  `:11889` (parallel path, on `scan_failed`).
- Lowering only — a second failure at a later watermark cannot shrink the hole
  the first one opened (`src/storage/sqlite.rs:7029-7034`).
- Cleared only by a later clean scan of the *same* connector that started at or
  below it: `src/indexer/mod.rs:11359-11366` and `:11890-11892`.
- Survives restarts, and is read through a *fresh* read-only open rather than
  the caller's long-lived handle, because a long-lived MVCC snapshot cannot see
  a floor committed during the scan (`src/indexer/mod.rs:10686-10721`).

### Why it must be per-connector

`ConnectorScanCoverage::new` (`src/indexer/mod.rs:10649-10665`):

```rust
let since_ts_by_connector = connector_names
    .into_iter()
    .map(|name| {
        let since = connector_scan_since_ts(run_since_ts, floors.get(name).copied());
        (name, since)
    })
    .collect();
```

`floors.get(name)` is the whole point. Each connector's scan window is the
run-wide watermark **lowered only by its own floor**:

- `since_ts_for(name)` (`:10669`) feeds the producer's `ScanContext.since_ts` at
  `:11595` (streaming) and `:11755` (parallel). That is the mtime cutoff the
  connector actually scans from.
- `failure_floor_for(name)` (`:10677`) is what gets written if this connector
  fails this run.
- `has_floor(name)` (`:10681`) gates whether a clean finish clears anything.

Semantics of the floor arithmetic (`src/storage/sqlite.rs:81-88`):
`None` run watermark → read everything; floor `<= 0` → the failed run had no
watermark at all, so nothing is proven, read everything; otherwise
`Some(run.min(floor))`.

So a healthy connector with no floor keeps the ordinary fast incremental
window, and only the connector that actually broke pays for re-reading.

### What the global-watermark mutant does differently

M2 replaces `floors.get(name).copied()` with `floors.values().copied().min()`.
Now every connector's window is widened back to the *earliest floor recorded by
any connector*. Consequences:

- **Correctness of the report is unchanged** — the floor map is still keyed by
  connector, so `cass health` still names the right broken connector. This is
  why the mutant is invisible to every existing assertion.
- **Cost explodes** — one broken connector drags every other connector back to
  its floor on every subsequent run, re-reading the whole archive for all of
  them until the broken one is repaired.
- **Clearing goes wrong** — `clear_connector_scan_floor` is called with
  `coverage.since_ts_for(name)` (`:11364`, `:11891`), so under M2 a clean
  connector's "I read from here" proof is a different connector's floor.
- With a floor of `0` recorded for any connector, `connector_scan_since_ts`
  returns `None` for **every** connector: every incremental run becomes a full
  rescan of everything, forever.

The property M2 destroys is *isolation*: a floor belongs to one connector and
must not move any other connector's window. Nothing asserts isolation today,
because the fixture registers one connector, and with one connector
"its own floor" and "the minimum of all floors" are the same value by
construction.

---

## 3. Second defect: `complete: true` over the hole — CONFIRMED

### The code that computes `complete`

`src/lib.rs:15126-15142`:

```rust
fn connector_coverage_json(floors: &BTreeMap<String, i64>) -> serde_json::Value {
    serde_json::json!({
        "checked": true,
        "complete": floors.is_empty(),
        "incomplete_connectors": floors.keys().cloned().collect::<Vec<_>>(),
        ...
```

`complete` is **exactly** `the floors map is empty`. It is not a comparison
against the on-disk corpus, not a count, not a re-scan. It is "no connector has
reported a mid-scan abort that is still outstanding."

That verdict propagates into the health status. `src/lib.rs:64882-64927`
(`run_status`, same shape at `:65528-65600` in `run_health`):

```rust
let connector_scan_floors = connector_coverage_floors_from_state(&state);
let connector_coverage_incomplete = connector_scan_floors
    .as_ref()
    .is_some_and(|floors| !floors.is_empty());
...
let healthy = db_exists && db_available && index_exists && index_fresh
    && !rebuild_active && !index_empty_with_messages
    && !ingest_quarantine_critical
    && !connector_coverage_incomplete;
```

So an empty floor map is an affirmative input to `status: "healthy"`.

### Why the floor map is empty over a real hole — two independent proofs

**(a) Dates.** The floor mechanism first shipped in `e3ed01f0`, 2026-08-10:

```
$ git log --format='%h %ad %s' --date=short -S 'CONNECTOR_SCAN_FLOORS_META_KEY' -- src/storage/sqlite.rs
e3ed01f0 2026-08-10 fix(indexer): an aborted connector scan can no longer claim complete coverage
```

The hole it describes was opened 2026-06-01 (`src/indexer/mod.rs:10635-10639`),
seventy days before any code existed that could record a floor. A floor is only
ever written from a live `ScanError`/`scan_failed` observed by the running
process (`src/indexer/mod.rs:11328`, `:11889`). Nothing back-fills floors, and
nothing derives a floor from the archive's own contents. The signal is strictly
forward-looking: it can only describe failures observed after 2026-08-10.

**(b) The live archive.** Read-only, against
`~/Library/Application Support/com.coding-agent-search.coding-agent-search/agent_search.db`:

```
$ sqlite3 "file:...agent_search.db?mode=ro" ".schema meta"
CREATE TABLE IF NOT EXISTS meta ("key" TEXT PRIMARY KEY, value TEXT NOT NULL);

$ sqlite3 "file:...agent_search.db?mode=ro" "SELECT key, substr(value,1,120) FROM meta ORDER BY key;"
last_indexed_at|1786791880990
last_scan_ts|1784196225836
schema_version|20

$ sqlite3 "file:...agent_search.db?mode=ro" \
    "SELECT count(*) FROM meta WHERE key='connector_scan_floors';"
0
```

Positive control on the same query shape (so the zero is a real zero, not a
dead instrument):

```
$ sqlite3 ... "SELECT count(*) FROM meta; SELECT count(*) FROM meta WHERE key='last_scan_ts';"
3
1
```

Timestamps for context:

```
$ sqlite3 ... "SELECT key, value, datetime(CAST(value AS INTEGER)/1000,'unixepoch') FROM meta
               WHERE key IN ('last_scan_ts','last_indexed_at');"
last_indexed_at|1786791979679|2026-08-15 11:06:19
last_scan_ts   |1784196225836|2026-07-16 10:03:45
```

The live archive holds **three** meta rows and none of them is
`connector_scan_floors`. Therefore `get_connector_scan_floors()`
(`src/storage/sqlite.rs:6991-7002`) returns an empty `BTreeMap`, therefore
`connector_coverage.complete == true`, therefore
`connector_coverage_incomplete == false` and the coverage term contributes
`healthy = true`. This is happening right now, while a codex backfill of
~4,895 rollouts is in flight and bead `-qtn0e` records 3,877 Claude Code
conversations that exist only in cass.

**Verdict: the handoff's claim is correct.** `connector_coverage.complete` is
not an acceptance signal for "the archive holds everything." It answers a
strictly narrower question — "has any connector reported a mid-scan abort since
2026-08-10 that has not yet been cleared?" — and the two questions are being
conflated at `src/lib.rs:64904` and `:65565`.

Note the code already knows this distinction and states it, one function up, at
`src/lib.rs:15144-15148`:

> `checked: false` is not the same claim as `complete: true`, and collapsing the
> two is the whole shape of this bug: an unchecked surface that reads as a clean
> one.

The same sentence applies one level out: *no floor recorded* is not the same
claim as *the archive is complete*, and the health verdict collapses those two.

### Related: the floor survives a rebuild

The `--force-rebuild` truncation path (`src/indexer/mod.rs:20718-20745`) deletes
conversations/messages/agents and `meta.last_scan_ts` but does **not** delete
`connector_scan_floors`. So the empty floor map on the live DB is not the result
of a rebuild wiping it — no floor was ever written.

---

## 4. Side finding for bead -a4xe1 (golden_robot_json red since e3ed01f0)

Not my bead, but it fell out of the same search and is directly actionable.

`e3ed01f0` added `connector_coverage` to four robot-JSON payloads and
regenerated **no** goldens:

```
$ git show --stat --format='' e3ed01f0
 src/indexer/mod.rs    | 542 +++++++...
 src/lib.rs            | 239 +++++...
 src/storage/sqlite.rs | 173 +++++...
 3 files changed, 932 insertions(+), 22 deletions(-)
```

Emit sites (all outside test modules):

| site | enclosing fn |
|---|---|
| `src/lib.rs:16229` | `state_meta_json_inner` (the `state` block) |
| `src/lib.rs:23806` | `run_stats` |
| `src/lib.rs:65095` | `run_status` |
| `src/lib.rs:65739` | `run_health` |

The shape goldens do not have the key:

```
health_shape.json.golden          -> connector_coverage present: False | 22 top-level keys
  keys: coverage_risk, data_dir, db, doctor_summary, errors, explanation,
        health_level, healthy, ingest_quarantine, initialized, latency_ms,
        parallel_wal_shadow, policy_registry, rebuild_progress,
        recommended_action, recommended_commands, remote_source_sync,
        responsiveness, runtime_optimizations, state, status, warnings
  state sub-keys: _meta, database, index, ingest_quarantine, pending,
        policy_registry, rebuild, semantic          <- no connector_coverage
status_shape.json.golden          -> False | 23 top-level keys
stats_full_payload_shape.json.golden -> False | 7 top-level keys
```

`tests/golden_robot_json.rs:1-24` says these goldens freeze the **shape** of the
payload, and `tests/golden/robot/health_shape.json.golden` was last regenerated
2026-05-28 (`fb75daab`), 74 days before `e3ed01f0`.

Status: the shape mismatch is CONFIRMED by reading both sides. That this is the
cause of `golden_robot_json` being red is UNVERIFIED here — I did not run the
test (read-only lane, no long builds).

---

## 5. Where the test belongs, and the mutant it must catch

### Test A — mandatory, cheap, kills M2 deterministically

**File:** `src/indexer/mod.rs`
**Module:** the top-level `#[cfg(test)] mod tests` that opens at line 26351-26352
(`use super::*;` at 26353, so private `ConnectorScanCoverage` is in scope).
`BTreeMap` is already imported at `src/indexer/mod.rs:18`.
**Placement:** immediately after
`aborted_connector_scan_does_not_leave_the_index_claiming_complete_coverage`
(ends line 33574), so the two coverage tests sit together.
**Name:** `each_connector_scans_from_its_own_coverage_floor`
**No `#[serial]`, no `TempDir`, no fixture statics** — this is a pure
constructor test.

**Body, precisely:**

1. Build `floors: BTreeMap<String, i64>` = `{"codex": 100, "claude": 400}`.
2. `let coverage = ConnectorScanCoverage::new(Some(1_000), floors, ["codex", "claude", "amp"]);`
3. Assert `coverage.since_ts_for("codex") == Some(100)`
4. Assert `coverage.since_ts_for("claude") == Some(400)`
5. Assert `coverage.since_ts_for("amp") == Some(1_000)`
   — *the load-bearing one*: a connector with no floor of its own keeps the
   run-wide watermark and is not dragged back by anybody else's floor.
6. Assert `coverage.failure_floor_for("codex") == 100` and
   `coverage.failure_floor_for("amp") == 1_000`
   — the floor recorded on failure is also per-connector.
7. Assert `coverage.has_floor("codex")` and `!coverage.has_floor("amp")`.

**Why these numbers.** Three expected values — 100, 400, 1000 — are pairwise
distinct and distinct from the run watermark, so no assertion can pass by
coincidence. Three connectors, two floored and one clean, is the minimum that
distinguishes "own floor" from every plausible aggregate.

**The mutant it must catch**, verbatim, at `src/indexer/mod.rs:10657`:

```rust
-  let since = connector_scan_since_ts(run_since_ts, floors.get(name).copied());
+  let since = connector_scan_since_ts(run_since_ts, floors.values().copied().min());
```

Under that mutant every connector receives `min(1_000, 100) = Some(100)`, so
step 4 fails (`Some(100)` vs `Some(400)`), step 5 fails (`Some(100)` vs
`Some(1_000)`), and step 6's `amp` assertion fails (`100` vs `1_000`). Three
independent assertions die, so the test cannot be re-armed by adjusting one
number.

The same test also kills the neighbouring mutants that a future refactor could
introduce: `.max()` instead of `.min()` (step 3 fails), `floors.values().next()`
/ first-floor-wins (steps 4 and 5 fail), and dropping the floor lookup entirely
(`None`, steps 3 and 4 fail).

**Verifying the test is not vacuous:** apply the mutant above, run
`cargo test --lib each_connector_scans_from_its_own_coverage_floor`, and confirm
*that named case* goes red — not merely that the suite's failure count moved.
Then revert and confirm it goes green. A green suite under the mutant means the
test was written against a weaker property than isolation.

### Test B — optional behavioral companion (the bead's own proposal)

**Same file and module.** Name:
`a_failed_connector_scan_does_not_widen_the_healthy_connectors_window`.

This is the end-to-end version, and it costs fixture surgery. The existing
fixture cannot express two connectors:

- `COVERAGE_FIXTURE_ROOT: Mutex<Option<PathBuf>>` (`:33323`) and
  `COVERAGE_FIXTURE_ABORT_AFTER: Mutex<Option<usize>>` (`:33324`) are single
  global slots, not per-connector.
- `MtimeFilteredAbortingConnector::scan_with_callback` hard-codes
  `conversation.agent_slug = "codex"` (`:33395`).

To register two connectors, both statics must become
`Mutex<BTreeMap<&'static str, _>>` keyed by connector name, and the connector
struct must carry its own `name` and use it for `agent_slug` and for its root.
Then both call sites (`:33492`, `:33555`) become
`vec![("codex", codex_factory), ("claude", claude_factory)]`.

Shape: pass 1 — codex aborts after 1 of 3 rollouts, claude reads its 3 cleanly;
assert `read_connector_scan_floors_fresh(&db_path)` afterwards contains exactly
one key, `"codex"`. Pass 2 — ordinary incremental; assert all 6 conversations
are present and that claude's producer ran with the run watermark rather than
codex's floor.

**Caveat, so nobody builds Test B expecting it to be sufficient:** pass 1 alone
does NOT distinguish M2. On pass 1 the floor map is empty, so
`floors.values().min()` is `None` and the mutant computes the same `since_ts`
as the baseline. Only pass 2, where `floors = {"codex": X}` is non-empty,
separates them — and separating them through conversation counts requires
arranging claude's rollout mtimes to straddle `X`, which is fragile. Test A
observes the same property directly and is the one that must exist.

---

## 6. What I did not do

- Did not run `cargo test`, `cargo check`, or any build (read-only lane, no
  long builds authorised). The bead's "5124 passed / 5127 tests" figure is
  cited, not re-measured — UNVERIFIED here.
- Did not run `cass` in any form. Did not write to the live archive.
- Did not implement either test.
