---
generation: 1
parent-session: 656f2411-6418-4df9-9965-55219cd71762
next-action-class: executable
---

# Continuation — cass retirement: the irreplaceable data is rescued and verified, the removal is half done

## The goal and authorization, verbatim

Dale, 2026-08-17, set as a session goal (a Stop hook is armed on this text):

> run step 1. retire cass completely. make sure that there are no remnants, no
> open beads, no references in .agent-config, no skills, no commands. nothing
> that would potentially ressurect it. run this to verified completion /my-way

"Step 1" is step 1 of the north-star plan Dale approved one turn earlier:
*"You approve cass retirement + disk reclaim — frees ~150–250G and ends the fix
campaigns."* **Disk reclaim is explicitly inside the authorization.**

The standing objective that governs every judgment call here, Dale, same day:

> I want to capture all sessions and I want agents to be able to search them.
> what is the stable, reliable, fast, best in class solution?

That is why session data is preserved rather than deleted along with the tool.

**Destructive and external-write approvals expired with the ending session and do
NOT transfer.** You do NOT have approval to: delete the GitHub repository (only
ARCHIVING it was decided, and even that is unexecuted — see Open), force-push,
rewrite history, change repo visibility, or delete anything on the mini beyond
the cass-specific paths named below. Deleting cass's own data/build residue on
the laptop IS authorized by the quote above.

## THE EXACT NEXT ACTION

**Delete the 77G production data directory. The safety gate that blocked it is
now green, measured and verified this session.**

```bash
du -sh "$HOME/Library/Application Support/com.coding-agent-search.coding-agent-search"   # expect ~77G
rm -rf "$HOME/Library/Application Support/com.coding-agent-search.coding-agent-search"
```

Before running it, re-confirm the rescue is still intact (10 seconds, and it is
the only thing standing between this command and permanent loss):

```bash
find "$HOME/session-archive/rescued-from-cass" -type f | wc -l    # expect 6894
du -sh "$HOME/session-archive/rescued-from-cass"                  # expect 1.8G
wc -l < "$HOME/session-archive/rescued-from-cass/index.tsv"       # expect 6893 (6892 + header)
```

If those three numbers do not match, STOP and do not delete anything — re-run
the verifier below instead. The rescue script is committed at
`thoughts/shared/handoffs/20260817-cass-retirement/rescue-unique-captures.py`
and `python3 <it> --dest "$HOME/session-archive/rescued-from-cass" --verify`
re-checks every file against the mirror's recorded blake3.

## What is already DONE (all verified, do not redo)

**The irreplaceable data is rescued — this was the gate on everything.**
6,892 files / 1,890,766,050 bytes (1.76 GiB) existed ONLY inside cass's raw
mirror: the harnesses had deleted the originals. Extracted to
`~/session-archive/rescued-from-cass/`, and the verify pass reported
**6892 ok / 0 mismatched / 0 missing**. Composition: `claude_code` 3,877 files
(1,787 MiB — 738 whole deleted Claude Code sessions plus 3,139 subagent
transcripts, not fragments), `openclaw-feature-dev-planner` 1,533,
`openclaw-feature-dev-developer` 1,482. Zero Codex captures were unique. The
uniqueness test ran twice, the second time against 2.9 million live files, with
a 12/12 positive control; the SQLite databases were separately shown to contain
nothing irreplaceable. Files are stored UNCOMPRESSED (1.8G) with `index.tsv`
carrying original path, provider, sizes, mtimes, mirror blake3 and an
extraction-time sha256.

**`cass` is unresolvable on both machines.**
- Laptop: 9 binaries in `~/.local/bin` (the live one plus 8 rollback copies,
  ~467MB) moved to Trash. `command -v cass` → empty.
- Mini: `brew uninstall cass` (Cellar 0.6.23) AND
  `brew untap dicklesworthstone/tap` — the tap mattered as much as the binary,
  because while it stayed installed a future `brew install cass` would silently
  reinstall cass from a fork Dale does not own. Verified: not resolvable, not in
  the formula list, tap gone.

