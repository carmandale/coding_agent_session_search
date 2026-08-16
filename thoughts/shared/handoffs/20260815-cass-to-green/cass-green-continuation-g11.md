---
generation: 11
parent-session: e6f96c37-dcde-4374-846b-964648d0bd0b
next-action-class: executable
---

# Continuation — 1pzs3 is fixed on the path that builds the archive, and the archive turns out to be missing most of its codex tool calls

## The goal and authorization, verbatim

Dale, 2026-08-14:

> /my-way fix cass to completion and 100% green working state and completely up to date or tell me why it can't or /grill-me with any questions.

Sent mid-work the same day, as a correction to the work in flight:

> make sure that you are looking at the recent (last 2 weeks) work on cass and not regressing

Dale, 2026-08-15:

> should we make a local fork of frankensqlite and fix it?

> your usage is good now. finish this to completion

Dale, 2026-08-16:

> give me the /tldr of what was wrong and where we stand today

And, the standing instruction that governs how you should work — Dale, 2026-08-16:

> if your senior dev recommendation is to delete those 4 stale directories do it. if that is what progresses us toward the goal. and I would prefer you just do that and only stop when it would break the pipeline or running sessions or if there is a true blocker or ambiguity or conflict rather than sitting here for something that I am going to just ask your recommendation on and agree to

**Read that last one as scoping your whole posture, not just the directories.**
Have the recommendation, act on it, report what you did. Stop only for: breaking
a running pipeline or session, a true blocker, real ambiguity, or a conflict.

**Destructive and external-write approvals expired with the ending session and do
NOT transfer.** You do NOT have approval to: delete any file in the repository,
delete anything under `/tmp` that you did not create yourself this session,
force-push, rewrite history, change repo visibility, file anything on the public
`Dicklesworthstone` repositories, or run `cass sources agents exclude`.
Clearing scratch directories **you created yourself this session** is inside the
§8 boundary and needs no approval; anything else does.

## The exact next action

**Run the ibuuh.29.1 probe. It is one command, it is decisive either way, and
the instrument it uses has never been run on this path.**

```bash
S=$(mktemp -d /tmp/cass-ibuuh-probe-XXXXXX)
CASS_SKIP_PREFLIGHT_CLEANUP_ORPHAN_FK_ROWS=1 CASS_PREP_PROFILE=1 \
  ~/.local/bin/cass index --force-rebuild \
    --db "$HOME/Library/Application Support/com.coding-agent-search.coding-agent-search/agent_search.db" \
    --data-dir "$S" 2>&1 | tee /tmp/ibuuh-probe.log
```

`--force-rebuild` **WITHOUT** `--full`. That takes the read-only canonical path,
which I verified in source rather than assuming: `should_try_readonly_canonical_force_rebuild`
(`src/indexer/mod.rs:2091-2101`) requires `force_rebuild && !full && !watch &&
!semantic && !build_hnsw` and no watch-once paths, and
`try_readonly_canonical_force_rebuild` then uses `FrankenStorage::open_readonly`
(:2112) and `close_without_checkpoint` (:2126). It rebuilds into the scratch
data dir, so it needs Tantivy index space only, not a 23 GB DB copy.

Read the stderr `CASS_PREP_PROFILE step=` lines:

- `open_readonly` → `prepare_db_state_deferred_fingerprint` → `load_checkpoint_state`
  **and then silence** means the wedge IS the whole-corpus `GROUP BY` at
  `src/storage/sqlite.rs:7501`, and ibuuh.29.1 owns it outright.
- **Zero lines at all** means the wedge is upstream in another preflight step and
  ibuuh.29.1 does not own it.

Two things to know before you run it. `CASS_PREP_PROFILE` writes with `eprintln!`,
not `tracing`, so no `EnvFilter` touches it — the recorded belief that `--json`
suppressed it is **wrong**, and that correction matters because it is why nobody
trusted the instrument. And there is real counter-evidence you must weigh:
`thoughts/shared/handoffs/20260814-cass-repair-to-green/lanes/backfill-falsifier.md:112-116`
records a full rebuild of this corpus at **12,438 ms** — 12 seconds for 12,722
conversations and 579,776 messages. If that rebuild went through the same prep
window, a whole-corpus GROUP BY cannot be costing 20+ minutes and the aggregate
is not the wedge. The ledger is written at `indexer/mod.rs:18182` and `:19212`,
both OUTSIDE `rebuild_tantivy_from_db_with_options`, so the 12s entry may come
from a different entry point. The probe settles it; reading more source will not.

