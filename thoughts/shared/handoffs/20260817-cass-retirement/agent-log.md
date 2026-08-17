# Coordinator log — cass retirement (2026-08-17)

Session: 656f2411-6418-4df9-9965-55219cd71762 (Claude Code, coordinator, Opus 5 1M).
Predecessor work: `thoughts/shared/handoffs/20260817-cass-viability-assessment/`
(five-lane assessment, commit `e28b3d8e`) → verdict: structurally non-viable.

## Goal and authorization, verbatim

Dale, 2026-08-17, after reading the assessment and the north-star answer:

> run step 1. retire cass completely. make sure that there are no remnants, no
> open beads, no references in .agent-config, no skills, no commands. nothing
> that would potentially ressurect it. run this to verified completion /my-way

Set as a session goal (Stop hook armed on the same condition). "Step 1" refers to
the north-star plan's step 1: *"You approve cass retirement + disk reclaim —
frees ~150–250G and ends the fix campaigns."* So disk reclaim is inside the
authorization, explicitly.

## Sharpened goal (coordinator's own words, per my-way)

Leave this machine and the mini in a state where a cold agent cannot discover,
invoke, rebuild, or be instructed to use cass — while losing none of Dale's
session history, because the standing objective he stated one turn earlier is
*"I want to capture all sessions and I want agents to be able to search them."*

Done means, all six:
1. No cass binary resolvable from a fresh login shell on either machine.
2. No cass skill, command, alias, watchdog, launchd job, or instruction on any
   runtime surface (Claude Code, Codex, Gemini, Cursor, Pi) on either machine.
3. Zero open or in_progress cass beads in any tracker on this machine.
4. `.agent-config` carries no instruction that would make an agent use cass, and
   carries a positive retirement record so a future agent does not re-create it
   (precedent: the dev-browser deletion recorded in AGENTS.md §10).
5. Every session capture that exists ONLY inside cass is extracted to plain
   readable files first. This gates every deletion.
6. The reclaimed disk is measured and reported.

Not done (false victories to refuse):
- Deleting cass data before the irreplaceable-capture extraction is verified.
- Leaving the skill hidden/quarantined instead of removed — measured precedent:
  quarantining hid nothing from Codex for 40 days (AGENTS.md §10, dev-browser).
- Closing beads without carrying forward what the replacement must inherit.
- Reporting "no references" from a `rg` sweep that was never shown to produce a
  positive control.

## Decisions made without asking (senior calls, per §3.6)

