# Lane: Cx acquisition surface — asupersync 0.3.4

Bead 759l7. READ-ONLY lane. No build, no test, no edits outside this file.

All `file:line` citations are relative to
`/Users/dalecarman/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/asupersync-0.3.4/src/`
unless prefixed with `cass:` (repo-relative) or `0.3.2:`.

Confirmed build config: `Cargo.toml:26` requests
`features = ["test-internals", "tls-native-roots"]`; `Cargo.lock:323-324` resolves
asupersync to **0.3.4**. So `test-internals` IS compiled in, and every
`#[cfg(any(test, feature = "test-internals"))]` / `visibility::make(pub)` item below
is genuinely reachable from cass.

---

## 0. What "a usable Cx for issuing HTTP requests" actually means

This turned out to be two separate requirements, satisfied by two different objects.
Getting this wrong is the difference between "works" and "spins".

**Requirement 1 — a `&Cx` value to pass as an argument.**
`HttpClient::request` is `http/h1/http_client.rs:776`, taking `cx: &Cx`. The only
thing it does with it up front is `check_cx(cx)?` at `:783`. `check_cx` is
`http/h1/http_client.rs:144` and its entire body is `cx.checkpoint().is_err()` →
`ClientError::Cancelled`. It reads **no capability**. The cass call sites also use
`cx.now()` for the timeout deadline (`cass:src/update_check.rs:869`).

So the argument `Cx` needs: a working `checkpoint()` (i.e. not cancelled, budget not
exhausted) and a time source. It does **not** need an IO capability.

Note the type: `Cx` with no parameter is `Cx<cap::All>` — the struct is declared
`pub struct Cx<Caps = cap::All>` at `cx/cx.rs:181`, and `lib.rs:384` re-exports
`pub use cx::{Cx, Scope}`. `HttpClient::request` names the bare `Cx`, so it takes
`Cx<cap::All>` specifically. Anything narrower (e.g. `Cx<cap::None>`) will not
type-check at the call site.

**Requirement 2 — an *ambient* `Cx::current()` carrying the runtime's IO driver, on
whatever thread polls the future.**

The socket path never sees the `cx` argument. `HttpClient` connects through
`TcpStream::connect(addr)` (`http/h1/http_client.rs:1448`, and
`connect_socket_addr` at `:1443`). `TcpStream::connect` is `net/tcp/stream.rs:192`
and **takes no `Cx` parameter at all**. Registration happens in `wait_for_connect`,
`net/tcp/stream.rs:681`, whose first line is:

```rust
let Some(driver) = Cx::current().and_then(|cx| cx.io_driver_handle()) else {
    wait_for_connect_fallback(socket).await?;
    return Ok(None);
};
```
(`net/tcp/stream.rs:682-685`)

If the ambient lookup misses, it degrades to `wait_for_connect_fallback`
(`net/tcp/stream.rs:766`), which is a `fallback_rewake` busy-loop — correct, but a
hot spin. The same ambient-lookup shape recurs at `net/happy_eyeballs.rs:511`
(`timeout_now`), `net/dns/resolver.rs:637`, `time/sleep.rs:91`, `:485`,
`time/timeout_future.rs:231`, `net/tcp/split.rs:472`, `net/udp.rs:1385`.

`Cx::io_driver_handle` is `pub(crate)` at `cx/cx.rs:721` and carries **no**
`visibility::make(pub)` attribute, so external code cannot even observe whether a
`Cx` it holds satisfies requirement 2. The only way to satisfy it is to be running
somewhere the runtime itself installed a driver-backed ambient Cx.

**Consequence for this bead:** the `Cx` handed to the closure by
`try_spawn_with_cx` satisfies requirement 1. It satisfies requirement 2 only
incidentally — because the *same* Cx is also installed ambiently by the worker that
polls the task (see §2). Passing a Cx across threads and using it as a function
argument alone would give you HTTP that connects on the fallback spin path.

---

## 1. The ambient store, read directly

