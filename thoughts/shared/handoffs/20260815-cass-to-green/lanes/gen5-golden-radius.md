# Lane: gen5-golden-radius

**Question:** exactly which golden fixtures would have to be re-adjudicated if the
cass state envelope gains honest tri-state for (a) database counts, (b)
last_scan_ts staleness, (c) connector coverage parseability?

**Lane type:** read-only survey. No files edited, no cargo commands run.

---

## Method

1. Located the golden fixture set with `fd`/`wc -l` (counted, never truncated with
   `| head`).
2. For each of the four target surfaces, ran a scoped `rg` (object-shaped pattern,
   e.g. `"database":\s*\{`, not the bare string) across
   `tests/golden/robot/*.golden`, then opened every hit with `Read` to confirm it
   is a real object with the named sub-keys, not a false positive (a check-id
   string, an enum list member, or a `fallback_mode` string value).
3. Read `tests/golden_robot_json.rs` end to end for the scrub/normalize/compare
   pipeline and cited the exact functions and line ranges.
4. Read `src/lib.rs` to find where each surface is actually serialized, to
   establish whether a 0→null or `checked:false` change is a **value** flip
   (existing key, no golden shape change needed beyond re-diffing) or a
   **shape** change (new key, definitely needs re-adjudication) — since the
   task explicitly distinguishes "contains the key" from "would change value".
5. Ran one whole-directory combined-pattern sanity pass at the end to catch
   anything the four separate searches might have missed, then manually
   triaged every new hit it surfaced.

All `rg` invocations were run directly (never `rg | rg`, never truncated with
`| head` when the question was existence/count). Every file:line cited below was
opened with `Read` in this session.

---

## Findings

### 1. Golden fixture count and location

`tests/golden/robot/*.golden` contains **37 files**, counted with
`fd . tests/golden/robot -e golden | wc -l` → `37`. This matches the task's
stated "37-case suite" and is the fixture set `tests/golden_robot_json.rs`
exercises via `assert_golden()`. (There are 76 total `*golden*`-named paths in
the repo, but the other ~39 belong to sibling suites —
`golden_metamorphic_search.rs`, `golden_regression_search.rs`,
`golden_readiness.rs`, `robot_docs/`, `swarm_status/`, `html_export/`,
`log/` — which are out of scope; this lane covers only the 37 named in the
task.)

Full file list (alphabetical):

```
api_version.json.golden                          health_shape.json.golden
api_version_shape.json.golden                     introspect.json.golden
capabilities.json.golden                          introspect_shape.json.golden
capabilities_shape.json.golden                    models_check_update_not_installed_shape.json.golden
diag.json.golden                                  models_status.json.golden
diag_quarantine.json.golden                        models_status_shape.json.golden
diag_shape.json.golden                             models_verify_not_acquired_shape.json.golden
doctor.json.golden                                 pack_empty_query.json.golden
doctor_quarantine.json.golden                      pack_missing_index.json.golden
doctor_shape.json.golden                           quarantine_summary_shape.json.golden
error_envelope_kinds.json.golden                   search_robot.json.golden
export_html_shape.json.golden                      search_robot_shape.json.golden
health_semantic_backfill_wait.json.golden          sessions_missing_db_shape.json.golden
health_semantic_progress.json.golden               stats_full_payload.json.golden
status_quarantine.json.golden                      stats_full_payload_shape.json.golden
status_quarantine_full.json.golden                 stats_missing_db.json.golden
status_semantic_backfill_wait.json.golden          stats_missing_db_shape.json.golden
status_semantic_progress.json.golden
status_shape.json.golden
health.json.golden
```

### 2. Per-file table — deliverable

"Object present" means an actual JSON object/scalar of that shape exists at
that path (verified by `Read`, not just a string match). "counts_skipped
present" is a sub-column of the database row because it only matters where a
database/db object exists at all.

