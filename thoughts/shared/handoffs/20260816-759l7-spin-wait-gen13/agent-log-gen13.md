# Coordinator log — generation 13 (session 0f9160b4)

Bead: `coding_agent_session_search-759l7`
Branch: `worktree-cass-759l7-spin-wait`
Chain: continuation of generation 12 (session 090aa9b4), which fixed 759l7 and
left one question open — what the 8 forward-line failures actually are.

Append-only. Synthesis and the operator-facing deliverable live in
`pin-move-cost.md` beside this file.

---

## Inherited state, verified rather than assumed

- Artifact `p3kgr-upstream-continuation.md` working copy matched its committed
  bytes at `7b9d6a74` (sha256 `3b16c23b…a277af` both sides), so no `git show`
  fallback was needed. Mid-session the parent pushed `1b37ebdc` correcting two
  things in it; pulled and re-read before acting.
- Bead 759l7 OPEN, unchanged. `main` still at `c4b3f955` and still carries the
  spin — `1fc20dbb` is pushed on the branch only.
- Dirtiness baseline `.agent-state/dirtiness/job-0f9160b4-gen13.json`.

## What the gen-12 triage workflow actually left behind

Run `wf_5db3409b-f14` died with its session after 2 of 5 triage lanes returned.
Both survivors are in its journal and both were carried forward here rather than
re-run.

**Why the other three stalled, which the parent had not diagnosed.** All three
prompts told their lane to read
`~/.cargo/registry/src/index.crates.io-*/frankensqlite-0.1.5` and `-0.1.19`.
Those directories do not exist. cass renames the crate in Cargo.toml
(`frankensqlite = { version = "0.1.5", package = "fsqlite" }`), so the published
name — and the vendored directory — is `fsqlite-0.1.x`, with the FTS5
implementation in the sibling crate `fsqlite-ext-fts5-0.1.x`. The two lanes that
returned are exactly the two that needed no library source. Reported to the
parent, which confirmed it as its own error.

## Lane declaration — run `wf_628b78dd-655`

Runtime Claude Code `Workflow`. Script persisted at
`…/0f9160b4-…/workflows/scripts/forward-line-failure-triage-gen13-wf_628b78dd-655.js`.
Transcripts under `…/subagents/workflows/wf_628b78dd-655/`. Visibility:
artifact-visible; journal and per-agent transcripts on disk. Write permissions:
**none** — every lane is read-only and told so; the coordinator owns all writes.
Stop condition: schema-validated classification plus verifier verdict per group.

| lane | group | model | verifiers |
|---|---|---|---|
| `triage:fts-repair-mode` | 2 failures, which repair branch is taken | inherited | 1, escalating to 3 if refuted |
| `triage:fts-shadow-table` | 2 failures, open now rejects the database | inherited | 3 lenses upfront (data-loss shaped) |
| `triage:salvage-counts` | 2 failures, salvage count off by one | inherited | 1, escalating to 3 if refuted |
| *(cached from gen 12)* | `dependency_drift::…` | — | 1 (sonnet/low — literal confirmation) |
| *(cached from gen 12)* | `pages::encrypt::…` | — | 1 (inherited — re-runs the probe) |

Corrections carried into the prompts: real `fsqlite-*` paths; the failure log
copied into this session's job tmp so it outlives the parent; and the measured
environment facts below, so no lane re-derives them.

## Coordinator-run evidence

These were run here, not delegated, because they are cheap and they decide the
framing every lane works inside.

**Stock SQLite settles what a contentless FTS5 table looks like.** sqlite3
3.54.0, temp dir, positive and negative controls both fired:

| table | shadow tables in `sqlite_master` | `_content` present |
|---|---|---|
| `fts5(body, content='', tokenize='porter')` | `_config _data _docsize _idx` | **0** |
| `fts5(body, tokenize='porter')` | `_config _content _data _docsize _idx` | **1** |

The contentless database still opens, inserts and MATCHes fine under stock
SQLite. So a missing `_content` is *correct* for a contentless table and
*genuinely corrupt* for an ordinary one.

**cass has both shapes, and the migration between them is dated.**
`FTS5_REGISTER_SQL` at `src/storage/sqlite.rs:1161-1166` is contentless
(`content='', tokenize='porter'`). Commit `5a304657`, **2026-03-21**,
"migrate FTS5 to contentless mode (schema V14)". Databases written before that
date carry the ordinary shape.

