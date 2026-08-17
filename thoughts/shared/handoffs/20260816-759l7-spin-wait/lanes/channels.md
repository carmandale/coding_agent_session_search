# Lane: channels — runtime-aware synchronisation in asupersync 0.3.4

Read-only lane. Only write is this file. All line numbers are asupersync-0.3.4
under `/Users/dalecarman/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`
unless a version is named.

---

## CRUX ANSWER FIRST

**Swapping the `std::sync::mpsc` side channel for an asupersync channel does NOT
fix bead 759l7, and it makes the failure worse.** Three measured facts:

1. **The root future's waker is real and cross-thread.** `Runtime::block_on`
   (`src/runtime/builder.rs:3276`) delegates to `run_future_with_budget`
   (`:4135`), which builds a `ThreadWaker` (`:4118-4133`) whose `wake` sets a
   flag and calls `self.thread.unpark()` (`:4126-4128`). The loop parks at
   `:4184` when the poll returned `Pending` without a self-wake, and re-polls
   after any `unpark`. So if a spawned task calls `waker.wake()` on the waker it
   received from the root poll, the block_on thread IS woken. There is no
   lost-wakeup window: `woken` is cleared before the poll (`:4151-4153`), read
   with `Acquire` after (`:4158-4160`), and a `wake` landing after that read
   still leaves a sticky unpark token that makes `park()` at `:4184` return
   immediately.

2. **That wake path is byte-identical between the working 0.3.2 and the hanging
   0.3.4.** `diff` of `asupersync-0.3.2/src/runtime/builder.rs:4088-4142`
   against `asupersync-0.3.4/src/runtime/builder.rs:4135-4189` → IDENTICAL.
   `Runtime::block_on` is also textually identical (0.3.2 `:3239-3254` vs 0.3.4
   `:3276-3291`), and `RuntimeBuilder::current_thread()` is
   `Self::new().worker_threads(1)` in both (0.3.2 `:2937`, 0.3.4 `:2974`).
   `src/channel/mod.rs` is byte-identical between the two versions.
   **The variable that changed is not on the receive side.** Whatever regressed
   between 0.3.2 and 0.3.4 lives in the scheduler/worker path, which is the
   driver lane's question.

3. **`block_on` cannot drive the scheduler, so if the spawned task never runs,
   parking is permanent.** `run_future_with_budget` (`:4135-4189`) polls exactly
   one future — the root — and touches nothing else. Spawned tasks reach the
   scheduler via `inject_ready` (`spawn` at `:4010`, `spawn_with_cx` at `:4055`)
   and are only ever executed by OS worker threads created in
   `NativeThreadHostServices::spawn_worker_threads` (`:322-354`, real
   `std::thread::Builder::spawn` at `:343`). There is no inline drive-from-root
   path.

**Consequence.** Today `yield_now()` (`src/runtime/yield_now.rs:36`) calls
`cx.waker().wake_by_ref()` then returns `Pending` (`:27-28`), which makes
`run_future_with_budget` take the self-wake branch (`:4158-4177`): re-poll up to
`poll_budget` times, then sleep 1ms → 5ms → 25ms and repeat. The three sites are
therefore in a **slow backoff spin**, not a park — they burn a little CPU
forever and are interruptible-looking. Replace the channel with any of the
waker-registering primitives below and the root future returns `Pending` with no
self-wake, so `run_future_with_budget` calls `std::thread::park()` at `:4184`
and the process blocks with zero CPU and no timeout. **Spin → silent hang.** A
channel swap is only correct once the driver lane has established that the
spawned task actually gets polled on 0.3.4.

**The one change that removes the dependency entirely:** don't spawn. All three
sites already have a Cx-taking async fn and `block_on` already installs a
fully-capable ambient Cx — see §3 and the `update_check.rs:858-860` fallback,
which is that exact shape and is already in the tree.

---

## 1. Inventory — public channel / sync types in 0.3.4

Module declarations: `src/lib.rs:172` (`pub mod channel`), `:208` (`pub mod
sync`). Public surface: `src/channel/mod.rs:41-50, 104-105` and
`src/sync/mod.rs:61-107`.

Legend for **Receive path**: *waker* = registers `Context::waker()` and is woken
by the producer; *poll* = returns immediately, caller must loop.

### `channel::oneshot` — `src/channel/oneshot.rs`