| # | Golden file | database/db object (conv+msg) | counts_skipped key | connector_coverage block | lexical{status,fresh} object | last_scan_ts literal |
|---|---|---|---|---|---|---|
| 1 | api_version.json.golden | No | — | No | No | No |
| 2 | api_version_shape.json.golden | No | — | No | No | No |
| 3 | capabilities.json.golden | No | — | No | No (only `"lexical"` as a `fallback_mode` enum list member, `tests/golden/robot/capabilities.json.golden:677,992`) | No |
| 4 | capabilities_shape.json.golden | No | — | No | No | No |
| 5 | diag.json.golden | **Yes** — 4-key: exists,size_bytes,conversations,messages (`:12-17`) | **No** | No | No | No |
| 6 | diag_quarantine.json.golden | **Yes** — same 4-key shape (`:12-17`) | **No** | No | No | No |
| 7 | diag_shape.json.golden | **Yes** — schema, same 4 props (`:32-47`) | **No** | No | No | No |
| 8 | doctor.json.golden | No (the only `"database"` hits are a check-id string `:271` and a check `"name": "database"` `:3050` — not a counts object) | — | No | **Yes** — full concrete object (`:678-705`), status="missing" | No |
| 9 | doctor_quarantine.json.golden | No (same check-id/check-name false positives, `:271`, `:3100`) | — | No | **Yes** — concrete, status="missing" (`:678-681`) | No |
| 10 | doctor_shape.json.golden | No | — | No | **Yes** — schema (`:759-`) | No |
| 11 | error_envelope_kinds.json.golden | No | — | No | No | No |
| 12 | export_html_shape.json.golden | No | — | No | No | No |
| 13 | health.json.golden | **Yes — two distinct blocks**: top-level `"db"` (7-key, no `path`/`open_retryable`, `:79-87`) and nested `"database"` under the embedded status snapshot (8-key incl. `path`/`open_retryable`, `:390-399`) | **Yes**, both blocks (`:85`, `:397`) | **Yes** (`:400-405`, already has `"checked": false`) | No (only `fallback_mode`/`semantic_fallback_mode` string values, e.g. `:97`) | No |
| 14 | health_semantic_backfill_wait.json.golden | No | — | No | No (only `"fallback_mode": "lexical"` string, `:10`) | No |
| 15 | health_semantic_progress.json.golden | No | — | No | No (same, `:10`) | No |
| 16 | health_shape.json.golden | **Yes** — schema for both `"db"` (`:172-`) and `"database"` (`:972-`) | **Yes**, both (`:190`, `:993`) | **Yes**, schema (`:75-`, `:1001-`) | No | No |
| 17 | introspect.json.golden | **Yes** — this file is the full API self-documentation surface; it embeds `response_schemas` copies of every other command's shape. Found a `"db"` schema (`:26279-`) and **six** `"database"` schema fragments (`:2900`, `:27298`, `:29484`, `:30394`, `:32200`, `:33895`) — one matches diag's 4-prop shape (no `counts_skipped`), the other five match the status/health 8-prop shape (with `counts_skipped`) | **Yes**, in 5 of 6 database schema fragments + the db schema (6 total `counts_skipped` hits: `:26306,27328,29514,30424,32230,33925`) | **No** — zero `"connector_coverage"` hits in this file despite documenting health/status | **Yes** — schema fragment (`:4050-`, includes `status`, `fresh`) | No |
| 18 | introspect_shape.json.golden | **Yes** — same self-documentation pattern, one level deeper nested (schema-of-schema); `"db"` at `:50377`, `"database"` at `:832,52566,57609,59579,63448,67049` | **Yes**, at multiple nested sites (e.g. `:50435,52632,57675,59645,63514,67115`) | **No** — zero hits | **Yes** — schema fragment (`:3118-`) | No |
| 19 | models_check_update_not_installed_shape.json.golden | No | — | No | No | No |
| 20 | models_status.json.golden | No | — | No | No | No |
| 21 | models_status_shape.json.golden | No | — | No | No | No |
| 22 | models_verify_not_acquired_shape.json.golden | No | — | No | No | No |
| 23 | pack_empty_query.json.golden | No | — | No | No | No |
| 24 | pack_missing_index.json.golden | No | — | No | No | No |
| 25 | quarantine_summary_shape.json.golden | No | — | No | No | No |
| 26 | search_robot.json.golden | No | — | No | No | No |
| 27 | search_robot_shape.json.golden | No | — | No | No | No |
| 28 | sessions_missing_db_shape.json.golden | No | — | No | No | No |
| 29 | stats_full_payload.json.golden | **Yes but different shape** — bare top-level scalars `"conversations": 2`, `"messages": 6` (`:2-3`), **not** wrapped in a `database`/`db` object, real non-zero demo-corpus values | **No** — no `counts_skipped` sibling exists in this shape at all | **Yes** (`:40-44`, already has `"checked": false`) | No | No |
| 30 | stats_full_payload_shape.json.golden | **Yes** — schema, same bare top-level `conversations`/`messages` props (`:4-9`) | **No** | **Yes**, schema (`:96-`, already has `"checked"` property) | No | No |
| 31 | stats_missing_db.json.golden | No — pure error envelope (`code`,`kind`,`message`,`hint`,`retryable`), short-circuits before any counts are built | — | No | No | No |
| 32 | stats_missing_db_shape.json.golden | No | — | No | No | No |
| 33 | status_quarantine.json.golden | **Yes** — 8-key `"database"` object (`:38-48`) | **Yes** (`:46`) | **Yes** (`:197-201`, already has `"checked": false`) | No | No |
| 34 | status_quarantine_full.json.golden | **Yes** — same 8-key shape (`:38-47`) | **Yes** (`:46`) | **Yes** (`:197-201`, already `"checked"`) | No | No |
| 35 | status_semantic_backfill_wait.json.golden | No | — | No | No (only `"fallback_mode": "lexical"`, `:10`) | No |
| 36 | status_semantic_progress.json.golden | No | — | No | No (same, `:10`) | No |
| 37 | status_shape.json.golden | **Yes** — schema, 8-key (`:112-142`) | **Yes**, schema (`:136-`) | **Yes**, schema (`:581-`, already has `"checked"`) | No | No |

