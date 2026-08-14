# Lane: test-integrity

Read-only grounding lane for the cass repair-to-green work. Assigned log; this
lane is its only writer. Every claim below is labelled **MEASURED** (I ran the
command shown and read its output) or **INFERRED** (reasoning over source I
read). No cargo was run. No database, data dir, or installed binary was touched.

Repo: `/Users/dalecarman/dev/coding_agent_session_search`
HEAD at start of lane: `37d52925` (`beads(nvq59): the status --json hang is a 20 GB raw-mirror walk`)
Date: 2026-08-14

---

## 1. Bead gxw32, in full

**MEASURED** — `perl -e 'alarm shift; exec @ARGV' 60 br show gxw32`

Title: *"Nothing tests that the coverage floor is per-connector — a mutant
restoring the global watermark passes all 5,127 tests"*. P2, OPEN, owner
dalecarman, created and updated 2026-08-11.

The exact mutant it describes, quoted verbatim from the bead body:

```
Mutant M2 turns the per-connector lookup into the literal relabelled global, at
ConnectorScanCoverage::new:

  -  let since = connector_scan_since_ts(run_since_ts, floors.get(name).copied());
  +  let since = connector_scan_since_ts(run_since_ts, floors.values().copied().min());

  $ git diff --stat   -> src/indexer/mod.rs | 6 +++++-   (one file, one site)
  $ cargo test --lib  -> 5124 passed; 0 failed; 3 ignored
  RC=0

Byte-identical to the clean baseline. Not one of 5,127 tests noticed.
```

The exact fixture limitation it names, verbatim:

```
The reason is the fixture: src/indexer/mod.rs:33492 and :33555 both register
`vec![("codex", mtime_filtered_aborting_connector_factory)]` — a single connector.
With one connector, per-connector and global are indistinguishable by construction,
so the property the fix is named after has zero coverage.
```

And its own recommended fix, verbatim:

```
Fix: register at least two connectors in the coverage fixture with different floors,
and assert each receives its own. That is the assertion M2 would fail.
```

