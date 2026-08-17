# Lane A — the downstream contract for the integrity-skip path

Read-only audit. Every claim below is `src/lib.rs:<line>` unless another file is named.
No source file was edited; no build or test was run.

---

## Headline

`db_ok` does **not** mean "structurally verified". Every consumer reads it as
**"the canonical archive DB is usable as an authority"**. The strongest evidence is the
parameter name at the call boundary: `run_doctor_impl` passes `db_ok` into
`doctor_build_derived_semantic_asset_report`'s parameter **`archive_db_usable`**
(src/lib.rs:70401 → src/lib.rs:31592). The repair planner says the same thing in prose —
`db_ok == true` emits the warning string *"canonical archive DB is readable"*
(src/lib.rs:48594-48597) and the authority label `"canonical_archive_db:read_only"`
(src/lib.rs:48678).

So the skip path must set **`db_ok = true`** and **must not touch `needs_rebuild`**, and the
"integrity was not verified" fact must be carried by a **separate check**, not by `db_ok`.
Setting `db_ok = false` on a readable 23 GB archive is not a conservative choice — it is the
direct trigger for a candidate-staging write, a bundle move, and a full source reindex.
Details and failure modes in §3.

---

## 1. Every read of `db_ok`

Declared `let mut db_ok = false;` at src/lib.rs:69348. Writes: 69655, 70707, 70728, 70743,
70763. Reads follow, in source order. `M` = can mutate the filesystem, `R` = report-only.

| # | line | expression | `true` ⇒ | `false` ⇒ | class |
|---|---|---|---|---|---|
| 1 | 69770 | `if num_docs == 0 && db_ok` | may add the `index_sync` warn check and **set `needs_rebuild = true`** (69792) | the check is never emitted | **M (indirect)** — the only rebuild path `db_ok=true` *opens* |
| 2 | 70245 | arg to `doctor_candidate_build_should_run(fix_can_mutate, db_ok, needs_rebuild, …)` → 38915 `!db_ok \|\| needs_rebuild \|\| archive_risk \|\| raw_mirror_expands_archive` | this disjunct contributes nothing | `!db_ok` fires the trigger ⇒ `build_doctor_reconstruct_candidate` (70251) — **a real write under `<data_dir>/doctor/candidates`** | **M** (needs `fix_can_mutate`) |
| 3 | 70334 | same predicate, but only to fill `fix_available` on the `candidate_staging` check (70327-70340) | — | advertises a fix that would stage a candidate | R |
| 4 | 70357 | `suppress_legacy_rebuild_for_verified_candidate = needs_rebuild && fix_can_mutate && !db_ok && <verified completed candidate>` | no suppression | **sets `needs_rebuild = false`** (70367) and pushes a `legacy_archive_rebuild` **warn** check (70368-70374) | M (suppressive — reduces risk) |
| 5 | 70401 | `doctor_build_derived_semantic_asset_report(…, db_ok, …)`, param `archive_db_usable` (31592) | semantic readiness is reported normally | `safe_to_rebuild = false` (31631) and `status = "skipped-archive-unavailable"` (31641-31642), which maps to check status **`"pass"`** (31659-31663) | R — but the `semantic_model` message (70412-70421) then asserts an archive state nobody measured |
| 6 | 70437 | arg to `build_doctor_repair_plan_preview` (48575). Three uses inside: | | | |
| 6a | 48592 | `let candidate_promotion_candidate = if db_ok { … None }` | refuses promotion, pushes *"canonical archive DB is readable"* warning (48594-48597) | the completed candidate becomes the promotion candidate ⇒ plans **`promote_reconstruct_candidate`** on `DoctorAssetClass::CanonicalArchiveDb` (48635-48656), reason string *"archive DB is not readable"* | **M (deferred)** — plan-only here, but this is the plan a fingerprint apply executes |
| 6b | 48673 | `if needs_rebuild && db_ok` | plans the SAFE derived-only `rebuild_from_archive_db` (48674-48686) | falls to 48687 ⇒ pushes the `archive-risk` blocker (48688-48694) | R (plan text) |
| 6c | 48774 | `"db_ok": db_ok` inside `fingerprint_inputs` | — | — | R, but **flipping it changes `plan_fingerprint`**, invalidating any fingerprint already issued to an operator |
| 7 | 70853 | `safe_auto_archive_rebuild_refused = safe_auto_run_requested && needs_rebuild && fix_can_mutate && !db_ok` | no refusal check | pushes a **`"fail"`** check (70855-70861) ⇒ `fail_count > 0` ⇒ **doctor exits non-zero** (71307, 71793) | R + **exit code** |
| 8 | 70880 | `let rebuild_from_db = db_ok && db_messages.unwrap_or(0) > 0` | takes the safe branch: rebuild Tantivy *from* the DB (70882-70943), DB untouched | takes the else branch at 70944 — the branch that moves the bundle | **M (branch selector)** |
| 9 | 70948 | `if !db_ok` | — | **`move_database_bundle(&db_path, &db_path.with_extension("corrupt.<ts>"))`** (70949-70951) — moves the live db + wal + shm aside | **M — the destructive one** |
| 10 | 71015 | `full: force_rebuild \|\| !db_ok` in `IndexOptions` | incremental UPSERT path | **full** reindex from source sessions (comment 71012-71014) | **M** |
| 11 | 71401 | field of `DoctorSafeAutoRunBuildInput` (25103). Two uses: 25753-25756 `check.safe_for_auto_repair && mutating_doctor_allowed && input.db_ok` gates `Eligible`; 25849-25855 `input.needs_rebuild && input.db_ok && db_messages>0` sets `next_exact_command = "cass doctor --fix --json"` | derived findings can be auto-eligible | every finding downgrades to ManualApprovalRequired / Blocked / Skipped (25758-25774) | R |

