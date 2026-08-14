# Lane: regression-guard (read-only)

**Owner:** regression-guard lane, launched by the cass-repair-to-green coordinator
**Date:** 2026-08-14
**Mandate:** Dale, mid-work — *"make sure that you are looking at the recent (last 2 weeks) work on cass and not regressing"*. Make that enforceable. Fix nothing.
**Repo:** `/Users/dalecarman/dev/coding_agent_session_search` @ `37d52925` (main, == `origin/main`)
**Writes:** this file only. No commits. No cargo. No index/doctor mutation.

Every claim below is marked **MEASURED** (I ran the command and read the output) or
**INFERRED** (reasoned from source I read, not executed).

---

## 0. Corrections to the coordinator's stated context

Four of the coordinator's premises are confirmed; three need correcting, and one of
the corrections changes where the repair has to land.

| Coordinator's premise | Verdict |
|---|---|
| 32 commits since 2026-07-31 | **MEASURED — correct.** `git log --since=2026-07-31 --oneline \| wc -l` → 32 |
| Only three src files touched | **MEASURED — correct.** `src/indexer/mod.rs`, `src/lib.rs`, `src/storage/sqlite.rs` are the only `src/` paths in the window |
| e3ed01f0 is 932 insertions, merged as 419437e6, in HEAD | **MEASURED — correct.** `git show e3ed01f0 --stat` → 932 insertions / 22 deletions across those three files |
| Rolled back by binary swap, not git revert | **MEASURED — correct.** `~/.local/bin/cass` sha256 `3d044227…` == `~/.local/bin/cass.pre-coverage-floor-20260601`; the fix build is preserved at `cass.coverage-floor-fix-20260810` sha256 `d0b860eb…`. Both hashes match bead 1a7mk exactly |
| **"the region around src/lib.rs:15095-15108"** is the fix site | **INCOMPLETE — see §3.** That region serves `cass health` **only**. `cass status` and `cass triage` read coverage at a *different* site, `src/lib.rs:15283`, which the coordinator's stated scope does not cover |
| e3ed01f0 was the only code change in the window | **MEASURED — incomplete.** `193d2ad6` (clippy/rustfmt, all three files) and `f619a74d` (git-sha identity, `src/lib.rs` +15) are also real code |
| The status --json hang is part of this regression | **MEASURED — false, and this matters.** Bead nvq59 was filed `2026-08-10T13:13:41Z`; e3ed01f0 was committed `2026-08-10T17:39:46Z`. The status hang **predates the fix by 4h26m** and was re-measured today on the pre-fix binary. It is a 20 GB raw-mirror walk (nvq59 comment, 2026-08-14), not the coverage floor. Do not let one fix be credited or blamed for the other |

---

## 1. Inventory — everything that landed since 2026-07-31

**MEASURED**, `git log --since=2026-07-31 --format='%H%n%ci%n%s' --stat`. 32 commits.
Three are real code; the other 29 are bookkeeping.

### Real code (3 commits)

| SHA | Date | Files | What behavior changed |
|---|---|---|---|
| `e3ed01f0` | 2026-08-10 12:39 | indexer/mod.rs +542, lib.rs +239, storage/sqlite.rs +173 | A connector scan that aborts now writes a durable per-connector coverage floor to the `meta` table, the next run lowers only that connector's scan window back to it, and `health`/`status`/`stats` report incomplete coverage as **degraded** instead of healthy. |
| `193d2ad6` | 2026-08-10 14:51 | same three files, +33/−29 | No behavior change by its own claim: drops a duplicated `#[allow]`, merges two adjacent `else if` arms that both returned `"degraded"` in each of `run_status` and `run_health`, rustfmt. **INFERRED from reading both diffs: the claim holds** — the merged arm is `(db_exists && !db_available) \|\| connector_coverage_incomplete`, which is the same predicate the two arms computed. |
| `f619a74d` | 2026-08-12 14:43 | Cargo.toml/lock, build.rs, lib.rs +15, 2 test files | `cass` now embeds its git revision so two builds reporting version 0.6.9 can be told apart. This exists **because of** the rollback: bead c7yaw records that `cass --version` could not distinguish the pre-fix binary from the fix binary. |

### Merge

