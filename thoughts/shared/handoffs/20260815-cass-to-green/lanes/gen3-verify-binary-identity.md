# Lane: gen3-verify-binary-identity

Adversarial verifier for the `gen3-binary-identity` finding. Default posture: the
finding is wrong. Read-only; the only file I wrote is this log.

Started 2026-08-15T11:45:39Z. All timestamps UTC unless marked local (CDT = UTC-5).

---

## Verdict, up front

**The finding is NOT refuted. It survives, and I strengthened it.**

The installed binary contains e3ed01f0 (the coverage-floor feature) and does NOT
contain 8dcd245b (the hang fix). `backfill.sh:12` is false and its own
`run.log:4` contradicts it. I reproduced this with a better instrument than the
original lane used, and I found one hard discriminator they reported as absent.

Two qualifications on their consequence paragraph, and one deploy-safety fact
they did not examine, are in the sections at the bottom.

---

## Attack (b) first: did the specimen move under me?

The repo has a recorded failure where a differential compared a binary to itself
because a sibling session overwrote the "before" copy 110 seconds earlier
(`.claude/rules/instrument-labels.md`). With ~20 concurrent sessions this is the
resting state, so I measured the live binary three separate times across my lane.

| when | sha256 | size | mtime |
|---|---|---|---|
| 11:45:39Z (`shasum -a 256`) | `5b3344fd94f93cd4ba0357a4c2d5b9de5733ead94ab404ff0963fdec29d01644` | 51900992 | 2026-08-14T21:56:50Z |
| 11:47:15Z (python hashlib) | `5b3344fd94f93cd4ba0357a4c2d5b9de5733ead94ab404ff0963fdec29d01644` | 51900992 | 2026-08-14T21:56:50Z |
| 11:50:18Z (python hashlib) | `5b3344fd94f93cd4ba0357a4c2d5b9de5733ead94ab404ff0963fdec29d01644` | 51900992 | 2026-08-14T21:56:50Z |

Unchanged, and identical to what the finding reported. The specimen did not move.
`which -a cass` returns only `/Users/dalecarman/.local/bin/cass` (twice — one PATH
entry duplicated), so there is no second copy shadowing it.

Full specimen table, all re-measured by me at 11:47:15Z:

```
LIVE  ~/.local/bin/cass  sha256=5b3344fd94f93cd4  size=51900992  mtime=2026-08-14T21:56:50Z
nvq59-0814               sha256=5b3344fd94f93cd4  size=51900992  mtime=2026-08-14T21:56:50Z
target/release           sha256=5b3344fd94f93cd4  size=51900992  mtime=2026-08-14T21:54:41Z
covfix-0810              sha256=d0b860eb6a8ef366  size=51900976  mtime=2026-08-11T01:37:41Z
pre-cov-0601             sha256=3d04422759268c17  size=51834784  mtime=2026-06-01T11:21:13Z
```

The worktree's own `target/release/cass` is absent (`os error 2`) — confirmed.

---

## Attack (c): were the markers guessed or derived?

Derived. I re-derived them independently with the command
`.claude/rules/instrument-labels.md` prescribes, and the finding's markers appear
in my derived set.

```
git diff 8dcd245b^1 8dcd245b -- '*.rs' | rg '^\+' | rg -o '"[^"]{12,}"' | sort -u
```

returned 24 literals including `"Scan Coverage: UNKNOWN — the coverage read did
not complete"` and `"connector coverage read exceeded its bound; reporting
coverage as unchecked"`. Same command against `e3ed01f0^1 e3ed01f0` returned 40+
including `"recording connector scan coverage floor"`, `"Scan Coverage:
INCOMPLETE — these totals undercount the archive"` and `"widening connector scan
window back to its recorded coverage floor"`.

**One thing the original lane did not check, which I did.** Many of the derived
literals are test-only (`"no-floors.db"`, `"with-floor.db"`, `"read must
succeed"`, `"control must succeed, otherwise a None from the subject proves
nothing"`). A test-only literal is absent from a release binary regardless of
whether the feature is present, so using one as a subject marker would
manufacture a false negative. Both subject markers are production code:

- `src/lib.rs:24091` — inside `run_stats`, which opens at `src/lib.rs:23821`. The
  nearest preceding `#[cfg(test)]` is far above and closed; the surrounding lines
  are the ordinary `Totals:` / `Raw Mirror:` print block.
