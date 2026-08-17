# Coordinator log — generation 14

Bead: `coding_agent_session_search-759l7`
Branch: `worktree-cass-759l7-spin-wait`
Session: `0faeab5e-ad5e-4491-b087-25103f2e4a10` (background job `0faeab5e`)
Parent: generation 13, session `0f9160b4-927c-47cf-89b4-ef92b18c63a4`

Assigned by the gen-13 handoff
(`20260816-759l7-spin-wait-gen14/p3kgr-upstream-continuation.md`, committed at
`3eab2195`). Entered through the resume-handoff skill's autolaunched direct
path: frontmatter carries `generation: 14`, a well-formed `parent-session`, and
`next-action-class: executable`; step-3 verification passed; so the confirmation
stop does not apply.

---

## What the previous run actually left behind

> **CORRECTION, written later the same session. The premise of this section was
> wrong, and the error is worth more than the section.** I read
> `wf_628b78dd-655`'s journal as a finished record. It was a **live file**. The
> workflow was still running while I read it, and it completed at 01:40:44Z with
> **all 10 agents returned and 0 errors** — including both lanes I describe below
> as missing. Everything I then spent a fan-out re-deriving already existed.
>
> The tell was in my own hands and I walked past it: the handoff said "4 of ~10
> had landed", I counted 6, and I explained the difference as "two more landed in
> the minutes after the handoff was written" — which is precisely the observation
> that the producer was still alive. A count that has moved since the last reader
> is not a late arrival, it is a running process. I treated a monotonically
> growing file as terminal because the session that launched it had ended, and a
> workflow does not stop when its launching session does.
>
> What it cost: two lanes launched at 20:32 and killed at 20:47, plus the
> coordination round with the live sibling that caught it. What it did not cost is
> anything downstream — no false claim reached `pin-move-cost.md`, because gen 13
> was writing that file, not me.
>
> The cheap check I did not run: `stat` the journal, wait, `stat` again. A file
> whose mtime is still advancing has a writer. The corrected verdicts are recorded
> at the end of this log; the table immediately below is left as written so the
> mistake stays legible, with each superseded row marked.

The gen-13 handoff said "4 of ~10 had landed". Read directly at 01:25Z, the
journal of `wf_628b78dd-655` showed **9 agents started and 6 results journaled**.
Recovering those 6 cost one `python3` pass and was worth doing; concluding
anything about the other 3 from the same read was the error.

Journal:
`~/.claude-accounts/george/projects/-Users-dalecarman-dev-coding-agent-session-search--claude-worktrees-cass-759l7-spin-wait/0f9160b4-927c-47cf-89b4-ef92b18c63a4/subagents/workflows/wf_628b78dd-655/`

The journal records `agentId` but **not the lane label**, so the mapping below
was recovered by reading each transcript's opening prompt for its `GROUP:` line.
Anyone re-reading this journal needs that step; the ids alone say nothing.

| agent id | lane | result journaled? |
|---|---|---|
| `a227e63c069c26189` | triage:salvage-counts | yes |
| `a71705d1019bb8d79` | triage:fts-shadow-table | yes |
| `a4ffab4b44162760e` | **triage:fts-repair-mode** | ~~no — killed mid-read~~ **SUPERSEDED: yes, 20,823 chars.** It was still running. |
| `a682c203879230414` | verify:dependency-drift | yes — `refuted: false` |
| `aae5b4997944048a3` | verify:encrypt | yes — `refuted: false` |
| `a001bd15f9797da52` | verify:salvage-counts | yes — `refuted: false` |
| `a31b385fc511fc699` | verify:fts-shadow-table (correctness lens) | yes — `refuted: false` |
| `ad0382db5cca84aea` | verify:fts-shadow-table (does-it-hold lens) | **no**, but recoverable — see below |
| `adf149551e76ab036` | verify:fts-shadow-table (reachability lens) | ~~no — killed mid-probe~~ **SUPERSEDED: yes, and it REFUTED.** |
| `a362b7771c0f754ef` | verify:fts-repair-mode (escalation lens) | **not in my read at all** — it had not started yet. `refuted: false`. |