| SHA | What |
|---|---|
| `419437e6` | Merge of `worktree-codex-coverage-gap-2bh4a` — carries `e3ed01f0` onto main. No conflict resolution content. |

### Bookkeeping (28 commits)

- **21** touch only `.beads/issues.jsonl` and/or `.beads/last-touched`.
- `8ac14e56` — **not code, but not trivial**: recovers 128 issues stranded by a March
  id-normalization split, plus the merge script at
  `thoughts/shared/handoffs/20260810-cass-tracker-merge/merge-cass.py`. A destructive
  `.beads` operation would undo this.
- `286529181` / `998d39ec` — re-triage of the recovered ten against v0.6.9 (±130 lines
  each in the JSONL).
- `acf623b0` napkin creation, `cf57692e` gitignore, `d4552fe9`/`2d0ae0d8` handoff prompt
  + launch receipt, `33ccf045` retired session-end artifacts, `d5cea071` AGENTS.md UBS
  wording, `37d52925` today's nvq59 mechanism comment.

---

## 2. The invariants of e3ed01f0 — the regression checklist's substance

**MEASURED** by reading `git show e3ed01f0` in full plus current HEAD source. These are
the properties any repair must leave standing.

### Storage layer (`src/storage/sqlite.rs`)

- **I1 — the meta key exists and holds a JSON object.** `CONNECTOR_SCAN_FLOORS_META_KEY
  = "connector_scan_floors"` (sqlite.rs:~56), value shape `{"<connector>": <epoch_ms>}`.
  Empty map ⇒ the row is **DELETEd**, not written as `{}` (`write_connector_scan_floors`).
- **I2 — recording is lowering-only.** `record_connector_scan_floor` returns early if an
  existing floor is `<=` the new one. A second failure at a later watermark **cannot
  shrink the hole the first opened**.
- **I3 — a floor of 0 means nothing is proven.** `connector_scan_since_ts(Some(run),
  Some(0)) → None` — read everything. Negative values clamp to 0 in
  `parse_connector_scan_floors`.
- **I4 — malformed JSON is "no floors", not an error.** Deliberate: failing the read
  would hide the connectors that *are* reporting. Note this is also I4's cost — see R4.
- **I5 — the window only ever widens.** `connector_scan_since_ts(Some(500), Some(900))
  → Some(500)`. A floor above the watermark never narrows the scan.

### Indexer (`src/indexer/mod.rs`)

- **I6 — floors are read through a fresh open, never the caller's long-lived handle.**
  `read_connector_scan_floors_fresh` (mod.rs:10696). The commit message records this as
  measured: a floor committed during a scan is invisible to the handle that started it.
- **I7 — the run does not die when one connector does.** `streaming_scan_error` still
  logs and continues; the batch path still collects other connectors' output.
- **I8 — but the failure is recorded durably and immediately.** `record_connector_scan_floor`
  fires at mod.rs:11324-11337 **before any further ingest**, so a process death from
  that point on still leaves the incompleteness on record.
- **I9 — the batch path reports scan errors at all.** Before e3ed01f0 it dropped them
  entirely (`scan_failed` is new; so is the `&& !scan_failed` guard at mod.rs:11871 that
  stops an all-failed connector from being filtered out of `pending_batches`).
- **I10 — each connector scans from its own since_ts.** `ConnectorScanCoverage::new`
  builds `since_ts_by_connector`; the producer config is cloned per connector with
  `since_ts: coverage.since_ts_for(name)` (mod.rs:11594-11598). **This is the property
  with zero test coverage — bead gxw32.**
- **I11 — the floor recorded on failure is the since_ts that run was using**, not the
  current clock (`failure_floor_for`).
- **I12 — a clean discovered scan clears the floor** (mod.rs:11359, 11890).

### CLI surfaces (`src/lib.rs`)

- **I13 — `checked` and `complete` are distinct.** `connector_coverage_state_json(None)`
  emits `{"checked": false, "complete": null}`. Collapsing these is the whole shape of
  the bug: an unchecked surface reading as a clean one.
- **I14 — incomplete coverage is `degraded`, never `healthy`.** In both `run_status`
  (lib.rs:64845, 64861) and `run_health` (lib.rs:65492, 65579).
