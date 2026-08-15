# Lane gen3-third-copy — `read_connector_scan_floors_fresh`

Read-only lane. Subject: `src/indexer/mod.rs:10696-10721`.
Every claim below cites file:line or verbatim command output. Where I did not run
something, I say so.

---

## 0. Background verified (not assumed)

`git show 8dcd245b --stat` → `src/lib.rs | 253 ++++...`, one file. So the committed
fix touched **only** `src/lib.rs`. `src/indexer/mod.rs` was not in the commit.

`git show 8dcd245b -- src/lib.rs` confirms both halves as described in the task:

- Fix A: `read_connector_scan_floors_bounded` (lib.rs:15161-15190 post-fix) now runs
  open+read+close on a `std::thread::spawn` worker and waits once with
  `rx.recv_timeout(timeout)`.
- Fix B: `read_connector_scan_floors` (lib.rs:15091-15129 post-fix) returns
  `Option<BTreeMap<String, i64>>`; the `Err(err)` arm logs `warn!` and returns `None`.

Call sites of the third copy — `rg -n "read_connector_scan_floors_fresh" src/`:

```
src/indexer/mod.rs:10696:fn read_connector_scan_floors_fresh(db_path: &Path) -> BTreeMap<String, i64> {
src/indexer/mod.rs:11568:        read_connector_scan_floors_fresh(&opts.db_path),
src/indexer/mod.rs:11737:        read_connector_scan_floors_fresh(&opts.db_path),
src/indexer/mod.rs:33527:        let floors_after_failure = read_connector_scan_floors_fresh(&db_path);
src/indexer/mod.rs:33567:            read_connector_scan_floors_fresh(&db_path).is_empty(),
```

11568 is inside `run_streaming_index_with_connector_factories` (declared at 11513);
11737 is inside `run_batch_index_with_connector_factories` (declared at 11699).
Both are on the live `cass index` path: `run_streaming_index` (11448) is called at
13055 and `run_batch_index` (11677) at 13078, the two arms of the same
`CASS_STREAMING_INDEX` branch. 33527/33567 are test uses.

---

## 1. CAN IT HANG?

**Yes. The path reaches an unbounded spin loop, and there is no wall-clock bound
anywhere on it.** But the branch that plausibly produced the lib.rs >150s stall is
skipped in the indexer for an incidental reason, which is why the running backfill
has not hit it. Both halves matter; I give the chain, then the calibration.

### 1a. The call chain, hop by hop

| # | file:line | code |
|---|---|---|
| 1 | `src/indexer/mod.rs:10700` | `FrankenStorage::open_readonly(db_path)` |
| 2 | `src/indexer/mod.rs:10719` | `let _ = storage.close();` |
| 3 | `src/storage/sqlite.rs:4190-4196` | `pub fn close(self) -> Result<()>` → `this.conn.close()` |
| 4 | `src/storage/sqlite.rs:3736` | field is `conn: FrankenConnection`; `sqlite.rs:7` imports `Connection as FrankenConnection` from `frankensqlite` |
| 5 | `fsqlite-0.1.5/src/lib.rs:6-9` | `pub use fsqlite_core::connection::{Connection, ...}` |
| 6 | `fsqlite-core-0.1.5/src/connection.rs:12660-12662` | `pub fn close(mut self) -> Result<()> { self.close_in_place() }` |
| 7 | `fsqlite-core-0.1.5/src/connection.rs:12676-12678` | `pub fn close_in_place(&mut self) { self.close_internal(false, true) }` |
| 8 | `fsqlite-core-0.1.5/src/connection.rs:12810-12812` | `self._shared_mvcc_state.release_connection(self.runtime_region, false)?` |
| 9 | `fsqlite-core-0.1.5/src/connection.rs:62707-62718` | `impl SharedMvccState::release_connection` → `lock_unpoisoned(&self.runtime_state)`, then `state.regions.close_and_drain(connection_region)` |
| 10 | `fsqlite-core-0.1.5/src/region.rs:465-468` | `close_and_drain` → `begin_close` then `drain_subtree` |
| 11 | `fsqlite-core-0.1.5/src/region.rs:481-483` | **`while self.active_tasks(id) > 0 \|\| self.active_obligations(id) > 0 { std::hint::spin_loop(); }`** |