### A result can survive its own workflow

`ad0382db5cca84aea` never reached the journal, but its **`StructuredOutput` tool
call is in its transcript**, complete. The agent finished its work and the
workflow died between the tool call and the journal write. So "no result line"
does not mean "no result": read the transcript's last `tool_use` before
concluding a lane produced nothing. That recovered a full three-lens verdict for
free.

`adf149551e76ab036` is the genuine loss — no `StructuredOutput`, cut off while
probing a `.corrupt` file and a V14-era backup on the Desktop.

---

## Lane declaration — this generation

Runtime: Claude Code `Workflow` tool, script persisted under the session
directory. Visibility: artifact-visible (each lane writes its own log below) plus
`/workflows`. Model: inherited (every lane here adjudicates or classifies —
none is a mechanical command-runner, so none is pinned down per AGENTS.md §3.9).

| lane id | purpose | log path | writes | stop condition |
|---|---|---|---|---|
| `triage-fts-repair-mode` | classify failures 7 and 8 (repair-mode change) | `lanes/triage-fts-repair-mode.md` | its own log only | returns a classification with `blocks_pin` |
| `verify-repair-correctness` | refute it as a correctness question | `lanes/verify-repair-correctness.md` | its own log only | returns `refuted` |
| `verify-repair-cost` | refute it as an efficiency/cost question on 23 GB | `lanes/verify-repair-cost.md` | its own log only | returns `refuted` |
| `verify-repair-holds` | re-derive the load-bearing fact independently | `lanes/verify-repair-holds.md` | its own log only | returns `refuted` |
| `verify-shadow-reachability` | the lens lost when gen 13 died | `lanes/verify-shadow-reachability.md` | its own log only | returns `refuted` |

**Forbidden to every lane:** any write outside its own assigned log path; any
`cargo build`/`cargo test` that would compile (disk is 75 GiB free against a
150 GiB floor); any write, rebuild, `cass` invocation, or non-`mode=ro&immutable=1`
open of the production database; `git` mutations of any kind; any change to
`Cargo.toml`, `Cargo.lock`, or `rust-toolchain.toml` in either tree.

---

## The instrument gen 13's lanes did not have

Gen 13's lanes were told, verbatim, that `/tmp/cass-759l7-forward/target` **had
been deleted**, that they could not run the forward tests, and that they must not
try. They classified from source alone.

That is no longer true, and it is the single biggest difference in this
generation. Measured this session, by content and never by mtime:

| | path | markers found by `strings` |
|---|---|---|
| forward | `/tmp/cass-759l7-forward-target/debug/build/coding-agent-search/b9364c709c6f41e6/out/coding_agent_search-b9364c709c6f41e6` | `fsqlite-core-0.1.19`, `asupersync-0.3.10` |
| shipping | `<worktree>/target/debug/deps/coding_agent_search-983a915ea0c0a592` | `fsqlite-core-0.1.5`, `asupersync-0.3.2` |

Both failures reproduce on demand, in under half a second each, with no build:

```
$ cd /tmp/cass-759l7-forward && "$FWD_BIN" --test-threads=1 \
    full_run_fallback_fts_repair_skips_rebuild_when_fts_is_already_healthy
  left: Some(Repaired(Rebuilt { inserted_rows: 4 }))
 right: Some(Repaired(AlreadyHealthy { rows: 4 }))
test result: FAILED. 0 passed; 1 failed; 5153 filtered out; finished in 0.47s

$ "$FWD_BIN" --test-threads=1 ensure_fts_consistency_via_rusqlite_catches_up_missing_rows
  left: Rebuilt { inserted_rows: 2 }
 right: IncrementalCatchUp { inserted_rows: 1, total_rows: 2 }
test result: FAILED. 0 passed; 1 failed; 5153 filtered out; finished in 0.32s
```

