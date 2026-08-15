# Coordinator log — cass to green (generation 2)

Session `c6bfb589-e0c3-4bb9-97b4-04c75f2a043d`, launched by the generation-1
continuation receipt at `74a72233`. Parent session `a91c2501`.

Goal and authorization, inherited verbatim from
`thoughts/shared/handoffs/20260814-cass-repair-to-green/backfill-continuation-prompt.md`
(Dale, 2026-08-14): *"/my-way fix cass to completion and 100% green working state
and completely up to date or tell me why it can't or /grill-me with any questions."*

Destructive and external-write approvals did NOT transfer. No file deletion
(repo AGENTS.md RULE 1), no force-push, no history rewrite, no public-repo
writes, no `cass sources agents exclude`.

## State at session start (measured 2026-08-15T10:57Z)

- Backfill **alive**: `backfill.sh` under `nohup caffeinate -is`, relaunched
  2026-08-15T10:53:41Z. Batch `aa` finished rc=0 in 3m53s. 1/20 batches.
- Live archive: 12,937 conversations (was 12,722 before the escape hatch).
- Disk: 131 GB free on `/` against a 150 GB floor that is already breached.
- Repo: `main` == `origin/main` == `74a72233`, clean.
- `br ready`: 19 issues.

Correction to my own first reading: I initially read `ps -o etime` `03:57` as
3h57m and concluded the backfill was crawling at one batch per four hours. It is
`MM:SS` — 3m57s. Batch `aa` took 3m53s. Recorded because the wrong number would
have justified killing and redesigning a healthy job.

## Isolation

This session runs as a background job, whose harness rejects file edits in the
shared checkout until the session isolates. Working in
`.claude/worktrees/cass-to-green-c6bfb589` on branch
`worktree-cass-to-green-c6bfb589`. This is harness enforcement, not a chosen
deviation from AGENTS.md's main-only rule; landing to `main` is named explicitly
in the final report rather than done unilaterally.

The live archive, the backfill, and the scratchpad all live outside the repo, so
worktree isolation does not touch them.

## Lane declaration — round 1 (read-only grounding)

Runtime: Claude Code `Workflow` tool, inspectable via `/workflows`. Each lane is
a workflow `agent()` call. Visibility class: artifact-visible — every lane writes
one append-only log at the path assigned below and writes nothing else.

Write permissions for every round-1 lane: **its own log file only.** Explicitly
forbidden: any source edit, any `cass index` / `cass sources` invocation, any
write to the live archive, starting a second backfill, `git` mutations, and any
write to another lane's log.

| lane | log path | purpose | stop condition | model |
|---|---|---|---|---|
| `fsqlite-claims` | `lanes/fsqlite-claims.md` | Verify the handoff's UNVERIFIED frankensqlite upstream claims from the local cargo registry cache | claims each marked CONFIRMED / REFUTED / UNAVAILABLE with cited paths | sonnet |
| `coverage-floor-regression` | `lanes/coverage-floor-regression.md` | Root-cause bead `-1a7mk` at the three named call sites | root cause named with file:line, smallest fix described | inherited |
| `golden-robot-json` | `lanes/golden-robot-json.md` | Characterize bead `-a4xe1`'s 9-of-37 red goldens | each failing case classified as real regression vs stale golden | inherited |
| `rebuild-safety` | `lanes/rebuild-safety.md` | Bead `-qtn0e`: does `--force-rebuild` drop source-absent rows? Source reading + design of the smallest falsifier | the code path traced, a runnable falsifier designed (not run) | inherited |
| `coverage-floor-test` | `lanes/coverage-floor-test.md` | Bead `-gxw32`: where a per-connector coverage-floor test belongs and what mutant it must catch | test location + mutant named | inherited |
| `corpus-ground-truth` | `lanes/corpus-ground-truth.md` | Count the real on-disk corpus so acceptance is measurable without `connector_coverage.complete` | on-disk vs indexed counts per connector | sonnet |

Round 2 verifies the load-bearing findings. Per the one-verifier default, only
`rebuild-safety` gets a three-lens panel, because it is the data-loss question.

## Coordinator's own measurements (main thread, not a lane)

### 1. The skip-var safety premise re-verified — and it is half vacuous