Hop 7 is **the identical function** the committed fix was written around:
`src/lib.rs:14229` in `close_franken_cli_read_db` is `conn.close_in_place()`.
`FrankenStorage::close()` reaches it through one extra hop. So the indexer's close
is not "an equivalently expensive step" — it is the same step.

The open side is the same function too. `open_franken_cli_read_db` (lib.rs:14084)
calls `open_franken_readonly_storage_with_timeout` (sqlite.rs:637-665), whose loop
body at 648 is `FrankenStorage::open_readonly(path)` — byte-identical to
indexer/mod.rs:10700.

Version check (instrument discipline): `Cargo.lock:2270-2273` pins `fsqlite 0.1.5`
and `Cargo.lock:2313-2316` pins `fsqlite-core 0.1.5`. My first `rg` for
`release_connection` matched `fsqlite-core-0.1.17`, which is **not** the pinned
crate; every line cited above was re-read from the 0.1.5 source tree.

### 1b. Every bound on the path, enumerated

- `FrankenStorage::open_readonly` (sqlite.rs:4172-4174) → 30s, and only on the
  *doctor mutation lock* (`DOCTOR_MUTATION_DB_OPEN_LOCK_TIMEOUT`, sqlite.rs:42;
  guard at sqlite.rs:494-565, deadline at 515, `Err` at 540-547).
- `open_franken_with_flags(...)` (sqlite.rs:4183) — **no bound**.
- `apply_readonly_config` (sqlite.rs:4299-4313) sets `PRAGMA busy_timeout = 5000`,
  which is a lock bound, not a work bound. Same category error Fix A named.
- `storage.get_connector_scan_floors()` (sqlite.rs:6991-7002) — **no bound**.
- `storage.close()` — **no bound**, and it terminates in the spin loop at
  region.rs:481-483, which has no deadline, no yield, and no `Result` escape.

There is no `recv_timeout`, no worker thread, no deadline anywhere in
`read_connector_scan_floors_fresh`. It runs on the caller's thread.

### 1c. No close variant escapes, including `Drop`

- `close_in_place` → `close_internal(false, true)` → `release_connection(_, false)` (12810)
- `close_without_checkpoint_in_place` → `close_internal(false, false)` → same (12810)
- `close_best_effort_in_place` → `close_internal(true, false)` → `release_connection(_, true)` (12806)
- `impl Drop for Connection` (connection.rs:56743-56757) → `close_internal(true, false)` → same

So "just drop it" or "use the cheap close" is not a remedy. Every exit from a
`Connection` value passes through `release_connection` and therefore through the
spin loop.

Note also that the WAL checkpoint at connection.rs:12792-12803 is gated on
`!self.pager.is_readonly()`, so for a `SQLITE_OPEN_READ_ONLY` handle the checkpoint
is skipped. The checkpoint is **not** the expensive part in either copy. That is
consistent with the lib.rs measurement, whose connection was also read-only.

### 1d. Calibration — why the running backfill has not hung

`release_connection` has a second, much more expensive branch:

```
connection.rs:62744   if state.open_connections == 0 {
connection.rs:62748       let db_root_region = state.db_root_region;
connection.rs:62757       if let Err(err) = state.regions.close_and_drain(db_root_region) {
```

That drains the **whole database's root region**, not just the closing
connection's. `SharedMvccState` is process-global per canonicalized path
(`SHARED_MVCC_STATE_BY_PATH`, connection.rs:62810; key is
canonical path + runtime id, 62261-62273, 62819-62830, 62833-62853).

In `cass index`, both call sites already hold a long-lived `storage:
&FrankenStorage` on the same `opts.db_path` (the signatures at 11513 and 11699 take
it; it is used at 11328/11889). So when the ephemeral coverage connection releases,
`open_connections` drops to ≥1, never to 0, and the root drain at 62744 is skipped.
Only the ephemeral connection's own region is drained — and that connection ran one
`SELECT value FROM meta WHERE key = ?1` (sqlite.rs:6992-6996), so it has almost
nothing to drain.

