---
generation: 11
parent-session: a91c2501-1830-4d3d-9430-3c9afe08a63c
next-action-class: executable
---

# Continuation — the pin verdict collapsed, and the ceiling that produced it was never real

## The goal and authorization, verbatim

Dale, 2026-08-14:

> /my-way fix cass to completion and 100% green working state and completely up to date or tell me why it can't or /grill-me with any questions.

Sent mid-work the same day, as a correction to the work in flight:

> make sure that you are looking at the recent (last 2 weeks) work on cass and not regressing

Dale, 2026-08-15:

> your usage is good now. finish this to completion

Dale, 2026-08-16:

> if your senior dev recommendation is to delete those 4 stale directories do it. if that is what progresses us toward the goal. and I would prefer you just do that and only stop when it would break the pipeline or running sessions or if there is a true blocker or ambiguity or conflict rather than sitting here for something that I am going to just ask your recommendation on and agree to

And, granting the two approvals this session acted on — Dale, 2026-08-16:

> do it. I approve a and b. set a /goal and run it to completion and do this /my-way

where (a) was filing two measured defects upstream on frankensqlite and (b) was
deleting six idle cargo target directories from `/tmp`.

**Those approvals are SPENT and do NOT transfer to you.** (b) is done. (a) was
investigated and correctly NOT exercised — see below — so no upstream write was
made, and you do not inherit permission to make one. You also do not have
approval to delete any file, force-push, rewrite history, change repo
visibility, or run `cass sources agents exclude` (that last would destroy 3,877
conversations that exist nowhere else).

## What this session settled, and it is mostly negative

**Bead `p3kgr`'s RED verdict does not survive. Neither half of it.** The pin
bump to fsqlite 0.1.14 was recorded as blocked by two independent upstream
defects. On investigation, one is ours and the other is not a defect.

**Half one is our own code.** Three sites hand-roll a spin-wait: spawn a task
onto the runtime, then poll a `std::sync::mpsc` receiver in a loop with
`yield_now()` between attempts.

```
src/update_check.rs:852            -> 12 hanging tests
src/search/model_download.rs:1022  -> 4 hanging tests
src/pages/deploy_cloudflare.rs:843 -> NO test coverage, latent
```

Under `block_on` on a `current_thread` runtime this is a self-deadlock by
construction. asupersync's own OPEN issue #58 documents why: the `block_on`
root future is outside task accounting, so the spawned task never gets polled
from the root's yield. asupersync 0.3.2 tolerated it; 0.3.4 does not. Filed as
**`759l7`**. The third site is the reason it is a code defect rather than a test
problem — nothing exercises it, so it will hang in production uncaught.

**Half two is correct library behaviour.** The two open-refusal tests build a
deliberately corrupt schema — a duplicate `sqlite_master` row for
`fts_messages`, inserted under `PRAGMA writable_schema=ON` — and then assert the
open succeeds. Stock SQLite refuses the same file:

```
$ sqlite3 dup.db "SELECT * FROM messages;"
Parse error: malformed database schema (fts_messages) - table fts_messages already exists (11)
rc=1                                              # sqlite3 3.54.0
```

So 0.1.14 refusing it is SQLite-compatible, and 0.1.5 accepting it was the
deviation our repair path had come to depend on. The other two failures compare
`FtsConsistencyRepair`, a private consumer enum at `src/storage/sqlite.rs:1790`
computed by our own code from a probe that swallows errors through
`unwrap_or(false)`. There is nothing upstream can act on in either.

**The A/B was also confounded**, which nobody had noticed: the two binaries do
not hold the same tests. 0.1.5 reports `5133 filtered out` (5134 total), 0.1.14
reports `5136 filtered out` (5140 total). Six tests differ, so the source was
not identical across the comparison, and asupersync moved 0.3.2 → 0.3.4 in the
same step without ever being held constant.

**THE ACTIONABLE FINDING: the pin ceiling is false.** The recorded reason for
capping at 0.1.14 was that later releases require rustc 1.95 while we run 1.94.
Measured directly in the vendored manifests:

