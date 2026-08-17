---
generation: 16
parent-session: df04cc54-dbaa-43de-b8ef-c76b8a70d4ea
next-action-class: user-owned
---

# Continuation — the last agent-doable item on this chain is done; what remains is Dale's

## The goal and authorization, verbatim

Dale, 2026-08-14:

> /my-way fix cass to completion and 100% green working state and completely up to date or tell me why it can't or /grill-me with any questions.

Sent mid-work the same day, as a correction to the work in flight:

> make sure that you are looking at the recent (last 2 weeks) work on cass and not regressing

Dale, 2026-08-15:

> your usage is good now. finish this to completion

Dale, 2026-08-16:

> if your senior dev recommendation is to delete those 4 stale directories do it. if that is what progresses us toward the goal. and I would prefer you just do that and only stop when it would break the pipeline or running sessions or if there is a true blocker or ambiguity or conflict rather than sitting here for something that I am going to just ask your recommendation on and agree to

And, granting two approvals a previous session acted on — Dale, 2026-08-16:

> do it. I approve a and b. set a /goal and run it to completion and do this /my-way

**Those approvals are SPENT and do NOT transfer to you.** Destructive and
external-write approvals expired with the ending session. You do not have
approval to delete any file, force-push, rewrite history, change repo
visibility, merge to `main`, file anything upstream on frankensqlite or
asupersync, open a public PR, or run `cass sources agents exclude` (that last
would destroy 3,877 conversations that exist nowhere else).

## Read this first, and do not re-derive any of it

- `thoughts/shared/handoffs/20260816-759l7-spin-wait-gen13/pin-move-cost.md` —
  the operator-facing decision document for the pin move. Still current.
- The gen-13 and gen-14 agent logs beside it.
- This file's parent, `p3kgr-upstream-continuation.md`, for the environment
  facts. They still hold, with **two corrections** in the section below.

## What generation 15 landed, verified and pushed

Branch `worktree-cass-759l7-spin-wait`, both commits on `origin`.

| commit | what |
|---|---|
| `d8f92763` | sidecar policy settled across enumeration, removal and relocation |
| `c1abf022` | bead closure + three new findings |

`d8f92763` did four things, and the last two are the ones the previous handoff
asked to be **decided**, not guessed:

1. `has_db_sidecar_suffix` gains `-wal-fec` and `-journal` (bead 7dewl, now
   closed). Unlike the `-fsqlite-ns-*` pair, both are creatable on the pin we
   ship today, so this is a live behaviour change.
2. `REMOVABLE_DB_SIDECAR_SUFFIXES` gains the same two. This was NOT in the bead,
   and it reverses my own first reading — see the correction below.
3. `copyable_bundle_sidecar_sources` — **no change**, now documented as
   deliberate. Copying the ns pair is inert, and harmful if a copy ever lands at
   mode 0644.
4. `quarantine_failed_seed_bundle` — the ns pair now **moves** with the bundle.
   Inert on the shipping pin; forward-correctness.

**Proof.** Clean suite 5156 passed / 0 failed / 3 ignored on the shipping pin.
Mutant run takes exactly the three new tests red at 5153/3, and gen-14's
`historical_bundle_discovery_skips_fsqlite_namespace_sidecars` stays GREEN under
that same mutant — the control proving the new tests discriminate the new
entries rather than re-testing the ns pair.

## The exact next action

**There is no agent-doable next action on this chain. Do not invent one.**

Every item that remained is Dale's, and each is blocked on a decision or an
approval that no session here holds. If you are an agent reading this because a
launcher started you anyway: stop, and report the four items below rather than
implementing any of them.

The one thing you MAY usefully do is answer a question from Dale about any of
them, or act on his answer once he gives it.

### 1. The `rootpage > 0` FTS gate — bead `coding_agent_session_search-hd4u5`

`fts_messages_present_cached` in `src/storage/sqlite.rs` gates every db-resident
FTS write on `AND rootpage > 0`. Stock SQLite writes rootpage `0` for a virtual
table; fsqlite 0.1.5 wrongly wrote 2-3, and 0.1.19 writes 0 correctly. So under
0.1.19 the gate goes false and all ten FTS write sites in
`insert_conversation_tree` silently no-op. Bounded — Tantivy is authoritative,
so search is never wrong — but the fallback index silently stops being
maintained.

**Not in the safe-today class.** The gate is equally wrong on the shipping pin
and only returns 1 there because 0.1.5 wrote a non-stock rootpage, so fixing it
changes production behaviour on the pin cass ships today. **Three** independent
sessions have now judged this Dale's call. Do not implement it without his
answer.

### 2. Merging `1fc20dbb` to `main`

The 759l7 fix is green on the shipping pin and pushed; `main` still carries the
spin. Background sessions may not merge.

### 3. The toolchain

`rust-toolchain.toml` pins bare `nightly`, resolving to rustc 1.94.0-nightly
dated 2025-12-10. Moving it changes the compiler for every session and worktree
in this repo at once. Note that `nightly-2026-08-10` is already installed
alongside it.