**Union: 15 of 37 goldens touch at least one of the four surfaces.** The other
22 are structurally outside this radius entirely — they carry none of
`database`/`db`/bare-count scalars, `connector_coverage`, or a concrete/schema
`lexical{status,fresh}` object. Verified with a final combined-pattern sweep
(`rg -c '"database"|"connector_coverage"|"lexical"|"last_scan_ts"|"conversations"|"messages"|"counts_skipped"'`)
and every non-zero hit outside the 15 was individually opened and confirmed to
be a false positive (a `fallback_mode` string value or a `lexical` enum-list
entry), not an object.

### 3. How the suite compares — exact equality, not fuzzy

`assert_golden()` (`tests/golden_robot_json.rs:927-966`) does **byte-for-byte
string equality**: `if actual != expected` (`:951`), against
`tests/golden/<name>` read straight off disk (`:940`). There is no JSON-level
diffing, no key-order tolerance beyond what `serde_json::to_string_pretty`
already canonicalizes, and no partial/subset comparison.

Before that comparison, output passes through two normalization stages, both
of which run unconditionally (not opt-in):

- **`scrub_robot_json`** (`:740-923`) — regex string substitutions for:
  `crate_version`/top-level `version` (`:745-752`), ISO-8601 timestamps
  (`:755-758`), the absolute test-HOME path and the repo root path
  (`:763-778`), UUIDs (`:781-784`), `latency_ms`/`elapsed_ms`/
  `probe_duration_ms`/`slowest_elapsed_ms`/`slowest_operation` (`:789-803`),
  `load_per_core`/`psi_cpu_some_avg10` (`:809-817`), watchdog counters
  `healthy_streak`/`ticks_total`/`load_window_len`/`psi_window_len`/
  `observations_total` (`:824-835`), `last_snapshot`/`last_reason`
  (`:855-867`), resource-policy worker/core counters and byte budgets
  (`:871-902`), `age_seconds` (`:904-907`), `last_read_at_ms` (`:909-912`).
- **`normalize_live_robot_values`** (`:329-539`) — a JSON-tree walk that folds
  host-derived topology/platform/resource-policy values to fixed Linux
  constants (`os`/`arch`, topology `source`, `memory_*_bytes`, reserved-core
  `policy`/`reason` text, topology-budget `fallback_active`/`decision_reason`/
  `proof_notes`, `current_capacity_pct`, `shrink_count`/`grow_count`,
  `recent_decisions`, `topology_class`/`logical_cpus`/`physical_cores`/
  `sockets`/`numa_nodes`/`llc_groups`/`smt_threads_per_core`,
  `semantic_batchers`, `steady_batch_fetch_conversations`,
  `startup_batch_fetch_conversations`, `controller_loadavg_*_watermark_1m`,
  plus any string starting with `"planned from "` or `"reserve "`), and skips
  entirely inside `response_schemas` / anything that looks like a JSON Schema
  object (`looks_like_json_schema_object`, `:278-327`) so schema type
  declarations are never mangled by the live-value folding meant for concrete
  payloads.
- **`sort_example_paths`** (`:239-256`) — sorts any `example_paths` array.

**None of these three passes touch any of the four target-surface keys.**
`database`, `db`, `conversations`, `messages`, `counts_skipped`,
`connector_coverage`, `checked`, `complete`, `incomplete_connectors`,
`floors`, `lexical`, `status`, `fresh`, `stale`, `reason`, `last_scan_ts` do
not appear anywhere in the scrub regex list or the normalize key-match list
(confirmed by grepping `tests/golden_robot_json.rs` for each name — only
unrelated hits at CLI-flag/test-name sites, e.g. `:1191,1338,1367,1418,2009`
for the string `"status"` as a CLI subcommand argument, not a JSON key).
**This means every golden that carries any of these four surfaces gets zero
scrub/normalize protection on them — any value or shape change there is
compared byte-for-byte and will fail every golden that carries it**, exactly
the "would have to be re-adjudicated" the question asks about.