In `cass status`/`stats`/`health`/`triage` the ephemeral coverage connection may
well be the *last* one in its process, which takes the root drain. I did not measure
this, and the commit itself says the hang was not re-measured — so treat this as the
most plausible mechanism, not a proven one.

### 1e. Live state, read-only

Deployed binary: `/Users/dalecarman/.local/bin/cass` (52M, 14 Aug 16:56),
`cass 0.6.9`, `git commit: 447d97fe60962d1ed1f34841e508f61a6b4302c4`.

`git merge-base --is-ancestor e3ed01f0 447d97fe` → rc=0, and
`git show 447d97fe:src/indexer/mod.rs` contains the function **byte-identical at the
same lines 10696-10721**. The function was introduced by `e3ed01f0` (2026-08-10),
so the running backfill's binary carries exactly this defect.

`ps -Ao pid,etime,%cpu,command | rg 'cass '` → pid 40482, etime 02:19, 102.1% CPU,
`cass index --watch-once <~250 rollout paths> --json --progress-interval-ms 60000`.
It is progressing. Backfill chunks are separate short `cass index` processes, so
this code has been traversed many times without hanging.

### 1f. Verdict on Q1

Structurally: **unbounded, and it can hang.** The one thing standing between
`cass index` and an unbounded spin is that some other connection to the same
canonical path happens to stay open for the duration — an incidental property of the
current call sites, not a designed invariant, and nothing in the code or the tests
records it. Any future call site that reads the floors without a live sibling
connection takes the full root drain with no bound at all.

Empirically: **this is not the hang that was observed.** I have no measurement
showing `cass index` stalling here, and the running backfill is direct evidence
against it firing routinely. Reporting it as an active production hang would
overclaim. The honest framing is a latent unbounded path, not a live fire.

---

## 2. WHAT DOES A SWALLOWED FAILURE ACTUALLY COST HERE?

### 2a. The reading in the task is CONFIRMED

`ConnectorScanCoverage::new` (indexer/mod.rs:10649-10665) builds
`since_ts_by_connector` at 10657:

```rust
let since = connector_scan_since_ts(run_since_ts, floors.get(name).copied());
```

`connector_scan_since_ts` (sqlite.rs:81-88):

```rust
match (run_since_ts, floor) {
    (None, _) => None,
    (Some(_), Some(floor)) if floor <= 0 => None,
    (Some(run), Some(floor)) => Some(run.min(floor)),
    (Some(run), None) => Some(run),
}
```

An empty `floors` map means `floors.get(name)` is `None` for every connector, so
every connector takes the `(Some(run), None) => Some(run)` arm — the ordinary
incremental watermark, unwidened. `since_ts_for` (10669-10671) then hands that to
each producer at 11595 (`since_ts: coverage.since_ts_for(name)`) and 11755.

So: **a failed read runs the scan from the ordinary watermark and silently re-skips
exactly the files the floor existed to recover.** The function's own doc comment at
indexer/mod.rs:10706-10707 admits this in the open-failure arm — *"previously failed
connectors will not be widened this run"* — and then returns `BTreeMap::new()`
anyway. The query-failure arm at 10712-10718 does the same thing without even that
sentence.

### 2b. But the miss is RECOVERABLE, not permanent — two independent mechanisms

**Mechanism 1 — the floor is never cleared, because `has_floor` is false.**
`has_floor` (10681-10683) is `self.floors.contains_key(connector)`. Both clear sites
are gated on it:

```
indexer/mod.rs:11359   if is_discovered && stats.error.is_none() && coverage.has_floor(connector_name) {
indexer/mod.rs:11360       clear_connector_scan_floor(...)
indexer/mod.rs:11890   } else if *discovered && coverage.has_floor(name) {
indexer/mod.rs:11891       clear_connector_scan_floor(storage, false, name, coverage.since_ts_for(name));
```

With an empty map `has_floor` is false, so the clear never fires and the `meta` row
survives the run.

**Mechanism 2 — a new floor can never be raised.**
`FrankenStorage::record_connector_scan_floor` (sqlite.rs:7026-7037) re-reads the
floors fresh from the database and refuses to write a *higher* one:

```rust
let mut floors = self.get_connector_scan_floors()?;      // 7027
let floor_ts = floor_ts.max(0);                          // 7028
if floors.get(connector).is_some_and(|existing| *existing <= floor_ts) {
    return Ok(());                                       // 7033
}
```

This matters because on a failed read `failure_floor_for` (10677-10679) returns the
*unwidened* run watermark, which is higher than the true floor. Without 7029-7034
that would overwrite the real floor with a worse one and make the hole permanent.
It does not. This property is directly pinned by a passing test —
`connector_scan_floors_round_trip_and_clear`, sqlite.rs:23559-23567: *"the earliest
unproven point wins; a second failure cannot narrow the first"*.

### 2c. So what is the real cost?

Not data loss. The cost is:

1. **One or more index runs silently fail to do the recovery they exist to do.** The
   operator runs `cass index`, it reports success, the hole is not filled, and
   nothing in the run's output distinguishes that from a run that did fill it.
   The only trace is a `warn!` (10703 / 10713).
2. **The failure repeats for as long as its cause persists.** It is not one lost
   run; it is every run until the read succeeds.
3. **The archive still reports INCOMPLETE**, because `cass stats`'s coverage block
   reads the floors through a different path (lib.rs). So the *coverage claim* is
   not falsified by this defect. That is the sharp difference from lib.rs, where an
   empty map rendered as `"complete": true`.

**Concrete non-hypothetical trigger.** `cass doctor --fix` holds the doctor mutation
lock. `FrankenStorage::open_readonly` waits 30s on it (sqlite.rs:515, 540-547) and
then returns `Err`. That lands in the 10702-10710 arm → `warn!` → empty map → no
widening, for that whole index run. No corruption and no hang required.

---

## 3. RECOMMENDED REMEDY

**Recommendation: make the read bounded and tri-state exactly as `src/lib.rs` now
does, and have the caller log at `error!` and proceed. Do not widen. Do not fail the
run.**

### Why not the alternatives

- **Widen to a full rescan on `None`.** Disproportionate. §2b proves the miss is
  recoverable at the cost of a delayed run, while a full rescan of a 7.9 GB archive
  is the expensive thing incremental indexing exists to avoid. Paying a full rescan
  to recover a hole that a later successful read recovers for free is a bad trade,
  and it would fire on every transient `cass doctor --fix` overlap.
- **Propagate the error and fail the index run.** Contradicts the repo's own stated
  stance at indexer/mod.rs:11323-11324 — *"The run continues — one dead connector
  must not abandon the others' work."* A failed floors read is strictly less severe
  than a failed connector scan, which explicitly does not abort the run. It would
  also convert a recoverable miss into a hard outage during any `doctor --fix`.
- **Add a `floors_known: bool` to `ConnectorScanCoverage` to gate the clear.**
  Unnecessary. §2b Mechanism 1 shows the clear is already gated by `has_floor`,
  which is false for both `None` and `Some(empty)`. A new field to re-express an
  invariant that already holds is a mechanism larger than its problem
  (`right-sized-mechanism.md`).

### The change — exact current code, and the replacement

**Replace this (`src/indexer/mod.rs:10696-10721`, verbatim):**

```rust
fn read_connector_scan_floors_fresh(db_path: &Path) -> BTreeMap<String, i64> {
    if !db_path.exists() {
        return BTreeMap::new();
    }
    let storage = match FrankenStorage::open_readonly(db_path) {
        Ok(storage) => storage,
        Err(error) => {
            tracing::warn!(
                db_path = %db_path.display(),
                error = %error,
                "could not open the archive to read connector scan coverage floors; \
                 previously failed connectors will not be widened this run"
            );
            return BTreeMap::new();
        }
    };
    let floors = storage.get_connector_scan_floors().unwrap_or_else(|error| {
        tracing::warn!(
            error = %error,
            "could not read connector scan coverage floors"
        );
        BTreeMap::new()
    });
    let _ = storage.close();
    floors
}
```

**With (sketch — this lane wrote no source; the parent owns the edit):**