| Item | Site | Cx needed? | Receive path |
|---|---|---|---|
| `oneshot::channel<T>() -> (Sender<T>, Receiver<T>)` | `:293` | no | — |
| `Sender::reserve(self, cx) -> Result<SendPermit<T>, _>` | `:338` | **yes** | — |
| `Sender::send(self, cx, value)` | `:386` | **yes** | — |
| `Sender::send_blocking(self, value)` | `:408` | **no** | — |
| `SendPermit::send(self, value)` | `:516` | no | — |
| `SendPermit::abort(self)` | `:553` | no | — |
| `Receiver::recv(&mut self, cx) -> RecvFuture` | `:859` | **yes** | **waker** |
| `Receiver::try_recv(&mut self)` | `:889` | no | poll |
| `Receiver::poll_closed`, `Sender::poll_closed` | `:936`, `:448` | no | waker |

`RecvFuture::poll` (`:728-808`) checks value (`:742`), closed (`:755`),
cancellation (`:765`), then registers `ctx.waker().clone()` (`:802`) and returns
`Pending`. The producer wakes it: `SendPermit::send` takes the waker under the
lock and wakes outside it (`:534, :540-542`); `Sender::drop` (`:463-486`) and
`SendPermit::drop` (`:586-605`) also wake, so a task that dies without sending
resolves the receiver with `RecvError::Closed` rather than hanging.
`send_blocking` (`:408-420`) is the notable one: it commits through the same
`SendPermit::send` and therefore does the same wake, with **no `Cx` at all** —
its doc comment explicitly says it is a sync bridge that cannot deadlock a
worker.

### `channel::mpsc` — `src/channel/mpsc.rs`

| Item | Site | Cx needed? | Receive path |
|---|---|---|---|
| `mpsc::channel<T>(capacity) -> (Sender<T>, Receiver<T>)` | `:348` | no | — |
| `Sender::reserve(&self, cx) -> Reserve` | `:375` | **yes** | — |
| `Sender::send(&self, cx, value).await` | `:385` | **yes** | — |
| `Sender::try_reserve(&self)` | `:399` | no | — |
| `Sender::try_send(&self, value)` | `:446` | **no** | — |
| `Sender::wake_receiver(&self)` | `:481` | no | — |
| `Receiver::recv(&mut self, cx) -> Recv` | `:981` | **yes** | **waker** |
| `Receiver::poll_recv(&mut self, cx, task_cx)` | `:994` | **yes** | **waker** |
| `Receiver::try_recv(&mut self)` | `:1036` | no | poll |

`poll_recv` (`:994-1032`) pops (`:1009`), reports `Disconnected` when
`sender_count == 0` (`:1019-1024`), else registers `task_cx.waker().clone()`
(`:1027-1030`). `try_send` (`:446-467`) pushes and wakes the receiver waker
outside the lock (`:461-465`) with **no `Cx`**. `Drop for Sender` (`:763-780`)
wakes the receiver when the last sender goes (`:767-778`), so a dropped/panicked
producer surfaces as `RecvError::Disconnected`.

`capacity` must be non-zero — `channel(0)` panics (`:349`).

### `channel::watch` — `src/channel/watch.rs`

| Item | Site | Cx needed? | Receive path |
|---|---|---|---|
| `watch::channel<T>(initial)` | `:347` | no | — |
| `Sender::send(&self, value)` | `:425` | **no** | — |
| `Sender::send_modify(&self, f)` | `:460` | no | — |
| `Sender::subscribe(&self)` | `:509` | no | — |
| `Receiver::changed(&mut self, cx) -> ChangedFuture` | `:599` | **yes** | **waker** |
| `Receiver::borrow` / `borrow_and_update` | `:704`, `:717` | no | poll |
| `Receiver::has_changed(&self)` | `:770` | no | poll |

`send` bumps the version and calls `wake_all_waiters` (`:437-439`), which drains
the waiter list and wakes each (`:280-289`). `poll_changed` (`:608`) registers
`context.waker().clone()` (`:643-644`, `:650-651`, `:658-659`) and re-checks
after registering to close the race (`:666`).

### `channel::broadcast` — `src/channel/broadcast.rs`

| Item | Site | Cx needed? | Receive path |
|---|---|---|---|
| `broadcast::channel<T: Clone>(capacity)` | `:190` | no | — |
| `Sender::reserve(&self, cx)` | `:332` | **yes** | — |
| `Sender::send(&self, cx, msg) -> Result<usize, _>` | `:363` | **yes** | — |
| `SendPermit::send(self, msg) -> usize` | `:473` | no | — |
| `Receiver::recv(&mut self, cx) -> Recv` | `:602` | **yes** | **waker** |
| `Receiver::try_recv(&mut self)` | `:563` | no | poll |