**The fsqlite check did not change.** `fsqlite-core-0.1.5/src/connection.rs:55531`
and `fsqlite-core-0.1.19/src/connection.rs:62679` are the same code: when the
fts5 declaration carries no `content=` option, demand `<table>_content` or raise
`DatabaseCorrupt`. Whatever changed between the versions is upstream of this
check — which rows reach it — not the check itself. `compat_persist.rs` is the
new neighbour, present from 0.1.14 on.

**fsqlite already knows about this failure, and names cass.**
`fsqlite-0.1.19/tests/fts5_contentless_reopen_mutate.rs:1-16` is a regression
test whose header reads "Regression for bd-sf8dx / cass y8n3i", quotes this exact
message, and calls the demand **wrong**. Present in 0.1.14, 0.1.17 and 0.1.19;
absent in 0.1.5. (`y8n3i` does not resolve in this repo's tracker — 0 hits in
`.beads/issues.jsonl`.)

**`PRAGMA writable_schema` is production code here, not a test-only trick.**
`src/storage/sqlite.rs:2187` in `probe_historical_bundle_via_sqlite3_metadata`
(which counts `sqlite_master WHERE name = 'fts_messages'`, so cass already
expects a count other than 1 to be possible in a real bundle) and
`src/storage/sqlite.rs:2480` in `scrub_staged_derived_fts_metadata_via_sqlite3`,
which DELETEs fts_messages rows from `sqlite_master` during salvage staging.

**The failing test says in its own comment that it simulates a real database.**
`src/storage/sqlite.rs:25197` — "Simulate a pre-fix upgraded database that has
never gone through the authoritative frankensqlite FTS rebuild generation yet."

## Controlled differential, both sides verified by content

Provenance was checked by grepping each binary for version markers, never by
mtime — this repo has a recorded incident of a stale binary reporting a false
green.

| | shipping worktree | forward clone |
|---|---|---|
| binary | `target/debug/deps/coding_agent_search-983a915ea0c0a592` | `…/build/coding-agent-search/b9364c709c6f41e6/out/coding_agent_search-b9364c709c6f41e6` |
| markers found | `fsqlite-core-0.1.5`, `asupersync-0.3.2` | `fsqlite-core-0.1.19`, `asupersync-0.3.10` |
| rustc | 1.94.0-nightly (f52090008) | 1.99.0-nightly (969b803cb) |
| the 8 tests, run individually | **8 pass, rc=0** | **8 fail, rc=101** |

Each forward failure was reproduced alone, so none of them is contention. Three
sibling `salvage_historical_databases_*` tests pass on the forward line, so the
salvage failures are specific rather than a broken fixture harness.

The panic in both shadow-table tests is inside `FrankenStorage::open`
(`src/storage/sqlite.rs:22853` and `:25213`), which is *before* cass's repair
runs — `rebuild_fts_via_rusqlite` → `FrankenStorage::open` →
`rebuild_fts_via_frankensqlite` (`src/storage/sqlite.rs:1382-1391`). The repair
that would drop and recreate the table cannot be reached.

## Two places a lane beat the coordinator, recorded because they matter

**The salvage sidecar, which I got half right and stopped too early.** I found
that `fsqlite-vfs` gained `-fsqlite-ns-gate` / `-fsqlite-ns-use` sidecars between
0.1.14 and 0.1.17, guessed they were the extra bundle, then checked
`historical_bundle_root_paths` (`src/storage/sqlite.rs:1933-1993`), saw that
`agent_search.db-fsqlite-ns-use` matches neither the `{db_name}.backup.` nor the
`{db_stem}.corrupt.` prefix, and abandoned the hypothesis. That check was right
and the conclusion was wrong, because I only tested the sidecar beside the
*canonical* database. The lane asked the other question: the sidecar written
beside the **quarantined** file is named
`agent_search.corrupt.20260324_212907-fsqlite-ns-use`, which *does* match the
`.corrupt.` prefix, is 40 bytes so it survives the `total_bytes > 0` filter, and
is not in `has_db_sidecar_suffix`'s allow-list
(`src/storage/sqlite.rs:3008-3017`, which knows only `-wal`, `-shm`, and the
three Windows lock suffixes).

