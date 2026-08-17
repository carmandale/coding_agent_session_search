# Lane D — test conventions in `tests/cli_doctor.rs`

Read-only audit. Repo: `/Users/dalecarman/dev/coding_agent_session_search/.claude/worktrees/cass-p3kgr-gen13`, branch `worktree-cass-p3kgr-gen13`. All line numbers are against the working tree; `tests/cli_doctor.rs` is clean (not in `git status`), so they also match HEAD.

The three tests added by commit `46d74410` now live at `tests/cli_doctor.rs:4725`, `:4789`, and `:4884`.

---

## 1. Fixture data dir and archive database

Two helpers, both in this file, and every doctor/status test in the file starts with the same two lines.

- `cass_cmd(test_home: &Path) -> Command` — `tests/cli_doctor.rs:204`
- `seed_healthy_empty_index(test_home: &Path, data_dir: &Path)` — `tests/cli_doctor.rs:215`
- `ensure_codex_agent(conn: &FrankenConnection) -> i64` — `tests/cli_doctor.rs:473` (returns the `agents.id` for slug `codex`, inserting the row if absent)

`seed_healthy_empty_index` does not hand-build a database. It shells out to the real binary — `cass index --force-rebuild --json --data-dir <data_dir>` — so the schema is whatever the product creates, and the archive lands at `<data_dir>/agent_search.db` (`tests/cli_doctor.rs:4733`, `:4802`, `:4897`). Rows are then inserted directly with `frankensqlite`.

Verbatim, the complete minimal example — the opening of `doctor_json_does_not_hash_live_sources_without_existing_mirror_evidence`, `tests/cli_doctor.rs:4884-4914`:

```rust
#[test]
fn doctor_json_does_not_hash_live_sources_without_existing_mirror_evidence() {
    let temp = tempfile::tempdir().expect("tempdir");
    let test_home = temp.path();
    let data_dir = test_home.join("cass-data");
    seed_healthy_empty_index(test_home, &data_dir);

    let session_dir = test_home.join(".codex/sessions/no-evidence");
    fs::create_dir_all(&session_dir).expect("session dir");
    let live_source = session_dir.join("unmirrored-session.jsonl");
    fs::write(&live_source, b"{\"type\":\"message\",\"role\":\"user\"}\n")
        .expect("write live source");

    let db_path = data_dir.join("agent_search.db");
    let conn = FrankenConnection::open(db_path.to_string_lossy().into_owned()).expect("open db");
    let agent_id = ensure_codex_agent(&conn);
    let live_source_str = live_source.to_string_lossy().into_owned();
    conn.execute_compat(
        "INSERT INTO conversations (id, agent_id, source_id, external_id, title, source_path, started_at, last_message_idx)
         VALUES (401, ?1, 'local', 'no-mirror-evidence', 'no mirror evidence', ?2, 1700000000000, 0)",
        frankensqlite::params![agent_id, live_source_str.as_str()],
    )
    .expect("insert conversation");
    conn.execute_compat(
        "INSERT INTO messages (conversation_id, idx, role, content)
         VALUES (401, 0, 'user', 'archived message')",
        frankensqlite::params![],
    )
    .expect("insert message");
    drop(conn);
```

Conventions visible here and worth copying: `tempfile::tempdir()` bound to a local named `temp` so it outlives the test body; `test_home` is the *whole* fake `$HOME` and `data_dir` is a child of it; explicit `drop(conn)` before invoking the CLI (the subprocess opens the same file); provider session fixtures under `<test_home>/.codex/sessions/<something>/`; hard-coded conversation ids in a per-test band (301/302, 401, 501) so `find(|r| r["conversation_id"] == Some(401))` is unambiguous.

Imports the new tests rely on are already at the top of the file (`tests/cli_doctor.rs:1-12`): `assert_cmd::Command`, `frankensqlite::Connection as FrankenConnection`, `frankensqlite::compat::{ConnectionExt, RowExt}`, `serde_json::{Value, json}`, `std::fs`.

