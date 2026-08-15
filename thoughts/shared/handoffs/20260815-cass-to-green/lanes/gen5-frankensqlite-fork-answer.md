# gen5 — answering "should we make a local fork of frankensqlite and fix it?"

Dale asked this directly on 2026-08-15. Bead
`coding_agent_session_search-p3kgr`. Investigated by session 09a898c8 from files
and from a real build's own output; no fork was cloned and no `[patch]` block was
enabled.

## Short answer: no. Bump the pin instead — upstream already fixed it.

The engine defect that wedges cass is fixed six releases past the version this
repo pins, and the fix is already sitting in the local cargo registry cache.

## Evidence

**1. The pin is 0.1.5, and the defect is in it.**
`Cargo.toml:45` — `frankensqlite = { version = "0.1.5", package = "fsqlite",
features = ["fts5"] }`. The failing dispatch path names itself in the runtime
WARN Dale captured on the bead: `decision_reason="correlated_exists_fallback"`.
That literal is present in the pinned source:

```
rg -c 'correlated_exists_fallback' .../fsqlite-core-0.1.5/src/connection.rs  -> 1
```

**2. The upstream fix is real, and it is a new fast path rather than a deletion.**

```
rg -c 'ExistsValueSet' .../fsqlite-core-0.1.5/src/connection.rs   -> 0
rg -c 'ExistsValueSet' .../fsqlite-core-0.1.17/src/connection.rs  -> 8
```

Same file, same instrument, both directions — so the zero is a real absence, not
a dead probe. **One correction to the bead comment**, which said issue #117 "was
fixed" such that the fallback is gone: `correlated_exists_fallback` is still
present in 0.1.17 (1 occurrence). What 0.1.17 adds is the set-based
`ExistsValueSet` path that avoids *taking* the fallback. The distinction matters
if anyone greps for the old string to check whether a build is fixed — the string
alone will not tell them.

**3. fsqlite 0.1.17 is already vendored on this machine**, at
`~/.cargo/registry/src/index.crates.io-*/fsqlite-0.1.17`, so evaluating the bump
needs no network.

## The fork wiring in this repo is broken, independent of whether we fork

This is worth fixing regardless, because anyone following the repo's own
instructions today gets a silent no-op.

`AGENTS.md` RULE 2 says to fix frankensqlite rather than route around it, and
names `/data/projects/frankensqlite` — **a path that does not exist on this
machine.** Dale does already have a public fork at `carmandale/frankensqlite`
(forked from `Dicklesworthstone/frankensqlite`).

The commented override at `Cargo.toml:244-246` is:

```toml
# [patch."https://github.com/Dicklesworthstone/frankensqlite"]
# fsqlite = { path = "../frankensqlite/crates/fsqlite" }
```

That is a **git-source** patch table. `fsqlite` is a **registry** dependency:

- `Cargo.toml:45` declares it by `version`, with no `git =`.
- Every `fsqlite*` entry in `Cargo.lock` reads
  `source = "registry+https://github.com/rust-lang/crates.io-index"`.
- `rg -c 'Dicklesworthstone/frankensqlite' Cargo.lock` -> **0 hits.**

A `[patch."<git-url>"]` table only rewrites dependencies resolved from that exact
git source, so this one matches nothing in the graph. Uncommenting it would leave
the registry copy in use. The form that would actually work is
`[patch.crates-io]`.

Independent confirmation from an executed build rather than from file reading —
this session's own `cargo build --release` output distinguishes the two source
kinds by whether it prints a URL:

```
Compiling fsqlite v0.1.5
Compiling franken-agent-detection v0.1.8 (https://github.com/Dicklesworthstone/franken_agent_detection?rev=b62d8597...)
Compiling frankensearch v0.3.2 (https://github.com/Dicklesworthstone/frankensearch?rev=2cad158f...)
```

`fsqlite` has no URL beside it; its two siblings do. Those two siblings are
exactly the ones whose commented patch tables ARE git-source tables and would
work as written.

## What this does not settle

- **I did not run the bump.** Whether cass builds and passes against fsqlite
  0.1.17 is unmeasured. `fsqlite-vfs` is already resolved at 0.1.6 while the rest
  of the family sits at 0.1.5, so the family is not uniformly pinned and a bump
  may not be a single-number edit.
- **I did not prove 0.1.17 fixes the cass symptom.** The chain — the WARN string
  is in 0.1.5, `ExistsValueSet` arrives by 0.1.17, upstream #117 names the same
  string — is strong circumstantial evidence, not a measured before/after on the
  live archive. The measurement that would settle it is a `--full` rebuild under
  the bumped pin, and that path is the one bead p3kgr records as *still* wedged.
- **Two distinct wedges, and only one is resolved.** The incremental /
  `--watch-once` path is already unwedged in production by
  `CASS_SKIP_PREFLIGHT_CLEANUP_ORPHAN_FK_ROWS=1`. The FULL REBUILD path is still
  wedged with that variable set, and its leading suspect is the unconditional
  `GROUP BY` over all 580,374 messages in
  `raise_lexical_rebuild_footprints_to_exact_message_counts`
  (`src/storage/sqlite.rs:7456`) — a cass-side design question that a
  frankensqlite bump might not touch at all.
- **I did not enable any `[patch]` block**, because
  `.github/workflows/fresh-clone-build.yml` fails the build if a sibling-path
  patch is committed. A local-only override belongs in an uncommitted
  `.cargo/config.toml`, not in `Cargo.toml`.

## Recommendation

1. Fix the misleading comment so it names `[patch.crates-io]` — cheap, and it
   stops the next person losing an afternoon to an override that cannot apply.
2. Try the pin bump to 0.1.17 on a branch and measure a `--full` rebuild. That is
   the experiment that would retire p3kgr, and it costs one build.
3. Fork only if the bump turns out to be insufficient. RULE 2's instinct is right
   — fix the engine, do not route around it — but "fix" here most likely means
   "use the version where upstream already fixed it."
