# Lane: driver-034 — what drives spawned tasks while `block_on`'s root is pending (asupersync 0.3.4)

Owner: read-only evidence lane, bead 759l7.
Scope: `asupersync-0.3.4` registry source only. No edits outside this file. No cargo run.
Registry root: `/Users/dalecarman/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`
Paths below are relative to `asupersync-0.3.4/src/` unless absolute.

---

## Headline

**The `block_on` driver does not poll spawned tasks and never has to.** Spawned tasks are
driven by dedicated OS worker threads created at `Runtime` construction, on a code path that
does not touch `block_on` at all. `RuntimeBuilder::current_thread()` is `worker_threads(1)` —
it creates **one worker OS thread separate from the calling thread**, not a
poll-on-the-calling-thread runtime.

That contradicts the load-bearing half of the recorded hypothesis (asupersync issue #58 as
paraphrased in the task): "the root Cx is not registered in `state.tasks`, **so** the spawned
task is never polled." The premise is true (the root Cx really is unregistered — the source
says so in a comment). The consequence does not follow, because nothing in the spawn→poll path
consults the root's registration.

---

## 1. Does the `block_on` driver poll spawned tasks before re-polling the root?

**No.** `Runtime::block_on` (`runtime/builder.rs:3276`) installs a thread-local runtime handle
and an ambient `Cx`, then hands the root future to `run_future_with_budget`
(`runtime/builder.rs:3290`). That function is the entire driver, and it is 55 lines with no
reference to the scheduler, the global injector, or any task queue:

```rust
// runtime/builder.rs:4135
fn run_future_with_budget<F: Future>(future: F, poll_budget: u32) -> F::Output {
    let thread = std::thread::current();
    let thread_waker = Arc::new(ThreadWaker { thread, woken: AtomicBool::new(false) });
    let waker = Waker::from(Arc::clone(&thread_waker));
    let mut cx = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    let mut polls = 0u32;
    let budget = poll_budget.max(1);
    let mut consecutive_budget_exhaustions: u32 = 0;

    loop {
        thread_waker.woken.store(false, Ordering::Relaxed);   // :4151

        match future.as_mut().poll(&mut cx) {                  // :4155  <-- ONLY the root
            Poll::Ready(output) => return output,
            Poll::Pending => {
                if thread_waker.woken.load(Ordering::Acquire) {
                    polls = polls.saturating_add(1);
                    if polls >= budget {
                        consecutive_budget_exhaustions += 1;
                        let backoff_ms = match consecutive_budget_exhaustions {
                            1 => 1, 2 => 5, _ => 25,
                        };
                        std::thread::sleep(Duration::from_millis(backoff_ms));  // :4175
                        polls = 0;
                    }
                } else {
                    polls = 0;
                    consecutive_budget_exhaustions = 0;
                    std::thread::park();                       // :4184
                }
            }
        }
    }
}
```

Cited: `runtime/builder.rs:4135-4189`. No scheduler call site exists in it; the only three
call sites of `run_future_with_budget` in the whole crate are `block_on` (:3290),
`block_on_with_cx` (:3306), and `block_on_current_with_cx` (:3325).

### Who does poll them

1. `Runtime::with_config_and_platform` builds the inner runtime and *immediately* spawns
   worker threads: `host_services.spawn_workers(&inner, workers)` — `runtime/builder.rs:3255`.
2. `NativeThreadHostServices::spawn_worker_threads` (`runtime/builder.rs:322-368`) spawns one
   `std::thread` per worker, each running `worker.run_loop()` (`:350`). It early-returns only
   when `config.worker_threads == 0` (`:327`).
3. `worker_threads` can never be 0 at that point: `config.normalize()` runs first
   (`runtime/builder.rs:3229`) and forces `worker_threads = 1`
   (`runtime/config.rs:2011-2014`). `ThreeLaneScheduler` clamps again with
   `worker_count.max(1)` (`runtime/scheduler/three_lane.rs:1463`).
4. `RuntimeBuilder::current_thread()` is literally `Self::new().worker_threads(1)`
   (`runtime/builder.rs:2974-2976`). **One worker OS thread, distinct from the `block_on`
   thread.** The doc comment "single-threaded runtime" is about worker count, not about
   running on the caller's thread.