Also available, and directly relevant to a doctor-integrity lane: `corrupt_unused_secondary_index_entry(db_path: &Path)` at `tests/cli_doctor.rs:494`. It builds a probe table + index, mutates one byte of the index root page, and then asserts BOTH directions itself — `quick_check(1)` still returns `"ok"` (`:573-577`) while `PRAGMA integrity_check` does not (`:578-587`). That helper is the existing fixture for archive corruption, and its two internal assertions are the file's own record that `quick_check` and `integrity_check` differ in *detection*, not in cost.

## 2. Env override for one invocation; in-process or subprocess?

**Subprocess.** `cass_cmd` returns an `assert_cmd::Command` built from the real built binary — `tests/cli_doctor.rs:204-214`, verbatim:

```rust
fn cass_cmd(test_home: &Path) -> Command {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("cass"));
    cmd.env("CODING_AGENT_SEARCH_NO_UPDATE_PROMPT", "1")
        .env("CASS_IGNORE_SOURCES_CONFIG", "1")
        .env("XDG_DATA_HOME", test_home)
        .env("XDG_CONFIG_HOME", test_home)
        .env("HOME", test_home)
        .current_dir(test_home);
    cmd
}
```

A per-invocation override is one more `.env()` chained onto that builder before `.args(...).output()`. From `status_json_declines_inline_coverage_on_an_archive_past_the_db_scan_cap`, `tests/cli_doctor.rs:4738-4748`:

```rust
    let status_out = cass_cmd(test_home)
        .env("CASS_STATUS_COVERAGE_MAX_ARCHIVE_DB_BYTES", "1")
        .args([
            "status",
            "--json",
            "--data-dir",
            data_dir.to_str().expect("utf8"),
        ])
        .output()
        .expect("run cass status --json");
```

Because it is a subprocess, the override is scoped to that one child process — no test-ordering hazard, no `std::env::set_var`. There are exactly three `.env("CASS…")` call sites in the file: the two baseline ones inside `cass_cmd` (`:206-207`), `CASS_LEXICAL_PUBLISH_BACKUP_RETENTION` at `:1276`, and the new `CASS_STATUS_COVERAGE_MAX_ARCHIVE_DB_BYTES` at `:4739`.

The product side reads it with `dotenvy::var` — `status_coverage_max_archive_db_bytes()` at `src/lib.rs:34471-34476` — which reads the process environment, so `.env()` on the child is the right and only lever.

## 3. Making the gate trip without writing gigabytes

The shipped test does **not** grow the database. It shrinks the cap to 1 byte via the env override and then proves the fixture is on the far side of it. `tests/cli_doctor.rs:4725-4737` verbatim:

```rust
#[test]
fn status_json_declines_inline_coverage_on_an_archive_past_the_db_scan_cap() {
    let temp = tempfile::tempdir().expect("tempdir");
    let test_home = temp.path();
    let data_dir = test_home.join("cass-data");
    seed_healthy_empty_index(test_home, &data_dir);

    let db_path = data_dir.join("agent_search.db");
    let db_bytes = fs::metadata(&db_path).expect("archive metadata").len();
    assert!(
        db_bytes > 1,
        "fixture must actually exceed the archive scan cap or this test proves nothing (db is {db_bytes} bytes)"
    );
```

That `assert!(db_bytes > 1, ...)` is a positive control on the fixture, and the message says so in as many words. The same shape appears in the older manifest-count sibling at `tests/cli_doctor.rs:4618-4621` ("fixture must actually exceed the scan cap or this test proves nothing"). Copy both halves: lower the cap by env, then assert the measured fixture value is past the lowered cap, in the same test, before invoking the CLI.

