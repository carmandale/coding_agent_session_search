# cass repair to green — coordinator log

**Date:** 2026-08-14
**Session:** a91c2501-1830-4d3d-9430-3c9afe08a63c (Claude Code, run from ~/.agent-config)
**Goal (Dale, verbatim):** "/my-way fix cass to completion and 100% green working state and
completely up to date or tell me why it can't or /grill-me with any questions."
**Mid-work instruction (Dale, verbatim):** "make sure that you are looking at the recent
(last 2 weeks) work on cass and not regressing"

Read taken on that second message: a correction to the work in flight — build on the last two
weeks, do not revert it — not new work and not a stop. Acted on by launching a dedicated
regression-guard lane whose only job is to make it enforceable.

## Sharpened goal

cass is "fixed" when all of the following hold, each proved rather than asserted:

1. Every agent-facing command returns promptly on the LIVE 7.9 GB archive — not on a fixture.
   Today `cass status --json` and `cass doctor` never return on the installed binary.
2. The coverage-floor fix (`e3ed01f0`, already in main) is DEPLOYED, with the hang that forced
   its 2026-08-10 rollback removed at the cause rather than worked around.
3. The index holds all Claude Code + Codex sessions from both machines: the ~13,300 missing on
   the laptop (including the 4,895-session codex hole and the 1,647 flat-layout rollouts) and
   the 4,878 on the mini.
4. It stays current without a human remembering to run anything.
5. The suite is green AND non-vacuous — bead gxw32 shows today's green survives a one-line
   mutant that reintroduces the exact defect e3ed01f0 fixed.

"Or tell me why it can't" is a live branch, not a formality. Anything in this list that turns
out to be blocked gets said plainly, with the evidence, rather than quietly dropped.

## Decisions taken (with evidence)

- **No new spec.** specs/016, 017, 018 are from May and cover different problems (recovery
  ingestion, watch-once OOM, lexical refresh). The work is already scoped by specific open
  beads — 1a7mk, nvq59, kfaid, gxw32, 2bh4a. Minting a spec would add a part that answers no
  observable requirement. Lane logs therefore live in this handoff directory, and bead comments
  carry the durable findings.
- **Build with the repo's pinned nightly, not the machine default.** See below.

## Blocking environment finding — the tree does not build with the default toolchain

`cargo check --all-targets` fails at `fsqlite-pager-0.1.5`:

```
error[E0554]: `#![feature]` may not be used on the stable release channel
 --> ~/.cargo/registry/.../fsqlite-pager-0.1.5/src/lib.rs:2:1