Disk is at ~50 GiB against the 150 GiB floor, so watch free space during the run.

After that, **9fnbr's counting half**, which is the last open unit of the
original goal and is described below.

## What this session did

**Proved 1pzs3 on `--watch-once`, closed it, then found the close was wrong and
reopened it.** That correction is the headline, because the same defect class
has now landed three times in this repo.

Generation 10's stated pass criterion was scoped to `--watch-once`. It passed:
17 conversations and 2,859 messages against a pre-fix control's 0, with control
validity established by ancestry (`git merge-base --is-ancestor 2e069037
89db6723` → YES) rather than by assumption. But the archive is not built by
`--watch-once`.

```
indexer/mod.rs:20974  reindex_paths_with_semantic_delta -> create_connector()
indexer/mod.rs:20647  Self::Codex => Box::new(CodexConnector::new())
                      ^ this crate's wrapper, WITH the 89db6723 recovery

indexer/mod.rs:11532  run_streaming_index -> configured_connector_factories()
indexer/mod.rs:11770  run_batch_index    -> configured_connector_factories()
connectors/mod.rs:40  get_connector_factories <- re-exported from franken
                      ^ franken's connector, WITHOUT the recovery
```

`cass index` and `cass index --full` took the second. This is the exact mirror of
`1c9c0cec`, where the 8llb5 fix went to the full-scan path and missed
`--watch-once`.

Measured on the deployed binary, isolated HOME, fresh data dir per arm:

```
arm                                    conversations  messages  rc
cass index          BEFORE                    1           387    0
cass index          AFTER                    18          4156    0
cass index --watch-once (unchanged)          17          2859    0
```

The BEFORE arm is the positive control that makes it legible: it indexed exactly
the one modern rollout and none of the 17, so discovery, scan and ingest all
worked on that path — only the pre-envelope shape fell through, at rc=0.

**Fixed at `9531315d`**: `get_connector_factories` is no longer a bare re-export;
it substitutes the codex entry for this crate's wrapper. Blast radius is bounded
because every other connector module here is a 3-5 line `pub use` of franken's
implementation and codex is 1,058 lines — codex was the entire divergence.

**The bigger half, found by reconciling a message count that did not add up.**
The 17 recovered conversations are byte-identical on both paths. The MODERN
rollout is not:

```
path          binary     messages
watch-once    pre-fix        1297
watch-once    new            1297
full scan     pre-fix         387   <- 30% of the file, ZERO tool rows
full scan     new            1297
```

So the two paths disagreed about message-level completeness on **every** modern
codex rollout, not just the pre-envelope ones. **The live archive carries that
loss**: 6,452 of 10,283 codex conversations hold no `tool` message at all. Six
zero-tool ones were re-indexed against their still-present sources — four grew
2.7x-4.5x (2597→8639, 2591→7053, 2121→9579, 1514→4240) and two were unchanged at
0 tool rows, which is the control proving the reindex is not inflating
everything. Not duplication: the largest file holds exactly 2,954 `function_call`
records and the reindex produced exactly 2,954 `tool` messages against
production's 0, with the total under the file's 8,957 message-bearing bound.
**Filed as `g0eyv` — a reindex is owed.**

**Both unexamined P0s examined, and one had a falsified premise.**

- **`qtn0e`** — "cass is the sole surviving copy of 3,877 Claude Code
  conversations" is **false**. The raw mirror holds a byte-complete,
  blake3-verified second copy of all 3,877 (1.87 GB, 0 missing, uncompressed and
  unencrypted, `blob_size_bytes == source_size_bytes`), and
  `doctor_candidate_reconstruct_archive_from_raw_mirror` (`src/lib.rs:38568`) is
  a supported reconstruction path that triggers on exactly the condition an
  exclude creates. Rebuild was never a deletion path either — 4 `DELETE FROM
  conversations` sites, 3 test-only. What was still exactly true is that
  `cass sources agents exclude` deleted the archive by default with no guard of
  any kind. **Fixed this session** (see the landing note below).