5. `ThreeLaneWorker::run_loop` (`runtime/scheduler/three_lane.rs:4233`) is the polling loop:
   `while !shutdown { if let Some(task) = self.next_task() { self.execute(task); continue; } ... }`
   (`:4243-4248`), with I/O driving at `:4255` and park/backoff at `:4269-4391`.

So while `block_on`'s root is `Pending` and the calling thread is parked or sleeping in the
backoff at `:4175`, the worker thread is concurrently running `next_task()`/`execute()`.

### The spawn→wake handshake is complete

Both spawn entry points end identically: store the future in the task table, then inject.

- `RuntimeInner::spawn` (`runtime/builder.rs:3986`) → `guard.create_task(...)` (`:4006`,
  which does `self.tasks.store_spawned_task(...)` at `runtime/state.rs:2081-2082`) →
  `self.scheduler.inject_ready(task_id, ...)` (`runtime/builder.rs:4010`).
- `RuntimeInner::spawn_with_cx` (`runtime/builder.rs:4020`) → `create_task_infrastructure`
  (`:4035`) → `guard.store_spawned_task(task_id, StoredTask::new_with_id(wrapped, task_id))`
  (`:4049`) → `self.scheduler.inject_ready(task_id, ...)` (`:4055`).

`ThreeLaneScheduler::inject_ready` (`runtime/scheduler/three_lane.rs:2325`) →
`inject_global_ready_checked` (`:2281`) → `self.global.inject_ready(...)` + **`self.wake_one()`**
(`:2308-2310`) → `WorkerCoordinator::wake_one` → `self.parkers[slot].unpark()` and
`io.wake()` (`:735-747`). A parked worker is therefore unparked by the injection.

The one place injection can be silently dropped — the governor drain throttle at
`three_lane.rs:2285-2295`, which `return`s without queueing — **cannot fire on a live
runtime**: `should_throttle_spawns` reads `self.workers.first()` (`:2117`), and
`RuntimeInner::new` has already moved every worker out with
`scheduler.take_workers()` (`runtime/builder.rs:3913`; `take_workers` does
`std::mem::take(&mut self.workers)` at `three_lane.rs:2582`). With `self.workers` empty the
predicate falls through to `false` (`:2124`). `enable_governor` also defaults to `false`
(`runtime/config.rs:2207`). Dead path, both ways.

---

## 2. Is the root future registered in task accounting?

**No, and the crate says so in its own comment.**

`block_on` builds its ambient Cx through `build_request_cx_from_inner`
(`runtime/builder.rs:3288` → `:4191`), which constructs `crate::cx::Cx::new_with_drivers(...)`
directly (`:4230-4244`). It never calls `create_task_infrastructure`, so no `TaskRecord` is
inserted, no region admission (`region_record.add_task`, `runtime/state.rs:1946`) happens, and
no cancel-protocol registration (`runtime/state.rs:1917-1919`) happens.

Two comments state it outright:

- `runtime/builder.rs:4196-4198`: *"The task is still not registered in `state.tasks` — that's
  a deeper refactor — but the determinism breach is closed."*
- `runtime/builder.rs:3941-3948`: *"the ID is still not in the runtime's task arena (closing
  that gap requires a deeper structured-spawn refactor)"*.

### What concretely follows

Each of these is a real consequence, and **none of them is "spawned tasks are not polled"**:

- **The scheduler cannot wake the root.** `RuntimeState::task_completed(task_id)` returns the
  waiter list from the task record; with no record there is no waiter slot, so
  `wake_dependents_locked` (`three_lane.rs:6127`) can never target the root. The root's only
  wake channel is the `ThreadWaker` in `run_future_with_budget` (`builder.rs:4118-4133`),
  i.e. `thread.unpark()`.
- **A scheduler-side wake aimed at the root is a silent no-op.** `inject_ready` takes the
  `None => "task record doesn't exist … allow injection"` branch
  (`three_lane.rs:2344-2348`), so the unregistered TaskId is pushed into the global ready
  queue. A worker then pops it, `execute` finds no stored future
  (`tt.remove_stored_future(task_id)` → `None`, `three_lane.rs:5869`) and no thread-local
  local task (`:5906`), and **returns silently** at `:5908`. Nothing is logged at error level.
- **Invisible to the deadline monitor.** It snapshots `guard.tasks_iter()`
  (`runtime/builder.rs:406-409`), which walks the arena; the root is not in it.
