# Adversarial verification of lane `fsqlite-claims` — 2026-08-15

Read-only verifier lane. Wrote only this file. No `cass index`/`cass sources`,
no writes to the live DB, no git mutations, no builds. Live DB touched only via
`/bin/ls -la` (stat, no open).

Target: `thoughts/shared/handoffs/20260815-cass-to-green/lanes/fsqlite-claims.md`
and its returned headline. Method: open every cited file:line and re-derive the
number myself; re-run every network call independently; hunt specifically for
(a) filtered probes stated unfiltered, (b) negatives from instruments never
shown capable of a positive, (c) drifted line numbers, (d) claims promoted from
another document rather than measured.

**Verdict: NOT REFUTED.** All four claims survive. Two defects found in the
lane's *evidence wording and citation chain*; neither overturns a conclusion,
and one of them I independently re-proved through a different code path that is
stronger than the lane's own.

---

## Claim 0 (baseline) — cass pins 0.1.5 — CONFIRMED

`rg -n frankensqlite Cargo.toml`:
```
45:frankensqlite = { version = "0.1.5", package = "fsqlite", features = ["fts5"] }
```
`rg -n -A3 '^name = "fsqlite"$' Cargo.lock`:
```
2270:name = "fsqlite"
2271-version = "0.1.5"
2272-source = "registry+https://github.com/rust-lang/crates.io-index"
2273-checksum = "5415050ff8a232b55f774a3fc9267164f5ad010c806899772c4e06f0d8988dc0"
```
`sed -n '175,190p' Cargo.toml` line 181:
`fsqlite-types = { version = "0.1.5", package = "fsqlite-types" }`

Every cited line number is exact. Requested and resolved both 0.1.5.

---

## Claim 2 — ExistsValueSet 0 in 0.1.5, 8 in 0.1.17 — CONFIRMED

I re-ran the sweep myself rather than trusting the lane's, and used a **stronger
positive control** than the lane did (same file, same identifier shape, not just
`fn `):

Positive control, on the exact file the negative is claimed about:
```
$ rg -c MemDatabase .../fsqlite-core-0.1.5/src/connection.rs
211
rc=0
```
So rg reads that file and would print a count for a CamelCase type that exists.

Negative sweep, all 19 `fsqlite*-0.1.5` crate dirs **plus** `fsqlite-vfs-0.1.6`
(the one dir at a different version), passed as explicit absolute paths in two
`rg -c ExistsValueSet` invocations:
```
(no output)   rc=1
(no output)   rc=1
```
Zero matches anywhere in 0.1.5.

0.1.17, unfiltered `rg -n`:
```
53246:                let mut set = ExistsValueSet::default();
81348:    sets: std::collections::HashMap<(i32, usize), ExistsValueSet>,
81358:struct ExistsValueSet {
81386:impl ExistsValueSet {
81451:    use super::{ExistsValueSet, values_equal_sqlite};
81490:            let mut set = ExistsValueSet::default();
81513:        let mut set = ExistsValueSet::default();
81533:        let mut set = ExistsValueSet::default();
```
Eight, at exactly the eight lines the lane listed. `rg -c` on that file: `8`.
No drift.

---

## Claim 1 — issue #117 / commit 64c45de0 / shipped in 0.1.11 — CONFIRMED,
## and the lane's own "5 not 6" correction is itself correct

`gh api repos/Dicklesworthstone/frankensqlite/issues/117`:
```json
{"closed_at":"2026-06-19T18:07:40Z","created_at":"2026-06-18T17:06:50Z",
 "state":"closed","state_reason":"completed",
 "title":"Correlated `NOT EXISTS` (anti-join idiom) routes to the in-memory interpreter — 0.01s on canonical SQLite becomes ~5s and climbs (verified at `main` @ d1caefb5)"}
```