- **`ibuuh.29.1`** — still true, and it BLOCKS the "100% green" half of the goal.
  `cass index --full --force-rebuild` has never completed on the live archive.
  "preparing" is not a phase; it is the catch-all `_` arm of a 3-value enum
  (`indexer/mod.rs:975-979`, duplicated at `lib.rs:79900-79904`), covering the
  whole rebuild prep with no transition until `phase.store(2)` after the producer
  spawn. Inside that serial window sits a whole-corpus `GROUP BY` over `messages`
  (`storage/sqlite.rs:7501`, reached from `indexer/mod.rs:18356`, 174 lines ahead
  of the first worker at :18530). It is a **regression introduced 2026-05-13**,
  three weeks AFTER this bead was closed on 2026-04-22, and the bead's own April
  proof test structurally cannot see it — it calls the producer directly with
  `planned_shard_plan = None`. It is NOT the same defect as p3kgr, which lives
  ~5,800 lines away in the run_index preflight.

**Both P0 findings were adversarially verified** by independent lanes that opened
every cited line. Both came back CONFIRMED, and the qtn0e verifier closed an open
question in the finding's favor by finding the reconstruction path above.

## What landed

- **`9531315d`** — the connector fix, pushed. Two tests pin the property rather
  than the mechanism: one builds the codex connector the way the archive-building
  scan builds it (out of `get_connector_factories()`) and asserts it recovers a
  pre-envelope rollout; the other pins why the substitution must exist and goes
  red if franken ever learns the shape. Mutants: M1 (remove the substitution —
  the shipped defect), M2 (match a nonexistent slug — the vacuous-guard shape)
  and M3 (substitute gemini) each turn the subject test RED while the franken
  control stays GREEN, so they are targeted.
- **`05dff6f4`** — the qtn0e default flip, pushed. `--keep-indexed-data`
  (opt-out) became `--purge-indexed-data` (opt-in), matching the sibling
  `cass sources remove --purge`. Retiring the old spelling rather than aliasing
  it is deliberate: an invocation passing `--keep-indexed-data` now fails to
  parse, which is the safe direction — it cannot silently be read as consent to
  delete. Its surviving mutant is disclosed in the commit message and filed as
  `n62wn`; do not treat that test as covering the branch.

**Suite, same clone and target dir throughout, so every delta is attributable:**

```
5146  generation 10 baseline
5148  + the two connector-factory tests      0 failed, 3 ignored, rc=0, 140.61s
5149  + the qtn0e exclude test               (verify this number yourself)
```

## State of the tree — READ THIS FIRST

This session wound down on a context threshold, not at a clean stopping point.
**Before anything else, run `git status --short` and `git log --oneline -5`.**

- If `src/lib.rs` is dirty and the last commit is `9531315d`, the qtn0e flip did
  NOT get committed. The commit message is written and waiting at
  `~/.claude-accounts/george/jobs/e6f96c37/tmp/commit-msg-qtn0e.txt` **but that
  job directory dies with the job** — if it is gone, the message content is
  reproduced in bead `qtn0e`'s comment and in this artifact. Verify the suite
  green yourself, then `git commit -F <msg> -- src/lib.rs && git push origin main`.
- If the last commit is the qtn0e one, it landed and there is nothing owed here.

`/tmp/cass-gen8/src/lib.rs` and the shared checkout's `src/lib.rs` were
byte-identical (md5 `bca9adfdfa10ee630c7119d3a2cefed2`) at the moment the patch
was applied.

**A trap in the clone:** `/tmp/cass-gen8` is detached at `1c9c0cec` and carries
276 uncommitted lines in `src/lib.rs` that are NOT yours — they are the
xarzt/b6xc3 work already landed on main as `3cda2531`; the clone was never synced
after that landing. `git diff` in the clone therefore shows changes that are
already on main. Do not sweep them into a commit. Because the two files are
otherwise identical, the reliable way to extract only your own change is:

```bash
diff -u --label a/src/lib.rs --label b/src/lib.rs \
  /Users/dalecarman/dev/coding_agent_session_search/src/lib.rs \
  /tmp/cass-gen8/src/lib.rs > /tmp/mychange.patch
```