The gate itself is one `stat` — `status_archive_scan_too_large` at `src/lib.rs:34493-34499`, `metadata.len() > status_coverage_max_archive_db_bytes()`, with `Err(NotFound) => false` and any other `Err` => `true`. Default `STATUS_COVERAGE_MAX_ARCHIVE_DB_BYTES_DEFAULT = 256 * 1024 * 1024` at `src/lib.rs:15107`, env name constant at `:15108`. A doctor-side twin should follow `doctor_raw_mirror_size_warn_threshold_bytes()` at `src/lib.rs:34411-34416`, which is the same `dotenvy::var(...).parse::<u64>().unwrap_or(DEFAULT)` shape.

## 4. Assertion style

Overwhelmingly **serde_json pointer/index reads compared by `assert_eq!` against `Some(...)`**, with the whole payload interpolated into the failure message via `{payload:#}`. Substring `.contains(...)` is used only for prose fields (recommended actions, human stdout). There is no `assert_json_eq`/whole-value equality anywhere in this file.

Example A — exact string equality on a typed field, `tests/cli_doctor.rs:4749-4764`:

```rust
    let status_payload: Value = serde_json::from_slice(&status_out.stdout).expect("status json");
    assert_eq!(
        status_payload["doctor_summary"]["coverage_source"]["source"].as_str(),
        Some("status-fast-state"),
        "status must take the fast path on an archive past the db scan cap: {status_payload:#}"
    );
    assert_eq!(
        status_payload["doctor_summary"]["coverage_source"]["status"].as_str(),
        Some("not_checked"),
        "status must say coverage was not checked rather than imply it verified the archive: {status_payload:#}"
    );
```

Example B — substring on a prose field, plus a null assertion, `tests/cli_doctor.rs:4771-4776` and `:4935-4939`:

```rust
    assert!(
        status_payload["coverage_risk"]["recommended_action"]
            .as_str()
            .is_some_and(|text| text.contains("cass doctor")),
        "declining the census must route the operator to the surface that does verify: {status_payload:#}"
    );
```

```rust
    assert!(
        snapshot["content_blake3"].is_null(),
        "with no mirror evidence to compare against, the live bytes must not be read and hashed: {receipt:#}"
    );
```

Other conventions in the same family, all present in the new tests: every CLI invocation is followed by an `assert!(out.status.success(), "… stdout={} stderr={}", …)` before the payload is parsed (`:4741-4746`, `:4837-4842`, `:4918-4923`); array members are located by `.iter().find(|r| r["conversation_id"].as_i64() == Some(301))` rather than by index (`:4855-4858`); presence-in-a-list checks use `.any(...)` (`:1447`, `:1771-1779`), which is why adding a list member never reddens them; and each test carries a doc comment naming the bead, the matched half of the pair, and the ceiling it is stating out loud (`:4713-4724`, `:4778-4788`, `:4874-4883`).

Failure messages are complete sentences that say what the product must do, not what the assertion checked. That is a real convention here, not decoration — "or this test proves nothing" appears twice.

## 5. Test count

`rg -c '#\[test\]' tests/cli_doctor.rs` → **54**. File length 7,699 lines. `rg -n '#\[ignore\]' tests/cli_doctor.rs | wc -l` → **0**, so all 54 are live. This matches the previous generation's recorded 54 passing.

## 6. An existing test that asserts doctor does NOT recommend a rebuild

**Not in the shape you want, in this file.** Null result, stated precisely.

`needs_rebuild` appears exactly once in `tests/cli_doctor.rs`, and it asserts the positive: `assert_eq!(payload["needs_rebuild"].as_bool(), Some(true));` at `tests/cli_doctor.rs:638`, inside `doctor_json_fails_when_full_integrity_check_finds_archive_corruption` (`:590`). There is no `Some(false)` counterpart anywhere in the file.

The negative side of `needs_rebuild` is pinned, but in a different file: `tests/golden/robot/doctor.json.golden:17` and `tests/golden/robot/doctor_quarantine.json.golden:17` both freeze `"needs_rebuild": false` for a fresh empty data dir, consumed by `doctor_json_matches_golden` (`tests/golden_robot_json.rs:1993`) and `doctor_quarantine_json_matches_golden` (`tests/golden_robot_json.rs:1302`).

