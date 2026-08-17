# Lane: driver-diff — what changed in the asupersync runtime driver between 0.3.2 and 0.3.4

Read-only lane. Sources read directly from
`/Users/dalecarman/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/asupersync-{0.3.2,0.3.4,0.3.9,0.3.10}/src/`.
Every path below is relative to that registry root. No cargo commands were run;
nothing outside this file was written.

---

## Headline

**`block_on` did not change. Not one byte.** The claim in the bead — that the
0.3.2→0.3.4 hang is `Runtime::block_on` polling the root future through a
separate `run_future_with_budget` loop — describes a property that is *identical*
in both versions, so it cannot by itself be the regression. The regression, if it
is in `runtime/`, is on the **worker/scheduler side**, and I could not reduce it
to a single proven mechanism from source reading alone. That negative is stated
plainly below rather than dressed up.

Two established facts in the task brief are **contradicted by the source** and
are recorded as such in §2. They matter because they change what the consumer fix
has to be.

---

## 1. What specifically changed in how `block_on` drives spawned work

**Nothing.** Both of the following are byte-identical between the two versions
(`diff -u` on `src/runtime/builder.rs` produces no hunk inside either function):

| function | 0.3.2 | 0.3.4 |
|---|---|---|
| `Runtime::block_on` | `builder.rs:3239–3253` | `builder.rs:3276–3290` |
| `run_future_with_budget` | `builder.rs:4088–4143` | `builder.rs:4135–4190` |

`block_on` installs a scoped runtime handle, builds an ambient request `Cx` via
`request_cx_with_budget(Budget::INFINITE)`, and calls `run_future_with_budget`
(0.3.2 `builder.rs:3250–3252`; 0.3.4 `builder.rs:3287–3289`). That loop polls the
root future against a `ThreadWaker` that only sets a flag and `unpark`s the
calling thread (0.3.2 `builder.rs:4074–4086`; 0.3.4 `builder.rs:4121–4133`). It
never touches the scheduler in either version.

`yield_now` is also byte-identical (`diff` on `src/runtime/yield_now.rs` exits 0)
and never touches the scheduler either — it calls `cx.waker().wake_by_ref()` and
returns `Pending` once (`0.3.4/src/runtime/yield_now.rs:20–30`). Under `block_on`
that waker *is* the `ThreadWaker`, so the root's `yield_now` re-wakes only the
root. **That is true on 0.3.2 as well.** The bead's sentence "`yield_now` just
re-wakes the root" is correct and is not a 0.3.4 change.

So the spin-wait's only hope of completing — in either version — is that a
**worker thread** picks up the spawned task. Which brings us to §2.

---

## 2. Two recorded claims the source contradicts

### 2a. A `current_thread()` runtime is not thread-less; it has one real worker

`RuntimeBuilder::current_thread()` is `Self::new().worker_threads(1)` in both
versions (0.3.2 `builder.rs:2937–2939`; 0.3.4 `builder.rs:2974–2976`).
`with_config_and_platform` unconditionally calls
`host_services.spawn_workers(&inner, workers)` (0.3.2 `builder.rs:3218`), and
`NativeThreadHostServices::spawn_worker_threads` returns early **only** when
`config.worker_threads == 0` (0.3.2 `builder.rs:326–328`). At 1, it spawns a real
`std::thread` running `worker.run_loop()` (0.3.2 `builder.rs:341–353`).

The consumer sites all use `current_thread()` — `src/update_check.rs:898`,
`src/search/model_download.rs:1000`. So there **is** a separate worker thread
whose whole job is to poll the spawned task, on both versions. Upstream issue
#58's framing ("the root Cx is not registered in `state.tasks`, so the spawned
task is never polled") describes the `worker_threads == 0` shape and does not, as
written, explain a hang on `current_thread()`.

### 2b. `spawn_with_cx` releases the state lock before injecting, in both versions

`RuntimeInner::spawn_with_cx` creates the task under the `RuntimeState` mutex,
`drop(guard)`s, and only then calls `self.scheduler.inject_ready(...)` (0.3.2
`builder.rs:3978–4011`; 0.3.4 `builder.rs:4020–4053`). There is no held-lock
spawn on the caller side in either version.

---

## 3. The four substantive driver/scheduler deltas, ranked

`diff -r` over `src/runtime/` shows churn in ~40 files, most of it added
metamorphic-test modules. Four changes touch the live spawn→dispatch path.

### (A) The worker idle/backoff loop was rewritten — strongest candidate

0.3.2 `three_lane.rs:3977–4073` vs 0.3.4 `three_lane.rs:4261–4392`.