Requires `T: Clone`. The send path needs a `Cx` unless you hold a `SendPermit`,
which itself came from a `Cx`-taking `reserve`.

### `channel::session` (obligation-tracked wrappers) — `src/channel/session.rs`

`tracked_channel<T>(capacity) -> (TrackedSender<T>, mpsc::Receiver<T>)` (`:363`)
and `tracked_oneshot<T>() -> (TrackedOneshotSender<T>, oneshot::Receiver<T>)`
(`:508`). Both re-exported at `src/channel/mod.rs:105`. These wrap the two
channels above; the receivers are the plain `mpsc`/`oneshot` receivers, so the
receive semantics are unchanged. Both senders' `reserve` takes a `Cx`
(`:217-224`, `:398`) and the permits panic on drop if neither committed nor
aborted — **strictly worse ergonomics here than plain `oneshot`**, since a
cancelled spawn would panic rather than close the channel.

### `sync::Notify` — `src/sync/notify.rs`

| Item | Site | Cx needed? | Path |
|---|---|---|---|
| `Notify::new()` | `:230` | no | — |
| `Notify::notified(&self) -> Notified` | `:272` | **no** | **waker** |
| `Notify::wait_until(&self, predicate).await` | `:319` | **no** | waker + re-check |
| `Notify::notify_one(&self) -> bool` | `:338` | **no** | — |
| `Notify::notify_waiters(&self)` | `:373` | **no** | — |
| `Notify::waiter_count(&self)` | `:415` | no | poll |

The only primitive in either module whose **wait side needs no `Cx`**.
`Notified::poll` (`:690-702`) → `poll_init` registers `cx.waker().clone()`
(`:607`); `notify_one` wakes the next active waiter outside the lock
(`:342-344, :360-362`) and, when there is no waiter, stores the notification
under the waiter lock so it cannot be lost (`:352`, with the comment naming the
lost-wakeup race at `:347-351`). Pairs with an `AtomicBool`/`Mutex<Option<T>>`
for the payload; the crate's own doc example at `:290-317` is exactly the
"other thread sets state, then notifies" shape.

### `sync::Semaphore` — `src/sync/semaphore.rs`

`Semaphore::new(permits)` `:259`; `acquire(&self, cx, count) -> AcquireFuture`
`:335` (**Cx required**, registers `context.waker()` — see the poll body around
`:469-600`, waker clones at `+113/+120/+127` from `:469`); `try_acquire(&self,
count)` `:351` (poll); `add_permits(&self, count)` `:422` (**no Cx**, wakes the
front waiter at `:436-440`); `close(&self)` `:316` (**no Cx**, wakes all at
`:328-330`). `Semaphore::new(0)` + `add_permits(1)` from the producer is a valid
signal, but the waiter needs a `Cx`.

### `sync::OnceCell` — `src/sync/once_cell.rs`

`new()` `:130`; `set(&self, value) -> Result<(), T>` `:208` (**no Cx**);
`get(&self)` `:165`; `wait(&self, cx)` `:477` (**Cx required**, async);
`get_or_init(&self, f).await` `:298`; `get_or_init_blocking` `:235` (**blocks
the calling OS thread** — never call this from the block_on root).
`set` → `transition_out_of_initializing` (`:509-522`) drains and wakes every
registered waker (`:516, :519-521`). Caveat: `set` returns `Err(value)` if the
cell is already `INITIALIZED` **or** currently `INITIALIZING` (`:222`).

### Remaining `sync` exports (not signalling primitives; listed for completeness)

- `Mutex` — `new` `:155`, `lock(&self, cx)` `:182` (**Cx**, waker),
  `try_lock` `:238` (poll). `OwnedMutexGuard::lock(Arc<Mutex<T>>, cx)` `:737`.
- `RwLock` — `new` `:232`, `read(&self, cx)` `:263`, `write(&self, cx)` `:284`
  (**Cx**, waker), `try_read`/`try_write` `:274`/`:296` (poll).
- `Barrier` — `new(parties)` `:64`, `wait(&self, cx)` `:120` (**Cx**, waker).
  N-way rendezvous; needs every party to arrive, so it is the wrong shape for
  one producer + one consumer.
- `Pool` / `GenericPool` — trait `acquire<'a>(&'a self, cx: &'a Cx)`
  `src/sync/pool.rs:252` (**Cx**), `try_acquire` `:260`.
  `PoolReturnSender<R>` / `PoolReturnReceiver<R>` (`:226`, `:229`) are **type
  aliases for `channel::mpsc::Sender`/`Receiver`**, not distinct primitives.
