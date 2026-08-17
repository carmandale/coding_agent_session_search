# Lane: repo-precedent (bead 759l7 spin-wait mapping)

Read-only mapping lane. No source files edited. Scope: find every
sync-into-async bridge pattern in `src/` and `tests/`, classify which work vs
which are the three known-broken spin-wait sites, and count the spin-wait
shape repo-wide.

## 0. Live-specimen note (read before trusting any "works today" claim below)

`Cargo.lock` is currently **dirty** in this worktree: `git status --short`
shows `M Cargo.lock`, and `git diff --stat -- Cargo.lock` reports 172
insertions / 116 deletions. The committed baseline at `HEAD` pins
`asupersync = 0.3.2` (`git show HEAD:Cargo.lock`, `name = "asupersync"` block).
The working-tree copy right now pins `asupersync = 0.3.4` (`Cargo.lock:324`,
read twice for consistency after an initial read returned a stale 0.3.2 —
almost certainly a race against a sibling session actively editing this same
file; `thoughts/shared/handoffs/20260815-cass-to-green/p3kgr-upstream-continuation.md`
and `verify-fsqlite-pin.sh` in this repo describe exactly this in-progress
0.3.2→0.3.4 experiment). So "exercised by passing tests" below means "passes
against the committed 0.3.2 baseline"; against the working tree's current
0.3.4, the three broken sites hang per those handoff docs (16 tests). I did
not run `cargo test` (out of scope, per rules) — this is read from source +
existing handoff evidence, not a fresh execution.

## 1. Every distinct sync↔async bridge pattern found

### Pattern A — plain `runtime.block_on(fut)`, no inner spawn, no side channel

The root future given to `block_on` directly contains (or itself calls) the
`Cx`-using code. Nothing needs a second wakeup path because there is only one
task: the root task `block_on` is already polling.

- `src/main.rs:285` — `runtime.block_on(coding_agent_search::run_with_parsed(parsed))`. Top-level entry point.
- `src/update_check.rs:898-901` — `fetch_latest_release_blocking()`: builds a fresh `current_thread` runtime and does `.block_on(fetch_latest_release())`. Note: `fetch_latest_release()` itself is **not** pure Pattern A — see Pattern B below, this is the outer wrapper.
- `src/update_check.rs:1423-1426` — test `test_update_state_save_async_replaces_existing_symlink`: `runtime.block_on(state.save_async())`.
- `src/update_check.rs:1870-1873` — test `integration_failed_async_check_does_not_throttle_future_checks`: `runtime.block_on(check_for_updates("0.1.0"))`.
- `src/update_check.rs:1929-1932` — test `integration_force_check_bypasses_cadence_even_when_state_save_fails`: `runtime.block_on(force_check("0.1.0"))`.
- `src/ui/app.rs:15441` — `TantivySearchService::run_live_search_stream`: `runtime.block_on(async move { client.search_progressive_with_callback(...).await })`. Root future directly awaits the client call; a **separate `std::thread::spawn`** cancel-watcher thread (`src/ui/app.rs:15426-15434`) signals cancellation via `cx.set_cancel_requested(true)` on an `AtomicBool` + `StopSignal`, not via spawning a second asupersync task. This is OS-thread-to-Cx signalling, not task-spawn-and-join.
- `src/search/query.rs:9617-9622` and `:9646` — `#[ignore = "profiling harness..."]` test `progressive_hybrid_profile_harness`: two `runtime.block_on(async { ... })` calls, root future again directly awaits the client call. `#[ignore]`d, so not run in normal `cargo test`.

### Pattern B — the broken shape: `current_handle()` + `try_spawn_with_cx` + `std::sync::mpsc` + `yield_now()` spin loop

Exactly this shape, three times, byte-for-byte identical control flow:

```
let handle = asupersync::runtime::Runtime::current_handle() [.ok_or_else / if let Some]
let (tx, rx) = std::sync::mpsc::channel();
handle.try_spawn_with_cx(move |cx| async move { let _ = tx.send(f(cx).await); })...;
loop {
    match rx.try_recv() {
        Ok(result) => return result,
        Err(TryRecvError::Empty) => asupersync::runtime::yield_now().await,
        Err(TryRecvError::Disconnected) => { ...bail... }
    }
}
```