The three nearest in-file precedents for "doctor declines and says so", in descending order of usefulness for a skip-path test:

1. **`doctor_check_json_reports_read_only_truth_surface_without_writes`, `tests/cli_doctor.rs:1697`** — the closest existing shape by intent. `tests/cli_doctor.rs:1770-1779`:

   ```rust
       assert!(
           payload["check_scope"]["skipped_expensive_collectors"]
               .as_array()
               .is_some_and(|collectors| collectors.iter().any(|collector| {
                   collector["name"].as_str() == Some("network_source_sync")
                       && collector["status"].as_str() == Some("not_checked")
               })),
           "doctor check must report expensive facts as not_checked instead of guessing: {payload:#}"
       );
   ```

   `skipped_expensive_collectors` is built by `doctor_check_scope_report` at `src/lib.rs:32146-32200` and is emitted on **every** doctor surface (`src/lib.rs:71332`, `:71533`), not only `doctor check` — the `Check` surface merely appends a fourth entry (`src/lib.rs:32167-32173`). It already carries three `status: "not_checked"` entries with a `next_action` string. That is the existing, shipped vocabulary for "doctor declined an expensive collector honestly", and a skipped integrity probe is the same kind of thing.

2. **The matched pair at `tests/cli_doctor.rs:4589` / `:4666`** — `status_json_declines_inline_coverage_on_a_raw_mirror_past_the_scan_cap` and `status_json_still_verifies_coverage_inline_on_a_raw_mirror_under_the_scan_cap`. The second exists solely so that "always take the fast path" cannot pass; its doc comment says so at `:4661-4664`. The new gate test at `:4725` explicitly names `:4666` as its matched half (`:4719-4722`). **A new doctor skip-path test needs the same pairing**, and its under-cap half already exists and must stay green: `doctor_json_fails_when_full_integrity_check_finds_archive_corruption` at `:590`, whose fixture (`:494`) is built specifically so `quick_check` passes and `integrity_check` fails.

3. **`doctor_fix_auto_runs_derived_lexical_rebuild_from_readable_archive`, `tests/cli_doctor.rs:757`** — asserts an empty list rather than an absent recommendation, `:836-841`:

   ```rust
       assert!(
           safe_auto["manual_approval_required"]
               .as_array()
               .expect("manual approval actions")
               .is_empty(),
           "derived-only rebuild from a readable archive should not require plan fingerprint approval: {safe_auto:#}"
       );
   ```

## 7. Would any test in this file fail on a new JSON field, or on a changed check status string?

Two different answers.

**A changed status string: yes, loudly, in many places.** There are 33 `assert_eq!(…["status"].as_str(), Some(…))` sites. The ones a doctor-integrity change would touch first:

- `tests/cli_doctor.rs:622` — `assert_eq!(database_check["status"].as_str(), Some("fail"));`, with `:623-626` pinning `anomaly_class == "archive-db-corrupt"` and `:627-632` requiring the message to contain `"integrity_check"`. Changing the `database` check's status string, its anomaly class, or dropping the word `integrity_check` from its message all redden this test.
- `tests/cli_doctor.rs:633-638` — `healthy == false`, `health_class == "degraded-archive-risk"`, `needs_rebuild == true` on the same payload.
- Others in the same style at `:736`, `:3235`, `:3622`, `:3810`, `:3986`, `:4110`, `:4238`, `:4327`, `:4361`, `:4445`.

**A new field: no. Nothing in `tests/cli_doctor.rs` fails when doctor's JSON gains a field.** I looked for the four shapes that would catch it and found none:

- no whole-payload `assert_eq!` between two `Value`s (the `test_canonical_json_value` / `test_doctor_canonical_blake3` helpers at `:14` and `:32` are used only to compute raw-mirror manifest ids at `:120` and `:169`, never to compare a CLI payload);
- no object-key-set or key-count assertion (`.as_object()` appears at `:6124`, `:7159`, `:7192`, `:7311`, and every one indexes named keys);
- no `.len()` assertion over a doctor payload's fields;
- the list-membership checks are all `.any(...)` — `repair_contract` at `:1322-1470`, `skipped_expensive_collectors` at `:1771`, `checks` at `:3235`/`:3622` — which by construction tolerate additions.