Also src/lib.rs:59949/59965 — `doctor_test_repair_plan_for_candidate_staging`, a `#[cfg(test)]`
helper. No production effect.

**Guard that makes #2, #8, #9, #10 reachable at all:** `fix_can_mutate` requires `fix`
(src/lib.rs:69322-69326), so a bare read-only `cass doctor --json` cannot execute any of them.
#2 (candidate staging) is the exception worth naming: it is gated on `fix_can_mutate` and
`!db_ok` but **not** on `needs_rebuild` (38915), so `db_ok = false` alone is sufficient to make
`cass doctor --fix` write a reconstruct candidate against a perfectly healthy archive. That is
exactly the fail-open shape bead `…-doctor-promote-gate-fails-open-sgvg3` already fixed on the
neighbouring term (comment at src/lib.rs:38899-38906).

**Which reads can cause a rebuild / destructive action / mutation:**
- Destructive: **#9** (bundle move, 70951).
- Mutating: **#2** (candidate write), **#8** and **#10** (which rebuild path runs), **#1** and
  **#4** (they move `needs_rebuild`, the master switch — see §2).
- Deferred-mutating: **#6a** (the promotion action a fingerprint apply performs).
- Exit-code: **#7**.
- Report-only: **#3**, **#5**, **#6b**, **#6c**, **#11**.

---

## 2. Every read of `needs_rebuild`

Declared `let mut needs_rebuild = force_rebuild;` at src/lib.rs:69347. Writes: 69707, 69716,
69721, 69732, 69744, 69792, 69801, 69805, 69817, 70367, 70721, 70725, 70730, 70745, 70765,
70781, 70798, 70813, 70826, 70901, 71009, 71050.

| # | line | expression | consequence when `true` | class |
|---|---|---|---|---|
| 1 | 70246 | `doctor_candidate_build_should_run(…)` → 38915 | one of four independent triggers for the candidate-staging write | **M** |
| 2 | 70335 | same predicate, `fix_available` only | advertises the fix | R |
| 3 | 70355 | first conjunct of `suppress_legacy_rebuild_for_verified_candidate` | can zero itself out at 70367 | M (suppressive) |
| 4 | 70436 | arg to `build_doctor_repair_plan_preview` (48574): 48657 plans a post-promotion derived rebuild; 48673 plans `rebuild_from_archive_db`; 48687 pushes the `archive-risk` blocker; 48773 fingerprint input | shapes the repair plan and its fingerprint | R (plan) / M (deferred) |
| 5 | 70853 | conjunct of `safe_auto_archive_rebuild_refused` | with `!db_ok`, produces a **fail** check ⇒ non-zero exit | exit code |
| 6 | **70863-70864** | `let derived_rebuild_attempted = needs_rebuild && fix_can_mutate && !safe_auto_archive_rebuild_refused;` | **the master switch for the whole mutating rebuild block 70865-71090**, including the bundle move (70951) and the full reindex (71028) | **M — the one that matters** |
| 7 | 70723 | `if promotion.derived_lexical_rebuild_required && !needs_rebuild` | post-promotion bookkeeping only | R |
| 8 | 71400 | `DoctorSafeAutoRunBuildInput.needs_rebuild` → 25849 | picks `next_exact_command` | R |
| 9 | 71519 | serialized as the top-level JSON field `"needs_rebuild"` | **public contract**: pinned by `tests/cli_doctor.rs:638` and `tests/golden/robot/doctor.json.golden:17`; declared in the schema at src/lib.rs:78135 | R (contract) |
| 10 | 71760 | `if needs_rebuild` in human output | prints *"Recommended action: cass index --full"* plus the reassurance at 71768-71773 | R — on a 23 GB archive this is a multi-hour recommendation |

