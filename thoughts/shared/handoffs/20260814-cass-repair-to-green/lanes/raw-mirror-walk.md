# Lane: raw-mirror-walk

**Owner:** read-only grounding lane (Claude Opus 5, 1M)
**Date:** 2026-08-14
**Scope:** bead `nvq59` (`cass status --json` never returns) plus the undocumented
`cass doctor` hang. Both are live defects on the **currently installed** pre-fix
binary, independent of the coverage-floor fix.
**Repo HEAD at lane start:** `37d52925` (`beads(nvq59): the status --json hang is a 20 GB raw-mirror walk`)
**Binary under test:** `/Users/dalecarman/.local/bin/cass` (cass 0.6.9, pre-coverage-floor)

Every claim below is tagged **MEASURED** (I ran it and read the output) or
**INFERRED** (read from source/history without execution). No cargo was run. No
file outside this log was written. No mutating command was issued.

---

## 0. Headline

`cass status --json` and `cass doctor` both call `collect_doctor_raw_mirror_report`,
which **walks all 125,607 raw-mirror manifests and BLAKE3-hashes all 125,601
blobs — 19.68 GiB of content — on every invocation, with no limit, no budget,
no cache, and no size gate.** Plain `cass status` escapes only because the call
sits inside the structured-output branch.

The gate that used to prevent this **existed and was deleted by accident** on
2026-05-28, in a commit whose own message says it was fixing a build break.

A cheaper collector that answers the same questions in **28 seconds** already
ships in the same binary and is already wired to `cass stats`.

---

## 1. What plain `cass status` does that `--json` does not

### The branch

`run_status` is `src/lib.rs:64688`. Both paths share everything up to
`src/lib.rs:64912`:

```rust
    let structured_format = output_format.or_else(robot_format_from_env).map(|fmt| { ... });

    if let Some(fmt) = structured_format {
        //  <-- everything expensive lives in here
        ...
        return output_structured_value(payload, fmt);
    }

    let status_icon = ...   //  <-- the human text path starts here
```

**INFERRED (source, `src/lib.rs:64912-65035`):** the entire doctor-collector
block — coverage risk, source inventory, remote-source sync, quarantine report,
topology budget — is inside `if let Some(fmt) = structured_format`. The human
text path never reaches any of it. That is the whole difference. It is not a
deliberate "fast status" decision for coverage; coverage collection simply was
never added to the text path.

### "Counts skipped for fast status on large database" is a *different* mechanism

**INFERRED (source):** that string is `src/lib.rs:65100` and it reports
`counts_skipped`, which comes from `state_meta_json_inner`
(`src/lib.rs:15101-15105`):

```rust
    let include_counts = include_counts_override.unwrap_or_else(|| {
        db_size_bytes
            .map(|size| size <= STATUS_COUNT_SCAN_MAX_DB_BYTES)
            .unwrap_or(false)
    });
```

`STATUS_COUNT_SCAN_MAX_DB_BYTES = 256 * 1024 * 1024` (`src/lib.rs:15065`).

This runs **before** the branch, so it applies to `--json` too. It governs
`SELECT COUNT(*)` on the archive DB, **not** the raw-mirror walk. So the
question "why does the --json path not take the same skip?" has a precise
answer: **it does take that skip.** The DB-count skip is not the skip that
matters, and the skip that *would* have mattered was removed.

**MEASURED** — the skip is visibly active:

```
$ perl -e 'alarm shift; exec @ARGV' 60 /Users/dalecarman/.local/bin/cass status
plain status rc=0 elapsed=0s
! CASS Status: Attention needed
...
Database:
  Counts skipped for fast status on large database
```

### The skip that was removed