**The four orphaned skill installs are gone.** `~/.claude/skills/cass`,
`~/.codex/skills/cass`, `~/.gemini/skills/cass`, `~/.cursor/skills/cass` were
REAL DIRECTORIES (not symlinks), byte-identical, dated May 9. The skill had
already been deleted from agent-config's `skills/tools/cass/` at some earlier
point, but `install.sh`'s collision-skip refuses to touch real directories, so
four live copies survived and kept teaching agents to use cass. All trashed;
a `find` across all five skill roots now returns nothing.

**gj-tool is fixed, committed and pushed — `446240f` in carmandale/gj-tool.**
This was the highest-traffic resurrection vector: `cmd_sessions` did not merely
call cass, it ran `python3 -m venv` + `pip install -e "$CASS_TUI_DIR"` on a
missing venv, i.e. **it rebuilt cass on demand**, and `gj` is the mandated build
tool for all Apple work. Removed 312 lines from `bin/gj` (both subcommands, both
`_cmd_meta_*` blocks, usage aliases, dispatch arms, help text, header comment),
plus completion entries in both shells and `CASS_TUI_DIR` from live config and
the example. Three test files updated because the commands genuinely no longer
exist — including `tests/gj-static.bats`, which used `/^cmd_sessions()/` as a
`sed` range END-anchor and would have silently run to EOF; it is now anchored on
`_resolve_usage()`. `bash -n` / `zsh -n` clean on all three shell files.

**Assessment evidence preserved out of the doomed checkout.** Copied to
`~/.agent-config/thoughts/shared/handoffs/20260817-cass-viability-assessment/`,
verified byte-identical with `cmp`, pushed as agent-config `4c10286d1`. The
replacement session cites that path.

**The stuck process is cleared.** PID 75534 (a cass probe, 5h40m, orphaned to
PPID 1) had held the machine-global `index-run.lock` and blocked all search.
Killed and verified gone; free disk went 29Gi → 57Gi.

## Open, with what is known

**1. The 77G data dir — the exact next action above.** Now unblocked.

**2. Build residue, ~190G reclaimable, no live owner.** Confirmed by the machine
lane with `lsof`: `.claude/worktrees` inside the cass repo is 69G (68G of it
three cargo target dirs); ~100G of `/tmp/cass-*` and `/tmp/fsq-*` (83 entries);
repo `target/` 2.7G; `~/backups/cass/agent_search-20260814-vacuum.db` 3.7G;
`~/.cass-catchup/`. **One hazard the machine lane flagged:** check
`/private/tmp/cass-759l7-forward` for unpushed commits before sweeping it — it
was reported as "8 dirty + 1 commit not on origin/main". Use `rm -rf` for these,
not `trash` — Trash is on the same volume, so trashing 190G reclaims nothing.

**3. 48 cass beads must close, not 44.** The main tracker
(`/Users/dalecarman/dev/coding_agent_session_search/.beads/issues.jsonl`, 1,927
records) holds 44 open/in_progress, but the beads lane found **4 more open beads,
two of them P0, living only inside worktree trackers** — those are invisible to
the main tracker and would survive its deletion. The mini's own clone holds a
further 19 open beads (cass's own unfinished roadmap: guided ops, swarm
coordination, trust scoring). Exact ids, statuses and titles are in
`lanes/beads.md`, which also separates out a **"carry forward to the
replacement"** list — measured facts about session-file formats and defects the
replacement must avoid. That list is a deliverable: send it to the
`groove-session-search-a5` session, which asked for it and will fold it into its
PROJECT_MEMORY and ADRs rather than let it die with the checkout.

**4. `.agent-config` live surfaces are NOT yet edited.** The agent-config lane's
findings are in `lanes/agent-config.md` (613 lines, read it). The scope line this
session drew deliberately, and which should hold: `.agent-config` mentions cass in
~140 files, but ~120 are dated spec artifacts, transcripts and receipts.
**Live surfaces get fixed; historical records stay untouched** — scrubbing them
would vandalize a nine-month audit trail, break tests that read them, and violate
`.claude/rules/ground-truth-updates-plan.md` on editing finalized spec bytes.
Known live items: `configs/skills/codex-visible.txt` (the cass exclusion line),
`configs/skills/claude-curation.txt`, `configs/shell/zshrc.local.mini:30-32` (the
CASS-mirror alias block), `configs/mini-host/bin/ssd-mirror.sh` + its README,
`scripts/disk-janitor.sh`, `scripts/lib/collision-check.sh`,
`skills/meta/compound-learnings/SKILL.md` (its "CASS is quarantined" line),
`skills/tools/gj-tool/SKILL.md`, `skills/tools/testflight/SKILL.md`,
`skills/_quarantine/tools/cm/SKILL.md.quarantined`, `napkin.md`,
`tests/test-agent-skill.sh`, `tests/test-disk-janitor.sh`,
`tools/codex-skill-review/index.html`,
`docs/goals/configurable-codex-skill-surface/state.yaml`.

