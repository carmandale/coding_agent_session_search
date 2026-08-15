# LANE gen3-verify-third-copy — adversarial verification of gen3-third-copy

Read-only lane. No file written except this one. No `cargo` command of any kind,
no test run, no timing measured, no binary touched.

Posture: try to refute. Result below is **NOT REFUTED, with two material
corrections and one recommendation reversed.**

Everything below was re-read from source by me. Pinned dep versions confirmed
first: `Cargo.toml:45` `frankensqlite = { version = "0.1.5", package = "fsqlite" }`,
`Cargo.lock:2270-2273` fsqlite 0.1.5, `Cargo.lock:2313-2316` fsqlite-core 0.1.5.
All fsqlite line cites are from
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/fsqlite-core-0.1.5/`.

---

## Q1 — the unbounded close. CONFIRMED, and I found the piece that makes it
## stronger, plus the piece that makes it latent.

### The spin loop is real and I walked every hop

```
region.rs:481-483
    while self.active_tasks(id) > 0 || self.active_obligations(id) > 0 {
        std::hint::spin_loop();
    }
```

No deadline, no yield, no `Result` escape. `close_and_drain` is `region.rs:465-468`
→ `drain_subtree` at `region.rs:472`.

Chain, each hop re-read by me:

| hop | file:line | verbatim |
|---|---|---|
| 1 | `src/indexer/mod.rs:10719` | `let _ = storage.close();` |
| 2 | `src/storage/sqlite.rs:4190-4196` | `pub fn close(self)` → `this.conn.close()` |
| 3 | `fsqlite-0.1.5/src/lib.rs:6` | `pub use fsqlite_core::connection::{Connection, ...}` |
| 4 | `connection.rs:12660-12662` | `pub fn close(mut self) -> Result<()> { self.close_in_place() }` |
| 5 | `connection.rs:12676-12678` | `close_in_place` → `self.close_internal(false, true)` |
| 6 | `connection.rs:12810-12812` | `self._shared_mvcc_state.release_connection(self.runtime_region, false)?` |
| 7 | `connection.rs:62707-62718` | `let mut state = lock_unpoisoned(&self.runtime_state);` … `state.regions.close_and_drain(connection_region)` |
| 8 | `region.rs:481-483` | the spin |

Hop 4's line numbers are byte-exact as the other lane cited them.

`Drop` does not escape: `connection.rs:56743-56757` →
`let _ = self.close_internal(true, false);` → same `release_connection`.
Confirmed; "just drop it" is not a remedy.

### The piece the other lane missed, which SUPPORTS its own hypothesis

`close_internal`'s passive WAL checkpoint is gated at `connection.rs:12792-12797`:

```
if checkpoint_on_close
    && !self.pager.is_memory()
    && self.pager.journal_mode() == JournalMode::Wal
    && !self.pager.is_readonly()          // ← connection.rs:12795
