# Lane log — triage: FTS repair mode (AlreadyHealthy / IncrementalCatchUp vs Rebuilt)

Append-only. Owner: triage-fts-repair-mode lane, generation 14.
Group: the last two of the eight forward-pin failures.

    indexer::tests::full_run_fallback_fts_repair_skips_rebuild_when_fts_is_already_healthy
    storage::sqlite::tests::ensure_fts_consistency_via_rusqlite_catches_up_missing_rows

---

## 1. Binary provenance verified by content (not mtime)

```
$ strings .../target/debug/deps/coding_agent_search-983a915ea0c0a592 | rg -o 'fsqlite-core-0\.1\.[0-9]+' | sort -u
fsqlite-core-0.1.5
$ strings /tmp/cass-759l7-forward-target/.../coding_agent_search-b9364c709c6f41e6 | rg -o 'fsqlite-core-0\.1\.[0-9]+' | sort -u
fsqlite-core-0.1.19
```

Both binaries are what they claim to be.

## 2. Both failures reproduce; both pass on the shipping pin

Shipping (0.1.5):

```
running 1 test
test storage::sqlite::tests::ensure_fts_consistency_via_rusqlite_catches_up_missing_rows ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5153 filtered out; finished in 0.26s
```

Forward (0.1.19):

```
thread 'storage::sqlite::tests::ensure_fts_consistency_via_rusqlite_catches_up_missing_rows' (373106450) panicked at src/storage/sqlite.rs:22769:9:
assertion `left == right` failed
  left: Rebuilt { inserted_rows: 2 }
 right: IncrementalCatchUp { inserted_rows: 1, total_rows: 2 }
```

```
running 3 tests
test indexer::tests::full_run_fallback_fts_repair_rebuilds_missing_schema_when_needed ... ok
test indexer::tests::full_run_fallback_fts_repair_skips_known_healthy_archive_fingerprint ... ok
test indexer::tests::full_run_fallback_fts_repair_skips_rebuild_when_fts_is_already_healthy ... 
thread '...' panicked at src/indexer/mod.rs:37365:9:
assertion `left == right` failed
  left: Some(Repaired(Rebuilt { inserted_rows: 4 }))
 right: Some(Repaired(AlreadyHealthy { rows: 4 }))
```

Note the two SIBLING tests in the same file pass on 0.1.19: the one that expects a
full `Rebuilt` when the FTS schema is missing, and the one that expects the
fingerprint short-circuit. So the rebuild machinery itself works on 0.1.19.

## 3. Instrument note: RUST_LOG buys nothing here

```
$ RUST_LOG=debug <forward binary> --nocapture ensure_fts_consistency_via_rusqlite_catches_up_missing_rows
(identical output to the run without RUST_LOG — no tracing lines)
```

The test harness installs no tracing subscriber (`rg -n 'tracing_subscriber' src/`
finds only per-test subscribers inside unrelated tests), so the
`target: "cass::fts_rebuild"` debug counters inside `stream_fts_rows_via_frankensqlite`
are NOT available. I cannot read the branch directly from logs. Recorded because it
shaped what evidence was reachable.

## 4. Wider FTS blast radius on the forward pin — 43 pass, 4 fail

```
$ <forward binary> --test-threads=4 fts
...
failures:
    indexer::tests::full_run_fallback_fts_repair_skips_rebuild_when_fts_is_already_healthy
    storage::sqlite::tests::ensure_fts_consistency_via_rusqlite_catches_up_missing_rows
    storage::sqlite::tests::franken_storage_open_repairs_duplicate_fts_messages_schema_rows
    storage::sqlite::tests::rebuild_fts_via_rusqlite_cleans_duplicate_legacy_schema_rows

test result: FAILED. 43 passed; 4 failed; 0 ignored; 0 measured; 5107 filtered out; finished in 1.47s
```

The other two failures are a DIFFERENT signature (not mine):

```
called `Result::unwrap()` on an `Err` value: opening frankensqlite db at .../test_open_repairs_duplicate_fts_schema.db
Caused by:
    database disk image is malformed: FTS5 table `fts_messages` is missing required content shadow table `fts_messages_content`
```

Load-bearing consequence: 43 FTS tests PASS on 0.1.19, including ones that write
FTS rows and read them back by `COUNT(*)`. So "reading rows back out of an fts5
table is broken on 0.1.19" is FALSIFIED. Whatever changed is narrower than that.

## 5. Mistake I made and corrected: the shipping tree is mutating under me

My first `wc -l src/storage/sqlite.rs` (relative, cwd = shipping worktree) returned
**26185**. Later, the same file at the same absolute path returned **26203**:

```
$ pwd
/Users/dalecarman/dev/coding_agent_session_search/.claude/worktrees/cass-759l7-spin-wait
$ wc -l src/storage/sqlite.rs
   26203 src/storage/sqlite.rs
$ /bin/ls -l src/storage/sqlite.rs
-rw-r--r--@ 1 dalecarman  staff  994265 Aug 16 20:35 src/storage/sqlite.rs
```

The coordinator is landing the other failure groups' fixes into the shipping tree
while I read it. Every line number I derived from the shipping tree before 20:35 is
stale by +18 for anything past line ~3005. I re-anchored all citations on the
FORWARD tree (`/tmp/cass-759l7-forward`), which is stable and is where the failures
are reported.

Cross-tree diff of the two source files that differ, to confirm neither touches the
FTS decision code:

```
$ diff -rq <shipping>/src /tmp/cass-759l7-forward/src
Files .../src/pages/encrypt.rs and /tmp/cass-759l7-forward/src/pages/encrypt.rs differ
Files .../src/storage/sqlite.rs and /tmp/cass-759l7-forward/src/storage/sqlite.rs differ
```

sqlite.rs delta is one comment block + two entries in `has_db_sidecar_suffix`'s
`SIDECAR_SUFFIXES` around line 3005 (a different lane's landed fix for the
`-fsqlite-ns-*` sidecars). encrypt.rs delta is a `TryFromIntError` Display wording
fix (rustc 1.94 vs 1.99). Neither is in the FTS path, and both tests in my group are
byte-identical across the two trees.

Cargo.toml delta is exactly the pin:

```
-frankensqlite = { version = "0.1.5", package = "fsqlite", features = ["fts5"] }
+frankensqlite = { version = "0.1.19", package = "fsqlite", features = ["fts5"] }
-fsqlite-types = { version = "0.1.5", package = "fsqlite-types" }
+fsqlite-types = { version = "0.1.19", package = "fsqlite-types" }
```