**Removal-order hazard, measured precedent:** removing a skill while leaving its
name in `codex-visible.txt` is what broke `install.sh`'s Codex verifier during the
dev-browser removal. The exclusion line and the skill must go in the SAME change.
After editing, re-run `install.sh` (or its skill-symlink step) and then
`tests/test-quarantine-invisible.sh` and `scripts/skill-hygiene-check.sh`.

**5. The retirement record in `.agent-config` is drafted but NOT landed.** A full
draft is at
`thoughts/shared/handoffs/20260817-cass-retirement/agents-md-cass-retirement-draft.md`
in this repo. It belongs as a bullet in `instructions/AGENTS.md` §10 Tooling
Defaults, next to the ego-browser/dev-browser removal record at lines 1023-1025 —
that entry is the repo's proven precedent for retiring a tool so a tree-scanning
runtime cannot re-offer it, and its central lesson applies verbatim here:
**hiding a skill does not hide a binary, and soft controls had already failed for
cass too** (compound-learnings called it "quarantined", codex-visible.txt excluded
it, and four real skill dirs plus a PATH binary made both irrelevant). Check
whether any test pins §10 before editing; `tests/test-255-context-followup-prompt.sh`
pins §5 against a fixture, so that hazard shape is real in this repo.

**6. Three repos' AGENTS.md instruct agents to invoke cass, and ownership decides
what to do.** Verified origins: `~/dev/destructive_command_guard` is
**carmandale** — Dale's own, so `AGENTS.md:1195-1199` should be fixed.
`~/dev/ultimate_bug_scanner:739-743`, `~/dev/asupersync:852-856` and
`~/dev/agent_flywheel_clawdbot_skills_and_integrations` are all
**Dicklesworthstone** upstreams — do NOT edit third-party upstream docs; note them
instead. The flywheel repo ships cass as an installable skill, but its skills are
confirmed NOT installed into any runtime skill directory, so it is an inert clone
rather than a live vector.

**7. `~/Projects/sessions-wiki/CLAUDE.md:62`** tells agents the nightly 3 AM
compile on the mini "reads new sessions from cass" and names a
`/sessions-compile` trigger. Cut the cass call; **leave the wiki content alone** —
the replacement session explicitly declined to take on knowledge-base compilation
in v1 and has filed the pipeline as a named downstream, shipping a stable `--json`
contract so `specs/074-.../scripts/cass_client.py` can be re-pointed later. Record
that the wiki's 120-day-stale compile is a known-open item with a named future
owner, not a silent casualty.

**8. Mini-side leftovers.** Its own 554M clone at
`/Users/chipcarman/dev/coding_agent_session_search`; an empty
`~/Library/Application Support/com.coding-agent-search.coding-agent-search`;
`~/cass-mirror -> /Volumes/SSD-1/cass-mirror` (empty); `~/.zshrc.local:30-32`
regenerated from agent-config, so it needs an explicit **mini-side `git pull`**
after the agent-config fix lands — the mini has a SEPARATE agent-config clone.
**Unverified:** a reported ~6.4GB `agent_search.db` at
`/Volumes/SSD-2/SSD-1-mirror/cass-mirror/agent_search.db`, cited only via a bead
comment; stat it before acting.

**9. The GitHub repo and the local checkout — both still standing.** The decision
recorded this session: **ARCHIVE `carmandale/coding_agent_session_search`, do not
delete it** (reversible, keeps 4,261 commits and the assessment readable, and an
archived private repo cannot be pushed to, which is the anti-resurrection property
Dale asked for), then delete the local 75G checkout, which is the real local
vector because handoff chains and worktrees all point at it. Deleting the GitHub
repo outright is irreversible and remains Dale's call, not yours.