Three behavioural changes, all in the direction of *parking sooner and staying
parked longer*:

1. **The backoff counter became persistent state.** 0.3.2 declares
   `let mut backoff = 0;` **inside** the outer `run_loop` iteration
   (`three_lane.rs:3978`), so every return to the outer loop restarted the
   8-spin + 2-yield budget from zero. 0.3.4 moves it to a worker field
   `empty_backoff` (`three_lane.rs:2696–2697`, initialised at `:1567`), reset
   only on real dispatch (`run_loop` calls `reset_empty_backoff()` at
   `:4252` when `next_task()` returns `Some`) or after an actual park
   (`:4380`). New constant `EMPTY_BACKOFF_PARK_THRESHOLD = SPIN_LIMIT +
   YIELD_LIMIT = 10` (`:179`), consumed by `advance_empty_backoff`
   (`:4211–4222`). Upstream's own test asserts the intent: *"spurious
   outer-loop breaks must not reset the idle backoff budget"*
   (`three_lane.rs:7132–7135`).
2. **A due timed deadline no longer breaks out to the outer loop.** 0.3.2:
   `BackoffTimeoutDecision::DeadlineDue => { break; }` with the comment "If
   deadline is due or passed, don't park - break to process timers/tasks"
   (`three_lane.rs:4053–4056`). The outer loop then re-runs `next_task()`,
   whose documented step 1 is "Process expired timers (wakes tasks via their
   wakers)" (`three_lane.rs:3941`). 0.3.4 replaces the `break` with
   `park_timeout(STALE_DUE_DEADLINE_PARK_NANOS)` (= 1 ns, `:180`) and
   *stays in the inner loop* (`three_lane.rs:4357–4373`, then `:4380`
   "Continue loop to re-check condition (no break!)").
3. **The idle-loop break condition was narrowed.** 0.3.2 broke on
   `self.global.has_runnable_work(now) || !self.fast_queue.is_empty()`
   (`three_lane.rs:3997`). 0.3.4 breaks only on concrete runnable work —
   `!fast_queue.is_empty() || has_cancel_work() || has_ready_work()` — and
   routes a merely-due *timed* entry through the backoff budget instead
   (`three_lane.rs:4278–4300`).

What I can defend: this is a real, deliberate change to when a worker stops
looking for work, and 0.3.2's version re-entered the dispatch path far more
eagerly. What I could **not** establish by reading: a path where the worker
stays asleep *forever*. `inject_ready` still calls `wake_one()`
(`0.3.4/three_lane.rs:2310`), which unparks a parker and wakes the reactor
(`:735–747`); and `drive_io_phase` returns `Progress` (→ `continue` →
`next_task()`) whenever it blocked on a non-zero timeout, defaulting to
`IDLE_IO_POLL_MAX_TIMEOUT = 250 ms` when no deadline exists
(`0.3.4/three_lane.rs:4164–4196`, constant at `:182`) — and that function is
itself unchanged between versions. On paper that bounds the worker's blindness at
250 ms. So (A) is the best candidate and is **not proven**.

### (B) Injection now happens inside the `RuntimeState` critical section

0.3.2 `inject_ready` reads `wake_state.notify()` under `with_task_table_ref`,
**releases**, then enqueues and wakes (`three_lane.rs:2137–2160`). 0.3.4 moves
`inject_global_ready_checked(...)` — which enqueues, records evidence and calls
`wake_one()` (`:2281–2311`) — *inside* the closure
(`three_lane.rs:2325–2350`). Same restructuring for `inject_cancel`
(`:2209–2238`) and `inject_timed` (`:2244–2272`).

`with_task_table_ref` takes the `RuntimeState` mutex whenever the sharded task
table is absent (`three_lane.rs:2132–2143`), and the shipped default is
`RuntimeStateShape::Unified` (`0.3.4/src/runtime/config.rs:2213`;
`0.3.2/src/runtime/config.rs:2169`) — so on the default configuration this is a
new "hold the runtime-state lock across the global-queue push and the worker
unpark" ordering. I traced `should_throttle_spawns` (`:2114–2125`),
`record_scheduler_evidence_enqueue` (`:1995–2002`) and `wake_one` (`:735–747`)
and found **no** re-entrant `state` lock, so this is not a self-deadlock. It is a
new lock-order edge (state → global-injector, state → parker) whose partner I did
not find. Unproven.

### (C) The global FIFO backend was swapped