`src/indexer/mod.rs` and `src/ui/app.rs` also contain `needs_rebuild` identifiers; they are
unrelated locals (analytics rollups at src/ui/app.rs:20363-20377, lexical population strategy at
src/indexer/mod.rs:1401-1426). Doctor's `needs_rebuild` has no consumer outside `run_doctor_impl`
other than the serialized JSON field.

---

## 3. What the skip path must set

### Recommendation

```
db_ok         = true        // the DB opened, both COUNT(*) queries returned
needs_rebuild = unchanged   // do not touch it; the size gate is not evidence of anything
```

and emit the "not verified" fact as a **separate check with status `"pass"`**, e.g.

```
add_check!(
    "database_integrity_scan",
    "pass",
    format!("Archive DB integrity walk skipped: {} bytes exceeds the {} byte scan gate \
             (override with CASS_DOCTOR_INTEGRITY_MAX_ARCHIVE_DB_BYTES); \
             the database opened and reported {conv} conversations and {msgs} messages, \
             but its structural integrity was NOT verified by this run",
            actual_bytes, gate_bytes),
    false
);
```

and leave the existing `database` check as `"pass"` with a message that states counts **and**
says the integrity walk did not run — so no reader can mistake "Database OK" for "verified".

Requirement (a) — *do not trigger or recommend a rebuild on no evidence of corruption* — is
satisfied because `needs_rebuild` is untouched, which closes the master switch at 70863-70864,
and `db_ok = true` closes the candidate-staging trigger at 38915.

Requirement (b) — *do not claim the database was verified* — is satisfied by the separate check
and by the amended `database` message. Nothing else in the pipeline reads `db_ok` as a
verification claim; §1 shows every consumer reads it as usability.

### Why `db_ok = false` is wrong (the tempting alternative)

It reads like the conservative choice and it is the opposite. On `cass doctor --fix` against a
healthy 23 GB archive it would:

1. Fire `doctor_candidate_build_should_run` on the `!db_ok` disjunct alone (38915) — **no
   `needs_rebuild` required** — and write a reconstruct candidate under
   `<data_dir>/doctor/candidates` (70251). Copying a fraction of a 23 GB archive on the
   strength of a `stat`.
2. If anything else in the run sets `needs_rebuild` (a stale Tantivy index at 69801/69817 is the
   common case), open `derived_rebuild_attempted` (70864) with `rebuild_from_db == false`
   (70880), reaching `if !db_ok` at 70948 and **moving the live 23 GB db/wal/shm to
   `agent_search.corrupt.<ts>.db`** (70951), then a full source reindex (71015).
3. Make the repair plan preview select the candidate for **promotion over the live archive**
   (48592 → 48635-48656) with the reason string *"archive DB is not readable"* — a false
   statement about an archive that opened and answered two queries.
4. Under safe-auto, push a `"fail"` check (70855) whose message says the archive is *"missing,
   unreadable, or corrupt"* — **non-zero exit** (71793) on a healthy archive.
5. Turn `semantic_model` into `"skipped-archive-unavailable"` (31641) and downgrade every
   safe-auto finding out of `Eligible` (25753-25756).

That is a false-corruption cascade, not caution.

### Why "leave `db_ok = false` but also force `needs_rebuild = false`" is wrong