- **I15 — the `connector_coverage` block is present in `stats --json` (23792),
  `status --json` (65022) and `health --json` (65666)**, and the plain-text surfaces
  print it (23849-23866, 65734-65747).
- **I16 — the recommended action names a real repair path** and is emitted on all three
  surfaces (`connector_coverage_recommended_action`).

---

## 3. Where the repair actually lands — brand-new code, and one call site more than the coordinator named

### 3a. The region is 100% new

**MEASURED.** `git blame -L 15085,15125 src/lib.rs --line-porcelain` — **every one of the
41 lines** is attributed to `e3ed01f0`, author-time `1786383586` (2026-08-10 12:39:46
-0500). Not one line predates the fix.

`git log -L 15085,15125:src/lib.rs` returns `e3ed01f0` and nothing older.

**Plainly: fixing the health hang means editing four-day-old code that has never run in
production, not old code with users.** The blast radius of getting it wrong is bounded to
the coverage feature itself — which cuts both ways. It is safe to change, and there is
nothing downstream that would notice if it quietly stopped working.

Current exact line numbers (**MEASURED**, `rg -n` at HEAD `37d52925`):

| Symbol | line |
|---|---|
| `fn read_connector_scan_floors(conn)` | `src/lib.rs:15077` |
| `const HEALTH_COVERAGE_OPEN_TIMEOUT = 2s` | `src/lib.rs:15095` |
| `fn read_connector_scan_floors_bounded` | `src/lib.rs:15099` |
| — its inner unbounded read | `src/lib.rs:15105` |

### 3b. There are THREE coverage read call sites, not one

**MEASURED**, `rg -n 'read_connector_scan_floors' src/lib.rs`:

| line | caller | reaches | bounded? |
|---|---|---|---|
| `src/lib.rs:65457` | `run_health` via `read_connector_scan_floors_bounded` | `cass health` | open takes a **busy_timeout**; read and close unbounded |
| `src/lib.rs:15283` | `probe_state_db` | **`cass status` AND `cass triage`** | inherits `STATE_DB_OPEN_TIMEOUT` = 5s busy timeout; the added query is unbounded |
| `src/lib.rs:23747` | `run_stats` | `cass stats` | uses the caller's already-open handle; unbounded |

Traced (**MEASURED** by reading the call chain):

- `run_health` → `state_meta_json_for_health` → `state_meta_json_full(…, skip_db_open =
  **true**, …)` (lib.rs:15754-15768) → the open is elided → `connector_scan_floors: None`
  → the `.or_else()` at 65455 fires the **bounded** reader. This is the path the
  coordinator named.
- `run_triage` (lib.rs:65174) and `run_status` both call `state_meta_json_for_status`
  → `state_meta_json_inner(…, allow_db_open = true, skip_db_open = **false**, …)`
  (lib.rs:15775) → **`probe_state_db` runs** → the coverage read at 15283 executes on the
  shared connection.

**Consequence, and this is the single most important line in this document: a repair that
only changes `read_connector_scan_floors_bounded` fixes `cass health` and leaves
`cass triage` and `cass status` exactly as they are.** Bead 1a7mk measured all three
hanging (health >90s, triage >45s, stats >45s). Its stated root cause — *"passes that
timeout to `open_franken_cli_read_db` ONLY"* — is a true and complete account of the
**health** path and cannot be the account for triage or status, which never call the
bounded reader.

### 3c. Two further source facts that sharpen the fix

- **MEASURED, `src/lib.rs:14066-14124`:** `open_franken_cli_read_db`'s third parameter is
  named `busy_timeout` and is used as a SQLite `PRAGMA busy_timeout`. It is **not** a
  wall-clock bound on the open. So `HEALTH_COVERAGE_OPEN_TIMEOUT` does not bound the open
  either — the bead's "the 2s bound covers only the DB open" is generous to the current
  code. A wall-clock variant already exists and is unused here:
  `open_franken_cli_read_db_with_hard_timeout` (`src/lib.rs:14127`), thread + `recv_timeout`.
  **INFERRED:** that is the existing platform facility a repair should reach for rather
  than hand-rolling a second bounding mechanism (minimalism ladder rung 4).