Bead also records that the two *prior* mutants (the implementing session's, and
challenger A's M1) kill the floor for **every** connector at once, so they prove
"a floor lowers since_ts" and cannot distinguish *which* connector's floor is
used. That is an accurate reading of the code (see §3).

---

## 2. What e3ed01f0 actually shipped, and what it tested

**MEASURED** — `git show e3ed01f0 --numstat --format=''`

```
523	19	src/indexer/mod.rs
237	2	src/lib.rs
172	1	src/storage/sqlite.rs
```

**MEASURED** — tests added by the commit
(`git show e3ed01f0 -U3 | rg -B1 -A3 '^\+\s*#\[test\]'`): exactly **four**.

| test | file | kind |
|---|---|---|
| `aborted_connector_scan_does_not_leave_the_index_claiming_complete_coverage` | `src/indexer/mod.rs:33444` (`#[serial]`) | end-to-end through the streaming indexer |
| `connector_scan_floors_round_trip_and_clear` | `src/storage/sqlite.rs:23548` | pure storage round-trip |
| `connector_scan_since_ts_lowers_to_the_floor` | `src/storage/sqlite.rs:23585` | pure function, 5 cases |
| `parse_connector_scan_floors_tolerates_junk` | `src/storage/sqlite.rs:23599` | pure parser |

**MEASURED** — `git show e3ed01f0 -- src/lib.rs | rg '#\[test\]|mod tests'` returns
**nothing**. The 237 lines added to `src/lib.rs` — `read_connector_scan_floors`,
`read_connector_scan_floors_bounded`, `connector_coverage_json`,
`connector_coverage_state_json`, `connector_coverage_floors_from_state`,
`connector_coverage_recommended_action`, `connector_coverage_warning`, and the
wiring into `stats --json` / `status` / `health` — shipped with **zero tests of
their own**.

That is the same file and the same block that carries the deployment blocker.
`read_connector_scan_floors_bounded` (`src/lib.rs:15099-15108`) is the function
bead 1a7mk pins as the hang:

```rust
fn read_connector_scan_floors_bounded(
    db_path: &Path,
    timeout: Duration,
) -> Option<BTreeMap<String, i64>> {
    let conn =
        open_franken_cli_read_db(db_path.to_path_buf(), "connector-coverage", timeout).ok()?;
    let floors = read_connector_scan_floors(&conn);
    let _ = close_franken_cli_read_db(conn, db_path, "connector-coverage");
    Some(floors)
}
```

**MEASURED** — the `timeout` reaches `open_franken_cli_read_db` only; the read
and the close on the next two lines are unbounded. Unchanged at HEAD, exactly as
the coordinator's established facts state. **INFERRED** — a test that would have
caught it (assert the whole call returns inside its budget against a contended
or slow archive) does not exist, and could not have been written by accident,
because the commit added no `src/lib.rs` tests at all.

**This is the more consequential half of the gxw32 shape.** gxw32 is about a
property with no test. The lib.rs block is 237 lines of new *agent-facing output
surface plus the hang* with no test.

---

## 3. Why one connector makes M2 invisible — confirmed from source, not assumed

The mutated site, **MEASURED** at `src/indexer/mod.rs:10648-10664`:

```rust
impl ConnectorScanCoverage {
    fn new(
        run_since_ts: Option<i64>,
        floors: BTreeMap<String, i64>,
        connector_names: impl IntoIterator<Item = &'static str>,
    ) -> Self {
        let since_ts_by_connector = connector_names
            .into_iter()
            .map(|name| {
                let since = connector_scan_since_ts(run_since_ts, floors.get(name).copied());
                (name, since)
            })
            .collect();
```

Line 10657 is the mutation site (**MEASURED**: `rg -n 'let since = connector_scan_since_ts' src/indexer/mod.rs` → `10657`).

The proof of indistinguishability is by construction, and it holds for **any**
`run_since_ts`:

- `floors` is a `BTreeMap<String, i64>` read from the archive; the fixture can
  only ever put `"codex"` in it, because `"codex"` is the only connector that
  runs and therefore the only one that can fail and record a floor.
- Clean: `floors.get("codex")` → `Some(F)` when a floor exists, `None` otherwise.
- M2: `floors.values().copied().min()` over a map with at most one entry →
  `Some(F)` when that entry exists, `None` otherwise.
- The two expressions are equal for every input the fixture can produce, so
  `connector_scan_since_ts` receives an identical argument in both builds and
  the whole downstream chain (`since_ts_for`, `failure_floor_for`, the producer
  `since_ts`, the recorded floor) is byte-identical.

**MEASURED** — the fixture's two registration sites, `src/indexer/mod.rs:33492`
(pass 1) and `:33555` (pass 2), are both inside the *same* test function; they
are not two tests. Both read:

```rust
vec![("codex", mtime_filtered_aborting_connector_factory)],
```

**MEASURED** — the whole repo's connector-factory injection surface, from
`rg -n 'vec!\[\("' src/indexer/mod.rs tests/`. There is **no** `register_connector`
API (the bead's word "register" is descriptive; the mechanism is a
`Vec<(&'static str, ConnectorFactory)>` argument). Five injection sites exist,
all in `src/indexer/mod.rs`, all one connector:

| line | connector list | path |
|---|---|---|
| 27314 | `vec![("codex", failing_explicit_file_root_connector_factory)]` | batch |
| 33492 | `vec![("codex", mtime_filtered_aborting_connector_factory)]` | streaming (coverage test, pass 1) |
| 33555 | `vec![("codex", mtime_filtered_aborting_connector_factory)]` | streaming (coverage test, pass 2) |
| 35768 | `vec![("claude", panic_connector_factory)]` | streaming |
| 35823 | `vec![("codex", deferred_batch_connector_factory)]` | batch |

**5 of 5 are one-connector worlds. There is no multi-connector test anywhere in
the crate.**

**MEASURED** — `ConnectorScanCoverage::new` is called at exactly two production
sites (`src/indexer/mod.rs:11566` streaming, `:11735` batch) and at **zero** test
sites. Every test reference to the type is
`&ConnectorScanCoverage::default()` — an *empty* floors map — at lines 34508,
34557, 34625, 34682, 35108, 35347, 35452, 35531, 35578, 35716. Ten call sites
that construct the type with nothing in it.

**MEASURED** — what production actually looks like, from
`get_connector_factories()` in the pinned sibling crate
(`~/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/b62d859/src/connectors/mod.rs:204`):
15 unconditional connectors (`codex, cline, gemini, claude, clawdbot, vibe, amp,
aider, pi_agent, factory, kimi, openclaw, copilot, copilot_cli, qwen`) plus
feature-gated `opencode, chatgpt, cursor, crush, hermes` — all five of those
features are enabled in `Cargo.toml:94`. **20 connectors in production.**

So the property is exercised over **1 of 20**, and the one is the degenerate
case where the property is unobservable.

**One nuance the bead does not state, worth having:** M2 fails *conservatively*.
With two connectors it hands every connector the lowest floor in the map, so a
healthy connector over-scans rather than under-scans. A test that only counts
conversations may therefore still pass under M2 (the counts come out the same or
higher). **The assertion has to be on the `since_ts` itself**, not on a
downstream count. That is why the design in §4 asserts `since_ts_for` directly.

---

## 4. The smallest test change that makes M2 fail

**Recommendation: a unit test on `ConnectorScanCoverage::new`, not a second
connector in the integration fixture.**

The bead recommends extending the fixture. That works, but it is the expensive
rung: a second connector needs a second connector type, a second fixture root, a
second mtime-controlled corpus, and it inherits the existing test's `#[serial]`
+ TempDir + Tantivy + FrankenStorage cost. The mutated line is a pure mapping
function, and a pure test kills the mutant on two assertions in microseconds.
Per the minimalism ladder, take that rung first.

**Exact file:** `src/indexer/mod.rs`
**Exact placement:** inside `mod tests` (opens at line 26352, `use super::*;` at
26353 — so `BTreeMap` from line 18 and the private `ConnectorScanCoverage` are
both already in scope). Insert immediately after line 33574, which closes
`aborted_connector_scan_does_not_leave_the_index_claiming_complete_coverage`,
and before line 33576 (`struct PanicConnector;`).

**Exact assertions:**

```rust
/// The coverage floor is per-connector. A regression that collapses it back to
/// one shared value — the global `last_scan_ts` watermark this fix exists to
/// replace — is invisible in a one-connector world, so this test is the only
/// place the property is actually observable.
///
/// Bead `coding_agent_session_search-gxw32`.
#[test]
fn coverage_floor_is_per_connector_not_one_shared_value() {
    let floors = BTreeMap::from([
        ("codex".to_string(), 100_i64),
        ("claude".to_string(), 400_i64),
    ]);
    let coverage =
        ConnectorScanCoverage::new(Some(500), floors, ["codex", "claude", "cursor"]);

    // Each connector gets its OWN floor.
    assert_eq!(
        coverage.since_ts_for("codex"),
        Some(100),
        "codex must scan from its own floor"
    );
    assert_eq!(
        coverage.since_ts_for("claude"),
        Some(400),
        "claude must scan from its own floor, not codex's lower one"
    );
    // A connector with no floor is not widened by another connector's failure.
    assert_eq!(
        coverage.since_ts_for("cursor"),
        Some(500),
        "a connector that never failed must scan from the run watermark"
    );

    // The floor a failure records is that connector's own scan position.
    assert_eq!(coverage.failure_floor_for("claude"), 400);
    assert!(coverage.has_floor("codex"));
    assert!(!coverage.has_floor("cursor"));
}
```

**Why this kills M2** (**INFERRED** from the source in §3, verifiable by the
coordinator with one `cargo test --lib coverage_floor_is_per_connector`):
under `floors.values().copied().min()` the whole map collapses to `Some(100)`,
so `since_ts_for("claude")` returns `Some(100)` against an expected `Some(400)`
and `since_ts_for("cursor")` returns `Some(100)` against an expected `Some(500)`.
Two independent assertion failures, plus `failure_floor_for("claude")` → 100.
The mutant cannot survive.

**Verify the test the honest way**: run the mutant, confirm this case goes red
*and read the failure text* (not just the count) to confirm it failed on the
floor values and not on something adjacent. Then revert the mutant and confirm
green.

**Optional second step, only if the coordinator wants the behavioural property
too** (it is genuinely more expensive and it is not required to close gxw32):
add a second, always-healthy connector to the existing fixture at
`src/indexer/mod.rs:33492`/`:33555` and assert that after pass 1
`read_connector_scan_floors_fresh(&db_path)` contains `"codex"` and **not** the
healthy connector's name. That pins the *recording* scope, which the unit test
does not reach. It does not, on its own, kill M2 — M2 does not change what gets
recorded, only which floor is read — so it is a complement, not a substitute.

---

## 5. Survey — the same shape elsewhere

**MEASURED** — the lane brief's suggested command is written for a
`register_connector` API that does not exist here; `rg -n 'register_connector' src/ tests/`
returns nothing. I replaced it with a search on the real mechanism
(`vec![(name, factory)]` passed to the two `*_with_connector_factories` entry
points), which is exhaustive because those are the only two seams.

Result, already tabled in §3: **5 of 5 connector-injection tests build a
one-connector world, and all 5 live in one file.** No test in `tests/` injects
factories at all — integration tests spawn the real `cass` binary against a temp
`HOME`, which exercises the real 20-connector registry but says nothing about
per-connector floors, because none of those connectors is made to fail.

Adjacent finding, same shape, **MEASURED**: the 10 `ConnectorScanCoverage::default()`
test call sites all pass an *empty* floors map. So across the entire suite the
`floors` map is either empty (10 sites) or has at most one key (1 test). The
map's multi-key behaviour is exercised only in
`src/storage/sqlite.rs:23548 connector_scan_floors_round_trip_and_clear`, which
does put two connectors in it (`"codex"`, `"pi_agent"`) — but that test covers
the **store**, not the per-connector **selection** at line 10657. The one place
two connectors coexist is the one place the mutation does not live.

---

## 6. The real baseline, CI, and what is safe to run here

### Test counts

**MEASURED** (`rg -c '#\[test\]' --glob '<dir>/**/*.rs' | awk -F: '{s+=$NF} END {print s}'`
— note `-F:` on the *last* field; my first attempt summed a path fragment and
printed a false 0, which is why the numbers below are the second measurement):

| | `#[test]` | `#[ignore]` |
|---|---|---|
| `src/**` (the lib suite) | **5,130** | 3 |
| `tests/**` (integration + e2e) | **2,750** | 59 |

No `#[tokio::test]` in either tree (**MEASURED**, both counts empty).
**MEASURED**: 209 files match `tests/*.rs`, plus three declared `[[test]]`
targets in `Cargo.toml` (`docs`, `upgrade`, `recovery`).

**So yes — the bead's 5,124 is the lib suite alone.** 5,124 passed + 3 ignored =
5,127, which matches the 5,130 attributes counted today minus 3 ignored, with
the small drift explained by commits landed since 2026-08-11. **The integration
and e2e suites add roughly 2,750 more test cases across 209 binaries, and the
mutant was never run against any of them.** The bead's headline claim is about
the lib suite only; it is not a claim about the whole repo. It does not need to
be — §3 shows the mutated line has no multi-connector observer anywhere.

**MEASURED** — 160 `#[serial]` attributes in `src/**`, so a meaningful slice of
the lib suite is serialised and the suite is not embarrassingly parallel.

### What actually gates a merge

**MEASURED** — `.github/workflows/ci.yml`, triggers `push: [main]`,
`pull_request: [main]`, `workflow_dispatch`. Eleven jobs: `no-mock-audit`,
`lint`, `ubs-changed-files`, `test-rust`, `ssh-sync-docker`, `e2e-orchestrated`,
`e2e-tui-matrix`, `crypto-vectors`, `security`, `build`, `e2e-log-summary`.

`test-rust` is a 3-OS matrix (`ubuntu-latest`, `macos-latest`, `windows-latest`)
with `timeout-minutes: 45`, and it runs, in order:

```
cargo test --features "qr encryption backtrace" --verbose -- --nocapture
cargo test --doc
E2E_LOG=1 cargo test --features "qr encryption backtrace" --verbose --test e2e_<each> ... -- --nocapture
```

**MEASURED** — the first line is a bare `cargo test`, so it already runs the lib
suite *and* every integration binary *including* every `e2e_*`; the third line
then re-runs the 22 `e2e_*` targets a second time with `E2E_LOG=1`. So the
merge gate is lib + integration + e2e + doc tests, twice over for e2e, and the
whole thing fits inside 45 minutes on a hosted runner.

`lint` runs `cargo fmt --all -- --check` and
`cargo clippy --all-targets --features "qr encryption backtrace" -- -D warnings`.
`ubs-changed-files` runs the base-vs-current UBS comparison described in
`AGENTS.md:241-259`; the pinned version is in `.github/workflows/ubs-version.txt`.

I could not check GitHub branch protection from this lane (no `gh` call made),
so **whether these jobs are *required* checks is UNVERIFIED**; what is measured
is only that they run on every PR and push to main.

### Is the e2e suite safe to run on this machine?

**Short answer: do not run bare `cargo test`, and do not run the `e2e_*` targets
here. Run the lib suite.** But the reason is narrower than the repo napkin
states, and the coordinator should have the correction.

**MEASURED** — repo `napkin.md:39-43`, marked `emerging`:

> The e2e integration suite is not hermetic on the read side: it spawns 8
> concurrent `cass index --full` that scan the operator's real `~/.codex` and
> `~/.claude` trees, isolating only the output via `--data-dir`. On this machine
> that is ~9,800 codex files each and it had not finished after 90 minutes.
> Evidence: `ps` showed 8 children of `e2e_cli_flows` at ~60% CPU each with
> `--data-dir /var/folders/...`, elapsed 01:27, 2026-08-10.

**I could not reproduce that mechanism from source at HEAD, and I looked hard.**
What I measured instead:

- All 6 `"index"` invocations in `tests/e2e_cli_flows.rs` set **both**
  `.env("HOME", home)` and `.env("CODEX_HOME", &codex_home)` (lines 327, 329,
  337, 1195, 1224; line 1520 is a JSON key assertion, not a spawn).
- `CODEX_HOME` is genuinely honoured — it is read in the pinned sibling crate at
  `franken_agent_detection/src/connectors/codex.rs:63`, and every other connector
  resolves through `dirs::home_dir()`, which reads `$HOME` on macOS. Setting
  `HOME` on the child isolates all 20.
- 132 `index --full` invocation sites exist across `tests/`. My first pass
  flagged 36 as lacking `HOME` within 15 lines; on inspection every one of those
  routes through a per-file helper that sets it — `base_cmd(temp_home)`
  (`tests/cli_index.rs:35`, `tests/tui_integration_smoke.rs:36`),
  `isolated_cass_cmd(temp_home)` (`tests/cli_robot.rs:7256`, which also sets
  `XDG_*` and `CODEX_HOME`), `cass_cmd(temp_home)`
  (`tests/e2e_lexical_fail_open.rs:34`) — or is `#[ignore]`d
  (`tests/e2e_large_dataset.rs`: 6 tests, 6 ignores), or is a string literal in a
  fixture ledger rather than a spawn (`tests/perf_evidence_replay.rs:202`).
- 49 integration files spawn a binary; 19 never set `HOME`. I checked all 19 for
  an actual `cass index` invocation and **none has one** — the `"index"` hits in
  `tests/e2e_sources.rs`, `tests/docs/help.rs`, `tests/util/*` are all assertion
  strings or path components.
- I raised and then **refuted** a destructive-hazard hypothesis: `tests/e2e_sources.rs`
  runs `cass sources remove laptop --purge -y` at lines 1283 and 1383 with
  `.env_remove("CASS_DATA_DIR")` and no `HOME`, which looked like it would purge
  Dale's real 29 GB data dir. It does not. `default_data_dir()`
  (`src/lib.rs:80127-80144`) checks `CASS_DATA_DIR`, then **`XDG_DATA_HOME`**,
  and only then `directories::ProjectDirs`. Both of those tests set
  `XDG_DATA_HOME` to a temp dir, so the purge is contained. Recording the
  refutation because the alarm would have been expensive and wrong.

So: **the napkin's mechanism is UNCONFIRMED at HEAD** (labelled `emerging`
there, correctly). **The napkin's observation stands** — 8 children of
`e2e_cli_flows` at ~60% CPU with 01:27 elapsed is a measured fact I have no
basis to overturn from source, and I am forbidden from running the suite to
settle it. Something in that binary consumed 87+ minutes on a corpus that should
have been two fixture files.