**Ordering:** the local checkout must go LAST. This session's dirtiness baseline,
lane logs and closeout machinery live inside it, so run the repo's closeout
(`sync-gate` / `reconcile` / `closeout-ledger` / `lease-release` / `close-check`)
and copy `thoughts/shared/handoffs/20260817-cass-retirement/` into
`~/.agent-config/thoughts/shared/handoffs/` FIRST, then delete the checkout from a
different working directory. If the main repo is deleted wholesale the linked
worktrees go with it; if anyone removes worktree directories independently while
keeping the repo, use `git worktree remove` or `rm -rf` + `git worktree prune` or
`.git/worktrees` is left with dangling entries.

**10. gj-tool's bats suite was mid-run at commit time.** 130 tests; one failure
was found and fixed in the working tree but is **NOT yet committed**: a hardcoded
count in `tests/gj-help.bats` asserting "exactly 29 command nodes (27 functional
+ version + help)", now 27/25 after removing two commands. Re-run
`bats tests/gj-help.bats tests/gj-completion-drift.bats tests/gj-static.bats`
(it exceeds 2 minutes — background it), then commit that one-line fix. Also
deploy: `gj` is installed at `~/bin/gj`, so the repo edit needs gj-tool's install
step run on the laptop, and a `git pull` + install on the mini.

## Environment facts that cost this session real time

1. **`gtimeout`/`timeout` do not exist on this machine.** Not installed, no
   coreutils keg. Bound long commands with `run_in_background` plus an `until`
   loop; foreground `sleep` is blocked by the harness.
2. **The Task tools are unavailable here** — `ToolSearch("select:TaskCreate,…")`
   returns no matches, so keep the plan visible in chat prose instead.
3. `rg -r` is `--replace`, not recursive, and it fabricates output at exit 0. It
   was tripped twice today (once by a lane, once by the coordinator). Never pass
   it.
4. Assertion-guarded edit scripts paid for themselves: the gj removal aborted
   cleanly on a one-line off-by-one (`_cmd_meta_last_sessions` was at 13517, not
   13518) before writing anything. Keep that pattern for the remaining
   multi-site edits.
5. This checkout is shared with concurrent sessions; bound every commit by
   pathspec (`git commit -F <msg> -- <exact paths>`) and read
   `git diff HEAD~1 HEAD --stat` before pushing.

## Coordination already in place

`groove-session-search-a5` is a live peer session building the replacement in
`/Users/dalecarman/dev/groove-session-search` (stock SQLite + FTS5, index as a
disposable cache, v1 scope = this Mac's Claude Code + Codex, primary consumer is
other agents). It has confirmed it holds nothing in the cass checkout and clears
the deletion. It has **claimed `~/session-archive/` as its archiver root** and
will read `rescued-from-cass/` in place, unmodified — do not move or restructure
that tree. It is owed two things: the carry-forward list from `lanes/beads.md`,
and confirmation of the rescued tree's real shape, which differs from what it was
promised: files are **uncompressed, not per-file zstd**, and the provider
directories are `claude_code`, `openclaw-feature-dev-developer`,
`openclaw-feature-dev-planner` rather than the `claude-code`/`codex` kebab
spelling it asked for. Tell it plainly so its ADRs match reality.

## Evidence

- `thoughts/shared/handoffs/20260817-cass-retirement/agent-log.md` — coordinator
  log: goal, six-part definition of done, decisions, lane declaration.
- `.../lanes/irreplaceable-captures.md` (659 lines) — the safety gate, its
  controls, and the extraction recipe.
- `.../lanes/machine.md` (470) — ~255G inventory with `lsof` liveness per path.
- `.../lanes/agent-config.md` (613) — the live-vs-historical reference split.
- `.../lanes/beads.md` (295) — the 48-bead close set and the carry-forward list.
- `.../lanes/mini.md` (340) — the second machine, the brew tap, gj-tool.
- `~/.agent-config/thoughts/shared/handoffs/20260817-cass-viability-assessment/`
  — why cass is being retired at all (agent-config `4c10286d1`).
