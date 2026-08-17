# Lane: call-sites — consumer surface map of the three spin-wait sites (bead 759l7)

Read-only mapping lane. No files edited except this log. No cargo build/test run.

## Bonus finding before the per-file maps: asupersync 0.3.4's own source explains *why* `current_thread()` is the trigger

Not asked for directly, but load-bearing for how the three maps below should be read, so it goes first.

`asupersync-0.3.4/src/runtime/builder.rs:3270-3291` (`Runtime::block_on`) installs an ambient `Cx`
for the duration of the call (`Cx::set_current(Some(request_cx))`, :3289) *and* makes
`Runtime::current_handle()` return `Some` for the same duration (doc comment :3272-3274: "While the
future is being polled, a thread-local `RuntimeHandle` is available via `Runtime::current_handle`").
This is true regardless of worker-thread count — confirmed by the crate's own test
`current_handle_available_inside_block_on` (:6027-6042), which builds with `.worker_threads(1)`,
the exact configuration `RuntimeBuilder::current_thread()` is documented as a preset for
(:2964-2976: "Equivalent to `RuntimeBuilder::new().worker_threads(1)`").

Two things follow, both directly relevant to the maps below:

1. **The `Cx::current()` fallback branch in `update_check.rs:860` is not dead code by accident — it
   would work if it were reached**, because `block_on` already installs an ambient `Cx::current()`
   result (:3288-3289). The code never reaches it because `current_handle()` is checked first
   (:840) and is *also* always `Some` inside `block_on` for the same reason. So the existing
   fallback branch is evidence a simpler fix (skip the spawn+channel dance, just use
   `Cx::current()` directly) may already be available in the surrounding code's own logic — that's
   for the fix lane to weigh, not for me to decide, but it's worth flagging since it's sitting
   right there in the current source.
2. **Worker-thread count, not "root future vs registered task," is what the crate's own test suite
   shows determines whether a spawned task actually runs.** `current_handle_spawn_completes_on_scheduler`
   (:6053-6075) spawns via `handle.spawn(...)` (not `try_spawn_with_cx`, but the same scheduler-queue
   mechanism) from inside `block_on` on a `.worker_threads(2)` runtime, and the spawned task **does**
   complete — a second worker thread picks it up while the first is occupied running the root future.
   There is **no equivalent test in asupersync's own suite with `worker_threads(1)`** — i.e., no
   coverage at all of the exact configuration (`current_thread()`, single worker) that all three cass
   call sites use. That gap is consistent with the bead's #58 hypothesis but I want to be precise
   about what's actually proven: the crate's tests prove multi-thread spawn-from-block_on works, and
   prove nothing one way or the other about single-thread spawn-from-block_on — they simply never
   exercise it.

Consequence for reading the maps below: **the two sites that unconditionally build their own nested
`current_thread()` runtime for every call (`model_download.rs`, `deploy_cloudflare.rs`) are
structurally always in the single-worker configuration.** `update_check.rs` is split — its sync path
always nests a fresh `current_thread()` runtime (same as the other two), but its async path
(`fetch_latest_release().await` at :200) runs directly under whatever runtime the *caller* already
established via `.await`, with no new runtime built. In production that ambient runtime is chosen by
`src/main.rs:272-283`, and it is `multi_thread()` for every CLI command except `Commands::Search` and
`Commands::Health` (`main.rs:272-278`) — `Commands::Upgrade` (`cass upgrade`, `src/lib.rs:1011`,
dispatched at `src/lib.rs:7177-7184`) is neither, so it runs multi-threaded. Two of the update_check
integration tests (`integration_failed_async_check_...`, `integration_force_check_...`) reach the same
async fn but do so through a test-built `current_thread()` runtime (`update_check.rs:1870-1872`,
`:1929-1931`), which is single-worker regardless of what production would use. **I have not executed
anything, so this is a citation-backed inference about exposure, not a verified claim that `cass
upgrade` is safe in production** — flagging it because it changes what "fixing the bug" has to cover:
the sync-blocking call sites are unconditionally exposed; the async call site's real-world exposure
depends on which CLI command drove it there.

---

## 1. `src/update_check.rs:852` — `fetch_latest_release()`

**Enclosing function of the loop:** `async fn fetch_latest_release() -> Result<GitHubRelease>`
(`update_check.rs:839-862`). It is **async**. The loop itself is at :849-857; the line named in the
bead (:852) is the `Err(TryRecvError::Empty) => asupersync::runtime::yield_now().await` arm.

**Who builds the runtime, and where — two distinct call chains reach this same async fn:**

- **Sync chain** — `fetch_latest_release_blocking()` (sync `fn`, :897-902) unconditionally builds a
  fresh runtime and blocks on it:
  ```
  asupersync::runtime::RuntimeBuilder::current_thread()   // update_check.rs:898
      .build()
      .context("building update-check runtime")?
      .block_on(fetch_latest_release())                    // update_check.rs:901
  ```
  `current_thread()` — single worker thread (see bonus section above). This nested runtime exists
  only for the duration of this one call; it is not the app's ambient runtime.

- **Async chain** — `check_for_updates_async_impl()` (async `fn`, :186-218) calls
  `fetch_latest_release().await` directly at :200, with **no runtime construction of its own**. It
  runs under whatever runtime is already driving its caller. In tests that's an explicitly-built
  `current_thread()` test runtime (see below); in production it's whatever `src/main.rs:279-283`
  chose for the dispatched CLI command (multi-thread for every command except `Search`/`Health`,
  per the bonus section).

**What the spawned closure needs `Cx` for:** the closure at :844-846 is
`move |cx| async move { let _ = tx.send(fetch_latest_release_with_cx(&cx).await); }`.
`fetch_latest_release_with_cx` (:864-894) uses `cx` for:
- `cx.now()` at :870 — clock source passed into `asupersync::time::timeout(...)`.
- `client.request(cx, ...)` at :872-873 — the asupersync HTTP client's request call takes `&cx`
  (actually `cx` by value reference through the timeout wrapper) to issue the GET.

No other use of `cx` in that closure; `response.json::<GitHubRelease>()` (:892) and the trusted-URL
check (:806) are plain sync/local calls.

**Every caller of `fetch_latest_release` (the enclosing fn), by `rg`:**
```
update_check.rs:200   check_for_updates_async_impl()  — fetch_latest_release().await
update_check.rs:901   fetch_latest_release_blocking()  — .block_on(fetch_latest_release())
```
Exactly two — both already covered above.

**Chasing one level further, up to real callers/CLI entry points** (not strictly asked, included for
context since it changes the production-exposure picture — see bonus section):

- `fetch_latest_release_blocking()` ← `check_for_updates_sync()` (`update_check.rs:766-798`, sync,
  `pub fn`) ← `spawn_update_check()` (`update_check.rs:906-919`, spawns a **new OS thread** and calls
  `check_for_updates_sync` on it) ← `src/ui/app.rs:5609`
  (`Some(spawn_update_check(env!("CARGO_PKG_VERSION").to_string()))`) — this is the interactive TUI's
  update-check kickoff, run on a plain `std::thread::spawn`, so whatever asupersync runtime that
  thread builds (via `fetch_latest_release_blocking`'s own `current_thread()`) is standalone and
  unrelated to the app's top-level runtime choice.
- `check_for_updates()` (`update_check.rs:182-184`, async, `pub async fn`) ← `src/lib.rs:81267`,
  inside `maybe_prompt_for_update()` (`lib.rs:81257-...`, async fn) ← (not traced further; this is
  the TUI startup update-nag path per its own doc/gating at :81258-81264, guarded on
  `io::stdin().is_terminal()`).
- `check_for_updates()` / `force_check()` (`update_check.rs:221-223`, async, `pub async fn`) ←
  `src/lib.rs:82234-82238`, inside `run_upgrade()` (`lib.rs:82222-...`, async fn) ← dispatched from
  `Commands::Upgrade` at `src/lib.rs:7177-7184` (`cass upgrade`).

**Test functions exercising this site, verified by reading the test module (`update_check.rs:921-2068`,
`#[cfg(test)] mod tests` at :921-922, `use serial_test::serial` at :924):**

There are **17** `integration_*` test functions total in this file. I traced the call graph of each by
hand (not by name pattern) and found **13** that reach the `fetch_latest_release()` loop, and **4**
that do not touch it at all. This is a static call-graph count, not an executed one — flagging that
the bead's context states "12 tests for update_check"; my count is **13**. I re-checked each of the 13
individually below rather than trusting the arithmetic, and I can't find a 14th or a reason to drop one
to reach 12 — noting the discrepancy plainly rather than silently matching the bead's number.

Reach the loop — direct `fetch_latest_release_blocking()` callers (10):
| test fn | def line | call line |
|---|---|---|
| `integration_fetch_release_success` | 1599 | 1616 |
| `integration_fetch_release_404_error` | 1632 | 1639 |
| `integration_fetch_release_malformed_json` | 1658 | 1665 |
| `integration_fetch_release_missing_fields` | 1678 | 1688 |
| `integration_fetch_release_server_error` | 1702 | 1709 |
| `integration_version_comparison_with_real_fetch` | 1722 | 1735 |
| `integration_prerelease_version_handling` | 1755 | 1768 |
| `integration_connection_refused_is_offline_friendly` | 1797 | 1803 |
| `integration_blocking_fetch_release_success_v1` | 1956 | 1969 |
| `integration_blocking_fetch_release_403_error` | 1983 | 1990 |

Reach the loop — via `check_for_updates_sync()` → `fetch_latest_release_blocking()` (1):
| test fn | def line | call line |
|---|---|---|
| `integration_failed_sync_check_does_not_throttle_future_checks` | 1830 | 1842 (`check_for_updates_sync("0.1.0")`) |

Reach the loop — via a **test-built** `current_thread()` runtime wrapping the async chain (2):
| test fn | def line | runtime build | `block_on` call |
|---|---|---|---|
| `integration_failed_async_check_does_not_throttle_future_checks` | 1858 | 1870-1872 | 1873 `runtime.block_on(check_for_updates("0.1.0"))` |
| `integration_force_check_bypasses_cadence_even_when_state_save_fails` (`#[cfg(unix)]`) | 1890 | 1929-1931 | 1932 `runtime.block_on(force_check("0.1.0"))` |

All 13 above are `#[serial]`-annotated (via `serial_test`), and all set/clear `CASS_UPDATE_API_BASE_URL`
and/or `CASS_DATA_DIR` env vars, so they must run serialized regardless — consistent with their being
grouped as a set.

Do **not** reach the loop (4) — confirmed by reading each body; none constructs a runtime or calls
`fetch_latest_release*`/`check_for_updates*`/`force_check`:
| test fn | def line | what it actually does |
|---|---|---|
| `integration_release_api_base_url_default` | 2003 | calls `release_api_base_url()` only |
| `integration_release_api_base_url_override` | 2022 | same |
| `integration_http_timeout_is_reasonable` | 2039 | `const _` assertion on `HTTP_TIMEOUT_SECS` |
| `integration_check_interval_is_reasonable` | 2055 | `const _` assertion on `CHECK_INTERVAL_SECS` |

**`cargo test` filter strings** (crate lib name is `coding_agent_search` — no `[lib]` section in
`Cargo.toml`, so it's derived from `name = "coding-agent-search"` with hyphens→underscores; confirmed
by `tests/deploy_cloudflare.rs:7` using `use coding_agent_search::pages::deploy_cloudflare::{...}`).
Full test path is `update_check::tests::<fn name>` under the lib target:

- All 17 `integration_*` tests (13 hang-exposed + 4 safe): `cargo test --lib -- update_check::tests::integration_`
- The 4 that do **not** reach the site, isolated by substring: none of them share a substring that
  the other 13 lack, so a single filter can't isolate "safe" vs "exposed" — I confirmed this by
  reading, not by pattern. Practical options for a lane that wants to run only the 13 exposed ones:
  run by exact name (`cargo test --lib --exact update_check::tests::integration_fetch_release_success`,
  repeated per row above), or run the full `integration_` set under an external per-test timeout,
  since none of `cargo test`'s own flags provide one.

## 2. `src/search/model_download.rs:1022` — `run_download_with_cx`

**Enclosing function of the loop:** `fn run_download_with_cx<T, F, Fut>(f: F) -> Result<T, DownloadError>`
(`model_download.rs:994-1031`). It is **sync** (not `async fn`). Unlike `update_check.rs`, there is
**only one call path** to this function's loop — no async/sync split, no fallback branch.

**Who builds the runtime, and where:**
```
let runtime = asupersync::runtime::RuntimeBuilder::current_thread()   // model_download.rs:1000
    .build()
    .map_err(...)?;

runtime.block_on(async move {                                          // model_download.rs:1006
    let handle = asupersync::runtime::Runtime::current_handle().ok_or_else(...)?;   // :1007
    ...
})
```
`current_thread()` — single worker thread, built fresh on **every call**. Unlike `update_check.rs`'s
async path, there is no way to reach this loop without constructing a new single-worker runtime first
— `run_download_with_cx` is always sync-entry, so it's unconditionally exposed on every invocation, in
tests and in production alike (this is the one site of the three where I found no split between
"exposed" and "possibly-safe" call paths).

Also notably different from `update_check.rs`: there is **no `Cx::current()` fallback** here — if
`current_handle()` is `None`, the function returns an error immediately (`.ok_or_else(...)`, :1007-1009)
rather than trying an alternate path to get a `Cx`.

**What the spawned closure needs `Cx` for** (the one call site, `model_download.rs:1302-1436`, inside
`download_file`): the closure needs `cx` for:
- `cx.now()` at :1324 and :1383 — clock source for two separate `asupersync::time::timeout(...)` calls
  (connect timeout at :1323-1336, per-frame read timeout inside the streaming loop at :1382-1388).
- `client.request_streaming(&cx, ...)` at :1326-1332 — issuing the (possibly range-resumed) GET.

Note: the streaming body's `poll_frame` call at :1385 uses a **different**, unrelated `task_cx:
&mut std::task::Context` from `poll_fn`, not the asupersync `Cx` — that's ordinary `Future::poll`
plumbing, not another use of the asupersync `Cx`.

**Every caller of `run_download_with_cx`, by `rg`:** exactly one —
`model_download.rs:1302`, inside `download_file()` (`fn download_file(&self, ...) -> Result<(),
DownloadError>`, sync, :1264-1436).

**Chasing up to the CLI entry point:**
- `download_file()` ← `download_with_mirror()` (`pub fn`, sync, `model_download.rs:1114-1227`), called
  once per manifest file inside a retry loop (:1173-1198). `download()` (`pub fn`, :1105-1111) is a
  thin wrapper (`self.download_with_mirror(manifest, None, on_progress)`).
- `download_with_mirror()` ← `src/lib.rs:90727`, inside `run_models_install()` (sync `fn`,
  `lib.rs:90501-...`). Production call wraps it in a **dedicated fresh OS thread**:
  ```rust
  let result = std::thread::spawn(move || {
      downloader.download_with_mirror(&manifest_clone, mirror_base_url_clone.as_deref(), ...)
  }).join()...   // lib.rs:90726-90745
  ```
  So every production invocation gets its own OS thread *and* (inside `run_download_with_cx`) its
  own nested single-worker asupersync runtime, per `download_file` call, per retry attempt.
- `run_models_install()` ← `src/lib.rs:90101-90107` (`ModelsCommand::Install` dispatch, i.e.
  `cass models install`) and ← `src/lib.rs:90952` (the `--repair` path of `cass models verify`,
  inside `run_models_command`'s `ModelsCommand::Verify` handling).

**Test functions exercising this site — verified by reading `#[cfg(test)] mod tests`
(`model_download.rs:2092-2093`) in full:** exactly **4**, all named `test_download_with_mirror_*`,
all calling `downloader.download_with_mirror(...)` on a real local HTTP fixture server
(`start_mirror_fixture_server`), which is the only path in the test module that reaches
`download_file` → `run_download_with_cx`. This matches the bead's "4 for model_download" exactly.

| test fn | def line | `download_with_mirror` call(s) |
|---|---|---|
| `test_download_with_mirror_installs_verified_model_from_http_mirror` | 3209 (`#[test]` at 3208) | 3238 |
| `test_download_with_mirror_reports_missing_artifact_from_http_mirror` | 3280 (3279) | 3293 |
| `test_download_with_mirror_discards_corrupt_payload_from_http_mirror` | 3309 (3308) | 3330 |
| `test_download_with_mirror_resumes_after_cancelled_partial_download` | 3348 (3347) | 3372 and 3398 (called twice: once expected to cancel mid-stream, once to finish) |

I confirmed no other test in the file's ~50-test module calls `download_with_mirror`, `download`, or
`download_file` — the other tests (listed via `rg '^\s*fn (test_|integration_)'`, 47 other matches)
exercise pure/local logic: manifest/state helpers, mirror-URL validation, `atomic_install`,
`prepare_temp_dir`, `compute_sha256`, error-classification, etc. — none of them touch
`run_download_with_cx`.

**`cargo test` filter string:** full test path is `search::model_download::tests::<fn name>` (module
declared `pub mod model_download;` at `src/search/mod.rs:33`). Unlike `update_check.rs`, a single
substring cleanly isolates exactly these 4 and nothing else — verified by `rg` that
`download_with_mirror` as a function-name substring appears in no other test name in the repo:

```
cargo test --lib 'search::model_download::tests::test_download_with_mirror'
```

## 3. `src/pages/deploy_cloudflare.rs:843` — `run_cloudflare_with_cx`

**Enclosing function of the loop:** `fn run_cloudflare_with_cx<T, F, Fut>(f: F) -> Result<T>`
(`deploy_cloudflare.rs:820-850`). **Sync**, same shape as `model_download.rs`'s
`run_download_with_cx` — no async/sync split, no `Cx::current()` fallback (`ok_or_else` returns an
error immediately if `current_handle()` is `None`, :831-832).

**Who builds the runtime, and where:**
```
let runtime = asupersync::runtime::RuntimeBuilder::current_thread()   // deploy_cloudflare.rs:826
    .build()
    .context("building Cloudflare API runtime")?;

runtime.block_on(async move {                                          // deploy_cloudflare.rs:830
    let handle = asupersync::runtime::Runtime::current_handle()
        .ok_or_else(...)?;                                             // :831-832
    ...
})
```
`current_thread()` — single worker thread, built fresh on every call, same as site 2. Every call
(there are 7 across this file, all funneling into the same one `run_cloudflare_with_cx` loop) is
unconditionally exposed the same way.

**What the spawned closures need `Cx` for.** There are **two** call sites of `run_cloudflare_with_cx`,
each wrapping a distinct closure:

- `execute_cloudflare_request()` (`deploy_cloudflare.rs:867-897`) — closure at :874-896 uses `cx` for
  `cx.now()` (:883, timeout clock) and `client.request(&cx, method, &url, headers, body)` (:885-891).
- `execute_cloudflare_multipart_request()` (`deploy_cloudflare.rs:899-928`) — closure at :905-927 uses
  `cx` for `cx.now()` (:914) and `client.request_multipart(&cx, Method::Post, &url, headers, &form)`
  (:916-922).

**Every caller, by `rg`.** `execute_cloudflare_request` is called from **6** sites, all in this file;
`execute_cloudflare_multipart_request` from **1**:

| caller fn | call line(s) | purpose |
|---|---|---|
| `check_project_exists_api` | 973 | `GET .../pages/projects/{name}` |
| `create_project_api` | 998 | `POST .../pages/projects` |
| `fetch_upload_token` | 1105 | `GET .../pages/projects/{name}/upload-token` |
| `check_missing_hashes` | 1265 | `POST .../pages/assets/check-missing` |
| `upload_bucket` | 1355 | `POST .../pages/assets/upload` |
| `upsert_hashes` | 1368 | `POST .../pages/assets/upsert-hashes` |
| `deploy_with_api` (multipart) | 1077 | `POST .../pages/projects/{name}/deployments` |

All 7 of the above are private (non-`pub`) helper `fn`s in this file. There is **no `pub fn` anywhere
in `deploy_cloudflare.rs`** except on the `CloudflareConfig`/`CloudflareDeployer`/`Prerequisites`
struct impls (confirmed: `rg 'pub fn|pub(crate) fn' deploy_cloudflare.rs` matches only struct methods,
none of the 7 network helpers above).

**Chasing up to the CLI entry point.** All 7 helpers above are reached only through
`CloudflareDeployer::deploy()` (`pub fn`, sync, `deploy_cloudflare.rs:262-...`), and — this is the
important structural fact — **only on the non-`wrangler` branch**. `deploy()` computes
`can_use_wrangler = prereqs.wrangler_version.is_some() && (prereqs.wrangler_authenticated ||
prereqs.api_credentials_present)` (:289-290) and takes the `execute_cloudflare_*`/API path only in the
`else if let (Some(account_id), Some(api_token)) = ...` branches (:307-309, :321-323, :340-348) — i.e.
when `wrangler` isn't usable but Cloudflare account id + API token are both present. When `wrangler` is
available, `deploy_with_wrangler`/`check_project_exists`/`create_project` (a completely different,
subprocess-based path, not shown here) are used instead and never touch asupersync at all.

Real callers of `.deploy()`:
- `src/pages/wizard.rs:1998`, inside `step_deploy()` (`fn step_deploy(&self, term: &mut Term) ->
  Result<()>`, sync, `wizard.rs:1855-...`), reached from `PagesWizard::run()`
  (`wizard.rs:239-...`) at `wizard.rs:287` (`self.step_deploy(&mut term)?;`) — the interactive `cass
  pages` wizard's Cloudflare branch (`DeployTarget::CloudflarePages`, `wizard.rs:1973-...`).
- `src/lib.rs:73693`, inside `run_config_based_export()` (sync `fn`, `lib.rs:73524-...`) —
  the config-driven, non-interactive path (`pages_config.deployment.target == "cloudflare"`),
  dispatched from CLI arg handling around `lib.rs:6380-6500` (config validation, then
  `run_config_based_export(...)` at `lib.rs:6490-6496`).
- `PagesWizard::run()` itself is invoked from `src/lib.rs:6784` + `:6810`
  (`let mut wizard = crate::pages::wizard::PagesWizard::new(); ... wizard.run()...`).

So this site's exposure in production is gated on wrangler being absent/unauthenticated — it is not
hit on every Cloudflare deploy, only the API-fallback ones.

**Test coverage — confirmed null result.** I searched the whole repo, not just this file's own test
module, for anything that reaches this site:

1. Read the full `#[cfg(test)] mod tests` block in `deploy_cloudflare.rs` (starts :1587-1588). It
   contains **25** `#[test] fn test_*` functions (`rg -c '^\s*fn (test_|integration_)'` → 25). All of
   them exercise structural/local logic: config builder defaults, base-URL trust validation, header
   generation, `_headers`/`_redirects` file writes, `copy_dir_recursive` symlink-safety cases,
   temp-dir cleanup-on-drop, `select_missing_files` dedup, etc. **None calls `.deploy(`,
   `execute_cloudflare_request`, `execute_cloudflare_multipart_request`, `run_cloudflare_with_cx`,
   `deploy_with_api`, `check_project_exists_api`, or `create_project_api`** — confirmed by
   `rg -n '\.deploy\(' src/pages/deploy_cloudflare.rs` returning zero matches inside the test module
   (the only match for the method itself is its own `pub fn deploy` definition).
2. Searched `tests/` (the crate's external integration-test crate) for anything touching this module:
   `tests/deploy_cloudflare.rs` (an integration-test file, distinct from the in-file unit-test module
   above) and `tests/e2e_deploy.rs` both construct `CloudflareDeployer::default()` /
   `CloudflareDeployer::with_project_name(...)` and call `.check_prerequisites()`,
   `.generate_headers_file()`, `.generate_redirects_file()`, config builder methods, etc. — but
   **`rg -n '\.deploy\(' tests/deploy_cloudflare.rs tests/e2e_deploy.rs` returns zero matches.** No
   test in either file ever calls the actual `.deploy(...)` method, so none of them can reach
   `run_cloudflare_with_cx`.
3. Searched the whole repo (`rg -n 'CloudflareDeployer|deploy_with_api|execute_cloudflare_request|
   execute_cloudflare_multipart_request|run_cloudflare_with_cx|check_project_exists_api|
   create_project_api'`, excluding this file itself) for any other consumer. Hits: the two production
   `use`/construction sites already covered (`wizard.rs:16,1985`; `lib.rs:73682`), the `tests/`
   references already covered, and two unrelated `docs/artifacts/refactor-runs/...` prose files
   (refactor-pass notes, not code). No hidden test harness, no `#[ignore]`d test, nothing under
   `tests/e2e/cloudflare/` (that directory is Playwright/TypeScript smoke tests against an already
   -deployed live URL via `CLOUDFLARE_TEST_URL`, checking HTTP headers on a running site — it never
   invokes the Rust deploy code at all).
4. Also checked `CLOUDFLARE_ACCOUNT_ID`/`CLOUDFLARE_API_TOKEN` env-var references repo-wide, since a
   test setting both would be the only way to reach the API-fallback branch of `.deploy()` even if it
   did call it: the only test hit is `tests/deploy_cloudflare.rs:99`
   (`assert_missing_contains(&missing, "CLOUDFLARE_API_TOKEN")`), which asserts the token's *absence*
   is reported by `Prerequisites::missing()` — it is a negative-path prerequisites test, not a live
   call.

**This is a genuine null result, not an absence-of-search:** every test that constructs a
`CloudflareDeployer` was individually inspected, and I traced every reference to the five private
network-helper function names and the two entry-point struct/method names across `src/`, `tests/`, and
`docs/`. Nothing in the tracked test surface (Rust unit, Rust integration, or the Playwright e2e specs)
reaches `run_cloudflare_with_cx`. This is consistent with the bead's premise that this site is latent
(would hang once triggered) rather than already observed hanging in CI — there is no test that could
have observed it.

## Summary table

| site | loop fn | sync/async | runtime built | # `run_*_with_cx`-loop call sites in file | tests reaching it | cargo filter |
|---|---|---|---|---|---|---|
| `update_check.rs:852` | `fetch_latest_release` | async (loop fn); reached via 1 sync + 1 async caller | sync path: always nests fresh `current_thread()` (:898). async path: no runtime of its own — inherits caller's (test: nested `current_thread()`; prod: `main.rs`'s per-command choice) | 1 loop, 2 callers of the loop-owning fn | **13** (static count; bead says 12 — see discrepancy note above) | no single substring isolates the 13; full `integration_` set is 17 (13 exposed + 4 not) |
| `model_download.rs:1022` | `run_download_with_cx` | sync | always nests fresh `current_thread()` (:1000), every call | 1 loop, 1 caller (`download_file`) | **4**, exact match to bead | `cargo test --lib 'search::model_download::tests::test_download_with_mirror'` |
| `deploy_cloudflare.rs:843` | `run_cloudflare_with_cx` | sync | always nests fresh `current_thread()` (:826), every call | 1 loop, 7 callers (6 `execute_cloudflare_request` + 1 `execute_cloudflare_multipart_request`) | **0** — confirmed null result | n/a (no test reaches it) |