The running backfill sets `CASS_SKIP_PREFLIGHT_CLEANUP_ORPHAN_FK_ROWS=1`. That is
only free if the sweep has nothing to clean. The handoff asserts all four child
tables hold zero orphans. Re-measured independently against the live archive with
stock `sqlite3` in `mode=ro`, at `2026-08-15T11:03:56Z`:

| child table | rows | orphans | positive control |
|---|---|---|---|
| `message_metrics` | 582,375 | **0** | 5000 — probe works |
| `token_usage` | 582,375 | **0** | 5000 — probe works |
| `snippets` | **0** | 0 | n/a — table is empty |
| `conversation_tags` | **0** | 0 | n/a — table is empty |

The conclusion holds: skipping the sweep is free, and now on two probes that were
shown capable of returning a positive rather than on four bare zeroes. The control
was the same correlated `NOT EXISTS` with the join key offset by 999999999, so it
exercises the identical code path as the subject.

The correction is that **two of the four tables are empty**, so half the handoff's
evidence is vacuously true. Nobody should carry "all four probes came back clean"
to another machine as if four independent checks had passed. Two did.

`snippets` and `tags` being empty is not a break: `cass search` renders snippets
from tantivy, verified below.

### 2. The wedge site, confirmed from source

Each direct-child probe is a correlated `NOT EXISTS`
(`src/storage/sqlite.rs:6002-6042`, four `OrphanFkTable` entries). That is exactly
the shape the handoff names as frankensqlite's `correlated_exists_fallback`. It
corroborates the handoff's *corrected* root cause and further refutes the earlier
published attribution to the GROUP BY aggregate.

Note `collect_orphan_message_ids` (`src/storage/sqlite.rs:5719`) uses a different
algorithm — min/max plus gap walking, no correlated EXISTS. So the wedge is in the
four direct-child probes, not the message-root probe.

### 3. cass search works

`cass search "frankensqlite" --limit 3` returned three scored, correctly-snippeted
hits across `pi_agent` and `claude_code` in about two seconds
(`2026-08-15T11:04:54Z`). The core purpose of the product is functional on the
live archive. Recording it because "cass is broken" was the inherited framing, and
the part users actually touch is not.

### 4. CORRECTION: 447d97fe **is** deployed — the handoff says it is not

The handoff states the `status --json` fix at `447d97fe` is "**not deployed**
(HEAD still carries the coverage-floor regression, so a HEAD build reintroduces
`-1a7mk`)", and `backfill.sh`'s header comment says it "Runs on the installed
PRE-FIX binary."

Both are false. Measured `2026-08-15T11:0x`:

```
5b3344fd94f93cd4ba03…  ~/.local/bin/cass                              (live)
5b3344fd94f93cd4ba03…  ~/.local/bin/cass.nvq59-status-gate-20260814-165549
d0b860eb6a8ef3664c38…  ~/.local/bin/cass.coverage-floor-fix-20260810
3d04422759268c1752ac…  ~/.local/bin/cass.pre-coverage-floor-20260601
```

The live binary is byte-identical to the nvq59 status-gate build, dated
2026-08-14 16:56 local, and it self-reports `git commit: 447d97fe`. `e3ed01f0`
(the coverage-floor fix) is an ancestor of `447d97fe` — verified with
`git merge-base --is-ancestor`. So the **coverage-floor regression of bead
`-1a7mk` is live on this machine right now**, not rolled back as bead `-1a7mk`'s
"Deployment state" paragraph records.

Caveat held open deliberately: the self-reported git sha cannot be trusted on its
own — bead `ff3d7125` files a known vergen git-sha gap. The load-bearing evidence
here is the SHA-256 match against a binary whose provenance is in its own
filename, not the version string. An empirical hang test of
`status --json`/`stats`/`health`/`triage` is running to settle it behaviourally.

This does not endanger the backfill: `-1a7mk` regresses the readiness surfaces
(health, triage, stats, status), and `cass index --watch-once` is unaffected —
batches are completing rc=0.

### 5. Both `-1a7mk` defects are in one 30-line window, and the second is worse

Read at HEAD (`74a72233`). The bead's line numbers have drifted — it cites
`src/lib.rs:15080/15084`, the real sites are below.

