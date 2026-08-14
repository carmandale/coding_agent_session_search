# cass repair to green — generation 2 (continuation) log

**Date:** 2026-08-14
**Session:** background job `8a00f9f4`, launched from the generation-1 continuation prompt
(`continuation-prompt.md`, committed at `2e931329`)
**Parent:** `a91c2501-1830-4d3d-9430-3c9afe08a63c` — still live while this ran; it owns
`agent-log.md` and this file is deliberately separate so neither session writes the other's log.
**Goal (Dale, verbatim, inherited):** *"/my-way fix cass to completion and 100% green working
state and completely up to date or tell me why it can't or /grill-me with any questions."*
**Mid-work instruction (Dale, verbatim, inherited):** *"make sure that you are looking at the
recent (last 2 weeks) work on cass and not regressing"*

Every claim below is **MEASURED** (a command I ran, whose output I read) or **INFERRED**.

---

## 0. Isolation — why this generation worked in a worktree, against §2.10

The global AGENTS.md §2.10 says work on `main` and never create a worktree unsolicited, and the
continuation prompt repeats it. This session could not comply: it runs as a background job whose
harness **rejects every file edit in the shared checkout** until the session isolates, and the
documented opt-out is a `.claude/settings.json` edit — itself blocked, and not a repo safety
setting an agent should flip on its own behalf.

Isolation turned out to be correct on the merits anyway, not merely forced. **MEASURED:** `HEAD`
moved from `eafae1e0` to `770c1d8b` between this session's first and second commands — the parent
session was committing into the same checkout while this one worked. That is precisely the
collision §4.2 exists to prevent.

Worktree: `.claude/worktrees/cass-nvq59-status-hang`, branch `worktree-cass-nvq59-status-hang`,
based on `770c1d8b` (== `origin/main`, **MEASURED** `git rev-list --left-right --count` → `0 0`).
It exists to hold one commit and land it on `main`; `main` remains the destination.

---

## 1. What shipped: the `cass status --json` hang (bead nvq59)

Implemented `lanes/raw-mirror-walk.md` §6a, which was the continuation prompt's exact next action.

### The defect, restated from the grounding lanes

`cass status --json` calls `collect_doctor_coverage_risk_summary`, which walks all 125,607
raw-mirror manifests and BLAKE3-hashes all 125,601 blobs — 19.68 GiB — with no limit, no budget,
no cache and no gate. Plain `cass status` escapes only because the call sits inside the
structured-output branch. The gate that prevented it existed and was **deleted by accident** on
2026-05-28 by `b8e3e78b`, a four-minute build repair for `fe3972dc`; the in-source comment at
`src/lib.rs:64931-64934` recorded that deletion as deliberate policy, which is false history.

### The change

Three edits, one file plus one test file.

1. **`src/lib.rs` — new constant `STATUS_COVERAGE_MAX_RAW_MIRROR_MANIFESTS: usize = 512`**, placed
   beside `STATUS_COUNT_SCAN_MAX_DB_BYTES` where the other status gate constant already lives.
2. **`src/lib.rs` — new `status_raw_mirror_scan_too_large(data_dir) -> bool`**, placed directly
   above the walk it gates. A single `read_dir` over `raw-mirror/v1/manifests` counting `.json`
   entries with an early exit at the cap, so cost is O(cap) not O(store). Written on the existing
   in-file precedent `doctor_remote_mirror_top_level_entry_count` (`src/lib.rs:32095`).
3. **`src/lib.rs:64935` — the gate itself**, plus a corrected history comment:
   ```rust
   let status_collects_coverage = db_exists && !status_raw_mirror_scan_too_large(&data_dir);
   ```

No new mechanism: the else-branch, the `"status-fast-state"` label, the `coverage_checked: false`
rendering, the operator routing text and a golden file were all already written and unreachable.

### Why gate on the mirror rather than restore the old DB-size predicate