So this generation's lanes can execute rather than deduce. Note the target dir
is a **sibling** of the source tree, not a child — `/tmp/cass-759l7-forward-target`
beside `/tmp/cass-759l7-forward`. Reading `debug/deps` and finding nothing is the
wrong instrument: this cargo uses a build-dir layout, and that mistake already
cost the parent session a false "the tree is cold" conclusion.

---

## Verifier verdicts recovered from generation 13

All three that exist say the same thing: **nothing was refuted.**

### `verify:dependency-drift` → `refuted: false`

Classification `expected-artifact-of-the-experiment` stands. The test parses
cass's own `Cargo.toml` and asserts the version strings equal literals hardcoded
in the test itself; the experiment moved the manifest and left the literals, so
the failure is the test doing its job.

### `verify:encrypt` → `refuted: false`

Classification `toolchain-artifact-rustc-or-std` stands, settled by an executed
probe: `u8::try_from(256usize)` prints `out of range integral type conversion
attempted` under 1.94 and `number too large to fit in target type` under 1.99.
rustc 1.99 dropped `TryFromIntError`'s flat message and routed it onto the
`IntErrorKind` descriptions.

### `verify:salvage-counts` → `refuted: false`, and it sharpened two things

Both `compatible-library-behavior-change` and `blocks_pin: true` survive. The
verifier confirmed the mechanism independently — including checking the escape
hatch the finder never tested: there is **no `Drop` impl** in `namespace.rs`
(positive control: `rg -c 'impl Drop' pager.rs` returns 2), and
`cleanup_abandoned_private_database` is called only from `vacuum.rs:175` and
namespace's own `#[cfg(test)]` block. So a failed open really does leave a
permanent 40-byte file. Had cleanup existed on the error path the whole
off-by-one story would have collapsed.

Two corrections that matter, and both **increase** what the fix has to cover:

1. **The backup-retention dilution is confirmed, not speculative.**
   `pin-move-cost.md` files it as "reasoned from source and not executed; treat
   it as a follow-up to size, not a verified defect." The verifier traced every
   link and found the trigger is ordinary rather than exotic:
   `has_pending_historical_bundles` (`sqlite.rs:8344-8351`) calls discovery, and
   it runs on **ordinary indexer startup** (`indexer/mod.rs:12807`). So a normal
   run probes every `agent_search.db.backup.*` root and plants both sidecars
   beside it with fresh mtimes — no corruption required. `is_backup_root_name`
   (`2998-3000`) then counts each as a backup, and `cleanup_old_backups`
   (`1729-1755`) sorts newest-first and deletes past `MAX_BACKUPS = 3`. Two
   freshly-mtimed junk entries per probed backup can evict the user's real
   backups of a 23 GB database. **That is user data loss, not hygiene.**

2. **The two-string fix is incomplete.** Enumerating dash-prefixed literals in
   both trees, 0.1.19 adds three names 0.1.5 lacks: `-fsqlite-ns-gate`,
   `-fsqlite-ns-use`, and **`-wal-fec`**, plus a `<db>-wal-fec.wal-fec.tmp` and a
   `<db>-wal-seg-*` family (`namespace.rs:415-424, 663, 673`). None is in
   `has_db_sidecar_suffix`, and **`-wal-fec` does not end with `-wal`**, so the
   existing entry does not cover it. Two strings unblock the two tests; the
   durable fix covers the whole 0.1.19 companion set and shares one list with the
   retention path.

   Worth recording how that enumeration was nearly lost: the verifier's first
   attempt used `rg -oh`, got ripgrep's help text at exit 0, and caught it. That
   is the AGENTS.md §10 trap firing in the field.

### `verify:fts-shadow-table` — correctness lens → `refuted: false`

Verdict holds. The verifier re-ran the decisive probes rather than trusting the
reported ones, and confirmed `rootpage=0` is what stock SQLite writes (executed,
sqlite3 3.54.0: `CREATE VIRTUAL TABLE ... USING fts5(...)` then
`SELECT rootpage FROM sqlite_master` → `0`). It also pinned the mechanism by
elimination: `fts_messages_integrity_reports_missing_shadow_tables`
(`sqlite.rs:25248-25288`) already `expect_err`s on open for a *lone* rootpage-0
legacy row and is **not** in the 8-failure list, so 0.1.5 already validated
eagerly and the only thing that can differ is the mask.