**Defect A — the hang.** `read_connector_scan_floors_bounded`
(`src/lib.rs:15113-15122`) takes a `timeout` and passes it to
`open_franken_cli_read_db` on line 15118 only. Line 15119
(`read_connector_scan_floors`) and line 15120 (`close_franken_cli_read_db`) are
unbounded. So `HEALTH_COVERAGE_OPEN_TIMEOUT` (2s, line 15109) bounds one third of
the operation, and the doc comment's promise — "prefers reporting `checked:
false` over blocking on a contended archive" (15106-15108) — does not hold. Bead
claim confirmed at current HEAD.

**Defect B — the false green, and it is the more damaging one.** Three lines
conspire:

```
15103   .unwrap_or_default()            // read failure  -> empty map
15129   "complete": floors.is_empty(),  // empty map     -> complete: true
```

A *failed* coverage read is indistinguishable from a *clean* one. The file
directly above them (15144-15149) argues this exact case for the `checked` flag —
"`checked: false` is not the same claim as `complete: true`, and collapsing the
two is the whole shape of this bug" — and then the error path collapses them
anyway, one screen higher.

Measured against the live archive at `2026-08-15T11:11:19Z`, `meta` holds
**three** keys and no floors row:

```
last_indexed_at|1786792278862
last_scan_ts|1784196225836
schema_version|20
```

`CONNECTOR_SCAN_FLOORS_META_KEY` is `"connector_scan_floors"`
(`src/storage/sqlite.rs:60`) and there is no such row. So the query finds
nothing, `.unwrap_or_default()` yields an empty map, and `complete` renders
**true** over an archive measurably missing thousands of conversations. The
handoff's warning that `connector_coverage.complete` is not a valid acceptance
signal is confirmed at both the source level and the data level — and the reason
is sharper than "the floor is forward-looking": even a *successful* read of an
empty table produces the same `true`, so the signal cannot distinguish a healthy
archive, a hole predating the mechanism, and a broken query.

That is why acceptance for this effort counts on-disk files against indexed rows
and never reads `complete`.

### 6. The disk-floor breach is 82 GB of abandoned cargo target dirs, not cass

The machine has been below its 150 GB floor for days and `disk-janitor` has been
reporting PARTIAL runs. While attributing a 12 GB drop in free space during this
session, the cause turned out to be my own builds — not the backfill, whose data
dir grew only 0.37 GiB over the same window. Widening the measurement:

```
 32.15 GiB  2026-08-14T22:33Z  /tmp/cass-nvq59-target
 13.21 GiB  2026-08-15T11:19Z  /tmp/cass-c6bfb589-target   <- this session, ACTIVE
  8.71 GiB  2026-08-12T19:28Z  /tmp/cass-il0e9-test-target
  8.01 GiB  2026-08-12T19:15Z  /tmp/cass-il0e9-check-target
  5.88 GiB  2026-08-14T20:46Z  /tmp/cass-repair-target
  4.04 GiB  2026-08-12T20:04Z  /tmp/cass-ubs-drift-test-target
  2.30 GiB  2026-08-12T19:33Z  /tmp/cass-il0e9-release-target
──────────
 74.30 GiB  TOTAL matching /tmp/cass-*target*
+ 7.39 GiB  /tmp/cass-lane-golden (this session's golden lane; the glob missed it)
 81.69 GiB
```

Reclaiming everything except this session's active 13.21 GiB frees ~68 GB and
puts the machine at roughly 182 GB — **above the floor**, without touching the
archive, the raw mirror, or anything irreplaceable. Cargo target dirs are pure
build cache; the cost of losing them is one rebuild each.

I have NOT deleted any of it. Repo AGENTS.md RULE 1 forbids deleting any file
without express written permission, and the continuation prompt restates that
the parent's approvals did not transfer. This is a recommendation for Dale, with
the exact command in the final report.

Worth noting for the pattern rather than the incident: AGENTS.md §2 already
requires a per-checkout `CARGO_TARGET_DIR` for this crate (a shared one silently
runs the wrong binary — it is in the napkin's Corrections table). Every session
that follows that rule mints another 5-32 GB directory under `/tmp`, and nothing
in the rule says to clean one up. The rule is right and it has an unpriced
side effect.

### 7. `br` does not work inside a git worktree

`.beads/beads.db` is gitignored, so a fresh worktree has only the tracked JSONL
and every `br` command fails with `Refusing storage open because pending
sync-merge state could not be inspected`. Bead reads must run from the main
checkout at `/Users/dalecarman/dev/coding_agent_session_search`. Flagged to the
lanes as an inherited hazard; it cost this session two failed calls.

