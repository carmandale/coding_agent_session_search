# fsqlite/frankensqlite claim verification — 2026-08-15

Read-only lane. Repo root:
/Users/dalecarman/dev/coding_agent_session_search/.claude/worktrees/cass-to-green-c6bfb589

Wrote only this file. Ran no `cass index`/`cass sources`, no writes to the live
DB, no git mutations, no long builds.

## Baseline: what cass actually pins

`Cargo.toml:45`:
```
frankensqlite = { version = "0.1.5", package = "fsqlite", features = ["fts5"] }
```
`Cargo.toml:181`:
```
fsqlite-types = { version = "0.1.5", package = "fsqlite-types" }
```
`Cargo.lock` (resolved), block starting at line 2270:
```
[[package]]
name = "fsqlite"
version = "0.1.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "5415050ff8a232b55f774a3fc9267164f5ad010c806899772c4e06f0d8988dc0"
```
Confirmed: cass pins exactly **0.1.5**, both requested and resolved. The prior
session's "0.1.5" baseline is correct, not assumed.

`/data/projects/frankensqlite` does NOT exist on this machine — checked with
`ls`, got `No such file or directory`. AGENTS.md's mention of that path does
not apply here; no local frankensqlite git checkout to search.

## Cargo registry cache: which fsqlite versions are actually present