- `src/lib.rs:15184` — inside `read_connector_scan_floors_bounded`
  (`src/lib.rs:15161`), a `tracing::warn!` in the `RecvTimeoutError::Timeout` arm.

---

## Attack (a): can the instrument actually fire for the subject's shape?

This is where the original lane's instrument was weakest, and where I rebuilt it.

Their positive controls were `last_scan_ts` (15) and `fts_messages_content` (5),
which fire in **all five** binaries. Those prove the byte reader works. They do
**not** prove that a *feature-added literal carried by the subject's macro shape*
would be found — which is exactly the claim a zero count rests on. A control that
cannot distinguish present from absent is not a control for a differential.

So I built shape-matched **differential** controls: literals added by e3ed01f0,
carried by the same macros as the 8dcd245b subject markers, measured across a
binary that has the feature and one that does not.

Byte-occurrence counts (`bytes.count`) over each whole file, measured 11:47:15Z:

```
marker                                          |  LIVE | nvq59 | targrel | covfix0810 | pre0601
------------------------------------------------|-------|-------|---------|------------|--------
NEGCTL  zzz-nonexistent-marker-9931             |     0 |     0 |       0 |          0 |       0
POSCTL  last_scan_ts                            |    15 |    15 |      15 |         15 |      15
POSCTL  fts_messages_content                    |     5 |     5 |       5 |          5 |       5
SHAPECTL println! e3ed 'Scan Coverage: INCOMPLETE'      | 1 | 1 | 1 | 1 | 0
SHAPECTL println! e3ed 'Scan Coverage: complete (no ...)'| 1 | 1 | 1 | 1 | 0
SHAPECTL info!   e3ed 'widening connector scan window'   | 1 | 1 | 1 | 1 | 0
SHAPECTL arg     e3ed 'recording connector scan coverage'| 1 | 1 | 1 | 1 | 0
SHAPECTL e3ed 'connector_scan_coverage_floor_cleared'    | 1 | 1 | 1 | 1 | 0
SHAPECTL e3ed 'scan aborted; unproven from'              | 1 | 1 | 1 | 1 | 0
SUBJECT println! 8dcd 'Scan Coverage: UNKNOWN'           | 0 | 0 | 0 | 0 | 0
SUBJECT println! 8dcd 'These totals may undercount ...'  | 0 | 0 | 0 | 0 | 0
SUBJECT warn!    8dcd 'connector coverage read exceeded its bound' | 0 | 0 | 0 | 0 | 0
SUBJECT warn!    8dcd 'reporting coverage as unchecked'  | 0 | 0 | 0 | 0 | 0
SUBJECT 8dcd "cass doctor' to check the database"        | 0 | 0 | 0 | 0 | 0
```

The `println!` control is as tight as it gets without building: `Scan Coverage:
INCOMPLETE` (`src/lib.rs:24074`) and `Scan Coverage: UNKNOWN` (`src/lib.rs:24091`)
are **seventeen lines apart in the same function**, same macro, same prefix. One
counts 1 in the live binary, the other counts 0.

### The `tracing::warn!` control, which is the decisive one

Both subject `warn!` markers count 0, so I needed proof that a `warn!` literal
lands contiguously in a release build at all. e3ed01f0 created a `warn!` whose
message 8dcd245b later *extended* at a different call site, which gives a clean
pair:

- `src/indexer/mod.rs:10715` (e3ed01f0, unchanged since) —
  `tracing::warn!(error = %error, "could not read connector scan coverage floors")`
- `src/lib.rs:15125` (8dcd245b) — same text plus
  `"; reporting coverage as unchecked"`

```
LIVE        {'e3ed warn! base': 1, '8dcd warn! suffixed': 0}
covfix0810  {'e3ed warn! base': 1, '8dcd warn! suffixed': 0}
pre0601     {'e3ed warn! base': 0, '8dcd warn! suffixed': 0}
```

A `warn!` literal appears when the code is there and is absent when it is not.
The instrument fires for the subject's shape. The zeros are real.