**Two real hazards I can name from source, either of which the coordinator
should treat as sufficient reason not to run e2e here:**

1. **`EnvGuard` mutates the test process's global environment.**
   `tests/util/mod.rs:459-483`: `EnvGuard::set` calls
   `unsafe { std::env::set_var(key, val) }` and restores on `Drop`. It is used
   for `HOME` at 10 sites in `tests/cli_index.rs` and 5 in
   `tests/e2e_large_dataset.rs`. Under the default multi-threaded harness this is
   process-global state with a Drop-ordered restore. Those two files happen to
   also set `HOME` explicitly per child, so the guard is belt-and-braces there
   *today* — but any future spawn in those binaries that forgets the explicit
   `.env("HOME", …)` inherits whatever the racing guard last wrote, which is
   sometimes the real `$HOME`. This is a live footgun, not a current failure.
2. **The known-hanging surfaces are in the blast radius.** The installed binary
   already never returns for `cass status --json` and `cass doctor` on the real
   data dir (bead nvq59), and `read_connector_scan_floors_bounded` never returns
   for `health`/`triage`/`stats` on the real archive once the fix is deployed
   (bead 1a7mk). `tests/cli_doctor.rs`, `tests/e2e_health.rs`,
   `tests/doctor_e2e_runner.rs`, and `tests/cli_status.rs` all exercise exactly
   those surfaces. They are isolated by `HOME`/`XDG` today, but this is the
   family of tests most likely to wedge if any isolation is missing, and a wedged
   `cargo test` on this machine is expensive to diagnose.

