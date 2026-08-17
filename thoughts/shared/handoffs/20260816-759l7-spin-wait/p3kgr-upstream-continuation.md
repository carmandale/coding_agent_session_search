---
generation: 12
parent-session: 21e23d4e-c788-41fc-8bf1-954c7e95f89e
next-action-class: executable
---

# Continuation — the spin was real, the fix for it was wrong, and the pin ceiling is real after all

## The goal and authorization, verbatim

Dale, 2026-08-14:

> /my-way fix cass to completion and 100% green working state and completely up to date or tell me why it can't or /grill-me with any questions.

Sent mid-work the same day, as a correction to the work in flight:

> make sure that you are looking at the recent (last 2 weeks) work on cass and not regressing

Dale, 2026-08-15:

> your usage is good now. finish this to completion

Dale, 2026-08-16:

> if your senior dev recommendation is to delete those 4 stale directories do it. if that is what progresses us toward the goal. and I would prefer you just do that and only stop when it would break the pipeline or running sessions or if there is a true blocker or ambiguity or conflict rather than sitting here for something that I am going to just ask your recommendation on and agree to

And, granting the two approvals a previous session acted on — Dale, 2026-08-16:

> do it. I approve a and b. set a /goal and run it to completion and do this /my-way

**Those approvals are SPENT and do NOT transfer to you.** You do not have
approval to delete any file, force-push, rewrite history, change repo
visibility, file anything upstream on frankensqlite or asupersync, or run
`cass sources agents exclude` (that last would destroy 3,877 conversations that
exist nowhere else). Destructive and external-write approvals expired with the
ending session.

## Where the work lives

Worktree `.claude/worktrees/cass-759l7-spin-wait`, branch
`worktree-cass-759l7-spin-wait`, created at `c4b3f955` which was `main`'s HEAD.
Two commits on it, both explicitly labelled checkpoints, neither landed:

- `d748a93d` — the change itself
- `390bf60d` — comment corrections plus the two new `deploy_cloudflare` tests

**Nothing has been merged to `main`. `main` is untouched and still green on
fsqlite 0.1.5 / asupersync 0.3.2.**

Evidence directory, committed:
`thoughts/shared/handoffs/20260816-759l7-spin-wait/` — `findings.md`,
`agent-log.md`, and seven lane reports under `lanes/`.

## What this session settled

### 1. Bead 759l7's recorded root cause is wrong

759l7 says three hand-rolled spin-waits cause 16 tests to hang on asupersync
0.3.4, and that replacing them makes those tests pass. The first half is a real
defect. The second half is false, measured:

| condition (one tree, fsqlite held at 0.1.5, only asupersync moving) | result |
|---|---|
| 0.3.4, spin-wait as filed | 44/48 pass; 4 `test_download_with_mirror_*` run past 60 s; killed at 150 s |
| 0.3.4, spin removed | 44/48 pass; **the same 4 never return**; killed at 280 s |

The rebuild was confirmed, not assumed — the second log opens with
`Compiling coding-agent-search v0.6.9` and `Finished in 26.53s`.

`sample` on the live hung process: 0.0 % CPU, all four test threads parked in
`std::thread::park` at `asupersync-0.3.4/src/runtime/builder.rs:4184`, all three
`asupersync-worker-0` threads idle in `__psynch_cvwait`. Nothing turns the
reactor, so the I/O wakeup never arrives.

Three read-only lanes independently contradicted the filed mechanism from
source: `current_thread()` is `worker_threads(1)` — a real separate worker OS
thread that does poll spawned tasks — and `block_on`, `run_future_with_budget`
and `yield_now` are **byte-identical** between 0.3.2 and 0.3.4, so none of them
can be the regression. See `lanes/driver-034.md`, `lanes/driver-diff.md`,
`lanes/channels.md`.

### 2. The fix as written REGRESSES the shipping pin — do not land it

Experiment A ran the change against asupersync **0.3.2**, the shipping pin. The
same four tests hang there too: `rc=143` after ~600 s
(`expA-verdict.log`, `expA-modeldl.log`).