| decision | why |
|---|---|
| Preserve irreplaceable captures to a plain compressed archive before any delete | Dale's own larger objective is capturing all sessions; the mirror holds files the harnesses deleted. Destroying them would be the literal-request-over-intent failure. |
| ARCHIVE the GitHub repo, do not delete it | Reversible, keeps 4,261 commits + the assessment readable, and an archived private repo cannot be pushed to — which is the anti-resurrection property asked for. Deleting a GitHub repo is irreversible; that needs Dale. |
| Delete the local 75G checkout, last, after its own closeout | Biggest local resurrection vector (handoff chains, worktrees, 20 sessions' history point at it) and the largest single reclaim. Recoverable by un-archiving + clone, which is a deliberate act. |
| Record the retirement in `.agent-config` AGENTS.md §10 + a rule file | That is the surface every agent on both machines reads; repo precedent is the dev-browser removal recorded there, including "do not restore out of git history". |
| Killed orphaned PID 75534 | Stuck 5h40m cass probe, PPID 1 (owning session gone), holding the machine-global index lock. Cleared; verified gone. |

## Lane declaration — discovery workflow `wf_12486f6b-c24`

Claude Code workflow subagents, visible via `/workflows`, read-only (no delete,
no edit, no bead writes, no cass invocation), write permission ONLY to their own
log below. Coordinator owns all mutation and synthesis. Stop condition:
single structured report each.

| lane | purpose | log |
|---|---|---|
| agent-config | every cass reference in .agent-config + all runtime surfaces | lanes/agent-config.md |
| machine | binaries, data, residue, launchd, shell, other repos, git origin | lanes/machine.md |
| mini | the second machine, over `ssh mini-ts` | lanes/mini.md |
| beads | exact close set across all trackers + carry-forward findings | lanes/beads.md |
| irreplaceable-captures | **gates all deletion** — what exists only in cass | lanes/irreplaceable-captures.md |

## Scope grew during discovery — three repos and two machines, not one repo

The original framing ("no references in .agent-config, no skills, no commands")
was narrower than the real footprint. Findings that changed the plan:

**1. `gj-tool` is the live resurrection mechanism, and it is a third repo.**
`~/dev/gj-tool` (github.com/carmandale/gj-tool) implements `gj sessions`
(`bin/gj:10593-10618`), which does not merely *call* cass — on a missing venv it
runs `python3 -m venv` and `pip install -e "$CASS_TUI_DIR"`, i.e. it **builds
cass on demand**. `gj last-sessions` (`bin/gj:10623-10706`) shells out to
`cass timeline --since 7d --json` and `cass status --json`.
`bin/gj-config.env:188` points `CASS_TUI_DIR` into the checkout being deleted,
and `skill/SKILL.md:65` + `docs/AGENT-INSTRUCTIONS.md:52` advertise the
subcommand to every agent. `gj` is the mandated build tool for all Apple work
(AGENTS.md §2.1), so this is the highest-traffic surface of all. Must be fixed
in gj-tool, pushed, then pulled + installed on BOTH machines.

**2. cass is installed on the mini from a third-party Homebrew tap.**
`/opt/homebrew/bin/cass` → Cellar 0.6.23, from
`brew install dicklesworthstone/tap/cass` — a fork's tap, not Dale's repo. The
tap is itself a resurrection vector: while it stays installed, a future
`brew install cass` silently reinstalls cass from a repository Dale does not
own. Needs `brew uninstall cass` AND `brew untap dicklesworthstone/homebrew-tap`.

**3. The skill was already removed from agent-config and that removal did not
take.** `skills/tools/cass/SKILL.md` exists only in a stale Codex worktree
(`~/.codex/worktrees/9218/.agent-config/`), so it was deleted from the live repo
at some point — yet cass skills are still installed as REAL DIRECTORIES in four
runtime surfaces: `~/.claude/skills/cass`, `~/.codex/skills/cass`,
`~/.gemini/skills/cass`, `~/.cursor/skills/cass` (all dated May 9 03:58).
`install.sh`'s collision-skip refuses to touch real directories, so deleting the
source left four orphaned live copies that still teach agents to use cass. This
is the single-source rule failing exactly as `.claude/rules/single-source.md`
describes, and it is why the earlier soft controls looked applied but changed
nothing.

**4. Second machine has its own everything.** The mini carries an independent
554M clone of the cass repo with 19 open internal beads, its own separate
agent-config clone (so a laptop-side config fix needs an explicit mini-side
pull), a `~/.zshrc.local:30-32` CASS-mirror alias block regenerated from
`configs/shell/zshrc.local.mini:30-32`, `~/cass-mirror -> /Volumes/SSD-1/cass-mirror`
(empty), and an unverified ~6.4GB `agent_search.db` reported on
`/Volumes/SSD-2/SSD-1-mirror/cass-mirror/`.

**5. Outside agent-config: `~/Projects/sessions-wiki/CLAUDE.md:62`** tells agents
the nightly 3 AM compile on the mini "reads new sessions from cass", and names a
`/sessions-compile` trigger. The wiki is 224K, unscheduled in launchd on the
laptop, and 120 days stale — consistent with cass having broken its ingest.

**6. A scope line I am drawing deliberately.** `.agent-config` holds cass
references in ~140 files, but ~120 are dated spec artifacts, transcripts and
receipts from nine months of work. Those are the audit trail; scrubbing them
would vandalize history, break tests that read them, and violate this repo's own
rule against editing finalized spec bytes
(`.claude/rules/ground-truth-updates-plan.md`). **Live surfaces get removed;
historical records stay and are disambiguated by the retirement record instead.**
Same call on the mini's dev-wiki compiled pages describing cass as active.

## Progress

- [x] Peer sessions notified (groove-session-search-a5) not to resume cass work.
- [x] Orphaned stuck cass process 75534 killed and verified gone (had held the
      machine-global index lock for 5h40m; disk went 29Gi → 57Gi free).
- [x] Assessment evidence preserved out of the doomed checkout at
      `~/.agent-config/thoughts/shared/handoffs/20260817-cass-viability-assessment/`
      (agent-config commit `4c10286d1`), verified byte-identical with `cmp`, and
      the replacement session told to cite that path.
- [x] Archive layout for rescued captures decided and published to the peer:
      per-file zstd, original mtime preserved, `MANIFEST.jsonl` + `README.md`,
      under `~/session-archive/rescued-from-cass/`.
- [x] All five discovery lanes returned; logs under `lanes/` (2,377 lines total).
- [x] **The safety gate is GREEN.** 6,892 files / 1.76 GiB existed only inside
      cass's mirror; extracted to `~/session-archive/rescued-from-cass/` and
      verified **6892 ok / 0 mismatched / 0 missing**. 3,877 of them are Claude
      Code session files — 738 whole deleted sessions plus 3,139 subagent
      transcripts. Zero Codex captures were unique. This unblocks the 77G delete.
- [x] Four orphaned skill installs removed (Claude, Codex, Gemini, Cursor).
- [x] Nine laptop binaries removed; `cass` unresolvable on the laptop.
- [x] Mini: `brew uninstall cass` + `brew untap dicklesworthstone/tap`;
      unresolvable there too.
- [x] gj-tool fixed, pushed: `446240f` (312 lines out of `bin/gj`, completions,
      config, docs, three test files) and `b69560e` (the command-node count).
      First bats run 129/130 with the count as the only failure — the correct
      shape for an intentional removal — then fixed.
- [ ] **Handed to a continuation** — see `cass-retirement-continuation.md`.
      Remaining: the 77G data dir, ~190G build residue, 48 beads (44 main + 4
      hiding in worktree trackers + 19 on the mini), `.agent-config` live
      surfaces, the AGENTS.md §10 retirement record (drafted here), Dale's own
      `destructive_command_guard` AGENTS.md, the sessions-wiki cass call, mini
      leftovers, GitHub archive, and the local checkout last.

## Verdict on the goal, honestly

Dale's six-part definition of done, as it stands at handoff: **#1 met** (no cass
binary resolvable on either machine), **#2 partly met** (skills, binaries, tap and
the gj subcommands are gone; agent-config config lines, one AGENTS.md, mini
aliases and the wiki line remain), **#3 not started** (48 beads), **#4 drafted not
landed**, **#5 MET and verified** (the irreplaceable captures are rescued), **#6
partial** (28Gi freed so far by clearing the stuck process; ~267G still to
reclaim). The retirement is real but not finished, and the successor's first
action is the one that was previously blocked.
- [ ] Irreplaceable captures extracted and verified.
- [ ] Removal executed.
- [ ] Beads closed, retirement recorded in .agent-config.
- [ ] Independent cold verification pass.
