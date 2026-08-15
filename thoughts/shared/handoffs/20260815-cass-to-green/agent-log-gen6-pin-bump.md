# gen6 — the frankensqlite pin-bump experiment

Session: `af6e155f` (background), generation 4 of the cass-to-green chain.
Branch: `worktree-cass-gen5-honesty`. Bead: `coding_agent_session_search-p3kgr`.
Resumed from `cass-green-continuation-g3.md` at `853ca11a` via the resume-handoff
autolaunched direct path.

Solo by constraint, not by choice: the signed-in account (`katherine`) measured
**100% of its weekly window** at session start, and AGENTS.md §3.9 forbids
launching lanes at or above 95%. No subagents, no workflow fan-out. Every number
below was produced by a command in this session.

## Conclusion first

The pin bump is real and it is **not** a bump to 0.1.17. The version that both
carries the fix and builds on this machine is **0.1.14**, and getting there needs
three exact-version pins in `Cargo.toml` rather than a single number edit.

The handoff's stated plan — `cargo update -p fsqlite --precise 0.1.17` — does not
work, for two independent reasons discovered by running it.

## What the bump actually does

`cargo update -p fsqlite --precise 0.1.17` returns rc=0 and looks fine. It is not:

- It moves **fsqlite to 0.1.17 but every sibling to 0.1.19**, because
  `fsqlite 0.1.17` requires its siblings at `^0.1.17` and cargo takes the newest
  match. `--precise` did not hold them even when all twenty family members were
  named with their own `-p` flag; the run was byte-identical to the one-package run.
- It rewrites **522 insertions / 405 deletions** of `Cargo.lock` — aes-gcm, the
  objc2 family, five windows crates, sysinfo — none of it fsqlite.

`cargo check --lib` against that resolution fails in 2 seconds:

```
error: rustc 1.94.0-nightly is not supported by the following package:
  sysinfo@0.39.6 requires rustc 1.95
```

## The constraint chain, measured end to end

`sysinfo` is not an fsqlite dependency at all. The lock says exactly one package
pulls it:

```
DEPENDENT: asupersync 0.3.10
```

Reading the sparse index for both crates gives the whole chain:

| crate | versions | requires |
|---|---|---|
| `fsqlite-types` | 0.1.10 – 0.1.14 | `asupersync ^0.3.4` |
| `fsqlite-types` | 0.1.15 – 0.1.19 | `asupersync ^0.3.9` |
| `asupersync` | 0.3.3 – 0.3.4 | `sysinfo ^0.33` |
| `asupersync` | 0.3.5 – 0.3.10 | `sysinfo ^0.39` |
| `sysinfo` | 0.38.4 and below | rustc 1.88 |
| `sysinfo` | all of 0.39.x | **rustc 1.95** |

Installed toolchain: `rustc 1.94.0-nightly (f52090008 2025-12-10)` — eight months
stale against a `rust-toolchain.toml` that pins only `channel = "nightly"`.

So **every fsqlite at or above 0.1.15 requires rustc 1.95**, transitively, and
this machine has 1.94.

## Where the fix actually landed — bisected

The lane doc measured `ExistsValueSet` at 0 in fsqlite-core 0.1.5 and 8 in
0.1.17. Downloading the intermediate releases and grepping the same file narrows
it to a single release boundary:

| fsqlite-core | `ExistsValueSet` | `correlated_exists_fallback` |
|---|---|---|
| 0.1.5 | 0 | 1 |
| 0.1.8 | 0 | 1 |
| 0.1.10 | 0 | 1 |
| **0.1.11** | **8** | 1 |
| 0.1.12 – 0.1.17 | 8 | 1 |

Both directions present, so the zeros are real absences rather than a dead
instrument. **The fix landed in 0.1.11.**

That is the whole result: the fix is available from 0.1.11, and the rustc-1.95
wall starts at 0.1.15. **0.1.14 is the only sensible target** — newest release
carrying the fix that still builds here — and its `ExistsValueSet` code is
identical in count to 0.1.17's, so it tests the same hypothesis.

`correlated_exists_fallback` is still present at 0.1.17, confirming the lane
doc's correction: the fix adds a fast path that avoids taking the fallback, it
does not delete it. Grepping for that string will not tell you whether a build is
fixed.

## Why `--precise` could never work

cass declares three of these crates directly, all with caret ranges:

```
Cargo.toml:26   asupersync    = { version = "0.3.2", ... }
Cargo.toml:45   frankensqlite = { version = "0.1.5", package = "fsqlite", ... }
Cargo.toml:181  fsqlite-types = { version = "0.1.5", package = "fsqlite-types" }
```

A caret range in the manifest outranks `cargo update --precise` on the next
resolution, which is why the family kept floating back to 0.1.19. The pins have
to live in the manifest.