The walk's cost is a function of manifest count and blob bytes; database size correlates with
those only by accident. A pruned-and-reindexed archive — small DB, large mirror — is exactly the
shape that breaks the old proxy.

### Why 512, and what the number does not cover

**MEASURED 2026-08-14** on the live mirror, 300 randomly sampled manifests read together with
each referenced blob in full: **0.25 s for 300, i.e. 0.83 ms per manifest warm**, mean blob
52.3 KB, 0 missing. So the cap holds the inline collector near 0.4 s on a mirror of this shape,
against a whole walk that did not return inside a 900 s bound.

**Ceiling, stated in the code as a `// ceiling:` comment rather than left implicit:** the gate
counts manifests, not blob bytes. A mirror holding a few very large blobs stays under the cap and
still pays to hash them. The live store has one 812 MiB blob, so this is not hypothetical — it is
simply not the shape that produced this hang, and bounding bytes as well would cost a second
mechanism for a store nobody has seen.

### Gates — all MEASURED, in the isolated worktree, `CARGO_TARGET_DIR=/tmp/cass-nvq59-target`

The repo's pinned nightly had to be put on `PATH` first; the continuation prompt's warning is
accurate and the failure is silent otherwise.

| gate | result |
|---|---|
| `cargo fmt -- --check` | clean (after one rustfmt reflow of the gate line) |
| `cargo check --all-targets` | **rc=0**, 115 s |
| `cargo clippy --all-targets -- -D warnings` | **rc=0**, 66 s |
| `cargo test --test cli_doctor` (3 named cases) | **3 passed, 0 failed**, 3.80 s |

`CHECK_RC=0` was not taken on faith — 115 s is fast enough to be worth doubting. **MEASURED:**
`/tmp/cass-nvq59-target/debug/deps/` holds `libcoding_agent_search-*.rmeta` and
`libcli_doctor-*.rmeta`, so the lib and the test target really were type-checked with these
changes rather than skipped.

### The pre-existing test the lane flagged as at-risk stayed green, unchanged

`tests/cli_doctor.rs:4481-4514` (`doctor_json_reports_missing_upstream_source_as_coverage_risk_not_data_loss`)
asserts `coverage_source.source == "status-inline-small-archive"` and `status == "checked"`. It
was run by name and passed. Its fixture uses `seed_healthy_empty_index` and writes no raw-mirror
manifests, so the manifest count is 0, the gate does not fire, and the inline branch is preserved.
**No pin was re-armed and no assertion was edited.**

---

## 2. The detector, and the mutant that proves it is not vacuous

Before this change, **nothing in the suite could tell the gate's presence from its absence** —
that is how `b8e3e78b` deleted it in 2026-05 and shipped green for two and a half months. Fixing
the hang without fixing that would have left the same hole open.

Two new cases in `tests/cli_doctor.rs`:

- `status_json_declines_inline_coverage_on_a_raw_mirror_past_the_scan_cap` — builds a mirror of
  **513** real manifests with real blobs (one past the cap), asserts `cass status --json` reports
  `coverage_source.source == "status-fast-state"`, `status == "not_checked"`,
  `archive_coverage_state == "not_checked"`, and routes the operator to `cass doctor`.
  It also asserts the fixture actually exceeded the cap, so a fixture that silently stopped
  building manifests cannot pass it.
- `status_json_still_verifies_coverage_inline_on_a_raw_mirror_under_the_scan_cap` — 4 manifests,
  asserts the inline branch is *still taken*. Without this, "always take the fast path" would
  satisfy the first case and status would silently stop reporting coverage for everyone.

**The mutant — MEASURED, and it is the only honest check.** The mutation restores the deleted
gate exactly (`let status_collects_coverage = db_exists;`), applied by an exact-string replace
that asserts it matched exactly once:

```
mutant applied
test status_json_still_verifies_coverage_inline_on_a_raw_mirror_under_the_scan_cap ... ok
test status_json_declines_inline_coverage_on_a_raw_mirror_past_the_scan_cap ... FAILED
  panicked at tests/cli_doctor.rs:4639:5:
  assertion `left == right` failed: status must take the fast path on a mirror past the scan cap
test result: FAILED. 1 passed; 1 failed        MUTANT_TEST_RC=101

  ... file restored from a pre-mutation copy ...

test status_json_still_verifies_coverage_inline_on_a_raw_mirror_under_the_scan_cap ... ok
test status_json_declines_inline_coverage_on_a_raw_mirror_past_the_scan_cap ... ok
test result: ok. 2 passed; 0 failed            RESTORED_TEST_RC=0
```

The over-cap case goes red, the under-cap case stays green, the failure message is the one the
case is named after, and both go green again on restore.

**One instrument in that script printed nothing, and the reason is worth recording rather than
shrugging off.** The script also ran `git diff -- src/lib.rs | rg '^[+-]        let
status_collects_coverage'` to show the mutation's landing site, and it matched zero lines. That is
not a dead probe — it is correct and expected: the mutant restores the *pre-fix* line, which is
byte-identical to `HEAD`, so the mutated line does not appear in a diff against `HEAD` at all. The
load-bearing evidence is the red-then-green transition, which the exact-string replace (asserting
exactly one match before writing) already pins to the intended site.

---

## 2a. Proportional test sweep, and one discovered red suite that is not mine

Scope: every non-e2e suite that drives `cass status` or the robot JSON envelope, plus the lib
unit suite. **`tests/e2e_*.rs` was deliberately NOT run** — per `lanes/test-integrity.md` and the
parent session's measured warning it spawns 8 concurrent `cass index --full` runs over the
operator's real `~/.codex` and `~/.claude` trees and has been measured not finishing in 90
minutes. That is a stated omission, not a pass.

| target | rc | result |
|---|---|---|
| `--lib` | 0 | **5124 passed, 0 failed**, 3 ignored (identical to the recorded baseline) |
| `cli_doctor` | 0 | 51 passed, 0 failed (49 pre-existing + the 2 new cases) |
| `cli_status` | 0 | 6 passed, 0 failed |
| `cli_robot` | 0 | 297 passed, 0 failed |
| `golden_readiness` | 0 | 4 passed, 0 failed |
| `spec_status_envelope_completeness` | 0 | 3 passed, 0 failed |
| `cass_257_status_quality_tier_aware` | 0 | 1 passed, 0 failed |
| `golden_robot_json` | **101** | **28 passed, 9 FAILED** |

### The 9 golden failures: REPRODUCED, pre-existing, attributable to `e3ed01f0`

Not asserted — established, and by evidence independent of this change.

**MEASURED, the drift itself.** The failing goldens include `health`, `diag` and `stats`, which
this change provably cannot reach: the new gate lives inside `run_status`'s structured-output
branch, while `run_health` takes the fast path unconditionally and `diag`/`stats` never consult
it. Diffing `health.json.golden` against the `.actual` the run produced shows the entire drift is
one added block, twice:

```
> "connector_coverage": { "checked": false, "complete": null,
>                         "incomplete_connectors": [], "floors": [] }
```

That is exactly the block `e3ed01f0` added to the health/status/stats JSON surfaces. **No
`coverage_source` change appears in any golden diff**, which is the key this change would have
moved if it had moved anything.

**MEASURED, the chronology.** `tests/golden/robot/health.json.golden` was last touched by
`fb75daab` (2026-05-28) and `status_quarantine.json.golden` by `b4463276` (2026-06-01).
`e3ed01f0` is 2026-08-10 and `git show --stat e3ed01f0 -- tests/golden/` is **empty** — it added
the JSON block and regenerated no golden. So `golden_robot_json` has been red on `main` since
2026-08-10, four days before this session started. This is the same shape as `193d2ad6`, the
commit that existed only to repair the clippy/rustfmt `e3ed01f0` also skipped.