- **Invisible to quiescence/live-task accounting.** `Runtime::is_quiescent`
  (`runtime/builder.rs:3378`) and `live_task_count` (`runtime/state.rs:2818`) count arena
  records only.
- **ID-aliasing hazard (structural, not observed).** Request-scoped IDs are
  `TaskId::from_arena(ArenaIndex::new(index, 1))` with `index` starting at 1
  (`runtime/builder.rs:3950-3957`, counter init `:3935`) — generation hard-coded to `1`.
  Real task records come from `TaskTable::insert_pooled_task_with` → `Arena::insert_with`
  (`runtime/task_table.rs:485`; `tasks: Arena<TaskRecord>` at `:38`), where a *fresh* slot
  gets generation `0` (`util/arena.rs:178,183,220,225`) and a *recycled* slot gets
  `cur_gen.wrapping_add(1)` (`util/arena.rs:266`). So after one recycle a real task occupies
  generation 1 and can share a TaskId with a request-scoped root Cx. I did not observe this
  firing; recording it as a hazard with citations, not as a diagnosis.

---

## 3. DECISIVE: does awaiting a `try_spawn` `JoinHandle` work?

**Yes. The task makes progress and the root is woken on completion.** Traced end to end, no
gap found. This is a live, working alternative to the mpsc spin.

Waker path, in order:

1. Root future awaits `JoinHandle<T>`. `impl Future for JoinHandle<T>`
   (`runtime/builder.rs:3686-3726`): locks `JoinState`, `guard.result.take()` is `None`, so it
   stores `guard.waker = Some(cx.waker().clone())` (`:3706-3712`) and returns `Poll::Pending`.
   The waker it stores is `run_future_with_budget`'s `ThreadWaker` (`:4141-4142`).
2. Root returns Pending, `woken` is false, so the calling thread calls `std::thread::park()`
   (`builder.rs:4184`).
3. The worker thread pops the spawned task (`three_lane.rs:4244`) and polls it in `execute`
   (`:6059-6062`).
4. The task's wrapper future — built in `RuntimeInner::spawn` (`builder.rs:3994-3999`) —
   completes and calls `complete_task(&join_state_for_task, result)`.
5. `complete_task` (`builder.rs:4107-4116`) sets `guard.result = Some(output)`, takes the
   stored waker **while holding the same mutex**, drops the guard, and calls `waker.wake()`.
6. `ThreadWaker::wake` (`builder.rs:4124-4128`) sets `woken = true` (Release) and calls
   `self.thread.unpark()`.
7. The `block_on` thread returns from `park()`, loops, re-polls the root, `JoinHandle::poll`
   takes `Some(Ok(output))` and returns `Poll::Ready` (`builder.rs:3715-3718`).

No lost wakeup in either direction:

- **JoinState side:** `JoinHandle::poll` holds the `JoinState` mutex across both the
  `result.take()` check and the waker store; `complete_task` takes the same mutex to set the
  result and take the waker. Whichever runs first, the other sees its effect.
- **Park side:** `unpark()` sets a token, so an `unpark` landing between the `woken.load` at
  `:4158` and the `park()` at `:4184` makes that `park()` return immediately.

Two caveats worth stating rather than a clean "yes":

- **Failure mode is a panic, not a silent deadlock.** If the executor side is dropped without
  producing a result (runtime shutdown, forced cancel), `JoinHandle::poll` sees
  `Arc::strong_count(&this.state) == 1` and **panics** with `"task was dropped or cancelled
  before completion"` (`builder.rs:3698-3704`). That is loud, and it is the opposite of the
  worse-than-spinning outcome the lane brief worried about.
- **`try_spawn` requires `F: Send + 'static` and `F::Output: Send + 'static`**
  (`builder.rs:3557-3560`), same as `try_spawn_with_cx`'s `Fut` bound (`:3611-3614`). No new
  constraint.

### Correction to an "already established" premise

> "`try_spawn` takes a bare future, so the task gets no `Cx`."

**Contradicted by the source.** Every task created through `create_task_infrastructure` is
given its own `Cx` built from the runtime's drivers (`runtime/state.rs:1977-1990`) and that Cx
is stored on the record via `record.set_cx(cx.clone())` (`runtime/state.rs:1996`;
field `pub cx: Option<Cx>` at `record/task.rs:336`, setter `:462-464`). `create_task` — the
path `try_spawn` uses — calls exactly that helper (`runtime/state.rs:2056-2057`).