### Recommended run plan for the coordinator

- **Safe, and the one that matters for gxw32:** `cargo test --lib`. One binary,
  no spawned `cass` processes (`rg 'cargo_bin!|CARGO_BIN_EXE' src/` finds none),
  TempDir-scoped, factory-injected. This is where all 5,130 lib tests live and
  where the new per-connector test belongs.
- **Safe and cheap while iterating:** `cargo test --lib coverage_floor_is_per_connector`
  and `cargo test --lib aborted_connector_scan`. The second is recorded as
  taking 1.48s (**MEASURED** by a prior session, quoted at
  `thoughts/shared/handoffs/20260810-codex-coverage-gap-2bh4a-fresh-agent-prompt.md:94-96`).
- **Probably safe, targeted, and worth it if lib.rs is touched:**
  `cargo test --test cli_robot`, `--test cli_status`, `--test cli_doctor` — the
  agent-facing JSON surfaces this repair changes. `cli_robot` uses
  `isolated_cass_cmd` (HOME + XDG + CODEX_HOME). Run them one target at a time
  under a wall-clock bound so a wedge is visible immediately, not one target
  into a 209-binary sweep.
- **Do not run here:** bare `cargo test`, `cargo test --all-targets`, or any
  `--test e2e_*`. Let CI run those — it is the environment where `~/.codex` and
  `~/.claude` are empty, which is precisely why the non-hermeticity the napkin
  observed is invisible on a runner and expensive on this Mac.
