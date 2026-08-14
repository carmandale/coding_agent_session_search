---
generation: 1
parent-session: a91c2501-1830-4d3d-9430-3c9afe08a63c
next-action-class: executable
---

# Continuation — fix cass to green and current

## The goal and authorization, verbatim (Dale, 2026-08-14)

> /my-way fix cass to completion and 100% green working state and completely up to date or tell me why it can't or /grill-me with any questions.

And, sent mid-work the same day, as a correction to the work in flight:

> make sure that you are looking at the recent (last 2 weeks) work on cass and not regressing

Nothing else was authorized. **Destructive and external-write approvals expired with the
parent session and do not transfer.** In particular you do NOT have approval to: delete any
file (this repo's AGENTS.md RULE 1 forbids it outright, including files you create), run the
12–15 hour backfill, run `cass doctor --fix`, force-push, or touch the shared stash stack.

## Where the work is

- **Repo:** `/Users/dalecarman/dev/coding_agent_session_search` (cass), branch `main`,
  at `4d82c377` when this was written. Nothing unpushed.
- **Evidence, all committed:** `thoughts/shared/handoffs/20260814-cass-repair-to-green/`
  — `agent-log.md` (coordinator) and `lanes/*.md` (seven grounding lanes, ~200 KB).
  **Read `agent-log.md` first, then `lanes/raw-mirror-walk.md` §6a.**
- **Task list:** eight tasks exist in the parent session's task tool. Task 1 (grounding) is
  done. Tasks 2–8 are pending. Re-create them if your harness lost them; they are listed
  under "What remains" below.

## Read this before touching anything

Two environment facts that will otherwise cost you an hour each:

1. **The build needs nightly, and calling nightly cargo by absolute path is NOT enough.**
   `rust-toolchain.toml` pins nightly; `rustup` is not on PATH; Homebrew's stable rust 1.96
   shadows it and ignores the pin. A rustup toolchain's cargo invoked directly still resolves
   `rustc` from PATH, so it fails identically with `E0554` on `fsqlite-pager`. Put the bin dir
   first:
   ```bash
   export PATH="$HOME/.rustup/toolchains/nightly-aarch64-apple-darwin/bin:$PATH"
   CARGO_TARGET_DIR=/tmp/cass-repair-target cargo check --all-targets
   ```
   Baseline measured green: exit 0 in 2m31s. The target dir is warm. `rch`, which AGENTS.md
   wraps every cargo command in, is not installed on this Mac.
2. **Never share a `CARGO_TARGET_DIR` between checkouts of this crate** — the repo napkin
   records it silently running the WRONG binary, with cargo printing `Finished in 0.41s`.

Repo rules that constrain the fix: no `rusqlite` in new code, ever, and explicitly *"do not
add rusqlite just to read an existing SQLite file"* — frankensqlite only. Work on `main`, and
after pushing `main` also `git push origin main:master`. Gates: `cargo check --all-targets`,
`cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, and
`ubs --format=json --ci $(git diff --name-only origin/main...HEAD)` (base-vs-current; only
*increases* block).

## What was established (do not re-derive; correct only with source evidence)

- **The `status --json` / `doctor` hang** is `collect_doctor_raw_mirror_report` BLAKE3-hashing
  125,601 blobs / 19.68 GiB on every invocation — no budget, no cache, no gate. Plain
  `cass status` escapes only because the call sits inside the structured-output branch. The
  gate existed and was **deleted by accident** on 2026-05-28 by `b8e3e78b`, a four-minute
  build fix for `fe3972dc`. The comment at `src/lib.rs:64931-64934` records that deletion as
  deliberate policy and is false history — fix it in the same change.
- **This hang predates the coverage-floor fix by 4h26m** (nvq59 filed 13:13, e3ed01f0
  committed 17:39) and is a separate defect. Do not let either fix be credited or blamed for
  the other.
- **The coverage read has three call sites**, not one: `src/lib.rs:65457` (health, via
  `read_connector_scan_floors_bounded`), `src/lib.rs:15283` (`probe_state_db` → serves
  `cass status`, `cass triage`, and `search --robot-meta`), `src/lib.rs:23747` (`run_stats`).
  Repairing only the bounded reader fixes health and leaves the rest hanging.
- **`read_connector_scan_floors` (src/lib.rs:15077-15090) ends in `.unwrap_or_default()`**, so
  any query error becomes an empty map, which `connector_coverage_json` (src/lib.rs:15115)
  renders as `"complete": true` — the exact defect its own doc comment says it exists to
  prevent. The storage layer already has the correct reader:
  `FrankenStorage::get_connector_scan_floors` (`src/storage/sqlite.rs:6991-7002`) uses
  `.optional()` so an absent key is `Ok(None)`. **This is a single-source defect; fix by
  reusing the storage reader, not by patching the duplicate.**
- **Bounding the lifecycle is NOT the fix for the health/triage/stats regression.** The full
  frankensqlite open → validate → two meta reads → close costs **40 ms** on the live 7.93 GB
  archive (measured: `cass triage --json` returns in 0.04s with `opened:true`,
  `open_skipped:false`). By elimination the only new operation e3ed01f0 adds is the single
  zero-row `connector_scan_floors` query. **Its mechanism is NOT yet established** — the two
  obvious hypotheses are both refuted in `lanes/bound-lifecycle.md` (the retry loop does not
  spin, because "query returned no rows" matches none of the retryable substrings; and the
  `?1` binding takes the same engine path as a literal). This is the single biggest open
  question and it blocks redeploying the coverage fix.
- **The coverage floor is forward-looking only.** The live meta table holds exactly
  `last_indexed_at`, `last_scan_ts` (2026-07-16), `schema_version` — no floor rows, because
  the fix never ran. So on deploy, coverage reports `complete: true` over a ~13,300-session
  hole. **`connector_coverage.complete` is therefore not a valid acceptance signal for this
  goal** — final proof must count indexed sessions against the on-disk corpus.
- **Five regression risks have no detector**, and `connector_coverage` appears in **zero**
  test files. The latency test that should catch the hang is a false green (fixture too
  small). Bead gxw32's one-line mutant restoring the global watermark passes all 5,124 tests
  because the coverage fixture registers only one connector. **Write detectors before fixing**
  — the suite currently cannot distinguish fixed from broken.
- **Deployed binary** is `cass 0.6.9`, sha256 `3d044227…`, byte-identical to
  `~/.local/bin/cass.pre-coverage-floor-20260601`. The fix build is preserved at
  `~/.local/bin/cass.coverage-floor-fix-20260810` (`d0b860eb…`). Rollback was a binary swap,
  not a git revert; `e3ed01f0` is in `main`.

## The exact next action

Implement the `status --json` / `doctor` fix from `lanes/raw-mirror-walk.md` §6a. It is the
highest-value change available: it unblocks every agent using cass today, on a defect
independent of the still-unexplained coverage-query regression.

At `src/lib.rs:64935`, replace:

```rust
let status_collects_coverage = db_exists;
```

with a gate on **the store actually walked** (the raw mirror), not the old DB-size proxy:

```rust
let status_collects_coverage = db_exists && !status_raw_mirror_scan_too_large(&data_dir);
```

Write `status_raw_mirror_scan_too_large` on the existing in-file bounded-count precedent
`doctor_remote_mirror_top_level_entry_count` (`src/lib.rs:32095-32124`): a `read_dir` over
`<data_dir>/raw-mirror/v1/manifests` that stops counting at the cap, so cost is O(cap) not
O(store). Use `doctor_raw_mirror_root` (`src/lib.rs:33155`) to build the path. Gate on the
mirror rather than DB size because the walk's cost is a function of manifest count and blob
bytes; DB size only correlates by accident, and a pruned-and-reindexed archive is exactly the
shape that breaks the proxy. Also correct the false-history comment at 64931-64934.

Everything downstream already exists: the else-branch, the `"status-fast-state"` label, the
`coverage_checked: false` rendering, and a golden file. This deletes a hang without adding a
mechanism.

**Test impact, unverified — confirm it:** `tests/cli_doctor.rs:4481-4514` asserts
`coverage_source.source == "status-inline-small-archive"` and `status == "checked"` against a
tiny fixture mirror, which should stay below the cap and keep the test green unchanged. If it
does flip, **do not re-arm the assertion to match new behavior** — adjudicate it with stated
evidence per `~/.agent-config/skills/meta/self-maintaining-tests/SKILL.md`.

Then: `cargo check` → `clippy -D warnings` → `fmt --check` → the `cli_doctor` tests → build a
release binary and verify `cass status --json` actually returns on the live archive (it
currently never does; 13 minutes was the longest measured wait). Preserve the current binary
as a dated specimen before swapping, matching the existing rollback ritual.

## What remains after that

2. The coverage-query mechanism (blocks redeploying e3ed01f0) — establish why one zero-row
   meta query costs >90s inside frankensqlite when stock sqlite answers it in 0.00s. Per
   AGENTS.md the sanctioned response if it is a frankensqlite defect is a targeted reproducer
   filed against frankensqlite, not a bypass.
3. Fix all three coverage call sites, and the `.unwrap_or_default()` single-source defect.
4. Write the missing detectors first: one fixture with a floor written directly into `meta`
   closes three risks; a second connector in the coverage fixture closes gxw32.
5. Backfill — **currently blocked on disk, see below.**
6. Configure the mini (`ssh mini-ts`, cass 0.6.23, 4,878 sessions) as a source;
   `cass sources list` is `total: 0`.
7. Freshness: nothing schedules indexing; the skill's bootstrap
   (`~/.claude/skills/cass/SKILL.md`, jsm-owned, auto-updates daily) makes
   `cass status --json && cass index --json` every agent's first command.
8. Independent challenge and a final proof that counts sessions rather than trusting coverage.

## Two blockers the user needs to know about, and they are the "tell me why it can't" branch

- **Disk.** `/System/Volumes/Data` has **149 GiB free, below the 150 GB floor**, and
  disk-janitor is reporting PARTIAL runs. The backfill is estimated at 24–30 GB and the cass
  data dir is already 29 GB (19.68 GiB of that is the raw mirror, which the code calls
  "precious evidence" and never an automatic cleanup candidate). Do not start the backfill
  without resolving this first, and do not free space by deleting anything without explicit
  permission.
- **Usage.** The parent session's account (`george`) hit **100% of its 5-hour window**; one
  grounding lane died on it. Weekly is only 31%, so the window recovers. **Do not fan out
  while at or above 95%** — check `cusage` first.
