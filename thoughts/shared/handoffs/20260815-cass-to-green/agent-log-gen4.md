# Generation 4 coordinator log — deploy, re-measure, catch up

Session: `f377035a-57f3-4cc4-9641-ee37db57202b` (Claude Code, background job),
account `erika`, worktree `.claude/worktrees/cass-to-green-c6bfb589`,
branch `worktree-cass-to-green-c6bfb589`.

Resumed from `backfill-continuation-prompt.md` at commit `9a2207ba` via the
resume-handoff skill's autolaunched direct path. Frontmatter validated:
`generation: 3`, `parent-session: 29dd053b-e4a3-4e71-89d6-a599d8c5e157`,
`next-action-class: executable`. Working copy matched the committed bytes for
the prompt itself; only its `.launch-receipt.md` sibling was dirty.

## State found at resume (2026-08-15T12:06Z)

| thing | measured |
|---|---|
| backfill | ALIVE, pids 4751/4753/59304, **13 of 20** batches |
| conversations | 15,996 |
| release build | ALIVE, pid 52723, `cargo build --release -j 6`, started 12:03:52Z, cwd = this worktree |
| stale binary in target dir | `/tmp/cass-c6bfb589-target/release/cass`, mtime 11:57Z — PRE-DATES the running build, do not deploy it |
| live binary | `~/.local/bin/cass`, sha256 `5b3344fd94f93cd4ba0357a4c2d5b9de5733ead94ab404ff0963fdec29d01644`, mtime 2026-08-14 |
| disk | 89 GiB available, floor is 150 GiB |
| beads | `1a7mk`, `a4xe1`, `gxw32`, `tutfy`, `t61zi`, `jck92` all open |

The stale-binary line is the one that would have bitten: the target dir already
holds a `cass` from 11:57Z, six minutes older than the build that is currently
running. Deploying on file existence alone would have shipped the wrong bytes.
Deploy gates on pid 52723 exiting AND an mtime later than 12:03:52Z.

## Usage check before fan-out (§3.9)

`cusage`, 2026-08-15T12:08Z: account `erika` — session (5h) **86%**, resets in
3h31m; week (7d) 57%. Under the 95% do-not-launch line, but not roomy. Fan-out
held to four lanes with three pinned to `sonnet`; only the lane that has to
adjudicate a falsifiable prediction runs on the inherited model.

## Lane declaration

Runtime: Claude Code `Workflow` tool, inspectable via `/workflows`. All four
lanes are **read-only** — their only write is their own assigned log. None may
edit source, fixtures, the archive, the live binary, or any `settings.json`.
None may run `cass sources agents exclude`, delete anything, or start a second
backfill. Stop condition for each: its log is written and its questions answered
from evidence, or it records what it could not determine.

| lane | model | log path | purpose |
|---|---|---|---|
| `gen4-close-path-routing` | inherited | `lanes/gen4-close-path-routing.md` | Adjudicate generation 3's prediction from source: which of `status`/`stats`/`health`/`triage` reach the archive through which close path, and whether `8dcd245b` bounds only `health` |
| `gen4-golden-plan` | sonnet | `lanes/gen4-golden-plan.md` | Exact mechanics of the `a4xe1` repair: where `.actual` lands, per-file wholesale-vs-hunk, what must not be touched |
| `gen4-catchup-audit` | sonnet | `lanes/gen4-catchup-audit.md` | Audit `catchup-manifest.py` + `catchup-run.sh` against the handoff's claims before committing hours to them |
| `gen4-deploy-precheck` | sonnet | `lanes/gen4-deploy-precheck.md` | Who else invokes `~/.local/bin/cass` (launchd, watchers, other sessions) and would be disturbed by an atomic rename |

Coordinator owns synthesis, the deploy, the re-measure, the fixture edit, the
catch-up run, and the closeout. No lane discharges any of those.

## Timeline

- 12:06Z — resumed, state measured, seven tasks created in the live task list.
- 12:08Z — usage checked, lanes declared.