```rust
/// How long an index run will wait for the whole coverage read — open, query and
/// close — before giving up and running unwidened.
///
/// Must exceed `DOCTOR_MUTATION_DB_OPEN_LOCK_TIMEOUT` (30s), or a legitimate wait
/// on `cass doctor --fix` is indistinguishable from a hang and always trips this
/// bound instead of resolving inside it.
const INDEX_COVERAGE_READ_TIMEOUT: Duration = Duration::from_secs(60);

/// ... (existing doc comment about reading through a fresh connection kept) ...
///
/// Tri-state, matching `read_connector_scan_floors` in `lib.rs`:
/// `Some(non-empty)` a scan aborted; `Some(empty)` none has; `None` the read failed
/// and this run cannot know. Three copies of one read must agree.
///
/// ceiling: on expiry the worker is orphaned holding the connection. Unlike the CLI
/// case this process is long-lived, and the orphan finishes inside
/// `fsqlite` `region.rs` `drain_subtree`, which spins rather than sleeps — so an
/// expiry costs a busy core for the remainder of the drain.
fn read_connector_scan_floors_fresh(db_path: &Path) -> Option<BTreeMap<String, i64>> {
    if !db_path.exists() {
        return Some(BTreeMap::new());
    }
    let (tx, rx) = std::sync::mpsc::channel();
    let path = db_path.to_path_buf();
    let _worker = std::thread::spawn(move || {
        let floors = match FrankenStorage::open_readonly(&path) {
            Ok(storage) => {
                let floors = storage.get_connector_scan_floors().map_err(|error| {
                    tracing::error!(error = %error, "could not read connector scan coverage floors");
                }).ok();
                let _ = storage.close();
                floors
            }
            Err(error) => {
                tracing::error!(
                    db_path = %path.display(), error = %error,
                    "could not open the archive to read connector scan coverage floors"
                );
                None
            }
        };
        let _ = tx.send(floors);
    });

    match rx.recv_timeout(INDEX_COVERAGE_READ_TIMEOUT) {
        Ok(floors) => floors,
        Err(_) => {
            tracing::error!(
                db_path = %db_path.display(),
                "connector coverage read exceeded its bound",
            );
            None
        }
    }
}
```

and at the two call sites, keep `ConnectorScanCoverage::new`'s signature as
`BTreeMap` and unwrap at the boundary with the honest log:

```rust
let floors = read_connector_scan_floors_fresh(&opts.db_path).unwrap_or_else(|| {
    tracing::error!(
        "coverage floors are UNKNOWN for this run; previously failed connectors \
         will NOT be widened and their holes stay open until a later run reads them"
    );
    BTreeMap::new()
});
let coverage = ConnectorScanCoverage::new(since_ts, floors, /* names */);
```

Note the `!db_path.exists()` arm returns `Some(BTreeMap::new())`, not `None` — a
database that does not exist yet genuinely has no aborted scan. That is the same
distinction lib.rs's `Ok(None) => Some(BTreeMap::new())` arm draws.

### What it costs

- One `std::thread::spawn` per index run (one of the two call sites executes per run).
- One new `Duration` constant, following `CLI_DIAG_DB_OPEN_TIMEOUT` (lib.rs:15088)
  and `HEALTH_COVERAGE_OPEN_TIMEOUT` (lib.rs:15135) as the repo's idiom.
- On expiry, an orphaned worker thread — and here it is worse than in lib.rs,
  because the CLI process exits shortly after while an index run does not, and
  region.rs:482 is `std::hint::spin_loop()` rather than a sleep. Naming that as a
  `ceiling:` comment is part of the change, not an afterthought.
- No behaviour change on the happy path: `Some(map)` unwraps to the same map.
- It does **not** fix the underlying unboundedness in `fsqlite`; it puts a wall
  clock around it, which is precisely what Fix A did one level up.

### A smaller alternative worth naming

If the parent judges the worker thread too much for a path that has never been
observed to stall, the *tri-state half alone* is a ~10-line change with no thread,
no constant, and no new failure mode, and it fixes the silent-miss defect in §2.
The bound is what fixes §1, and §1 is latent rather than live. They are separable
and can land in that order.

---

## 4. EXISTING TESTS

**No test exists that would catch either defect.** Enumerated, with the negative
searches shown so the null is checkable:

### 4a. What exists