`ls ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/ | rg -i fsqlite`
and the matching `cache/*.crate` archives both show the same two version
families present for every `fsqlite-*` sub-crate: **0.1.5** and **0.1.17**,
with one exception — `fsqlite-vfs` is present at **0.1.6** and 0.1.17, not
0.1.5 (see the version-gap note under claim 3 below; this is a real gap in
the umbrella crate's own history, not a caching artifact). No other version
(and specifically no 0.1.11) is present in the local cache. Verifying the
0.1.11 fix-landed claim therefore required network access, not just the
local cache — see claim 1.

## Claim 2 — "ExistsValueSet appears 0 times in 0.1.5 and 8 times in 0.1.17"

**CONFIRMED EXACTLY.**

Ran `rg -c "ExistsValueSet" <dir>` one crate-dir at a time (batched `rg -c`
across a variable-held glob was refused by the harness as "too complex to
verify worktree isolation"; individual absolute paths worked fine) against
every `fsqlite*-0.1.5` and `fsqlite*-0.1.17` source dir in
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`:

- **0.1.5**: zero matches in every one of the 19 crate dirs (fsqlite,
  fsqlite-ast, fsqlite-btree, fsqlite-core, fsqlite-error, fsqlite-ext-fts5,
  fsqlite-ext-icu, fsqlite-ext-json, fsqlite-ext-misc, fsqlite-ext-rtree,
  fsqlite-func, fsqlite-mvcc, fsqlite-observability, fsqlite-pager,
  fsqlite-parser, fsqlite-planner, fsqlite-types, fsqlite-vdbe, fsqlite-wal).
  Positive control: `rg -c "fn " fsqlite-core-0.1.5` returned 87 matching
  files, proving `rg` does search this directory tree and a genuine hit
  would have printed — the 0 is real, not a dead instrument.
- **0.1.17**: exactly one file matches, everywhere else zero:
  `fsqlite-core-0.1.17/src/connection.rs:8` (rg -c file:count format — 8
  occurrences in that one file). Total across all 0.1.17 crate dirs: **8**.

Line numbers inside `fsqlite-core-0.1.17/src/connection.rs`: 53246, 81348,
81358, 81386, 81451, 81490, 81513, 81533 — a `struct ExistsValueSet`
definition, an `impl ExistsValueSet`, a `HashMap<(i32, usize),
ExistsValueSet>` field, and its unit tests. This is a genuinely new type
introduced between 0.1.5 and 0.1.17, not a rename of something older.

## Claim 1 — "Issue #117 names correlated_exists_fallback, fixed in fsqlite 0.1.11, six releases past 0.1.5"

**Substance CONFIRMED via GitHub/crates.io API (network reachable — `gh api`
and `curl` both worked). One precise correction on "six releases."**

`gh api repos/Dicklesworthstone/frankensqlite/issues/117`:
- Title: "Correlated `NOT EXISTS` (anti-join idiom) routes to the in-memory
  interpreter — 0.01s on canonical SQLite becomes ~5s and climbs (verified at
  `main` @ d1caefb5)"
- State: closed, `state_reason: completed`, created 2026-06-18T17:06:50Z,
  closed 2026-06-19T18:07:40Z.
- Body cites the exact same code path cass hit: `connection.rs:23602 else if
  select_has_correlated_exists_in_where(select)` →
  `:23609 log_mem_execution_fallback("select", "correlated_exists_fallback")`
  → `execute_join_select`. This is the identical string, confirmed — not
  just a name match, the reporter quoted the exact source lines.

`gh api repos/Dicklesworthstone/frankensqlite/issues/117/comments` — closing
comment verbatim: **"Fixed in 64c45de0, shipping in 0.1.11."** — followed by
a description of the fix: an `ExistsValueSet` memo (O(1) lookup), armed by a
depth-scoped RAII guard, mirroring `values_equal_sqlite` for bit-identical
results.

`gh api repos/Dicklesworthstone/frankensqlite/commits/64c45de0`:
- Message: `fix(core): O(1) memo for correlated NOT EXISTS/EXISTS probe
  (GH#117)`
- Files: `crates/fsqlite-core/src/connection.rs` (+267), `crates/fsqlite-e2e/
  tests/issue_117_correlated_not_exists.rs` (+212, new file)
- Commit date: 2026-06-19T02:08:42Z (same day as the issue's close).

`crates.io` versions API confirms `0.1.11` published at
`2026-06-19T18:16:12.927034Z` (fsqlite-vfs sibling crate: 18:13:37Z) — minutes
after the issue closed, consistent with "shipping in 0.1.11."

**Local-source corroboration, independent of the GitHub text**: I read the
actual branch in both cached versions myself rather than trusting the issue
report.
- `fsqlite-core-0.1.5/src/connection.rs:22653`: `self.
  log_mem_execution_fallback("select", "correlated_exists_fallback")?;`
  inside `} else if select_has_correlated_exists_in_where(select) {`, whose
  body then calls the generic `execute_join_select(&bound, None)` — a plain
  join-select fallback, matching the issue's claimed O(outer×child) shape.
- `fsqlite-core-0.1.17/src/connection.rs:24380`: the **same log string**
  still appears — `self.log_mem_execution_fallback("select",
  "correlated_exists_fallback")?;` — now inside `} else if self.
  select_correlated_exists_where_requires_fallback(select) {`, and the body
  calls a **new**, dedicated function, `self.
  execute_correlated_subquery_where_fallback(cx, select, params)`, not the
  generic join-select. This matches the fix commit's own description
  exactly: the fix memoizes the per-row child scan into `ExistsValueSet`
  rather than removing the fallback route, so the log string and the routing
  decision both persist — only the cost of what happens once inside the
  fallback changed. **Do not read the string's continued presence in 0.1.17
  as evidence the defect wasn't fixed** — that would be exactly the mistake
  this lane exists to catch; the commit and the source agree the fix is a
  cost fix, not a rerouting.

**Correction — "six releases past 0.1.5" is off by one for the crate cass
actually depends on.** crates.io's full `fsqlite` (the umbrella package
cass's Cargo.toml names) version history, sorted:
```
0.1.0 .. 0.1.4, 0.1.5 (2026-05-28T00:42:31Z), 0.1.7, 0.1.8, 0.1.9, 0.1.10,
0.1.11 (2026-06-19T18:16:12Z), 0.1.12 ...
```
**0.1.6 was never published for the `fsqlite` package.** It WAS published for
the `fsqlite-vfs` sub-crate (`crates.io/api/v1/crates/fsqlite-vfs/versions`:
0.1.6 at 2026-05-28T00:42:04Z, 27 seconds before fsqlite's own 0.1.5) — the
workspace bumped every crate's version field to 0.1.6 together, but the
top-level `fsqlite` publish at that number apparently did not go out (or was
skipped), and the next `fsqlite` release is numbered 0.1.7. So counting
*published* `fsqlite` releases strictly newer than 0.1.5 up to and including
0.1.11: 0.1.7, 0.1.8, 0.1.9, 0.1.10, 0.1.11 — **five**, not six. If you count
by version-NUMBER distance (0.1.6 through 0.1.11 is six numbers), "six" reads
naturally because the workspace's shared version counter did pass through
all six numbers — just not as six separate publishes of the specific crate
cass pins. Report this as a minor imprecision in the previous session's
count, not a defect in the underlying claim (issue exists, fix commit exists,
0.1.11 is real and is where it shipped).

## Claim 3 — "23 releases since February, 0.3.1 published 2026-08-14"

**CONFIRMED, exactly, via network.**

`curl -H "User-Agent: ..." https://crates.io/api/v1/crates/fsqlite`:
```
"created_at": "2026-02-21T23:43:07.596138Z"
"num_versions": 23
"max_version": "0.3.1"
"default_version": "0.3.1"
```
Full version list from `.../fsqlite/versions` (23 entries, first-published
timestamps), sorted by semver — first is 0.1.0 at 2026-02-21T23:43:07Z, last
is 0.3.1 at 2026-08-14T05:16:30Z. `gh api
repos/Dicklesworthstone/frankensqlite/releases` shows the matching GitHub
Release object: `v0.3.1` `published_at: 2026-08-14T05:26:50Z` (10 minutes
after the crates.io publish, consistent with tag-then-release-notes
ordering). `git tag` count on the repo (`gh api .../tags --paginate`) is 18,
not 23 — GitHub tags undercounts crates.io releases here because 0.1.0
through 0.1.4 and 0.1.5/0.1.6 have no corresponding tags in the list returned
(only v0.1.7 upward, plus two unrelated `jsm-v0.3.x-fsqlite-snapshot` tags
that are not fsqlite version releases at all). **Use crates.io's own
`num_versions` (23) as the citable count, not `git tag`.** "23 releases since
February, 0.3.1 published 2026-08-14" is accurate as stated. Today is
2026-08-15, so 0.3.1 (yesterday) is current evidence of active maintenance,
not stale.

## Claim 4 — "parity_cert default-on, disable-able, doesn't change routing, triggers full in-memory hydration"

**CONFIRMED on every sub-clause, all four independently, from
`fsqlite-core-0.1.5/src/connection.rs`.**

1. **Default-on**: constructor sets `reject_mem_fallback: RefCell::new(true)`
   (two call sites, :9105 and :8798, matching comment at :9100-9104: "bd-
   zjisk.1: Default to parity-cert mode"). Unit test
   `test_zjisk1_memory_conn_parity_cert_default_on` and
   `test_zjisk1_pragma_parity_cert_query` (`PRAGMA fsqlite.parity_cert;`
   with no value returns 1, comment: `"default parity_cert must be 1 (ON)"`).

2. **Consumer-disable-able**: `pub fn set_reject_mem_fallback(&self, reject:
   bool)` at :11058, and the PRAGMA handler at :43704 —
   `"fsqlite.parity_cert" | "parity_cert" => { ... *self.
   reject_mem_fallback.borrow_mut() = enabled; ... }` — accepts `PRAGMA
   fsqlite.parity_cert = OFF`.

3. **Does NOT change routing for this fallback shape**: the guard that
   selects the `correlated_exists_fallback` branch,
   `select_has_correlated_exists_in_where(select)` (0.1.5) /
   `self.select_correlated_exists_where_requires_fallback(select)` (0.1.17),
   is purely a function of the parsed SQL shape — neither version's
   condition reads `reject_mem_fallback` or any parity_cert field at all. I
   read the branch bodies directly (see claim 1 excerpts above) rather than
   trusting the GitHub issue's own assertion of this ("PRAGMA
   fsqlite.parity_cert = OFF only changes the log level for this shape"),
   and the source confirms it independently: nothing in that `else if` arm
   or its guard touches `reject_mem_fallback`.

4. **Triggers full in-memory hydration**: exact mechanism, read end to end.
   - PRAGMA handler (:43704-43714), when disabling on a file-backed
     connection that hasn't loaded memdb rows yet:
     ```
     *self.reject_mem_fallback.borrow_mut() = enabled;
     if !enabled && self.path != ":memory:" && !self.memdb_rows_loaded.get() {
         let cx = self.op_cx()?;
         self.reload_memdb_from_pager_with_mode(&cx, true)?;
     }
     ```
     — an immediate, synchronous, eager reload with `hydrate_rows = true`.
   - `should_eagerly_hydrate_memdb_rows()` (:12005-12006): `self.path ==
     ":memory:" || !*self.reject_mem_fallback.borrow()` — once parity_cert
     is off, every subsequent path that consults this also hydrates eagerly,
     not just the one-time PRAGMA-set call.
   - `reload_memdb_from_pager_with_mode(cx, hydrate_rows=true)` (:55342) →
     `reload_memdb_from_txn_with_mode` → (traced into
     `reload_memdb_rows_from_txn_preserving_schema`, :55357 on): for **every
     table in the schema**, opens a b-tree cursor at its root page,
     `cursor.first(cx)?`, then loops `cursor.rowid_and_payload_cow(cx)?` and
     `parse_record(...)`, inserting every row into a cloned in-memory
     `MemDatabase`. This is a full linear walk of every row in every table —
     literally "full in-memory hydration of the database," not a metaphor or
     a guess.

   Not executed as a live timing probe (would require a build and a run
   against real or synthetic data, out of scope for a read-only lane, and
   the live DB must not be touched while the backfill runs) — the "strictly
   worse" characterization is a direct, correct read of what the code does
   on disable, not a benchmark result. Flagging that boundary rather than
   overclaiming it as measured.

   Live DB size for context (stat only, no write):
   `ls -la ".../agent_search.db"` → **7.96 GB** as of 2026-08-15 06:02 (the
   file the backfill is actively growing). The claim's "7.4 GB" was
   presumably measured at an earlier point in the same backfill run; the
   database is real, multi-GB, and growing, so the substance of "strictly
   worse to hydrate all of it into memory" holds regardless of which exact
   snapshot size is cited.

## Summary table

| # | Claim | Verdict | Key correction |
|---|---|---|---|
| 1 | Issue #117 = same fallback string, fixed in fsqlite 0.1.11, six releases past 0.1.5 | CONFIRMED (issue, commit, version all verified via `gh api`/`crates.io`) | "six releases" → 5 published `fsqlite`-package releases between 0.1.5 and 0.1.11 (0.1.6 exists only for the `fsqlite-vfs` sub-crate, not the umbrella `fsqlite` crate cass pins) |
| 2 | ExistsValueSet: 0 in 0.1.5, 8 in 0.1.17 | CONFIRMED EXACTLY | none |
| 3 | 23 releases since February, 0.3.1 published 2026-08-14 | CONFIRMED EXACTLY | none (git tag count is 18 and undercounts; crates.io num_versions=23 is the right instrument) |
| 4 | parity_cert default-on, disable-able, no routing change, triggers full hydration | CONFIRMED on every sub-clause, read at the source-line level | "strictly worse" is a correct code-level read, not an executed benchmark |

All four claims hold up. The one number that needed adjustment (six→five
releases) does not touch the load-bearing part of claim 1 — the issue, the
fix commit, and the 0.1.11 landing version are all independently confirmed
via network sources, not just inferred from local cache diffing.