- **Use an explicit, checkout-private `CARGO_TARGET_DIR`.** Repo `napkin.md:22`
  records a **MEASURED** incident: a shared target dir between two checkouts of
  this crate silently ran the *wrong* binary, because both trees resolve to the
  same artifact name and cargo's freshness check then prints `Finished in 0.41s`
  and re-runs the other tree's test binary. A "full lib suite" result was the
  pre-change clone's. If the coordinator compares pre-fix and post-fix
  behaviour — which this repair requires — this is the failure that silently
  invalidates the comparison. Confirm `Compiling coding-agent-search (<the path
  you mean>)` before believing any cross-tree result. **MEASURED**: `target/`
  exists in the checkout; `/tmp/cass-test-target` (the path `AGENTS.md:363`
  suggests) does not, and `df -h /` shows 150 GiB available.
- **Capture exit codes directly, never through a pipe.** Repo `napkin.md:24`
  records `cargo clippy | tail` reporting exit 0 while emitting 3 errors. Use
  `out=$(cmd); rc=$?`.

### The unknown that most threatens "100% green"

**INFERRED, and it needs the coordinator to settle it.** A prior handoff states
this repo has *"a known pre-existing red band from frankensqlite FTS5
differences (napkin, spec 011)"*
(`thoughts/shared/handoffs/20260810-codex-coverage-gap-2bh4a-fresh-agent-prompt.md:112-115`).
**MEASURED**: that is the *only* mention of it anywhere in the repo
(`rg 'red band|pre-existing red|FTS5 differences'` over the tree, one hit), the
current `napkin.md` no longer carries it (it was flushed), and `specs/011*`
does not exist (`specs/` exists; no `011` directory). So the claim is a single
unconfirmed reference with both of its cited sources gone.

