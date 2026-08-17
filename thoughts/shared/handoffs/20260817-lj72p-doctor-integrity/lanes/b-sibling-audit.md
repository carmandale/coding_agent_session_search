# Lane B — sibling audit of `run_doctor_impl`

Read-only. Scope: `run_doctor_impl` (`src/lib.rs:69233-71846`) and every helper it
calls, under a plain `cass doctor --json` (no flags). Every claim below is cited;
where I could not establish something I say so.

## The one thing to take away

**The doctor's integrity probe is not the only unbounded whole-archive operation in
this function — it is the only one anybody has *observed*, because it is the first
one, and nothing downstream of it has ever run on this archive.** The two largest
costs in `run_doctor_impl` both sit *after* the hang, and both are the *same two
collectors* that `cass status` already refuses to run on this data dir. Gating only
the integrity probe converts an unobserved hang into an observed one.

Measured on the live specimen data dir
(`~/Library/Application Support/com.coding-agent-search.coding-agent-search`), read-only:

| thing | measured | the gate `cass status` applies | ratio |
|---|---|---|---|
| `agent_search.db` | 23,313,477,632 bytes | `STATUS_COVERAGE_MAX_ARCHIVE_DB_BYTES_DEFAULT` = 268,435,456 (`src/lib.rs:15107`) | **87x over** |
| `raw-mirror/v1/manifests` | 147,844 files, 242,715,998 bytes | `STATUS_COVERAGE_MAX_RAW_MIRROR_MANIFESTS` = 512 (`src/lib.rs:15084`) | **289x over** |
| `raw-mirror/v1/blobs` | 140,344 files, **48,573,578,826 bytes** | — | — |
| live provider corpus | `~/.claude/projects` 20,690 files / 9.03 GB; `~/.codex/sessions` 10,391 / 30.50 GB; `~/.cursor` 33,325 / 1.60 GB | — | — |

`cass doctor --json` applies **neither** gate. `status_raw_mirror_scan_too_large`
(`src/lib.rs:34441`) and `status_archive_scan_too_large` (`src/lib.rs:34493`) have
exactly two call sites, both inside the status path at `src/lib.rs:66078-66079`.
`rg -n "status_raw_mirror_scan_too_large|status_archive_scan_too_large" src/lib.rs`
returns 5 lines: the two definitions, one doc-comment mention, and those two call
sites. Doctor is not among them.

And the collectors those gates protect are literally the same functions doctor calls
inline. `collect_doctor_coverage_risk_summary` (`src/lib.rs:37190`), the thing status
gates, calls `collect_doctor_source_inventory` / `collect_doctor_raw_mirror_report` /
`collect_doctor_raw_mirror_backfill_report` at `src/lib.rs:37204-37206`. Doctor calls
the same three at `src/lib.rs:69923`, `70031`, `70042` with no gate at all.

---

## BEFORE vs AFTER the integrity probe — the most useful distinction

The integrity probe is `doctor_database_integrity_probe(&conn)` at
**`src/lib.rs:69653`**. The generation-17 sample was taken 25 s into the run with the
process inside `pragma_integrity_check_rows` (coordinator log
`thoughts/shared/handoffs/20260817-lj72p-doctor-integrity/agent-log.md:65-72`), which
is what proves everything ordered before line 69653 returned.

### Proven to complete within 25 s on the specimen (runs BEFORE line 69653)

| # | file:line | operation | cost driver | bounded? |
|---|---|---|---|---|
| 1 | 69256 | `probe_index_run_lock` | one lock file | yes |
| 2 | 69263 | `collect_doctor_repair_failure_marker` | one `read_dir` of the marker dir (`src/lib.rs:48961`) | by marker count |
| 3 | 69302 | `build_doctor_operation_state_report` | lock/path metadata; no walk, no query | yes |
| 4 | 69456 | `collect_doctor_storage_pressure` → `doctor_available_space` | one statfs | yes |
| 5 | 69504-69509 | stale `.index.lock` metadata | one stat | yes |
| 6 | 69626 | `open_franken_cli_read_db_with_hard_timeout(…, 30s)` | opening a 23 GB DB | **yes — the only real deadline in the function** |
| 7 | 69635-69641 | `SELECT COUNT(*) FROM conversations` | rows (27,441) | **no** |
| 8 | 69642-69648 | `SELECT COUNT(*) FROM messages` | rows (~2.3M) | **no** |

Items 7 and 8 are unbounded in shape but are *not* this bead's blocker: they demonstrably
returned. Ranked low for that reason and no other.

