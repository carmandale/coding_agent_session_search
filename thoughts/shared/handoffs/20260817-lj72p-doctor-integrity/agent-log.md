# Coordinator log — bead `lj72p`: `cass doctor` never returns on a large archive

Generation 18. Job `7a00a988`, account `katherine`, branch
`worktree-cass-p3kgr-gen13`.

Resumed from `thoughts/shared/handoffs/20260815-cass-to-green/p3kgr-generation-17.md`
(committed `10e7d35b`) via the resume-handoff skill's autolaunched direct path.
Exact next action from that artifact: **fix `-lj72p`**.

## Verification on resume

- Frontmatter complete and `next-action-class: executable`. Repo matches.
- Bead `lj72p` OPEN, P0. Bead `zumve` closed as the artifact says.
- HEAD `182a2c7c` (the continuation receipt) on `worktree-cass-p3kgr-gen13`.
- **Drift**: `origin/main` has moved from `5d1718a3` (the figure in the artifact)
  to `eefadadc` since the artifact was written. The landing recipe's shape is
  unchanged — still not a fast-forward — but the SHA in the artifact is stale.
- Working tree carries only the three `cargo fmt`-churned files the artifact
  names, plus ignored `.agent-state/`.

## Finding 1 — the bead's suggested fix (b) does not work, and its `WHERE` points at the wrong statement

The bead says "Prefer `PRAGMA quick_check` on large archives" and locates the
blocking call at `src/lib.rs:25431-25441`, the `integrity_check` guarded by
`quick_check_ok`.

Read the pinned engine instead of the suggestion. `fsqlite-core` 0.1.5
dispatches **both** pragmas to the same function
(`src/connection.rs:43348-43349`):

```rust
if matches!(name.as_str(), "quick_check" | "integrity_check") {
    return Ok(self.pragma_integrity_check_rows(name == "quick_check"));
}
```

and the walk's own doc comment (`src/connection.rs:42434-42440`) is explicit
about what `quick` buys:

> When `owners` is `Some`, page-ownership tracking is performed (for full
> `integrity_check`). When `None`, **the walk still validates every page's
> structural integrity** but skips the ownership HashMap and orphan detection —
> this is the `quick_check` path.

So `quick` skips the ownership map, orphan detection, rowid-alias and
column-default lookups, and some per-row cell validation. It does **not** skip
the page walk, which is the superlinear term. `PRAGMA quick_check(1)` on a 23 GB
archive is the same O(pages) walk through the same `evict_any` pathology.

Two consequences:

1. Fix (b) is a no-op at best. It must be corrected in the bead, exactly as
   generation 17 had to correct `-zumve`'s own suggested fix.
2. The **first** call in `doctor_database_integrity_probe` is
   `PRAGMA quick_check(1)` at `src/lib.rs:25435`, and it is unbounded. The
   `integrity_check` the bead points at is downstream of it and is not reached.
   Gating only the `integrity_check` would have shipped a fix that changed
   nothing.

The captured generation-17 sample cannot separate the two — they share every
frame below `pragma_integrity_check_rows` — so which one the process was inside
at 25 s is not established by it. It does not need to be: both are unbounded and
the gate covers the probe as a whole.

## Finding 2 — the two `COUNT(*)`s in front of the probe completed

`run_doctor_impl` runs `SELECT COUNT(*) FROM conversations` and
`SELECT COUNT(*) FROM messages` (`src/lib.rs:69635-69648`) before the integrity
probe. The generation-17 sample was taken 25 s into the run and shows the
process inside `pragma_integrity_check_rows`, which proves both counts returned
within 25 s. They are the same unbounded shape as `-zumve`, but they are not
this bead's blocker.

## Lanes

Declared per §3.9. Runtime: Claude Code `Workflow`, four read-only lanes, no
writes outside each lane's own assigned log below. Stop condition: each returns
its structured finding. Visibility: artifact-visible.

| lane | log | purpose |
|---|---|---|
| A | `lanes/a-db-ok-contract.md` | what the skip path must set for `db_ok` / `needs_rebuild` / check status, and what breaks if it is wrong |
| B | `lanes/b-sibling-audit.md` | every other unbounded whole-archive operation reachable from `run_doctor_impl` |
| C | `lanes/c-degrade-precedent.md` | how `cass status` expressed its degrade, and doctor's JSON/robot schema constraints |
| D | `lanes/d-test-conventions.md` | conventions in `tests/cli_doctor.rs` so new tests match the shipped ones |

## Measurements this session (all read-only; specimen byte-identical throughout)

Harness and raw logs in `~/.claude-accounts/katherine/jobs/7a00a988/tmp/`
(`which-pragma.sh`/`.log`, `threshold.sh`/`.log`, `which-pragma-t30.sample`).
That directory dies with the job — the numbers that matter are reproduced here
and in bead `lj72p`'s correction comment.

