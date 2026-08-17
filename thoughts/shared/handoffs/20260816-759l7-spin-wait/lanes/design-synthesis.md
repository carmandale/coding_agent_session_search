# design-synthesis — bead 759l7, the minimal correct fix

Lane: design synthesis. Read-only. Six lanes fed this; every claim below is cited
to `file:line`. Registry paths are relative to
`/Users/dalecarman/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`.

---

## 0. FIRST — the fix is already in this worktree, and its own commit disproves the bead

Before proposing anything I read the tree. `HEAD` is `d748a93d checkpoint(759l7):
drop the three spawn-and-spin bridges for the ambient Cx`, and all three sites
already carry the change. `git status --short` is clean apart from `?? .agent-state/`.

So this is not a greenfield proposal. It is an adjudication of an applied change,
and it lands in three parts:

1. **The applied change is the minimal correct fix.** I would have written it
   byte-for-byte. Section 2 gives it verbatim so an implementer working from a
   clean checkout can reproduce it.
2. **The doc comments the change added are wrong**, and they are wrong in the
   exact way the same commit's own message says is wrong. That is the one edit I
   am proposing. Section 2.4.
3. **Bead 759l7's recorded root cause must be rewritten.** The spin was a real
   defect and it was never the cause of the hang. Section 1.

That third point is the expensive one. The commit already measured it, and the
measurement is the strongest evidence in this whole investigation.

---

## 1. ROOT CAUSE

**The three sites hand-rolled a bridge to obtain something the runtime already
had in scope.** `Runtime::block_on` installs an ambient `Cx` — built from the
runtime's own IO driver, timer driver, blocking pool and observability, at
`Budget::INFINITE` — for the entire duration of the root poll:
`asupersync-0.3.4/src/runtime/builder.rs:3276-3291`, the last two lines being
`let request_cx = self.request_cx_with_budget(Budget::INFINITE);` and
`let _cx_guard = crate::cx::Cx::set_current(Some(request_cx));`. That block is
**byte-identical in 0.3.2** (`asupersync-0.3.2/src/runtime/builder.rs:3239-3254`),
comment included, and it exists precisely so `Cx::current()` returns `Some`
inside `block_on` (the crate's own comment at `:3278-3288` says networking APIs
otherwise "fall back to a tight `accept4` / `WouldBlock` poll"). `Cx::current()`
is public (`0.3.4/src/cx/cx.rs:361`) and `Cx` is re-exported at the crate root in
both versions (`0.3.4/src/lib.rs:384`, `0.3.2/src/lib.rs:254`). So every one of
the three sites spawned a task, moved a `std::sync::mpsc::Sender` into it, and
polled the receiver in a `yield_now` loop in order to be handed a value that was
already sitting one function call away. Because `std::sync::mpsc` has no async
wakeup, the wait *had* to be a spin; because `block_on`'s driver
(`run_future_with_budget`, `0.3.4/builder.rs:4135-4189`) treats a self-wake as a
budget event, that spin degrades into 128 polls then 1 ms / 5 ms / 25 ms sleeps
forever (`:4158-4177`, budget default `runtime/config.rs:2185`). **That is the
whole defect, and it is version-independent.**

