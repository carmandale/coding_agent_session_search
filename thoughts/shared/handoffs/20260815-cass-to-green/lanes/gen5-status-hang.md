# gen5 lane — nvq59: the `cass status --json` raw-mirror hang

Lane: read-only source survey. Worktree `.claude/worktrees/cass-gen5-honesty`,
branch `worktree-cass-gen5-honesty`, base commit `6bcc51b7`.
Written 2026-08-15T17:2xZ. No source file was edited; this log is the lane's
only write.

## Line-number basis — read this before checking any citation

`src/lib.rs` **changed underneath this lane while it ran.** At my first command it
was 91,859 lines; at 17:08:15Z it was 91,940. The delta is a single uncommitted
`+81` insertion at lines 15,312–15,402 inside `probe_state_db`, which is the
sibling lane's `-nao4q` work, not mine (`git diff -U0 src/lib.rs` hunk headers:
`@@ -15311,0 +15312,30 @@` and `@@ -15321,0 +15352,51 @@`).

Every line number below is **the working tree as of 91,940 lines**, and I re-read
each cited region at those numbers after the shift. For committed `HEAD`
(`6bcc51b7`), subtract 81 from any number above 15,311. Numbers at or below
15,311 are identical in both.

If the sibling has since committed, re-derive rather than trusting these.

## The question

What is the mechanism behind bead
`coding_agent_session_search-status-json-hang-nvq59`, and what is the smallest
correct fix?

## The short answer, which changes the shape of the lane