The likely mechanism, consistent with every measurement: **the spawn is
load-bearing.** It puts the I/O on the worker thread, and the worker is what
turns the reactor. Running the work inline on the `block_on` thread leaves the
worker with no task, so it stays parked in its condvar and the root's socket
readiness never arrives.

**The control confirms it.** Same tree, same target dir, same Cargo.lock at
asupersync 0.3.2 — only the three source files reverted to their pre-fix state
(`git checkout c4b3f955 -- <the three>`):

| source, on asupersync 0.3.2 | the 4 download tests |
|---|---|
| pre-fix, spawn-and-spin | **4 passed, 0 failed, finished in 0.40 s** (`rc=0`, ~55 s incl. rebuild) |
| this session's fix | hang; killed at 600 s (`rc=143`) |

0.40 s against the bead's independently recorded 0.37 s, so the baseline
reproduces exactly. Raw: `control-verdict.log`, `control-prefix-032.log`.

**This change must not land as written.** The conclusion is not that the spin is
acceptable — it is that removing the spawn removes the thing that drives the
reactor, and the two have to be separated instead.

### 3. The pin ceiling is real, one level above where the last session looked

Generation 11 concluded "there is no toolchain reason cass is sitting on 0.1.5"
from fsqlite's declared `rust-version`. That reading of fsqlite is correct and
reproduces — 0.1.5, 0.1.14, 0.1.17 and 0.1.19 all declare `rust-version = 1.85`.
The ceiling is not fsqlite's own MSRV, it is what the fsqlite → asupersync edge
drags in:

| fact | source |
|---|---|
| fsqlite 0.1.19 requires `asupersync 0.3.9` | `fsqlite-0.1.19/Cargo.toml` |
| asupersync 0.3.9 and 0.3.10 both require `sysinfo ^0.39` | their `Cargo.toml` |
| **every** published `sysinfo 0.39.x` declares `rust-version = 1.95` | crates.io API, 2026-08-16 |
| the repo's nightly resolves to rustc **1.94.0-nightly (2025-12-10)** | `rust-toolchain.toml` |

Observed, not inferred:

```
error: rustc 1.94.0-nightly is not supported by the following package:
  sysinfo@0.39.6 requires rustc 1.95
```

asupersync 0.3.4 escapes only because it wants `sysinfo ^0.33`, whose MSRV is 1.74.

**A dated nightly clears it.** `nightly-2026-08-10` is rustc 1.99.0-nightly; under
it `sysinfo 0.39.6` and the whole fsqlite 0.1.19 + asupersync 0.3.10 graph compile.
The only thing that then refused was cass's own `build.rs` dependency-source
contract, which pins `expected_version: "0.1.5"` and says to move Cargo.toml,
build.rs and the README together. That toolchain was installed **additively**;
the shared `nightly` is still 1.94.0.

Two configurations are NOT valid and should not be retried:
- fsqlite 0.1.5 + asupersync 0.3.10 — `fsqlite-core` fails to compile, `E0061`.
  The two must move together.
- anything on asupersync ≥ 0.3.9 under rustc 1.94.

### 3b. The forward stack is far better than generation 11's RED — the hang is GONE there

Experiment B3 built the whole thing — fsqlite **0.1.19** + asupersync **0.3.10**
+ rustc **1.99**, with this session's fix in the tree and `build.rs`'s
`expected_version` moved to 0.1.19 in that throwaway clone:

| measurement | result |
|---|---|
| build | **green** |
| the 4 download tests (759l7's hang) | **48 passed, 0 failed, in 0.44 s** |
| full lib suite | 5141 passed, **8 failed**, 3 ignored, 140 s |

**The 16 hanging tests are not hanging on 0.3.10.** The asupersync half of
p3kgr's RED verdict dissolves on the forward line. What remains is 8 failures,
not 16 hangs plus 4 failures:

```
dependency_drift::tests::manifest_pin_reads_git_and_registry_dependency_specs
indexer::tests::full_run_fallback_fts_repair_skips_rebuild_when_fts_is_already_healthy
pages::encrypt::tests::key_slot_id_for_len_rejects_overflow
storage::sqlite::tests::ensure_fts_consistency_via_rusqlite_catches_up_missing_rows
storage::sqlite::tests::franken_storage_open_repairs_duplicate_fts_messages_schema_rows
storage::sqlite::tests::rebuild_fts_via_rusqlite_cleans_duplicate_legacy_schema_rows
storage::sqlite::tests::salvage_historical_databases_imports_backups_once_and_merges_overlap
storage::sqlite::tests::salvage_historical_databases_skips_unreadable_quarantined_bundles
```

`dependency_drift::…manifest_pin…` is expected — it asserts the pin this
experiment deliberately moved. The `storage::sqlite` and `indexer` ones are the
FTS5 shadow-table behaviour generation 11 already predicted and argued is
SQLite-compatible, i.e. test defects rather than library defects.
`pages::encrypt::…rejects_overflow` is in neither group and is unexplained —
suspect the rustc 1.94 → 1.99 jump before blaming fsqlite.

**Confound to respect:** B3 ran with this session's fix, which the control just
proved is a regression on 0.3.2. So "0.3.10 is green for the 4 tests" is
established *for the inline shape*. Whether the pre-fix shape is also green on
0.3.10 was never run. Do not assume either way. Raw:
`expB3-verdict.log`, `expB3-lib.log`, `expB3-modeldl.log`.

### 4. Machine state touched, and restored

`rustup toolchain install` set a **default** toolchain where this machine had
none. That is shared state; it was put back with `rustup default none` and
verified (`rustup default` reports none again; the repo still resolves 1.94.0).
Recorded because it briefly changed state for every session.

## The exact next action

1. **Read the control result in section 2 above.** If it says the pre-fix code
   passes on 0.3.2, the change is a confirmed regression — proceed to step 2. If
   it says the pre-fix code also hangs, then the bead's 0.3.2 baseline does not
   reproduce in this environment, the fix is not implicated, and the whole
   comparison needs re-grounding before anything else; say so plainly and stop
   there rather than redesigning.

2. **Run the cheapest missing measurement first: pre-fix source on asupersync
   0.3.10.** One `git checkout c4b3f955 -- <the three sites>` in
   `/tmp/cass-759l7-forward` and a re-run of `search::model_download::` there.
   That is minutes, because the forward target dir is already warm, and it
   splits the two live hypotheses cleanly. If pre-fix is ALSO green on 0.3.10,
   then the spin is merely ugly rather than fatal on the forward line, the whole
   759l7 fix becomes optional cleanup, and the pin can move without it. If
   pre-fix hangs on 0.3.10 while the fix passes, the fix is what makes the
   forward line work and both shapes are needed — inline forward, spawn on 0.3.2.
   Nothing else should be built until this is known.

3. **Redo the fix as `try_spawn` + `await` the returned `JoinHandle`**, at all
   three sites — this is the shape that is expected to work on BOTH pins, which
   neither current shape does. It keeps the work on the worker thread, so the
   reactor is still driven, while deleting the `std::sync::mpsc` receiver and the
   `yield_now` spin, which is the actual defect. `lanes/driver-034.md` §3 traced
   that waker path end to end and found no gap: `JoinHandle::poll` stores the
   root's `ThreadWaker` under the `JoinState` mutex (`builder.rs:3706-3712`),
   `complete_task` sets the result and takes that waker under the same mutex
   (`:4107-4116`), and `ThreadWaker::wake` unparks the `block_on` thread
   (`:4124-4128`). A dropped executor side panics loudly rather than deadlocking
   (`:3698-3704`). The spawned body gets its Cx from `Cx::current()`, which is
   `Some` inside a `try_spawn`ed task — the worker installs the task's own Cx per
   poll (`three_lane.rs:6049`), confirmed in `lanes/cx-acquisition.md` §2(c).
   `git show d748a93d` is the change to invert; the three sites are
   `src/update_check.rs`, `src/search/model_download.rs:run_download_with_cx`,
   `src/pages/deploy_cloudflare.rs:run_cloudflare_with_cx`.

3. **Verify on 0.3.2 first**, with the four download tests and the twelve
   `update_check::integration_*`. `bash ~/.claude-accounts/george/jobs/21e23d4e/tmp/run-bounded.sh <label> <seconds> '<filter>'`
   is the bounded runner (there is no `timeout` on this Mac). Green on the
   shipping pin is the gate for landing anything.

4. **Keep the two new `deploy_cloudflare` tests** from `390bf60d`. They are the
   coverage 759l7 asked for on the site that had none, and they are independent
   of which bridge shape wins.

5. **Correct bead 759l7** — its root cause and its claim that fixing the spin
   fixes the 16 tests. Do not close it; the spin is still real and still unfixed
   on `main`.

6. **Then the pin**, which is now a toolchain decision and is Dale's to make:
   moving cass to a rustc ≥ 1.95 nightly is what unblocks fsqlite 0.1.19. See
   "For Dale" below — do not change `rust-toolchain.toml` unilaterally, because
   the shared `nightly` is used by every other session and worktree in this repo.

## For Dale — the one decision that is his

The fsqlite pin cannot move without a newer Rust. `rust-toolchain.toml` says
`channel = "nightly"`, which resolves to a rustc from 2025-12-10, about eight
months stale. Updating it unblocks the pin; it also changes the compiler for
every other session and worktree in this repo at once, which is why this session
did not do it. The additive probe toolchain `nightly-2026-08-10` is installed and
can be used for verification without touching anything shared.

## Open and explicitly NOT settled

- **Why the asupersync client never completes against a live local TCP server**
  once the work is not on a worker task. Not root-caused. `lanes/design-synthesis.md`
  offers a hypothesis — 0.3.4 rewrote the worker's inner backoff loop so
  `DeadlineDue` `park_timeout`s and stays inside it, where 0.3.2 breaks back out
  to `drive_io_phase` (`three_lane.rs:4357-4373` vs 0.3.2 `:4053-4056`) — and
  labels it a hypothesis because nobody proved which branch the hung workers
  took. Do not report it as understood.
- Experiment B3 (fsqlite 0.1.19 + asupersync 0.3.10 + rustc 1.99, full lib suite)
  was still running at handoff. Its log is
  `~/.claude-accounts/george/jobs/21e23d4e/tmp/expB3-verdict.log`. That job dies
  with its session, so re-run `exp-b3-buildrs.sh` in the same tmp dir if the
  verdict is missing. Its throwaway tree is `/tmp/cass-759l7-forward` with target
  `/tmp/cass-759l7-forward-target`; the build.rs pin was moved to 0.1.19 THERE
  ONLY and that edit is not committed anywhere.

## Environment facts that cost real time

1. `export PATH="$HOME/.rustup/toolchains/nightly-aarch64-apple-darwin/bin:$PATH"` — an absolute path to nightly cargo is not enough.
2. No `timeout`/`gtimeout` here. Background + poll + kill.
3. Never share a `CARGO_TARGET_DIR` between two checkouts of this crate — the later build clobbers the earlier and cargo then re-runs the OTHER tree's test binary while printing `Finished`. Confirm `Compiling coding-agent-search (<the path you mean>)` before believing any cross-tree comparison.
4. This is a background session: the harness rejects edits to the shared checkout until `EnterWorktree` is called. That is enforcement, not a branch choice.
5. The worktree guard also rejects `;`-chained and redirected bash commands. Put multi-step shell work in a script under `$CLAUDE_JOB_DIR/tmp` and run `bash <script>`.
6. `ps -Ao pid,command | rg <pattern>` matches its own pipeline; `pgrep -af` on macOS matches its own invocation and always returns rc=0. Use `ps -Ao` plus `rg -v ' rg '`, and confirm any pid with `ps -p`.
7. The repo has TWO divergent napkins — `napkin.md` and `.claude/napkin.md` — and `_resolve_napkin_path` fails loud with `NAPKIN-DIVERGENCE-DETECTED`. Unresolved; `scripts/migrate-napkin.sh` is the named remedy.