```

A read-only open takes `compat/flags.rs:203` → `open_read_only_connection`
(`flags.rs:168-175`) → `Connection::open_schema_only` (`connection.rs:8635-8637`)
→ `open_schema_only_with_env` → `PagerBackend::open_readonly_with_page_buffer_max`.
So `is_readonly()` is true and **the checkpoint is skipped**.

That matters because it removes the only other expensive candidate. For a
read-only handle with no open transaction, every earlier branch of
`close_internal` is a no-op, so `release_connection` → `close_and_drain` is
essentially the *whole* of close. The repo's own committed doc comment
(`src/lib.rs:15168-15172`) records `close_franken_cli_read_db` — which is
`conn.close_in_place()` at `src/lib.rs:14229` — as "on a 7.7 GB archive **is the
expensive step**", measuring >150s against a declared 2s ceiling. That close is
also read-only (`open_franken_cli_read_db` at `lib.rs:14084` →
`open_franken_readonly_storage_with_timeout`), so the >150s cannot be a
checkpoint. The drain is the residual explanation. I did not measure it.

### Worse than stated: the spin holds a process-global mutex

`connection.rs:62708` takes `lock_unpoisoned(&self.runtime_state)` and the guard
is **still alive** at `62718` when `close_and_drain` spins. Every
`register_connection` takes the same mutex (`connection.rs:62436`). So a thread
spinning in that drain blocks every subsequent open *and* close on that path in
this process. The other lane did not say this; it is the hinge of my Q4 answer.

### Why it is latent in the indexer — I closed the gap the other lane left open

The root-region drain (`connection.rs:62744-62757`), the expensive one, is gated
on `state.open_connections == 0` (`connection.rs:62743`). The other lane argued
the count never reaches 0 and flagged the `runtime_id` half of `SharedMvccKey` as
**unchased**. I chased it. Three parts, all verified:

1. **Path.** `mvcc_state_path_key` (`connection.rs:62819-62826`) canonicalizes.
   The index pipeline's long-lived `storage` is opened from `&opts.db_path`
   (`src/indexer/mod.rs:12217-12219`, `12275-12277`,
   `open_franken_storage_with_timeout(&opts.db_path, …)`) — the same value passed
   to `read_connector_scan_floors_fresh(&opts.db_path)` at `11568` and `11737`.
2. **Runtime.** `SharedMvccKey::new` (`connection.rs:62266-62272`) mixes
   `runtime.runtime_id()`. `ConnectionEnv::default()` (`connection.rs:2077-2084`)
   is `runtime: RuntimeContext::global()`, and `global_runtime_context`
   (`connection.rs:1956-1960`) is a `OnceLock` singleton.
   `rg -n "init_global_runtime|RuntimeConfig|ConnectionEnv" src/` in cass returns
   **no production hit** — cass never installs a custom runtime and never builds a
   `ConnectionEnv`. So every connection in the process shares one `runtime_id`.
   **The unchased gap is closed and the mitigation holds.**
3. **Count.** Both call sites take `storage: &FrankenStorage` in their signatures
   (`indexer/mod.rs:11513`, `11699` — both verified byte-exact). So
   `open_connections` goes 2→1, not 1→0, and the root drain is skipped. Only the
   ephemeral connection's own `PerConnection` region drains, and it ran one small
   `SELECT`.

**Verdict Q1: structurally real, not a live fire in the indexer, and the
mitigation is now fully verified rather than asserted.** The other lane's
"do not report it as a live fire" was correct.

### Two overstatements in Q1

- "Bounds on the whole path: **exactly one**, 30s, and only on the doctor lock."
  Not quite. The fsqlite open path also carries a busy-retry deadline derived
  from the default `busy_timeout` pragma — `retry_busy_connection_bootstrap`,
  `connection.rs:335-361`, `let deadline = Duration::from_millis(busy_timeout_ms)`.
  It only covers `Busy`/`BusyRecovery`, so the conclusion stands, but the open is
  not literally unbounded everywhere.
- Citation drift, immaterial: `open_franken_readonly_storage_with_timeout` is
  `sqlite.rs:636-663` with the `open_readonly` call at `647` (cited 637-665 / 648).
  `record_connector_scan_floor` is `sqlite.rs:7025-7036` (cited 7026-7037).
  `FrankenStorage::open_readonly` at `sqlite.rs:4172-4174`, the doctor const at
  `sqlite.rs:42`, the guard deadline at `sqlite.rs:515`, and
  `PRAGMA busy_timeout = 5000` at `sqlite.rs:4304` are all exact as cited.

### A live concern about the ALREADY-COMMITTED lib.rs fix, which nobody flagged

`read_connector_scan_floors_bounded(&db_path, HEALTH_COVERAGE_OPEN_TIMEOUT)` uses
**2s** (`lib.rs:15134`). Its worker calls `open_franken_cli_read_db(path, …, 2s)`
→ `open_franken_readonly_storage_with_timeout(&path, 2s)` → and inside its retry
loop calls plain `FrankenStorage::open_readonly(path)` (`sqlite.rs:647`), which
hardcodes the **30s** `DOCTOR_MUTATION_DB_OPEN_LOCK_TIMEOUT` (`sqlite.rs:4172-4174`).
The 2s never reaches the doctor lock. So whenever `cass doctor --fix` holds the
lock, the 2s `recv_timeout` fires and orphans a worker that then waits up to 30s
on a *file* lock. For a short-lived CLI that is the accepted ceiling and is
benign — a file lock, not the runtime mutex — and the surface correctly reports
unchecked. Recording it because it independently justifies the other lane's
"outer bound must exceed 30s" warning.

---

## Q2 — CONFIRMED, exactly. Not cosmetic. It narrows.

`connector_scan_since_ts`, `src/storage/sqlite.rs:81-88`, verbatim:

```rust
match (run_since_ts, floor) {
    (None, _) => None,
    (Some(_), Some(floor)) if floor <= 0 => None,
    (Some(run), Some(floor)) => Some(run.min(floor)),
    (Some(run), None) => Some(run),
}
```

A floor can only **lower** `since_ts` (`run.min(floor)`) or open the window fully
(`floor <= 0 => None`). An absent floor yields the unwidened watermark. So an
empty map is strictly narrower-or-equal, never wider. Pinned by a passing test at
`sqlite.rs:23585-23596`. **The "absent floor causes a wider scan" refutation I was
sent to look for does not exist.**

Recoverability confirmed, both mechanisms:

1. `has_floor` (`indexer/mod.rs:10681-10683`) is `contains_key`, false on an empty
   map, and both clear sites are gated on it — `indexer/mod.rs:11359` (streaming)
   and `indexer/mod.rs:11890` (batch). The `meta` row survives a failed read.
2. `record_connector_scan_floor` (`sqlite.rs:7025-7036`) re-reads through the
   ephemeral **writer** connection — `with_ephemeral_writer` passes `writer` into
   the closure (`indexer/mod.rs:10731-10736`), so the read is fresh, not the stale
   empty map — and refuses to raise:
   `if floors.get(connector).is_some_and(|existing| *existing <= floor_ts) { return Ok(()) }`.
   Pinned at `sqlite.rs:23558-23568`, *"the earliest unproven point wins."*

The un-gated record at `indexer/mod.rs:11888-11890` (`if *scan_failed { record… }`)
is the one path that is *not* `has_floor`-gated, and I checked it: it passes
`failure_floor_for` = the unwidened (higher) watermark, and mechanism 2 refuses it
against the surviving lower floor. Safe.

So: the miss is real, repeated, silent but for one `warn!`, and recoverable.
`cass stats` still reports INCOMPLETE because it reads floors through the lib.rs
path (`lib.rs:23970`, `lib.rs:65095`), not this one. All as the other lane said.

---

## Q3 — the remedy. **One half is right, one half should be dropped, and there
## is a smaller bound the other lane did not consider.**

### Correction A: "the tri-state half alone … fixes the silent miss" is WRONG.

The other lane also says do not widen and do not fail the run. Given that, an
`Option<BTreeMap<…>>` return changes **nothing at runtime**: the caller still hands
`ConnectorScanCoverage::new` an empty map and the scan narrows identically. I
checked every consumer of `coverage.floors` — `has_floor` (10681), the info log at
`indexer/mod.rs:11570-11575`, and `ConnectorScanCoverage::new` (10648-10664). There
is no fourth.

What tri-state actually buys, and it is worth having:

- The log level and message become honest (`warn!` → `error!`, naming the
  consequence). Today's two `warn!` arms are `indexer/mod.rs:10702-10709` and
  `10712-10716`.
- The type stops a future `.unwrap_or_default()` — the exact regression bead
  `1a7mk` was filed for, per `lib.rs:15096-15106`.
- Three copies of one read finally agree: `sqlite.rs:6991-7002` already uses
  `.optional()` and returns `Result`; `lib.rs:15107-15129` already returns
  `Option`; this is the only one that collapses.
- It kills a vacuous assertion — see Q4.

Say that plainly in the commit. Anyone reading "tri-state fixes the silent miss"
will believe the hole is closed. It is not; it is by design left open.

### Correction B: do NOT put the bounded worker in the indexer.

Cost/benefit is inverted here versus lib.rs:

- **Benefit ≈ 0.** The expensive root drain provably cannot fire at these two call
  sites (`open_connections` ≥ 1, verified above). The one concrete trigger the
  other lane itself names — `doctor --fix` holding the lock — is *already* bounded
  at 30s by `acquire_doctor_mutation_db_open_guard` (`sqlite.rs:494-515`) and
  already lands in the existing `Err` arm.
- **Cost is worse than the other lane stated.** It called the orphan "a busy core
  for the rest of the drain." It is more than that. An orphaned worker inside
  `close_and_drain` holds `runtime_state` (`connection.rs:62708`), and
  `register_connection` needs that same mutex (`connection.rs:62436`).
  `with_ephemeral_writer` → `acquire_cached_ephemeral_writer`
  (`indexer/mod.rs:22975`) opens connections on that path. So the run does not
  merely lose a core — **the next ephemeral-writer acquisition blocks forever,
  with no timeout and no log, at a site far from the cause.** The bound converts a
  visible hang at a named call into an invisible wedge somewhere else. Acceptable
  ceiling in a short-lived CLI (lib.rs's existing use); not in a long-lived
  `--watch` indexer, which is precisely the difference the other lane identified
  and then recommended past.

`right-sized-mechanism.md` applies: the mechanism (thread + channel + constant +
ceiling comment + a new deadlock class) is larger than the problem (a latent spin
that the same lane proved cannot fire here).

### The smaller bound the other lane did not consider

If a bound is wanted at all, bound the one blocking step that is actually
reachable, with no thread and no orphan. `sqlite.rs:4180` already exposes:

```rust
pub fn open_readonly_with_doctor_lock_timeout(path: &Path, timeout: Duration) -> Result<Self>
```

Swapping `FrankenStorage::open_readonly(db_path)` at `indexer/mod.rs:10700` for
that, with a short timeout, bounds the doctor-lock wait — the only non-hypothetical
trigger anyone has named — and the existing `Err` arm at `10701-10710` already
handles the result. That is a one-line change with zero new failure modes, and it
is what `right-sized-mechanism.md` rung 4 asks for: use the facility already
installed.

---

## Q4 — new failure modes in the recommended remedy, named concretely

1. **`runtime_state` deadlock in a long-lived process** (the big one, above). Not
   hypothetical about the mechanism: the mutex is taken at `connection.rs:62708`
   and held across the spin at `region.rs:481-483`; the blocked acquirer is
   `connection.rs:62436`. I did not execute it.
2. **Thread spawn per index run** — trivial, and the other lane said so.
3. **Not a forced full re-scan.** I checked for this specifically because the
   prompt asked. `read_connector_scan_floors_fresh` returning `None` → caller
   substitutes an empty map → `connector_scan_since_ts(Some(run), None) => Some(run)`
   (`sqlite.rs:86`). The 7.9 GB full-rescan path is `floor <= 0 => None`
   (`sqlite.rs:84`), which requires a floor that is **present and ≤ 0**. A failed
   read cannot produce it. **No full-rescan risk in either variant of the remedy.**
4. **No happy-path behavior change** from tri-state, per Q3 Correction A.

## Q4 — test coverage. CONFIRMED, and there is a vacuous assertion nobody named.

`src/indexer/mod.rs:33444`
`aborted_connector_scan_does_not_leave_the_index_claiming_complete_coverage` uses
the function as an **instrument** at `33527` and `33567`, on a healthy `TempDir`
DB where open and query both succeed. Neither failure arm is exercised; no wall
clock is bounded.

My own `tests/` searches, run by me:

```
rg -l "read_connector_scan_floors_fresh|connector_scan_since_ts|scan_floors" tests/   → rc=1
rg -l "connector_coverage" tests/                                                     → rc=1
rg -c "fn " --glob '*.rs' tests/ | wc -l                                              → 218
```

Positive control: 218 files in `tests/` match `fn `, so `rg` reads that directory
and the two `rc=1` results are true absences, not a dead instrument. With
`--no-ignore`, `connector_coverage` appears in exactly 7 files, all
`tests/golden/robot/*.json.actual` recorded output — matching the other lane.

**The addition:** `indexer/mod.rs:33565-33569` asserts
`read_connector_scan_floors_fresh(&db_path).is_empty()` with the message *"a clean
scan that read from the floor should clear it."* Today that assertion passes
identically whether the floor was cleared **or the read failed** — both return an
empty map. It is a vacuous guard in the `no-vacuous-test-guards.md` sense, and it
sits inside the one test that exists for this feature. The tri-state change turns
it into `Some(empty)` vs `None` and kills the vacuity. That is the strongest
concrete argument for the tri-state half, and the other lane did not make it.

The transferable test shape the other lane named is real: `src/lib.rs:15686-15691`
states the mutant explicitly (*"restoring `.unwrap_or_default()` … turns it red"*),
with the meta-table positive control at `lib.rs:15743` and the expiry case at
`lib.rs:15795-15815`.

---

## What I did not do

No `cargo check`, no `cargo test`, no build, no timing, no sqlite read, no
process inspection. Every claim above is a source read or an `rg` I ran in this
lane. The >150s figure is quoted from a committed doc comment
(`src/lib.rs:15168-15172`), not measured by me. The `runtime_state` deadlock is
derived from source and is **not executed**.