- `src/update_check.rs:838-856` — `fetch_latest_release()`. Has a fallback branch (see Pattern C) when `current_handle()` is `None`.
- `src/search/model_download.rs:994-1035` — `run_download_with_cx<T, F, Fut>`. No fallback branch — `current_handle()` returning `None` is treated as a hard error (`DownloadError::NetworkError("download runtime handle unavailable")`), because this helper builds its own fresh runtime immediately above and calls `runtime.block_on` around the whole thing, so a handle is always expected to be present.
- `src/pages/deploy_cloudflare.rs:819-847` — `run_cloudflare_with_cx<T, F, Fut>`. Same shape as model_download's version, same no-fallback structure.

All three build a **fresh `RuntimeBuilder::current_thread()`** immediately before entering `block_on`, and the `try_spawn_with_cx` calls happen **inside** that same `block_on`'s root future — i.e., the spawned task and the yield-loop that's supposed to wake it are both aspects of one `block_on` call. This matches the bead's cited root cause (asupersync issue #58: `block_on`'s root future is polled outside `state.tasks`, so `yield_now` in the root only re-wakes the root, never the spawned child).

### Pattern C — fallback when no runtime handle: use `Cx::current()` directly, no spawn

- `src/update_check.rs:858-861`, inside `fetch_latest_release()`:
  ```rust
  let cx = asupersync::Cx::current().context("update check requires an active asupersync Cx")?;
  fetch_latest_release_with_cx(&cx).await
  ```
  Only reachable when `Runtime::current_handle()` returned `None` — i.e., when this function is itself already running as a spawned/registered task carrying its own `Cx`, rather than as `block_on`'s bare root future. I could not establish from `src/` alone which call sites land in this branch vs the Pattern-B branch above; `current_handle()`'s Some/None semantics live in `asupersync-0.3.4/src/runtime/builder.rs:3364` and its own test names (`current_handle_available_inside_block_on`, `current_handle_available_inside_spawned_task`, `current_handle_restored_after_block_on`, `:6028-6113`) suggest it is available in *both* places, which would mean every call in this codebase in fact takes the Pattern-B branch, but I did not read those asupersync test bodies to confirm — noting this as unestablished rather than guessing.

### Pattern D — OS-thread spin/backoff (`std::thread::yield_now`), unrelated to the async runtime

Three sites, all synchronous, all lock-free/atomic-flag polling loops with no `Cx`, `Runtime`, or asupersync task involved:

- `src/indexer/mod.rs:28541` — `while !started.load(Ordering::SeqCst) { thread::yield_now(); }`, waiting for a `std::thread::spawn`'d worker to set a flag before asserting on a heartbeat lock (test helper).
- `src/indexer/mod.rs:34403` — a single `thread::yield_now()` used once, deliberately, to hand the scheduler to a racing waiter thread before mutating `limiter.update_max_bytes_in_flight(64)` — comment: "we WANT the update to race against the waiter's predicate-check window."
- `tests/frankensqlite_concurrent_stress.rs:360` — `std::thread::yield_now()` inside a stress-test read loop, comment "Small yield to prevent spinning."

These are a different problem class entirely (racing/backoff between OS threads) and are not candidates for conflation with the asupersync bug.

### Pattern E — OS-thread + `std::sync::mpsc`, no async runtime at all

- `src/update_check.rs:906-916` — `spawn_update_check()`: `std::thread::spawn(move || { let result = check_for_updates_sync(&current_version); let _ = tx.send(result); })`, returns the raw `Receiver` to the caller. The receiver is read later by the TUI event loop (`src/ui/app.rs:5609`, wired as `Some(spawn_update_check(...))` into app state) presumably via non-blocking poll on the UI tick, not via a spin loop in this function. This is a legitimate, working, and structurally different pattern: the "task" is a whole OS thread, not an asupersync task, so there is no `Cx`/task-registration problem to hit. It cannot be a template for fixing the three Pattern-B sites because those three exist specifically to run `Cx`-carrying async HTTP code (`asupersync::http::h1::HttpClient`), not sync code.