**The field-addition pin lives one file over, and a doctor change will hit it.** `tests/golden_robot_json.rs` holds three doctor freezes, all present on HEAD:

- `doctor_json_matches_golden` — `tests/golden_robot_json.rs:1993`, instance freeze against `tests/golden/robot/doctor.json.golden`;
- `doctor_shape_matches_golden` — `tests/golden_robot_json.rs:2038`, schema freeze against `tests/golden/robot/doctor_shape.json.golden`;
- `doctor_quarantine_json_matches_golden` — `tests/golden_robot_json.rs:1302`, against `tests/golden/robot/doctor_quarantine.json.golden`.

Its own comment block at `tests/golden_robot_json.rs:1972-1991` states the purpose in terms of exactly this question: "A regression that added, removed, or re-typed a field in the base envelope would compile clean and pass the existing suite."

Practical consequences for the doctor-integrity fix:

- Any new top-level field, any new `skipped_expensive_collectors` entry, and any status/`next_action` string change on the fresh-empty-data-dir path will redden all three. `tests/golden/robot/doctor.json.golden:42` and `doctor_quarantine.json.golden:42` already contain the `skipped_expensive_collectors` array; `doctor_shape.json.golden:125` types it.
- Regeneration is `UPDATE_GOLDENS=1` — the panic text spells out the exact command at `tests/golden_robot_json.rs:942-947` and `:958-963`: `UPDATE_GOLDENS=1 rch exec -- env CARGO_TARGET_DIR=/tmp/cass-golden-target cargo test --test golden_robot_json`, then `git diff tests/golden/` and `git add tests/golden/`. On mismatch it also writes a `.actual` sibling next to the golden for diffing (`:952-953`).
- There is a fourth, non-golden pin worth knowing about: `src/lib.rs:78231` carries an explicit `"required": [...]` list for doctor's response schema (33 names including `check_scope`, `coverage_summary`, `checks`). Adding a required field means editing that list too; a purely additive optional field does not.

**Caveat / null result, stated plainly.** I did not run the suite (this lane is read-only and forbidden from `cargo`), so "no test in `cli_doctor.rs` fails on a new field" is established by reading every candidate assertion shape in the file, not by executing a mutant. The claim I would stand behind without a run is the narrower one: the file contains no exhaustive-field, key-set, key-count, or whole-payload-equality assertion over a doctor payload. If a stronger guarantee is needed, the cheap falsifier is to add one throwaway field to doctor's JSON and see which suites go red — the prediction is `golden_robot_json` red, `cli_doctor` green.

---

## Recipe for the new skip-path test, assembled from the above

Name it as a pair, matching `:4725`/`:4666`:

- skip half — the gate fires: `seed_healthy_empty_index`, `.env("CASS_DOCTOR_…_MAX_ARCHIVE_DB_BYTES", "1")`, `assert!(db_bytes > 1, "fixture must actually exceed … or this test proves nothing")`, run `cass doctor --json`, then assert the database check reports `not_checked` (or whatever honest status the fix introduces) rather than `pass`, that `needs_rebuild` is `Some(false)`, and that the recommended action names the surface that does verify — `.is_some_and(|text| text.contains(...))`.
- verify half — the gate must not fire by default: `doctor_json_fails_when_full_integrity_check_finds_archive_corruption` at `:590` already is that half, unchanged and green, because its fixture database is far under any sane default cap. Say so in the new test's doc comment the way `:4719-4722` does.

Both halves subprocess-invoked through `cass_cmd`, payload read with `serde_json::from_slice`, every assertion carrying `{payload:#}` in its message. Expect to regenerate the three doctor goldens in `tests/golden/robot/` in the same commit.