**An empty result I had to explain first.** `git show e3ed01f0:src/lib.rs | rg
'could not read connector scan coverage floors'` returned nothing, and
`git show 447d97fe:src/lib.rs` likewise. That is not absence: at those commits the
string lives in `src/indexer/mod.rs`, not `lib.rs`. `git grep -n <str> e3ed01f0 --
'*.rs'` → `e3ed01f0:src/indexer/mod.rs:10718`; at 447d97fe → `:10715`; at HEAD it
appears **twice** (indexer 10715, plus lib.rs 15125 with the new suffix). I was
searching the wrong file.

### Source-side confirmation

`git show 447d97fe:src/lib.rs | rg -c <marker>` returns 0 (rc=1, rg's genuine
no-match) for all four of `Scan Coverage: UNKNOWN`, `exceeded its bound`,
`reporting coverage as unchecked`, `may undercount the archive`. The 8dcd245b
strings postdate the commit the binary identifies as.

---

## What I found that the original lane reported as absent

They wrote: *"no string discriminator exists for 447d97fe; its identity rests on
sha256 equality with the dated specimen instead."*

That is an under-claim. The binary embeds the **full 40-character commit SHA**:

```
LIVE        {'447d97fe(full)': 1, 'e3ed01f0(full)': 0, '8dcd245b(full)': 0}
nvq59       {'447d97fe(full)': 1, ...}
targrel     {'447d97fe(full)': 1, ...}
covfix0810  {'447d97fe(full)': 0, ...}
pre0601     {'447d97fe(full)': 0, ...}
```

`447d97fe60962d1ed1f34841e508f61a6b4302c4` occurs exactly once in the three
matching binaries and zero times in the two older ones. Mechanism: `build.rs:701
emit_vergen_metadata()` → `GixBuilder::default().sha(false).build()` (build.rs:713),
which emits `VERGEN_GIT_SHA`. Commit `f619a74d fix(build): embed git revision in
cass identity` added it.

**Caveat I am obliged to state.** That builder sets only `.sha(false)`; it does
not request a dirty flag, so the embedded SHA proves which commit the build tree
was *at*, not that the tree was *clean*. A build from 447d97fe plus uncommitted
changes would carry the same SHA. This does not move the conclusion, because the
marker matrix independently rules out 8dcd245b content whether or not the tree was
dirty — but the SHA alone is not a completeness proof.

Ancestry, run by me:
```
git merge-base --is-ancestor e3ed01f0 447d97fe   → rc=0
git merge-base --is-ancestor 447d97fe 8dcd245b   → rc=0
git merge-base --is-ancestor 8dcd245b HEAD       → rc=0
```

---

## The backfill.sh contradiction, verbatim

`/private/tmp/claude-501/-Users-dalecarman--agent-config/a91c2501-1830-4d3d-9430-3c9afe08a63c/scratchpad/backfill.sh`

```
12  # Runs on the installed PRE-FIX binary. A HEAD build would reintroduce the
13  # coverage-floor regression (bead 1a7mk).
...
17  BIN=/Users/dalecarman/.local/bin/cass
...
28    echo "binary  : $(shasum -a 256 "$BIN" | cut -c1-16)"
```

`backfill/run.log` line 4:

```
binary  : 5b3344fd94f93cd4
```

The script's own provenance line records the post-fix build. Line 12 is false, and
its direction is inverted: the coverage regression is what is installed, and a
HEAD build is what removes it.

I reproduced the original lane's explained null too: `connector_scan_floors`
counts 0 in the live binary while `or_scan_floors` counts 1, at offset
`0x2882e00`, surrounded by `\x04\x00\x00\x00\x00\x00\x00\x00or_scan_floors\x00...\x80`
— consistent with a split/inline string representation rather than one contiguous
literal. Their hedge ("I did not identify which type produces that layout") is the
honest form and I am not improving on it. The null is genuine and that marker is a
dead discriminator on every release binary.

---

## Deploy safety — the part the original lane did not examine

### The backfill re-invokes the binary once per batch

`backfill.sh:33-37`:

```
33  for b in "$BF"/batch-*; do
36    "$BIN" index --watch-once "$(paste -sd, "$b")" \
```

Twenty batches, one process each. The path `~/.local/bin/cass` is resolved fresh
every iteration. Process tree confirms it:

```
pid=58404 ppid=4751 cmd=/Users/dalecarman/.local/bin/cass index --watch-once ...
pid=4751  ppid=1    cmd=bash .../scratchpad/backfill.sh
```

So a deploy mid-run does not affect the batch already running (an atomic rename
leaves the running process on its original inode) but **every later batch runs the
new binary**.