One correction, and it moves risk **down**: the claim that such a bundle "can no
longer be opened, probed, or healed" is too strong on *probed*.
`probe_historical_bundle` (`2139-2141`) falls back to
`probe_historical_bundle_via_sqlite3_metadata` (`2180-2205`), which shells out to
real `sqlite3` with `PRAGMA writable_schema=ON` first — executed, that reads the
duplicate-row database fine. `sqlite3 .dump` also still emits the user's rows.
What is genuinely lost is the automatic in-place heal.

### `verify:fts-shadow-table` — does-it-hold lens → `refuted: false`

Recovered from the transcript's `StructuredOutput` call. Verdict holds on both
fields, and it added a fourth independent leg the finder did not have: the
enclosing function is named `read_fts5_rootpage_zero_content_rows_for_reload`,
and **every** call site in both versions is a rootpage-zero path (0.1.5:
`{55519, 55686}`; 0.1.19: `{62667, 62822, 12332}`). A rootpage>0 vtab cannot
reach the check. That converts the finder's deduction into a structural fact.

It also killed the strongest rival mechanism the finder never considered: if
virtual-table option parsing had changed, the *healthy* row rather than the
injected one would error and the whole story would invert. Diffed
`virtual_table_option_value` + `normalize_virtual_table_option_token`
(0.1.5:`76456-76485` vs 0.1.19:`86353-86382`): byte-identical.

Three corrections, all of which **shrink** the blast radius:

- **The finder's isolation probe does not replicate as described.** The real
  discriminator is the `IF NOT EXISTS` spelling, not the content option: a
  4-cell probe gives contentless+`IF NOT EXISTS` → accepted rc=0,
  contentless+bare `CREATE` → rejected, legacy+`IF NOT EXISTS` → accepted,
  legacy+bare → rejected. The conclusion survives (the shadow is irrelevant to
  real SQLite) but the mechanism was misstated, and the consequence matters:
  cass's own canonical SQL uses `IF NOT EXISTS`, so a real-world duplicate
  written that way would be **accepted by stock and still rejected by 0.1.19**.
  That weakens, without defeating, the argument against calling this a
  regression.
- **Healing is not lost for seeded bundles.**
  `scrub_staged_derived_fts_metadata_via_sqlite3` (`2479-2498`) is production
  code, not `#[cfg(test)]`, and `ensure_seeded_canonical_fts_consistency`
  (`2539-2570`) invokes it exactly when the frankensqlite open returns an FTS
  integrity error — and the 0.1.19 error qualifies, since
  `fts_messages_integrity_error_from_message` (`1248-1265`) matches on "shadow
  table" and "missing required", both present. So that path still self-heals.
- **The proposed fix is cheaper than assessed.** The finder rejected a pre-flight
  because "rusqlite is dev-only" and promoting C-SQLite would contradict stated
  policy. cass **already** shells out to the system `sqlite3` in two production
  paths for exactly this scenario (`2182`, `2482`). No new dependency needed.

Why it still blocks: `fts_messages_integrity_error_from_message` has exactly
three production consumers and only **one** repairs. `sqlite.rs:2543` is the
seeded-bundle scrub; `indexer/mod.rs:14723` and `lib.rs:14109` only reformat the
error for the operator. So on a **live** database carrying a duplicate row, 0.1.5
self-healed and 0.1.19 hard-fails.

### `verify:fts-shadow-table` — reachability lens → **lost**

No `StructuredOutput`. Re-run this generation as `verify-shadow-reachability`.
Its partial work reached two real specimens on disk worth handing forward: a
file cass itself named `.corrupt`, and
`~/Desktop/cass-backups-parked/agent_search.db.v14-backup-20260401-042413.parked`
— a V14-era backup from exactly the era of commit `e4796ba6`.
