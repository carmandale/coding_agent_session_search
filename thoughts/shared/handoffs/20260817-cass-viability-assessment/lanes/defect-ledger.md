# Lane: defect-ledger — the honest defect ledger for cass, from the repo's own records

Date: 2026-08-17. Read-only lane; no production data touched, no cass invocations run
(one `br ready --json` against the repo's own beads db, subsecond; PID 75534 untouched).
Sources: continuation artifacts g11/g12/g13, both handoff directories, `.beads/issues.jsonl`
parsed line-by-line with python, `git log`, and one negative source probe (`rg` for the g13
fix symbol in `src/storage/sqlite.rs`).

## 1. What the newest artifacts say (g11, g12, g13)

Three continuation artifacts, all under the same verbatim goal (Dale, 2026-08-14:
"/my-way fix cass to completion and 100% green working state and completely up to date or
tell me why it can't"). Two chains converged:

- **g11** (`cass-green-continuation-g11.md`, 24KB, mtime 08-17 06:37, reconciled by commit
  `eefadadc`): 1pzs3 fixed on the path that actually builds the archive (`9531315d`); the
  archive is missing tool messages in **6,452 of 10,283 codex conversations** (filed `g0eyv`);
  ibuuh.29.1 diagnosed to the statement — a whole-corpus
  `SELECT conversation_id, COUNT(*) ... GROUP BY` that is 96% of the prep step that wedges,
  whose output is discarded one line later (`indexer/mod.rs:9185`); qtn0e's premise ("sole
  surviving copy of 3,877 Claude conversations") **falsified** — the raw mirror holds a
  blake3-verified second copy — but the destructive default was real and was fixed
  (`05dff6f4`).
- **g12** (`p3kgr-generation-12.md`, 08-16 20:29): "cass is not broken; the SQLite engine it
  is pinned to cannot run a GROUP BY." Stock sqlite3: **77 ms** on the live 22 GB archive.
  fsqlite 0.1.5: **7h26m+, never returned** (the probe process later measured grinding at
  97.4% CPU, not deadlocked). Rebuild works at 12,722 conversations (4.2 s for the step) and
  stops working at 27,441. The pin bump to 0.1.19 does NOT fix it (4,880 ms at control scale;
  2h28m48s at prod scale per g13).
- **g13** (`p3kgr-generation-13.md`, 08-17 03:50): the guard inversion in
  `list_conversation_footprints_for_lexical_rebuild` (`src/storage/sqlite.rs:7405-7414`)
  found and fixed — the exact-count fallback fired on healthy archives instead of only on
  ones missing tail metadata. **Result: the full lexical rebuild that had never completed ran
  in 57.4 s on the shipping 0.1.5 pin** (159 shards, 2,334,366 docs, ledger line quoted in
  the artifact). But: "The change is NOT yet in the repo. Landing it is the main deliverable."
  And a third defect remains open: the **query phase** itself — `cass search` against the
  already-built index returned nothing in 980 s (16m20s) before an external SIGTERM
  (commit `5d1718a3`; `/tmp/rerun.log` confirms rc=143, empty stdout). `sample(1)` puts every
  working frame in fsqlite's pager (S3Fifo insert/trim/build_model), hypothesized 64 MB page
  cache against a 22 GB database.

**Verified this lane, 2026-08-17:** the g13 guard-fix symbol
(`lexical_rebuild_tail_estimates_understate_message_total`) is **absent from
`src/storage/sqlite.rs` on main** (`rg -c` exit 1). The one fix that made the rebuild complete
lives only in `/tmp/cass-0119-test` + `/tmp/cass-fix-target/release/cass`. The sibling probe
still running 4h48m+ today (PID 75534, coordinator ground truth) is the query-phase defect
reproducing live.

## 2. Beads: what is actually in the 6 MB file

Parsed `.beads/issues.jsonl` per line (python, no br):

- **1,927 lines, 0 parse errors, 1,927 unique ids** — one line per bead, no duplicate/update
  rows. 6,242,328 bytes ≈ **3.2 KB per bead**: descriptions carry full measurement writeups,
  which is why the file is 6 MB.
- **Final status: 1,885 closed / 35 open / 6 in_progress / 1 tombstone.**
- The "only 5 rows" reading does **not** reproduce: live `br ready --json` returns a bare
  array of **23 rows** (open, unblocked, not deferred — excludes the 6 in_progress by
  definition). Of the 35 open, 25 have no unresolved dependency; 10 are dep-blocked (mostly
  epic children). Any 5-row result was a filtered or paginated view (`br list --json` returns
  an OBJECT with `has_more`/`limit`/`total`, so reading it as an array or taking one page
  under-reports), not the file's content.
- **Stale in_progress claims:** 5 of the 6 in_progress beads were created 2026-05-15..17
  (1vxuf, 2d37b, 2gif2, 373b1, 81z91 — ingestion/watcher/watch-once-OOM work) and have sat
  claimed since May. The sixth is p3kgr (the live P0).
- Churn in the rescue window: **42 beads created since 08-10; 24 closed since 08-14.**

### Every open defect (final state, wishlist epics excluded)

P0:
- `ibuuh.29.1` (open, created 2026-04-19) — single-core "preparing" plateau; full rebuild has
  never completed on the live archive. Closed 2026-04-22 in an earlier form; regression
  re-introduced 2026-05-13; the April proof test structurally cannot see it.
- `p3kgr` (in_progress, created 2026-08-14) — "cass cannot index at all: one lexical-prep
  aggregate over messages takes >20 min in frankensqlite, 0.03 s in stock." Carries the
  three-defect root cause (guard inversion / fsqlite GROUP BY / query-phase pager thrash).

P1 bugs:
- `g0eyv` — 6,452 of 10,283 codex conversations hold zero tool messages; a reindex is owed.
- `pfar8` — `cass mirror prune` pins by recency only, never by upstream absence, so it deletes
  exactly the irreplaceable blobs; cass's own doctor RECOMMENDS the prune past 100 GB.
- `hd4u5` — FTS write gate `AND rootpage > 0` silently disables all FTS maintenance under
  fsqlite >= 0.1.17.
- `xybl9` — sidecar allowlist misses fsqlite's `-fsqlite-ns-gate`/`-ns-use` family (reopens
  the orphan-amplification class).
- `move-bundle-stale-hot-journal-gtfx5` — stale rollback journal can be replayed into a
  freshly created database at the same path.
- `iapqz` — the repo's own git object store has 16 broken links (8 trees, 6 blobs, 2 commits).
- `759l7` (task) — three hand-rolled spin-waits self-deadlock under asupersync 0.3.4+, one on
  the CLI startup path.

P2/P3 defects: `n62wn` (exclude's --purge branch untested; known surviving mutant),
`ns-sidecar-transport-bricks-db-1mgjd` (lost mode bit bricks the DB permanently), `d907f`
(contentless FTS5 stores column values anyway), `mgw1o` (fsqlite 0.1.19 stale COUNT(*) —
upstream report owed), `export-temp-sidecar-orphans-gd0dm`, `pi-agent-missing-workspaces-le8s1`
(41 workspaces, 199 files, zero rows), `6t64c` (six branches with broken history), `2hrs`
(tantivy opens-but-spins corruption spike), plus the five stale May in_progress beads above.

The rest of the open set is aspiration, not defect: two epics ("Guided operations, repro
capsules, trust surfaces"; "Swarm coordination intelligence") with ~13 open feature/test
children.

## 3. The rescue effort, quantified

- **Commit shape of the whole project:** 4,261 commits since 2025-11-20. By month:
  225 / 318 / 702 / 485 / 329 / **1,281 (Apr)** / 805 (May) / **2 (Jun)** / **0 (Jul)** /
  114 (Aug). The project went dormant for two months while the source corpus kept growing —
  the archive crossed the size where the pinned engine stops finishing during exactly that gap.
- **Rescue window:** 2026-08-14 15:25 → 2026-08-17 06:37 ≈ **2.6 days**, **83 commits**
  (105 in the last 7 days; 92% of the month's 114).
- **Generations:** 13 numbered continuation generations across overlapping chains
  (cass-repair g1-2 on 08-14; 1a7mk g2-4 early 08-15; cass-to-green g2-g11 through 08-16/17;
  p3kgr g11-g13 overnight 08-16→08-17). **10 launch receipts** (autolaunched successor
  sessions), **32 lane logs** (24 + 8) across the two handoff dirs, ~1 MB of handoff artifacts.
- The chain stopped continuing itself only because of usage caps: the g11 closeout records the
  launcher refusing at **95.0% of the weekly window on account "george"** (`eefadadc`).
- Prior aborted attempt inside the window: the coverage-floor fix `e3ed01f0` deployed 08-10 and
  **rolled back the same day** (`667aeb49`) because it made health/triage/stats hang (1a7mk).

## 4. Declared fixed, then recurred — the stale-claims record

The handoffs themselves are unusually honest about this; the pattern is the finding.

1. **ibuuh.29.1** — closed 2026-04-22 with a proof test; the regression was re-introduced
   2026-05-13; the test "structurally cannot see it" (calls the producer directly with
   `planned_shard_plan = None`). Reopened; still open P0 today.
2. **8llb5** — fixed at `0f8c1541` (08-16 10:36); re-fixed the same day at `1c9c0cec`,
   commit subject: "the fix for this bead never reached the path the bead is about."
3. **1pzs3** — fixed at `89db6723` on `--watch-once`, closed in generation 10; the close was
   **wrong** (the archive is built by the full-scan path, which uses franken's connector, not
   the fixed wrapper); reopened and re-fixed at `9531315d`. g11's own words: "the same defect
   class has now landed three times in this repo."
4. **1a7mk** — closed 08-15 as fixed and deployed ("health, triage and stats hang"). The
   coordinator's ground truth TODAY: PATH `cass stats --json` ran >3.5 min to 5.2 GB RSS and
   never returned. Either the close did not hold on the deployed binary or the same symptom
   family has a second root (plausibly the fsqlite query path); either way the symptom the
   bead names is reproducing after close.
5. **qtn0e** — filed 08-14 as "cass is the sole surviving copy of 3,877 conversations";
   premise falsified (raw mirror holds a byte-complete blake3-verified copy); the destructive
   default was nonetheless real and fixed.
6. **kfaid** — closed with "layout was never the variable, format era is" — the original
   framing would have sent the next agent to directory globs.
7. **Drawdown model** — `ad7d8b07` ("fixed, not proportional") falsified within hours by
   `01a86794`: the model it was written to criticize (2.1x) "was the right one to plan on."
8. **Generation-11 artifact** carried two stale claims reconciled by `eefadadc`: "NOT the same
   defect as p3kgr" (contradicted by the fsqlite finding) and a free-disk figure of ~50 GiB
   against a measured 141 GiB.
9. **"Background jobs cannot push main"** — false, corrected on the bead (`73faacb1`).
10. **Toolchain "environment fact #1"** — every handoff in the chain named a nightly that is
    rustc 1.94 from December 2025 while 1.99 was installed the whole time; g12 calls this
    single unchecked inherited line "most of the whack-a-mole."
11. **CASS_PREP_PROFILE** — the recorded belief that `--json` suppressed it was wrong
    (`eprintln!`, not tracing), "why nobody trusted the instrument for weeks."
12. **The pin ceiling** (rustc 1.95 barrier on fsqlite 0.1.19) — "false in a second, dumber
    way" (g12); the RED pin verdict collapsed (`8e4e0241`).

## 5. Structural vs incidental

**Structural (inherent to the design; fixing instances does not retire the class):**

- **The frankensqlite pin.** fsqlite 0.1.5 (and 0.1.19) cannot plan an indexed GROUP BY:
  77 ms in stock SQLite vs 2h28m48s (0.1.19) / >10h (0.1.5) on the same 22 GB file. The
  query phase thrashes its pager (16m20s+ with no result — still unresolved, live right now
  as PID 75534). The engine also drags in: the ns-sidecar family that can brick a database on
  a lost mode bit (1mgjd, xybl9), FTS maintenance silently disabled (hd4u5), contentless FTS5
  storing content (d907f), stale COUNT(*) (mgw1o), and asupersync spin-wait deadlocks on the
  CLI startup path (759l7). Two P0s are "measurements of one defect" (g11's words). Upstream
  is unfixed in every release; a standalone repro exists but filing needs approval.
- **Dual connector paths.** cass re-exports franken's connectors AND maintains its own
  wrappers; `--watch-once`, full-scan, and streaming resolve connectors through different
  factories. This is the mechanism behind 8llb5-twice and 1pzs3-closed-wrong — three landings
  of one class. A fix on one path structurally misses the other.
- **Triple store + scale wall.** raw-mirror 46G + SQLite 22G + tantivy 9.5G = 77G to serve
  37.5G of source (2x amplification), on a disk at 29Gi free. The rebuild works at 12,722
  conversations and stopped working at 27,441; the corpus only grows. Napkin extrapolation
  for the codex backlog: ~12-15 h and 24-30 GB. The mirror's own prune deletes exactly the
  irreplaceable blobs and doctor recommends running it (pfar8) — a data-loss hazard built
  into the maintenance path.
- **The 400k-LOC surface as a verification problem.** sqlite.rs ~20k lines; citations in the
  handoffs run to `lib.rs:91682` and `indexer/mod.rs:20974`. A whole-corpus exact aggregation
  ran to refine an estimate that was overwritten on the next line, and survived from
  2026-05-13 to 2026-08-17 because the guarding test cannot reach the integration. The
  archive silently dropped 30% of message content per modern codex rollout (zero tool rows in
  6,452 conversations) at rc=0. The product's own stall detector fires during healthy runs.
  At this surface area, "green" and "working" are decoupled — which is the recurring theme of
  the whole ledger.

**Incidental (ordinary bugs; fixed, and the fixes look durable):**

- The honesty family — failed reads rendered as good news (nao4q, 0gzok, ddkwa, a59ou, sgvg3,
  xarzt, b6xc3) — all closed with mutant-checked tests.
- qtn0e's destructive default (`05dff6f4`), 9fnbr's silent-drop reporting (`1aa8172a`),
  the goldens host-drift (tutfy) and coverage-block goldens (a4xe1), the status --json
  raw-mirror walk bound (447d97fe / nvq59), the 1pzs3/8llb5 parser+progress fixes as landed.
- The guard inversion itself is a one-line incidental bug — but it is **unlanded**, and what
  made it survive three months (blind test, dual paths, engine 645x slower so the wrong guard
  cost hours instead of milliseconds) is all structural.

## 6. What the record claims WORKS now, with evidence quality

| Claim | Evidence quality |
|---|---|
| Full lexical rebuild completes in 57.4 s on the shipping pin (159 shards, 2,334,366 docs) | **Executed** — ledger log lines quoted in g13. But the fix is **not on main** (verified: symbol absent) and lives only in /tmp |
| Control rebuild (12,722 conv) completes ~18 s | Executed, twice, two sessions agreeing |
| 1pzs3 connector fix on the archive-building path; 18 conv / 4,156 msgs vs pre-fix 1/387, with positive control | Executed, with mutants M1-M3 red; landed `9531315d`, pushed |
| Exclude no longer destroys the archive by default | Executed test; landed `05dff6f4`; known surviving mutant disclosed as n62wn |
| Silent codex drops now surface in machine-readable output | Executed incl. delete-the-JSON-lines mutant; landed `1aa8172a` |
| Suite green: 5,150 tests, 0 failed, 3 ignored, 125.19 s | Executed (in the /tmp clone, not CI) |
| Raw mirror holds a byte-complete, blake3-verified copy of all 3,877 Claude conversations | Executed verification (0 missing) |
| Archive not corrupt; catch-up did not break anything | Executed — stock SQLite reads all tables in ms with correct counts |
| Watch-once targeted sweep works (28/28 batches rc=0, gen7) | Executed |
| **Search returns results end-to-end on production** | **NOT claimed by anyone.** 980 s with empty stdout then SIGTERM (`5d1718a3`, /tmp/rerun.log rc=143 confirmed this lane); a 4h48m+ probe still running today. g13's own title: "one blocker left in the query phase" |
| Index is complete and current | NOT claimed. g0eyv reindex owed; indexed_docs short 1,148 of messages rows; codex backlog ~12-15 h at measured throughput |

Bottom line the artifacts themselves support: after 13 generations, the diagnosis is finally
complete and correct (three named defects, each with an executed discriminator), one of the
three fixes is proven-but-unlanded, and the user-facing verb of the product — search — has
still never been demonstrated returning a result against the production archive.
