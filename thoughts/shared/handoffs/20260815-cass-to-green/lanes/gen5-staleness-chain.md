# gen5-staleness-chain

Read-only survey lane. Bead `coding_agent_session_search-0gzok` part 1.

## Question

What breaks if `StateDbSnapshot.last_scan_ts` gains a third state, so that "the
read FAILED" is distinguishable from "the key is absent"?

## Method

`rg -n` for every literal appearance of `last_scan_ts` and `StateDbSnapshot` in
`src/lib.rs`, then `Read` on every hit and its enclosing function. Traced each
consumer forward to its JSON output field and to the CLI command that surfaces
it. Cross-checked against the 11 golden fixtures that contain an `"index"` JSON
key by grepping for the field (`"fresh"`) that is unique to this code path, to
separate them from `cass diag`'s unrelated `"index"` key of the same name. No
edits made anywhere. No cargo command run.

**Important caveat on method**: `src/lib.rs` is under active concurrent edit by
another lane in this same worktree for the whole duration of this survey (bead
`coding_agent_session_search-nao4q`, task #2/#3 on the shared board — bounding
`probe_state_db` and fixing the sibling `conversation_count`/`message_count`
honesty gap). `git diff --stat -- src/lib.rs` grew from `+81` to `+181/-30`
lines between my first and last read. Every line number below is the number I
read at the moment I read it; I re-verified the full `last_scan_ts` chain a
final time at 2026-08-15T17:14Z immediately before writing this file and cite
those numbers. `git diff -- src/lib.rs | rg 'last_scan_ts'` returns **nothing**
— the concurrent edits have not touched the code path this lane is about, only
inserted a bounding wrapper above it and rewritten the sibling
count-scan logic below it, which is why the line numbers moved without the bug
moving.

## Findings

### 1. Bead-cited line numbers: drifted, contents confirmed

- `StateDbSnapshot.last_scan_ts` — bead cites `src/lib.rs:15295`. **Holds
  exactly**, unaffected by the concurrent edit because it sits before the
  insertion point. `src/lib.rs:15295`: `last_scan_ts: Option<i64>,`.
- The `.ok().and_then(..)` read — bead cites `src/lib.rs:15357-15364`. **Drifted
  to `src/lib.rs:15470-15477`** (currently +113 lines from the bead's number;
  it was +81 an hour into this survey and the concurrent lane kept editing
  after that). Content unchanged:
  ```
  15470  snapshot.last_scan_ts = franken_query_row_map_retry(
  15471      &conn,
  15472      "SELECT value FROM meta WHERE key = 'last_scan_ts'",
  15473      params![],
  15474      |r| r.get_typed::<String>(0),
  15475  )
  15476  .ok()
  15477  .and_then(|s| s.parse::<i64>().ok());
  ```
  This function is now named `probe_state_db_blocking` (the sibling lane
  renamed the old `probe_state_db` body when it wrapped it in a bounded
  worker-thread caller also named `probe_state_db`). The bug's shape is
  identical: `franken_query_row_map_retry(..).ok()` turns a real DB error
  (open failed, WAL corruption, malformed non-numeric row, the query itself
  erroring) into the exact same `None` that a fresh archive's absent
  `last_scan_ts` meta row produces.
- Staleness rule — bead cites `src/lib.rs:16206-16210`. **Drifted to
  `src/lib.rs:16308-16313`.** Content unchanged:
  ```
  16308  if !assets.lexical.rebuilding
  16309      && last_scan_ts.is_some_and(|scan_ts| {
  16310          last_indexed_at
  16311              .map(|indexed_at| scan_ts > indexed_at.saturating_add(1_000))
  16312              .unwrap_or(true)
  16313      })
  ```
- The stale-setting branch — bead cites `src/lib.rs:16212-16219`. **Drifted to
  `src/lib.rs:16314-16321`.** Content unchanged:
  ```
  16314  {
  16315      assets.lexical.status = "stale";
  16316      assets.lexical.fresh = false;
  16317      assets.lexical.stale = true;
  16318      assets.lexical.status_reason = Some(
  16319          "last_scan_ts is newer than last_indexed_at; a prior scan advanced without a completed projection into the searchable index"
  16320              .to_string(),
  16321      );
  ```