- `ContendedMutex` — `new(name, value)` `src/sync/contended_mutex.rs:197`,
  `lock(&self)` `:208`. This is a **blocking** `std`-style mutex, not async.
  Calling `lock()` from the block_on root blocks the whole thread.

### Version stability of the useful surface

`oneshot::Sender::send_blocking` exists in 0.3.2 (`:408`), 0.3.4 (`:408`),
0.3.9 (`:421`), 0.3.10 (`:496`). `mpsc::Sender::try_send` and `wake_receiver`
exist in 0.3.2 (`:398`, `:433`), 0.3.4 (`:446`, `:481`), 0.3.9 (`:559`, `:594`),
0.3.10 (`:626`, `:661`). `Notify::{new, notified, notify_one}` exist in 0.3.2
(`:230, :272, :338`) and 0.3.4 (same lines). Nothing recommended here is new in
0.3.4 or removed by 0.3.10.

---

## 2. Would the root future be woken? — per type, with the dependency stated

Common mechanism, established once: the waker handed to the root future by
`run_future_with_budget` (`src/runtime/builder.rs:4141-4142`) is a `ThreadWaker`
whose `wake`/`wake_by_ref` call `unpark()` on the block_on thread
(`:4124-4133`). Spawned tasks run on separate OS worker threads
(`:322-354`; `current_thread()` still spawns one, `:2974-2976`). So a producer
calling `waker.wake()` from a spawned task is a genuine cross-thread unpark.

| Primitive | Wakes the block_on root? | Producer needs a Cx? |
|---|---|---|
| `oneshot` `recv` ← `send_blocking` / `SendPermit::send` | **yes**, `oneshot.rs:540-542` → `ThreadWaker::wake` | **no** (`:408`) |
| `oneshot` `recv` ← `Sender::drop` | yes, `:479-481` (→ `RecvError::Closed`) | no |
| `mpsc` `recv` ← `try_send` | **yes**, `mpsc.rs:463-465` | **no** (`:446`) |
| `mpsc` `recv` ← last `Sender::drop` | yes, `:775-777` (→ `Disconnected`) | no |
| `watch` `changed` ← `send` | yes, `watch.rs:437-439` → `:285-288` | **no** (`:425`) |
| `broadcast` `recv` ← `send` | yes | **yes** (`broadcast.rs:363`) |
| `Notify::notified` ← `notify_one` | **yes**, `notify.rs:360-362` | **no** (`:338`) |
| `Semaphore::acquire` ← `add_permits` | yes, `semaphore.rs:436-440` | no for the producer; **yes for the waiter** (`:335`) |
| `OnceCell::wait` ← `set` | yes, `once_cell.rs:519-521` | no for `set` (`:208`); **yes for `wait`** (`:477`) |
| `Mutex`/`RwLock`/`Barrier`/`Pool` | yes on release | Cx on the wait side |

**All of that is conditional on the spawned task being polled at all.** The
whole table assumes `waker.wake()` executes, which requires the spawned future
to reach the point where it sends. If the driver lane confirms asupersync #58 —
that the root `Cx` is absent from `state.tasks` and the injected task is never
picked up — then none of these producers ever runs, no wake is ever issued, and
the root future sits in `std::thread::park()` at `builder.rs:4184` forever. That
is strictly worse than today's 1/5/25ms backoff spin, because a parked process
looks idle and is indistinguishable from a hang with no CPU signal.

I did **not** establish whether the spawned task runs on 0.3.4. What I did
establish is that the mechanism the fix would depend on is unchanged between the
version that works and the version that does not (§CRUX fact 2), so the channel
is not where the regression lives, and swapping it is not a fix on its own.

Ranking, **if and only if** the driver lane proves the task is polled:

1. `oneshot::channel()` + `Sender::send_blocking(result)` in the task,
   `rx.recv(&cx).await` in the root. Closest to the current shape, exactly one
   value, producer needs no `Cx`, and a task that dies without sending closes
   the channel instead of hanging. `Cx` for the root comes from
   `Cx::current()` (§3).
2. `mpsc::channel(1)` + `try_send`. Same properties; only preferable if more
   than one value or more than one producer is ever wanted.
3. `Notify` + `Mutex<Option<T>>`. The only option needing no `Cx` on either
   side, at the cost of hand-rolling the payload slot.