Plain `diff -u` without the `--label` flags produces paths `git apply` cannot
strip correctly; it fails with `tmp/cass-gen8/src/lib.rs: No such file or directory`.

## Open, with what is known

- **`ibuuh.29.1` (P0)** — the probe above. Decisive either way.
- **`9fnbr` (P1) — the counting half, the last unit of the original goal.**
  `recover_rollouts_the_base_parser_dropped` (`src/connectors/codex.rs:99-138`)
  counts **nothing** today; its only output is three `warn!`s — a discovery
  failure (:108, which then `return Ok(())`, a swallowed failure), the SKIP
  (:123, per unrecoverable file) and the RECOVERY (:129, per rescued file). The
  two numbers the bead wants are how many times the `else` at :120-128 fires
  versus how many times `on_conversation` at :134 does. Under `--json` the filter
  is hard-coded to `error` (`src/lib.rs:5769-5774`, verified), so an operator sees
  `quarantined_conversations: 0` and `last_error: null` over real drops.
  Surface them beside `quarantined_conversations` on `IndexingStats`
  (`src/indexer/mod.rs:818`, serialized at :1172; the accumulate/saturating_add
  template is at :850-862 and :11491-11493).
  The investigating lane recommends **option (b)** — module-level atomics in
  `codex.rs` drained into `IndexingStats` at end of run, ~35 lines, no signature
  changes — because `create_connector` boxes as `Box<dyn Connector + Send>` and
  the factory is a bare `fn()` pointer, so there is no handle to hang an `Arc` on.
  Global mutable state needs a ceiling comment and care about test parallelism.
  **Now that `9531315d` landed, counters added there are reached by BOTH paths**,
  which was not true before — that was the lane's main worry and it is resolved.
  The test must pin that a `--json` run over a stub REPORTS the skip rather than
  only logging it.
- **`g0eyv` (P1, new)** — the codex tool-message loss and the reindex owed.
  Three questions it has to answer first: does incremental `cass index` re-read
  already-indexed files or does `last_scan_ts` skip them (if it skips, the loss
  is sticky); `--full --force-rebuild` is blocked by ibuuh.29.1; a targeted
  `--watch-once` sweep is the path known to work (gen7 ran 28/28 batches rc=0)
  and is the likely answer.
- **`pfar8` (P1, new)** — the inverse of qtn0e and genuinely dangerous.
  `cass mirror prune` pins only by recency and `--keep-tag`, never by upstream
  absence (`src/raw_mirror.rs:539-577`), so `cass mirror prune --older-than 90d
  --apply` deletes exactly the irreplaceable blobs — and cass's own doctor
  RECOMMENDS that prune once the mirror crosses 100 GB (`lib.rs:34879`, `:34376`,
  threshold at `:30539`). Mirror measured 46 GB. Smallest fix: pin any manifest
  whose `original_path` no longer exists.
- **`n62wn` (P2, new)** — the qtn0e test's surviving mutant, reported not hidden.
  Q1 (restore the destructive default) turns it RED; **Q2 (invert the branch back
  so a bare exclude purges) leaves it GREEN.** The test pins the flag at the parse
  boundary and is blind to the branch consuming it at `lib.rs:91682`. Catching Q2
  needs a behavioural test with a temp config and DB, or a subprocess test like
  `src/sources/probe.rs`'s `real_probe_*`.
- **`3azjb` (P1, new) — disk, and it needs Dale.** Free is ~50 GiB against the
  150 GiB floor. 113.85 GiB is reclaimable from five idle cargo targets
  (`cass-repair-target` 37.16, `cass-nvq59-target` 32.25, `cass-gen3-golden-target`
  18.81, `cass-c6bfb589-target` 18.22, `cass-lane-golden` 7.41), all with **0 open
  handles** by `lsof` and all 23h+ stale. `jck92` deliberately kept four of them
  on 2026-08-16 because "repair is in active use"; that ground has expired.
  **MUST NOT DELETE**: `/tmp/cass-gen8-target` (36.39 GiB, 40 open handles, the
  warm target), `/tmp/cass-fsqlite014-target` (4.59 GiB, written TODAY and named
  as the default `CARGO_TARGET_DIR` for OPEN P0 bead p3kgr at
  `thoughts/shared/handoffs/20260815-cass-to-green/verify-fsqlite-pin.sh:64`), and
  `/tmp/cass-gen8` (the source clone). **This is the one thing that needs Dale's
  express approval** — it is destructive and the approval did not transfer.
