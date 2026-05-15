---
task: T4
date: 2026-05-15
spec: 014-pi-agent-memset-stall
profile_pid: 87884
profile_binary: target/profiling/cass (HEAD 05ba881b, profiling profile, debuginfo=2 strip=false)
---

# T4 — Profile Evidence

## Reproduction summary

- Binary: `target/profiling/cass` rebuilt from `HEAD = 05ba881b` (current `dac/main`, post-PR-#233 chunk-size fix).
- Watcher (`com.cass.index-watch`) stopped via `launchctl bootout` to release the index-run lock.
- Command: `./target/profiling/cass index --watch-once ~/.pi/agent/sessions --json --no-progress-events`.
- Repro PID 87884, launched 12:16:37.
- Stall reproduced cleanly within ~80 s; sampled at 12:22:51 (≈6 min elapsed, 5+ min plateau at 99 % CPU).

## RSS + row-count timeline (from `notes/T3-monitor.csv`)

| Elapsed | pi_count | CPU % | RSS (MB) |
|--------:|---------:|------:|--------:|
| 00:19   | 33       | 48.0  | 10,690   |
| 00:49   | 33       | 19.5  | 23,533   |
| 01:19   | 33       | 97.3  | 30,805   |
| 01:49   | 33       | 100.0 | 35,240   |
| 02:19   | 33       | 99.7  | 36,418   |
| 02:49   | 33       | 99.3  | 39,405   |
| 03:19   | 33       | 94.5  | 41,957   |
| 03:49   | 33       | 93.7  | 42,714   |
| 04:19   | 33       | 94.8  | 47,875   |
| 04:49   | 33       | 100.0 | **49,849** (peak observed) |
| 05:19   | 33       | 100.0 | 48,961   |
| 05:49   | 33       | 100.0 | 49,038   |
| 06:19   | 33       | 0.0*  | 47,146   |
| 06:49   | 33       | 99.4  | 46,902   |

\* CPU = 0 at 06:19 is the brief window during which `sample`/`lsof`/`vmmap` ran against the process.

- Pi conversation row count frozen at **33** for the entire run (= the pre-existing count; no progress).
- RSS climbed steadily for ~5 min and plateaued at **≈48–50 GB** (well past the spec's 22 GB evidence).
- CPU sustained at 94–100 % from the 80-s mark onward.
- `lsof -p 87884 | grep jsonl` returned **zero** open `.jsonl` files at sample time — so the stall is **not** in FAD's `fs::read_to_string(file)` path. All `.jsonl` files have already been read and closed by sample time.

## Stack analysis — the busy thread

Hot path on the main thread (`DispatchQueue_1: com.apple.main-thread`, 3610 samples, all 99 %+):

```text
main (lib.rs:5801)
  └─ run_index_with_data (lib.rs:74059)
     └─ Connection::execute_with_params (fsqlite_core/connection.rs:12788)
        └─ Connection::execute_prepared_with_params_after_background_status
            (fsqlite_core/connection.rs:12952)
           └─ Connection::with_internal_statement_savepoint…
                execute_precompiled_prepared_insert_fast (fsqlite_core/connection.rs:20677)
              ├─ Connection::live_vtab_savepoint_all (fsqlite_core/connection.rs:10009)   [2,822 samples]
              │  └─ Fts5Table::ErasedVtabInstance::savepoint (fsqlite_ext_fts5/vtab.rs:690)
              │     └─ Fts5Table::snapshot_state (fsqlite_ext_fts5/lib.rs:2147–2148)      [1,924 samples]
              │        └─ <HashMap<SmallText, SmallVec<Posting>>>::clone (map.rs:194)     [1,677 samples]
              │           └─ _platform_memmove (libsystem_platform.dylib)                 [502 samples]
              │              + SmallVec::try_grow → _platform_memmove etc.
              │              + _xzm_xzone_malloc (libsystem_malloc.dylib)                 [257 samples]
              └─ Connection::live_vtab_release_all (fsqlite_core/connection.rs:10127)     [776 samples]
                 └─ Fts5Table::ErasedVtabInstance::release (fsqlite_ext_fts5/vtab.rs:693)
                    └─ <InvertedIndex>::drop_in_place (fsqlite_ext_fts5/mod.rs:805)       [254 + 254 + 31]
                       └─ _xzm_free → _platform_memset / __bzero                          [115 + 35 + … samples]
```

Bottom-of-stack symbol summary (top frames by sample count):

| Samples | Symbol |
|--------:|--------|
| 57,760  | `semaphore_wait_trap` (idle worker threads) |
| 3,609   | `__semwait_signal` (idle main-thread sleeps between phases) |
| 3,579   | `semaphore_timedwait_trap` (idle merge threads) |
| **1,677** | **`hashbrown::HashMap<SmallText, SmallVec<Posting>>::clone` (`fsqlite_ext_fts5`)** |
| 502     | `_platform_memmove` |
| 257     | `_xzm_xzone_malloc` |
| 255     | `_xzm_free` |
| **254** | **`<InvertedIndex>::drop_in_place` (`fsqlite_ext_fts5`)** |
| 115     | `_platform_memset` |

The 16 `asupersync-worker-*` threads, 16 `thrd-tantivy-index*` threads, and the rayon pool are all in `__psynch_cvwait` / `semaphore_wait_trap` — **starved**, exactly as spec 014 described, but the starver is a single main-thread allocator loop inside the FTS5 vtab savepoint/release, not the cass clone chain.

## vmmap evidence (notes/T3-vmmap.txt)

```text
Physical footprint:         33.1G  (live)
Physical footprint (peak):  43.1G  (peak before sampling)
Writable regions:           53.2G total, 47.3G written, 45.8G resident, 1.8G swapped

REGION TYPE              VIRTUAL   RESIDENT   DIRTY     COUNT
MALLOC_SMALL              30.8 GB   28.7 GB    26.1 GB   7,891   ← live FTS5 inverted-index allocations
MALLOC_SMALL (empty)      19.8 GB   15.1 GB     3.1 GB   5,056   ← fragmented free space left by clone/drop cycles
MALLOC_LARGE               2.1 GB    1.7 GB     1.7 GB     482
DefaultMallocZone          52.9 GB virtual, 30.9 GB allocated, 2.0 GB frag (7%), 42,955,363 allocations
```

42.9 million live allocations is consistent with a HashMap<SmallText, SmallVec<Posting>> where SmallText keys are per-term and SmallVec<Posting> holds per-doc postings. Each insert into the SQLite FTS5 vtab causes a savepoint that clones this entire map.

## Top three Rust frames driving `_platform_memset` / `_platform_memmove`

1. **`fsqlite_ext_fts5::Fts5Table::snapshot_state`** at `fsqlite_ext_fts5/lib.rs:2147–2148`
   — clones the entire in-memory `InvertedIndex` on every savepoint.
2. **`fsqlite_ext_fts5::InvertedIndex::clone`** at `fsqlite_ext_fts5/lib.rs:1187` and `1193`
   — body of (1); the actual `HashMap::clone` lives here.
3. **`fsqlite_ext_fts5::InvertedIndex::drop_in_place`** at `fsqlite_ext_fts5/mod.rs:805`
   — releases the just-cloned snapshot when the SQLite transaction commits or releases.

Plus the per-Posting `SmallVec::try_grow` at `fsqlite_ext_fts5/lib.rs:1177–1215`.

## Currently open jsonl files

`lsof -p 87884 | grep jsonl` → **empty**. The repro process has already finished reading all source files. The stall is entirely in the in-memory FTS5 vtab state plumbing on the persist/commit path, not in scan-time.

## Implications for the spec 014 candidate space

This profile is incompatible with the binding decision tree in `plan.md ## Architecture` as written:

- **The bytes driving the memset loop are NOT in the cass clone chain** (`map_to_internal`, MessagePack serialize, lexical packets). Candidates C1 / C3 alone do not address this.
- **They are NOT in FAD scan-time** (`fs::read_to_string`, `val.clone`). Candidates C2 / C4 do not address this.
- **They are NOT in `ingest_watch_batch_with_oom_split` batch construction**. Candidate C5 does not address this.

The bytes are in **`frankensqlite_ext_fts5`** — a different crate from FAD, a different layer from cass — specifically in `Fts5Table::snapshot_state`'s decision to clone the entire in-memory `InvertedIndex` on every SQLite virtual-table savepoint, executed once per row inserted by the indexer. The cost is O(terms × postings × inserts), which is quadratic in the size of the existing FTS5 index against the number of inserts in flight.

This also explains the spec's "worked for claude_code/codex/opencode but stalls on pi" observation: those connectors ran when the FTS5 index was empty/small, so each savepoint clone was cheap. By the time pi_agent runs (after claude_code 2,573 + codex 5,712 + opencode 976 + others ≈ 9.3 K conversations already indexed), the inverted-index snapshot cost is catastrophic.

**T5 should escalate to user**: the planned C1–C5 fixes will not satisfy acceptance criterion #2 (peak RSS < 8 GB) because the dominant memory allocator is two layers below cass and one layer below FAD.

## Source code refs (frankensqlite checkout)

`Cargo.toml:45`:
```
frankensqlite = { version = "0.1.3", git = "https://github.com/Dicklesworthstone/frankensqlite",
                  rev = "eba969ec45d102071b90519d3b819ddbcecf3d61",
                  package = "fsqlite", features = ["fts5"] }
```

Local checkout: `~/.cargo/git/checkouts/frankensqlite-1cf785d0bee3c042/` (rev to be confirmed; `cargo metadata` will resolve to the pinned rev `eba969e`).

## Files (this run)

- `notes/T2-repro.log` — repro process stdout/stderr (currently empty: `--json` ran without writing structured output before kill).
- `notes/T3-sample.txt` — full 10-second sample at minute 6 of the stall (1263 lines).
- `notes/T3-vmmap.txt` — vmmap summary.
- `notes/T3-monitor.csv` — RSS + pi_count + CPU samples every 30 s.

## End-of-task state

- Indexer (PID 87884) killed cleanly at 12:24 after evidence capture.
- `launchctl bootstrap` reloaded `com.cass.index-watch`; new PID 30892 confirmed running.
- DB pi_agent count unchanged at 33 (no progress, no regression).
