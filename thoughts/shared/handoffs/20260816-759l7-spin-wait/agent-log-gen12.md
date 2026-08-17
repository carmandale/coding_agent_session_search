# Coordinator log — generation 12 (session 090aa9b4)

Continuation of `p3kgr-upstream-continuation.md` (generation 12, parent session
`21e23d4e`). Autolaunched direct path per the resume-handoff skill: all three
frontmatter fields present, `next-action-class: executable`, working copy of the
artifact byte-identical to the committed bytes at `53cef040`, every referenced
path present.

Append-only. Earlier generations' logs — `agent-log.md` and `lanes/*` — are not
touched by this session.

---

## Step 1 — the control result, read rather than re-run

The handoff's exact next action opens by asking which branch the control landed
on. It landed on the first: **pre-fix code PASSES on asupersync 0.3.2**, so this
session's inline fix is a confirmed regression on the shipping pin. Verbatim from
`~/.claude-accounts/george/jobs/21e23d4e/tmp/control-verdict.log`:

```
=== running the 4 download tests, pre-fix code, asupersync 0.3.2 ===
control rc=0 after ~55s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 5148 filtered out; finished in 0.40s

=== CONTROL VERDICT ===
  PRE-FIX CODE PASSES on 0.3.2.
  => the fix REGRESSES the shipping pin. Do not land it as written.
```

`control-prefix-032.log` opens with `Compiling coding-agent-search v0.6.9
(/Users/dalecarman/dev/coding_agent_session_search/.claude/worktrees/cass-759l7-spin-wait)`,
so the rebuild is confirmed and the binary under test is the one meant.

Proceeding to step 2 as the handoff directs.

## Step 2 — the cheapest missing measurement: pre-fix source on asupersync 0.3.10

Script: `~/.claude-accounts/george/jobs/090aa9b4/tmp/step2-prefix-on-0310.sh`.
Raw: `step2-verdict.log`, `step2-build.log`, `step2-modeldl.log` in that dir.

Tree `/tmp/cass-759l7-forward` at `d748a93d`, with `Cargo.toml`/`Cargo.lock`
moved to fsqlite 0.1.19 + asupersync 0.3.10 and `build.rs`'s `expected_version`
moved to 0.1.19 (all uncommitted, throwaway tree only). Toolchain
`nightly-2026-08-10` = rustc 1.99.0-nightly. Target dir
`/tmp/cass-759l7-forward-target`, not shared with the worktree.

Only the three bridge sites were reverted:

```
git checkout c4b3f955 -- src/update_check.rs src/search/model_download.rs src/pages/deploy_cloudflare.rs
```

Spin-wait confirmed present again (one `TryRecvError::Empty` hit per site), and
`build.rs` confirmed still on 0.1.19 (two hits), before building.

Rebuild confirmed — the trap named in the handoff's environment facts:

```
build rc=0
   Compiling coding-agent-search v0.6.9 (/private/tmp/cass-759l7-forward)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 26.29s
```

(`/private/tmp` is macOS's canonical form of `/tmp`; this is the forward tree,
not the worktree.)

Result:

```
model_download rc=0 after ~10s
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 5104 filtered out; finished in 0.54s
```

**PRE-FIX PASSES on 0.3.10.** The forward tree was restored to `d748a93d`
afterwards and the restore verified (no `TryRecvError::Empty` hits).

### What the grid now says

One tree per column, only the pin moving, the four
`test_download_with_mirror_*` tests as the subject:

| consumer source | asupersync 0.3.2 (shipping) | asupersync 0.3.4 | asupersync 0.3.10 (forward) |
|---|---|---|---|
| pre-fix — `try_spawn_with_cx` + `std::sync::mpsc` + `yield_now` spin | **PASS**, 0.40 s | HANG, killed 150 s | **PASS**, 0.54 s |
| gen-12 fix — inline on the `block_on` root via ambient `Cx` | **HANG**, killed 600 s | HANG, killed 280 s | PASS, 0.44 s |

Every cell is measured in this session or the last; none is inferred. Sources:
`control-verdict.log` (0.3.2 pre-fix), `expA-verdict.log` (0.3.2 fix),
`fixed-modeldl-verdict.log` and the `d748a93d` commit message (0.3.4 both),
`step2-verdict.log` (0.3.10 pre-fix), `expB3-verdict.log` (0.3.10 fix).

Three conclusions follow, and the first two are corrections to the record:

1. **The 759l7 fix is not a prerequisite for moving the pin.** The pre-fix
   source is green on the forward line. Bead 759l7's premise — that the spin is
   what blocks the newer runtime — is false in the direction that matters:
   the spin does not block 0.3.10.
2. **0.3.4 is the only version that hangs, and it hangs BOTH shapes.** It is not
   on the path to anywhere: the forward line runs fsqlite 0.1.19, which requires
   asupersync 0.3.9+. 0.3.4 was only ever reached via fsqlite 0.1.14, an
   intermediate pin nothing needs.
3. **The spin is still a real defect and still worth deleting** — a
   `std::sync::mpsc` receiver has no async wakeup, which is the whole reason the
   wait had to be a spin, and `deploy_cloudflare.rs` has no coverage — but it is
   cleanup, not a blocker, and it must not be paid for with a regression on the
   shipping pin.

## Step 3 — designing the replacement

The handoff prescribes `try_spawn` + `await` on the returned `JoinHandle`. A
first read of the crate found the prescription cannot be written as stated:

```
asupersync-0.3.2/src/runtime/builder.rs:3517
    pub fn try_spawn<F>(&self, future: F) -> Result<JoinHandle<F::Output>, SpawnError>
asupersync-0.3.2/src/runtime/builder.rs:3571
    pub fn try_spawn_with_cx<F, Fut>(&self, f: F) -> Result<(), SpawnError>
```

`try_spawn_with_cx` returns **no handle at all** — which is precisely why the
original author reached for a `std::sync::mpsc` channel. `try_spawn` returns a
`JoinHandle` but takes a plain future and hands it no `Cx`. So the redesign
needs a way to get a `Cx` to work that is spawned through `try_spawn`.

Five source questions were fanned out across both crate versions and the three
call sites, each answer then handed to an adversarial verifier told to refute it
(workflow `wf_bdd97755-4c5`; transcripts under this session's `subagents/`).
**Every verifier returned `refuted: false`**, two of them after running their own
executed probe rather than only re-reading.

### What the source says

| question | answer | where |
|---|---|---|
| Does a task spawned by plain `try_spawn` get an ambient `Cx`? | **Yes, unconditionally.** The runtime synthesizes a `Cx` for *every* task at admission (`state.rs:1871`, `record.set_cx` at `:1890`) and the worker installs it around each poll (`Cx::set_current` at `three_lane.rs:5729`, immediately before the poll at `:5741`). `try_spawn_with_cx` adds nothing to this — it only *also* hands the same `Cx` to a factory closure. | 0.3.2 |
| Same question on 0.3.10 | **Yes**, same mechanism, and the crate ships an in-crate test doing exactly this pattern: `runtime.block_on(runtime.handle().spawn(async { let cx = Cx::current().expect("task context"); … }))` | 0.3.10 |
| `JoinHandle` semantics | `Output = T` (bare `T`, no `Result`). A task panic is re-raised on the awaiting thread via `resume_unwind`, so it is visible rather than swallowed. Dropping the handle detaches. The waker path is race-free: `poll` stores the waker under the same `parking_lot::Mutex` that `complete_task` takes to store the result and take the waker, and the wake happens after the guard drops. On a `current_thread` runtime the root's `ThreadWaker::wake` unparks the `block_on` thread. | 0.3.2 |
| Call-site bounds | All three sites already declare exactly what `try_spawn` needs: `T: Send + 'static`, `F: FnOnce(Cx) -> Fut + Send + 'static`, `Fut: Future<Output = …> + Send + 'static`. No signature change required. | worktree |
| Is there an API giving both a `Cx` and an awaitable handle? | `Cx::spawn -> Result<TaskHandle<_>, SpawnError>` exists in **0.3.10 only**; 0.3.2 has no `Cx::spawn` at all (probe: `error[E0599]: no method named 'spawn' found`). The only 0.3.2-compatible alternative is `try_spawn_with_cx` + `asupersync::channel::oneshot`. | both |

### The shape chosen, and why

```rust
runtime.block_on(async move {
    let handle = asupersync::runtime::Runtime::current_handle().ok_or_else(…)?;
    let join = handle
        .try_spawn(async move {
            let cx = asupersync::Cx::current().ok_or_else(…)?;
            f(cx).await
        })
        .map_err(…)?;
    join.await
})
```

`try_spawn` + `Cx::current()` inside the task + `.await` on the `JoinHandle`.

- It exists **identically on 0.3.2 and 0.3.10**, so it does not have to be
  revisited when the pin moves. `Cx::spawn` would be nicer and does not exist on
  the shipping pin.
- It keeps the work **on a spawned task**, which is the only topology measured
  green in every condition. The inline shape is a measured regression on 0.3.2.
- It deletes the actual defect — the `std::sync::mpsc` receiver with no async
  wakeup, and therefore the `yield_now` spin.
- It is preferred over `try_spawn_with_cx` + `asupersync::channel::oneshot`
  (the other 0.3.2-compatible option) on two grounds: no channel at all rather
  than a different channel, and a task panic surfaces as a real panic on the
  awaiting thread instead of collapsing into a generic "task exited" error.

Applied at all three sites, including `update_check`, where the inline shape
was measured green. One idiom everywhere is worth more than shaving a task off
the one site where inline happens to be safe — a per-site rule that nobody can
explain is what produced this bead in the first place.

### Verification

`cargo check --lib --tests` clean. Then, on the **shipping pin** (fsqlite 0.1.5 /
asupersync 0.3.2 / rustc 1.94), script `verify-032.sh`, raw `gate-032-verdict.log`:

| filter | result |
|---|---|
| `search::model_download::` | **48 passed, 0 failed, 0.50 s** (rc=0) — the four that hung at 600 s under the inline shape |
| `update_check::` | **44 passed, 0 failed, 0.42 s** (rc=0) |
| `run_cloudflare_with_cx` | **2 passed, 0 failed** (rc=0) — the previously untested site |

**Binary provenance checked by content, not by mtime.** The gate run printed no
`Compiling` line, because `cargo check --lib --tests` had already produced the
artifacts. Rather than trust that, the test binary was interrogated directly:

```
target/debug/deps/coding_agent_search-983a915ea0c0a592: Mach-O 64-bit executable arm64
  'download task started without a Cx'                  1   (new code)
  'Cloudflare API task started without a Cx'            1   (new code)
  'update check task started without an asupersync Cx'  1   (new code)
  'download runtime context unavailable'                0   (old inline code, absent)
  'Cloudflare API runtime context unavailable'          0   (old inline code, absent)
```

On the **forward line** (fsqlite 0.1.19 / asupersync 0.3.10 / rustc 1.99), script
`verify-forward-0310.sh`, raw `fwd-verdict.log`, rebuild confirmed
(`Compiling coding-agent-search v0.6.9 (/private/tmp/cass-759l7-forward)`, 29.80 s)
and the same content check run against that tree's binary (new string 1, old
string 0):