**A second, separate component of the same drift is environmental, and it is why these goldens
must NOT be regenerated on this Mac.** The status goldens also differ in host-topology fields —
`"source": "linux_sysfs"` against `"fallback"`, and `memory_total_bytes` / `memory_available_bytes`
as `[LIVE_BYTES]` against `null`. The goldens were generated on a Linux host and CI runs Linux.
Running the `UPDATE_GOLDENS=1` regeneration the failure message suggests, **on macOS**, would bake
macOS topology into goldens that CI compares on Linux — turning one pre-existing red into a
permanent CI red. Regeneration belongs on a Linux runner, or the topology block needs the same
`[LIVE_*]` normalization the memory fields already have.

Filed rather than fixed here: out of scope for the nvq59 gate, and the fix is not the one the
tool's own error message recommends.

---

## 2b. UBS base-vs-current — and a correction to my own first measurement

**First attempt was invalid and is recorded as such.** It compared a scan run *inside* the project
against a scan run in a bare temp dir — two different scan workspaces, so the two sides were not
comparable. That is the differential-specimen trap in `instrument-labels.md`: the control has to
be the same kind of thing as the subject.

**Re-run like-for-like** — two bare temp dirs holding only the two changed files, identical `ubs`
invocation, with each specimen's sha256 printed so the comparison can be re-read later:

```
src/lib.rs           base=31896eb387cd9fc4  cur=29d8f02a0edbf9e2
tests/cli_doctor.rs  base=17b05945ef727249  cur=5780b489ec895513

  critical     22 ->     22   delta  +0
  warning    7998 ->   8031   delta +33
  info       5961 ->   5969   delta  +8
```

**Critical is flat. Warning is up 33 and info up 8 across 217 added lines** — a density of about
0.15 findings/line against the baseline file's ~0.11, i.e. the same order, in a file that already
carries 7,998 warnings.

**What I could not get, stated as a boundary rather than a pass:** the per-finding breakdown. Both
`--format=jsonl` and `--beads-jsonl=` returned **only summary rows** from this ubs version's rust
scanner (2 lines, keys `critical/warning/info/files`), so a first pass that reported "0 net-new
findings" was a dead instrument counting summaries, not findings — it is discarded, not reported.
**So the 33 new warnings are counted but not named.** Someone with a working per-finding path
should name them before treating this gate as fully discharged.

- **`cass doctor` still hangs.** `run_doctor_impl` calls the same walk unconditionally
  (`src/lib.rs:68744`, and a second time at `68768` when backfill applies), with no surface, mode,
  or `--fix` gate. `cass doctor --check` is documented as *"the bounded read-only doctor truth
  surface"* and is not bounded. That is `lanes/raw-mirror-walk.md` §6b and it was deliberately not
  attempted here: §6c shows a naive limit would make a truncated scan report a false coverage gap
  and route the operator at a **mutation** (`cass doctor --fix`), because
  `build_doctor_coverage_summary` cannot distinguish *unverified* from *unscanned*. `unscanned`
  has to become its own state before any limit ships.
- **Consequence worth naming:** status's fast path tells the operator to *"Run cass doctor check
  --json for current archive coverage"* (`src/lib.rs:36681`) — a command that does not return on
  this archive. This is **not introduced by this change**: `cass health` already takes that same
  fast path unconditionally and already emits that same routing, so the gap predates the fix and
  is now simply reachable from one more surface.
- **The coverage-floor fix (`e3ed01f0`) is still not deployed**, and this change does not deploy
  it. The two hangs are independent — nvq59 was filed 4h26m before `e3ed01f0` was committed.

---

## 4. Corrections and additions to the inherited facts