2 | #![feature(core_intrinsics)]
```

Cause, measured: `rust-toolchain.toml` pins `channel = "nightly"`, but `rustup` is **not on
PATH** on this Mac and the active toolchain is Homebrew's stable `rust` 1.96.0. Homebrew's cargo
does not read `rust-toolchain.toml` — that is a rustup shim feature — so the pin is silently
ignored and the build fails in a way that reads as "the repo is broken."

It is not broken. A nightly toolchain is installed at
`~/.rustup/toolchains/nightly-aarch64-apple-darwin/`.

**Calling the nightly cargo by absolute path is NOT enough, and it fails identically.** Measured:
invoking `~/.rustup/toolchains/nightly-.../bin/cargo check` produced the exact same E0554 at the
exact same line. A rustup toolchain's cargo binary, invoked directly rather than through the
rustup shim, resolves `rustc` from `PATH` — so Homebrew's stable rustc 1.96.0 still did the
compiling while the cargo reported itself as `1.94.0-nightly`. The two `--version` lines at the
top of that run both said nightly, which is precisely why the second failure was confusing: the
instrument was reporting the toolchain I asked about, not the one doing the work.

Put the nightly bin directory first on `PATH` (or set `RUSTC` explicitly):

```bash
export PATH="$HOME/.rustup/toolchains/nightly-aarch64-apple-darwin/bin:$PATH"
CARGO_TARGET_DIR=/tmp/cass-repair-target cargo check --all-targets
```

This also explains why AGENTS.md documents every cargo command through `rch exec --`: `rch` is
not installed here, and it is presumably what selects the toolchain on the author's machine.

Two further notes for anyone who follows: the repo napkin records that a shared
`CARGO_TARGET_DIR` between two checkouts silently ran the WRONG binary — cargo's freshness check
printed `Finished in 0.41s` and re-ran the other tree's test binary. Hence the dedicated
`/tmp/cass-repair-target`. And `pipestatus`/`$?` after a piped cargo run reports the pipe, not
cargo; the first baseline here reported `CHECK_RC=101` only because the status was captured
explicitly.

## Regression risk is concentrated, which is good news

32 commits since 2026-07-31, but only **three** source files were touched in that window:
`src/indexer/mod.rs`, `src/lib.rs`, `src/storage/sqlite.rs`. Everything else is beads, handoffs
and docs bookkeeping. The recent work is narrow and legible, so "don't regress it" is a
checkable property rather than a hope. The regression-guard lane owns turning it into a list.

The specific trap, named before starting: the cheapest way to stop the hang is to revert or
neuter `e3ed01f0`, and that would undo the exact repair this work exists to finish. A subtler
version is reintroducing an error path that reports coverage complete when the floor read fails
— which is the original defect wearing a timeout.

## Lane declaration (AGENTS.md §3.9)

Runtime: Claude Code `Workflow` (inspectable via `/workflows`) plus one named background agent.
Visibility: artifact-visible; every lane log is committed here. Write permission: **each lane may
write only its own log path.** No lane may run cargo, `cass index`, `cass doctor --fix`, any
database or binary mutation, or any commit. Stop condition: structured finding returned and log
written.

| lane | purpose | log | model |
|---|---|---|---|
| `bound-lifecycle` | The deploy blocker (1a7mk): the unbounded open/read/close and the smallest fix that removes the cause | `lanes/bound-lifecycle.md` | inherited |
| `raw-mirror-walk` | nvq59 + the undocumented `cass doctor` hang: why `status --json` walks 20 GB that plain `status` skips | `lanes/raw-mirror-walk.md` | inherited |
| `backfill-mechanics` | The recovery runbook, and whether spec 017's watch-once OOM would kill a 12-hour run | `lanes/backfill-mechanics.md` | inherited |
| `test-integrity` | What "green" currently proves; the exact change that closes gxw32; which suites are safe to run here | `lanes/test-integrity.md` | inherited |
| `freshness-and-skill` | Durable freshness within the repo's own anti-launchd decision; the jsm-owned skill that points agents at the hang | `lanes/freshness-and-skill.md` | sonnet |
| `build-deploy-path` | Exact build/deploy/rollback ritual and every gate before a push | `lanes/build-deploy-path.md` | sonnet |
| `regression-guard` | Dale's mid-work instruction, made enforceable: invariants of the last 14 days and the detector for each way a repair could break them | `lanes/regression-guard.md` | opus |

Synthesis, dispositions (Applied / Rejected with contrary evidence / Superseded), and the final
result map stay with the coordinator and are appended below.

## Coordinator's own reading of the deploy blocker — recorded BEFORE the lanes reported

Written deliberately ahead of `bound-lifecycle` returning, so its findings adjudicate against this
rather than the reverse. If the lane contradicts any of it, the lane's source evidence wins and the
correction gets recorded here.

**The defect, read at HEAD.** `read_connector_scan_floors_bounded` (src/lib.rs:15099-15108):

```rust
let conn = open_franken_cli_read_db(db_path.to_path_buf(), "connector-coverage", timeout).ok()?;
let floors = read_connector_scan_floors(&conn);
let _ = close_franken_cli_read_db(conn, db_path, "connector-coverage");
Some(floors)
```

The `timeout` argument reaches `open_franken_cli_read_db`, which (src/lib.rs:14120-14122) spends it
as `PRAGMA busy_timeout`. **A busy timeout is not a deadline.** It bounds how long SQLite waits on a
contended *lock*; it does not bound the open itself, the query, or the close. So the 2 s in
`HEALTH_COVERAGE_OPEN_TIMEOUT` never had the meaning its name and doc-comment claim, and the read
and close that follow are unbounded outright. That is why 2 s became >90 s.

**The repo already solved this, one function below.** `open_franken_cli_read_db_with_hard_timeout`
(14127-14148) is the established precedent: spawn a worker thread, hand the result back over an
`mpsc` channel, and enforce a real deadline with `rx.recv_timeout(timeout)` in
`receive_franken_cli_read_db_open_result_with_hard_timeout` (14193), which has explicit `Timeout` and
`Disconnected` arms. The bug is not a missing mechanism — it is the *wrong one of two adjacent
functions* being called. `read_connector_scan_floors_bounded` reaches for the soft variant and then
adds two more unbounded steps after it.

**Intended fix (smallest, reuses the precedent):** move the whole open → read → close lifecycle into
the worker thread and bound the receive once. Sending back a `BTreeMap<String, i64>` rather than a
connection also avoids needing `SendFrankenConnection`, so the change is smaller than the precedent
it copies.

**The invariant this must not break, and it is exactly the risk Dale's instruction points at.**
`connector_coverage_json` (15112-15116) computes:

```rust
"complete": floors.is_empty(),
```

An **empty map means complete coverage.** So a fix that returns `Some(BTreeMap::new())` when the
probe times out would report *complete coverage on failure* — which is precisely the defect
`e3ed01f0` was written to remove, reintroduced through the error path instead of the watermark. The
timeout arm must return `None`, which routes to the sibling renderer at 15130 whose doc-comment
already says it is "honest about a probe that never opened the database."

That sibling function existing is good evidence the original author understood the distinction; the
bounded reader simply never used it for the timeout case, because it had no timeout case to speak of.

**Open empirical question, deliberately not answered from the armchair:** whether 2 s is enough for a
legitimate coverage read on a 7.9 GB archive once the bound is real. If it is not, every health
probe will honestly report "not checked", which is safe but useless, and the fix is then also to make
the read cheap rather than only to bound it. This is measurable after the build and will be measured,
not assumed.

**Constraint confirmed from AGENTS.md, ruling out the obvious shortcut.** Bead 1a7mk notes stock
sqlite3 reads the same meta table in 0.000s with `mode=ro&immutable=1`, which invites "just read it
with plain sqlite." AGENTS.md forbids that twice over: RULE 2 bans rusqlite in new code outright, and
the "Verified Standard SQLite File Reads" section says explicitly *"Do not add rusqlite just to read
an existing SQLite file"* and to file a reproducer against frankensqlite instead. So if the read is
genuinely slow inside frankensqlite, the sanctioned response is a targeted reproducer, not a bypass.

## Coordinator measurements on the live archive (before lanes returned)

**Baseline is green.** With nightly on PATH, `cargo check --all-targets` finished in 2m31s at
`CHECK_RC=0`. The tree was never broken — the earlier failure was toolchain selection, nothing else.
Warm target dir now at `/tmp/cass-repair-target`, so later checks are cheap.

**The data side of the coverage read is free.** Three runs of stock sqlite3 against the live 7.93 GB
archive:

```
sqlite3 "file:$DB?mode=ro&immutable=1" "SELECT key,value FROM meta WHERE key LIKE '%scan%' ...;"
real 0.01 / 0.00 / 0.00
```

So bead 1a7mk's claim holds, measured independently: reading these rows costs nothing. The >90 s
therefore lives entirely in frankensqlite's open (or close) path, not in the query. This settles the
shape of the fix — there is no query to optimize. Bounding the lifecycle is the correct change, and
the frankensqlite open cost is separately worth the targeted reproducer AGENTS.md asks for.

**Discovery that changes the plan: the coverage floor is forward-looking only.** The live meta table
holds exactly three keys:

```
last_indexed_at | 1784200805044
last_scan_ts    | 1784196225836      → 2026-07-16T10:03:45Z
schema_version  | 20
```

There are **no connector floor rows**, because `e3ed01f0`'s code has never run against this database
— it was rolled back a minute after install. Now read that against `connector_coverage_json`
(src/lib.rs:15115), which computes `"complete": floors.is_empty()`.

**Consequence: the moment the fixed binary is deployed, cass will report `complete: true` on a
database with a known ~13,300-session hole.** The floor mechanism stops a *future* aborted scan from
claiming completeness. It cannot retroactively know about the 2026-06-01 abort that caused the
existing gap, because that abort predates the mechanism and left no floor row behind.

This is not a defect in `e3ed01f0` — the design is right and the semantics (absence of a floor means
no known incomplete scan) are the sane ones. But it means two things for this work, both of which
would otherwise be easy to get wrong:

1. **Deploying the fix does not fix the coverage gap.** It prevents recurrence. The backfill is a
   separate and still-mandatory operation. Anyone reading "the coverage fix is deployed" as "coverage
   is complete" is wrong.
2. **`connector_coverage.complete` is not a valid acceptance signal for this goal.** Final proof that
   the index is "completely up to date" must be a count of indexed sessions against the on-disk
   corpus, not cass's own coverage verdict — which will say complete either way. Task 8's proof is
   written accordingly.

## Findings and dispositions

(appended as lanes return)