- **MEASURED, `src/storage/sqlite.rs:739-760`:** `retryable_franken_error` matches only
  Busy/Locked/Conflict variants plus message substrings (`busy`, `locked`, `contention`,
  `would block`). **"no rows" is not retryable.** So the popular hypothesis — that a
  *missing* `connector_scan_floors` key makes `franken_query_row_map_retry` spin for its
  10s deadline — is **refuted**: a missing key returns immediately. Whatever blocks, it is
  not that. Recording it so nobody spends a round on it.

---

## 4. Regression checklist, ordered by severity

Each item: the invariant, how a well-intentioned repair breaks it, and the detector.
**"NO DETECTOR" is the finding, not an aside** — write it before touching anything.

---

### R1 — Reverting or neutering e3ed01f0 to make the hang go away
**Severity: fatal.** This is the exact move Dale's instruction names.

**Invariant:** I1-I16 all present in HEAD; the coverage floor is consulted on every
readiness surface.

**How it breaks:** `git revert 419437e6`, or `git revert e3ed01f0`, or deleting the
`connector_coverage` block from `health`/`status`/`stats` "temporarily to unblock the
deploy". Every readiness surface answers in milliseconds again, every test stays green,
and the 4,895 missing codex sessions become invisible again.

**Detector — MEASURED, partial:**
- `src/indexer/mod.rs:33444`
  `aborted_connector_scan_does_not_leave_the_index_claiming_complete_coverage`
  — the only end-to-end guard. **Kills a full revert of the indexer half.**
- `src/storage/sqlite.rs:23548 / 23585 / 23599` — round-trip+clear, since_ts lowering,
  malformed JSON. **Kill a revert of the storage half.**

**NO DETECTOR** for the CLI half. **MEASURED:** `rg -n 'connector_coverage' tests/`
returns **zero matches**; every occurrence in `src/lib.rs` is a definition or a
production call site, none is a test. Deleting the `connector_coverage` block from all
three surfaces, and deleting `!connector_coverage_incomplete` from both healthy-ladders,
ships green. `tests/spec_status_envelope_completeness.rs` does **not** save you: it
compares the uninitialized key set against the initialized key set for *equality* and
pins only `status`, `healthy`, `recommended_action`, `data_dir`, `recommended_commands`
by name — dropping `connector_coverage` from **both** states passes.

**Write before touching:** a test that asserts `health --json`, `status --json` and
`stats --json` each carry `connector_coverage`, that a seeded floor makes `healthy` false
and `status == "degraded"`, and that `checked:false` is emitted with `complete:null`
rather than `complete:true`.

---

### R2 — Widening or removing the timeout so the floor read is skipped and coverage reports complete anyway
**Severity: fatal — it reintroduces the exact defect e3ed01f0 was written to fix.**

**Invariant I13:** an unchecked surface must never read as a clean one.

**How it breaks:** the tempting repair is "bound the whole read, and on timeout carry on".
If the timeout path yields an **empty floors map** instead of **`None`**, then
`connector_coverage_json(&empty)` emits `"complete": true` — a surface that could not read
the archive announcing that the archive is complete. That is a fallback returning
"complete" on error, which is the shape of the original bug.

**Already present in HEAD, MEASURED, `src/lib.rs:15077-15089`:** `read_connector_scan_floors`
returns a bare `BTreeMap` and ends `.unwrap_or_default()`. **A query failure on an open
connection already collapses to `complete: true` today** at both `src/lib.rs:15283`
(status, triage) and `src/lib.rs:23747` (stats). The `Option` that carries the
checked/unchecked distinction only exists at the *open* boundary in
`read_connector_scan_floors_bounded`. So R2 is not merely a risk the repair might
introduce — one instance is in the shipped code and a repair should close it, not widen it.

**NO DETECTOR.** Nothing asserts the timeout/error path at all.

**Write before touching:** a test that forces the coverage read to fail (unreadable db
path, or a `meta` table the read cannot query) and asserts the surface reports
`checked: false` / `complete: null` and does **not** report healthy. Mutant to run against
it: change the timeout branch to `Some(BTreeMap::new())` and confirm the case goes red.

---

### R3 — Dropping the per-connector dimension back to a global value
**Severity: fatal. Proven undetectable.**

**Invariant I10:** each connector scans from its own floor-lowered since_ts.