`Cx::current()` — `cx/cx.rs:361`, `pub fn current() -> Option<Self>` on
`impl FullCx` (`FullCx = Cx<cap::All>`, aliased at `cx/cx.rs:290`).

It consults exactly one thing: a thread-local stack.

```rust
thread_local! {
    static CURRENT_CX_STACK: RefCell<Vec<CurrentCxFrame>> = const { RefCell::new(Vec::new()) };
}
```
(`cx/cx.rs:309-313`; frame type `CurrentCxFrame { cx: FullCx, mask: cap::CapMask }`
at `cx/cx.rs:303-306`.)

`current()` returns `stack.last()`, cloned, with the frame's `mask` written over
`cx.runtime_mask` (`cx/cx.rs:362-371`). Empty stack → `None`. Thread-local teardown
→ `None` via `.unwrap_or(None)`.

**It is a *thread*-local, not a task-local.** There is no task-local storage
anywhere in this path. Whether `Cx::current()` returns `Some` is purely a question
of whether *the thread currently executing this poll* has a frame pushed. Two
neighbours worth knowing:

- `Cx::is_active()` — `cx/cx.rs:384` — `pub`, existence check without the 3 `Arc`
  clones.
- `Cx::with_current<F, R>(f) -> Option<R>` — `cx/cx.rs:426` — `pub`, borrows the
  frame instead of cloning. Closure is **not** invoked when the stack is empty. Note
  its documented restriction (`cx/cx.rs:411-417`): the `RefCell` borrow is held for
  the whole closure body, so you cannot call `set_current*` inside it, and the `&Cx`
  cannot be moved into an async block.

**Who pushes frames** (exhaustive for the runtime paths that matter):

| pusher | file:line |
|---|---|
| `Runtime::block_on` | `runtime/builder.rs:3289` |
| `Runtime::block_on_with_cx` (`pub(crate)`) | `runtime/builder.rs:3305` |
| `Runtime::block_on_current_with_cx` (`pub(crate)`) | `runtime/builder.rs:3324` |
| worker poll loop | `runtime/scheduler/worker.rs:430` |
| three-lane poll loop | `runtime/scheduler/three_lane.rs:6049` |
| blocking pool | `runtime/spawn_blocking.rs:416`, `:441` |

---

## 2. Settling the three situations

### (a) Inside the root future of `Runtime::block_on` on `RuntimeBuilder::current_thread()`

**`Cx::current()` returns `Some`, with a full-capability, driver-backed Cx.**

This contradicts the brief's framing (which implied the root Cx is not usable) and it
contradicts nothing in issue #58 — #58 is about the root Cx not being *registered in
`state.tasks`*, which is a different claim from *not installed ambiently*.

`Runtime::block_on` is `runtime/builder.rs:3276`:

```rust
let _guard = ScopedRuntimeHandle::new(self.handle());          // :3277
...
let request_cx = self.request_cx_with_budget(Budget::INFINITE); // :3288
let _cx_guard = crate::cx::Cx::set_current(Some(request_cx));   // :3289
run_future_with_budget(future, self.inner.config.poll_budget)   // :3290
```

The in-source comment at `:3279-3287` states the intent explicitly: "install an
ambient Cx backed by this runtime's drivers (IO + timer + blocking pool +
observability). Without it, `Cx::current()` returns None inside the polled future,
so public async networking APIs … fall back to a tight `accept4` / `WouldBlock` poll
instead of waiting through the configured reactor."

The Cx is built by `build_request_cx_from_inner` (`runtime/builder.rs:4191`), which
supplies `guard.io_driver_handle()`, `timer_driver`, `blocking_pool`, `entropy`,
`observability`, `logical_clock`, trace buffer (`:4225-4243`). `Cx::new_with_drivers`
sets `runtime_mask: cap::CapMask::all()` unconditionally (`cx/cx.rs:712`), and
`cap::All = CapSet<true,true,true,true,true>` (`cx/cap.rs:99`) whose `MASK` is every
bit (`cx/cap.rs:185-193`). Budget is `Budget::INFINITE`.