| filter | result |
|---|---|
| `search::model_download::` | **48 passed, 0 failed, 0.57 s** (rc=0) |
| `update_check::` | **44 passed, 0 failed, 0.53 s** (rc=0) |

So the shape is green on both pins, which is what neither previous shape was.

**Full lib suite on the shipping pin: 5151 passed, 0 failed, 3 ignored, rc=0**
(168 s; `gate-full-verdict.log`). That is the whole suite, not a filter.

### What the adversarial verifier refuted, and what it changes

Four of the five verifiers returned `refuted: false`. The fifth refuted two
claims in the `JoinHandle` answer, both by executed probe, and one of them is
worth carrying into the code:

1. **"Nothing prints when a panicking task's handle is dropped" is false.**
   `std::panic::catch_unwind` does not suppress the default panic hook — the
   hook prints at the panic site, before the unwind is caught. Probed directly:
   `catch_unwind(|| panic!("probe-payload-zzq"))` printed to stderr while the
   catch returned `Err`. So a spawned task's panic always reaches stderr;
   awaiting the handle buys *programmatic* propagation, not first-time
   visibility. This does not change the design — awaiting is still strictly
   better than dropping — but the reason is narrower than the first answer gave.

2. **"`try_spawn` + `.await` cannot deadlock" is not categorical.**
   `RuntimeInner::spawn` calls `scheduler.inject_ready(...)` (`builder.rs:3968`),
   which returns `()`, and then returns `Ok(JoinHandle)` regardless.
   `inject_global_ready_checked` (`three_lane.rs:2093-2123`) returns early
   without enqueuing while the Lyapunov governor is in drain mode. In that state
   `try_spawn` hands back a handle for a task that will never run, the stored
   future keeps the `Arc` alive so the "executor side vanished" panic does not
   fire, and the `.await` is `Pending` forever. The crate pins the drop in its
   own `regression_governor_spawn_throttling_in_drain_mode`.

   **Inert here, and verified inert rather than assumed:** `enable_governor`
   defaults to `false` (`config.rs:2163`) and is set only by three 64-core host
   profiles; `rg -n 'enable_governor|governor' src/` in this repo returns only
   cass's own unrelated indexer-responsiveness governor. Every runtime cass
   builds is `RuntimeBuilder::current_thread()` or `multi_thread()` with default
   config. Recorded as a `// ceiling:` comment at the canonical site
   (`src/search/model_download.rs`) naming the condition and the upgrade
   trigger, rather than pre-emptively wrapping the await in a timeout for a
   configuration nothing sets.

   Note this exposure is not new: the pre-fix spin had the same one, and worse —
   an unscheduled task made it spin rather than park.