### Nothing is known about these — the process never reached them (runs AFTER line 69653)

Ordered as the function runs them.

| # | file:line | operation | cost driver | bounded? | blocking rank on this specimen |
|---|---|---|---|---|---|
| 9 | 69671 | `probe_doctor_fts_table` → `SELECT rowid FROM fts_messages LIMIT 1` (`src/lib.rs:67208`) | first FTS5 row | `LIMIT 1` | low |
| 10 | 69723 | `close_franken_cli_read_db` → `close_in_place` (`src/lib.rs:14234`) | WAL checkpoint | WAL is 32 bytes here | low |
| 11 | 69758-69759 | `searchable_index_summary` (`src/search/tantivy.rs:938`) | reads `meta.json`; only opens the index if a segment has deletes | effectively yes | low |
| 12 | 69771 | second DB open + `COUNT(*) FROM messages`, 1 s open timeout | rows | no — but only reachable when `num_docs == 0` | low |
| 13 | 69870 | `collect_doctor_config_exclusion_risks` (`src/lib.rs:31363`) | 7 fixed targets × a fixed config-file list | yes | low |
| 14 | 69923 | **`collect_doctor_source_inventory`** | conversations rows + one `stat` per group | **no** | **medium** |
| 15 | 69977 | `collect_doctor_remote_source_sync_report` (`src/lib.rs:33174`) | configured sources; mirror scans capped at 128 entries / 512 files (`src/lib.rs:33268,33270`) | yes | low |
| 16 | 70031 | **`collect_doctor_raw_mirror_report`** | **147,844 manifests + blake3 over 48.57 GB of blobs** | **no** | **HIGHEST** |
| 17 | 70042 | **`collect_doctor_raw_mirror_backfill_report`** | **27,441 rows, one receipt each, blake3 over the live provider file per row** | **no** | **HIGHEST** |
| 18 | 70185-70191 | `build_doctor_sole_copy_warnings` / `build_doctor_coverage_summary` (`src/lib.rs:36848`) | iterates all manifests + all receipts in memory | O(N), no I/O | medium (memory) |
| 19 | 70228 | `build_doctor_source_authority_report` (`src/lib.rs:37666`) | iterates all manifests | O(N), no I/O | low |
| 20 | 70240 | `collect_doctor_candidate_staging_report` (`src/lib.rs:39477`) | one `read_dir`; hashes only each `manifest.json` | by candidate count (0 here) | low |
| 21 | 70378 | `state_meta_json_inner` | called with `allow_db_open=false, include_counts_override=Some(false), skip_db_open=true` (`src/lib.rs:70378-70386`) → DB open elided at `src/lib.rs:16455-16461`, lexical fingerprint off | yes | low |
| 22 | 71495 | `collect_diag_quarantine_report` (with cleanup plan — LegacyDoctor is not `Check`) | `fs_dir_size` recursion (`src/lib.rs:65633`) over backup/publish-backup dirs | by those dirs (absent here) | low |
| 23 | 71497-71597 | `serde_json` of the whole payload | **147,844 manifest reports + 27,441 backfill receipts, both serialized** | **no** | **high (memory + output size)** |

Skipped entirely in a plain read-only run, verified by their gates:
repair plan (`src/lib.rs:70423`, needs `surface == Repair`), candidate promotion
(`70637`, needs `fix_can_mutate`), derived rebuild (`70864`), cleanup apply (`71101`),
post-repair probes (`51017-51041`, returns `requested: false`).

---

## The three findings, ranked

### Finding B1 (highest) — `collect_doctor_raw_mirror_report` blake3-hashes every mirror blob

`src/lib.rs:34501` → `34508`. The manifest loop at `src/lib.rs:34557-34595` walks
`raw-mirror/v1/manifests` with `walkdir`, reads and JSON-parses every `.json`, and
calls `doctor_verify_raw_mirror_manifest` on each. That function reaches
**`src/lib.rs:34297`**:

```rust
            match doctor_file_blake3(&blob_path) {
```

`doctor_file_blake3` (`src/lib.rs:34075-34087`) opens the blob and reads it whole
through a 64 KiB buffer. There is **no** manifest cap, **no** byte cap, **no**
deadline, and no early exit. On the specimen that is 147,844 manifest reads/parses
plus a full-content hash of **48.57 GB across 140,344 blob files**, single-threaded.

This is the exact operation the status gate was written for, and the comment at the
status call site records what it already cost once:

> `src/lib.rs:66062-66064` — "on a 125,607-manifest mirror `cass status --json` then
> ran 15 minutes at 4 GB resident and wrote zero bytes (bead nvq59)."

The specimen's mirror is **147,844** manifests — larger than the one that produced
that 15-minute, zero-byte run. Doctor runs it with no gate.

Two secondary costs ride along, both unbounded and both O(manifests):

- `report.manifests` accumulates a full `DoctorRawMirrorManifestReport` per manifest
  (`src/lib.rs:34588`), then sorts them (`34599`).
- That `Vec` has **no `#[serde(skip_serializing)]`** (`src/lib.rs:33570`, contrast the
  attribute on `root_path` at `33560`), so all 147,844 reports are serialized into the
  `--json` payload at `src/lib.rs:71563`.

### Finding B2 (highest) — `collect_doctor_raw_mirror_backfill_report` hashes the live provider file per conversation

`src/lib.rs:36560`. `query_doctor_raw_mirror_backfill_candidates` (`35908`) returns
**one row per conversation, `ORDER BY c.id`, no `LIMIT`** (`src/lib.rs:36002-36007`)
— 27,441 rows on this archive. The loop at `src/lib.rs:36694-36710` then builds one
receipt per candidate.

The zumve fix already landed here: the correlated `COUNT(*)` is gone in favour of
`c.last_message_idx + 1` (`src/lib.rs:35981-35984`), and the point-probe fallback is
capped at 64 (`src/lib.rs:36038,36061`). That half is fixed. **The other half is not.**

Inside each receipt, `src/lib.rs:36387-36389`:

```rust
    let has_existing_evidence = by_conversation_id.contains_key(&candidate.conversation_id)
        || by_source_key.contains_key(&source_key);
    let source_stat = doctor_raw_mirror_backfill_source_stat(path, has_existing_evidence);
```

and `doctor_raw_mirror_backfill_source_stat` (`src/lib.rs:36131`) hashes the file
whenever that flag is true (`src/lib.rs:36147-36152`, calling the same
`doctor_file_blake3`).

The comment above it (`src/lib.rs:36380-36386`) says the skip exists because "on a
data dir with no raw mirror both maps are empty, so every candidate misses, and
hashing there reads every provider session file in the archive." **The specimen is the
opposite shape.** `doctor_raw_mirror_existing_evidence_maps` (`src/lib.rs:36192`)
populates both maps from every *verified* manifest, and the specimen has 140,344 blobs
backing 147,844 manifests — so the maps are dense, `has_existing_evidence` is true for
most candidates, and the expensive branch is the one taken. That is a second
full-content read pass, this time over the live provider corpus (measured above at
~41 GB across ~64k files; the reachable subset is the ~27k files the archive rows
point at).

`receipts: Vec<DoctorRawMirrorBackfillReceipt>` (`src/lib.rs:29841`) is likewise
un-skipped and serialized at `src/lib.rs:71564`.

Note this collector runs *before* the `raw_mirror_backfill_applied` refresh at
`src/lib.rs:70055`, which would run `collect_doctor_raw_mirror_report` a **second**
time — but only under `--fix`, so not on the plain run.

### Finding B3 (medium) — `collect_doctor_source_inventory`: full `conversations` scan plus one `stat` per group

`src/lib.rs:32711`. The SQL is built at `src/lib.rs:32508-32513`:

```sql
SELECT …, COUNT(*) FROM conversations c LEFT JOIN agents a … LEFT JOIN sources s …
GROUP BY 1, 2, 3, 4, 5 ORDER BY 1, 3, 4, 2
```

No `LIMIT`, no deadline. `source_path` is one of the group keys
(`src/lib.rs:32482-32486`), so group cardinality approaches the conversation count —
~27,441 groups, each materialised into a `Vec`. Then `src/lib.rs:32589-32593`:

```rust
        let local_path_missing = !is_remote
            && source_path
                .as_deref()
                .map(|path| !Path::new(path).exists())
```

— one filesystem `stat` per returned row, uncapped. Metadata only, so this is seconds
rather than minutes on local APFS; it is ranked medium rather than high on that basis,
and it would be much worse against a network-mounted provider root. I did not measure
it directly, because it is downstream of the hang.

---

## Sub-finding — the openers' `Duration` is a *busy* timeout, not a statement deadline

This is the shared root shape behind everything above, and it is worth stating plainly
because the argument names read like ceilings and are not.