The worker then installs it as the ambient Cx for the duration of the poll:

```rust
// runtime/scheduler/three_lane.rs:5875-5876
// Preserve full Cx so scheduler sets CURRENT_CX during poll.
let task_cx = record.cx.clone();
...
// three_lane.rs:6049
let _cx_guard = crate::cx::Cx::set_current(task_cx);
```

`Cx::set_current` pushes the frame with the full capability mask (`cx/cx.rs:465-480`).

So a future spawned with bare `try_spawn` reaches its own task Cx via `Cx::current()`. The
only real difference from `try_spawn_with_cx` is ergonomic: `spawn_with_cx` hands the Cx to a
factory closure by value (`builder.rs:4042`), `try_spawn` requires the `Cx::current()`
lookup. That matters for the cass call sites, whose spawned bodies take `&cx` explicitly —
they would need `Cx::current()` at the top of the async block instead.

---

## 4. What `yield_now()` actually does

It **re-wakes the current future and does not touch the scheduler.** Full implementation:

```rust
// runtime/yield_now.rs (impl Future for YieldNow)
fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    assert!(!self.completed, "yield_now future polled after completion");
    if self.yielded {
        self.completed = true;
        Poll::Ready(())
    } else {
        self.yielded = true;
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}
```

`cx.waker()` is whatever waker is driving the *current* future. Inside `block_on`'s root that
is the `ThreadWaker` (`builder.rs:4141`), so `wake_by_ref` sets `woken = true` and unparks the
already-running thread (`builder.rs:4129-4132`). There is no queue push, no
`scheduler.inject_ready`, no `wake_one`. The file is **byte-identical between 0.3.2 and
0.3.4** (`diff -u` returns empty), so `yield_now` is not the behavioural delta between the two
versions.

Concretely, inside the cass spin loop the effect is: root polls → `Empty` → `yield_now`
Pending with `woken = true` → `run_future_with_budget` takes the spin branch (`:4158-4177`),
counts `poll_budget` polls (default **128**, `runtime/config.rs:2185`), then sleeps 1ms, then
5ms, then 25ms per 128 polls forever. It is a throttled spin, not a CPU peg — and it never
hands control to the scheduler, because on this thread there is no scheduler to hand it to.

---

## Null results / what this lane could NOT establish

- **I cannot explain the 0.3.2 → 0.3.4 hang from the driver mechanism.** Everything above is
  materially the same in both versions. `diff -u` on `builder.rs` between 0.3.2 and 0.3.4
  shows **no change** to `block_on`, `run_future_with_budget`, `ThreadWaker`, `JoinHandle`,
  `RuntimeInner::spawn`, or worker startup. The only change touching the spawn path is
  `spawn_with_cx` gaining an explicit `system_cx` argument
  (0.3.4 `builder.rs:4034-4040` vs 0.3.2's `create_task_infrastructure(self.root_region, ...)`),
  and that argument is **discarded** — `let _ = caller_cx;` at `runtime/state.rs:1884`.
  `runtime/yield_now.rs` is identical. `runtime/scheduler/three_lane.rs` has ~1081 changed
  lines, which I did not audit; if the regression is a runtime bug rather than an
  application-level one, that file and the timer/clock baseline change (`Time::ZERO` →
  `Time::from_nanos(1_000_000_000)` throughout the 0.3.4 tests) are where I would look next.
- I did not run anything. No cargo, no probes. Every claim above is a source reading.

## Bearing on the fix

- The side channel exists because `try_spawn_with_cx` returns `Result<(), SpawnError>`
  (`builder.rs:3611`). `try_spawn` + `await JoinHandle` removes the side channel, removes the
  spin, and — per section 3 — has a complete waker path back to the `block_on` thread.
- The spawned body's `&Cx` requirement is satisfiable under `try_spawn` via `Cx::current()`
  (section 3, `three_lane.rs:6049`), so the rewrite does not lose the Cx.
- If the tasks are in fact never being polled on 0.3.4, the cause is **not** the root's
  absence from `state.tasks`, and swapping the channel for a `JoinHandle` would then convert
  the spin into a `park()` that also never returns. That is the risk to falsify before
  committing to the rewrite: it needs a lane that proves the worker thread polls *anything*
  on 0.3.4, not a source reading.
