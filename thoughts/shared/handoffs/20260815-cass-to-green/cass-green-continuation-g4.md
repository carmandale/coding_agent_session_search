---
generation: 4
parent-session: af6e155f-ede0-4ed2-8c3a-deb7600fb980
next-action-class: executable
---

# Continuation — the pin bump is landed and measured; the regression suite never returned

## The goal and authorization, verbatim

Dale, 2026-08-14:

> /my-way fix cass to completion and 100% green working state and completely up to date or tell me why it can't or /grill-me with any questions.

Sent mid-work the same day, as a correction to the work in flight:

> make sure that you are looking at the recent (last 2 weeks) work on cass and not regressing

Dale, 2026-08-15:

> should we make a local fork of frankensqlite and fix it?

And the standing instruction, Dale, 2026-08-15:

> your usage is good now. finish this to completion

**Destructive and external-write approvals expired with the parent session and do
not transfer.** You do NOT have approval to: delete any file (this repo's
`AGENTS.md` RULE 1 forbids it outright, including files you created yourself),
force-push, rewrite history, change repo visibility, file anything on a public
third-party repo, or run `cass sources agents exclude`. The `/tmp` cargo reclaim
on bead `-jck92` is a deletion and needs Dale's explicit written permission; it
has not been given.

## THE HEADLINE — the fork question is answered, and the answer is a pin bump to 0.1.14

Dale asked whether to fork frankensqlite. No. Upstream already fixed it, and the
usable version is **0.1.14**, not the 0.1.17 the previous handoff proposed.

Landed as **`cd1089a8`** on `worktree-cass-gen5-honesty`, pushed.

- `ExistsValueSet` landed in **fsqlite-core 0.1.11**. Bisected by grepping each
  release's `connection.rs`: 0 occurrences at 0.1.5 / 0.1.8 / 0.1.10, 8 from
  0.1.11 onward. Both directions fire, so the zeros are real absences.
- **0.1.14 is the ceiling on this machine.** From fsqlite 0.1.15 up,
  `fsqlite-types` requires `asupersync >= 0.3.5`, which requires `sysinfo 0.39`,
  which requires **rustc 1.95**. Installed nightly is
  `rustc 1.94.0-nightly (f52090008 2025-12-10)` — eight months stale against a
  `rust-toolchain.toml` that pins only `channel = "nightly"`.
- The `=` version requirements are **load-bearing and were measured both ways**.
  A bare caret re-resolves to fsqlite 0.1.19 / asupersync 0.3.10 and hits the
  same rustc-1.95 wall; the committed lock does not save it.
- `Cargo.toml`, `build.rs` `CONTRACTS` and the README table move together because
  `build.rs` enforces exactly that. The README row was already stale before this
  session (`0.1.4` against a `0.1.5` pin).
- One line of `build.rs` logic changed: the manifest check now accepts `=X` as
  well as `X`, because `expected_version` is also compared against a sibling
  repo's bare `package.version` under `strict-path-dep-validation`.

**Measured, controlled A/B** — same fixture bytes (`sha 93d8e02f2046f2f9` on both
copies), identical argv, two binaries (`49fbba6e` @17:45:05Z vs `572ae86d`
@18:29:38Z), both controls firing:

| | fsqlite 0.1.5 | fsqlite 0.1.14 |
|---|---|---|
| connector-coverage read | `WARN ... not implemented: reloading populated WITHOUT ROWID table fts_messages_idx into MemDatabase is not yet supported` | no WARN |
| `Scan Coverage` | `UNKNOWN — the coverage read did not complete` | `complete` |
| conversations / messages | 2 / 6 | 2 / 6 |

That failure is exactly what the honesty family (`-nvq59`, `-a59ou`, `-ddkwa`,
`-xarzt`) was built to report truthfully. The honesty work made it legible; the
bump removes it.

## The exact next action

**Run the regression suite and read its verdict.** It compiled and was still
executing when the parent session hit its usage limit, so it never returned. The
change is therefore UNVERIFIED against the test suite — that is the one gap in an
otherwise measured change.

```bash
cd /Users/dalecarman/dev/coding_agent_session_search/.claude/worktrees/cass-gen5-honesty
export PATH="$HOME/.rustup/toolchains/nightly-aarch64-apple-darwin/bin:$PATH"
export CARGO_TARGET_DIR=/tmp/cass-repair-target   # WARM — do not make a new one
cargo test --lib
```

The harness is already built in that target dir, so it should go straight to
running rather than recompiling. Two tests in `src/dependency_drift.rs` were
updated to the new pins (`=0.1.14`, `=0.3.4`) — they read the checked-in
`Cargo.toml`, so they are the ones most likely to speak first if something is
wrong.

If it is green, promote `cd1089a8` from checkpoint to a completion claim (say so
plainly; do not amend the commit). If it is red, adjudicate each failure against
the pre-change SHA `0df963c7` before attributing it.

**Watch free disk the whole time.** It fell 31 → 24 GB building this, and the
codex catch-up's guard floor is 25 GB, so the machine is already under it.

## THE BLOCKER — disk, and it is Dale's call

`/System/Volumes/Data` was at **24 GB free** at handoff. Two corrections to the
numbers earlier handoffs carried:

- **The live archive is 17.1 GB, not 7.9 GB.** `agent_search.db` measured
  17,116,061,696 bytes at 2026-08-15 13:18. The whole cass data dir is **51 GB**,
  of which `raw-mirror` alone is **32 GB** and `index` is 2.3 GB.
- `/tmp` cass cargo target dirs now total ~104 GB. `/tmp/cass-repair-target`
  (28 GB) is IN USE and must be excluded from any reclaim.

This is what blocks the one measurement that would actually retire `-p3kgr`:

```bash
# against a THROWAWAY copy, never the live archive
CASS_SKIP_PREFLIGHT_CLEANUP_ORPHAN_FK_ROWS=1 cass index --full --force-rebuild --json --verbose --data-dir <throwaway>
```

Baseline to beat: that command sat in `phase=preparing` for a full 30-minute
bound and never advanced, *with* the skip var set. The success signal is
**advancing past `phase=preparing`**, not completing — so it can be bounded
tightly.

The cheapest faithful specimen is `~/backups/cass/agent_search-20260814-vacuum.db`
— 3.98 GB and **580,374 messages**, the exact count the wedge was measured at. But
4 GB does not fit in the headroom above the guard floor, and under RULE 1 the copy
could not be deleted afterwards. Unblock is bead
`coding_agent_session_search-reclaim-tmp-cargo-targets-jck92`, ~82 GB of stale
`/tmp` target dirs from worktrees with no live session. **Needs Dale's written
deletion approval.**

## A finding that changes the wedge theory

The statement the previous handoff named as the leading suspect,
`raise_lexical_rebuild_footprints_to_exact_message_counts`
(`src/storage/sqlite.rs:7486`), is:

```sql
SELECT conversation_id, COUNT(*) AS message_count
FROM messages GROUP BY conversation_id ORDER BY conversation_id ASC
```

**There is no `EXISTS` in it**, correlated or otherwise — so the `ExistsValueSet`
fix may not touch it, and the two wedges (incremental, already fixed by the skip
var; full rebuild, still wedged) plausibly have two different causes. On real
SQLite, read-only against the backup, that statement runs in **32 ms**:
`SCAN messages USING COVERING INDEX sqlite_autoindex_messages_1`, 12,722 groups
over 580,374 messages. So the data and schema are fine and the engine's plan is
the problem. Checked and dismissed: `sqlite_autoindex_messages_1` being the only
index is intentional — `src/storage/sqlite.rs:3559` deliberately drops
`idx_messages_conv_idx`, and the `INDEXED BY` at :7563 is guarded by a
`no such index` fallback at :7570.

## Open, with what is known

- **`-p3kgr`** — the pin bump is landed; the wedge itself is still unmeasured.
  Comment added recording all of the above.
- **`-b6xc3`** — three non-gating doctor surfaces still state a failed query as
  measured fact. Located and ready to wire, NOT done: the prose at
  `src/lib.rs:37351`, the evidence string at `:37360`, and
  `doctor_coverage_confidence_tier` at `:36450`/`:36864` returning
  `no_archive_rows` where it should say `unchecked`. The flag
  `archive_conversation_count_unknown` is already in scope at each. Separate
  commit from the pin bump.
- **`-xarzt`** — still a product call, still Dale's: should "could not check"
  degrade the one-word verdict? The bump lowers the stakes without answering it,
  since under 0.1.14 the coverage read succeeds on the specimen tested.
- **`-2bh4a`** — the codex catch-up. **Owned by a live peer session**
  (`coding_agent_session_search-cont-...-2bh4a-g1`, confirmed alive this session,
  a `cass` process 65+ minutes in). Do not compete with it.
- **`-qtn0e`** — the data-loss question is STILL UNANSWERED, and it is coupled to
  the next action: fixing the full-rebuild path re-arms a destructive path that
  has never been proven safe.
- **`-0gzok`** — already closed before the parent session started. The previous
  handoff's "nobody has closed the bead yet" was stale.

## Remainders this session could not discharge

- **The merge to `main`.** This background harness rejects edits to the shared
  checkout until the session isolates, then forbids pushing `main`. Work is safe
  on `origin/worktree-cass-gen5-honesty` through `cd1089a8`. A shared-checkout
  session merges it, then `git push origin main:master` per this repo's `AGENTS.md`.
- **`cargo test --lib`** — see the exact next action.

## Environment facts that cost real time

1. Build needs nightly on `PATH`; an absolute path to nightly cargo is not enough.
2. `--json` sets robot mode, which hard-codes the log filter to `error` and
   ignores `RUST_LOG` (`src/lib.rs:5769-5775`). Add `--verbose`.
3. Deploy by **atomic rename**, never `cp` over the live path. Nothing has ever
   been deleted. `~/.local/bin/cass` and `/tmp/cass-repair-target/release/cass`
   are separate inodes, so a release build does NOT re-deploy the installed binary.
4. No `timeout`/`gtimeout`. A bound helper is at
   `~/.claude-accounts/erika/jobs/af6e155f/tmp/bound.sh` (job-scoped; copy it
   rather than relying on it surviving).
5. **`br` from inside a worktree fails.** Pass
   `br --db /Users/dalecarman/dev/coding_agent_session_search/.beads/beads.db`,
   and the comment subcommand is `br comments add <id> "..."`, not `br comment`.
6. `cass` has a global `--db <path>` flag and `index` has `--data-dir` — either
   gives a throwaway target without touching the live archive.
7. **Check `cusage` before any fan-out.** The signed-in account was at 100% of its
   weekly window all session, so this generation ran solo with no lanes, per the
   95% rule.
8. `cargo update -p X --precise V` cannot beat a caret range written in
   `Cargo.toml`; and a bare `cargo update` after a pin edit drags the whole graph
   to latest and imports `kstring 2.0.4` needing rustc 1.96. Restore the committed
   lock and let `cargo check` make the minimal adjustment.

## Evidence

`thoughts/shared/handoffs/20260815-cass-to-green/agent-log-gen6-pin-bump.md`
(this chain's gen6 coordinator log, committed in `cd1089a8`, including its full
proof boundary), and `lanes/gen5-frankensqlite-fork-answer.md` for the fork
question's original evidence. Backup:
`~/backups/cass/agent_search-20260814-vacuum.db`, 3.98 GB, 580,374 messages,
verified.