If it is true, **the suite is not green today and was not green before this
work**, and every failure the coordinator meets has to be classified REPRODUCED
/ NOT REPRODUCED / UNAVAILABLE against the pre-change SHA rather than waved
through as pre-existing. If it is false, the goal is simply "keep it green."
Either way it is the first thing the coordinator's own `cargo test --lib` run
answers, and it should be answered before any other verification claim is made.

---

## What "green" currently proves, and what it does not

**Proves.** That a coverage floor, once recorded, lowers a connector's
`since_ts`; that the floor round-trips through the meta table and clears; that
`connector_scan_since_ts` handles its five input shapes; that an aborted
streaming scan keeps its partial output, records a durable floor, and that the
next incremental run reaches the files the aborted scan never opened. That is
real, and it is the incident this fix was written for.

**Does not prove.** That the floor is *per-connector* — the property the fix is
named after, over a production registry of 20 connectors, tested over 1
(bead gxw32, confirmed from source in §3). That the batch path's floor
record/clear works — its only two tests inject one connector each and neither
fails a scan while a floor exists. That anything in the 237 new lines of
`src/lib.rs` behaves correctly — no test was added there. And most sharply:
**that `cass health` / `triage` / `stats` return at all on a real archive.** The
suite is green over a build that hangs those three commands, because the only
thing standing between the timeout and the unbounded read is a line of code no
test observes.

