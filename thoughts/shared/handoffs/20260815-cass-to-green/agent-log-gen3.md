# Coordinator log — generation 3 (session 29dd053b)

Continuation of `thoughts/shared/handoffs/20260815-cass-to-green/agent-log.md`
(generation 2, session c6bfb589). Launched by `scripts/launch-continuation.sh`
per the receipt at `backfill-continuation-prompt.md.launch-receipt.md`.

- worktree: `.claude/worktrees/cass-to-green-c6bfb589`
- branch: `worktree-cass-to-green-c6bfb589`
- parent commit at start: `9d4814d2`
- dirtiness baseline: `.agent-state/dirtiness/29dd053b-e4a3-4e71-89d6-a599d8c5e157.json`

Authorization inherited verbatim (Dale, 2026-08-14): *"/my-way fix cass to
completion and 100% green working state and completely up to date or tell me why
it can't or /grill-me with any questions."* Destructive and external-write
approvals did NOT transfer and are not exercised here.

## Entry state, measured at 2026-08-15 (this session's first reads)

| Fact | Value | How measured |
|---|---|---|
| backfill batches complete | 7 of 20 | `grep -c '^=== batch-.* END' run.log` |
| conversations | 14,583 | `sqlite3 ... 'SELECT count(*) FROM conversations'` |
| backfill alive | yes, pids 4751 / 4753 | `pgrep -f scratchpad/backfill.sh` |
| disk free | 112 GB | `df -h /` |

Handoff recorded 14,430 conversations at 7 batches; 14,583 now at 7 batches, so
batch 8 is mid-flight and the job is progressing, not wedged.

**Deploy is blocked on the backfill, and the reason is mechanical.**
`backfill.sh:38` re-invokes `"$BIN" index --watch-once` where
`BIN=/Users/dalecarman/.local/bin/cass`, once per batch, inside the loop. An
atomic rename at that path mid-run therefore changes the binary the *next* batch
executes. That is a stronger reason to wait than the handoff's "do not overwrite
while it runs" — it is not just a risk to the running process, it silently
splits one backfill across two binaries.

## Ordering decision, and why it deviates from the handoff's literal sequence

The handoff's exact next action is deploy + re-measure, whose own step 1 is
"wait for the backfill to reach 20/20". The wait is real and hours long. Three
pending items are code changes that must be in the binary being deployed:

1. the third copy of the coverage read (`src/indexer/mod.rs`),
2. the per-connector floor test (bead `-gxw32`),
3. the golden repair (bead `-a4xe1`), whose `.actual` files must be regenerated
   **after** all code changes, not before.

Doing them during the wait means one build, one deploy, one golden regeneration.
Deploying first would mean building twice and taking goldens against a tree that
is about to change. The exact next action is unchanged in substance — it is
gated on a wait, and this is the work that fills the wait.

## Lane declaration — generation 3, ground phase

Runtime/tool: Claude Code `Workflow` (dynamic workflow, inspectable via
`/workflows`). Not hidden `Agent` fan-out. Coordinator owns synthesis and every
commit; no lane may commit, push, deploy, touch `~/.local/bin/cass`, start a
second backfill, run `cass sources agents exclude`, or delete any file.

All lanes are read-only against the repo except for their own assigned log.
Visibility class: artifact-visible (each lane writes a durable log under the
path assigned below).

| Lane id | Purpose | Assigned log | Model | Stop condition |
|---|---|---|---|---|
| `gen3-third-copy` | Characterize `read_connector_scan_floors_fresh` (`src/indexer/mod.rs:10696`): can it hang, and what is the correct remedy given widen-vs-narrow semantics | `lanes/gen3-third-copy.md` | inherited | remedy recommended with cited line numbers |
| `gen3-coverage-sweep` | Find every OTHER copy of this read and every sibling swallow of a coverage/floor failure in the tree | `lanes/gen3-coverage-sweep.md` | inherited | enumeration complete, each hit classified |
| `gen3-golden-diff` | Enumerate exactly what differs in each of the 9 `golden_robot_json` failures on the current branch and re-check the handoff's 4/3/2 classification | `lanes/gen3-golden-diff.md` | sonnet | per-file diff table produced |
| `gen3-binary-identity` | Establish what is actually installed at `~/.local/bin/cass` and what preserved copies exist | `lanes/gen3-binary-identity.md` | inherited | sha256 + provenance for each binary |