### The forward-line full suite, and why its extra failures are not attributed here

`fwd-lib.log`: 5131 passed, **20 failed**, 3 ignored, 197 s. Experiment B3
recorded **8** failures on the same tree. The 20 are those 8 plus 12
`indexer::persist::persist_internal_tests::*`, and the 12 are a single cascade:

```
parallel_wal_shadow_observer_does_not_change_persisted_state
  panicked at src/indexer/mod.rs:25646:
  closing frankensqlite writer for begin-concurrent mode
  Caused by: 0: closing frankensqlite connection
             1: database is busy
```

That test holds a process-wide env-mutation lock; its panic poisoned it, and the
other 11 then failed identically at `src/indexer/mod.rs:24301` with
`env mutation lock: PoisonError { .. }`. The primary is `database is busy` —
SQLite write contention — and this run was concurrent with (a) this session's own
full 0.3.2 suite and (b) another live session running
`cass index --force-rebuild` against the production database.

**Re-run, and the attribution holds** (`fwd-lib-rerun.log`,
`fwd-rerun-verdict.log`). With this session's own 0.3.2 suite finished:

```
test result: FAILED. 5143 passed; 8 failed; 3 ignored; finished in 143.09s
--- any PoisonError cascade? ---
  0 — no cascade
```

Eight failures, byte-identical in name to the eight experiment B3 recorded, and
zero `PoisonError`. So the 12 extras were contention and are excluded on
evidence rather than on assumption. The re-run was *not* fully quiet — the
sibling session's `cass index --force-rebuild` (pid 38174) was still running and
was left alone — so what this establishes is that the cascade is not
deterministic, which is enough to stop attributing it.

The 8 are the pin's remaining work, not this change's: none is in a module this
change touches. They are being triaged separately (workflow `wf_5db3409b-f14`)
so the toolchain decision comes with its full cost attached.

## The triage returned: the pin move is blocked by two real defects, both in cass

`wf_5db3409b-f14` completed after 39 minutes — 10 agents, 0 errors, every lane
returning. Five groups, each classified by one lane and then attacked by an
independent verifier lane.

| group | tests | classification | blocks pin |
|---|---|---|---|
| dependency-drift | 1 | expected artifact of the experiment | no |
| encrypt-overflow | 1 | rustc/std `Display` change, 1.94 -> 1.99 | no |
| fts-repair-mode | 2 | **split** — see below | **yes** |
| fts-shadow-table | 2 | fixtures build a database real SQLite also rejects | no |
| salvage-counts | 2 | new fsqlite sidecar family not in cass's allowlist | **yes** |

### Blocker 1 — the FTS write gate stops discriminating (bead `-hd4u5`)

Every in-transaction FTS write is gated on
`SELECT COUNT(*) FROM sqlite_master WHERE name = 'fts_messages' AND rootpage > 0`
(`src/storage/sqlite.rs:4126-4131`); all 14 write sites funnel through
`flush_pending_fts_entries` (:15281-15296), which consults it at :15292.
`rootpage > 0` was a proxy for "fsqlite has registered this vtable", and fsqlite
0.1.19 made the CREATE side match real SQLite: literal rootpage `0` plus shadow
tables. Measured in two probe crates pinned to the exact stacks, plus a third
party:

```
fsqlite 0.1.5        [("fts_messages", 2)]                    gate rows = 1
fsqlite 0.1.19       [("fts_messages", 0), + 4 shadow rows]   gate rows = 0
stock sqlite3 3.54.0 on cass's own FTS5_REGISTER_SQL          gate rows = 0
```

Stock SQLite agrees with 0.1.19, so 0.1.5 was the non-conformant one. This is
not a test artifact: `FrankenStorage::open` never arms
`fts_messages_present_cache`, the five sites that do are all inside the repair
path, and every production caller of `ensure_search_fallback_fts_consistency`
runs on a fresh short-lived handle. On the long-lived indexing handle the gate is
permanently false and nothing re-arms it — ordinary inserts would stop
maintaining the fallback FTS index, silently.