### Progress at 11:47Z

`run.log`: batches `aa` through `aj` all `END rc=0`, conversation count climbing
12934 → 15179. `batch-ak START 2026-08-15T11:46:19Z`. Ten of twenty done, roughly
4–8 minutes each; `backfill.sh:5` says the single 2.57 GB rollout lands last, so
the tail batch will be longer than the mean.

### Would a mid-run swap corrupt anything?

Probably not, and I can say why rather than guessing. `git show --stat 8dcd245b`
is `src/lib.rs | 253 ++++---`, **1 file changed**. Every hunk lands in:

```
@@ ... const CLI_DIAG_DB_OPEN_TIMEOUT
@@ ... fn read_connector_scan_floors
@@ ... fn probe_state_db
@@ ... fn index_orphan_fk_cleanup_cli_error   (a new #[cfg(test)] module, +144)
@@ ... fn run_stats                            (x3)
```

Callers are `probe_state_db` (15368), `run_stats` (23970), and a health surface
(65743). None is on the `cass index` write path, and the indexer's own copy in
`src/indexer/mod.rs` is untouched. So the change is confined to how CLI surfaces
*read and report* coverage.

The real cost of a mid-run swap is provenance, not corruption: `run.log:4` records
one binary SHA for the whole run and would silently become false for batches
`al`–`at`. Waiting for 20/20 is still the right call, for that reason.

`cp` over the live path would be the dangerous move — it writes through the
existing inode and would corrupt PID 58404. Atomic rename (already task #6) is
correct.

### Two qualifications on "a HEAD build is what removes it"

**(a) HEAD is 8dcd245b, for this purpose.** `git diff --stat 8dcd245b HEAD --
src/ tests/` returns empty. There is no source delta between the fix commit and
HEAD, so a HEAD build carries exactly the fix and nothing else. This supports the
original lane rather than qualifying it.

**(b) 8dcd245b fixed two of three copies.** `read_connector_scan_floors_fresh` at
`src/indexer/mod.rs:10696` still collapses a failed read to an empty map
(`storage.get_connector_scan_floors().unwrap_or_else(|error| { warn; BTreeMap::new() })`,
lines 10712-10718) — the same Fix-B shape. But state the consequence precisely:
that map feeds the indexer's scan-window *widening*, not a reporting surface, so a
failed read there causes under-scanning, not a false "complete" report, and it is
not the hang. It is pre-existing e3ed01f0-era code, not something 8dcd245b
introduced. Task #1 in the session list already owns it. Deploying HEAD is still a
strict improvement over what is installed.

---

## What I did NOT run — no first-hand behavioural evidence

- **No `cass` invocation of any kind.** No `health`, `status --json`, `stats`,
  `triage`, `index`, `doctor`. A backfill is in flight and those are the exact
  surfaces bead 1a7mk says do not return. I have **no first-hand measurement of
  the hang**; the 150s figure is cited from commit b0780df0's message and from
  `8dcd245b`'s own commit body, not measured by me.
- **No build, no `cargo check`, no tests.** My task did not authorize them. So I
  cannot state whether HEAD compiles or whether the suite is green.
- **Known red test I did not verify.** Commit `5e4fcbb9 beads(nvq59,a4xe1)` says
  `golden_robot_json` has been red since e3ed01f0 (bead a4xe1), and task #3 is
  pending. I read that from the commit subject; I did not run it. That is a
  separate gate on deploying HEAD and it is not resolved by anything in this lane.
- **No `br show`.** Bead text was read from commit messages in this worktree.

---

## Bottom line

| question | answer |
|---|---|
| Is the verdict about what is installed correct? | **Yes.** e3ed01f0 present, 8dcd245b absent, under six shape-matched differential controls and a negative control. |
| Did the specimen move? | No — three measurements, 11:45/11:47/11:50Z, identical. |
| Were the markers guessed? | No — re-derived from both diffs; subject markers verified to be production code, not `#[cfg(test)]`. |
| Could the instrument fire? | Yes, proven for `println!`, `tracing::warn!` and `tracing::info!` shapes, each present in a feature-carrying binary and absent in the pre-feature one. |
| Is it safe to deploy over it after the backfill finishes? | **Yes, by atomic rename, after 20/20 — subject to the a4xe1 red test and a build I did not run.** |