**How it breaks:** any refactor that reads "the floor" rather than "this connector's
floor" — `floors.values().min()`, a single `floor_ts` field, a cached scalar.

**Detector: NONE, and this is measured rather than argued.** Bead
`coding_agent_session_search-gxw32` records an executed one-line mutant at
`ConnectorScanCoverage::new`:

```
-  let since = connector_scan_since_ts(run_since_ts, floors.get(name).copied());
+  let since = connector_scan_since_ts(run_since_ts, floors.values().copied().min());
```

`cargo test --lib` → **5124 passed, 0 failed, 3 ignored, rc=0** — byte-identical to the
clean baseline. Cause: the fixture at `src/indexer/mod.rs:33485` and `:33548` registers
`vec![("codex", …)]`, a **single** connector, so per-connector and global are
indistinguishable by construction.

**Write before touching:** register at least two connectors in the coverage fixture with
different floors and assert each receives its own `since_ts`. Then re-run the M2 mutant
above and confirm it goes red. Until that assertion exists, **no reviewer and no test run
can tell the coordinator whether a refactor preserved the property the fix is named after.**

---

### R4 — Making the floor read lazy, optional, or cached so it stops being consulted
**Severity: high.**

**Invariant I6:** floors are read fresh, per run, from the durable archive.

**How it breaks:** three plausible "performance" repairs, all of which look reasonable:
1. Cache floors in a `OnceLock`/`static` so repeated surfaces reuse one read — a floor
   recorded mid-run then reads stale, and I6's measured reason (a long-lived handle's MVCC
   snapshot predates the write) applies to a process-lifetime cache just as much.
2. Read floors only when some other signal already says something is wrong — coverage
   becomes unobservable exactly in the healthy-looking case it exists to catch.
3. Gate the read behind a flag defaulting off.

**Detector — MEASURED, partial:** the indexer end-to-end test at `src/indexer/mod.rs:33527`
reads through `read_connector_scan_floors_fresh` deliberately, "the way a later process
would", so it kills a cache **in the indexer**. **NO DETECTOR** on the CLI side: nothing
asserts that `health`/`status`/`stats` observe a floor written by a different process.

**Write before touching:** a CLI-level test that writes a floor into a temp archive's
`meta` table out-of-band, then runs `cass health --json` / `status --json` / `stats --json`
against it and asserts each reports it. That single test is also the R1 and R2 detector —
one fixture closes three of the four fatal gaps.

---

### R5 — Clearing a floor from a scan that never actually read it
**Severity: high, and it is live in HEAD — it will bite task #5 (backfill), not the repair.**

**Invariant I12** says a floor clears after "a scan that actually read from at or below it".
**MEASURED, `src/indexer/mod.rs:11359` and `:11890`:** the condition is
`is_discovered && stats.error.is_none() && coverage.has_floor(name)` — **no guard on scan
scope**, and `opts.watch_once_paths` is not consulted at either site.

The fix's own `recommended_action` (`src/lib.rs:15179-15186`) tells the operator to run
`cass index --watch-once <path>[,<path>...]`. **INFERRED from the two sites above: a
`--watch-once` run over one narrow directory that completes cleanly clears the connector's
entire floor**, and the archive then reports complete coverage over a hole that was only
partly read. The product's own advice walks into it.

**NO DETECTOR.** The fixture never exercises `watch_once_paths`.

**Guard for the coordinator, regardless of the code fix:** when backfilling the codex hole,
verify coverage *after* the backfill by reading the `meta` row directly, not by trusting
that `health` went green — and if the backfill is done with `--watch-once`, treat a cleared
floor as unproven.

---

### R6 — Losing the differential specimens
**Severity: high. Not a code risk — an evidence risk.**

**MEASURED just now:**

```
3d04422759268c17  ~/.local/bin/cass                            (Aug 10 20:38:23)
d0b860eb6a8ef366  ~/.local/bin/cass.coverage-floor-fix-20260810 (Aug 10 20:37:41)
3d04422759268c17  ~/.local/bin/cass.pre-coverage-floor-20260601 (Jun  1 06:21:13)
```