### Blocker 2 — a new sidecar family reopens the #236 class (bead `-xybl9`)

fsqlite-vfs 0.1.17 added `namespace.rs`, which lays two persistent sidecars per
opened database: `-fsqlite-ns-gate` (0 bytes) and `-fsqlite-ns-use` (40 bytes,
header `FSQLNS01`). Both are created before the main file is opened, on the
read-only path too, and are deliberately never unlinked. Vendored crates confirm
the boundary — 0.1.6 and 0.1.14 have no `namespace.rs`; 0.1.17 and 0.1.19 do.

cass's `has_db_sidecar_suffix` (:3008-3017) knows five suffixes and neither new
one, and the parent-dir scan applies that filter *before* the
`agent_search.corrupt.` / `.backup.` prefix test — so those sidecars become
bundle roots. The 0-byte gate is culled by the `total_bytes > 0` filter at :2051,
which is exactly why both failures are off by one rather than two. The chain
`...-fsqlite-ns-use-fsqlite-ns-use` is reachable, which is the #236 shape
(~789k orphan files / 195 GB). Worse than phantom enumeration:
`cleanup_old_backups` (:1718-1755) selects on the same prefix, so orphan sidecars
occupy retention slots and can evict real backups.

**Checked against the machine, not just the source.** The pair is already on the
production database — `agent_search.db-fsqlite-ns-gate` 0 B,
`agent_search.db-fsqlite-ns-use` 40 B with the `FSQLNS01` header. Neither
`~/.local/bin/cass` nor any cass binary under the dev tree carries the literal
(positive controls on the same search: `agent_search` 47 hits, `fsqlite` 673;
`strings` agrees), so a different fsqlite >= 0.1.17 consumer already reaches that
file. That writer is not identified. **No damage today**: no prefix-matching
sidecars, no chained names, no backups dir. The exposure arrives when cass itself
starts writing these next to quarantine and backup files — with the pin.

### What the verifiers changed

Two of the five verdicts moved, which is the reason the panel was worth running.

`fts-repair-mode` was **refuted and split**. Its two tests have different causes:
the indexer test is the gate defect above (cass owes the adaptation), while
`ensure_fts_consistency_via_rusqlite_catches_up_missing_rows` is a genuine fsqlite
deviation — a contentless FTS5 table appended in-session reports `COUNT(*) = 1`
under 0.1.19 where both 0.1.5 and stock sqlite3 3.54.0 report 2. One label would
have travelled downstream as "adaptation owed by cass" and nobody would have
filed the upstream bug. Filed as bead `-mgw1o`, unsent: filing upstream on
frankensqlite is outside this session's authorization.

`dependency-drift` kept its verdict but **half its evidence was fabricated**. The
lane reported asupersync moved to 0.3.10 in the forward manifest and that the
matching assertion failed for the same reason. Neither is true: that Cargo.toml
still reads `0.3.2` byte-identically, 0.3.10 is only the lockfile resolution of a
caret range, and the assertion never executed because `ensure(...)?` short-circuits
at the earlier line. Its proposed fix — bump the literal to `0.3.10` — would turn a
currently-green assertion red in both trees, and bumping the manifest instead would
hit `build.rs:119/138` and hard-exit the build. **Correct fix: one literal**,
`src/dependency_drift.rs:869` `"0.1.5"` -> `"0.1.19"`, in the same commit that moves
`Cargo.toml:45/181` and `build.rs:56/74`. Leave line 882 alone.

### My own error, recorded

Three lane prompts told their agents to read
`~/.cargo/registry/src/*/frankensqlite-0.1.x`. That directory does not exist — cass
renames the crate through `package = "fsqlite"`, so the vendored trees are
`fsqlite-0.1.x`, `fsqlite-ast-*`, `fsqlite-ext-fts5-*`, `fsqlite-vfs-*`. The three
lanes that stalled longest were exactly the three carrying that path. They
recovered by searching — the salvage lane's evidence cites real
`fsqlite-vfs-0.1.19/src/namespace.rs` lines — so the error cost wall-clock, not
results. The successor session relaunched a corrected copy before this one
returned; that run is redundant with these findings rather than contradicting
them.

### Net, for the toolchain decision

Moving to fsqlite 0.1.19 is not a compiler bump with a tail of test churn. It needs
three code changes in cass — the FTS gate, the sidecar allowlist plus its sweep, and
the catch-up recount — two test-only edits, one version literal, and it carries one
upstream library bug that cass can defend against but not fix. None of that is a
reason against the move; all of it is cost the decision should carry, which is why
the triage ran.