It blocks §3.2 but not §3.1, §3.3, §3.4 or §3.5 — the candidate-staging write, the promotion
plan, the failing safe-auto check and the exit code all key off `!db_ok` with no `needs_rebuild`
term. It also collides with `force_rebuild`: `needs_rebuild` is *initialised* from
`force_rebuild` (69347), so hard-clearing it would silently swallow an explicit
`cass doctor --fix --force-rebuild`.

### Why "set `needs_rebuild = true` so someone re-checks later" is wrong

`needs_rebuild` is the master switch at 70863-70864 and the top-level JSON contract at 71519. On
a 23 GB archive setting it recommends `cass index --full` in the human output (71760-71766) and,
under `--fix`, actually runs it. A size gate is not evidence of corruption; nothing here has
observed a defect.

### Why a novel status string (`"not_checked"`, `"skipped"`) is wrong

See §4 — it is neither counted nor rendered.

### One thing to get right in the message text

If the gate check is ever emitted with a **non-pass** status under the name `database`,
`doctor_anomaly_for_check` matches the substring `"integrity_check"` or `"quick_check"` in the
message and returns `DoctorAnomaly::ArchiveDbCorrupt` (src/lib.rs:25528-25534) ⇒ health class
`degraded-archive-risk`, severity Error, data-loss risk High (25264-25273). Under `"pass"` the
function short-circuits at 25513 and the message text is inert, so `"pass"` is also the safe
choice with respect to that substring match. Do not name a warn-status check `database` while
its message mentions either pragma.

---

## 4. The `add_check!` macro, legal statuses, and aggregation

**Definition:** src/lib.rs:69357-69367, inside `run_doctor_impl`.

```rust
macro_rules! add_check {
    ($name:expr, $status:expr, $message:expr, $fix_available:expr) => {
        checks.push(Check {
            name: $name.to_string(),
            status: $status.to_string(),
            message: $message.to_string(),
            fix_available: $fix_available,
            fix_applied: false,
        });
    };
}
```

Four arguments; `fix_applied` is always `false` (a later pass mutates it in place, e.g.
70904-70909). Call sites that need `fix_applied: true` bypass the macro and push a `Check`
literal directly (e.g. 70327, 70911). 57 emission sites in the file.

**The final boolean is `fix_available`.** It means "doctor knows a repair for this and could
apply it." Consumers:
- src/lib.rs:25611 — `safe_for_auto_repair: policy.safe_for_auto_repair && fix_available && status != "pass"`. So `fix_available: false` on a `pass` check is doubly inert.
- src/lib.rs:71618 — human output prints ` [fixable]` when `fix_available && !fix`.
- src/lib.rs:78224/78227 — part of the pinned per-check JSON schema.
For the skip check the correct value is **`false`**: there is no repair, only a measurement that
was declined.

**Legal status strings: `"pass"`, `"warn"`, `"fail"`.** Declared in the struct comment at
src/lib.rs:69340 (`status: String, // "pass", "warn", "fail"`) and in the JSON schema at
src/lib.rs:78214 (`"status": { "type": "string", "description": "pass | warn | fail" }`). The
schema types it as a bare string with no `enum`, so a novel value would serialize — but nothing
reads it:

- `fail_count` (71307) and `warn_count` (71308) both test string equality, so a novel status
  counts as neither, and `issues_found = fail_count + warn_count` (71309) stays 0.
- `all_pass` (71472) tests `== "pass"`, so it goes false and the human summary prints the
  contradiction *"0 failure(s), 0 warning(s)"* (71654-71665).
- The human icon match falls through to `"?"` (71604-71609).
- `doctor_anomaly_for_check` short-circuits only on `"pass"` (25513); anything else falls into
  the name match, and an unknown name lands on `_ => DoctorAnomaly::RepairBlocked` (25588).

So a novel status is silently mis-aggregated in three places. Use `"pass"`, `"warn"` or `"fail"`.

**Aggregation, in order:**

1. `fail_count` / `warn_count` / `issues_found` / `issues_fixed` — src/lib.rs:71307-71310.
2. Each `Check` → `DoctorCheckReport` via `doctor_check_report` (71311-71322, defined 25592-25616).
   That call derives `anomaly_class` from `(name, status, message)` (25599 → 25512-25590), then
   the whole policy row — `health_class`, `severity`, `affected_asset_class`, `data_loss_risk`,
   `recommended_action`, `default_outcome_kind` — from `doctor_anomaly_policy`
   (25600 → 25406, table 25203-25404).