| crate | declared `rust-version` | vendored locally |
|---|---|---|
| fsqlite-core 0.1.14 | **1.85** | yes |
| fsqlite-core 0.1.17 | **1.85** | yes |
| fsqlite-core 0.1.19 | **1.85** | yes |

Our nightly is 1.94.0. Upstream has since shipped through 0.3.4. There is no
toolchain reason cass is sitting on 0.1.5.

## The exact next action

1. **Fix `759l7`** — replace the three spin-waits. A `std::sync::mpsc` receiver
   has no async wakeup, which is what forces the spin; await the spawned task
   instead of spawning and polling, or use a channel the runtime can wake. Give
   `deploy_cloudflare.rs:843` test coverage so the untested site stops being
   untested.

2. **Then retry the pin against 0.1.19, not 0.1.14**, on a single tree with
   only the pin varying, so the A/B is actually controlled. `verify-fsqlite-pin.sh`
   is the runner; its `=0.1.14` guard needs updating for the new target.

3. **Expect the two open-refusal tests to still fail, and treat that as a TEST
   defect.** They assert a corrupt-schema database opens. SQLite does not do
   that. The repair route for duplicate `sqlite_master` rows is
   `writable_schema`, the same as SQLite's own.

## Do NOT file these upstream

This session drafted an issue, ran it past a cold reader and an independent
skeptic, and both found it unfilable. The skeptic reproduced the refusal against
stock sqlite3 in one command. Filing would have publicly blamed a maintainer for
SQLite-compatible behaviour, and the first reply would have been that command.

One residual is real but not filed: fsqlite's error text says the content shadow
table is *missing* when it is present (`fts_messages_content` exists in the
corrupt database alongside `_config`, `_data`, `_docsize`, `_idx`). The refusal
is right; the diagnosis is misleading. Worth filing on its own **once someone
builds a clean standalone reproduction** — and that needs Dale's approval, which
is not inherited.

## Also done this session

**98 GiB reclaimed** (46 → 144 GiB free), six idle cargo targets, bead `3azjb`
closed. `/tmp/cass-gen8-target` was deliberately spared: a census had marked it
safe, and a build started writing into it in the six minutes before the
deletion. Re-verify liveness immediately before any future reclaim rather than
trusting an earlier census.

**An evidence claim was withdrawn.** Earlier notes cite a profiler stack showing
1203 of 1214 samples in `run_future_with_budget -> yield_now`. It is in no saved
artifact: recursive search of `~/.cass-catchup` for `run_future_with_budget`,
`Call graph`, and `Analysis of sampling` returns zero hits, while a positive
control on `test result:` in the same directory returns three files. Do not cite
that stack as measured.

## State

- `main` is green on fsqlite 0.1.5 and is the shipping pin. Do not merge
  `worktree-cass-gen5-honesty` — it carries the 0.1.14 bump and is refused.
- Open: `p3kgr` (carries both corrections as comments), `759l7`, `9fnbr`,
  `qtn0e`.
- Other live sessions are working this repo. Check `claude agents` before
  assuming a stale branch or dirty file is yours.

## Environment facts that cost real time

1. `export PATH="$HOME/.rustup/toolchains/nightly-aarch64-apple-darwin/bin:$PATH"` — an absolute path to nightly cargo is not enough.
2. `cargo test --lib` shells out to the INSTALLED `cass` binary, so a broken install looks like a compiler problem.
3. `--json` sets robot mode and ignores `RUST_LOG` (`src/lib.rs:5769-5775`). Add `--verbose`.
4. Deploy by atomic rename, never `cp` over the live path — stale signature cache gives SIGKILL.
5. No `timeout`/`gtimeout` here. Background + poll + kill.
6. **zsh does not word-split an unquoted parameter.** `for d in $LIST` runs once with the whole string. Cost a wrong "all six directories missing" reading this session. Use a real array or `${=LIST}`.
7. `ps -Ao command | rg "$path"` matches its own pipeline. Use `lsof +D` for a check that cannot self-match.