`FaaFifoQueue`/`faa_array_queue::FaaArrayQueue` → `GlobalFifoQueue`/
`crossbeam_queue::SegQueue` (`src/runtime/scheduler/global_queue.rs:6–105`,
0.3.2 vs 0.3.4). Upstream's stated reason is in the new doc comment: to avoid
"the pthread-key teardown hazards seen in `faa_array_queue`'s
`os-thread-local` dependency" (`0.3.4/global_queue.rs:71–76`). The enqueue/
dequeue semantics are a mechanical `enqueue→push` / `dequeue→pop` rename;
I found no semantic change. Noted for completeness, not as a suspect.

Related and more interesting: `inject_ready_contentious`'s combiner path gained a
double-check (`0.3.4/global_injector.rs:315–341`). 0.3.2 pushed the entry into
`pending` whenever `active` was set and returned — if the combiner had just gone
inactive, that entry was **stranded and never flushed**, i.e. a lost wakeup.
0.3.4 re-checks `active` after the push and falls back to a direct
`ready_queue.push` (`:336–345`). That is a *hang fix* in 0.3.4, and it only
engages under producer contention (`READY_COMBINER_IN_FLIGHT_THRESHOLD = 4`,
`global_injector.rs:12`), which a single-producer `block_on` thread does not
reach.

### (D) The runtime clock epoch moved

`RuntimeState::now` initialises to `Time::ZERO` in 0.3.2 (`state.rs:818`) and to
`Time::from_nanos(1_000_000_000)` in 0.3.4 (`state.rs:870`). Roughly forty test
assertions were re-based by the same second across `builder.rs`, `state.rs` and
`three_lane.rs`. `current_scheduler_time()` prefers the timer driver and falls
back to `state.now` (`0.3.4/three_lane.rs:5284–5293`), so the epoch is only
load-bearing where the two clocks meet. I did not find a place where the 1 s
offset produces an unbounded stall, but I also did not exhaust the timer path.
Recorded because a clock-epoch change is exactly the shape that turns "due" into
"not due" and it moved in the release that broke.

### Also changed, and inert

Spawn-authorization plumbing was added: `SecurityConfig`/
`spawn_authorization_key` (`0.3.4/config.rs:1886–1890`, `:2002`),
`SpawnError::AuthorizationDenied` (`state.rs:408–414`), and
`create_task_infrastructure` gained a `caller_cx: &Cx` parameter
(`state.rs:1866–1871`, vs `0.3.2/state.rs:1763–1768`). The parameter's first
statement is `let _ = caller_cx;` (`0.3.4/state.rs:1883`), so no check runs on
the `create_task`/`spawn_with_cx` path. Not a suspect; recorded so nobody else
spends time on it.

---

## 4. Guarantee or accident?

**Accident, decisively.** Four independent pieces of evidence:

1. Nothing in `block_on`'s contract promises it. Its doc comment promises only
   that "a thread-local `RuntimeHandle` is available … This allows futures inside
   `block_on` to spawn tasks onto the real scheduler" (0.3.2 `builder.rs:3233–
   3238`). Spawning onto the scheduler is the promise; *when a worker gets to it*
   is not.
2. The thing that actually ran the task in 0.3.2 was a worker-side idle
   heuristic — a local `backoff` counter and a `break` on a due deadline
   (`three_lane.rs:3978`, `:4053`) — which 0.3.4 rewrote for cost reasons, with
   tests asserting the *new* behaviour is the intended one
   (`0.3.4/three_lane.rs:7113–7155`). Depending on a heuristic's incidental
   eagerness is not depending on a guarantee.
3. `try_spawn_with_cx` returns `Result<(), SpawnError>` in **every** version
   examined — 0.3.2, 0.3.4, 0.3.9 (`builder.rs:3719`) and 0.3.10
   (`builder.rs:3719`). The API has never offered a way to await the task. The
   side channel exists because the API had no completion signal, and a pattern
   built to route around a missing completion signal cannot be relying on a
   documented one.
4. Upstream subsequently had to **add machinery** so that `block_on` cooperates
   with runtime tasks at all (§5). You do not add machinery to restore a
   guarantee that already held.

**Consequence for the fix: do not restore the old pattern.** Restoring it means
betting on a worker-side idle heuristic that upstream has already rewritten once
and left rewritten through 0.3.10 (`EMPTY_BACKOFF_PARK_THRESHOLD` and
`STALE_DUE_DEADLINE_PARK_NANOS` are still present at
`0.3.10/three_lane.rs:179–180`). Abandon it.

---

## 5. Did 0.3.9 / 0.3.10 restore or further change this? — yes, and it matters

The 0.3.4 scheduler changes were **not reverted**: 0.3.10 still carries the
persistent `empty_backoff` (`three_lane.rs:4430`), both new constants
(`:177–180`), and the inside-the-lock `inject_ready` shape (`:2510–2516`). So
"upgrade and the old pattern works again" is not on the table.

What upstream did instead is make `block_on` a real participant. Three additions,
all absent from 0.3.2 and 0.3.4:

**In 0.3.9 (`builder.rs`), `run_future_with_budget` was reworked:**
- `process_current_block_on_timers()` runs at the top of every loop iteration and
  again immediately before parking (`0.3.10/builder.rs:4437`, `:4474–4480`;
  0.3.9 `builder.rs:4411`). It processes the ambient Cx's timer driver and, if
  anything fired, calls `inner.scheduler.wake_all()`
  (`0.3.10/builder.rs:4388–4396`). The `block_on` thread now drives timers and
  kicks the scheduler.
- Parking became bounded. Where 0.3.2/0.3.4 did a bare `std::thread::park()`,
  0.3.9/0.3.10 park to the next timer deadline, else
  `park_timeout(BLOCK_ON_RUNTIME_RECHECK_INTERVAL)` (1 ms) whenever
  `current_runtime_has_live_tasks()` is true, else `park()`
  (`0.3.10/builder.rs:4482–4496`; constant `:4408`; predicate `:4410–4421`;
  0.3.9 `builder.rs:4382`, `:4384`, `:4464`).

**In 0.3.10 specifically — and this is the one that answers this bead:**
a spawn mailbox and producer gateway (`src/runtime/spawn_mailbox.rs`, a file that
does not exist in 0.3.2 or 0.3.4) are created unconditionally at build time
*"so the `Cx::spawn` surface works in every mode"*
(`0.3.10/builder.rs:4030–4047`), and `build_request_cx_from_inner` — the function
that builds **`block_on`'s ambient root Cx** (`builder.rs:3396`) — now attaches
them: `.with_spawn_gateway(spawn_gateway).with_pending_spawn_counter(pending_spawns)`
(`0.3.10/builder.rs:4566–4567`, values fetched at `:4521–4522`, `:4541`).
**0.3.9 creates the gateway on `RuntimeState` (`builder.rs:4047`) but never
attaches it to the request Cx** — `with_spawn_gateway` appears nowhere in
0.3.9's `builder.rs`. So the root-Cx wiring lands in 0.3.10 and not before.

That unlocks `Cx::spawn`, new in the 0.3.9/0.3.10 line and absent from 0.3.2
(`0.3.10/src/cx/cx.rs:4006`; `rg 'pub fn spawn' 0.3.2/src/cx/cx.rs` → no
matches):

```rust
pub fn spawn<F, Fut>(&self, f: F)
    -> Result<crate::runtime::TaskHandle<Fut::Output>, SpawnError>
where F: FnOnce(Cx<Caps>) -> Fut + Send + 'static,
      Fut: Future + Send + 'static, Fut::Output: Send + 'static
```

It hands the factory a child `Cx` **and** returns an awaitable
`TaskHandle<Fut::Output>` — precisely the two things `try_spawn_with_cx` could
not give at once, which is the whole reason the `std::sync::mpsc` side channel
exists. It returns `SpawnError::RuntimeUnavailable` when the Cx carries no
gateway (`cx.rs:4017–4023`), which is exactly the 0.3.9-and-earlier root Cx.

One honest limit: `build_request_cx_from_inner`'s own comment still says, in
0.3.10, that the request-scoped task "is still not registered in the runtime's
task arena (closing that gap requires a deeper structured-spawn refactor)"
(`0.3.10/builder.rs:4504–4510`). Issue #58's core is **mitigated, not closed**,
even at 0.3.10.

---

## 6. What I could not establish

I did not find a source-level path that hangs *forever* on 0.3.4, and I am not
going to invent one. Specifically unresolved:

- Whether the worker can miss `wake_one()` and then stay parked past
  `drive_io_phase`'s 250 ms idle re-poll. Everything I read says it re-checks;
  the symptom says otherwise.
- Whether the state→global-injector→parker lock ordering introduced by (B) has a
  partner edge somewhere I did not read (the reactor, `epoch_gc`, the deadline
  monitor).
- Whether the (D) clock-epoch move interacts with `timeout()`'s deadline
  arithmetic on the HTTP path.

Settling any of these needs an executed probe — a thread dump of one hanging test
under 0.3.4 (`lldb -p` / `sample`, or `RUST_LOG=trace` on the
`inject_ready: task injected` / `task NOT scheduled` trace lines at
`0.3.4/three_lane.rs:2367–2375`) would name the mechanism in minutes. This lane
was read-only and did not run one.

**The recommendation does not depend on resolving it.** §4 already settles that
the 0.3.2 behaviour was an accident, and §5 shows the API the pattern should be
replaced with.