- `open_franken_cli_read_db_with_hard_timeout` (`src/lib.rs:14132`) does bound the
  open: it hands the open to a worker thread and `recv_timeout`s on it
  (`src/lib.rs:14204-14216`). That ceiling covers **the open and nothing else**. Once
  `Ok(conn)` returns at `src/lib.rs:69631`, every statement on that connection —
  both `COUNT(*)`s, the integrity probe, the FTS probe — runs with no ceiling at all.
- `open_franken_cli_read_db` (`src/lib.rs:14071`) names its third parameter
  `busy_timeout` and passes it straight to
  `open_franken_readonly_storage_with_timeout` (`src/lib.rs:14089-14092`). It is a
  lock-wait, not a deadline, and it is **not** wrapped in the hard-timeout thread.
  The source-inventory open (`src/lib.rs:32731`) and the backfill open
  (`src/lib.rs:36593`) both use this bare form with `Duration::from_secs(1)`. So those
  two opens of the 23 GB database have no wall-clock ceiling either.

## What I checked and found NOT to be a problem

- **The second `PRAGMA quick_check(1)` at `src/lib.rs:70697`** is **not reachable** in
  a plain `cass doctor --json` run. It sits inside
  `if candidate_promotion_apply_requested && fix_can_mutate` (`src/lib.rs:70637`).
  `candidate_promotion_apply_requested` (`70629`) requires `repair_plan.apply_authorized`,
  and `repair_plan` is `Some` only when `command_surface == DoctorCommandSurface::Repair`
  (`70423`). A plain `cass doctor` resolves to `LegacyDoctor` + `ReadOnlyCheck` —
  `from_legacy_flags` (`src/doctor.rs:304-328`) passes `repair: false`, and
  `run_doctor_impl` receives `fix = request.mode.permits_mutation()` (`src/doctor.rs:798`),
  which is false. Both conjuncts fail. It is real and unbounded on the `doctor repair
  --apply <fingerprint>` surface, and it is downstream of a promotion that only happens
  under an explicit fingerprint-approved apply.
- `state_meta_json_inner` — the doctor call site explicitly passes
  `allow_db_open=false, skip_db_open=true, include_counts_override=Some(false)`
  (`src/lib.rs:70378-70386`), which elides the DB open (`16455-16461`) and the lexical
  fingerprint (`16507`). Bounded.
- Post-repair probes, repair plan, candidate promotion, derived rebuild, cleanup apply
  — all gated off in a read-only run (cited in the table above).
- The remote-mirror scans are genuinely capped: 128 top-level entries
  (`src/lib.rs:32765,33268`) and 512 files (`32796,33270`).
- `resolve_doctor_backfill_unknown_message_counts` is capped at 64
  (`src/lib.rs:36038,36061`) and, per the comment at `36042-36046`, does nothing at all
  on a current-schema archive.

## What I could not establish

- **Wall-clock estimates.** I did not run `cass doctor` and did not benchmark blake3 on
  this machine, so I am not putting a "this takes N minutes" number on B1 or B2. What is
  established is the byte and file counts above, and the recorded precedent at
  `src/lib.rs:66062-66064` that a *smaller* mirror (125,607 manifests) produced a
  15-minute zero-byte run through the same collector.
- **Which of B1 or B2 is actually first to blow up.** B1 runs first (line 70031 vs
  70042) and reads more bytes (48.6 GB vs the reachable subset of ~41 GB), so I rank it
  first — but both are unbounded and neither has ever run here, so the ordering is an
  inference from the code and the byte counts, not an observation.
- **How many of the 147,844 manifests verify.** `by_source_key` is populated only from
  manifests passing `doctor_raw_mirror_manifest_is_verified` (`src/lib.rs:36204,36171`),
  which requires both checksums matched. I did not verify checksums myself. If a large
  fraction fail, B2's hashing branch fires less often — but B1's cost is unchanged,
  because B1 is what *computes* those checksums.

## Recommendation for the fix

The pattern from `46d74410` applied to the integrity probe alone will move the hang from
`src/lib.rs:69653` to `src/lib.rs:70031`, and the next symptom will look identical from
outside: `cass doctor --json` returns zero bytes and never finishes. The same one-`stat`
gate wants to cover, at minimum, the pair at `src/lib.rs:70031` and `70042` — which is
what `cass status` already does at `src/lib.rs:66077-66079`, on both dimensions, using
the two predicates that already exist and are already tested. Doctor needs its own
honest-degrade shape for those three reports (`raw_mirror`, `raw_mirror_backfill`,
`source_inventory`), not a skip that leaves the JSON reading like a measurement.