The bead's read of the bug is accurate. `last_scan_ts.is_some_and(...)` is
false whenever `last_scan_ts` is `None`, and `None` is what a database open
error, an FTS-integrity failure, a malformed/unparseable meta row, and (as of
the concurrent lane's still-unmerged work) a probe timeout all produce —
identically to a legitimately fresh archive that has never been scanned. On any
of those four failure paths the override simply never fires, so
`assets.lexical.status`/`fresh`/`stale` keep whatever the ordinary age-based
computation (`src/search/asset_state.rs:1313`, `lexical_state_from_observations`
— see finding 3) set them to, with no signal that this specific check never
ran.

**One nuance the bead's framing elides**: a failed `last_scan_ts` read does not
force the whole index to read "fresh forever." `lexical_state_from_observations`
(`src/search/asset_state.rs:1313`) computes `fresh`/`stale` independently from
`age_seconds` vs. `stale_threshold`, checkpoint mismatches, and fingerprint
mismatches — `last_scan_ts` is not one of its inputs (`LexicalObservationInput`
has no such field; confirmed by reading its destructuring at
`src/search/asset_state.rs:1314-1323`). The override at `src/lib.rs:16308` only
ever *adds* staleness on top of that computation; it never removes staleness the
age-based path already found. So the concrete harm is scoped to the window this
specific signal exists to catch — a scan advanced (files changed on disk,
`meta.last_scan_ts` bumped) but the incremental lexical projection did not
finish, while `last_indexed_at` is still recent enough that the ordinary
age-based check has not yet tripped. In that window, a failed read makes the
index report fresh when it is provably not; outside that window, the ordinary
age-based staleness check still catches it, just later than it should have.

### 2. Every consumer of `last_scan_ts`, in `src/lib.rs`

Exhaustive `rg -n 'last_scan_ts' src/lib.rs` (11 hits, final pass):

| line | what |
|---|---|
| 15295 | struct field declaration, `Option<i64>` |
| 15470-15477 | the read that produces the field's value (`probe_state_db_blocking`) |
| 16196 | `let last_scan_ts = db_snapshot.last_scan_ts;` in `state_meta_json_inner` |
| 16308-16313 | the staleness rule (only consumer of the local `last_scan_ts` binding) |
| 16319 | the `status_reason` string that names `last_scan_ts` in prose when the rule fires |
| 66537-66538 | test setup: `seed_cli_db()` calls `storage.set_last_scan_ts(1_732_999_999_000)` |
| 66641 | test assertion: `probe_state_db_reads_meta_without_count_scan` asserts `snapshot.last_scan_ts == Some(1_732_999_999_000)` on a successful read |
| 66732 | test setup: `status_state_marks_scan_ahead_of_projection_stale` bumps `last_scan_ts` ahead of `last_indexed_at` |
| 66749 | test assertion: same test, asserts the `reason` string contains `"last_scan_ts is newer"` |

There is exactly one production consumer of the `last_scan_ts` local variable:
the staleness rule at 16308-16313. `state_meta_json_inner` is the only function
that reads `db_snapshot.last_scan_ts` (confirmed: `rg -n 'StateDbSnapshot'
src/lib.rs` shows the struct is constructed in three places —
`probe_state_db_blocking`, the timeout-fallback branch of the new `probe_state_db`
wrapper, and the `skip_db_open` fast path in `state_meta_json_inner` — and read
back out in exactly one place, `state_meta_json_inner:16196`).

**Existing tests, and the gap**: two tests touch `last_scan_ts`
(`probe_state_db_reads_meta_without_count_scan` at 66627ish and
`status_state_marks_scan_ahead_of_projection_stale` at 66706ish — both
confirmed by direct read). Both exercise only the **successful-read** path
(a real, parseable value written by `FrankenStorage::set_last_scan_ts`). I
searched the whole file (`rg -n` across all of `src/lib.rs`, not just the
`cli_read_db_tests` module) for any test that seeds a malformed/unparseable
`last_scan_ts` meta value, or that distinguishes a DB-open failure from an
absent key for this specific field, and found none — null result, not a
missed search: the only two `last_scan_ts`-touching test functions in the
file are the two above. `status_state_still_probes_malformed_non_file_db_path`
(current line ~66765) is the closest existing failure-path test, but it
exercises a directory-at-db-path (open fails entirely, `opened: false`) — it
does not seed a DB that opens successfully with a corrupt/unparseable
`last_scan_ts` row, which is the specific ambiguity in question.

### 3. What `fresh` and `assets.lexical.status` feed into

`state_meta_json_inner` never emits the raw `last_scan_ts` value as its own
JSON field — confirmed by `rg -n '"last_scan_ts"' src/lib.rs`, zero hits. It
only reaches JSON two ways: as prose inside `"reason"` when the override fires
(line 16319), and as the derived fields `"status"`, `"fresh"`, `"stale"` under
the top-level `"index"` key (`src/lib.rs:16471-16482`, confirmed by direct
read of the final `json!({...})` in `state_meta_json_inner`).

Two production callers reach the real (unbounded-until-the-wrapper) probe and
so can actually observe the read-failure ambiguity:

- **`cass status`** (`run_status`, `fn run_status(` currently at
  `src/lib.rs:65041`) calls `state_meta_json_for_status` (line 65122ish →
  confirmed call site reads `state.get("index").get("fresh")` at
  `src/lib.rs:65058-65061`-ish, re-verified live). `index_fresh` feeds directly
  into the `healthy` boolean (`src/lib.rs:65191-65198`), which feeds the
  one-word `status` ladder (`src/lib.rs:65204-65221`): `healthy` → literal
  string `"healthy"` (65210-65211). This is `cass status`'s one-word verdict.
- **`cass triage`** (`run_triage`, currently `src/lib.rs:65535`) — identical
  shape: `state_meta_json_for_status` → `index_fresh` → `healthy` → status
  ladder (`"healthy"`/`"stalled"`/`"rebuilding"`/`"not_initialized"`/
  `"degraded"`/`"unhealthy"`), confirmed by direct read of the analogous block
  around current line 65600-65620.

One caller does **not** reach the ambiguity, by design, and it is worth being
precise about why: **`cass health`** (`run_health`, currently
`src/lib.rs:65731`) calls `state_meta_json_for_health`, which always passes
`skip_db_open: true` (confirmed at the function definition, current line
~15990-16005 area, unchanged content: `state_meta_json_full(data_dir, db_path,
stale_threshold, true, Some(false), true, true)`). With `skip_db_open: true`,
`state_meta_json_inner`'s `db_snapshot` is built from the pure-filesystem
branch (`StateDbSnapshot { opened: true, counts_skipped: true, open_skipped:
true, ..Default() }`) and `probe_state_db`/`probe_state_db_blocking` is never
called at all — `last_scan_ts` is `None` unconditionally, not because a read
failed. So `cass health`'s one-word verdict is untouched by this specific bug:
it never attempts the read this bug is about, by an explicit, already-documented
design decision (the comment at current line ~15973-15976: "health is the
documented <50ms fast surface; force-skip... the FrankenStorage open").

**Concrete, currently-live consequence of the read-ambiguity, sharpened by the
concurrent lane's still-in-progress work**: the sibling lane's new bounded
`probe_state_db` wrapper (current lines 15343-15388, confirmed by direct read)
returns, on a real timeout, `StateDbSnapshot { counts_skipped: true, open_error:
Some(...), open_retryable: true, ..Default() }` — `last_scan_ts: None` via
`..Default()`, and `opened: false`. In `run_status`, `db_available = db_opened
|| (db_exists && db_open_retryable)` (current line 65176) evaluates to `true`
on a timeout (since `db_exists && open_retryable`), and `index_fresh` stays
whatever the age-based computation set it to (the staleness override never
fires, since `last_scan_ts` is `None`). If the archive was indexed recently
enough that the age check alone would not yet flag it stale, `healthy` can
evaluate `true` and the one-word verdict prints `"healthy"` on a probe that
just timed out and read nothing. `run_status` has no `"errors"` array at all
(confirmed: `rg -n '"errors"' src/lib.rs` restricted to the `run_status`
function range returns nothing) — the only place `db_open_error` surfaces in
JSON output is `database.open_error` (current line 65400), a field a caller
would have to know to check independently of the top-level verdict. This was
true of a plain DB-open failure before the concurrent lane's work; the
concurrent lane's bounding fix (which is real progress — it stops `cass triage
--json` from hanging forever, per its own doc comment) additionally routes a
new failure mode, probe timeout, into the same blind spot, because it reuses
`StateDbSnapshot::default()`'s `last_scan_ts: None` for the timeout case too.

`cass stats` (`run_stats`, `src/lib.rs:23821`) is a **null result** for this
question: `rg -n 'state_meta_json|last_scan_ts|\.fresh\b|\.stale\b|"index"'
src/lib.rs` restricted to the `run_stats` function's line range (23821-24148)
returns nothing. `cass stats --json` touches `connector_coverage` (the sibling,
already-fixed floors bug) but never touches `last_scan_ts`, `fresh`, or
`stale`. I looked; there is no consumer there, not an unsearched gap.

### 4. Existing tri-state idiom to copy

Yes, and it lives in the same file. `read_connector_scan_floors_bounded`
(current `src/lib.rs:15161-15190`, doc comment at 15137-15160) and the plain
`read_connector_scan_floors` it wraps (current lines ~15111-15130) both return
`Option<BTreeMap<String, i64>>`, where:

- `None` = the probe never opened the database, so it did not check (open
  failed, or the bounded read exceeded its wall clock).
- `Some(empty map)` = checked, and every connector's coverage is complete.
- `Some(non-empty map)` = checked, and named connectors have an unproven
  floor.

The struct field doc comment states this explicitly (current lines
15306-15309, unchanged content): *"Per-connector scan coverage floors, or
`None` when this probe never opened the database and so did not check.
`Some(empty)` means checked and complete; the two are not interchangeable."*
The render side keeps the distinction visible in JSON:
`connector_coverage_state_json` (current `src/lib.rs:15217-15227`) emits
`"checked": false` for `None` and `"checked": true, "complete": ...` for
`Some`, rather than collapsing both into one shape.

**The direct scalar mirror**: a bare `i64` has no natural "empty" sentinel the
way a `BTreeMap` has `Some(empty)`, so the literal analogue for
`last_scan_ts` is nested `Option<Option<i64>>` (outer `None` = did not check;
`Some(None)` = checked, key genuinely absent — a fresh archive that has never
been scanned; `Some(Some(ts))` = checked, value present) — or, more readably
at call sites, a three-variant enum such as
`enum ScanWatermark { Unchecked, NeverScanned, At(i64) }` carrying the same
three states without nested-`Option` ergonomics. I checked for a precedent of
either shape already in this file: `rg -n 'Option<Option<' src/lib.rs` returns
zero hits, and I found no hand-rolled tri-state enum for a scalar meta value
either. So there is a directly analogous **idiom** to copy
(`connector_scan_floors`'s None-means-unchecked convention, and its
`connector_coverage_state_json`-style honest render), but no existing **type**
in this file already shaped this way for a bare scalar — it would be new code,
not a copy-paste of an existing struct.

(Aside, outside `src/lib.rs` and therefore outside this lane's primary charge:
`src/search/asset_state.rs:449-462` defines `LexicalFingerprintState` and
`LexicalCheckpointState` with fields like `matches_current_db_fingerprint:
Option<bool>`, which is the same "None = not computed" convention applied to a
scalar — but it does not distinguish "computation was attempted and failed"
from "computation was never attempted," so it is not a stronger precedent than
the connector-floors one above.)

### 5. Does this ripple into the state envelope JSON and the `golden_robot_json` goldens?

**Partially confirmed, partially undetermined — and the full audit is a
sibling lane's job (`gen5-golden-radius`), not mine.** What I directly
verified:

- The raw `last_scan_ts` value is never its own JSON key (finding 3, `rg -n
  '"last_scan_ts"' src/lib.rs` → zero hits). Any ripple is through the derived
  `fresh`/`stale`/`status`/`reason` fields under `"index"`, or through a
  wholly new field the eventual fix might add.
- Of the golden files under `tests/golden/robot/`, exactly **7** carry the
  `state_meta_json_inner`-sourced `"index"` block (identified precisely, not
  by the looser `"index"` key match: `rg -l '"fresh"' tests/golden/robot/*`
  restricted to the 11 files that contain any `"index"` key returns exactly
  `status_quarantine.json.golden`, `introspect.json.golden`,
  `health.json.golden`, `status_shape.json.golden`,
  `introspect_shape.json.golden`, `status_quarantine_full.json.golden`,
  `health_shape.json.golden`). The other 4 files that contain an `"index"` key
  (`diag.json.golden`, `diag_shape.json.golden`, `diag_quarantine.json.golden`,
  `error_envelope_kinds.json.golden`) carry `cass diag`'s **unrelated**
  `"index"` object (`{"exists": bool, "size_bytes": ...}`, confirmed by direct
  read of `run_diag`'s JSON construction at current `src/lib.rs:24374-24377` —
  it never calls `state_meta_json_inner` — and confirmed those 4 goldens have
  zero `"fresh"` hits). `introspect.json.golden` is not itself a status/health
  output; it is `cass introspect --json`'s self-documented response-schema
  dump, and it embeds the status command's schema nested under a `"state"` key
  (confirmed by direct read at `tests/golden/robot/introspect.json.golden:
  27172-27204`) — so it is a real, if indirect, consumer of the same shape.
- **Null result on today's fixtures**: `rg -n '"fresh": true|"status":
  "healthy"|"stale": true' tests/golden/robot/*.golden` returns nothing. Every
  one of the 7 relevant goldens is built from a fixture where the database
  either does not exist or fails to open (confirmed: every `"opened"` value in
  `health.json.golden` is `false`; `status_quarantine.json.golden`'s `"index"`
  block shows `"exists": false`). None of today's goldens reach the
  `probe_state_db_blocking` line that reads `last_scan_ts` at all — the
  short-circuit at `!db_path.exists()` (current line 15400-15401, and the
  earlier `open_franken_cli_read_db` error return at 15410-15414) fires first
  in every case I found. So **as the fix is scoped today, with zero golden
  fixtures exercising a successful DB open**, a type-only change to
  `last_scan_ts` (no new JSON field) would not change a single existing
  golden's bytes — I could not find a fixture where it would matter.
- **What I did not determine**: whether the eventual fix will add a new
  visible JSON field (mirroring `connector_coverage`'s own `"checked"` key,
  which the commit history in this repo shows required updating "seven
  goldens" per the `git log` line `45d93234 fix(a4xe1): teach seven goldens the
  connector_coverage block, without regenerating them` — a suggestive but not
  identical count to the 7 I found above; I did not check whether it is
  literally the same 7 files, only that the counts match, which may be
  coincidence). If the fix adds a sibling field to `"index"` the way
  `connector_coverage` was added as a sibling top-level key, all 7 files above
  need updating (2 are `_shape` schema goldens needing a new property entry
  rather than a new value). If the fix stays internal to the struct (e.g.
  guards the override on `db_snapshot.opened` rather than exposing a new JSON
  field), none of today's 7 need touching, on the evidence above. This
  decision has not been made — no such fix exists in this checkout as of this
  read (confirmed: `git diff -- src/lib.rs | rg 'last_scan_ts'` → empty) — so
  I am reporting the two branches and their evidence rather than a single
  verdict. The full enumeration and exact golden-by-golden adjudication is
  `gen5-golden-radius`'s stated charge per the coordinator log
  (`thoughts/shared/handoffs/20260815-cass-to-green/gen5-coordinator.md:55`); I
  did the boundary-check the bead's blanket claim needed and stopped there
  rather than duplicating that lane's work.

## Proof boundary — what I did NOT establish

- I did not run `cargo build`/`cargo test`/`cargo check` (forbidden by this
  lane's hard limits) — every claim above is from static reading of source and
  golden text, not from executing anything. In particular I did not execute
  `status_state_marks_scan_ahead_of_projection_stale` or the new
  `probe_state_db_that_exceeds_its_bound_elides_counts_instead_of_inventing_zeros`
  test to confirm they currently pass; I only read their source and asserted
  what they exercise by inspection.
- I did not verify whether any test OUTSIDE `src/lib.rs` (e.g. `tests/*.rs`
  integration tests, `tests/golden_robot_json.rs`'s command-level tests)
  exercises a successful `probe_state_db`/`probe_state_db_blocking` open on a
  populated `agent_search.db` that reaches the `last_scan_ts` query line. I
  searched golden fixture JSON files (static text) for evidence of what
  scenario they represent, and separately confirmed no `src/lib.rs`-internal
  unit test exists for the malformed/failed-read case; I did not exhaustively
  read every integration test file's DB-seeding helper to rule out a
  `tests/*.rs` fixture I have not seen.
- I did not determine the exact shape the eventual `-0gzok` part 1 fix will
  take (nested `Option<Option<i64>>`, a 3-variant enum, or a sibling boolean
  field on `StateDbSnapshot`) — that is an implementation decision for
  whichever lane writes the fix, not something the current code commits to.
  My answer to question 4 names the analogous idiom and the two plausible
  scalar shapes; it does not pick one.
- I did not check every one of the 11 `"index"`-bearing golden files' exact
  assertion mode (byte-equality vs. structural/shape-tolerant comparison)
  beyond the two representative samples I read directly
  (`status_quarantine.json.golden`, `status_shape.json.golden`). The full
  golden-by-golden blast-radius audit is explicitly `gen5-golden-radius`'s
  charge; my finding 5 above is a boundary check (does the bead's blanket
  ripple claim hold on today's fixtures — no, on the evidence I found — and
  under what condition would it start holding), not that lane's exhaustive
  enumeration.
- The file was under active concurrent edit by another lane for this survey's
  entire duration. All line numbers above are what I read at
  2026-08-15T17:14Z or the specific earlier read time noted; if the sibling
  lane lands further commits before this log is consumed, re-verify with
  `rg -n 'last_scan_ts' src/lib.rs` before trusting any line number in this
  file at face value — I state this not as hedging but because I directly
  measured two rounds of drift (+81, then +181/-30) while writing this log,
  and the mechanism (a second concurrent lane editing the same file) is
  ongoing, not a one-time event I already priced in.
