# Lane C — the degrade precedent (46d74410) and doctor's output-schema constraints

Read-only audit lane. Repo: `/Users/dalecarman/dev/coding_agent_session_search/.claude/worktrees/cass-p3kgr-gen13`, branch `worktree-cass-p3kgr-gen13`. No source edited, no cargo run. All line numbers are the current working tree unless the text says "in the commit".

---

## PART 1 — How `cass status`'s honest degrade is expressed, end to end

The degrade is **not one field**. It is a three-layer chain: a size gate returns a boolean, the boolean selects a *pre-existing* "unchecked" summary struct instead of running the collector, and three separate renderers translate that struct's `confidence_tier`/`status` into operator-facing strings. Commit 46d74410 added **only the gate**; every string below already existed. That is the load-bearing property the doctor fix should copy — the commit body says so in its own words: *"Nothing downstream is invented: `doctor_fast_coverage_risk_unchecked` already reports 'not_checked' and routes to `cass doctor --json`."*

### Layer 1 — the gate (the only thing 46d74410 added to the degrade path)

| what | where |
|---|---|
| `const STATUS_COVERAGE_MAX_ARCHIVE_DB_BYTES_DEFAULT: u64 = 256 * 1024 * 1024;` | src/lib.rs:15107 |
| `const CASS_STATUS_COVERAGE_MAX_ARCHIVE_DB_BYTES: &str = "CASS_STATUS_COVERAGE_MAX_ARCHIVE_DB_BYTES";` | src/lib.rs:15108 |
| `fn status_coverage_max_archive_db_bytes() -> u64` — `dotenvy::var(...).and_then(parse::<u64>).unwrap_or(DEFAULT)` | src/lib.rs:34471–34476 |
| `fn status_archive_scan_too_large(db_path: &Path) -> bool` — one `std::fs::metadata`; `NotFound => false`; any other `Err => true` (fails toward declining) | src/lib.rs:34493–34499 |
| call site: `let status_collects_coverage = db_exists && !status_raw_mirror_scan_too_large(&data_dir) && !status_archive_scan_too_large(&db_path);` | src/lib.rs:66077–66079 |

Siblings it was modelled on, all pre-existing: `STATUS_COUNT_SCAN_MAX_DB_BYTES = 256 MiB` (src/lib.rs:15070), `STATUS_COVERAGE_MAX_RAW_MIRROR_MANIFESTS = 512` (src/lib.rs:15084), `status_raw_mirror_scan_too_large` (src/lib.rs:34441), and the env-override shape of `doctor_raw_mirror_size_warn_threshold_bytes` (src/lib.rs:34411).

### Layer 2 — the branch: a struct that *means* "not checked"

src/lib.rs:66080–66092 — a 3-tuple `(coverage_risk, coverage_source, coverage_checked)`:

- gate passes → `(collect_doctor_coverage_risk_summary(&data_dir, &db_path), "status-inline-small-archive", true)`
- gate fires → `(doctor_fast_coverage_risk_unchecked(db_exists), "status-fast-state", false)`

`doctor_fast_coverage_risk_unchecked` (src/lib.rs:37217–37233) returns a `DoctorCoverageRiskSummary` (struct at src/lib.rs:30207–30221, `schema_version: u32, status, confidence_tier, …, recommended_action`) carrying:

- `status`: `"unchecked_fast_health"` when the db exists, `"not_initialized"` when it does not — src/lib.rs:37220–37224
- `confidence_tier`: `"unchecked"` — src/lib.rs:37225
- **`recommended_action`: `"Run 'cass doctor --json' for source coverage and sole-copy analysis."`** — src/lib.rs:37227 (this is the exact string the commit's proof calls "a recommended action pointing at `cass doctor`"); the not-initialized arm is `"Run 'cass index --full' once before coverage can be assessed."` — src/lib.rs:37229
- every count field left at `Default` — i.e. zero, but explicitly *unread rather than measured*, which is why the renderers below suppress them.

### Layer 3 — the three renderer functions that turn that into JSON string values

All in `build_doctor_runtime_summary` (src/lib.rs:37330–37535), fed by `DoctorRuntimeSummaryInput { coverage_risk, coverage_source, coverage_checked, … }` (struct src/lib.rs:37235–37250).

| JSON key | value on degrade | producer |
|---|---|---|
| `doctor_summary.archive_coverage_state` | `"not_checked"` | `doctor_summary_coverage_state` — `status if status.starts_with("unchecked") => "not_checked"` — src/lib.rs:37260; emitted at src/lib.rs:37508 |
| `doctor_summary.source_mirror_state` | `"not_checked"` | `doctor_summary_source_mirror_state` — `else if coverage_risk.confidence_tier == "unchecked"` — src/lib.rs:37268–37269; emitted at src/lib.rs:37509 |
| `doctor_summary.risk_level` | `"unknown"` | `doctor_summary_risk_level` — src/lib.rs:37286–37287; emitted at src/lib.rs:37462 |
| `doctor_summary.coverage_source.status` | `"not_checked"` | inline ternary on `input.coverage_checked` — src/lib.rs:37515 |
| `doctor_summary.coverage_source.source` | `"status-fast-state"` | passed through from src/lib.rs:66089 → src/lib.rs:37516 |
| `doctor_summary.coverage_source.confidence_tier` | `"unchecked"` | src/lib.rs:37517 |
| `doctor_summary.coverage_source.stale_after_seconds` | `0` (vs `300` when checked) | src/lib.rs:37522 |
| `doctor_summary.coverage_source.recommended_action` | `"Run cass doctor check --json for current archive coverage; health/status did not run deep collectors."` | src/lib.rs:37524–37530 |
| `doctor_summary.recommended_action` | `"Run 'cass doctor check --json' to refresh archive coverage and repair readiness."` | `doctor_check_recommended` is forced true by `!input.coverage_checked` — src/lib.rs:37361–37368, string at src/lib.rs:37379–37381, emitted at src/lib.rs:37472 |
| `doctor_summary.status` | `"skipped"` | `else if !input.initialized \|\| !input.coverage_checked` — src/lib.rs:37429–37430 |
| `doctor_summary.coverage_delta.*` | `"unknown"` + every count `null` | `coverage_known = coverage_checked && confidence_tier != "unchecked"`, then `coverage_known.then_some(...)` per field — src/lib.rs:37403–37421 |
| `doctor_summary.operation_outcome.next_command` | `"cass doctor check --json"` | src/lib.rs:37440–37441 |
| `doctor_summary.operation_outcome.reason` | `"health/status used bounded readiness evidence and recommends doctor check for full archive coverage"` | src/lib.rs:37481–37482 |
| top-level `coverage_risk` (the whole struct, incl. `recommended_action` above) | serialized verbatim | src/lib.rs:66171 |

### `counts_skipped` — a SEPARATE, older mechanism on the same command

The commit's proof line mentions `counts_skipped: true` alongside `archive_coverage_state: "not_checked"`, and they are unrelated code paths. `counts_skipped` is a `StateDbSnapshot` field (src/lib.rs:15407) gated by `STATUS_COUNT_SCAN_MAX_DB_BYTES` at src/lib.rs:16430–16434 (`db_size_bytes <= 256 MiB`), read back at src/lib.rs:65837–65841 (defaulting to `true` when absent — fail toward "we did not measure"), emitted at `database.counts_skipped` src/lib.rs:66158, and the counts themselves render as JSON `null` rather than `0` via `state_db_count_json` (src/lib.rs:17084–17090). Human line: `"  Counts skipped for fast status on large database"` — src/lib.rs:66243–66244. The 23 GB archive trips this too, which is why both appear in the same proof.

**Shape summary the doctor fix must mirror:** one `stat`, one `u64` default constant, one `CASS_*` env override for fixture-scale testing, an unreadable-target failing *toward* declining, and — critically — **routing the degrade into vocabulary that already exists**, so the skipped work reports as "not checked / unknown" rather than as zero, healthy, or measured.

### The tests 46d74410 shipped for the gate (the pinning shape)

`tests/cli_doctor.rs:4724–4776`, `status_json_declines_inline_coverage_on_an_archive_past_the_db_scan_cap`:

- forces the gate with `.env("CASS_STATUS_COVERAGE_MAX_ARCHIVE_DB_BYTES", "1")` (tests/cli_doctor.rs:4739);
- asserts the fixture actually exceeds the cap first, with the stated reason *"or this test proves nothing"* (tests/cli_doctor.rs:4733–4736);
- four targeted field reads: `coverage_source.source == "status-fast-state"`, `coverage_source.status == "not_checked"`, `archive_coverage_state == "not_checked"`, and `coverage_risk.recommended_action` **contains** `"cass doctor"` (tests/cli_doctor.rs:4755–4775);
- its doc comment names the matched negative control — `status_json_still_verifies_coverage_inline_on_a_raw_mirror_under_the_scan_cap` (tests/cli_doctor.rs:4711) — whose archive is far under the default cap, so a gate stuck ON goes red. Both directions are pinned by a pair, not by one test.

---

## PART 2 — Doctor's own output: schema, versioning, goldens

### The struct: free-form `Vec`, but with a typed policy join

Two structs, and the distinction matters.

**`Check`** — declared *inside* `run_doctor_impl`, src/lib.rs:69337–69344:

```rust
#[derive(serde::Serialize)]
struct Check { name: String, status: String, /* "pass", "warn", "fail" */ message: String,
               fix_available: bool, fix_applied: bool }
```

Built through the `add_check!` macro (src/lib.rs:69357–69367), pushed into `let mut checks: Vec<Check>` (src/lib.rs:69346). **`status` is a bare `String` with no enum, no validator, no `TryFrom`.** A new status value is legal at the type level everywhere.

**`DoctorCheckReport`** — the thing actually serialized, src/lib.rs:29–44 of the doctor block at src/lib.rs:25029–25044: `name, status, message, anomaly_class, health_class, severity, affected_asset_class, data_loss_risk, recommended_action, safe_for_auto_repair, default_outcome_kind, fix_available, fix_applied`. Every `Check` is mapped into one at src/lib.rs:71311–71322 by `doctor_check_report` (src/lib.rs:71592–71616 → definition at src/lib.rs:25592–25616), which derives the eight typed fields by looking up `doctor_anomaly_for_check(name, status, message)` (src/lib.rs:25512–25590) in `DOCTOR_ANOMALY_POLICY_TABLE`.

**Answer to the question as asked:** the list is a free-form `Vec<{name, status, message, …}>` for the *author* of a check — you add one with a two-line `add_check!` and nothing forces a schema change. But `status` is not free at the *semantic* layer: `doctor_anomaly_for_check` short-circuits `if status == "pass" { return Healthy }` (src/lib.rs:25513) and otherwise dispatches **on `name`, not on `status`**, with a catch-all `_ => DoctorAnomaly::RepairBlocked` (src/lib.rs:25588). So a novel status string on an existing check name silently inherits that name's existing anomaly row. There is no typed status enum to extend.

Places a new status string would be read and would *not* do what you want:

| src/lib.rs | consequence of a status that is not `pass`/`warn`/`fail` |
|---|---|
| 71307–71308 | not counted in `fail_count` or `warn_count` → `issues_found` misses it |
| 71472 | `all_pass` becomes false → human epilogue at 71648 changes |
| 71473–71479 | `healthy`/`doctor_status` unaffected (they key off `fail_count`) |
| 71604–71608 | human renderer icon falls to `_ => "?"` — degrades safely, no panic |
| 32128 | `doctor_recommended_action_for_reports` picks the first `status != "pass"` check with a non-`none` action — a new status IS eligible here |
| 32101 | `doctor_risk_level_for_reports` bumps to `"low"` on any `status == "warn"` — a new status is NOT eligible |
| 26042, 26134, 26169, 49185, 49205 | safe-auto-run / incident collection filters, all spelled `!= "pass"` or `== "fail"` |

### schema_version

- **Top-level doctor envelope: `"schema_version": 2`, hardcoded at src/lib.rs:71504**, with `"doctor_contract_version": 1` at src/lib.rs:71505. The comment above it (src/lib.rs:71498–71503) says it was bumped to 2 for the per-run artifact directory + undo surface, that sub-reports keep their own `schema_version` (typically 1), and that it is *"Pinned by the `doctor_top_level_schema_version_present` golden-test contract."*
- **Null result, and it is a real one: that named test does not exist.** `rg` over the whole repo returns exactly one hit for `doctor_top_level_schema_version_present`, and it is that comment itself (src/lib.rs:71503). No test anywhere asserts the top-level doctor `schema_version == 2`; the only thing pinning the literal `2` is the byte-equality goldens below. The comment is stale and should not be trusted as a pointer.
- **Does adding a field require a bump?** Nothing mechanical forces one — there is no validator, no `#[serde(deny_unknown_fields)]` consumer, and no test comparing emitted keys against the declared schema. The stated convention (src/lib.rs:71498–71501) is that `schema_version` is an *envelope* version for agents to gate parsing on, and an additive field does not break a gating consumer. **Recommendation: do not bump it for an additive field, and do not bump it silently either way** — a bump would itself change all three doctor goldens with no consumer that reads it.
- Sub-report versions that do exist and are asserted: `check_scope.schema_version: 1` (src/lib.rs:32176), `DoctorCoverageRiskSummary.schema_version: 1` (src/lib.rs:37219), plus `tests/cli_doctor.rs:1906/1986/2041` asserting `2` on the *baseline save/diff/update* payloads (different surfaces, not the doctor envelope).

### Goldens — exactly which assertions are whole-document equality

`assert_golden` (tests/golden_robot_json.rs:926–966) is **plain `actual != expected` string comparison over the entire pretty-printed document** (tests/golden_robot_json.rs:950). Regeneration is `UPDATE_GOLDENS=1` (tests/golden_robot_json.rs:932).

**Whole-document exact equality, doctor-side:**

| test | golden | notes |
|---|---|---|
| `doctor_json_matches_golden` (tests/golden_robot_json.rs:1993–2001) | `tests/golden/robot/doctor.json.golden` (120 KB, 59 top-level keys, 17 checks) | full instance freeze on a **fresh empty tempdir** |
| `doctor_shape_matches_golden` (tests/golden_robot_json.rs:2038–2048) | `tests/golden/robot/doctor_shape.json.golden` (78 KB) | equality over `json_value_schema(doctor)` — types only |
| `doctor_quarantine_json_matches_golden` (tests/golden_robot_json.rs:1302–1327) | `tests/golden/robot/doctor_quarantine.json.golden` (130 KB) | same 17 checks, same anomaly classes |
| `introspect_json_matches_golden` (tests/golden_robot_json.rs:1589–1607) | `tests/golden/robot/introspect.json.golden` (1.0 MB) | contains the doctor **response schema** (src/lib.rs:78130–78232); `"pass \| warn \| fail"` occurs there exactly once |
| `introspect_shape_matches_golden` (tests/golden_robot_json.rs:1611–1620) | `tests/golden/robot/introspect_shape.json.golden` (3.0 MB) | |
| `robot_docs_schemas_matches_golden` (tests/golden_robot_docs.rs:112–114) | `tests/golden/robot_docs/schemas.txt.golden` | renders the doctor response schema's **top-level** keys alphabetically; line 97 is a bare `- checks: array` with no item detail |

**Targeted field reads (safe against additive change):**

- `tests/cli_doctor.rs:616–638` (`doctor_json_fails_when_full_integrity_check_finds_archive_corruption`) — finds the check named `database`, asserts `status == "fail"`, `anomaly_class == "archive-db-corrupt"`, `message` **contains** `"integrity_check"`, plus `healthy == false`, `health_class == "degraded-archive-risk"`, `needs_rebuild == true`.
- `tests/cli_doctor.rs:1752–1779` — pointer-existence loop over 12 pointers including `/check_scope/skipped_expensive_collectors` and `/checks`, then an `.any(...)` search for the `network_source_sync` collector. Additive-safe.
- `tests/cli_robot.rs:4377–4498`, `6050+` — pointer reads only.
- `tests/cli_robot.rs:2277–2297` (`capabilities_matches_golden_contract`) IS whole-document equality — but against `capabilities.json.golden`, which **does not carry `response_schemas`** (verified: `response_schemas` is an `introspect` key, src/lib.rs:73609–73616, and is absent from the capabilities golden). Doctor schema edits do not reach it.
- `tests/fixtures/cli_contract/introspect.json` is **orphaned** — `tests/fixtures/README.md:91` names `tests/e2e_cli_contract.rs` as its consumer and that file does not exist; the only `read_fixture` call in the repo (tests/cli_robot.rs:2311) reads `api_version.json`. It will not fail.

**Concrete blast radius, by kind of change:**

1. **New field on `DoctorCheckReport`** → serialized on *every* check → `doctor.json.golden` + `doctor_quarantine.json.golden` red (17 entries each), and `doctor_shape.json.golden` red too, because `json_value_schema` derives an array's item schema from `values.first()` only (tests/golden_robot_json.rs:216–225) and the first check (`operation_state`) carries it.
2. **New top-level doctor key** → all three doctor goldens red, plus `schemas.txt.golden` if the response schema at src/lib.rs:78130–78232 is extended to declare it.
3. **Editing the doctor response schema** (properties, the `"pass | warn | fail"` description at src/lib.rs:78214, or the `required` array at src/lib.rs:78231) → `introspect.json.golden` red; `introspect_shape.json.golden` only if a *type* changes; `schemas.txt.golden` only for top-level keys.
4. **A new entry appended to `check_scope.skipped_expensive_collectors`** → doctor instance goldens red **only if it is emitted unconditionally**. The doctor golden fixture is a fresh tempdir with **no database** (its `database` check reads `"Database not initialized yet - no archive has been created in this data dir"`, src/lib.rs:69735–69741), so an entry emitted only when an existing archive exceeds a byte cap changes **nothing** in any golden. `doctor_shape.json.golden` is also unaffected either way — first-element-only item schema, and the entry would share `{name, status, next_action}`.

**The hard constraint on any integrity-check size gate.** `tests/cli_doctor.rs:588+`, `doctor_json_fails_when_full_integrity_check_finds_archive_corruption`, builds its fixture (`corrupt_unused_secondary_index_entry`) specifically so that `PRAGMA quick_check(1)` returns `"ok"` and full `PRAGMA integrity_check` does not — it asserts both properties on the fixture before running doctor (tests/cli_doctor.rs:566–586). If the fix skips `integrity_check` past a threshold, **that fixture must stay under the threshold**, which the env override (mirroring `CASS_STATUS_COVERAGE_MAX_ARCHIVE_DB_BYTES`) gives you for free. Skipping `quick_check` as well would additionally strand the `"database failed frankensqlite quick_check"` arm at src/lib.rs:69694–69708.

### Renderers

- **Robot:** `output_structured_value` (src/lib.rs:22932–22949) — `RobotFormat::Json` pretty-prints, `Jsonl|Compact|Sessions` compact-print, `Toon` encodes via the `toon` crate. All generic over the `serde_json::Value`; **no renderer has per-key handling, so none needs updating for a new field.**
- **Human:** src/lib.rs:71598–71634 — one loop over `checks` with `match check.status.as_str()` for the icon and a `_ => "?"` fallback (src/lib.rs:71608), so an unknown status renders as `?` rather than panicking; passing checks are hidden without `--verbose` (src/lib.rs:71612). `check_scope`/`skipped_expensive_collectors` are **not rendered in human output at all** — if the degrade lands only there, a human `cass doctor` run says nothing about it.

---

## PART 3 — `DoctorAnomaly` / `DOCTOR_ANOMALY_POLICY_TABLE`: is there a "could not be run" class?

Enum: src/lib.rs:24961–24984, 20 variants, `#[serde(rename_all = "kebab-case")]`. Table: src/lib.rs:25203–25404, one row per variant. Lookup: `doctor_anomaly_policy` (src/lib.rs:25406–25411) — `.find(...).expect("doctor anomaly policy table must cover every anomaly")`.

### Is there an existing class meaning "a check could not be run"? — **No.**

Every variant names a *state of the archive or the repair machinery*, not a state of the *measurement*. The "not checked" vocabulary in this codebase lives entirely on the **coverage/summary** side (`doctor_summary_coverage_state` → `"not_checked"`, src/lib.rs:37260; `confidence_tier: "unchecked"`, src/lib.rs:37225) and on the **check-scope** side (`skipped_expensive_collectors[].status == "not_checked"`, src/lib.rs:32153/32158/32163/32170/32184) — never inside `checks[].anomaly_class`.

### Closest candidates, with their full policy rows

| candidate | policy row | health_class | severity | asset class | data_loss_risk | outcome | auto-repair | recommended_action | why it does / does not fit |
|---|---|---|---|---|---|---|---|---|---|
| `RepairBlocked` | src/lib.rs:25234–25243 | `repair-blocked` | `warn` | `operation_receipt` | **`unknown`** | `blocked` | false | `inspect-blocker-before-retrying` | **closest.** It is already the catch-all (`_ =>` at src/lib.rs:25588), and `data_loss_risk: unknown` is the only row in the table that honestly says "we do not know". But its asset class is the *operation receipt*, its action tells the reader to inspect a blocker that does not exist, and it drives top-level `risk_level` to `"medium"` (src/lib.rs:32090–32097 treats `Unknown` like `Medium`). |
| `ArchiveDbUnreadable` | src/lib.rs:25274–25283 | `degraded-archive-risk` | **`error`** | `canonical_archive_db` | **`high`** | `blocked` | false | `inspect-archive-db-and-preserve-sidecars` | the class doctor already uses for "we could not read the archive" (`"could not query archive coverage"`, src/lib.rs:25553) and for a *missing* db (src/lib.rs:69735–69741). **Wrong for a declined check:** `high` data-loss risk + `error` severity + `degraded-archive-risk`, and `doctor_health_class_for_checks` (src/lib.rs:26277–26282) promotes that to the top-level `health_class` for the whole report. Declining to run a probe would report the archive as at high risk of data loss. That is exactly the lie the status precedent avoided. |
| `LockContention` | src/lib.rs:25344–25353 | `repair-blocked` | `warn` | `operation_receipt` | `low` | `blocked` | false | `wait-or-inspect-active-owner-before-repair` | right severity band, but its whole meaning is "someone else holds it"; the action is actively wrong. |
| `StoragePressure` | src/lib.rs:25354–25363 | `repair-blocked` | `warn` | `unknown` | `medium` | `blocked` | false | `free-space-without-deleting-archive-evidence` | `affected_asset_class: unknown` is the only "we can't attribute this" asset value in the table, but the action is about disk space. |
| `Healthy` | src/lib.rs:25204–25213 | `healthy` | `info` | `unknown` | `none` | `no_op` | false | `none` | **automatic** for any check emitted with `status == "pass"` (src/lib.rs:25513) regardless of name. Reporting a skipped integrity probe as `pass`/`healthy` is the silent-failure outcome this whole bead exists to prevent. |

### Cost of adding a new variant

Adding e.g. `DoctorAnomaly::IntegrityCheckNotRun`:

**Compile errors — exactly one match arm becomes non-exhaustive.** `DoctorAnomaly` is referenced 91 times in src/lib.rs and 0 times anywhere else in `src/`. Only two functions take or return it (`doctor_anomaly_policy` src/lib.rs:25406, `doctor_anomaly_for_check` src/lib.rs:25512) and neither matches exhaustively. The single exhaustive `match` on the enum is:

- **`doctor_incident_kind_for_check`, `match check.anomaly_class {` at src/lib.rs:25912–25965** — no wildcard arm; last arm `DoctorAnomaly::PrivacyRedactionRequired => …Unknown` at src/lib.rs:25964. You would add an arm here, almost certainly mapping to `DoctorIncidentRootCauseKind::Unknown` alongside `SourceAuthorityUnsafe | RepairBlocked` (src/lib.rs:25961–25963).

`DoctorHealth` has **no** exhaustive `match` anywhere (only `==` comparisons in `doctor_health_class_for_checks`, src/lib.rs:26270–26302, and `matches!` guards), so reusing an existing health class costs nothing there.

**Runtime panic if you forget the table row.** `doctor_anomaly_policy`'s `.expect(...)` at src/lib.rs:25410 fires on the first check that maps to the new variant.

**Test failures you must satisfy:**

1. `ALL_DOCTOR_ANOMALIES` (test const, src/lib.rs:54746–54767) must gain the variant, or `doctor_anomaly_taxonomy_explicitly_covers_every_class` (src/lib.rs:54966–55025) fails on the set comparison at 54973 and the length check at 54977 (which also refuses duplicate rows).
2. Same test asserts kebab-case serialization with no `_` (src/lib.rs:54999–55008).
3. `doctor_anomaly_policy_fails_closed_for_precious_assets` (src/lib.rs:55027+) constrains the new row: `safe_for_auto_repair: true` would require `health_class == DegradedDerivedAssets` **and** `data_loss_risk == None` **and** a non-precious asset. Keep `safe_for_auto_repair: false`.
4. **Three goldens go red unconditionally**, because `doctor_anomaly_taxonomy_report()` (src/lib.rs:25413–25427) serializes the entire table into the top-level `anomaly_taxonomy` key (src/lib.rs:71558) on *every* doctor run: `doctor.json.golden` (currently 20 entries), `doctor_quarantine.json.golden`, and `doctor_shape.json.golden` — the last only if the *first* entry's shape changes, which it does not, so in practice the two instance goldens. This is unavoidable for a new variant and is the one cost a `skipped_expensive_collectors` entry does **not** pay.
5. `doctor_anomaly_for_check` (src/lib.rs:25512–25590) must route something to it, or the variant is unreachable and only shows up in the taxonomy.

### Read on the cheapest honest landing (offered, not decided)

There is a pre-existing surface whose entire purpose is "doctor declined to do this expensive thing": **`doctor_check_scope_report`, src/lib.rs:32146–32199**, emitted at top-level `check_scope` (src/lib.rs:71332, 71533). It already carries three `{ "name": …, "status": "not_checked", "next_action": … }` entries (`full_raw_log_reparse`, `semantic_embedding_deep_integrity`, `network_source_sync`) plus a fourth added conditionally for the `Check` surface (src/lib.rs:32167–32173). Its consumers are additive-safe: `tests/cli_doctor.rs:1771–1779` searches it with `.any(...)`; `tests/doctor_e2e_runner.rs:1500` only requires the pointer to exist; the doctor response schema declares `check_scope` as `response_schema_opaque_object()` (src/lib.rs:78140), so **no schema edit and no introspect/schemas golden churn**. And because the doctor golden fixture has no database, a conditionally-emitted entry leaves every golden byte-identical.

Two things that surface does **not** give you, and they should be weighed rather than assumed away: `check_scope` is absent from the human renderer entirely (the human loop at src/lib.rs:71603 iterates `checks` only), and `doctor_check_scope_report`'s current signature takes only `(command_surface, execution_mode)` — an archive-size-dependent entry needs the db path or a computed boolean threaded in. A `checks[]` entry, by contrast, is visible to humans and to every `status != "pass"` consumer, but drags in either a wrong-meaning anomaly class or the new-variant cost above.

---

## Nulls and caveats

- `doctor_top_level_schema_version_present` — named in a src comment (src/lib.rs:71503) as the contract pinning `schema_version`. **It does not exist**; one repo-wide hit, and it is the comment. The literal `2` is pinned only by byte-equality goldens.
- `tests/fixtures/cli_contract/introspect.json` — its documented consumer `tests/e2e_cli_contract.rs` (tests/fixtures/README.md:91) does not exist. No test reads it.
- Working tree at audit time had `M src/connectors/codex.rs`, `M src/connectors/mod.rs`, `M tests/golden_robot_json.rs` from another session. The golden-test diff is rustfmt-only (one wrapped line at tests/golden_robot_json.rs:356) and does not affect anything above. Not touched.
- No build or test was executed in this lane, so every claim here is source-read and golden-file-read, not runtime-observed. The three golden files were parsed with `python3 -c` (read-only) to count keys and entries.
- `tests/golden/robot/introspect.json.golden` is not parseable as a single JSON document (`json.load` → `Extra data: line 28013`). I did not chase that; the `"pass | warn | fail"` occurrence count in it was established by `rg -c`, which does not depend on parsing.