It also explains the ordering that made the test look self-contradictory: the
assertion at `:22541` passes because `historical_bundle_root_paths` materializes
its vector *before* any probe runs; the probe at `:2041-2042` then opens the
garbage file read-only, which writes the sidecar; salvage's own discovery at
`:9029` re-reads the directory and now sees two. And it predicts test 1's `2 → 3`
rather than a naive `2 → 4`, because the `backups/` branch requires a `.bak`
suffix (`:1983`) so the backup's sidecar is not matched. A story that gets a
non-obvious magnitude right is worth more than one that merely fits.

**The rootpage change, which I predicted but could not locate.** I established
that the shadow-table check is byte-identical between 0.1.5 and 0.1.19 and
concluded that whatever changed must be upstream of it — which rows reach the
check. The lane found the actual mechanism: fsqlite changed the rootpage it
writes for its own FTS5 catalog rows, and upstream inverted its own assertion in
the same-named test —
`fsqlite-core-0.1.5/src/connection.rs:119502` asserts `root_page > 0`, while
`fsqlite-core-0.1.19/src/connection.rs:133564` asserts `root_page == 0` with the
reason "must use a stock-compatible rootpage=0 catalog row". Under 0.1.5 the
healthy table had rootpage>0, landed in `materialized_virtual_tables`, and
*masked* the injected rootpage-0 duplicate through the `shadowed_by_materialized`
skip. Under 0.1.19 both rows are rootpage=0, the mask is gone, and the legacy row
reaches validation. So 0.1.5's tolerance was an accident of a non-stock
representation, not a designed repair affordance.

## Toolchain ceiling — re-verified from primary sources

| fact | where |
|---|---|
| asupersync 0.3.9 and 0.3.10 declare a `sysinfo` dependency; 0.3.2 declares none | their `Cargo.toml` |
| sysinfo 0.39.6 declares `rust-version = "1.95"` | `sysinfo-0.39.6/Cargo.toml` |
| the forward lock resolved `sysinfo 0.39.6`; the shipping lock has no sysinfo at all | both `Cargo.lock` |
| `rust-toolchain.toml` pins bare `nightly`, and the installed nightly is rustc **1.94.0-nightly (2025-12-10)** | the file, and `rustc --version` |

The framing worth correcting from the inherited artifact: this is not a ceiling
so much as an **8-month-stale installed nightly**. `rustup update nightly` would
clear it and would change the compiler for every repo on the machine; pinning
`nightly-2026-08-10` in `rust-toolchain.toml` would clear it for this repo only.
Both are Dale's call; neither is a cass code problem.

## Scope caveat, stated rather than papered over

Every green and red number in this chain — 5151/0 shipping, 5143/8 forward — is
`cargo test --lib`. `Cargo.toml` declares 3 `[[test]]` targets and cargo
auto-discovers a further 209 top-level files under `tests/`. **None of that
surface has been measured on either pin in this chain.** It is also not cheap to
measure: the napkin records the e2e suite spawning 8 concurrent
`cass index --full` against the operator's real `~/.codex` and `~/.claude` trees
and not finishing in 90 minutes. So "100% green" is true of the lib suite and
unmeasured beyond it.

## Verifier verdicts

Five returned before this session wound down. **Every one came back
`refuted=false`**, each naming the load-bearing claim it confirmed and how.

| verifier | verdict | what it independently established |
|---|---|---|
| `dependency-drift` | holds | read `src/dependency_drift.rs:799-889`; `checked_in_manifest()` (`:815`) reads the real manifest via `CARGO_MANIFEST_DIR`, and `dependency_spec()` (`:826`) carries no version data feeding the failing assertions |
| `encrypt-overflow` | holds | opened `src/pages/encrypt.rs:298-306` and `:1819-1826` verbatim; confirmed `encrypt.rs` byte-identical across the two trees (84,030 bytes both sides), ruling out a different conversion path |
| `fts-shadow-table` (lens: reproduce) | holds | re-ran the decisive probes rather than trusting the reported ones; found no test-name reasoning, no unopened-library assertion, no dead instrument |
| `fts-shadow-table` (second lens) | holds | **closed the finder's own stated gap** — independently confirmed the rootpage mechanism with a fourth leg of evidence the finder did not have |
| `salvage-counts` | holds | confirmed from source that `bundles_considered` is openability-independent (`:9029-9031` over `:2032-2052`, whose only filters are `exists()` and `total_bytes > 0`), killing the mirror-image hypothesis outright |