### 4. Value-flip vs. shape-change — sourced from `src/lib.rs`

**(a) Database counts, 0 → null.** This is not hypothetical future work — the
mechanism already exists:

```rust
fn state_db_count_json(count: i64, counts_skipped: bool) -> serde_json::Value {
    if counts_skipped {
        serde_json::Value::Null
    } else {
        serde_json::Value::from(count)
    }
}
```
(`src/lib.rs:16761-16767`), wired into the status/health `"database"` builder
at `src/lib.rs:16512-16513` (`"conversations": state_db_count_json(conversation_count, counts_skipped)`)
and into `refresh_state_database_counts_if_needed` at `:16819,16823`.

**None of the 37 goldens currently has `"counts_skipped": true`** — confirmed
with a whole-directory search, zero hits. Every golden that has this shape
today (`health.json.golden` both blocks, `status_quarantine.json.golden`,
`status_quarantine_full.json.golden`, and the two schema files
`health_shape.json.golden`/`status_shape.json.golden`) is currently pinned on
the `counts_skipped: false` branch, so `conversations`/`messages` render as a
literal integer (`0`) everywhere they appear. **If a new/changed fixture
scenario flips `counts_skipped` to `true` for any of those code paths, those
6 goldens are exactly the ones whose `conversations`/`messages` value would
flip from `0` to `null` and require re-adjudication** — a pure value change,
same keys, same shape. `introspect.json.golden`/`introspect_shape.json.golden`
would additionally need their embedded schema fragments' declared type
widened from `"type": "integer"` to something nullable if the schema is meant
to stay honest about what the field can hold — this is a **pre-existing
latent gap**: the code can already emit `null` here (once a fixture exercises
`counts_skipped: true`), but every schema declaration for `conversations`/
`messages` in this suite currently pins `"type": "integer"` with no `null`
member (verified `diag_shape.json.golden:41-43`, `status_shape.json.golden:121-126`
declare it plainly as `"type": "integer"`).

**`diag.json.golden`/`diag_quarantine.json.golden`/`diag_shape.json.golden`
and `stats_full_payload.json.golden`/`stats_full_payload_shape.json.golden`
are structurally outside this mechanism entirely.** Their `conversations`/
`messages` are raw `i64` values assigned directly with no `counts_skipped`
concept at all — `src/lib.rs:24365-24370` (diag) and `src/lib.rs:24086-24089`
(stats), confirmed by reading both call sites. Extending tri-state to these
5 goldens is a **key-addition** (a new `counts_skipped`-equivalent field),
not a value flip — a bigger re-adjudication than the 6 above.

**(b) `connector_coverage` gaining `"checked": false`.** This is **already
shipped in every one of the 7 goldens that carry the block** — verified by
reading each: `health.json.golden:401`, `health_shape.json.golden:78,1004`
(schema), `stats_full_payload.json.golden:41`,
`stats_full_payload_shape.json.golden:99` (schema),
`status_quarantine.json.golden:198`, `status_quarantine_full.json.golden:198`,
`status_shape.json.golden:584` (schema). The source function
(`connector_coverage_state_json`, `src/lib.rs:15217-15227`) always emits
`"checked": false, "complete": null, "incomplete_connectors": [], "floors": []`
for the `None` branch, and that is exactly what every one of these 7 goldens
already pins. **This matches the 7-golden count in the recent commit
`45d93234 "fix(a4xe1): teach seven goldens the connector_coverage block,
without regenerating them"` — so "checked": false is not new work; none of the
37 goldens would need re-adjudication for it.** `introspect.json.golden`/
`introspect_shape.json.golden` do **not** document `connector_coverage` at all
(zero hits, confirmed) despite documenting health/status elsewhere in the same
file — the self-documentation surface is out of sync with the real command
output on this one field, independent of any tri-state work.

**(c) `last_scan_ts` / lexical staleness.** `last_scan_ts` itself is never
serialized to JSON under that literal name in any of the 37 goldens (genuine
null result, confirmed by a whole-directory search). It is an internal
`Option<i64>` field (`src/lib.rs:15295`), populated from a SQLite `meta` table
read (`src/lib.rs:15438-15440`), and consumed only as a **comparison input**
that derives the `lexical` object's `status`/`fresh`/`stale`/`reason` fields:

```rust
if !assets.lexical.rebuilding
    && last_scan_ts.is_some_and(|scan_ts| { ... scan_ts > indexed_at + 1000 ... })
{
    assets.lexical.status = "stale";
    assets.lexical.fresh = false;
    assets.lexical.stale = true;
    assets.lexical.status_reason = Some("last_scan_ts is newer than last_indexed_at; ...".to_string());
}
```
(`src/lib.rs:16287-16300`), and `status_reason` is serialized as the golden's
`"reason"` key (`src/lib.rs:16475`: `"reason": lexical.status_reason`). So
**any honest-tri-state work on `last_scan_ts` surfaces, if at all, in the same
5 goldens already carrying a `lexical` object**: `doctor.json.golden`,
`doctor_quarantine.json.golden`, `doctor_shape.json.golden`,
`introspect.json.golden`, `introspect_shape.json.golden`.

A further, non-obvious point: **none of the 37 goldens currently exercises
the "stale" branch this code drives.** Both goldens with a concrete
(non-schema) `lexical` object pin `"status": "missing"` — `doctor.json.golden:680`
and `doctor_quarantine.json.golden:680` — which is a different branch (no
Tantivy index present at all), not the `last_scan_ts`-driven `"stale"` branch.
`doctor_shape.json.golden` and the two `introspect*` files are schema-only
(`"type": "string"` for `status`, no branch-specific value). **So the 5
lexical-bearing goldens pin the field *shape* that `last_scan_ts`'s derived
output lives inside, but zero of the 37 pin its actual stale-branch values**
— adding real tri-state there would need either a new fixture (to first give
any golden coverage of that branch) or would only touch the 5 goldens' shape
if new keys are added, not their currently-pinned values.

---

## Proof boundary — what this lane did NOT establish

- **No code was read for how a "tri-state" would actually be designed** (e.g.
  whether the third state is a new enum value, a new boolean flag, or a
  restructured object). This lane reports what exists today and where the
  four surfaces currently live; it does not propose or evaluate an
  implementation.
- **`introspect.json.golden`/`introspect_shape.json.golden` were sampled, not
  exhaustively traced.** These are 34,599 and 68,550 lines respectively and
  embed the full self-documentation schema for every robot command. I opened
  and confirmed every `"database"`/`"db"`/`"lexical"`/`"counts_skipped"` hit
  `rg` returned (6+1 in introspect.json.golden, matching counts in
  introspect_shape.json.golden), but I did not verify there is no *seventh*
  occurrence of any pattern under a spelling `rg` with these exact patterns
  would miss (e.g. a differently-cased key, or a key reached only through a
  JSON Schema `$ref`/`oneOf` I did not expand). The per-file table's entries
  for these two files should be read as "confirmed present, confirmed which
  branches," not "every last occurrence enumerated."
- **I did not run the test suite.** This lane is read-only per its hard
  limits (no `cargo test`/`cargo build`/`cargo check`, no `cass`). All
  "would change" claims are derived from reading the comparison code and the
  golden bytes, not from an executed mutation-and-rerun. If a stronger
  falsifier is wanted, the mutant-testing pattern in
  `~/.agent-config/.claude/rules/no-vacuous-test-guards.md` — flip
  `counts_skipped` in a fixture, or emit a `"checked": true` scan, and rerun
  `cargo test --test golden_robot_json` — would confirm this table
  empirically; that rerun is outside this lane's charter.
- **I did not check whether any golden outside `tests/golden/robot/` (the
  other ~39 `*golden*` paths found by the initial `find`) also encodes any of
  these four surfaces.** The task scoped this lane to the 37-case
  `golden_robot_json.rs` suite specifically, and that is what this table
  covers. `tests/golden/metamorphic/`, `tests/golden/regression/`, and
  `tests/golden/swarm_status/` were not opened.
- **I did not determine whether `stats`'s bare top-level `conversations`/
  `messages` (no wrapper, no `counts_skipped`) is an intentional distinct
  contract or a drift from the status/health shape.** I report it as a third,
  structurally different shape; whether it *should* be unified with
  `state_db_count_json` is a design question, not a fact this lane can settle
  from reading alone.
- **The claim "capabilities.json.golden's two `\"lexical\"` hits are enum-list
  members, not an object" and the four semantic-progress/backfill files'
  single `fallback_mode` hits were confirmed by reading the matched line plus
  1-2 lines of context, not the full surrounding object.** I'm confident in
  the classification (a bare string value or array element cannot also be an
  object with `status`/`fresh` sub-keys at the same JSON path), but I did not
  read each of those five files end-to-end.
