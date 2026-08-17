---
generation: 15
parent-session: 0faeab5e-ad5e-4491-b087-25103f2e4a10
next-action-class: executable
---

# Continuation — the triage is finished; what is left is one small fix and two calls that are Dale's

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

## The triage is DONE. Do not re-derive any of it.

All 8 forward-line failures are classified, every classification survived an
adversarial verifier, and the operator-facing document is written and pushed:

- `thoughts/shared/handoffs/20260816-759l7-spin-wait-gen13/pin-move-cost.md` —
  complete, all 8 rows, written by the gen-13 session at `67bfa6eb`.
- `thoughts/shared/handoffs/20260816-759l7-spin-wait-gen13/agent-log-gen13.md` —
  gen 13's evidence trail.
- `thoughts/shared/handoffs/20260816-759l7-spin-wait-gen14/agent-log-gen14.md` —
  gen 14's, including a correction section worth reading before you launch
  anything (below).

**Read `pin-move-cost.md` before doing anything else.** It is the decision
document and it is current.

### One process lesson that cost this generation a fan-out

I read the gen-13 triage workflow's `journal.jsonl` and treated it as a finished
record. It was a **live file** — the workflow was still running, and it completed
minutes later with all 10 agents returned. I launched lanes to re-derive results
that already existed. The tell was in my own output: the handoff said 4 of ~10
had landed, I counted 6, and I explained the gap as "late arrivals" rather than
as evidence of a running producer. **A workflow does not stop when the session
that launched it stops.** `stat` the journal twice before concluding anything
from its contents.

## What this generation landed, verified and pushed

Branch `worktree-cass-759l7-spin-wait`. All green on the shipping pin.

| commit | what |
|---|---|
| `22dce91a` | the two safe-today fixes: `encrypt.rs` stops pinning std's half of an error string; `SIDECAR_SUFFIXES` gains `-fsqlite-ns-gate`/`-fsqlite-ns-use` |
| `5eb5a2ce` | promoted the shared-`CARGO_TARGET_DIR` napkin row into `AGENTS.md`'s `### Unit Tests`, which is the section that caused it |
| *(see git log)* | the `xybl9` sweep: `remove_database_files` and `export.rs`'s temp cleanup share one `REMOVABLE_DB_SIDECAR_SUFFIXES` list, plus two regression tests |

Shipping-pin proof, run twice: **5151 passed, 0 failed, 3 ignored**, identical to
the recorded baseline. Verified the edits were compiled into the binary that ran,
by content (`strings` reports `fsqlite-core-0.1.5`) and by a positive control on
a string only the change introduces — not by mtime.

## The exact next action

**Implement and verify bead
`coding_agent_session_search-sidecar-suffixes-missing-wal-fec-jou-7dewl`.**
Read the bead first — it carries the full evidence.

`has_db_sidecar_suffix` (`src/storage/sqlite.rs`, the `SIDECAR_SUFFIXES` const)
is still missing two suffixes:

- `-wal-fec` — **not** new in 0.1.19. `fsqlite-wal-0.1.5/src/wal_fec.rs:2119`
  builds it with `format!`, and its own tests pin `test.db-wal` →
  `test.db-wal-fec`. It does not end with `-wal`, so the existing entry does not
  cover it.
- `-journal` — the stock rollback journal. cass shells out to the system
  `sqlite3` binary in two production paths (`src/storage/sqlite.rs:2182`, `:2482`),
  either of which can create one.

Unlike the pair already added, **these are creatable on the pin we ship today**,
so adding them is a live behaviour change, not a no-op. That is exactly why they
were filed rather than folded into `22dce91a`. Verify with the full library suite
on the shipping pin and expect 5151/0/3; if a count moves, read why before
touching a test.

While you are there, decide the two sites this generation deliberately left
alone, and say which way you went and why:

- `copyable_bundle_sidecar_sources` (`src/storage/sqlite.rs:1622`) — a **copy**.
- `src/indexer/mod.rs:15013` — an `fs::rename`, i.e. a **move**.

Both relocate a database. The `-fsqlite-ns-use` file is a 40-byte *per-file
identity record* (`fsqlite-vfs-0.1.19/src/namespace.rs`, `write_identity_record`),
not a path record, and `read_identity_record` refuses a mismatch. So whether it
may travel depends on whether that identity survives a copy or a rename — which I
did not establish, and guessing could make a copied bundle unopenable. Establish
it, then act. `bead xybl9` step 2 lists both sites but does not settle this.