| test | file:line | what it asserts | catches the defect? |
|---|---|---|---|
| `aborted_connector_scan_does_not_leave_the_index_claiming_complete_coverage` | `src/indexer/mod.rs:33444` | Two-pass fixture: an aborting codex connector leaves a durable floor (33528-33532), and a clean second pass recovers the 2 missed rollouts (33560-33565) and clears the floor (33566-33569). | **No.** It uses `read_connector_scan_floors_fresh` at 33527 and 33567 as an *instrument*, on a healthy temp DB whose open and query both succeed. It never exercises either failure arm and never bounds wall clock. |
| `connector_coverage_honesty_tests` (4 tests) | `src/lib.rs`, added by `8dcd245b` | `read_connector_scan_floors` and `read_connector_scan_floors_bounded` — the **lib.rs** copies. | **No.** Different functions. Reverting `src/indexer/mod.rs:10696` to today's code leaves all four green. |
| `connector_scan_floors_round_trip_and_clear` | `src/storage/sqlite.rs:23548` | Storage-layer round trip; pins monotone-downward floors at 23559-23567. | **No** — but it is the test that proves §2b Mechanism 2. |
| `connector_scan_since_ts_lowers_to_the_floor` | `src/storage/sqlite.rs:23585` | Pure logic on the `(run, floor)` formula. | No. |
| `parse_connector_scan_floors_tolerates_junk` | `src/storage/sqlite.rs:23599` | Malformed JSON parses as no floor. | No. |

### 4b. The negative searches, with a positive control

```
$ out=$(rg -n "read_connector_scan_floors_fresh|read_connector_scan_floors|ConnectorScanCoverage|connector_scan_since_ts" --glob '*.rs' tests/); echo "EXIT=$?"
EXIT=1

$ out=$(rg -n "connector_scan_floor|scan_floors" --glob '*.rs' tests/); echo "EXIT=$?"
EXIT=1

$ out=$(rg -n "connector_coverage" tests/ -g '!*.actual' -g '!*.json'); rc=$?; echo "rc=$rc"
rc=1
```

Positive control (the instrument is alive): `rg -c "fn " --glob '*.rs' tests/ | wc -l`
→ `218` files. So `rg` does read `tests/`, and the three `rc=1` results above are
true absences, not a dead probe. The only `tests/` hits for "connector_coverage" at
all are golden output files (`tests/golden/robot/*.json.actual`) — recorded output,
not assertions about this function.

`rg -n "could not open the archive|could not read connector scan coverage floors" src/`
returns three source lines (indexer/mod.rs:10706, 10715; lib.rs:15125) and zero test
lines, so neither warn message is asserted anywhere.

### 4c. What a test that would catch it looks like

The lib.rs mutant-killer `failed_coverage_read_is_unknown_and_never_complete`
transfers directly: open a temp DB with **no `meta` table**, so the query itself
fails, and assert `read_connector_scan_floors_fresh(&db_path) == None` — with the
positive control the lib.rs test already carries (a sibling DB that *does* have the
table must return `Some`, otherwise a `None` proves nothing). Restoring today's
`BTreeMap::new()` on the error arm turns it red with
`left: Some({}) right: None`, the same shape the commit message reports for lib.rs.

There is a second, stronger case available here that lib.rs cannot express, because
the indexer copy has real callers: build a floor, make the read fail, run a scan, and
assert the floor **still exists afterwards** — pinning §2b so that the recoverability
this lane relied on is a tested property rather than a coincidence of `has_floor`.

---

## 5. What this lane did NOT do

- Ran no `cargo` command of any kind. No `check`, no test, no build.
- Wrote no source file. Only this log.
- Took no measurement of `cass index` timing, and did not run `cass` other than
  `--version` on the already-deployed binary.
- Did not verify the `open_connections == 0` theory for the lib.rs >150s stall. It
  is the most plausible mechanism from source, and it is unmeasured.
- Did not chase the `runtime_id` half of `SharedMvccKey` (connection.rs:62261-62273).
  If two connections in one process were ever built on different `RuntimeContext`s
  they would not share a `SharedMvccState`, and the §1d mitigation would not hold.
  Unverified either way.