The guard is `let _cx_guard`, so it lives to the end of the function body — across
every poll. I checked that the poll driver does not disturb the stack:
`run_future_with_budget` is `runtime/builder.rs:4135` and its body (`:4135-4188`)
touches only a `ThreadWaker`, a poll counter, `thread::sleep` and `thread::park`. No
`CURRENT_CX_STACK` access.

One caveat, stated plainly: `build_request_cx_from_inner` carries its own comment
(`runtime/builder.rs:4192-4198`) noting "The task is still not registered in
`state.tasks` — that's a deeper refactor". So this Cx is fine as an *effect
capability* and is not visible to oracles / deadline monitor / futurelock detection.
That is exactly the #58 residue, and it does not impair HTTP.

`Runtime::current_handle()` is also `Some` here, installed at `:3277`.

### (b) Inside an `async fn` where `Runtime::current_handle()` returns `Some`

**Not settleable in general. `current_handle()` being `Some` does not imply
`Cx::current()` is `Some` — they are two independent thread-locals.**

`CURRENT_RUNTIME_HANDLE` is declared at `runtime/builder.rs:207`; `current_handle()`
reads it at `runtime/builder.rs:3364`. `CURRENT_CX_STACK` is `cx/cx.rs:313`. Nothing
couples them.

What I *can* establish is that on every thread where the runtime installs one, it
installs both, so they co-occur in practice:

- block_on thread — handle at `runtime/builder.rs:3277`, cx at `:3289`.
- worker threads — handle at `runtime/builder.rs:345`
  (`let _guard = ScopedRuntimeHandle::new(runtime_handle);` inside the spawned
  thread closure), cx per-poll at `runtime/scheduler/worker.rs:430`.

Known divergence in the other direction: `runtime/spawn_blocking.rs:416` and `:441`
push a Cx onto blocking-pool threads; I found no `ScopedRuntimeHandle` installation
on those threads, so there `Cx::current()` is `Some` while `current_handle()` is
likely `None`. I did not chase that to a conclusion because it is out of scope for
these three call sites.

**Verdict for the cass fallback path.** `cass:src/update_check.rs:859` does
`asupersync::Cx::current().context("update check requires an active asupersync Cx")?`
in the branch where `current_handle()` was `None`. That is the correct API and the
only correct API — but the two conditions are independent, so the branch is only
reachable in an ambient-Cx-without-handle configuration. Nothing is *wrong* with it;
it is simply not the guarantee its structure implies.

### (c) Inside a task spawned via `try_spawn` (bare future, no Cx passed in)

**`Cx::current()` returns `Some`, and the Cx is the task's own runtime-created,
driver-backed, full-capability Cx. This is the headline finding of this lane and it
contradicts the brief's stated reason for the side channel.**

The brief says `try_spawn` "takes a bare future, so the task gets no `Cx`". The first
half is true; the second is false. Trace:

1. `RuntimeHandle::try_spawn` — `runtime/builder.rs:3557` — → `RuntimeInner::spawn`
   (`runtime/builder.rs:3986`).
2. `RuntimeInner::spawn` calls `guard.create_task(self.root_region, Budget::new(), wrapped)`
   at `runtime/builder.rs:4006`.
3. `RuntimeState::create_task` — `runtime/state.rs:2042` — mints a system Cx
   (`create_system_cx`, `runtime/state.rs:1853`) and delegates to
   `create_task_infrastructure` at `runtime/state.rs:2053-2055`.
4. `create_task_infrastructure` — `runtime/state.rs:1866` — builds the task's Cx with
   the full driver set at `runtime/state.rs:1966-1983`
   (`Cx::new_with_drivers(region, task_id, budget, observability,
   self.io_driver_handle(), None, self.timer_driver_handle(), Some(entropy))`
   `.with_blocking_pool_handle(...)` `.with_logical_clock(...)`), then:

   ```rust
   self.update_task(task_id, |record| {
       record.set_cx_inner(cx.inner.clone());
       record.set_cx(cx.clone());        // runtime/state.rs:1991
   });
   ```

   `TaskRecord.cx` is `pub cx: Option<Cx>` (`record/task.rs:336`); `set_cx` is
   `record/task.rs:462`.