**The `=` prefix is load-bearing, and that was measured rather than assumed.**
I first tried the bare form `version = "0.1.14"`, matching the repo's existing
convention and relying on the committed `Cargo.lock` for exactness. It does not
hold: changing the requirement at all forces cargo to re-resolve, and a caret
range then takes the newest match, so `0.1.14` resolved straight back to
**fsqlite 0.1.19 / asupersync 0.3.10 / sysinfo 0.39.6** and failed on the same
rustc-1.95 error. Only `=0.1.14` / `=0.1.14` / `=0.3.4` pins the family down.

One trap worth recording: a bare `cargo update` after editing those pins resolves
the pins correctly *and* drags the entire rest of the graph to latest, which
imports `kstring 2.0.4` needing rustc 1.96 — a different wall, same shape. After
a manifest pin change the correct move is to restore the committed lock and let
`cargo check` make the minimal adjustment.

## The change surface is three files, because the repo enforces it

`build.rs` carries a `CONTRACTS` table and fails the build when `Cargo.toml`
drifts from it:

```
cargo:error=dependency source contract violation for frankensqlite facade:
  dependency `frankensqlite` in [dependencies] must pin version = `0.1.5`, found `=0.1.14`
  update Cargo.toml, build.rs, and the README dependency source contract together
```

That guard is doing exactly its job, so the change is `Cargo.toml` + `build.rs` +
`README.md` together. Three contracts moved: frankensqlite facade and
frankensqlite shared types to `0.1.14`, asupersync to `0.3.4`.

One line of `build.rs` logic changed, and only one. `expected_version` is
compared in **two** places against two different kinds of string: the manifest
*requirement* (`validate_manifest_dependency_version`) and, under the opt-in
`strict-path-dep-validation` feature, a sibling repo's bare `[package] version`
(`validate_local_sibling_manifest`, build.rs:546). A single constant cannot be
both `=0.1.14` and `0.1.14`, so the requirement check now strips an optional
leading `=` before comparing. `=X` is strictly narrower than the caret default
and satisfies the contract's intent, so rejecting it was a false positive; every
other spelling still fails.

The relaxed check accepts exactly two spellings and no others: `0.1.14` and
`=0.1.14`. `^0.1.14`, `>=0.1.14` and any other version still fail, because the
strip only removes a leading `=`. It deliberately still accepts the bare form
even though we now know the bare form drifts — making the guard *require* `=`
would need a new per-contract field, and the bare form already fails loudly at
the rustc error with the explanation sitting in the comment beside it. Not worth
growing the mechanism for.

The README's row was **already stale before this session** — it read `0.1.4`
while the pin had been `0.1.5`, because the build-time guard reads `Cargo.toml`
and `build.rs` but cannot read prose. Corrected to `0.1.14` / `0.3.4`, with the
rustc ceiling written next to it so the next reader knows why not newer.

## Result: `cargo check --lib` is green

```
Checking fsqlite-core v0.1.14
Checking fsqlite v0.1.14
Finished `dev` profile [unoptimized + debuginfo] target(s) in 34.53s
CHECK_RC=0
```

Resolved: `fsqlite 0.1.14`, `fsqlite-core 0.1.14`, `fsqlite-types 0.1.14`,
`asupersync 0.3.4`, `sysinfo 0.33.1`, `kstring 2.0.2`. Lock churn is 268
insertions / 382 deletions — against 522/405 for the naive 0.1.17 bump.

Zero errors and zero warnings. **This proves the pin set compiles. It proves
nothing about the wedge**, exactly as the handoff warned.

## Two corrections to the handoff's environment facts

- **The live archive is 17.1 GB, not 7.9 GB.** `agent_search.db` measured
  17,116,061,696 bytes at 2026-08-15 13:18, and the whole cass data directory is
  **51 GB**. Any plan costing "a copy of the archive" costs 17 GB, not 8.
- **`-0gzok` was already closed** before this session started, with a closure
  reason recording both parts fixed. The handoff's "nobody has closed the bead
  yet" is stale; no action was needed.

## The falsifier specimen exists and is affordable

`~/backups/cass/agent_search-20260814-vacuum.db` is 3.98 GB and holds
**580,374 messages** — the exact count the wedge was measured against in
`raise_lexical_rebuild_footprints_to_exact_message_counts`. So the throwaway
rebuild target costs 4 GB rather than 17 GB, at full message scale. That is the
right specimen, and it is the one to use when disk allows.

## The suspect query has no correlated EXISTS in it

The handoff named `raise_lexical_rebuild_footprints_to_exact_message_counts`
(`src/storage/sqlite.rs:7486`) as the leading suspect for the full-rebuild wedge,
and told the next session to read where the WARN fires rather than guess. Reading
it, the statement is:

```sql
SELECT conversation_id, COUNT(*) AS message_count
FROM messages
GROUP BY conversation_id
ORDER BY conversation_id ASC
```

**There is no `EXISTS` in it, correlated or otherwise.** The fsqlite fix is a
set-based path for correlated `EXISTS` (`ExistsValueSet`). So if this statement is
what wedges the full rebuild, the pin bump may not touch it at all — the two
wedges plausibly have two different causes, exactly as the fork-answer lane
suspected. That is an argument for measuring, not for assuming the bump fixed it.

Reference timing for that statement on real SQLite, read-only against the backup:

```
QUERY PLAN
`--SCAN messages USING COVERING INDEX sqlite_autoindex_messages_1
12722 rows,  0.032s total
messages=580374  conversations=12722
```

**32 milliseconds** on stock SQLite against reported ">20 min" in frankensqlite —
so the data and the schema are not the problem, the engine's plan for it is. Note
`sqlite_autoindex_messages_1` is the *only* index on `messages`, in both the
backup and the live archive. That is intentional and not a defect:
`src/storage/sqlite.rs:3559` deliberately drops `idx_messages_conv_idx` because
the autoindex covers the same key, and the `INDEXED BY` use at :7563 is guarded by
a `no such index` fallback at :7570. Checked before reporting; it is not a bug.

## What the bump DOES measurably fix — a controlled A/B

The wedge falsifier is blocked on disk (below), so this is the largest
measurement that was affordable. It is a real one, and it found something.

Two binaries, one specimen, identical argv:

```
source fixture : tests/fixtures/search_demo_data/agent_search.db  sha=93d8e02f2046f2f9
ab-old.db      : sha=93d8e02f2046f2f9
ab-new.db      : sha=93d8e02f2046f2f9
OLD binary     : sha=49fbba6e3789c252 mtime=2026-08-15T17:45:05Z   (fsqlite 0.1.5)
NEW binary     : sha=572ae86d4a2a2ae5 mtime=2026-08-15T18:29:38Z   (fsqlite 0.1.14)
```

```
OLD:  WARN could not read connector scan coverage floors; reporting coverage as unchecked
        error=not implemented: reloading populated WITHOUT ROWID table `fts_messages_idx`
        into MemDatabase is not yet supported
      Conversations: 2   Messages: 6
      Scan Coverage: UNKNOWN — the coverage read did not complete

NEW:  (no WARN)
      Conversations: 2   Messages: 6
      Scan Coverage: complete (no connector scan has aborted)
```

Both controls fire: the old binary never prints `Scan Coverage: complete` on this
specimen (0 occurrences), and the new binary never prints `not yet supported`
(0 occurrences). Both report identical counts, so both read the data correctly —
the only thing that changed is whether the coverage read *completed*.

**This is the honesty family's own subject, from the other end.** `-nvq59`,
`-a59ou`, `-ddkwa` and `-xarzt` were all about making cass tell the truth when the
connector-coverage read FAILS. The failure is a named frankensqlite 0.1.5
limitation — it cannot reload a populated `WITHOUT ROWID` table into MemDatabase —
and 0.1.14 does not have it. The honesty work made the failure legible; the pin
bump removes the failure.

It also lowers the stakes on `-xarzt` (should "could not check" degrade the
one-word verdict?) without answering it. Under 0.1.14 the read succeeds on this
specimen, so the surface reports a real value rather than a defensible unknown.
The design question is unchanged and is still Dale's.

Specimen discipline, because this file already records a session that got it
wrong: every sha and mtime above was captured in the same run that produced the
readings, and the pre-bump binary was copied aside *before* the release build
overwrote the target path. `~/.local/bin/cass` and
`/tmp/cass-repair-target/release/cass` were confirmed to be separate inodes
(`2238297969` vs `2238296974`, both `links=1`), so the release build did not
silently re-deploy the installed binary.

## Why the wedge falsifier did NOT run

It needs a throwaway archive. The cheapest faithful specimen is the 3.98 GB
vacuum backup (580,374 messages). The arithmetic does not clear:

| | GB |
|---|---|
| free at session end | ~29–30 |
| the codex catch-up's own guard floor | 25 |
| usable headroom | ~4–5 |
| the copy alone | 4 |
| what a rebuild's prep then writes | unbounded; `raw-mirror` on the live dir is **32 GB** |

And the copy would be **permanent**. This repo's `AGENTS.md` RULE 1 forbids
deleting any file without express written permission, including a file the agent
created itself, so I have no reclaim path for the 4 GB after the test. Spending
the last of a shared, critical, unreclaimable resource — on a machine whose own
disk-janitor already reports PARTIAL against a 150 GB floor — is an operator
decision, not an agent one.

The unblock already exists and is already filed: bead
`coding_agent_session_search-reclaim-tmp-cargo-targets-jck92`, ~82 GB of stale
`/tmp` cass cargo target dirs from worktrees with no live session. It needs
Dale's written deletion approval. `/tmp/cass-repair-target` (23 GB) is IN USE and
must be excluded from any such reclaim.

## Proof boundary

- A green `cargo check` proves the pin set compiles. It proves **nothing** about
  the wedge. The falsifier is the full rebuild, against a throwaway archive copy.
- Nothing here has been deployed and nothing has run against the live archive.
- `/System/Volumes/Data` was 30–32 GB free throughout, against the catch-up's
  25 GB guard floor. No cargo target directory was created or deleted; the warm
  `/tmp/cass-repair-target` was reused as the handoff instructed.
- The peer session owning the codex catch-up (`-2bh4a-g1`) was live and was not
  competed with.