Not recommended: `broadcast` (needs a `Cx` to send and `T: Clone`), `Barrier`
(wrong shape), `tracked_*` (permit panics on drop), `Semaphore` (payload still
needs a side slot, and the waiter needs a `Cx` anyway).

---

## 3. Is there a `futures::executor::block_on` or a `LocalSet` equivalent?

**No `LocalSet`.** `rg 'LocalSet|local_set'` over `asupersync-0.3.4/src/` returns
only HTTP `local_settings` fields — nothing task-related.
`src/runtime/local.rs` is thread-local *storage* backing a `spawn_local` that
has **no public constructor**: `rg 'pub fn spawn_local'` over
`src/runtime/*.rs` returns nothing, and `src/runtime/mod.rs:177-236` does not
export one.

**One public `block_on` in the whole crate:** `Runtime::block_on`
(`src/runtime/builder.rs:3276`). `rg 'pub fn block_on'` over `src/` returns that
single hit. `block_on_with_cx` (`:3299`) and `block_on_current_with_cx`
(`:3318`) are `pub(crate)`. `Cx::set_current` is `pub(crate)`
(`src/cx/cx.rs:465`), so a caller cannot install a `Cx` and hand-roll an
executor through the supported API.

**But the supported way to run a Cx-taking future to completion on the current
thread already exists, and it is what these three sites should use.**
`Runtime::block_on` itself installs a fully-capable ambient `Cx` before polling:

```
let request_cx = self.request_cx_with_budget(Budget::INFINITE);   // :3288
let _cx_guard = crate::cx::Cx::set_current(Some(request_cx));     // :3289
run_future_with_budget(future, self.inner.config.poll_budget)     // :3290
```

That `Cx` is built by `build_request_cx_from_inner` (`:4191-4245`), which wires
the real `io_driver`, `timer_driver`, `blocking_pool`, `entropy`, logical clock
and trace handles (`:4214-4243`) — so it is not a stub, it can do HTTP.
`Cx::current()` is **public** (`src/cx/cx.rs:361`) and returns the innermost
installed context. Therefore, inside `runtime.block_on(async { … })`, this is
legal and needs neither a spawn nor a channel:

```rust
let cx = asupersync::Cx::current().context("requires an active asupersync Cx")?;
do_work_with_cx(&cx).await
```

**This exact code is already in the tree** as the non-spawn fallback at
`src/update_check.rs:858-860`, immediately below the spinning loop. The spawn
path is only taken when `Runtime::current_handle()` is `Some`
(`src/update_check.rs:840`) — i.e. precisely when `block_on` has already
installed both the handle and the ambient `Cx`. So on every one of the three
defect sites the spawn branch is reachable only in the situation where the
already-written fallback would work.

Two caveats on that fallback shape:
- `Cx::detached_cancel_context()` (`src/cx/cx.rs:3190`) is public but returns
  `Cx<cap::None>` with an empty capability mask (`:3196-3198`) — no I/O, no
  timer. It cannot substitute for the ambient `Cx` in HTTP work.
- The awaited future then runs **on the block_on thread**, inside
  `run_future_with_budget`. Its I/O must wait through the reactor via the `Cx`,
  which is what `block_on`'s own comment at `:3278-3287` says the ambient `Cx`
  exists to enable.

`futures-lite` 2.6 is a **normal (non-dev) dependency** of asupersync
(`asupersync-0.3.4/Cargo.toml:270-271`), and the crate's own doctests use
`futures_lite::future::block_on` (e.g. `src/sync/notify.rs:297`). asupersync does
**not** re-export it (`rg 'pub use futures_lite'` over `src/` → no hits), so cass
would have to take its own direct dependency. That would give a current-thread
executor, but it would **not** install an ambient `Cx` and would not drive the
asupersync scheduler either, so it does not solve anything the `Cx::current()`
path does not already solve.

---

## Null results / not established

- I did not establish whether a task injected by `try_spawn_with_cx` is actually
  polled on 0.3.4. Driver lane owns it. Every recommendation above is
  conditional on it.
- I did not diff the *bodies* of `oneshot.rs` / `mpsc.rs` / `notify.rs` between
  0.3.2 and 0.3.4 line by line; `diff -q` reports they differ. I verified only
  that the public items named in this report exist at the cited lines in 0.3.4
  and exist (at other lines) in 0.3.2, 0.3.9 and 0.3.10.
- I did not verify the `Semaphore::acquire` waker-registration lines against
  absolute file offsets; the citations `:469-600` are the `AcquireFuture::poll`
  body and the waker clones sit at relative offsets +113/+120/+127 from `:469`.