5. The scheduler reads it back and installs it for the duration of the poll:
   `let task_cx = record.cx.clone();` at `runtime/scheduler/worker.rs:361` and `:379`
   (the source comment reads "Preserve full Cx so scheduler sets CURRENT_CX during
   poll", `runtime/scheduler/three_lane.rs:5875`, `:5916`), then
   `let _cx_guard = crate::cx::Cx::set_current(task_cx);` at
   `runtime/scheduler/worker.rs:430` and `runtime/scheduler/three_lane.rs:6049`.

`create_task_infrastructure` is shared by **both** `create_task` (the `try_spawn`
path, `runtime/state.rs:2053`) and `RuntimeInner::spawn_with_cx` (the
`try_spawn_with_cx` path, `runtime/builder.rs:4034`). The Cx is created identically
in both cases. The **only** difference is that `spawn_with_cx` hands the value to the
user's factory closure (`let future = f(cx);` at `runtime/builder.rs:4042`) whereas
`spawn` keeps it and moves on. Both store it in the record; both therefore get it
installed ambiently by the worker.

Budget is not a caveat here: `try_spawn` passes `Budget::new()`
(`runtime/builder.rs:4006`), and `Budget::new()` is `types/budget.rs:193` with
`poll_quota: u32::MAX` (`types/budget.rs:196`).

**Practical upshot.** `try_spawn(async { let cx = Cx::current().expect(...); ... })`
gives you the same capability surface as `try_spawn_with_cx`, *and* returns
`JoinHandle<F::Output>` which `impl Future` (`runtime/builder.rs:3679` / `:3686`),
which is directly awaitable. That removes the reason for the `std::sync::mpsc` +
`yield_now` side channel. I have not run this — it is a source-derived claim and it
needs the falsifier lane to execute it.

**Contradiction worth flagging to the coordinator:** `RuntimeBuilder::current_thread()`
is `runtime/builder.rs:2974` and is literally `Self::new().worker_threads(1)`
(`:2975`) — it is *not* a "no worker threads, block_on drives everything" preset.
`spawn_worker_threads` (`runtime/builder.rs:322`) returns early only when
`config.worker_threads == 0` (`:327-329`) and otherwise spawns a real OS thread per
worker (`:339-356`). So on these three call sites there IS one worker OS thread that
should be draining the ready queue. The recorded hypothesis ("the spawned task is
never polled") does not follow from the block_on code path alone. I am not
adjudicating the hang — that is another lane — but whoever owns it should not treat
"root Cx not in `state.tasks`" as sufficient explanation without checking whether the
worker thread is running and whether `inject_ready` (`runtime/builder.rs:4054`)
reaches it.

---

## 3. Enumeration: every public API that yields or installs a `Cx`

Legend: **(a)** root future of `block_on`; **(b)** async fn with `current_handle()`
`Some`; **(c)** inside a `try_spawn`ed bare future. "HTTP-usable" = satisfies
requirement 1 *and* requirement 2 from §0.

### 3.1 Genuinely public, ungated — the production surface

| API | signature | file:line | (a) | (b) | (c) | HTTP-usable |
|---|---|---|---|---|---|---|
| `Cx::current` | `pub fn current() -> Option<Self>` (on `Cx<cap::All>`) | `cx/cx.rs:361` | **Some** | see §2(b) | **Some** | **Yes** — this is the one |
| `Cx::is_active` | `pub fn is_active() -> bool` | `cx/cx.rs:384` | true | — | true | n/a (probe only) |
| `Cx::with_current` | `pub fn with_current<F, R>(f: F) -> Option<R> where F: FnOnce(&Self) -> R` | `cx/cx.rs:426` | invokes | — | invokes | Yes, but `&Cx` cannot cross an await (`cx/cx.rs:411-417`) — unusable for `client.request(cx, …).await` |
| `Cx::set_current_restricted` | `pub fn set_current_restricted(self) -> CurrentCxGuard` where `Caps: cap::CapSetRuntimeMask` | `cx/cx.rs:508` | installs | installs | installs | **Installer, not a source.** See §3.2 |
| `Cx::push_restriction` | `pub fn push_restriction(mask: cap::CapMask) -> CurrentCxGuard` | `cx/cx.rs:543` | narrows | narrows | narrows | No — only narrows (intersects with top, `cx/cx.rs:557-559`) |
| `Cx::detached_cancel_context` | `pub fn detached_cancel_context() -> Self` on `impl Cx<cap::None>` | `cx/cx.rs:3190` | works | works | works | **No.** Returns `Cx<cap::None>` with `runtime_mask = CapMask::none()` (`cx/cx.rs:3196`). Will not type-check against `HttpClient::request`'s `&Cx` = `&Cx<cap::All>` |
| `Cx::restrict` | `pub fn restrict<NewCaps>(&self) -> Cx<NewCaps>` | `cx/cx.rs:751` | — | — | — | No — narrows an existing Cx; not a source |
| `Cx::scope` | `pub fn scope(&self) -> Scope<'static>` | `cx/cx.rs:3061` | works | works | works | See §3.4 — the `Scope` it returns is unusable |
| `Runtime::current_handle` | `pub fn current_handle() -> Option<RuntimeHandle>` | `runtime/builder.rs:3364` | Some | Some | Some | Yields a handle, **not** a Cx |

`Cx::attenuate` (`cx/cx.rs:1000`), `attenuate_time_limit` (`:1038`),
`attenuate_scope` (`:1050`), `attenuate_rate_limit` (`:1065`),
`attenuate_from_budget` (`:1079`) all return `Option<Self>` but require an existing
`&self` and only ever narrow. Not sources.

Crate-wide sweep for public `-> Cx` returns found only these, none of them relevant
to a CLI: `grpc/server.rs:1952` `cx_narrow`, `grpc/server.rs:1961` `cx_readonly`,
`web/request_region.rs:395` `cx_narrow`, `web/request_region.rs:405` `cx_readonly`,
`net/atp/test_utils.rs:21` `test_cx` (which just wraps `Cx::for_testing_with_budget`,
so it inherits the test-only gating below).

### 3.2 `set_current_restricted` — the one public ambient *installer*

Worth calling out because it is easy to miss and it is genuinely `pub` with no cfg
gate (`cx/cx.rs:508`).

```rust
pub fn set_current_restricted(self) -> CurrentCxGuard {
    let mask = <Caps as cap::CapSetRuntimeMask>::MASK;   // :509
    let cx = self.retype::<cap::All>();                  // :510
    ...
    s.push(CurrentCxFrame { cx, mask });                 // :521
}
```

For `Caps = cap::All` the mask is every bit (`cx/cap.rs:99`, `:185-193`), so despite
the name this is a full-capability ambient install. Unlike `push_restriction`
(`cx/cx.rs:543`) it does **not** intersect with the existing top frame — it pushes
`Caps::MASK` outright — so it can widen relative to the frame beneath it.

Use for this bead: if work must run on a thread that has no ambient Cx, you can clone
a driver-backed `Cx<cap::All>` obtained from `Cx::current()` and re-install it there
with `set_current_restricted()`. Two hard limits: `CurrentCxGuard` carries
`_not_send: PhantomData<*mut ()>` (`cx/cx.rs:324`) so the guard cannot cross threads
(the `Cx` itself can), and depth is asserted against `MAX_CONTEXT_STACK_DEPTH`
(`cx/cx.rs:513-520`).

This is not needed for any of the three cass call sites, since (a) and (c) both
already have an ambient Cx. Recording it because it is the only supported public way
to establish ambient Cx on a thread the runtime did not set up.

### 3.3 Compiled-in under `test-internals` — available to cass, but a smell

Every one of these is `#[cfg(any(test, feature = "test-internals"))]` or
`#[cfg_attr(feature = "test-internals", visibility::make(pub))]`. Because
`Cargo.toml:26` enables `test-internals`, **they will compile in cass production
code.** They should not be used there, and the crate says so in its own words.

| API | file:line | gate | Why it is wrong for production |
|---|---|---|---|
| `Cx::for_testing()` | `cx/cx.rs:3232` (gate at `:3230`) | cfg | Doc at `:3212-3214`: "intended for testing only. Production code should receive Cx instances from the runtime". No drivers — `Self::new(...)` gives `io_driver: None`, so requirement 2 fails and HTTP falls to the spin path |
| `Cx::for_testing_with_budget(Budget)` | `cx/cx.rs:3264` | cfg | same, plus budget |
| `Cx::for_testing_with_io()` | `cx/cx.rs:3293` (gate at `:3291`) | cfg | Installs `LabIoCap::new_for_tests()` (`cx/cx.rs:3302`) — a lab IO cap, not the real reactor. Would make HTTP talk to a test double, not the network |
| `Cx::for_request()` | `cx/cx.rs:3357` | cfg | See the crate's own security note, `cx/cx.rs:3306-3344`: this shape was "a fully ambient capability source available to any caller in any crate" and was gated *deliberately* to stop exactly this. Also no drivers |
| `Cx::for_request_with_budget(Budget)` | `cx/cx.rs:3346` | cfg | same |
| `Cx::for_testing_with_remote(RemoteCap)` | `cx/cx.rs:3372` | cfg | same |
| `Cx::set_current(Option<Self>) -> CurrentCxGuard` | `cx/cx.rs:465` (attr at `:464`) | `visibility::make(pub)` | The runtime's own installer. `set_current_restricted` (`cx/cx.rs:508`) is the ungated equivalent — use that instead |
| `Cx::new_with_io(...)` | `cx/cx.rs:643` (attr at `:642`) | `visibility::make(pub)` | Raw constructor; requires you to supply driver handles you cannot obtain (`io_driver_handle` is `pub(crate)`, `cx/cx.rs:721`) |
| `Cx::new_with_drivers(...)` | `cx/cx.rs:668` (attr at `:666`) | `visibility::make(pub)` | same |

The crate names the sanctioned production alternative explicitly at
`cx/cx.rs:3334-3337`: "Production callers that need a request-scoped Cx must go
through `crate::runtime::Runtime::request_cx_with_budget`". **That method is
`pub(crate)`** (`runtime/builder.rs:3331`) and carries no `visibility::make(pub)`, so
it is *not* reachable from cass. Same for `current_request_cx_with_budget`
(`runtime/builder.rs:3339`), `block_on_with_cx` (`:3299`) and
`block_on_current_with_cx` (`:3318`).

**So: there is no public, non-test constructor for a driver-backed `Cx`.** The
runtime mints them internally and publishes them exclusively through the ambient
thread-local. Which is the whole answer to this lane — `Cx::current()` is not one
option among several, it is the only production door.

Flagging as requested: reaching for any row in this table from cass production code
is a smell. It works only because a test feature is enabled in a shipping binary, and
`for_request`'s doc comment describes that precise pattern as a sandbox-escape bug
the crate closed on purpose.

### 3.4 `Scope` — structured concurrency. Not viable.

Asked specifically about `cx/scope.rs:381`. Verdict: **not usable from cass at all,
in any of (a)/(b)/(c).**

`Scope::spawn` is `cx/scope.rs:381`:

```rust
pub fn spawn<F, Fut, Caps>(
    &self,
    state: &mut RuntimeState,     // :383
    cx: &Cx<Caps>,
    f: F,
) -> Result<(TaskHandle<Fut::Output>, StoredTask), SpawnError>
where Caps: cap::HasSpawn + Send + Sync + 'static,
      F: FnOnce(Cx<Caps>) -> Fut + Send + 'static, ...
```

Two independent blockers.

**Blocker 1 — `&mut RuntimeState` is unobtainable.** The runtime's state lives at
`RuntimeInner.state: Arc<ContendedMutex<RuntimeState>>` (`runtime/builder.rs:3752`).
`RuntimeInner` is a private struct (`runtime/builder.rs:3750`, no `pub`) and the field
is private. I swept for any public API that *hands out* a `&mut RuntimeState` and
found none — every public mention takes one as a parameter: `app.rs:516`,
`runtime/io_op.rs:64`, `:70`, `cx/scope.rs:1753`, `:1780`. `Runtime` exposes
`handle()` (`runtime/builder.rs:3267`), `config()` (`:3372`), `is_quiescent()`
(`:3377`), `draining_region_count()` (`:3386`) — no state accessor.

**Blocker 2 — it does not schedule.** `spawn` returns `(TaskHandle, StoredTask)`; the
caller is responsible for storing and injecting the task. Contrast
`RuntimeInner::spawn_with_cx`, which does the `store_spawned_task` +
`scheduler.inject_ready` itself (`runtime/builder.rs:4049`, `:4053`).

Both blockers apply to every sibling: `spawn_task` `cx/scope.rs:531`,
`spawn_registered` `:572`, `spawn_local` `:629`, `spawn_blocking` `:810`, `region`
`:893`, `region_with_budget` `:913`, `region_with_priority` `:934`,
`region_with_budget_and_priority` `:952`,
`region_with_budget_and_capability_budget` `:986`, `defer_sync` `:1753`,
`defer_async` `:1780` — all take `state: &mut RuntimeState`.

The combinators `join` (`cx/scope.rs:1172`), `race` (`:1224`), `hedge` (`:1312`),
`race_all` (`:1454`), `join_all` (`:1593`) do *not* take state — but they consume
`TaskHandle<T>` values, which only the state-taking spawn methods produce. So they
are unreachable transitively. (`TaskHandle` is also a different type from the
`runtime::JoinHandle` that `try_spawn` returns; they are not interchangeable.)

`Cx::scope()` (`cx/cx.rs:3061`) constructs a `Scope` happily. The `Scope` just cannot
be driven. Consistent with the `Scope::spawn` doctests, which are all `ignore`d or
`compile_fail` and thread a `&mut RuntimeState` in from nowhere
(`cx/scope.rs:365-379`).

---

## 4. Null results and limits of this lane

- **I did not execute anything.** Everything above is read from source. The §2(c)
  claim in particular (`Cx::current()` is `Some` inside a `try_spawn`ed bare future)
  is a five-hop source derivation and deserves an executed falsifier before anyone
  builds on it.
- **§2(b) is genuinely unsettled and I am not going to pretend otherwise.** There is
  no API contract linking `current_handle()` to `Cx::current()`. I established
  co-occurrence on the two thread classes that matter and one likely divergence
  (blocking-pool threads, `runtime/spawn_blocking.rs:416`). A general answer would
  need an enumeration of every thread the runtime creates, which is beyond this lane.
- **I did not diff 0.3.2 against 0.3.4 for the hang.** I checked one thing only:
  `Runtime::block_on` installs the ambient Cx in *both* versions —
  `0.3.2:src/runtime/builder.rs:3239` has the byte-identical comment and
  `request_cx_with_budget` call as `0.3.4:runtime/builder.rs:3276`. So the ambient-Cx
  install is **not** the 0.3.2→0.3.4 behavioural difference. Whatever changed is
  elsewhere; that is the falsifier lane's ground.
- **I could not verify `cx.io()` / `cx.fetch_cap()` are irrelevant to `HttpClient`.**
  I grepped `http/h1/http_client.rs` for `cx.io(`, `cx.fetch_cap(`, `has_io`,
  `register_io` and got zero hits, and traced the connect path to `TcpStream::connect`
  which takes no Cx. That is strong but it is an absence-of-evidence result over one
  file; a redirect or TLS path I did not read could consult them.
  (`Cx::io` is `cx/cx.rs:1344`, `Cx::fetch_cap` is `cx/cx.rs:1390`, both `pub`, both
  cap-gated and `Option`-returning.)