Both hashes match bead 1a7mk exactly, and the live `cass` **is** the pre-fix binary. These
two files are the only way to re-run the 6/6 alternating trial that established the hang is
deterministic. A `cargo install`, a `cp` over `~/.local/bin/cass*`, or a cleanup pass
destroys the baseline, and every later before/after comparison becomes a self-comparison
that reads as "identical" — the failure mode recorded in
`~/.agent-config/.claude/rules/instrument-labels.md`.

**Detector: none possible.** The guard is procedure: name the explicit preserved path and
print its `sha256` in the same output as any timing, per 1a7mk's own instruction. Never
re-deploy by `cp` over the live path — 1a7mk records that in-place overwrite gives SIGKILL
from a stale signature cache even when `codesign` reports the bytes valid. Use an atomic
rename.

---

### R7 — Attributing the status/doctor hang to the coverage fix (or vice versa)
**Severity: medium — it corrupts the verdict, not the code.**

**MEASURED:** nvq59 filed `2026-08-10T13:13:41Z`; e3ed01f0 committed `2026-08-10T17:39:46Z`.
The `status --json` hang predates the fix by **4h26m**, and today's comment (37d52925)
re-measured it on the pre-fix binary `3d044227…`: CPU-bound, `STAT=R`, no sqlite FD open at
any point, one read FD walking 125,601 raw-mirror blobs / 20 GB, RSS climbing ~4 MB/s.
`cass doctor` without `--json` also does not return on that same pre-fix binary.

Two independent hangs share one symptom. If the coordinator fixes the coverage read and
`status --json` still hangs, that is **not** a failed fix; if the raw-mirror walk is fixed
and health returns, that is **not** evidence the coverage read is bounded.

**Detector: MEASURED and adequate** — bead 1a7mk's alternating-binary protocol, which
separates them by construction. Reuse it. Also note 1a7mk's own caveat: `stats` 26.9s →
>45s is **not** separated from ordinary slowness by that measurement; health and triage
(sub-100ms → no return) are the decisive rows.

---

### R8 — Sweeping the recovered beads
**Severity: medium.**

`8ac14e56` recovered 128 issues stranded by the March id-normalization split, and two
later commits re-triaged them (±130 lines of `.beads/issues.jsonl` each). A `br sync`
without `--flush-only` imports from JSONL and overwrites the DB; a stale index committing
`.beads/issues.jsonl` reverts the recovery silently.

**Detector — MEASURED, adequate:** `git log -- .beads/issues.jsonl` plus
`git diff HEAD~1 HEAD --stat` before any push. Population today: 1,897 rows in the JSONL,
22 updated since 2026-07-31.

---

### R9 — Clippy/rustfmt drift reopening the same round
**Severity: low, but it costs a full cycle.**

`e3ed01f0` landed without `cargo clippy --all-targets -- -D warnings` or `cargo fmt --check`
and `193d2ad6` existed solely to repair that. The most likely place a repair reintroduces
it is the same one: a new `else if` arm returning `"degraded"` next to the merged arm at
`src/lib.rs:64861` / `:65579` re-trips `clippy::if_same_then_else`.

**Detector: adequate**, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
— **owned by the coordinator, not this lane** (I ran no cargo).

---

### R10 — Trusting the existing latency test to catch the hang
**Severity: medium — a live false-green.**

`tests/spec_health_latency_contract.rs` exists and pins `cass health --json` at
`latency_ms <= 150` in both uninitialized and initialized states. **MEASURED from source:**
`latency_ms` is computed at `src/lib.rs:65588`, *after* the coverage read at `:65455`, so
the metric is honest. **The fixture is not.** It runs against `tests/fixtures/search_demo_data`
— a few MB — while the hang needs the 7.9 GB archive and frankensqlite's open/close
lifecycle. This test was green while `cass health` sat at >90s in production. A repair that
"passes the latency contract" has proven nothing about the failure it is repairing.

**NO ADEQUATE DETECTOR** at the scale that matters, and honestly none is cheap to write —
a multi-GB fixture is not a unit test. The realistic substitute is procedural: after the
repair, run 1a7mk's alternating protocol against the real archive with explicit binary
paths and printed hashes, and report *that*, not the suite.

---

## 5. Unlanded and destroyable work

**MEASURED.**