- **The parent's `agent-log.md` and `lanes/raw-mirror-walk.md` §6a were followed as written and
  found correct.** The predicted test impact ("the tiny fixture stays below the cap and the test
  stays green unchanged") held exactly, verified by running that test by name.
- **`doctor_raw_mirror_manifest_relative_path` (`src/lib.rs:33327`) writes
  `manifests/<manifest-id>.json`** — the manifests directory is flat *by construction*, not merely
  flat in practice. This is what makes a single-level `read_dir` a complete count rather than an
  undercount, and it is why the gate does not need `walkdir`. `src/raw_mirror.rs:99`'s own cheap
  collector already relies on the same fact.
- **Per-manifest cost is 0.83 ms warm, not the ~7 ms a naive `900 s / 125,601` would give.** That
  division would have been an instrument-label error: the 900 s run never finished, so it covered
  an unknown prefix of the store, not all of it.

---

## 5. Blockers for the rest of the goal — the "or tell me why it can't" branch

**Disk (blocks the backfill).** **MEASURED**, this session: `/System/Volumes/Data` free went
149 GiB → 145 GiB over about an hour (some of that is this session's own 5.9 GB cargo target dir,
which is reclaimable scratch). The floor is 150 GB and disk-janitor's latest run
(`~/Library/Logs/disk-janitor/report-floor-recover-2026-08-14-161601-90732.md`) reports
`FLOOR UNMET (floor 150GB). Run status PARTIAL.` The backfill is estimated at 24-30 GB.

The janitor's own report names where the space is, and this is the actionable part:

```
   148.6GB  ~/Library/Application Support/com.groovetech.media-server
    42.8GB  ~/Library/Developer/Xcode/DerivedData
    13.1GB  ~/.Trash
    12.4GB  ~/Library/Developer/Xcode/iOS DeviceSupport
    11.3GB  ~/Library/Developer/Xcode/visionOS DeviceSupport
```

So roughly **70 GB sits in regenerable Xcode caches plus the Trash** — clearing those would put
the machine well above the floor with room for the backfill. **No agent is authorized to delete
any of it**, and the continuation prompt forbids freeing space by deletion without explicit
permission. This is a decision for Dale, not a blocker anyone can engineer around.

**Usage.** **MEASURED** via `cusage`: this session's account (`erika`) is at 6% of its 5-hour
window and 22% weekly — ample. Four other accounts are at 87-100% weekly, which is why no fan-out
was run here.

---

## 6. Hazard inherited mid-session from the parent, and honored

The parent session sent a cross-session message with a measured finding the handoff did not carry:
**of 4,050 indexed `claude_code` conversations, 3,877 (95.7%) have no surviving source file on
disk** — Claude Code rotates transcripts off disk, so for those the cass archive is the only copy
in existence. Its consequences were treated as binding for the rest of this session:

- No `cass index --full`, `--force-rebuild`, or `cass doctor --fix` against the live data dir.
- No `tests/e2e_*.rs` — that suite spawns 8 concurrent `cass index --full` runs over the
  operator's real `~/.codex` / `~/.claude` trees and has been measured not finishing in 90 min.
- Never `cass sources agents exclude claude_code` — `purge_agent_archive_data`
  (`src/storage/sqlite.rs:7121-7190`) deletes every conversation for an agent and is reachable
  from that path.

Every cass invocation from this session was read-only (`cass sources list`, and the timed
`status`/`health`/`triage` reads). No index, doctor, or mutation command was run.

---

## 7. Commands that matter, for whoever follows

```bash
# The toolchain selection is load-bearing; without it Homebrew's stable 1.96 silently
# shadows the rust-toolchain.toml nightly pin and fails at fsqlite-pager with E0554.
export PATH="$HOME/.rustup/toolchains/nightly-aarch64-apple-darwin/bin:$PATH"

# Never share a CARGO_TARGET_DIR between checkouts of this crate (repo napkin: it
# silently runs the WRONG binary while cargo prints "Finished in 0.41s").
export CARGO_TARGET_DIR=/tmp/cass-nvq59-target   # this worktree only
```