**Which pragma blocks.** The two share every frame below
`pragma_integrity_check_rows`, so the sample cannot separate them. They differ in
memory: `integrity_check` builds a `HashMap<PageNumber, String>` with one entry
per page walked, and `prod.db` is 5,691,767 pages, so that map would be hundreds
of MB growing monotonically. RSS sampled every 5 s for 200 s:

| t (s) | RSS (KiB) |
|---|---|
| 0 | 32 — the instrument moves; positive control |
| 5 | 4,025,728 |
| 25 | 4,026,672 |
| 30 … 195 | 4,026,672, flat at every one of 34 samples |

A sample at t=30 puts the process inside `walk_integrity_btree_pages` with zero
ownership-map frames. Flat RSS inside the walk = `owners: None` = **quick_check**,
the first call. It had not finished at 200 s.

**Stock SQLite reference.** sqlite3 3.54.0, same files, `mode=ro`:

| database | size / pages | `PRAGMA quick_check(1)` |
|---|---|---|
| `control.db` | 3.98 GB / 972,677 | **rc=0 in 10 s → "ok"** |
| `prod.db` | 23.3 GB / 5,691,767 | **rc=0 in 50 s → "ok"** |

Linear in pages, and it says `ok`. **The operator's archive is not corrupt**, and
the operation is not inherently expensive. The whole cost is the pinned engine's
pager.

**Where the degradation starts.** `cass doctor --json` against the 3.98 GB
`control.db` — which stock answers in 10 s — had not returned at **390 s**. So
the pathology is severe well below the size that provoked the bug report, and the
gate's default must be conservative rather than generous. 256 MiB it is, the
figure `STATUS_COUNT_SCAN_MAX_DB_BYTES` already applies to a plain `COUNT(*)`.

## Synthesis

**Lane A overturned the first design and its evidence is the reason.** The plan
had been a `warn` check plus a new `DoctorAnomaly` variant. Lane A showed that
this codebase already has a settled convention for a declined measurement —
status `pass` with a message that says it was declined — used in three places
(`source_inventory` at `src/lib.rs:69943`, `raw_mirror_backfill` at `:70117`,
`semantic_model` via `skipped-archive-unavailable` at `:31641`). It also showed
that any non-`pass` check joins `build_doctor_root_cause_incidents`
(`:26134` filters `status != "pass"`), so a `warn` would manufacture a doctor
incident and could capture the top-level `recommended_action` on a healthy
archive. The new anomaly class, its policy row, the `ALL_DOCTOR_ANOMALIES` entry,
the incident-kind arm and the `anomaly_taxonomy` golden churn all became
unnecessary. §13 in practice: nothing broke without them.

**Lane A also found the landmine the bead warned about.** `doctor_anomaly_for_check`
classifies the `database` check by **substring match on its message**
(`src/lib.rs:25528-25533`): any non-pass `database` message containing
`quick_check` or `integrity_check` becomes `ArchiveDbCorrupt` — severity Error,
data-loss risk High, "capture-backup-and-reconstruct-from-verified-authority".
The obvious naive edit would have told the operator their healthy 23 GB archive
was corrupt. Under `pass` the function short-circuits at `:25513` and the message
text is inert, which is what makes the chosen shape safe.

**`db_ok` must be `true` on the skip path, and this is not the cautious-looking
answer.** Lane A enumerated all eleven readers. `!db_ok` alone — with no
`needs_rebuild` term — fires `doctor_candidate_build_should_run` and, under
`--fix`, reaches `move_database_bundle` at `src/lib.rs:70951`, which renames the
live db/wal/shm to `agent_search.corrupt.<ts>`. Every consumer reads `db_ok` as
"usable as an authority" (the one call boundary that names it spells it
`archive_db_usable`), which the open and the two counts did establish.

**Lane B is the finding that changes what "done" means.** Gating the integrity
probe alone moves the hang from `src/lib.rs:69653` to `:70031`, where
`collect_doctor_raw_mirror_report` blake3-hashes **48.57 GB across 140,344 blobs**
with no cap, no deadline and no early exit — and the comment at the *status* call
site already records that a **smaller** mirror (125,607 manifests against this
one's 147,844) produced a 15-minute zero-byte `cass status` run. Doctor applies
neither of the two gates status applies. Filed as bead
**`coding_agent_session_search-gf1f0`** (P0) with B1/B2/B3 and the suggested fix.
`cass doctor --json` will not return on the real archive until that lands too.

**Lane C and D** supplied the output-schema constraints and the test conventions:
subprocess invocation through `cass_cmd`, env override scoped to the child,
`assert!(db_bytes > cap, "... or this test proves nothing")` as a fixture
positive control, and the matched-pair discipline (a gate-fires test must be
paired with a gate-does-not-fire test). `tests/cli_doctor.rs` has no
whole-payload or key-set assertion, so a new field does not redden it; the three
doctor goldens in `tests/golden/robot/` do freeze the whole document, but the
fix emits nothing new unless the gate fires, and the goldens use a fresh empty
data dir — so **no golden regeneration is expected**. That prediction is stated
here so it can be falsified by running the suite.