**INFERRED (git):** the gate was introduced with the coverage collector itself,
in `fcc9f385` (2026-05-05, *"feat(doctor): emit unified runtime summary across
surface variants"*). Its added line:

```
$ git show fcc9f385 -- src/lib.rs | rg -n 'status_collects_coverage'
369:+        let status_collects_coverage = db_exists && !status_should_skip_db_open(&db_path);
```

`status_should_skip_db_open` was a pure size predicate
(`git show fe3972dc^:src/lib.rs`, line 15617):

```rust
fn status_should_skip_db_open(db_path: &Path) -> bool {
    std::fs::metadata(db_path).ok().is_some_and(|metadata| {
        metadata.is_file() && metadata.len() > STATUS_COUNT_SCAN_MAX_DB_BYTES
    })
}
```

The live DB is 7.93 GB, so on this archive the predicate was `true`,
`status_collects_coverage` was `false`, and the else-branch
(`doctor_fast_coverage_risk_unchecked`, `"status-fast-state"`,
`coverage_checked: false`) ran. **Status never walked the mirror.**

### How it got turned on

**INFERRED (git, two commits four minutes apart):**

| sha | date | what it did |
|---|---|---|
| `fe3972dc` | 2026-05-28 02:18:15 -0400 | `feat(state): detect scan-ahead-of-projection stale index via last_scan_ts` — deleted `status_should_skip_db_open` and set `skip_db_open=false` in `state_meta_json_for_status`. Its message describes **only** the DB-open policy. It did **not** touch the `run_status` call site, leaving the build broken. |
| `b8e3e78b` | 2026-05-28 02:22:35 -0400 | `fix(state): drop dangling status_should_skip_db_open call site in run_status` — replaced the expression with `db_exists`. |

`b8e3e78b`'s message states its own rationale:

> This left the build broken against the removed symbol and re-imposed the
> size-based skip on the doctor coverage-risk collection path that fe3972dc
> was supposed to retire.

**That sentence is wrong, and it is the root cause.** `fe3972dc`'s message
retires exactly one thing — trusting index mtime instead of opening the DB
("switches the status probe to always open the database instead of trusting an
index-mtime-based size optimization"). It says nothing about coverage
collection. The deleted helper was serving **two unrelated policies** through
one boolean; the build-break repair inlined `true` for both. The current
in-source comment (`src/lib.rs:64931-64934`) repeats the same conflation:

```rust
        // Commit fe3972dc deliberately dropped status_should_skip_db_open and
        // its STATUS_COUNT_SCAN_MAX_DB_BYTES short-circuit; the policy is now
        // "always probe via DB open" — see the commit message. This call site
        // was missed in that cleanup; inlining the now-unconditional `true`.
        let status_collects_coverage = db_exists;
```

**INFERRED:** so the answer to "was the walk always in status, or did it leak in
from doctor?" is neither. The walk was written for status from day one
(`fcc9f385`), but **gated off on any archive over 256 MB**. It leaked in on
2026-05-28 as collateral damage from a DB-open policy change.

Corroborating textual evidence that the gate *was* the boundedness — the
existing test's own assertion message (`tests/cli_doctor.rs:4513`):

> "status doctor_summary should explain the **bounded** inline coverage source"

The label it pins is `"status-inline-small-archive"` (`tests/cli_doctor.rs:4514`).
On a 7.93 GB DB and a 19.68 GiB mirror, `cass status --json` today reports its
coverage provenance as *small archive*. **The instrument label is now a false
claim** — the exact shape `.claude/rules/instrument-labels.md` warns about.

---

## 2. The raw-mirror walk: what it is and what it is FOR

### Entry point

`collect_doctor_raw_mirror_report` → `collect_doctor_raw_mirror_report_with_threshold`
(`src/lib.rs:33756`, `33763`).

**INFERRED (source, `src/lib.rs:33808-33846`):** it `walkdir`s
`<data_dir>/raw-mirror/v1/manifests`, reads every `*.json` to a `String`, parses
it with serde, and pushes a `DoctorRawMirrorManifestReport` into
`report.manifests`. **There is no `limit`, no `truncated` flag, no deadline, and
no early exit.**

### The cost centre

Each manifest goes to `doctor_verify_raw_mirror_manifest` (`src/lib.rs:33518`),
which after cheap structural checks does this (`src/lib.rs:33620-33627`):

```rust
        Ok(metadata) => {
            let size_matches = metadata.len() == manifest.blob_size_bytes;
            match doctor_file_blake3(&blob_path) {
```

and `doctor_file_blake3` (`src/lib.rs:33405`) streams the **entire blob** through
a BLAKE3 hasher in 64 KiB chunks. Every manifest ⇒ one full blob read.

### What the walk is FOR

**INFERRED (source + its own comments).** Three jobs, and they are not equally
expensive:

1. **Integrity verification** — `blob_checksum_status`, `manifest_checksum_status`,
   `checksum_mismatch_count`, `missing_blob_count`. This is the *only* part that
   needs the hashing. The report's own notes say why the mirror matters
   (`src/lib.rs:33788-33790`): *"Raw mirror blobs are precious evidence and are
   never automatic cleanup candidates"* / *"A verified mirror blob remains useful
   when the upstream provider file has been pruned."*
2. **Orphan / id-mapping detection** — `mirror_without_db_link_count`,
   `raw_mirror_db_link_count`, from `manifest.db_links`, which is **manifest
   metadata**, not blob content.
3. **Size accounting** — `total_blob_bytes`, `duplicate_blob_reference_count`,
   from `metadata.len()` and the content-addressed path, again **not** content.

The coupling that forces (2) and (3) to pay for (1) is
`build_doctor_coverage_summary` (`src/lib.rs:35974-35985`), which filters through
`doctor_raw_mirror_manifest_is_verified` (`src/lib.rs:35328`):

```rust
fn doctor_raw_mirror_manifest_is_verified(manifest: &DoctorRawMirrorManifestReport) -> bool {
    manifest.status == "verified"
        && manifest.blob_checksum_status == DoctorArtifactChecksumStatus::Matched
        && manifest.manifest_checksum_status == DoctorArtifactChecksumStatus::Matched
}
```

so the *coverage* numbers are only counted over *hash-verified* manifests.
**This matters for the fix — see §6 risk 1.**

### Store measurements

**MEASURED:**

```
$ /bin/ls -la "$HOME/Library/Application Support/com.coding-agent-search.coding-agent-search/raw-mirror/v1"
drwx------@     3 blobs
drwx------@ 65535 manifests      <- flat directory, link count saturated at 65535
drwx------@     2 tmp

$ fd -t f -e json . ".../raw-mirror/v1/manifests" | wc -l
  125607
$ fd -t f . ".../raw-mirror/v1/blobs" | wc -l
  125601
$ /usr/bin/du -sk ".../blobs" ".../manifests"
21015764  .../blobs        (20.04 GiB)
  502428  .../manifests    ( 0.48 GiB)
```

Confirms the established facts (125,601 blobs / 125,607 manifests / ~20 GB) and
adds two things: the manifests directory is **flat**, and reading every manifest
alone is ~491 MiB of small-file I/O.

Cost of the two cheap passes the walk could have used instead — **MEASURED**
(warm cache, so these are floors, not cold-start figures):

```
$ /usr/bin/du -sk ".../blobs" >/dev/null        # stat all 125,601 blobs
metadata-only stat pass over blobs: rc=0 elapsed=2s

$ fd -t f -e json . ".../manifests" -0 | xargs -0 cat > /dev/null
read-all-manifest-bytes pass: rc=0 elapsed=20s
```

So: **2 s to stat every blob, 20 s to read every manifest, ≥900 s to hash every
blob.** The hashing is the entire cost.

`cass stats` reports the same store from the cheap collector — **MEASURED**:

```
Raw Mirror:
  Storage bytes: 21326374995
  Manifests: 125607
  Unique blobs: 125601
  Blob bytes: 21126756078      (19.68 GiB)
  Largest blob bytes: 851886080 (812 MiB — a single blob)
```

---

## 3. Reproduction, with the mechanism caught in the act

**MEASURED** — plain vs structured, same binary, same machine, minutes apart:

| command | rc | elapsed | bytes out |
|---|---|---|---|
| `cass status` | 0 | **0 s** | full human report |
| `cass health --json` | 1 (unhealthy is the documented nonzero) | **0 s** | 20,848 |
| `cass triage --json` | 0 | **0 s** | 24,319 |
| `cass stats` | 0 | **28 s** | full report incl. whole mirror |
| `cass status --json` | 142 (SIGALRM) | **90 s cap, no output** | 0 |
| `cass status --json` (long bound) | 142 (SIGALRM) | **900 s cap, no output** | 0 |
| `CASS_OUTPUT_FORMAT=json cass status` | 142 (SIGALRM) | **45 s cap, no output** | 0 |

**MEASURED, and this is the number to quote:** a single `cass status --json`
capped at 900 s ran the **full fifteen minutes at ~99% CPU and 3.95 GB resident,
then died on the alarm having written zero bytes to stdout and zero bytes to
stderr.** Not slow — silent. An agent calling it gets no partial output, no
progress, and no error, which is why it reads as a hang rather than as a long
command.

```
$ perl -e 'alarm shift; exec @ARGV' 900 cass status --json >/tmp/stj_long.json 2>/tmp/stj_long.err
rc=142 elapsed=900s bytes=0
$ wc -c /tmp/stj_long.err /tmp/stj_long.json
0 /tmp/stj_long.err
0 /tmp/stj_long.json
```

Two consequences of the last row. **MEASURED:** the hang is not flag-triggered —
`robot_format_from_env` (`src/lib.rs:22204-22214`) reads `CASS_OUTPUT_FORMAT` and
`TOON_DEFAULT_FORMAT`, so **an agent harness that exports a structured default
turns the 0-second command into the unbounded one without ever typing `--json`.**
And **MEASURED:** `cass triage --json` does *not* hang on this binary, so bead
`nvq59` is scoped to `status --json` + `doctor`; triage's hang under the
coverage-floor fix is bead `1a7mk`'s separate cause. **INFERRED (source):**
`run_triage` (`src/lib.rs:65166-65364`) contains no call to
`collect_doctor_coverage_risk_summary` or `collect_doctor_raw_mirror_report`.

### Caught in the act

**MEASURED** — `lsof` + `ps` against the live hung process:

```
t = 8 s
  PID ELAPSED  %CPU    RSS
44268   00:08  44.8 570384      (557 MB)
  lsof: 2 fds under raw-mirror/v1/manifests
  lsof: 0 fds on agent_search.db          <-- no sqlite work at all

t = 55 s
74465   00:55  41.2 530976
  lsof: 1 fd under raw-mirror/v1/blobs
  lsof: 1 fd under raw-mirror/v1/manifests   <-- interleaved read-manifest / hash-blob
```

That is `doctor_verify_raw_mirror_manifest`'s loop, observed directly: one
manifest open + one blob open at a time, zero database involvement. It confirms
the established mechanism and adds the blob fd, which the earlier measurement
did not name.

### Memory

**MEASURED** — a single 900 s-capped run, RSS sampled over time:

| elapsed | RSS | %CPU |
|---|---|---|
| 8 s | 0.57 GB | 45 |
| 68 s | 1.28 GB | 29 |
| 93 s | **3.95 GB** | 95 |
| 3 m 47 s | 3.95 GB (plateau) | 99 |
| 6 m 04 s | 3.95 GB | 98 |
| 8 m 55 s | 3.95 GB | 99 |
| 15 m 00 s | killed by the alarm, still running | — |

**INFERRED:** the plateau is `report.manifests` — a `Vec` of 125,607
`DoctorRawMirrorManifestReport`, each holding ~13 owned `String`s (manifest path,
redacted manifest path, blob relative path, blob path, redacted blob path, blob
blake3, provider, source id, origin kind, original path, redacted original path,
original path blake3, status). After the plateau the process is pure CPU on the
19.68 GiB of hashing.

This machine has 128 GB and was 78% free, so nothing was harmed. **A machine with
less headroom takes a ~4 GB resident spike from `cass status --json`.** That is a
finding in its own right and is not recorded in bead `nvq59`.

---

## 4. `cass doctor` reaches the same walk — unconditionally

**INFERRED (source):** `run_doctor_impl` (`src/lib.rs:67946`) calls it directly
at `src/lib.rs:68743-68744`:

```rust
    let raw_mirror_scan_started = Instant::now();
    let mut raw_mirror = collect_doctor_raw_mirror_report(&data_dir);
    doctor_push_timing_span(
        &mut timing_spans,
        "raw_mirror_scan",
        ...
        DOCTOR_SLOW_OPERATION_DEFAULT_THRESHOLD_MS,
        vec!["raw mirror manifests and blob checksums were summarized".to_string()],
    );
```

with no `command_surface` gate, no `execution_mode` gate, and no `fix` gate. So
**bare `cass doctor`, `cass doctor --check`, and `cass doctor --fix` all pay the
full 19.68 GiB re-hash.** `--check` is documented in the CLI as *"Run the
**bounded** read-only doctor truth surface"* (`src/lib.rs:769`); it is not
bounded.

It is worse than unbounded — it can run **twice**. When backfill applies
(`src/lib.rs:68768`), `raw_mirror = collect_doctor_raw_mirror_report(&data_dir);`
runs the entire walk a second time in the same invocation.

**INFERRED:** the code already knows this must be fast.
`DOCTOR_SLOW_OPERATION_DEFAULT_THRESHOLD_MS = 500` (`src/lib.rs:29918`) is the
threshold the `raw_mirror_scan` span is measured against. The span therefore
records "slow" past half a second and reports it **after** the fact, with no
budget and no progress output. That is the difference between a slow command and
a command indistinguishable from a hang.

**NOT MEASURED, deliberately.** I did not run `cass doctor`. My lane constraints
forbid mutating the data dir, and `run_doctor_impl` writes doctor run records
(`src/doctor_runs.rs`, e.g. `list_runs`/`read_actions` consumers at
`src/lib.rs:40664`, `41132`) and status event drafts
(`doctor_status_event_draft`, `src/lib.rs:48853`). The hang is established by
source identity with the `status --json` path — the same function, the same
store, and no gate — plus the coordinator's prior measurement. **This is a null I
chose, not a null I found: someone should confirm doctor's timing under a
throwaway `--data-dir`.**

Other production callers of the same walk (all inherit the cost) —
**INFERRED (`git grep`, non-test sites only):**

| site | surface |
|---|---|
| `src/lib.rs:36298` | `collect_doctor_coverage_risk_summary` → **`cass status --json`** |
| `src/lib.rs:68744`, `68768` | `run_doctor_impl` → **`cass doctor`** (twice when backfill applies) |
| `src/lib.rs:28832` | doctor forensic bundle |
| `src/lib.rs:34144` | `build_doctor_archive_scan_context` → `cass doctor archive-scan` |
| `src/lib.rs:44099` | `build_doctor_baseline_snapshot` → `cass doctor baseline` |

---

## 5. Is the walk O(store) by design, or accidentally unbounded?

**Accidentally unbounded, on four independent pieces of evidence.**

1. **A cheaper collector already ships, in the same binary, and answers the same
   questions.** `crate::raw_mirror::storage_summary` (`src/raw_mirror.rs:73-166`)
   reads each manifest and `symlink_metadata`s each blob — **no BLAKE3, no
   content read** — producing `manifest_count`, `unique_blob_count`,
   `total_blob_bytes`, `largest_blob_bytes`, `missing_blob_count`,
   `invalid_manifest_count`, and capture-time min/max. It is wired to `cass stats`
   at `src/lib.rs:23743`. **MEASURED: `cass stats` covers this exact
   125,607-manifest store and returns in 28 seconds.**

   Precision about that 28 s, because the label matters: it is the whole
   `cass stats` command, which also opens the 7.93 GB DB and runs several
   queries. So 28 s is an **upper bound** on `storage_summary` alone, not a
   measurement of it. The independent decomposition — **MEASURED** — is ~20 s to
   read all 125,607 manifest files (491 MiB of small-file I/O, warm) and ~2 s to
   stat all 125,601 blobs. Either way the honest comparison is *tens of seconds
   against ≥15 minutes and counting*.

2. **A bounded-walk pattern already exists in the same file, ~1,600 lines
   above.** `doctor_remote_mirror_top_level_entry_count(mirror_dir, limit)`
   (`src/lib.rs:32094-32122`) counts with an early exit and returns a `truncated`
   flag; `doctor_remote_mirror_file_fingerprint(mirror_dir, limit)`
   (`src/lib.rs:32126-32172`) walks with a `limit` and returns `truncated`. The
   raw-mirror walk takes neither. The author of the bounded ones and the author
   of the unbounded one had the same tools available.

3. **The "skip" contract for status is fully built and currently unreachable.**
   `doctor_fast_coverage_risk_unchecked` (`src/lib.rs:36310`) is the else-branch
   value; `"status-fast-state"` is its label; `coverage_checked: false` renders
   as `"not_checked"` with the recommended action *"Run cass doctor check --json
   for current archive coverage; health/status did not run deep collectors."*
   (`src/lib.rs:36622`). There is even a golden file already pinning it —
   `tests/golden/robot/status_quarantine.json.golden:419` → `"source":
   "status-fast-state"`. Every byte of the fast path is written, tested, and
   dead because the gate reads `db_exists`.

4. **The freshness contract implies a cache that does not exist.** The runtime
   summary emits `"stale_after_seconds": if input.coverage_checked { 300 }`
   (`src/lib.rs:36615`) — the JSON tells consumers a checked coverage reading is
   good for five minutes, while the producer recomputes 19.68 GiB from scratch on
   every call.

**Is there a cache, manifest index, or summary row to read instead?** **No —
MEASURED and INFERRED, a real null.** `rg` over `src/lib.rs` for
`raw_mirror_cache|mirror_summary_cache|raw-mirror-summary` returns nothing; the
only `raw_mirror_summary` binding is the live `storage_summary` call at
`src/lib.rs:23743`. No meta row, no sidecar index, no persisted verification
ledger. There is a persisted *baseline* mechanism (`build_doctor_baseline_snapshot`,
`src/lib.rs:43758`/`44099`) that already embeds the raw-mirror report, but it
recomputes rather than reuses, and nothing reads it back as a cache.

**What the content-addressed layout makes cheap.** A blob's *path is its BLAKE3*
(`doctor_raw_mirror_blob_relative_path`, `src/lib.rs:33296-33311`:
`blobs/blake3/<xx>/<64-hex>.<ext>`). So `(path, size, mtime, inode)` unchanged
since a recorded verification is strong evidence the content is unchanged.
**MEASURED: a metadata-only stat pass over all 125,601 blobs takes 2 seconds**
(`du -sk` on the blobs tree, warm). That is the ceiling for a ledgered
steady-state verification — 2 s against never.

---

## 6. The fix

Two surfaces, two different answers. They are not the same defect wearing two
hats: status has no business verifying 19.68 GiB, and doctor has every business
doing it — just not on every invocation with no budget.

### 6a. `cass status --json` → **(a) skip the walk, using the skip that already exists**

**Smallest correct change, one expression, `src/lib.rs:64935`:**

```rust
        let status_collects_coverage = db_exists;
```
becomes a gate on **the thing actually walked** — the raw-mirror store — rather
than restoring the old DB-size proxy:

```rust
        let status_collects_coverage = db_exists && !status_raw_mirror_scan_too_large(&data_dir);
```

with the helper written on the **existing in-file bounded-count precedent**,
`doctor_remote_mirror_top_level_entry_count` (`src/lib.rs:32094-32122`): a
`read_dir` over `raw-mirror/v1/manifests` that stops counting at the cap. Cost is
O(cap), not O(store).

Why gate on the mirror and not re-add the old DB-size predicate: the walk's cost
is a function of manifest count and blob bytes, and the DB size only correlates
with those by accident. Re-adding `metadata(db_path).len() > 256 MB` would fix
this machine and mis-fire on a small DB with a large mirror — which is exactly
the shape a pruned-and-reindexed archive takes.

Everything downstream is already written: the else-branch, the label, the
`coverage_checked: false` rendering, the operator routing text, and a golden
file. **This deletes a hang without adding a mechanism** — §13's internalization
over addition.

**Closest existing precedent, three lines away in the same file:** `run_health`
(`src/lib.rs:65620-65623`) takes the fast path unconditionally —
`doctor_fast_coverage_risk_unchecked(db_exists)` +
`collect_doctor_remote_source_sync_fast_report` + `coverage_source:
"health-fast-state"` + `coverage_checked: false`. **MEASURED: `cass health --json`
returns in under a second on this same binary and this same archive, emitting
20,848 bytes.** Status should look like health.

Also fix the comment at `src/lib.rs:64931-64934`. It records a false history
(that `fe3972dc` retired the coverage skip) and it is what a future reader will
believe.

**Test impact — INFERRED, needs the coordinator's cargo run to confirm:**
`tests/cli_doctor.rs:4481-4514` runs `cass status --json` against a fixture
`--data-dir` and asserts `coverage_source.source == "status-inline-small-archive"`
and `status == "checked"`. The fixture mirror is tiny, so under a
manifest-count gate it stays below the cap, keeps the inline branch, and the test
stays green **unchanged**. Under a *DB-size* gate the same holds. This is the
main reason to prefer a gate over run_health's unconditional fast path, which
would flip that assertion and force a test edit.

**Rejected: (b) cache/summarize for status.** Right answer for doctor, wrong
altitude here — status does not need the number at all, and it already ships a
contract for saying so.

**Rejected: (c) remove the walk from status entirely.** Nearly right, and it is
what `run_health` does. It is one line simpler than the gate but it deletes real
capability for small archives and breaks a green test, so the gate is the
smaller net change. If the coordinator prefers strictly fewer moving parts, (c)
is defensible — say so explicitly and re-adjudicate `tests/cli_doctor.rs:4514`
rather than re-arming it silently
(`~/.agent-config/skills/meta/self-maintaining-tests` applies: a pin changed to
match new behavior without stated evidence is the same defect as deleting it).

### 6b. `cass doctor` → **(b) budget the walk now, ledger it next**

Doctor legitimately owns integrity verification, so the answer is not to skip.
Two steps, and only the first is needed to stop the hang.

**Step 1 (smallest correct fix): give the walk a limit and say so in the report.**
Reuse `doctor_remote_mirror_file_fingerprint`'s exact shape
(`src/lib.rs:32126-32172`): a `limit`, a `truncated` flag surfaced in
`DoctorRawMirrorSummary`, and a `status` of `"partial"` when it fires. Doctor
returns in bounded time and **states how much it verified**. Full verification
moves behind an explicit `--deep` (or the existing `archive-scan` surface).

**Step 2 (restores full coverage cheaply): a verification ledger.** Persist
`(blob_relative_path, size, mtime_ns, verified_at_ms, result)` per verified blob
under the data dir; re-hash only entries whose stat tuple moved. The layout makes
this sound because the path *is* the content hash. **MEASURED: the stat pass this
reduces to takes 2 seconds** against a first-run 19.68 GiB.

**Closest existing precedent:** `doctor_runs` (`src/doctor_runs.rs`) already
persists per-run records under the data dir, and `build_doctor_baseline_snapshot`
(`src/lib.rs:44099`) already persists a doctor snapshot **that includes the
raw-mirror report**. The persistence layer, its path conventions, and its
serialization are all built; step 2 is reading one back instead of only writing
it.

**Also fix, same change:** the double walk at `src/lib.rs:68744` + `68768`.
Refreshing after backfill should re-verify only the manifests backfill touched,
not all 125,607.

### 6c. What must NOT be done

Do **not** land 6b's truncation without handling this: `build_doctor_coverage_summary`
(`src/lib.rs:35974-35985`) counts **only** manifests passing
`doctor_raw_mirror_manifest_is_verified` (`src/lib.rs:35328`). A manifest that is
unverified *because it was not scanned* is arithmetically identical to one that
is unverified *because its blob is missing*. A truncated scan would therefore
inflate `mirror_without_db_link_count` and `db_without_raw_mirror_count`, and
`build_doctor_coverage_summary` routes those straight to *"Run 'cass doctor --fix
--json' to add raw-mirror coverage for eligible live source files."*
(`src/lib.rs:36043`). **A bounded scan that reports a false coverage gap points
the operator at a mutation.** `unscanned` must be its own state, distinct from
`unverified`, before any limit ships.

---

## 7. Repo rules — compliance and observations

**Reported as required by the lane brief.** Read from
`/Users/dalecarman/dev/coding_agent_session_search/AGENTS.md`.

| rule | source | bearing on this fix |
|---|---|---|
| **Never delete a file** — *"YOU ARE NEVER ALLOWED TO DELETE A FILE WITHOUT EXPRESS PERMISSION."* | `AGENTS.md:15` | Not engaged. Both fixes are edits; §6a deletes no test, and §6c explicitly forbids silently re-arming one. |
| **No `rusqlite` in new code**; *"existing rusqlite usage is LEGACY DEBT, not a pattern"* | `AGENTS.md:53-54, 63` | Not engaged. The whole fix is filesystem + one boolean. No new DB code. If step 2's ledger is ever put in SQLite instead of a file, it must be `frankensqlite::Connection` (`AGENTS.md:56`). |
| **Work on main; push `main:master` after** | `AGENTS.md:39, 44` | Coordinator's, not mine. |
| **Compiler gates:** `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` | `AGENTS.md:233` | I ran none — lane constraint. The documented `rch exec --` wrapper is absent on this Mac; explicit `CARGO_TARGET_DIR` is the local equivalent. |
| **UBS pre-merge gate:** `ubs --format=json --ci <changed files>`, base-vs-current | `AGENTS.md:243-254, 940` | Coordinator's. Changed file will be `src/lib.rs` (already a huge existing-findings surface, so base comparison matters). |

**Observation, not a rule violation — `git log -S` errors on a missing object:**

```
$ git log -S 'status_collects_coverage' --oneline
fatal: unable to read 14dc83a5018fc18a53477625e3a8d778afc45a64
2aa2cc92 feat(doctor): doctor v2 ...
fcc9f385 feat(doctor): emit unified runtime summary ...
```

**MEASURED:** the same `fatal:` line appears on every `git log -S` I ran (four
separate invocations, different pickaxes). Results still came back, so history
search works, but **one object in this repo's object database is unreadable.**
That is worth a `git fsck` by whoever owns repo health; it is outside my lane and
I did not chase it. Flagging it because a partially-unreadable object store can
silently truncate a history search, and history search is how this lane found the
root cause.

---

## 8. Commands run (complete)

Read-only. Nothing mutating. Nothing written outside this file.

```bash
git log --oneline -5
git grep -n 'Counts skipped for fast status' -- src/
git grep -n 'status_collects_coverage' -- src/
git grep -n 'fn run_status' -- src/
git grep -ln 'raw_mirror\|raw-mirror' -- src/
rg -n 'fe3972dc|status_should_skip_db_open|STATUS_COUNT_SCAN_MAX_DB_BYTES' src/
rg -n 'fn collect_doctor_(coverage_risk_summary|source_inventory|raw_mirror_report|raw_mirror_backfill_report)' src/
rg -n 'doctor-raw-mirror-manifest-id-v1' src/
rg -n 'fn doctor_file_blake3|fn file_blake3_hex' src/lib.rs
rg -n 'fn run_doctor_impl' src/
rg -n 'collect_doctor_raw_mirror_report\b' src/
rg -n 'DOCTOR_SLOW_OPERATION_DEFAULT_THRESHOLD_MS' src/lib.rs
rg -n 'raw_mirror_summary|raw_mirror_cache|mirror_summary_cache|raw-mirror-summary' src/lib.rs
rg -n 'doctor_fast_coverage_risk_unchecked|health-fast-state|status-inline-small-archive|status-fast-state' src/lib.rs
rg -n 'status-inline-small-archive|status-fast-state|coverage_source' tests/
rg -n -A12 'fn robot_format_from_env' src/lib.rs
rg -n -i 'delete|rusqlite|main:master|ubs |cargo clippy' AGENTS.md

git log -S 'raw-mirror' --oneline -- src/
git log -S 'doctor-raw-mirror-manifest' --oneline
git log -S 'status_collects_coverage' --oneline
git log -S 'status-inline-small-archive' --oneline
git log -S 'status_should_skip_db_open' --oneline
git log --oneline -S 'let status_collects_coverage = db_exists;'
git show --format=full --stat fcc9f385
git show --format=full --stat fe3972dc
git show --format=full --stat b8e3e78b
git show fcc9f385 -- src/lib.rs | rg -n 'status_collects_coverage'
git show fe3972dc -- src/lib.rs | rg -n 'status_should_skip_db_open|skip_db_open'
git show 'fe3972dc^:src/lib.rs' > /tmp/prefe.rs   # scratch, outside repo

/bin/ls -la "$M"
fd -t f -e json . "$M/manifests" | wc -l
fd -t f . "$M/blobs" | wc -l
/usr/bin/du -sk "$M/blobs" "$M/manifests"
fd -t f -e json . "$M/manifests" -0 | xargs -0 cat > /dev/null   # timed manifest-read pass

perl -e 'alarm shift; exec @ARGV' 60  .../cass status
perl -e 'alarm shift; exec @ARGV' 90  .../cass status --json
perl -e 'alarm shift; exec @ARGV' 900 .../cass status --json     # long bound
CASS_OUTPUT_FORMAT=json perl -e 'alarm shift; exec @ARGV' 45 .../cass status
perl -e 'alarm shift; exec @ARGV' 60  .../cass health --json
perl -e 'alarm shift; exec @ARGV' 60  .../cass triage --json
perl -e 'alarm shift; exec @ARGV' 180 .../cass stats
/usr/sbin/lsof -p <pid>    # x2, against the live hung status process
ps -p <pid> -o pid,etime,%cpu,rss,command   # repeated sampling
```

**Not run, on purpose:** `cargo` anything (coordinator owns the build);
`cass doctor` in any form (writes doctor run records to the data dir);
`cass index`, `cass models install`, `cass doctor --fix`.