Two false-positive greps worth flagging so nobody chases them: `src/pages/profiles.rs` (6 hits) and `src/pages/redact.rs` (2 hits) define/use a `bool` field literally named `block_on_critical_secrets` (a secrets-redaction policy flag). No relation to `Runtime::block_on` — confirmed by reading both files' surrounding struct definitions.

## 2. Which patterns work today vs which are broken

| Pattern | Site | Exercised by tests? | Status against committed 0.3.2 | Status against working-tree 0.3.4 |
|---|---|---|---|---|
| A | `main.rs:285` | Not unit-tested (CLI entry) | works (no spawn involved) | works (no spawn involved) |
| A | `update_check.rs:1426,1873,1932` | Yes — 3 named tests | pass | pass (Pattern A itself isn't the hanging shape) |
| A | `ui/app.rs:15441` | No unit test found for `run_live_search_stream` itself (only the static eligibility predicate is tested, `app.rs:27727+`) | untested | untested |
| A | `search/query.rs:9622,9646` | `#[ignore]`d harness, not run normally | n/a | n/a |
| **B (broken)** | `update_check.rs:852` (`fetch_latest_release`/`fetch_latest_release_blocking`) | **Yes — at least 12 tests** call `fetch_latest_release_blocking()` directly or transitively (`integration_fetch_release_success`, `_404_error`, `_malformed_json`, `_missing_fields`, `_server_error`, `integration_version_comparison_with_real_fetch`, `integration_prerelease_version_handling`, `integration_connection_refused_is_offline_friendly`, `integration_blocking_fetch_release_success_v1`, `integration_blocking_fetch_release_403_error`, plus `integration_failed_sync_check_does_not_throttle_future_checks` and others reaching it through `check_for_updates_sync`) | pass | **hang** (per `verify-fsqlite-pin.sh` and `p3kgr-upstream-continuation.md`, both already in this repo: "4 search::model_download::* and 12 update_check::integration_*" hang) |
| **B (broken)** | `search/model_download.rs:1022` (`run_download_with_cx`) | **Yes — 4 tests**, e.g. `test_download_with_mirror_installs_verified_model_from_http_mirror` (`model_download.rs:3208`) and its 3 siblings (`_reports_missing_artifact_from_http_mirror`, `_discards_corrupt_payload_from_http_mirror`, `_resumes_after_cancelled_partial_download`) | pass | **hang** |
| **B (broken)** | `pages/deploy_cloudflare.rs:843` (`run_cloudflare_with_cx`) | **No test coverage found.** `tests/deploy_cloudflare.rs` and `tests/e2e_deploy.rs` construct `CloudflareDeployer` and check prerequisites/config/dir-staging, but none calls `.deploy(...)` or any function that reaches `execute_cloudflare_request`/`execute_cloudflare_multipart_request` → `run_cloudflare_with_cx`. Confirmed by grep across `tests/` for `execute_cloudflare` (zero hits) and by reading the full `#[test] fn` list in `src/pages/deploy_cloudflare.rs` (50 tests, all structural/config, none touching the network path). | latent (would hang once triggered) | latent (would hang once triggered) |
| D | `indexer/mod.rs:28541,34403`, `tests/frankensqlite_concurrent_stress.rs:360` | Yes, but irrelevant shape | pass | pass (independent of asupersync version) |
| E | `update_check.rs:906` | Indirectly, via UI wiring; no dedicated unit test found for `spawn_update_check` itself | pass | pass (independent of asupersync version) |

So: **do not** treat "compiles" or "is a `block_on` call" as evidence of health. Pattern A sites are fine because they never spawn a second task inside the same `block_on`. Pattern B is uniformly broken — the code that "works today" only works because the committed lockfile still pins the tolerant 0.3.2, not because the pattern is sound.

## 3. Is any existing pattern a ready-made replacement for the spin-wait?

**No.** Searched for `\.try_spawn\(` (without `_with_cx`), `JoinHandle`, and `\.spawn\(` across `src/`: every `JoinHandle` in this codebase is `std::thread::JoinHandle` (OS threads — `indexer/mod.rs`, `search/model_download.rs:2148`, `search/query.rs:7826`, `tui_asciicast.rs:103`, `lib.rs:66805`), and every bare `.spawn(` call is either `std::process::Command::spawn()` or `std::thread::spawn`/`std::thread::Builder::spawn`. There is no existing call to asupersync's own `try_spawn` (the variant that returns `Result<JoinHandle<F::Output>, SpawnError>` per `asupersync-0.3.4/src/runtime/builder.rs:3557`) anywhere in `src/`. The bead's own "already established" notes list that API as available but unused in this repo — confirmed here: unused.

Pattern E (`spawn_update_check`, OS thread + `std::sync::mpsc`) is a working, tested bridge, but it bridges *sync* code out to a background OS thread — it carries no `Cx` and cannot run the `Cx`-requiring async HTTP client code the three broken sites need (`asupersync::http::h1::HttpClient::request(&cx, ...)`, `deploy_cloudflare.rs:838` / `model_download.rs:1013` / `update_check.rs` fetch path all take `&cx` as a parameter). It is not a drop-in template.

Pattern A is the closest thing to a working precedent for the *destination* shape (a `Cx`-carrying async body driven straight from `block_on`'s root future, no second task) — but none of the three Pattern-A sites needed to *spawn* anything, because their bodies had no separate-thread ownership requirement (the `f: F` closures being spawned in the three broken sites are `Send + 'static` and constructed by the caller precisely so they can be handed to `try_spawn_with_cx`; whether that spawn is actually load-bearing — e.g., for cancellation, `Send` boundary needs, or something else the fix lane needs to investigate — is outside this lane's scope. I did not find any comment in the three broken sites explaining *why* they spawn rather than just `.await`ing `f(cx).await` inline inside `block_on`.). Given `try_spawn` (returning an awaitable `JoinHandle`) exists in asupersync per the bead's own citations (`builder.rs:3557`, `:3686`) but is unused here, the nearest asupersync-native replacement is not a pattern already proven in this repo — it would be new code following an asupersync API this codebase has never exercised.

## 4. Repo-wide count of the `yield_now` spin-wait shape

**Exactly three sites carry the broken shape** (`asupersync::runtime::yield_now().await` inside a `loop { match rx.try_recv() ... }` fed by a `try_spawn_with_cx`'d task):

1. `src/update_check.rs:852`
2. `src/search/model_download.rs:1022`
3. `src/pages/deploy_cloudflare.rs:843`

Full repo-wide grep for `yield_now` (src/, tests/, benches/, and everything else excluding `target/` and `.git/`) turns up two more hits, both unrelated (`std::thread::yield_now`, Pattern D above, described in section 1):

- `src/indexer/mod.rs:28541`
- `src/indexer/mod.rs:34403`
- `tests/frankensqlite_concurrent_stress.rs:360`

`benches/` has zero hits for `yield_now`, `try_spawn`, or `block_on` (checked all ten files under `benches/`). So the bead's "three call sites" count is confirmed exhaustive for this shape — fixing exactly those three closes it, nothing in `tests/` or `benches/` independently reproduces the same broken pattern.

## Bottom line for the fix lane

- 3 sites, confirmed exhaustive, no 4th site hiding in tests/benches.
- 2 of the 3 (`update_check.rs`, `model_download.rs`) are covered by tests that currently pass on committed 0.3.2 and are documented (in this repo's own handoff docs, not just this lane) to hang on 0.3.4.
- The 3rd (`deploy_cloudflare.rs`) has **zero test coverage** — it will hang silently in production once triggered, with nothing in CI to catch it. Any fix should add coverage here, not just fix the code — echoing what `p3kgr-upstream-continuation.md` already recommended.
- No existing in-repo pattern is a ready-made template for the replacement. The two working async patterns (A: plain `block_on`, no spawn; C: `Cx::current()` fallback, no spawn) both avoid spawning entirely rather than solving spawn-and-wait. The one OS-thread spawn-and-wait pattern (E) doesn't carry a `Cx` and can't run the async HTTP client code. Whatever replaces the spin loop (awaiting `try_spawn`'s `JoinHandle` directly instead of a side `mpsc` channel, or some other shape) will be new to this codebase, not a copy of something proven here.