Do **not** add the ns suffixes to `bundle_total_bytes` (`:2025`), and leave
`is_backup_root_name` (`:2998-3000`) loose — `xybl9` settles both, per commit
`37b42058`.

## Then stop. What remains is Dale's, and it is the real headline.

Three things are blocked on him, and the first is the most important line in the
whole chain:

1. **The `rootpage > 0` FTS gate — bead `coding_agent_session_search-hd4u5`.**
   `fts_messages_present_cached` (`src/storage/sqlite.rs:4127-4131`) gates every
   db-resident FTS write on `AND rootpage > 0`. Stock SQLite writes rootpage `0`
   for a virtual table; fsqlite 0.1.5 wrongly wrote 2-3, and 0.1.19 writes 0
   correctly. So under 0.1.19 the gate goes false and all ten FTS write sites in
   `insert_conversation_tree` silently no-op. Measured on both prebuilt rlibs
   with a live positive control and stock sqlite3 3.54.0 as ground truth.
   Bounded — Tantivy is authoritative, so search is never wrong — but the
   fallback index silently stops being maintained.
   **This is not in the safe-today class.** The gate is equally wrong on the
   shipping pin and only returns 1 there because 0.1.5 wrote a non-stock
   rootpage, so fixing it changes production behaviour on the pin cass ships
   today. Two independent sessions judged it Dale's call. Do not implement it
   without his answer.
2. **Merging `1fc20dbb` to `main`.** The 759l7 fix is green on the shipping pin
   and pushed; `main` still carries the spin. Background sessions may not merge.
3. **The toolchain.** `rust-toolchain.toml` pins bare `nightly`, resolving to a
   rustc from 2025-12-10. Moving it changes the compiler for every session and
   worktree in this repo at once.

One more that needs his approval because it is an external write: failure 8 is a
genuine bounded fsqlite 0.1.19 regression worth reporting upstream —
`hydrate_contentless_index_from_segments`
(`fsqlite-ext-fts5-0.1.19/src/lib.rs:7480`) clears `shadow_rows` without
repopulating `documents`, so `row_count()` under-reports a reopened contentless
table until the next open. Transient, MATCH stays correct, no data loss.

## Environment facts that cost real time

1. **Two checkouts, never one target dir.** Shipping worktree builds into its own
   `target/` (21 GB, warm). The forward experiment is `/tmp/cass-759l7-forward`
   building into the **sibling** `/tmp/cass-759l7-forward-target` (12 GB, warm).
   `AGENTS.md`'s `### Unit Tests` now carries this; read it.
2. **The forward test binary is not under `debug/deps`.** It is at
   `/tmp/cass-759l7-forward-target/debug/build/coding-agent-search/b9364c709c6f41e6/out/coding_agent_search-b9364c709c6f41e6`.
   You can run it directly — both fts-repair-mode failures reproduce in under
   half a second with no build.
3. **Verify binary provenance by content, never mtime.**
   `strings <bin> | rg -o 'fsqlite-core-0\.1\.[0-9]+'`.
4. Disk is ~71 GiB free against a 150 GiB floor. A full `cargo test --lib` on the
   shipping tree costs ~3 min compile + ~2.5 min tests. Do not start speculative
   builds.
5. No `timeout`/`gtimeout`. Background + poll.
6. The worktree guard rejects `;`-chained and redirected bash commands. Put
   multi-step shell work in a script under `$CLAUDE_JOB_DIR/tmp` and `bash` it.
7. `br` does not work from inside the worktree — run it from
   `~/dev/coding_agent_session_search`.
8. `agent-dirtiness.py sync-gate` reports `base_not_ancestor_of_remote` on this
   branch; that is the tool assuming main-based work.
   `git merge-base --is-ancestor origin/main HEAD` succeeds.
9. **You are probably not alone on this branch.** Generations 12, 13 and 14 of
   this chain were all live simultaneously, in one shared worktree, and two of
   them were editing the same artifacts. Run `ListAgents` before you start; if a
   sibling is live, agree a file split by `SendMessage` before writing, and
   pathspec-bound every commit (`git commit -- <exact paths>`). This is not
   hypothetical: it happened twice this session and cost a duplicated fan-out.
10. Read the tracker before filing. This generation filed a bead that was already
    `xybl9`, and closed it as a duplicate. `br list --status all --limit 5000`.
