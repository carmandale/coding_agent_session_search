# Findings — 759l7 and the pin ceiling, generation 12

Session `21e23d4e`. Every number here was executed in this session and its raw
output is named. Where something is unproven it says so.

## 1. Bead 759l7's root cause is wrong, and the fix does not fix the symptom

759l7 records three hand-rolled spin-waits as the cause of 16 hanging tests on
asupersync 0.3.4, and asserts that replacing them makes those tests pass.
Measured, in ONE tree, with fsqlite held at 0.1.5 and only asupersync moving:

| condition | result |
|---|---|
| 0.3.4, spin-wait as filed | 44/48 pass; the 4 `test_download_with_mirror_*` tests run past 60 s; killed at 150 s |
| 0.3.4, spin-wait removed | 44/48 pass; **the same 4 tests still never return**; killed at 280 s |

Raw: `test-baseline-034-modeldl.log`, `test-fixed-034-modeldl.log`,
`fixed-modeldl-verdict.log` under this session's job tmp.

The rebuild is confirmed rather than assumed — the second run's log opens with
`Compiling coding-agent-search v0.6.9 (…/cass-759l7-spin-wait)` and
`Finished in 26.53s`, so the binary under test carried the change. This repo has
a recorded incident of a shared `CARGO_TARGET_DIR` silently running the other
tree's binary, so that check is not a formality.

**What changed is the failure mode, not the outcome.** `sample` on the live hung
process (pid 68181, 5 s, saved to `hung-sample.txt`, parsed by
`parse-sample.py`):

- 0.0 % CPU — it is not spinning any more.
- All four test threads: `std::thread::park` at
  `asupersync-0.3.4/src/runtime/builder.rs:4184`, reached through
  `run_download_with_cx` → `Runtime::block_on` → `run_future_with_budget`.
- All three `asupersync-worker-0` threads: idle in `__psynch_cvwait` via
  crossbeam `Condvar::wait`.

Nothing is driving the reactor, so the I/O wakeup never arrives and the park is
permanent. Before the change the root future burned CPU re-polling; after it, it
sleeps. Neither completes.

**The fixture server is not the culprit.** `start_mirror_fixture_server`
(`src/search/model_download.rs:2158`) is a plain `std::net::TcpListener` on an
OS thread with a non-blocking accept loop. No asupersync anywhere in it. So the
failure is on the client side, below this repo.

Three read-only lanes reached the same place independently from source alone,
and each said plainly that it could not close the gap:

- `lanes/driver-034.md` — `current_thread()` is `worker_threads(1)`, a *separate*
  worker OS thread, so "the spawned task is never polled because the root Cx is
  unregistered" does not follow. The premise is true; the consequence is not.
- `lanes/driver-diff.md` — `block_on`, `run_future_with_budget` and `yield_now`
  are **byte-identical** between 0.3.2 and 0.3.4. The regression is not there.
- `lanes/channels.md` — swapping the channel type cannot help and would make it
  worse. That prediction was then confirmed empirically by the park above.

## 2. The change is still worth having, on its own merits

Three sites spawned a task purely to be handed a `Cx`, then read the result back
over a `std::sync::mpsc` receiver — which has no async wakeup, which is what
forced the spin. None of it was necessary: `Runtime::block_on` installs an
ambient `Cx` for the duration of the poll (asupersync #41), in **both** 0.3.2
(`builder.rs:3239`) and 0.3.4 (`builder.rs:3276`), byte-identical.

The replacement is this repo's own dominant working shape — `lanes/repo-precedent.md`
calls it Pattern A and finds it at `src/main.rs:285`, which runs the entire
application that way, and at `src/ui/app.rs:15441`.

It also removes a latent hazard: `src/pages/deploy_cloudflare.rs:843` had the
same shape and **no test coverage at all**, so it would have hung in production
with nothing to catch it.

A fourth `TryRecvError::Empty` site exists at `src/ui/app.rs:19812` and is
**correct as written** — a UI tick that polls once and returns, no loop, no
spawn. It is not part of this defect. Counted so that "three sites" is a
verified claim rather than an inherited one.

Landing still depends on the shipping-pin regression run (experiment A).

## 3. The pin ceiling is real after all — one level up from where it was looked for

The generation-11 handoff concluded there is "no toolchain reason cass is
sitting on 0.1.5", from fsqlite's declared `rust-version`. That reading of
fsqlite is correct and I reproduced it — 0.1.5, 0.1.14, 0.1.17 and 0.1.19 all
declare `rust-version = "1.85"`.

But the ceiling is not fsqlite's own MSRV, it is what the fsqlite → asupersync
edge drags in:

| fact | source |
|---|---|
| fsqlite 0.1.19 requires `asupersync 0.3.9` | `fsqlite-0.1.19/Cargo.toml` |
| asupersync 0.3.9 and 0.3.10 both require `sysinfo ^0.39` | their `Cargo.toml` |
| **every** published `sysinfo 0.39.x` (0.39.0–0.39.6) declares `rust-version = 1.95` | crates.io API, queried 2026-08-16 |
| this repo pins nightly, resolving to rustc **1.94.0-nightly (2025-12-10)** | `rust-toolchain.toml`, `rustc --version` |

Observed directly, not inferred — `cargo update -p asupersync --precise 0.3.10`
then building gives:

```
error: rustc 1.94.0-nightly is not supported by the following package:
  sysinfo@0.39.6 requires rustc 1.95
```

(`expB-verdict.log`.) asupersync 0.3.4 escapes this only because it requires
`sysinfo ^0.33`, and 0.33.x declares 1.74.

So the ceiling is genuine, and the remedy is a toolchain update rather than a
different pin. The repo's nightly is from 2025-12-10 — about eight months stale
as of today.

## 4. Toolchain probe — additive, and one unintended side effect, corrected

To test that without disturbing the other sessions building this repo on 1.94,
a **dated** nightly was installed alongside: `nightly-2026-08-10`, which is
rustc 1.99.0-nightly. The shared `nightly` was re-checked afterwards and is
still 1.94.0.

`rustup toolchain install` also set that new toolchain as the **default**, where
this machine previously had none configured. That is shared state and was not
intended, so it was put back with `rustup default none`, verified by
`rustup default` again reporting `no default toolchain is configured` and by
`rustc --version` inside the repo still reporting 1.94.0
(`restore-default.sh` output). Recorded here because it briefly changed machine
state for every session, not only this one.

## 5. Open, and explicitly not settled here

- **Why the 0.3.4+ client never completes against a live local TCP server.** Not
  root-caused. `lanes/driver-diff.md` narrows it to the worker/scheduler side —
  `three_lane.rs` has ~1081 changed lines between 0.3.2 and 0.3.4 that no lane
  audited — but nothing here proves the mechanism. It should not be described as
  understood.
- Whether that behaviour survives on 0.3.10 under rustc 1.99 (experiment B2, Q1).
- Whether fsqlite 0.1.19 then builds and passes (experiment B2, Q2).