Still outstanding at wind-down: the third `fts-shadow-table` lens, and the
`fts-repair-mode` triage lane with its verifier. Generation 14 owns both.

## Addendum, 2026-08-17T01:42Z — the workflow completed after wind-down

`wf_628b78dd-655` finished with all 10 agents returned and 0 errors, minutes
after the paragraph above was written. Both outstanding items landed, so the
handoff to generation 14 was superseded before it was acted on; gen-14 and I
each independently proposed the same split, our messages crossed, and I kept
ownership of the two gen-13 artifacts while gen-14 keeps `gen14/`.

Final verdict tally: **seven verifiers, six confirmed, one refuted.**

The refutation is the important one, and it lands on my own reporting. The
`fts-shadow-table` reachability lens refuted the finder's cost analysis — class
and blocks-pin survive, the reason inverts — and I had already relayed the
superseded version to the operator. Three points, each executed rather than
read: two production `writable_schema` paths exist and are not test-gated
(`sqlite.rs:2180-2196`, `2479-2498`); the range cited to prove the probe dies on
open contains its own `sqlite3` fallback at `2141-2142`, and the verifier ran
both cass's probe SQL (`13 / 2 / 1`, rc=0) and its scrub (rc=0, clean) against a
duplicate-row database it built; and the `shadowed_by_materialized` mask is
built from the file's own rootpages with byte-identical code across versions, so
two real damaged cass databases on this machine (10.5 GB and 11.3 GB under
`~/Desktop/cass-backups-parked/`) still open under 0.1.19 because both carry a
positive-rootpage twin and `fts_messages_content`. The failing shape is a
fixture artifact cass cannot produce.

The `fts-repair-mode` classification is the one nobody predicted: failure 7 is a
production defect on the ordinary insert path, not a test artifact, via cass's
own `rootpage > 0` gate at `sqlite.rs:4127-4131`. Failure 8 is the single
genuine fsqlite 0.1.19 regression in the eight. Both were measured against both
prebuilt rlibs with live positive controls and a stock-sqlite3 ground truth;
details in `pin-move-cost.md`.

Two corrections to evidence recorded above, both from generation 14 rather than
from me, and both worth keeping because they show where this run's instruments
were too narrow:

- The salvage verifier reported `-wal-fec` as new in 0.1.19. It is not —
  `fsqlite-wal-0.1.5/src/wal_fec.rs:2119` builds it with `format!`, and 0.1.5's
  own tests pin `test.db-wal` → `test.db-wal-fec`. The verifier enumerated
  quoted literals in the *vfs* crate only, so a name constructed in the *wal*
  crate was invisible to it. That makes `-wal-fec` a pre-existing gap on the pin
  we ship today, not a pin-move item. It never reached `pin-move-cost.md`, so
  nothing operator-facing had to be retracted — but the near miss is the point:
  an enumeration scoped to one crate cannot answer a question about a name.
  Filed as `…-sidecar-suffixes-missing-wal-fec-jou-7dewl`.
- One verifier's structured output never reached the journal, though it is
  complete in that agent's transcript. Reading only the journal would have
  scored it as missing. Generation 14 recovered it.

## Notes for whoever is next

- The forward target dir is `/tmp/cass-759l7-forward-target`, a **sibling** of
  `/tmp/cass-759l7-forward`, not a child. It is warm: `cargo test --lib --no-run`
  finished in 0.53 s. The test binary lives under `debug/build/…/out/`, not
  `debug/deps/`, because this cargo uses a build-dir layout — reading `deps/`
  and finding it empty is the wrong instrument, and it fooled me first.
- A sibling session has been running `cass index --force-rebuild` against the
  live 23 GB database for ~7 h. Reading `sqlite_master` there shows 71 objects
  and zero containing "fts", but the rebuild can legitimately have dropped them,
  so that observation is confounded and is not evidence about a database at rest.