3. `health_class = doctor_health_class_for_checks(&check_reports)` (71323 → 26270-26302): a
   priority scan — RepairPreviouslyFailed > DegradedArchiveRisk > SourceAuthorityUnsafe >
   RepairBlocked > DegradedDerivedAssets > Healthy. **Any one non-Healthy check sets the whole
   run's class.**
4. `risk_level = doctor_risk_level_for_reports` (71324 → 32077-32106): High ⇒ `"high"`;
   Medium or **Unknown** ⇒ `"medium"`; any `warn` or Low ⇒ `"low"`; else `"none"`.
5. `recommended_action` (71325 → 32108-32130): the first check with `status != "pass"` and a
   non-`"none"` recommended action wins.
6. `incidents = build_doctor_root_cause_incidents(...)` (71343 → 26113-26268): the loop at
   **26134 iterates `checks.iter().filter(|check| check.status != "pass")`** — so **every**
   non-pass check, warn included, creates or joins a doctor incident. It surfaces in
   `primary_incident_id` / `incidents` in JSON and under *"Primary incident:"* in human output
   (71695-71718).
7. `doctor_status` / `healthy` (71473-71480): `healthy = fail_count == 0 && !not_initialized`;
   status is `not_initialized` / `healthy` / `unhealthy` keyed on `fail_count`.
8. Exit code — src/lib.rs:71793: `if fail_count == 0 || operation_exit_code_kind == Success { Ok(()) }`.

**Is `"warn"` a legal status that does NOT make doctor exit non-zero or recommend a rebuild?**

*Exit code:* **yes, warn is exit 0** — but only because of the first disjunct at 71793. Note the
trap: one warn makes `issues_found > 0`, and in a read-only run
`doctor_top_level_operation_outcome` then returns `OkReadOnlyDiagnosed` (26474-26488), whose
policy row carries **`exit_code_kind: DoctorExitCodeKind::HealthFailure`** (src/lib.rs:27615).
The `fail_count == 0` short-circuit is the only thing keeping the process at 0.

*Rebuild:* a warn check does not itself set `needs_rebuild`; only the writes listed in §2 do.

*But warn is not free.* A warn check named anything not in the `doctor_anomaly_for_check` table
falls to `_ => DoctorAnomaly::RepairBlocked` (25588), whose policy (25234-25243) is
`health_class: RepairBlocked`, `data_loss_risk: Unknown`, `recommended_action:
"inspect-blocker-before-retrying"`. Consequences on an otherwise healthy 23 GB archive:
`health_class` becomes `"repair-blocked"` (26289-26294), `risk_level` becomes `"medium"`
(32090-32097, via `Unknown`), the top-level `recommended_action` becomes
`"inspect-blocker-before-retrying"` (32126-32129), an incident is created with
`root_cause_kind: unknown` and `stale_or_unknown_fields: ["root_cause_kind"]`
(25961-25963, 26182-26186), and the JSON then says `"status": "healthy"` alongside
`"health_class": "repair-blocked"` — a self-contradiction.

That is why §3 recommends `"pass"`, not `"warn"`.

---

## 5. Existing precedent for "I did not check this"

Yes — three shapes exist, and two are directly copyable.

### 5a. `pass` + an explicit skip message (**the precedent to copy**)

- **`source_inventory`**, src/lib.rs:69943-69949:
  ```rust
  } else if !source_inventory.db_available {
      add_check!(
          "source_inventory",
          "pass",
          "Source inventory skipped until the cass archive database exists",
          false
      );
  }
  ```
  Exact shape: status `"pass"`, `fix_available: false`, a message that names the skip and its
  reason. `pass` ⇒ `DoctorAnomaly::Healthy` (25513) ⇒ no health-class change, no incident, no
  `[fixable]` marker, exit 0. The honesty lives entirely in the message.

- **`raw_mirror_backfill`**, src/lib.rs:70117-70122: `"skipped" => ("pass", "Raw mirror backfill
  skipped until archive rows exist")`, pushed at 70178-70184. Same shape via a match arm rather
  than the macro.