Green today means the archive-recovery mechanism works in a world with one
connector and no clock.

---

## Files and lines cited

| what | where |
|---|---|
| mutation site (M2) | `src/indexer/mod.rs:10657` |
| `ConnectorScanCoverage` | `src/indexer/mod.rs:10641-10684` |
| `ConnectorScanCoverage::new` production callers | `src/indexer/mod.rs:11566`, `:11735` |
| one-connector fixture | `src/indexer/mod.rs:33404`, `:33492`, `:33555` |
| the coverage test | `src/indexer/mod.rs:33444-33574` |
| proposed insertion point | `src/indexer/mod.rs`, after 33574, before 33576 |
| tests module opens | `src/indexer/mod.rs:26351-26353` |
| `connector_scan_since_ts` | `src/storage/sqlite.rs:81-88` |
| storage floor tests | `src/storage/sqlite.rs:23548`, `:23585`, `:23599` |
| the hang | `src/lib.rs:15095-15108` |
| untested coverage surface | `src/lib.rs:15077-15205` |
| `default_data_dir` (refutes purge hazard) | `src/lib.rs:80127-80144` |
| `EnvGuard` global env mutation | `tests/util/mod.rs:459-483` |
| production connector registry (20) | `franken_agent_detection@b62d859 src/connectors/mod.rs:204-237` |
| CI merge gate | `.github/workflows/ci.yml:359-420` |
| UBS gate | `AGENTS.md:241-259`, `.github/workflows/ubs-version.txt` |
| e2e non-hermeticity claim | `napkin.md:39-43` |
| shared-target-dir incident | `napkin.md:22` |
| unconfirmed red band | `thoughts/shared/handoffs/20260810-codex-coverage-gap-2bh4a-fresh-agent-prompt.md:112-115` |