**The mechanism is confirmed, and the fix already landed on this base commit.**
`447d97fe` ("fix(status): bound the raw-mirror walk so `cass status --json`
returns", 2026-08-14T16:47:15-05:00) is an ancestor of `HEAD`
(`git merge-base --is-ancestor 447d97fe HEAD` → exit 0). The bead is still `open`
in `.beads/issues.jsonl` because the remaining work is **deploy and measure**, not
source.

So this lane's job turned into three things it could actually settle by reading:
whether the landed gate is complete, whether the fix the prompt recommended
(reuse the existing fast-path guard) would have been correct, and what is in the
stale worktree. All three have answers below, and one of them is a finding nobody
has filed.

## Method

- `rg` and `Read` against absolute paths in this worktree only. No `cargo`, no
  `cass`, no `br` mutation, no access to the live archive.
- Every cited line was opened with `Read` at the number cited, after the
  mid-session file shift was detected.
- Reachability was established in the **reverse** direction — enumerate every
  caller of the walking function, then ask which of those callers `run_status`
  can reach — rather than by guessing forward from `run_status`.
- One instrument I built was wrong and I discarded it: a `/tmp` function-line
  index built before the sibling's edit gave `build_doctor_archive_scan_context`
  at 34414, which `Read` showed to be a struct field. That contradiction is what
  exposed the +81 shift. Nothing in this log is cited from that index.

## Findings

### 1. Where `--json` diverges, and why the existing fast-path guard is the wrong instrument

`run_status` is at `src/lib.rs:65041`. The divergence is a single branch:

- `src/lib.rs:65272` — `if let Some(fmt) = structured_format {`
- `src/lib.rs:65400` — `return output_structured_value(payload, fmt);`

Everything between those two lines runs **only** under `--json` (or
`CASS_OUTPUT_FORMAT=json`, via `robot_format_from_env` at `src/lib.rs:65264`).
The plain-text path at `src/lib.rs:65403` onward never enters it.

The prompt asked me to quote the fast-path guard the non-json side uses and say
why `--json` does not take it. **It does take it, and it is not the relevant
guard.** The guard that prints `"Counts skipped for fast status on large
database"` (`src/lib.rs:65467`) is:

```rust
let include_counts = include_counts_override.unwrap_or_else(|| {
    db_size_bytes
        .map(|size| size <= STATUS_COUNT_SCAN_MAX_DB_BYTES)
        .unwrap_or(false)
});
```

`src/lib.rs:16136-16140`, against `const STATUS_COUNT_SCAN_MAX_DB_BYTES: u64 =
256 * 1024 * 1024;` at `src/lib.rs:15065`. It sets
`StateDbSnapshot::counts_skipped` at `src/lib.rs:15404`.

That guard runs inside `state_meta_json_for_status`, which `run_status` calls at
`src/lib.rs:65050` — **before** the `--json` branch. Both paths take it
identically, and the JSON payload publishes the result as `"counts_skipped"` at
`src/lib.rs:65381`. So the non-json side is not fast because it holds a guard the
JSON side skipped. It is fast because the JSON branch does **extra work the plain
path never does at all**.

This matters for the recommendation: that guard measures the **database**, and
the hang's cost is a function of the **raw mirror**. The two are only
accidentally correlated. The code says so itself at `src/lib.rs:65298-65301` —
"a pruned and reindexed archive is exactly the shape that breaks the proxy."

### 2. The walk: a doctor verification path, reused deliberately, whose only bound was deleted by accident

Call chain, all under the `--json` branch:

`run_status` `src/lib.rs:65305`
→ `collect_doctor_coverage_risk_summary` `src/lib.rs:36636`
→ `collect_doctor_raw_mirror_report` `src/lib.rs:36651` (and
  `collect_doctor_raw_mirror_backfill_report` `src/lib.rs:36652`)
→ `collect_doctor_raw_mirror_report_with_threshold` `src/lib.rs:34116`

The walk itself is `src/lib.rs:34163-34168`:

```rust
let manifest_root = root.join("manifests");
if manifest_root.exists() {
    for entry in walkdir::WalkDir::new(&manifest_root)
        .follow_links(false)
        .into_iter()
```

Each manifest is read and parsed (`src/lib.rs:34182-34184`) and handed to
`doctor_verify_raw_mirror_manifest` (`src/lib.rs:33826`), which calls
`doctor_file_blake3(&blob_path)` at `src/lib.rs:33935`. That helper
(`src/lib.rs:33713`) streams the **entire blob** through a BLAKE3 hasher in 64 KB
chunks.

This accounts for every symptom the bead measured, with nothing left over:
CPU-bound with `STAT=R` (BLAKE3 over 20 GB), no sqlite/`.db`/`-wal` fd (the DB is
not touched in this block), one read fd alternating between `.raw` blobs and
`doctor-raw-mirror-manifest-id-v1-*.json` manifests (manifest read, then blob
hash, then next manifest), and monotonic RSS (`report.manifests.push(...)` at
`src/lib.rs:34196` accumulates one struct per manifest, 125,607 of them, never
freed until the report is dropped).

**Answering the bead's own open question 1 — flag, cache, or removal?** None of
the three. The manifest prefix is a true clue: this is `cass doctor`'s
verification machinery, every function in the chain is named `doctor_*`, and the
prefix is written by `src/raw_mirror.rs:1497` and `src/lib.rs:33642`. But the
reuse is **deliberate and was originally bounded** — status wants a cheap
coverage read on a small archive and correctly declines it on a large one. The
correct shape was a size gate, which is what `447d97fe` restored:

```rust
let status_collects_coverage = db_exists && !status_raw_mirror_scan_too_large(&data_dir);
```

`src/lib.rs:65302`, with the else branch at `src/lib.rs:65310-65314` returning
`doctor_fast_coverage_risk_unchecked` and the honest source label
`"status-fast-state"`.

### 3. Bounds — what is bounded now, and what is still not

`status_raw_mirror_scan_too_large` (`src/lib.rs:34082`) is a single
`std::fs::read_dir` with an early exit at `STATUS_COVERAGE_MAX_RAW_MIRROR_MANIFESTS`
(`src/lib.rs:34104`), the constant being `512` at `src/lib.rs:15079`. It fails
**safe**: an unreadable directory returns `true` (too large) at `src/lib.rs:34088`
and `34094`, and an absent mirror returns `false` at `src/lib.rs:34085`.

The walk **itself remains completely unbounded**: `collect_doctor_raw_mirror_report_with_threshold`
has no limit, no deadline, no cache, and no size guard. Its own doc comment says
so at `src/lib.rs:34070-34072` — "reads every manifest and hashes every blob with
no limit and no deadline." The one threshold in that function,
`doctor_raw_mirror_size_warn_threshold_bytes` (`src/lib.rs:34049`), only appends a
**warning string** (`src/lib.rs:34056-34061`); it stops nothing.

So the bound is at the call site, not in the walk. Two consequences:

- **Stated ceiling, already in the code** (`src/lib.rs:15076-15078`): the cap
  counts manifests, not bytes, so a mirror of a few very large blobs passes the
  gate and still pays to hash them.
- **Unstated, and mine**: the gate uses a flat `read_dir` while the walk uses a
  recursive `WalkDir`. Today that is safe, because
  `doctor_raw_mirror_manifest_relative_path` writes `manifests/{manifest_id}.json`
  flat (`src/lib.rs:33621-33623`), and the gate's comment names that assumption
  (`src/lib.rs:34074-34076`). But the two traversals are different shapes over the
  same tree, so if manifests are ever nested the gate counts zero `.json` files,
  returns "not too large," and the recursive walk proceeds over all of them. Low
  severity, latent, cheap to close by giving the gate the same `WalkDir` with the
  early exit. I am flagging it, not recommending it be done now.

### 4. The gate is complete for `status --json` — established in reverse

Every production caller of the walking function (`rg` over `src/lib.rs`,
test-module callers excluded by mapping each hit to its enclosing `fn`):

| caller | line | reachable from `run_status --json`? |
|---|---|---|
| `capture_doctor_forensic_bundle` | 29140 | no — its callers are `collect_doctor_raw_mirror_backfill_report` (36150, already inside the gate), `apply_diag_quarantine_cleanup` (51747, a mutation path), and `run_doctor_impl` (68632) |
| `build_doctor_archive_scan_context` | 34497 | no — called only at 35301 and 35372, both doctor-archive commands |
| `collect_doctor_coverage_risk_summary` | 36651 | **yes — and this is the one `447d97fe` gates** |
| `build_doctor_baseline_snapshot` | 44452 | no — called at 43660, 43874, 44141, 44286, all doctor baseline/restore |
| `run_doctor_impl` | 69163, 69187 | no — that is `cass doctor`, a separate command |

The three callees that run **ungated** inside the `--json` branch were each
checked for expensive traversal:

- `collect_diag_quarantine_report` (`src/lib.rs:65273`, defined 51063) — two
  `std::fs::read_dir` calls at `src/lib.rs:51091` and `51146`, over the index
  backups and retained-publish directories. Bounded by those small directories,
  not by the mirror.
- `build_doctor_runtime_summary` (`src/lib.rs:65339`, defined 36776) — no
  filesystem walk; it reads an already-collected report and writes
  `raw_mirror_manifest_count: Null` on the fast path (`src/lib.rs:36860`).
- `readiness_recommended_commands` (`src/lib.rs:65355`, defined 17037) — no
  `WalkDir`, `read_dir`, `blake3`, or `raw_mirror` reference in its body.

**Conclusion: the only raw-mirror walk `cass status --json` can reach is the one
`447d97fe` gated.** I did not find a second uncovered path.

### 5. The detectors exist and are not vacuous — but they do not prove the command is fast

`tests/cli_doctor.rs:4589` (`status_json_declines_inline_coverage_on_a_raw_mirror_past_the_scan_cap`)
builds 513 real manifests with real blobs (`tests/cli_doctor.rs:4598-4611`),
asserts the fixture actually exceeds the cap (`tests/cli_doctor.rs:4618-4621`),
and asserts status reports `"status-fast-state"` / `"not_checked"`
(`tests/cli_doctor.rs:4639-4653`). Its paired under-cap case is at
`tests/cli_doctor.rs:4666`, which is what stops "always take the fast path" from
satisfying the pair.

State this precisely, because it is the kind of claim that gets overstated: these
assert **which branch was taken and that the output says so honestly**. They do
not assert a wall-clock bound. A second, ungated walk elsewhere would leave both
green. That is exactly why finding 4 above was done by enumeration rather than by
citing the tests.

### 6. The residual, and it is the one that still bites an operator today

`status`'s own fast path routes the operator to the surface that does verify:
`doctor_fast_coverage_risk_unchecked` recommends `"Run 'cass doctor --json' for
source coverage and sole-copy analysis."` (`src/lib.rs:36673`), and
`run_status` recommends `"Run 'cass doctor check --json' before any repair..."`
(`src/lib.rs:65237`). The over-cap test at `tests/cli_doctor.rs:4654-4659`
asserts that routing.

`cass doctor` walks the same store **unconditionally**: `run_doctor_impl` calls
`collect_doctor_raw_mirror_report` at `src/lib.rs:69163`, and again at
`src/lib.rs:69187` when backfill applied. No gate on either.

So after deploy, `cass status --json` returns and honestly says coverage was not
checked — and then points at a command that does not return on this archive.
The bead's own second comment names this and deliberately scoped it out. I agree
with that scoping and I am not recommending it be folded in: its comment also
records the reason a naive limit is wrong there (a truncated doctor scan reports a
**false coverage gap** and recommends a **mutation**, because
`build_doctor_coverage_summary` cannot tell unverified from unscanned). "Unscanned"
has to become its own state first. That is a separate bead's worth of work.

### 7. The stale worktree contains unmerged work — and it is not nvq59 work

`.claude/worktrees/cass-nvq59-status-hang` (mtime 2026-08-14T22:23:48Z on
`src/lib.rs`). `git -C` into it was refused by the harness as documented, so this
is by plain file read and checksum.

Its committed work **is fully merged** — `447d97fe` is an ancestor of `HEAD`, and
the gate is present there at the same constant and call site
(`.../cass-nvq59-status-hang/src/lib.rs:15079`, `:33791`, `:65014`).

But its working tree is **dirty with a different fix that never landed**:

| file | sha256 |
|---|---|
| `447d97fe:src/storage/sqlite.rs` | `c2bd2ec1…` |
| this worktree's `src/storage/sqlite.rs` | `c2bd2ec1…` (identical — nothing landed since) |
| stale worktree's `src/storage/sqlite.rs` | `8f42a60a…` (**differs**) |

What that difference contains:

- `pub fn connector_scan_floors_select_sql()` — new, at
  `.../cass-nvq59-status-hang/src/storage/sqlite.rs:92`. It inlines the meta key
  into the SQL instead of binding it, carrying a measured table: inlined key
  answers in 1–2 ms, `?1` and anonymous `?` **did not return in 45 s**, against
  the live 7.38 GiB archive, one process per corner.
- Two call sites converted to it: `read_connector_scan_floors` in `src/lib.rs`,
  and `get_connector_scan_floors` at that file's `:7033`.
- A non-vacuous shape test at that file's `:15360`
  (`connector_scan_floors_sql_inlines_the_key_and_binds_nothing`).
- `tests/probe_live_state_db.rs`, 9.3 KB, **exists only there** — absent from
  this tree.

None of it is on `main`. This tree still binds the key at `src/lib.rs:15114` and
`src/lib.rs:15855`.

I am flagging this rather than acting on it, for two reasons. It is outside my
lane and outside my write permission. And a sibling lane is editing
`probe_state_db` in this very worktree right now for `-nao4q` — the same symptom
family — so whoever owns that lane should see this before choosing a remedy, in
case the two are the same defect approached from opposite ends. I have not
verified that they are; see the proof boundary.

## Recommendation

**Smallest correct fix for nvq59: no source change. It is already written.** The
remaining steps are the ones the bead's second comment names — release build,
atomic deploy (temp name inside `~/.local/bin` then `mv -f`, never `cp` over the
live path), preserve a dated specimen, and time `cass status --json` on the live
archive against the 900 s no-return baseline. Then close.

**The one alternative I reject is the one this lane was told to prefer:** reusing
the existing fast-path guard, i.e. gating the walk on `counts_skipped` /
`STATUS_COUNT_SCAN_MAX_DB_BYTES` (`src/lib.rs:15065`). Preferring an existing
guard over a new one is the right instinct and it is wrong here, because that
guard measures the wrong object. It reads `db_size_bytes`
(`src/lib.rs:16137-16139`) while the cost is `manifest_count × mean_blob_bytes`.
The two decouple exactly in the failure shape this archive has: prune the mirror
or reindex to a smaller database and the DB guard passes while the mirror walk
still runs for fifteen minutes. That is also the historical bug — `b8e3e78b`
collapsed two policies into one boolean because they *looked* like the same
question. Re-coupling them would rebuild the defect with a different constant.
`STATUS_COVERAGE_MAX_RAW_MIRROR_MANIFESTS` is a new constant, and that is
correct: it is a new constant measuring the thing that actually costs.

Secondary, and genuinely optional: close the flat-`read_dir`-vs-recursive-`WalkDir`
mismatch in finding 3 by giving the gate the walk's own traversal. It is currently
latent, not a live defect.

## Proof boundary — what I did NOT establish

1. **I did not run anything.** No build, no test, no `cass`. Every claim here is
   source reading. That `447d97fe` makes the command return is a claim I inherit
   from the bead's comment and the code's own logic; I have not measured it, and
   the bead itself says the fix is **not deployed**.
2. **I did not establish that the deployed binary contains the fix.** I was
   forbidden to touch `~/.local/bin/cass*`. The bead records the live binary as
   pre-fix (sha256 `3d044227…`). If that is still true, the four measured
   non-returns say nothing about the current source, and anyone re-measuring must
   print the binary's sha256 in the same output as the timing.
3. **"The gate is complete" is a reachability argument, not an execution trace.**
   I enumerated callers of `collect_doctor_raw_mirror_report` and checked three
   ungated callees one level deep for `WalkDir`/`read_dir`/`blake3`/`raw_mirror`.
   A walk reached at depth ≥2 through a name matching none of those patterns
   would not have been caught. What would settle it: run `cass status --json`
   against a fixture mirror past the cap under `fs_usage` or with an
   instrumented counter, and confirm zero blob reads.
4. **I did not measure the cap.** The 512 figure and its "0.83 ms per manifest"
   justification are the bead's and the code comment's
   (`src/lib.rs:15071-15074`), taken on a machine and archive I did not touch.
5. **I did not establish whether the stale worktree's unmerged
   `connector_scan_floors_select_sql` work is correct, still needed, or the same
   defect the `-nao4q` lane is fixing.** I established only that it exists, is
   uncommitted, is absent from this tree, and carries its own measurements and a
   test. Whether inlining is the right remedy — versus fixing the engine, which
   its own doc comment says it is a workaround for — is not a question I looked
   at. What would settle it: the `-nao4q` lane comparing its remedy against that
   diff, and someone deciding whether the frankensqlite bound-parameter defect is
   filed anywhere (`-p3kgr` is adjacent but is about a different statement).
6. **I did not check for uncommitted files in the stale worktree beyond the two I
   compared.** `git -C` was refused, so I checksummed `src/lib.rs` and
   `src/storage/sqlite.rs` and checked one test path. There may be more. What
   would settle it: a non-worktree-isolated session running `git -C
   .claude/worktrees/cass-nvq59-status-hang status --short`.
7. **`src/lib.rs` was being edited while I read it.** All numbers were re-derived
   after the shift, but the file may have moved again since. The offset is stated
   at the top; re-derive if anything does not match.
8. **The bead's open question 2 — does `status --json` ever complete on a 20 GB
   mirror — is still unanswered and I could not answer it by reading.** It is now
   moot for the gated path, since the gate declines the scan rather than
   completing it.