Lane logs are append-only after launch. The coordinator seeds nothing in them.

## Measurement: what "completely up to date" actually requires

Read-only, measured by the coordinator at 2026-08-15T11:39:13Z by direct set-diff
of the filesystem against `conversations.source_path` (never by reading
`connector_coverage.complete`, which the previous generation proved is
structurally incapable of reporting this hole).

**The archive has no coverage floors at all.** `meta` holds exactly three keys:

```
last_indexed_at  1786793909322  ->  2026-08-15T11:38:29Z   (moving; the backfill writes it)
last_scan_ts     1784196225836  ->  2026-07-16T10:03:45Z   (stuck for a month)
schema_version   20
```

There is no `connector_scan_floors` row. So the floor map is legitimately empty
rather than failed, no connector is ever widened, and an incremental scan is
bounded below by a watermark that has not moved since 2026-07-16.

| connector | on disk | indexed rows | unindexed | unindexed & older than watermark | unindexed & newer |
|---|---:|---:|---:|---:|---:|
| `claude_code` | 8,229 | 4,050 | 8,056 | 15 | 8,041 |
| `codex` | 10,314 | 5,297 | 5,017 | 2,653 | 2,364 |

Two things follow, and they change the shape of the remaining work.

**The running backfill does not finish the job.** Its manifest is 4,895 Codex
files. It covers zero Claude Code files — 0 of 4,895 manifest lines are under
`.claude/projects` (verified by the generation-2 corpus lane). Claude Code has
8,056 unindexed transcripts on disk and its indexed row count has not moved all
session.

**`claude_code` indexed 4,050 against only 173 files still present** is the
`-qtn0e` fact restated from live data: 3,877 indexed rows point at source files
that no longer exist, and cass is their only copy. Nothing this session runs may
purge them. `cass index` does not delete conversation rows (generation 2 verified
this); `cass sources agents exclude claude_code` does, and is not run.

**Why the catch-up cannot be a single ordinary `cass index`.** An incremental run
filters by mtime against `last_scan_ts`, so it would reach 8,041 + 2,364 = 10,405
files and silently skip 2,668 older ones — and it would advance the watermark past
them on the way, closing the door. The proven path is the same one the running
backfill uses: `cass index --watch-once <explicit file paths>`, which is
path-scoped and bypasses the mtime filter. The napkin records that `--watch-once`
does not advance `last_scan_ts`, which is what makes it safe to run first.

So the catch-up is: rebuild a manifest of every unindexed file across every
connector after the backfill finishes, run it path-scoped in batches, then let an
ordinary incremental run advance the watermark. Scale: roughly 10,300 files
against the current backfill's 4,895, so expect it to take longer than the
backfill has.

## Ground-phase synthesis (four lanes in, verifiers still running)

### The record was wrong about the installed binary, and it was wrong in the safe direction

Lane `gen3-binary-identity` settled a contradiction the handoff carried forward.
`backfill.sh:12-13` says it "runs on the installed PRE-FIX binary" and that "a
HEAD build would reintroduce the coverage-floor regression". That comment is
false, and the script's own `run.log:4` refutes it — it records
`binary : 5b3344fd94f93cd4`, the post-fix build.

The live binary is byte-identical to the dated specimen
`cass.nvq59-status-gate-20260814-165549`, contains three production literals
added by `e3ed01f0` and absent from the preserved 2026-06-01 pre-fix binary,
under a positive control firing 15/15 on both and a negative control firing 0/5.

So **the `-1a7mk` coverage regression is live on this machine right now**, and
`8dcd245b` is absent from it. "Don't deploy HEAD, it would reintroduce the
regression" had it backwards: a HEAD build is what removes it.

### The complete reader set is four entry points over two SQL reads — and there is no fifth

Lane `gen3-coverage-sweep` established the closure the previous generation kept
missing. `CONNECTOR_SCAN_FLOORS_META_KEY` has exactly seven uses and the literal
string exactly one; no dynamically built key exists.