- `git status --short` — four untracked paths, none of them source or beads:
  `.agent-state/`, `.grok/`, `docs/goals/cass-lexical-refresh-finalization/.goalbuddy-board/`,
  `solo.yml`.
- `git log origin/main..HEAD` — **empty.** Nothing unpushed.
- **Three stashes, all pre-dating this window and none of it in main:**
  `stash@{0}` "On main: pre-upstream-sync last-touched"; `stash@{1}` "On
  fix/watch-once-chunk-size: beads auto-staged during PR work"; `stash@{2}` "WIP on
  feat/007-watchdog-subcommand: spec 008 upstream sync". **Do not run a bare `git stash`
  or `git stash pop`** during the repair — the stack is shared and these three are not
  yours.
- `git log --all --not --remotes --oneline` — 11 entries, all stash internals plus a
  `sync/012` chain from March/April. Nothing from this window.
- **Worktree branch `worktree-codex-coverage-gap-2bh4a`: fully merged, holds nothing.**
  `git log main..worktree-codex-coverage-gap-2bh4a` → **empty**; main is 15 commits ahead.
  The worktree at `.claude/worktrees/codex-coverage-gap-2bh4a` is clean but for an
  untracked `.agent-state/`. Safe to leave; safe to remove; removing gains nothing.
- **Six broken branches — 6/6 reproduced just now**, `git rev-list --count <ref>`:
  `beads-sync`, `feat/007-watchdog-subcommand`, `feat/doctor-reconciliation-v2`,
  `fix/index-gaps`, `fix/watch-state-skip-prevention`, `fix/watcher-cpu-spin` all error on
  missing objects `cb78850f…` / `cba21b28…`. `main` (4179) and the worktree branch (4164)
  count clean. **Safe to leave alone** for a source repair — the tips resolve, only ancestry
  has holes. One live consequence to expect and not misread: any all-branch walk fails,
  which is why `close-check` can report freshly-pushed main commits as unpushed (false red,
  exit 65). Verify pushes with `git rev-list main ^origin/main --count` instead. Bead 6t64c
  holds the decision; it is Dale's call under RULE 1, not the repair's.

---

## 6. Detector summary — what to write before touching anything

| Risk | Detector today | Status |
|---|---|---|
| R1 indexer/storage revert | `src/indexer/mod.rs:33444`, `src/storage/sqlite.rs:23548/23585/23599` | **adequate** |
| R1 CLI surface removal | none — `rg -n 'connector_coverage' tests/` = 0 hits | **NO DETECTOR** |
| R2 error path reports complete | none | **NO DETECTOR** (and one instance is live at `src/lib.rs:15077`) |
| R3 per-connector → global | none — proven by executed mutant, 5124/0 green | **NO DETECTOR** |
| R4 lazy/cached floor, CLI side | indexer only (`:33527`) | **partial** |
| R5 clear without reading | none | **NO DETECTOR** |
| R6 specimen loss | procedure only | **not testable** |
| R7 hang attribution | 1a7mk alternating protocol | **adequate** |
| R8 bead recovery swept | `git diff HEAD~1 HEAD --stat` | **adequate** |
| R9 clippy/fmt | `cargo clippy -D warnings`, `cargo fmt --check` | **adequate** (coordinator-owned) |
| R10 latency contract at scale | `spec_health_latency_contract.rs` — fixture too small | **false green** |

**One fixture closes R1-CLI, R2 and R4-CLI:** a temp archive with a floor written directly
into `meta`, run `cass health --json` / `status --json` / `stats --json` against it, assert
the block is present, that `healthy` is false and `status == "degraded"`, and that an
unreadable archive yields `checked:false` / `complete:null` rather than `complete:true`.
**R3 needs its own change** — a second connector in the existing coverage fixture — and it
is the one gap where no amount of review substitutes for the assertion.

---

## 7. What I did not do

- Ran no `cargo` anything. Every claim about tests is from reading source, from the recorded
  results in `193d2ad6`'s commit message (5124 passed / 0 failed / 3 ignored) and from bead
  gxw32's executed mutant (5124 passed / 0 failed, rc=0). I did not re-run either.
- Ran no `cass` binary, no `cass index`, no `cass doctor`.
- Did not enter or modify the worktree beyond a read-only `git status`.
- Wrote no file but this one. Committed nothing.