Issue body, grepped for the cited tokens (verbatim lines):
```
23:> - `:23602` `else if select_has_correlated_exists_in_where(select)` →
24:> - `:23609` `log_mem_execution_fallback("select", "correlated_exists_fallback")` →
25:>   `execute_join_select` (`:55230`, the interpreter).
26:> - `execute_join_select` does not read `reject_mem_fallback`, so `PRAGMA fsqlite.parity_cert
28:> - `correlated_exists_fallback` is excluded from `KNOWN_BENIGN_FALLBACK_REASONS` (`:11738`),
```
Identical string, identical routing description. Confirmed.

Closing comment, verbatim from `gh api .../issues/117/comments`:
> Fixed in 64c45de0, shipping in 0.1.11.

`gh api .../commits/64c45de0`:
```
sha   64c45de05b7fc95ebe4159518b8452f78cf2f54a
date  2026-06-19T02:08:42Z
msg   fix(core): O(1) memo for correlated NOT EXISTS/EXISTS probe (GH#117)
files crates/fsqlite-core/src/connection.rs            +267 -0
      crates/fsqlite-e2e/tests/issue_117_correlated_not_exists.rs  +212 -0
```
Exact on all four fields.

Local-source corroboration, re-derived independently of the lane:
```
$ rg -n correlated_exists_fallback .../fsqlite-core-0.1.5/src/connection.rs
22653:                    self.log_mem_execution_fallback("select", "correlated_exists_fallback")?;
$ rg -n correlated_exists_fallback .../fsqlite-core-0.1.17/src/connection.rs
24380:                    self.log_mem_execution_fallback("select", "correlated_exists_fallback")?;
```
0.1.5 branch body (`sed -n '22640,22672p'`): guard is the free function
`select_has_correlated_exists_in_where(select)`; body calls
`self.execute_join_select(&bound, None)?`.
0.1.17 branch body (`sed -n '24365,24400p'`): guard is
`self.select_correlated_exists_where_requires_fallback(select)`; body calls
`self.execute_correlated_subquery_where_fallback(cx, select, params)`.
Exactly as the lane described. The log string persisting into 0.1.17 is a cost
fix, not a missing fix — the commit message says so and the source agrees.

**Version arithmetic, re-derived from crates.io myself.** Full sorted
`fsqlite` version list (23 entries):
```
0.1.0 2026-02-21  0.1.1 2026-02-22  0.1.2 2026-03-22  0.1.3 2026-05-14
0.1.4 2026-05-26  0.1.5 2026-05-28T00:42:31.461907Z
0.1.7 2026-06-05  0.1.8 2026-06-06  0.1.9 2026-06-07  0.1.10 2026-06-13
0.1.11 2026-06-19T18:16:12.927034Z
0.1.12 … 0.1.19 2026-07-26  0.2.0 2026-08-09  0.2.1 2026-08-11
0.3.0 2026-08-13  0.3.1 2026-08-14T05:16:30.339853Z
```
**0.1.6 is absent from the `fsqlite` package.** Publishes strictly after 0.1.5
through 0.1.11 = 0.1.7, 0.1.8, 0.1.9, 0.1.10, 0.1.11 = **five**. The lane's
correction is right and I reproduced it from the primary source, not from the
lane's text. `fsqlite-vfs-0.1.6` does exist locally
(`/bin/ls -d …/fsqlite-vfs-0.1.6` → the path), corroborating the
sibling-crate-only explanation.

---

## Claim 3 — 23 releases since February, 0.3.1 on 2026-08-14 — CONFIRMED

```
$ curl -s https://crates.io/api/v1/crates/fsqlite   (User-Agent set)
created_at 2026-02-21T23:43:07.596138Z
num_versions 23     (len(versions) == 23, computed after the parse, not asserted)
max_version 0.3.1   newest 0.3.1   default 0.3.1
$ gh api .../releases --jq '.[0]|{tag_name,published_at}'
{"published_at":"2026-08-14T05:26:50Z","tag_name":"v0.3.1"}
$ gh api .../tags --paginate --jq '.[].name' | wc -l
18
```
All exact, including the lane's footnote that git tags (18) undercounts and is
the wrong instrument.

---

## Claim 4 — parity_cert — CONFIRMED on substance; TWO EVIDENCE DEFECTS

Sub-clauses 1, 2 and the routing half of 3 verified line by line in
`fsqlite-core-0.1.5/src/connection.rs`:
```
$ rg -n 'reject_mem_fallback: RefCell::new'
8798:            reject_mem_fallback: RefCell::new(true),
9105:            reject_mem_fallback: RefCell::new(true),
$ rg -n 'fn set_reject_mem_fallback|fn should_eagerly_hydrate_memdb_rows|fn reload_memdb_from_pager_with_mode|fn reload_memdb_rows_from_txn_preserving_schema|fn reload_memdb_from_txn_with_mode'
11058:    pub fn set_reject_mem_fallback(&self, reject: bool) {
12005:    fn should_eagerly_hydrate_memdb_rows(&self) -> bool {
55342:    fn reload_memdb_from_pager_with_mode(&self, cx: &Cx, hydrate_rows: bool) -> Result<()> {
55358:    fn reload_memdb_rows_from_txn_preserving_schema(
55875:    fn reload_memdb_from_txn_with_mode(
9100:            // bd-zjisk.1: Default to parity-cert mode — all cursor operations
126310:    fn test_zjisk1_memory_conn_parity_cert_default_on() {
126332:    fn test_zjisk1_pragma_parity_cert_query() {
```
PRAGMA arm at **43704** exactly (`"fsqlite.parity_cert" | "parity_cert" => {`),
with the eager reload at **43712** (`self.reload_memdb_from_pager_with_mode(&cx, true)?;`)
guarded by `if !enabled && self.path != ":memory:" && !self.memdb_rows_loaded.get()`.
`should_eagerly_hydrate_memdb_rows` at 12005-12006 reads
`self.path == ":memory:" || !*self.reject_mem_fallback.borrow()`. Tests at
126332 assert `SqliteValue::Integer(1)` with the message
`"default parity_cert must be 1 (ON)"`. All exact.

0.1.17 guard chain checked for hidden state reads: `29812` →
`select_correlated_exists_where_can_use_indexed_count_probe` (29841) →
`prepared_count_indexed_rowid_probe_fast_path` (29516). Full-file listing of
every `reject_mem_fallback` occurrence in 0.1.17 (37 lines) has **no hit inside
29516–29890**. The guard genuinely does not consult parity_cert in either
version.

### Defect A — the hydration citation names a function the PRAGMA path never calls

The lane wrote:
> `reload_memdb_from_pager_with_mode(cx, hydrate_rows=true)` (:55342) →
> `reload_memdb_from_txn_with_mode` → (traced into
> `reload_memdb_rows_from_txn_preserving_schema`, :55357 on)

That chain does not exist.
```
$ rg -n 'reload_memdb_rows_from_txn_preserving_schema' .../fsqlite-core-0.1.5/src/connection.rs
12409:            let reload_result = self.reload_memdb_rows_from_txn_preserving_schema(
55358:    fn reload_memdb_rows_from_txn_preserving_schema(
```
Definition plus exactly **one** call site, at 12409 — inside a cached-write-txn
memdb row-mirror refresh, not reachable from `reload_memdb_from_txn_with_mode`.
(Also: the function begins at 55358, not ":55357".)

The conclusion survives via a different, better site, which I read end to end.
`reload_memdb_from_pager_with_mode` (55342) calls `reload_memdb_from_txn_with_mode`
(55875) with `hydrate_rows=true`, and *that* function carries the walk at
**56451**:
```rust
// For file-backed databases in parity-cert mode, execution routes
// through pager-backed cursors and does not need every table row
// duplicated into the compatibility MemDatabase. Preserve only the
// schema-shaped placeholders unless Mem fallback is explicitly
// enabled. `sqlite_sequence` is still scanned so AUTOINCREMENT state
// survives reopen/reload without fully hydrating user tables.
let should_scan_rows = hydrate_rows || name.eq_ignore_ascii_case("sqlite_sequence");
if should_scan_rows {
    …
    if cursor.first(cx)? {
        loop {
            let (rowid, payload) = cursor.rowid_and_payload_cow(cx)?;
            …
            if hydrate_rows {
                …
                mem_table.insert_row(rowid, values);
```
That upstream comment states the mechanism outright: parity-cert ON ⇒ schema
placeholders only; parity-cert OFF ⇒ every row of every table walked and
inserted into the MemDatabase. So claim 4 sub-clause 4 is **confirmed more
strongly than the lane proved it** — but a reader re-deriving from the lane's
citation lands on a function the PRAGMA path never calls.

### Defect B — "never reads reject_mem_fallback in the branch body" is over-broad

The lane's sub-clause 3 evidence says the fallback shape's guard
"never reads `reject_mem_fallback` in either version's branch body." The guard
does not — confirmed above. The **branch body's first statement** does:
```
11185:    fn log_mem_execution_fallback(
11191:        let reject_mem = *self.reject_mem_fallback.borrow();
11192:        let strict_reject = *self.reject_mem_fallback_strict.borrow();
11193:        if reject_mem {
11194:            if strict_reject {
…                 return Err(FrankenError::not_implemented(format!(
                      "in-memory fallback disabled in strict parity-cert mode: …")));
```
Under `parity_cert_strict` this is not a log-level difference at all — it is a
hard `Err` on the statement, i.e. parity_cert becomes outcome-changing.

The claim still holds for cass, on two measured facts:
- `reject_mem_fallback_strict` defaults to **false** (`rg -n reject_mem_fallback_strict`
  → `8800:` and `9108: reject_mem_fallback_strict: RefCell::new(false)`), and it
  has its own separate PRAGMA (`parity_cert_strict`, arm at 43723).
- cass never touches either. `rg -c parity_cert <repo>/src` → no matches;
  positive control `rg -c frankensqlite <repo>/src` → 26 files (lib.rs:156,
  storage/sqlite.rs:174, …), proving rg searches that tree.

So the accurate wording is "changes the log level (warn↔debug), not the route,
**so long as parity_cert_strict stays off** — and cass never sets it," rather
than "never reads reject_mem_fallback."

### Minor — live DB size

Lane cited 7.96 GB at 06:02; `/bin/ls -la` at 06:17 reports **8,092,905,472
bytes** (8.09 GB decimal / 7.54 GiB), consistent with a growing backfill. Not
load-bearing; the "multi-GB, hydrating it all is strictly worse" point is
unaffected.

---

## Summary

| # | Claim | My verdict | Note |
|---|---|---|---|
| 0 | cass pins 0.1.5 | CONFIRMED | every line exact |
| 1 | issue #117 → 64c45de0 → 0.1.11 | CONFIRMED | issue, comment, commit, publish date all exact; the "5 not 6" correction independently reproduced from crates.io |
| 2 | ExistsValueSet 0 / 8 | CONFIRMED | re-swept with a same-file positive control (MemDatabase = 211) |
| 3 | 23 releases, 0.3.1 on 2026-08-14 | CONFIRMED | num_versions computed post-parse; tags=18 footnote also correct |
| 4 | parity_cert default-on / disable-able / no route change / full hydration | CONFIRMED on substance | **Defect A**: cited hydration function has one caller and it is not on the PRAGMA path — real walk is `should_scan_rows` at 56451. **Defect B**: the branch body *does* read reject_mem_fallback via log_mem_execution_fallback (11191) and errors under strict (default off; cass never sets it). |

Nothing promoted from another document: every number above came from a command
I ran in this lane.
