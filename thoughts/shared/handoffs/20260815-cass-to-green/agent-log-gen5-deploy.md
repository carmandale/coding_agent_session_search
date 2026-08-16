# agent-log — gen5 deploy-and-measure (session 09a898c8)

Coordinator log for the continuation launched from
`cass-green-continuation-g2.md` (generation 2, committed at `0ff74463`).

Lane declaration: **no child lanes launched.** The `cusage` reading at session
start showed the signed-in account (katherine) at **100.0% of its weekly window**
with 1h21m to reset. `skills/workflows/stay-within-limits` and AGENTS.md §3.9 set
the hard line at 95%, and the 2026-08-12 incident on record is twelve
inherited-Opus lanes launched into an exhausted bucket that wrote 1,084,496 cache
tokens and returned 29 output tokens. So this generation ran solo and consumed
the five committed gen5 survey lanes as evidence instead of re-running them.
Visibility class: artifact-visible (this log + the bead comments).

## The experiment the handoff set up, and its result

The handoff's exact next action was: build the branch, deploy by atomic rename,
re-measure the three commands, and treat `status --json` as the open question.
It named a falsifiable prediction — that `nvq59` (status hang) was caused by the
same unbounded `probe_state_db` that `nao4q` named, so bounding it would fix
both.

**Prediction held.** All three measured against the LIVE archive, each bounded at
90s by `jobs/036c5f98/tmp/bound.sh`, with the codex catch-up running throughout
(pid 21123, 7m43s elapsed at measurement) so the load matches the baseline
readings.

| command | deployed `82f316a7` | gen5 `463f2649` | verdict |
|---|---|---|---|
| `triage --json` | TIMEOUT 75.84s / 99.26s | **rc=0, 5.17s, 24,242 B** | returns |
| `status --json` | **TIMEOUT 99.26s** | **rc=0, 5.13s, 22,913 B** | returns |
| `health --json` | 3.05s rc=1 | 2.15s rc=1, 20,954 B | unchanged |

Binary identity is self-reported by `cass --version`: `git commit:
463f26490965dde9f55d61e1120470e34ab3ae0f`, against `82f316a75d43f...` before.
`git diff --name-only 0cf37f0c..463f2649 -- src/ Cargo.toml Cargo.lock` is
**empty** — the two intervening commits are `.beads/*` and the handoff artifact
only — so the deployed binary's source is byte-identical to the four-fix commit
`0cf37f0c`.

Napkin guard against a shared `CARGO_TARGET_DIR` satisfied: the build log line
614 reads `Compiling coding-agent-search v0.6.9
(/Users/dalecarman/dev/coding_agent_session_search/.claude/worktrees/cass-gen5-honesty)`
— this worktree, not a sibling checkout.

## The four fixes, observed live rather than inferred

`cass status --json` against the real 7.9 GB archive, fields quoted from
`jobs/09a898c8/tmp/status.json`:

- `database.open_error` = `"state database probe exceeded its 5000ms bound for
  ..."` — **`-nao4q`**'s bound firing and naming itself. `database.opened` =
  `false`.
- `conversation_count` = `null`, `message_count` = `null`, alongside
  `counts_skipped` = `true` — **`-0gzok`** part 2. The pre-fix binary rendered a
  fabricated `0` here with `counts_skipped: false` beside it.
- `connector_coverage` = `{"checked": false, "complete": null,
  "incomplete_connectors": [], "floors": []}` — **`-ddkwa`**. `complete` is
  `null`, not `true`, on a probe that never opened the database.
- `quarantine` unreadable-file fields absent — **`-a59ou`**'s
  `skip_serializing_if = "Vec::is_empty"` behaving as designed on an archive with
  no unreadable poison files.
- `status` = `"stalled"`, `healthy` = `false`. The archive is **not** reported
  healthy while the state probe failed. That is the whole point of the family.

## What this settles, and what it does not

Settles: `-nvq59` and `-nao4q` were one defect, not two. Closed on this evidence.

Does **not** settle `-0gzok` part 1 (`last_scan_ts`). On this run the probe timed
out, so `opened: false`, and the open-failure is already surfaced honestly via
`open_error` + `healthy: false`. The residual dishonest case — the database opens
fine but the `last_scan_ts` meta read itself errors or holds an unparseable value
— is not exercised by this measurement and is still live in source. Fixed
separately this session; see the commit that follows.

## Proof boundary

- The `health --json` row is unchanged because `state_meta_json_for_health`
  passes `skip_db_open: true` unconditionally and never calls the probe. Its
  2.15s vs 3.05s difference is run-to-run variation on a loaded machine, not an
  effect of this change, and should not be cited as one.
- `triage --json`'s envelope nests its database fields differently from
  `status`'s, so the field-by-field honesty inspection above is `status`'s
  output. Triage's measured result here is that it **returns**; its field-level
  honesty was not separately inspected.
- One run each, not a distribution. The pre-fix TIMEOUTs are from the handoff's
  recorded measurements, not re-run today — re-running them would mean
  redeploying the old binary over the working one.