| | reader | definition | on failure |
|---|---|---|---|
| R1 | `FrankenStorage::get_connector_scan_floors` | `src/storage/sqlite.rs:6991` | `Err(e)` |
| R2 | `read_connector_scan_floors` | `src/lib.rs:15107` | `None` (fixed by `8dcd245b`) |
| R3 | `read_connector_scan_floors_bounded` (wraps R2) | `src/lib.rs:15161` | `None` (fixed, and bounded) |
| R4 | `read_connector_scan_floors_fresh` (wraps R1) | `src/indexer/mod.rs:10696` | **`BTreeMap::new()`** |

R4 is the only swallower left, and it is the only one whose consumer is not an
operator surface: its failure mode is "the hole is never repaired", not "the hole
is reported as clean".

### `8dcd245b` probably does not fix three of the four hanging surfaces

This is a prediction, and the deploy is the test that settles it. R3 — the thing
`8dcd245b` bounded — has exactly **one** production call site, `src/lib.rs:65743`,
reached through `cass health`. The other three surfaces reach the archive through
`probe_state_db` (`src/lib.rs:15312`), whose own close is
`close_franken_cli_read_db` at relative line 93, and that function is:

```rust
fn close_franken_cli_read_db(mut conn, path, reason) -> CliResult<()> {
    if let Err(err) = conn.close_in_place() {   // <- no wall-clock bound
```

`close_in_place` on a 7.7 GB archive is the exact expensive step `8dcd245b`
identified. `probe_state_db` is called for `state-meta` (`:16088`), a refresh
(`:16720`) and `status` (`:66528`). So `cass status`, `cass triage` and
`cass stats` may still fail to return after the deploy.

Recorded before measuring, so the prediction is falsifiable rather than fitted
afterwards. If all four return, this reading is wrong and I will say so.

### The hang inside R4 is latent, not live

Lane `gen3-third-copy` traced it and then argued against overclaiming it.
Structurally R4 is unbounded. Empirically it has not fired, and the reason is
that both of its call sites already hold a long-lived `storage: &FrankenStorage`
on the same path, so when the ephemeral coverage connection releases,
`open_connections` never drops to 0 and the expensive root drain at
`connection.rs:62744` is skipped. That is an incidental property of the current
call sites, not a designed invariant — but it means the running backfill is
evidence against R4 hanging routinely, and reporting it as a live production
hang would be false.

### Why `a4xe1` sat unnoticed for five days — the root cause, not the symptom

Lane `gen3-golden-diff` confirmed all nine of the previous generation's
classifications exactly, including trailing-byte parity on every file. But the
more useful finding is why nobody noticed.

Five goldens carry Linux-only host values: `diag.json` and `diag_quarantine.json`
pin `platform.os: "linux"` / `arch: "x86_64"`, and the three `status_*` goldens
type four topology fields as `number`/`integer` where macOS returns `None`
(`src/topology_budget.rs:568-588`). `diag.json.golden` last changed 2026-06-01;
`e3ed01f0` landed 2026-08-10. So the suite has been red on macOS for two months
before the regression, and the regression moved it from 5 red to 9 red — invisible
on a suite a developer already knows is partly red.

Filed as `coding_agent_session_search-golden-robot-json-host-drift-tutfy`. It
also means the honest target on macOS is not 37/37: the `a4xe1` repair takes only
the `connector_coverage` hunk for the three `status_*` files precisely so no macOS
value is baked into a contract Linux CI checks, which leaves those five standing.

### Adjacent defects of the same shape, filed rather than fixed here

- `coding_agent_session_search-doctor-promote-gate-fails-open-sgvg3` (P0) — an
  unreadable archive gives the doctor promotion coverage gate a baseline of 0, so
  `promote_allowed` comes back **true**. The same function fails *closed* on the
  candidate side, because the candidate arrives as an `Option` and the baseline as
  a bare `usize` that cannot express "unknown". Same tri-state collapse as `1a7mk`,
  worse consequence, and this archive is the only copy of 3,877 conversations.