- Adjacent, same idea inside a sub-report: `doctor_build_derived_semantic_asset_report` maps
  `status == "skipped-archive-unavailable"` to `doctor_check_status = "pass"`
  (src/lib.rs:31641-31663), which is how the `semantic_model` check at 70412-70421 already
  reports a declined measurement as `pass`.

### 5b. `warn` + "was not checked" (the weaker precedent — do not copy)

- **`storage_pressure`**, src/lib.rs:69489-69499: the `_` arm emits `"warn"` with
  `"Storage pressure was not checked"`. It is survivable only because the name *is* in the
  anomaly table (25519 → `StoragePressure`, policy 25354-25363: health class RepairBlocked,
  risk Medium). A new name gets no such row and lands on the `_ => RepairBlocked` fallback with
  `data_loss_risk: Unknown`, which is strictly worse (see §4).

### 5c. A structured `not_checked` vocabulary that is **not** a `Check`

`doctor_check_scope_report` (src/lib.rs:32146-32200) already publishes declined work as data
rather than as a check:

```json
"skipped_expensive_collectors": [
  { "name": "full_raw_log_reparse", "status": "not_checked", "next_action": "…" },
  { "name": "semantic_embedding_deep_integrity", "status": "not_checked", "next_action": "…" },
  { "name": "network_source_sync", "status": "not_checked", "next_action": "…" }
]
```

plus a `cleanup_planning` block that is `"not_checked"` on the bounded `Check` surface
(32182-32197). This array is emitted into the doctor payload at `"check_scope"` (71533) and is
already in the golden at `tests/golden/robot/doctor.json.golden:44-56`. The parallel
`unchecked_fast_health` / `confidence_tier: "unchecked"` idiom for coverage lives at
src/lib.rs:37217-37233 and is consumed at 38897 and 26189-26193.

**Recommended combination:** the §5a `pass` check (so a human reading the check list sees it)
**plus** a fourth `skipped_expensive_collectors` entry named e.g.
`archive_db_integrity_walk` with `status: "not_checked"` and a `next_action` naming the env
override (so a machine reading `check_scope` sees it without string-matching a message). The
second half costs one `json!` literal in `doctor_check_scope_report`, needs a signal for whether
the gate fired, and updates `tests/golden/robot/doctor.json.golden`.

---

## 6. Contract surfaces the fix will touch

- `tests/cli_doctor.rs:616-638` pins the corrupt-archive path: the `database` check must be
  `status: "fail"`, `anomaly_class: "archive-db-corrupt"`, message containing `integrity_check`,
  with `health_class == "degraded-archive-risk"` and `needs_rebuild == true`. That fixture's DB
  is tiny, so a byte-size gate must not fire for it — pick a default well above test-fixture
  size (the `cass status` twin uses 256 MiB, src/lib.rs:15107).
- `tests/golden/robot/doctor.json.golden` is byte-exact over the whole `checks` array
  (names at lines 2990-3230) and over `check_scope.skipped_expensive_collectors` (lines 44-56).
  A check emitted **only when the gate fires** leaves it untouched; an unconditional one, or a
  new `skipped_expensive_collectors` entry, requires regenerating it.
- The per-check JSON schema requires `["name","status","message","anomaly_class","health_class",
  "severity","affected_asset_class","data_loss_risk","recommended_action","safe_for_auto_repair",
  "default_outcome_kind","fix_available","fix_applied"]` (src/lib.rs:78227) — all derived, so a
  new check needs no schema change.
- Flipping `db_ok` changes `plan_fingerprint` (src/lib.rs:48774), so any repair fingerprint
  issued by a pre-fix binary will not match a post-fix one. Expected, worth stating in the
  commit.

---

## 7. Null results / limits of this audit

- I did not run `cargo build`, `cargo test`, or the binary. Every statement is read from source.
- I did not find any consumer of `db_ok` outside `src/lib.rs`; the `needs_rebuild` hits in
  `src/indexer/mod.rs` and `src/ui/app.rs` are unrelated locals (§2).
- I did not verify the runtime cost claim about `PRAGMA quick_check` in fsqlite-core 0.1.5 —
  that is the coordinator's established root cause and outside this lane.
- I did not check whether `doctor_push_timing_span`'s existing note at src/lib.rs:69746-69754
  (*"…completed or were skipped by state"*) already covers the skip case adequately for the
  slow-operation report; it reads as though it was written to.