- **`p3kgr` (P0)** — the frankensqlite pin bump, refused on evidence by
  generation 8 and still refused. Do not land `worktree-cass-gen5-honesty`.

## Environment facts that cost real time

1. **Develop in `/tmp/cass-gen8`** with `CARGO_TARGET_DIR=/tmp/cass-gen8-target`.
   `Write`/`Edit` are refused in the shared checkout, and `EnterWorktree` is wrong
   here (§2.10). Land by patch (see the trap above). The same guard blocks `Write`
   for repo artifacts such as this handoff — write to `$CLAUDE_JOB_DIR/tmp` and
   `cp` it in; `cp` is not the Write tool.
2. Build needs nightly on PATH:
   `export PATH="$HOME/.rustup/toolchains/nightly-aarch64-apple-darwin/bin:$PATH"`.
3. Release build is **5m42s** warm. Wait on the `cargo` pid, not `rustc`, and
   check the binary's mtime before deploying.
4. **Deploy by atomic rename**, never `cp` over the live path — stale signature
   cache gives SIGKILL. Preserve the outgoing binary first. This session's
   preserved copy is `~/.local/bin/cass.pre-gen11-deploy-20260816-122921`.
5. **`--version` cannot distinguish a locally-built binary from its parent.** Both
   report the clone's HEAD (`1c9c0cec`) because the tree is dirty. Identify by
   `shasum` or mtime. This is the specimen trap and it is live here.
6. **Any acceptance arm needs a FRESH scratch data dir.** A reused one has already
   ingested everything, ingests nothing, and passes or fails for unrelated reasons.
7. **`CODEX_HOME` overrides Codex discovery** (`src/lib.rs:72842`). Setting `HOME`
   to a temp dir holding only `.codex/sessions/<files>` makes a real `cass index`
   full-scan cheap and isolated — that is how the full-scan defect was proven.
8. Indexing requires `CASS_SKIP_PREFLIGHT_CLEANUP_ORPHAN_FK_ROWS=1`.
9. Do not pass `--verbose` to an indexing run: DEBUG emits a line per SQL token.
10. `cargo test --lib` depends on the INSTALLED binary — `src/sources/probe.rs`'s
    `real_probe_*` tests shell out to `cass health --json`.
11. **`Cli::try_parse_from` overflows the default 2 MB test stack in a debug
    build.** Wrap any CLI parse test body in `run_on_large_stack(|| { ... })` like
    its neighbours, or it aborts with SIGABRT instead of failing an assertion.
12. `cargo test --lib -- <filter>` — the `--` is required; `cargo test --lib a b`
    fails with "unexpected argument".
13. `br` needs an explicit db. Use a function, not a variable:
    `brx() { br --db /Users/dalecarman/dev/coding_agent_session_search/.beads/beads.db "$@"; }`.
    Close by **full** bead id for the long slug ones. `br comments add <id> -m "..."`
    works for recording findings on an open bead.
14. No `timeout`/`gtimeout`, and foreground `sleep` is blocked. Use
    `run_in_background` with an `until` loop.
15. dcg blocks `rm -rf` even on your own `mktemp` scratch dir. Just don't clean up
    — it is not worth an override.

## Evidence

- `~/.cass-catchup/gen11-1pzs3-evidence/` — `FULLSCAN-FINDING.md` (the full-scan
  defect with its executed falsifier and positive control), `README.md` (the
  watch-once proof), `control-run.log`, `subject-run.log`, `fullscan-run.log`,
  `watchonce-isolated-run.log`, `fullscan-positive-control-run.log`,
  `paths-17.txt`, `census.txt`.
- `~/.cass-catchup/gen9-survey-preenv.py` — the archive census that names the 17.
- Beads `1pzs3`, `qtn0e` carry their measurements as comments; `g0eyv`, `pfar8`,
  `n62wn`, `3azjb` are new and carry theirs in their descriptions.
- Backup: `~/backups/cass/agent_search-20260814-vacuum.db`, 3.98 GB, verified.