**The recorded hypothesis (asupersync #58) is contradicted, and so is the bead's
causal claim.** #58's premise is true — `build_request_cx_from_inner` constructs
the root `Cx` directly and the crate says so in its own comment, "The task is
still not registered in `state.tasks`" (`0.3.4/builder.rs:4196-4198`). Its stated
consequence does not follow. `RuntimeBuilder::current_thread()` is
`worker_threads(1)` (`0.3.4/builder.rs:2974-2976`), which spawns a real, separate
OS worker thread (`:322-368`, early-returning only at `worker_threads == 0`,
`:327`); spawned tasks are polled there (`three_lane.rs:4233-4248`, poll at
`:6059-6062`), reached by `inject_ready` + `wake_one()` (`:2281-2310`, `:735-747`).
`block_on` was never supposed to poll them. And the entire driver —
`block_on`, `run_future_with_budget`, `ThreadWaker`, `JoinHandle`,
`RuntimeInner::spawn`, worker startup, and `runtime/yield_now.rs` — is
**byte-identical between 0.3.2 and 0.3.4** (lanes `driver-034`, `driver-diff`,
both by `diff -u`). A byte-identical mechanism cannot be a version regression.

**The 0.3.4 hang has a different cause, below this repo, and it is measured.**
Commit `d748a93d` ran a controlled A/B in one tree with fsqlite held at 0.1.5 and
only asupersync moving: on 0.3.4 the same four download tests hang **before** the
change (spinning, killed at 150 s) and **after** it (parked, killed at 280 s).
44/48 both times. `sample` on the live hung process shows 0.0 % CPU, every test
thread parked at `runtime/builder.rs:4184`, and the worker threads idle in a
condvar — *nothing is turning the reactor*, so the socket readiness never
arrives. The source is consistent with that: `drive_io_phase` is byte-identical
between the two versions (verified this lane by diffing the extracted bodies) and
lives in the worker's **outer** loop (`0.3.4/three_lane.rs:4260`), while 0.3.4
rewrote the **inner** backoff loop so that the `BackoffTimeoutDecision::DeadlineDue`
arm `park_timeout(1 ns)`s and stays inside it (`:4357-4373`) where 0.3.2 `break`s
back out to the outer loop and thus back into `drive_io_phase`
(`0.3.2/three_lane.rs:4053-4056`), and the empty-backoff counter went from a
per-outer-iteration local to a persistent field reset only on real dispatch
(`0.3.4/three_lane.rs:2696-2697`, `:4252`). A worker that enters that inner loop
with no deadline parks indefinitely (`:4380`) and never returns to the I/O phase.
**I am labelling that last chain a hypothesis, not a finding** — I did not prove
which branch the hung workers took (§6).

One paragraph, then, for the bead: *the three sites spun because they used a
non-async channel to fetch a context `block_on` already provided; that is wrong on
every version and is what this change fixes. The 0.3.4 test hang is a separate,
still-open defect in asupersync's worker idle path that starves the I/O reactor,
and it is not caused by, or fixed by, anything in this repository.*

---

## 2. THE FIX, SITE BY SITE

Real Rust, matching what is in the tree at `HEAD`. All three type-check against
`Runtime::block_on<F: Future>(&self, future: F) -> F::Output`
(`0.3.4/builder.rs:3276` — note: no `Send` and no `'static` bound) and
`Cx::current() -> Option<Cx>` (`0.3.4/cx/cx.rs:361`).

### 2.1 `src/update_check.rs` — `fetch_latest_release`

This site is different from the other two, and the difference is why it needs no
new code at all: **it already had the correct implementation as a fallback.** The
pre-change body took a spawn branch when `Runtime::current_handle()` was `Some`,
and fell through to `Cx::current()` otherwise. The fix is a deletion — the branch
that was taken was the wrong one.

```rust
async fn fetch_latest_release() -> Result<GitHubRelease> {
    let cx = asupersync::Cx::current().context("update check requires an active asupersync Cx")?;
    fetch_latest_release_with_cx(&cx).await
}
```

Also delete the now-unused `use std::sync::mpsc::TryRecvError;` at the top of the
file. `fetch_latest_release_with_cx` is unchanged; both callers
(`fetch_latest_release_blocking` at `:897-902`, which builds its own
`current_thread` runtime, and `check_for_updates_async_impl` at `:198`, which
inherits the caller's) are unchanged.

### 2.2 `src/search/model_download.rs` — `run_download_with_cx`

```rust
    runtime.block_on(async move {
        let cx = asupersync::Cx::current().ok_or_else(|| {
            DownloadError::NetworkError("download runtime context unavailable".into())
        })?;
        f(cx).await
    })
```

Delete `use std::sync::mpsc::TryRecvError;`. The generic signature, the runtime
construction above it, and every caller are unchanged.

### 2.3 `src/pages/deploy_cloudflare.rs` — `run_cloudflare_with_cx`

```rust
    runtime.block_on(async move {
        let cx = asupersync::Cx::current()
            .ok_or_else(|| anyhow::anyhow!("Cloudflare API runtime context unavailable"))?;
        f(cx).await
    })
```

Delete `use std::sync::mpsc::TryRecvError;`. `bail!` stays imported — it is still
used at `:286`, `:324`, `:349`, `:422`, `:455`. `use std::thread;` stays — still
used at `:639`.

### 2.4 REQUIRED CORRECTION — the doc comments assert the mechanism this work disproved

This is the one change I am proposing that is not yet in the tree, and it is not
cosmetic. All three sites now carry a comment saying:

> …on a `current_thread` runtime the root future is outside task accounting, so
> the spawned task never got polled and the spin never ended. 0.3.2 tolerated it;
> 0.3.4 does not.

That is issue #58's story, written into the source as settled fact, in the same
commit whose message says the A/B disproved it. A future reader who trusts the
comment will re-derive the wrong root cause; a future reader who trusts the commit
message will not find it, because commit messages are not where people look. Two
sources, one already drifted (`.claude/rules/single-source.md`). Replace all three
with the same corrected text, adjusted per site:

```rust
/// Fetch latest release using the native asupersync HTTP client.
///
/// The context comes from the ambient `Cx` that `Runtime::block_on` installs for
/// the duration of the root poll (`runtime/builder.rs:3276-3291`, byte-identical
/// in 0.3.2 and 0.3.4), so `Cx::current()` is sufficient and nothing needs to be
/// spawned to obtain one.
///
/// An earlier revision spawned a task purely to be handed a `Cx` and read the
/// result back over a `std::sync::mpsc` receiver. That receiver has no async
/// wakeup, so the wait had to be a `yield_now` spin. The spin was wrong on every
/// version — it burned CPU fetching something already in scope — but it was NOT
/// the cause of the 0.3.4 test hang: a controlled A/B leaves the same four
/// download tests hanging with and without it. That hang is an asupersync worker
/// idle-path defect and is tracked separately. See bead 759l7.
```

Drop the last sentence of the Cloudflare variant's existing comment ("This site
had no test coverage, so it would have hung in production…") — §2.5 removes that
condition, so the comment would go stale immediately.

### 2.5 REQUIRED — one test for the latent site

`deploy_cloudflare.rs` has **zero** coverage of this path (lane `call-sites`
checked all 25 in-file tests, `tests/deploy_cloudflare.rs`, `tests/e2e_deploy.rs`,
and a repo-wide grep; the Playwright specs test an already-deployed URL). That is
the site the bead calls latent, and it is the one that would have hung in
production with nothing to catch it. It is also the only site whose fix has no
falsifier at all. Ten lines close that, with no network:

```rust
    #[test]
    fn run_cloudflare_with_cx_provides_ambient_context() {
        // Bead 759l7: this path had no coverage, so a runtime that failed to
        // hand back an ambient `Cx` would have surfaced only as a production
        // hang. Asserts acquisition, not networking.
        let result = run_cloudflare_with_cx(|cx| async move {
            let _ = cx.now();
            Ok(())
        });
        assert!(
            result.is_ok(),
            "ambient Cx must be available inside the Cloudflare runtime: {:?}",
            result.err()
        );
    }
```

Goes in the existing `#[cfg(test)] mod tests` at `:1581-1583`, which already has
`use super::*`, so the private `run_cloudflare_with_cx` is in scope. `T = ()`
satisfies `Send + 'static`; the closure captures nothing.

Do **not** assert anything about the value of `cx.now()`. The clock baseline moved
between versions — `RuntimeState::now` is `Time::ZERO` in 0.3.2 (`state.rs:818`)
and `Time::from_nanos(1_000_000_000)` in 0.3.4 (`state.rs:870`) — so a
`> 0` assertion is a false failure waiting on the shipping pin.

The mutant that proves this test is not vacuous: change `Cx::current()` in that
function to `None::<asupersync::Cx>` and confirm **this named case** goes red
(`.claude/rules/no-vacuous-test-guards.md`). Revert immediately.

---

## 3. SHARED ABSTRACTION — none is warranted

**No.** Three near-identical three-line fixes are the correct outcome here, and a
helper invented to unify them would be larger than what it replaces.

The minimalism ladder settles it at rung 3: `Cx::current()` **is** the
platform-native facility, and the whole defect was reaching past it to hand-roll a
task-plus-channel bridge. Having deleted one hand-rolled mechanism, adding a
second would be the same mistake in a smaller font.

Concretely, a shared helper would have to be generic over three things that
genuinely differ — the error type (`DownloadError` vs `anyhow::Error`), the
message, and the error-construction closure — so its signature would be longer
than the four lines it replaces, and it would need a home module that does not
exist. `.claude/rules/right-sized-mechanism.md`: the mechanism must be smaller
than the problem.

Two things that look like abstractions and should stay exactly as they are:

- **`run_download_with_cx` and `run_cloudflare_with_cx` keep earning their place.**
  They are not there to share the `Cx` acquisition; they are there because a sync
  function needs a runtime, and Cloudflare has seven call sites through them
  (`deploy_cloudflare.rs`, six via `execute_cloudflare_request` and one via
  `execute_cloudflare_multipart_request`). Leave them.
- **`fetch_latest_release_with_cx` stays split from `fetch_latest_release`.** The
  `&Cx`-taking half is the reusable unit and the ambient-acquisition half is the
  adapter. That is already the right seam.

**One optional subtractive follow-on, explicitly not part of the fix.** The
`T: Send + 'static`, `F: … + Send + 'static`, `Fut: … + Send + 'static` bounds on
both wrappers exist only because the old code moved the closure into a spawned
task. `Runtime::block_on<F: Future>` imposes neither bound
(`0.3.4/builder.rs:3276`), and both functions are private, so relaxing them is
zero-risk and purely subtractive. It is also churn on a change that has not yet
earned its place on the shipping pin. File it, do not bundle it.

---

## 4. FALSIFIERS

Four claims, four observables, four commands. Run them against the shipping pin
(`Cargo.toml:26` and `Cargo.lock:329-331` both say **0.3.2**).

**Claim A — `Cx::current()` is `Some` everywhere these sites run.** Falsified by
any test failing with `requires an active asupersync Cx`,
`download runtime context unavailable`, or
`Cloudflare API runtime context unavailable`. That string is the whole point: the
fix converts a would-be silent hang into a named error.

```
cargo test --lib update_check
cargo test --lib 'search::model_download::tests::test_download_with_mirror'
```

The first covers the 13 tests lane `call-sites` traced to the update path (note:
the bead says 12 — unresolved discrepancy, §6). The second is the 4 download
tests.

**Claim B — no behaviour change on 0.3.2.** Falsified by any test that passed at
`d748a93d^` failing at `d748a93d`. The whole-suite comparison is the only honest
version of this:

```
cargo test --lib 2>&1 | tail -5
```

Compare the pass/fail counts against the same command at `d748a93d^` (use a
detached temporary clone outside the repo — do not create a branch or a sibling
worktree, per §2.10).

**Claim C — the latent Cloudflare site actually acquires a Cx.** Falsified by
§2.5's test failing. Until that test exists, this claim has **no** falsifier and
must not be reported as verified:

```
cargo test --lib 'pages::deploy_cloudflare::tests::run_cloudflare_with_cx_provides_ambient_context'
```

**Claim D — this does not fix the 0.3.4 hang.** Already measured in `d748a93d`.
It would be falsified by the four download tests passing under 0.3.4, which is
the observable to watch if anyone re-runs the upgrade attempt. That run belongs to
the separate upgrade work, not to this fix.

**What no command here proves:** that a real Cloudflare deploy works. Only
`cass pages` against live credentials proves that, nobody has run it, and §2.5's
test is deliberately narrower — it proves context acquisition, not networking.

---

## 5. RISKS

**5.1 Behaviour on 0.3.2 (the shipping pin) — one real change, and it is the
panic path.** This is the risk that matters and it is easy to miss.

*Before:* a panic inside `f(cx)` ran on a worker thread, inside
`std::panic::catch_unwind` (`0.3.4/three_lane.rs:6059`). The unwind dropped the
`tx` that had been moved into the task, the root's loop saw
`TryRecvError::Disconnected`, and each site returned a tidy `Err(... exited before
returning a result)`. The panic was **contained**.

*After:* the work runs on the root future, so a panic unwinds straight out through
`block_on` and into the caller. Where it lands:
- `update_check`: through `fetch_latest_release_blocking` into
  `spawn_update_check`'s `std::thread` (`update_check.rs:906-916`), killing that
  thread and disconnecting its channel. The TUI already handles `Disconnected`
  (`ui/app.rs:19809-19811`), so the UI degrades cleanly. Acceptable.
- `model_download`: through `run_download_with_cx` into `run_models_install`'s
  `std::thread::spawn` (`lib.rs:90726-90745`). Same shape.
- `deploy_cloudflare`: through `run_cloudflare_with_cx` into the wizard
  (`wizard.rs:1998`) or `run_config_based_export` (`lib.rs:73693`) — i.e. onto the
  main thread. **This one changes an error return into a process-level panic.**

Net: strictly better diagnosis (a panic that used to be laundered into a generic
"task exited" string is now visible with its real message and backtrace), and
strictly worse containment on the Cloudflare path. No caller relied on the old
containment — none of them could distinguish it from a network error. I judge
this acceptable and worth naming in the bead; if it is not, the fix is `f(cx)`
wrapped in `FutureExt::catch_unwind`, which would be new machinery for a case
nobody has hit, so do not add it pre-emptively.

**5.2 Cancellation — unchanged in every way a caller can observe, one theoretical
loss.** The timeout is `asupersync::time::timeout(cx.now(), …)` driven by the same
`cx` before and after (`update_check.rs:859-873`), so timeout semantics are
identical. What is gone is that the work no longer has its own task record, so
scheduler-level cancellation can no longer target it. Nothing used that:
`try_spawn_with_cx` returns `Result<(), SpawnError>` (`0.3.4/builder.rs:3611`) —
no handle, no cancel token, in 0.3.2, 0.3.4, 0.3.9 and 0.3.10 alike. The
downloader's cancellation is its own `Arc<AtomicBool>` (`model_download.rs`), which
is untouched.

**5.3 Budget — strictly more permissive, so nothing new can be cut off.** The old
spawned task got a per-task `Budget::new()`; the root `Cx` carries
`Budget::INFINITE` (`0.3.4/builder.rs:3288`). No path can newly exhaust a budget.

**5.4 Concurrency — none was there to lose.** Each site spawned exactly one task
and immediately blocked the root waiting for it. Running inline is the same
serial order.

**5.5 The change does not fix the reported symptom, and must not be reported as
if it does.** The 16 hanging tests still hang on 0.3.4. This is a correctness and
CPU-burn fix that stands on its own merits; the upgrade blocker is separate and
open. Any status line saying "759l7 fixed, upgrade unblocked" is false.

**5.6 The checkpoint has not earned the pin yet.** `d748a93d`'s own message says
the shipping-pin regression run had not finished. Until Claim B's command is run
on 0.3.2 and compared, this is a checkpoint, not a completion.

**5.7 One thing I looked for and did not find — no fourth site.** A repo-wide
grep returns 6 `yield_now` hits: the 3 fixed here plus 3 unrelated
`std::thread::yield_now` (`indexer/mod.rs:28541`, `:34403`,
`tests/frankensqlite_concurrent_stress.rs:360`). `ui/app.rs:19804`'s `try_recv` is
a per-tick non-blocking poll of an OS-thread channel with an empty `Empty` arm —
not a spin. Re-verified this lane. The bead's "three call sites" is exhaustive.

---

## 6. WHAT I COULD NOT SETTLE

Stated plainly rather than smoothed over.

- **Why 0.3.4 starves the reactor.** I have a specific, cited hypothesis (§1: the
  inner backoff loop keeps the worker away from `drive_io_phase`, which lives in
  the outer loop) and the measured fact that the workers are in a condvar rather
  than in `epoll`. I did **not** establish which branch the hung workers actually
  take, whether `io.try_turn_with` returned `Follower` or `NoProgress`, or whether
  the `Time::ZERO` → `Time::from_nanos(1e9)` clock-baseline shift
  (`state.rs:818` → `:870`) is what flips `io_timeout` to `Duration::ZERO`. The
  decisive probe is a debugger or `sample` on a hung worker showing its frame
  inside `run_loop`, plus the `inject_ready` trace lines at
  `0.3.4/three_lane.rs:2367-2375`. That is the real blocker's investigation, not
  this one's.
- **Test count discrepancy.** The bead says 12 tests reach `update_check`'s loop;
  lane `call-sites` counted 13 by static call graph. Nobody executed a count. It
  does not change the fix; it does mean "16 tests" in the bead is an unverified
  figure.
- **Whether `Cx::current()` can ever be `None` where `Runtime::current_handle()`
  is `Some`.** I enumerated every `ScopedRuntimeHandle::new` call site in both
  versions — three each, identical: `builder.rs:345` (worker thread startup),
  `:3277` (`block_on`), `:3304` (`block_on_with_cx`). The latter two install a
  `Cx` in the same function (`:3289`, `:3305`); the worker installs one per task
  poll (`three_lane.rs:6049`). So no *async* context has the handle without the
  `Cx`. A **blocking** closure running on a worker thread between task polls would
  — none of our three sites is that, but I did not audit the blocking pool, and
  lane `cx-acquisition` flagged that region as unresolved in the opposite
  direction (Cx present, handle absent).
- **Whether the Cloudflare path works end to end.** Never executed by anyone, on
  either version, before or after this change. §2.5 narrows the gap; it does not
  close it.
- **I ran no cargo command.** Per this lane's rules. Everything above is source
  reading, `git show`, and the measurements recorded in `d748a93d`.

---

## 7. WHAT AN IMPLEMENTER DOES NEXT

1. Apply §2.4 (comment correction, three files) and §2.5 (one test).
2. Run §4 Claim A and Claim C. Then Claim B against a detached temporary clone of
   `d748a93d^`.
3. Rewrite bead 759l7's root cause to §1's closing paragraph, and open (or point
   at) the separate bead for the asupersync worker idle-path defect with §6's
   first bullet as its starting evidence.
4. Only then promote `d748a93d` from checkpoint to completion.