### 4. Two drafted-and-UNSENT upstream reports, both needing his say-so

Filing against frankensqlite is an external write.

- `coding_agent_session_search-mgw1o` — fsqlite 0.1.19 contentless FTS5 reports
  a stale `COUNT(*)` on the appending connection. Transient, no data loss.
- `coding_agent_session_search-ns-sidecar-transport-bricks-db-1mgjd` — carries
  two further notes: `namespace.rs:399` propagates `Err` where its own doc
  comment promises `Ok(false)`, and fsqlite-pager 0.1.19 has no regression test
  for the namespace-repair behaviour a pin move would depend on.

## Three findings generation 15 filed, with evidence — none is agent-blocked, all are unclaimed work

These are real and tracked. They are NOT part of the four decisions above and
any agent may pick them up from `br ready`.

- **`coding_agent_session_search-move-bundle-stale-hot-journal-gtfx5` (P1).**
  `move_database_bundle` renames only the main file, `-wal` and `-shm`, leaving
  `<db>-journal` at the canonical path — and cass then creates a fresh database
  at exactly that path (doctor quarantine; seed promotion; rollback restore).
  Measured on stock sqlite3 3.54.0: a journal from a *different* database was
  replayed into an unrelated populated database at the same path and destroyed
  it. fsqlite is worse — its zero-size guard covers only the `ExistingOnly`
  disposition, so under a create disposition it replays where stock discards.
  **Honest scope limit:** the mechanism is armed and unguarded, but the trigger
  was not demonstrated inside cass. The fix is to move the journal with the
  bundle or refuse the quarantine while one is present — never to delete it.
- **`coding_agent_session_search-export-temp-sidecar-orphans-gd0dm`.** The pages
  export sweeps its temp-database sidecars only when the export failed AND the
  atomic replace was never attempted, so the success path and a failed replace
  both leak into the user's output directory.
- **`coding_agent_session_search-ns-sidecar-transport-bricks-db-1mgjd`.** Never
  transport the `-fsqlite-ns-*` sidecars through git, tar, or an image layer.
  The record's *content* self-heals, but the sidecar *file's* inode metadata does
  not: at mode 0644 fsqlite refuses the open permanently, with an error naming
  the sidecar rather than the database.

## Environment facts — two CORRECTIONS to the parent handoff

The parent's list still holds except for these. Both cost this generation real
time.

1. **The baseline is 5153, not 5151.** The parent says "expect 5151/0/3". That
   was true before gen-14's `9d0a58cd`, which added two tests. Before this
   generation the number was **5153/0/3**; after `d8f92763` it is **5156/0/3**.
2. **`cargo` on this session's PATH is the WRONG compiler.** `/opt/homebrew/bin`
   precedes `~/.cargo/bin`, so a bare `cargo` is Homebrew's **stable 1.96**.
   `rust-toolchain.toml` pins `nightly` and the crate graph uses `#![feature]`,
   so the stable compiler dies with `E0554: #![feature] may not be used on the
   stable release channel` in `fsqlite-pager` before a single test runs. There
   is no `rustup` on PATH either. Prepend it:
   `export PATH="$HOME/.cargo/bin:$PATH"`, then confirm
   `rustc --version` reads `1.94.0-nightly (f52090008 2025-12-10)`.
   Working scripts are at
   `~/.claude-accounts/george/jobs/df04cc54/tmp/{run-suite,run-mutants,reverify}.sh`
   — the job dir is deleted with the job, so copy them if you want them.

Still true from the parent, and still worth obeying: the worktree's own `target/`
is the warm one, so **do not set `CARGO_TARGET_DIR`** (that forces a cold
multi-GB rebuild); a full `cargo test --lib` is ~2 min compile + ~2 min tests;
disk is ~70 GiB against a 150 GiB floor; there is no `timeout`/`gtimeout`; `br`
does not work from inside the worktree (run it from
`~/dev/coding_agent_session_search`, then copy `.beads/issues.jsonl` into the
worktree and commit it there); and `agent-dirtiness.py sync-gate` reports
`base_not_ancestor_of_remote` on this branch because the tool assumes main-based
work.

One more, learned the hard way: **`${PIPESTATUS[1]}` after `cargo test | tee` is
`tee`'s status, which is essentially always 0.** A first run of the suite
reported exit 0 over a compile that had failed outright. Use `${PIPESTATUS[0]}`,
and read the log rather than the exit code.

## A note on siblings

Generations 12, 13 and 14 were all live in this shared worktree when generation
15 started. Both 13 and 14 replied to a `SendMessage` within minutes, confirmed
they were read-only or winding down, and handed over useful corrections — that
exchange is cheap and it worked. `HEAD` also moved under this session mid-run
(gen-14's `de61c316` napkin commit) with the tree clean, which is normal here
rather than alarming. Run `ListAgents` before writing, and pathspec-bound every
commit (`git commit -- <exact paths>`).
