# cass retirement — mini sweep

Lane: mini sweep (read-only discovery). Machine: Mac mini (`chips-mac-mini` /
`Chips-Mac-mini.local`), user `chipcarman`, reached via `ssh mini-ts`.
Date: 2026-08-17.

Reachability confirmed: `ssh mini-ts 'echo ok; hostname; df -h / | tail -1'`
returned `ok`, `Chips-Mac-mini.local`, `460Gi 12Gi 276Gi 5% /`. All findings
below are read-only observations; nothing was deleted, moved, edited, or
executed against cass on the mini.

## Top finding — cass is a live, functional dependency of `gj-tool`, not just a standalone binary

This is the most important thing this lane found and it was **not** in the
known inventory. The `gj` tool — the one AGENTS.md §2.1 mandates for every
Apple/GJ build/test/run — has a real, wired-up cass integration in its own
source repo, `~/dev/gj-tool` (`git remote`:
`https://github.com/carmandale/gj-tool.git`, mini HEAD `ee9c0f9`).

**Source file:** `/Users/chipcarman/dev/gj-tool/bin/gj` (this is the actual
authored source; the ~496KB, 12,475-line deployed copy at `~/bin/gj` on the
mini is a built/bundled artifact of it — line numbers differ between the two,
noted below for both).

Two subcommands depend on cass:

- **`gj sessions [path]`** (source `bin/gj:9752-9777`; deployed
  `~/bin/gj:10094-10122`) — launches "CASS TUI" via
  `python -m cass_tui "$workspace"`, gated on `CASS_TUI_DIR` pointing at
  `$DEV_ROOT/coding_agent_session_search/tui` and that directory having a
  `pyproject.toml`.
- **`gj last-sessions [-n N] [--all] [--agent <name>]`** (source
  `bin/gj:9781-9865`; deployed `~/bin/gj:10124-10211`) — checks
  `command -v cass`, then runs `cass timeline --since 7d --json --group-by none
  [--agent X]` and `cass status --json` and parses the JSON.

**`gj last-sessions` is live and functional on the mini right now**, because
cass is on PATH there (Homebrew install, see below) — `command -v cass`
succeeds and the eval'd `cass timeline`/`cass status` calls will actually
execute against whatever cass finds. Retiring only the binary/data and leaving
this code in place means the very next `gj last-sessions` invocation either
breaks with a raw `cass not found` shell error surfaced through `gj`, or (if
cass is left installed anywhere on PATH) silently keeps working and
resurrecting cass usage through Dale's most-used tool.

`gj sessions` is currently inert on the mini specifically, but not because the
cass dependency was removed — because the mini's checked-in config is
internally inconsistent (see below), not because anyone intended to disable
it.

**Config, both tracked in git** (`git ls-files` confirms both are tracked, not
gitignored):
- `~/dev/gj-tool/bin/gj-config.env:7` — `DEV_ROOT="/Users/dalecarman/dev"`
  (the laptop's home directory, hardcoded into a tracked file, so this is
  presumably correct/live on Dale's laptop and simply wrong on the mini,
  where the user is `chipcarman`). Line `161`:
  `CASS_TUI_DIR="$DEV_ROOT/coding_agent_session_search/tui"   # Required for 'gj sessions'`
  — **uncommented, active**.
- `~/dev/gj-tool/bin/gj-config.env.example:179` — same line, commented out,
  as the shipped template default.
- The mini's *deployed* `~/bin/gj-config.env` (separately, not the repo
  checkout) has `DEV_ROOT="/Users/YOUR_USERNAME/path/to/dev"` — literally the
  unedited template placeholder — and `CASS_TUI_DIR` commented out at line
  125. So on the mini, `gj sessions` fails today on the placeholder path, not
  because cass was ever removed from consideration.
- The mini's cass repo clone has no `tui/` subdirectory at all
  (`~/dev/coding_agent_session_search/tui` does not exist), so even a
  correctly configured `CASS_TUI_DIR` would fail the `pyproject.toml` check
  on this machine. Whether a `tui/` still exists in the cass repo's git
  history, or ever shipped, is unverified by this lane — worth checking `git
  log -- tui/` in the cass repo, or ask this repo's own lane.

**Docs also reference `gj sessions` as a CASS TUI feature:**
- `~/dev/gj-tool/skill/SKILL.md:61` — `gj sessions [path]        # CASS TUI for agent history`
- `~/dev/gj-tool/docs/AGENT-INSTRUCTIONS.md:52` — `| `gj sessions [path]` | CASS TUI for agent conversation history |`

**gj-tool's own bead tracker** (`~/dev/gj-tool/.beads/issues.jsonl`) has no
open beads about cass — only one closed, unrelated bead
(`gj-tool-7gj`, log-viewer search feature) turned up on a `cass` grep and its
title has nothing to do with cass; not a resurrection vector.

**Coordinator action needed, outside this repo:** the actual fix for this
finding lives in the `gj-tool` repo, not in `coding_agent_session_search` or
`agent-config`. It needs: removing/replacing the `cmd_sessions` and
`cmd_last_sessions` cass dependency in `bin/gj`, dropping the `CASS_TUI_DIR`
lines from both `gj-config.env` and `gj-config.env.example`, updating
`skill/SKILL.md` and `docs/AGENT-INSTRUCTIONS.md`, committing, pushing, and
then **both the laptop's and the mini's separate `gj-tool` checkouts need to
pull it and re-run `install.sh`** so the deployed `~/bin/gj` on each machine
is rebuilt from the fixed source — editing the source repo alone does not
touch the already-deployed 12,475-line bundle at `~/bin/gj`.

## cass itself on the mini

**Not what the known inventory expected.** The mini does not have the
laptop's `~/.local/bin/cass` + 8 rollback copies pattern at all — confirmed
absent (`ls ~/.local/bin | grep -i cass` → no matches). Instead:

- **`/opt/homebrew/bin/cass`** → symlink to
  `/opt/homebrew/Cellar/cass/0.6.23/bin/cass` (`cass 0.6.23`, Mach-O
  arm64), installed via a **third-party Homebrew tap**:
  `/opt/homebrew/Library/Taps/dicklesworthstone/homebrew-tap/` (formula:
  `dicklesworthstone/tap/cass`, pointed at
  `github.com/Dicklesworthstone/coding_agent_session_search` releases —
  note this is a *different* GitHub account/fork than Dale's own
  `carmandale/coding_agent_session_search`). `brew info cass` currently
  refuses with "untrusted tap" (never trusted/run), so this install is a
  standing artifact, not something actively re-verified.
  - Cellar size: 39M.
  - Retirement requires `brew uninstall cass` **and** `brew untap
    dicklesworthstone/homebrew-tap` (or at least removing the tap) — leaving
    the tap in place means a future `brew install cass` on this machine
    silently resurrects it from a third-party fork, not even Dale's own repo.
  - Homebrew-managed completions ride along and are removed automatically
    with the uninstall: `/opt/homebrew/etc/bash_completion.d/cass`,
    `/opt/homebrew/share/zsh/site-functions/_cass`,
    `/opt/homebrew/share/fish/vendor_completions.d/cass.fish`,
    `/opt/homebrew/var/homebrew/linked/cass`, `/opt/homebrew/opt/cass`.
- **`~/Library/Application Support/com.coding-agent-search.coding-agent-search`**
  exists but is **0 bytes** (empty directory) — no real production data on
  the mini under this identifier.
- **`~/.local/share/cass`** — does not exist on the mini.
- **`~/.cass-catchup`** — does not exist on the mini.
- **`/tmp/cass-*`, `/tmp/fsq-*`** — no matches on the mini (zsh confirmed "no
  matches found" for both globs; the laptop's ~100G residue does not exist
  here).

## Full source repo clone on the mini

**`~/dev/coding_agent_session_search`** — a complete, independent git clone,
554M, origin `git@github.com:carmandale/coding_agent_session_search.git`
(Dale's real repo — separate from the Homebrew tap's fork target above), HEAD
`dea44291 fix(fuzz): remove || true that masked crash exit codes`, working
tree clean (`git status --short` empty).

- No `.claude/worktrees` directory (the laptop's 69G of worktrees does not
  exist here).
- No `target/` build directory present (no stray Cargo build residue found).
- **Its own bead tracker has 19 open beads**, all epics/sub-tasks under two
  feature epics (`coding_agent_session_search-guided-ops-repro-trust-5u82n`
  and its 12 children; `coding_agent_session_search-oh96l`; and
  `coding_agent_session_search-swarm-coordination-intelligence-gnrxb` and 3
  children) plus 1,737 closed and 1 tombstone. These are cass's *own*
  internal feature-planning beads (guided ops, swarm coordination, trust
  scoring) — not beads *about* retiring cass. Whether "no open beads" is
  meant to reach into the retired repo's own internal tracker, or only beads
  about cass elsewhere, is a judgment call for the coordinator; flagging the
  count (19) so it isn't missed either way. This clone should be presumed at
  or near the same commit as the laptop's canonical checkout given a clean
  working tree, but that was not diffed against the laptop HEAD by this lane.

## `~/.agent-config` on the mini is a SEPARATE git clone, not a mount or symlink

`readlink -f ~/.agent-config` resolves to itself (real directory, not a
symlink); it is its own checkout, origin
`git@github.com:carmandale/agent-config.git`, HEAD on the mini:
`8c1912d4 triage(machine): Dale was right — six of my fseventsd claims were
wrong`. **This means any cass-reference cleanup made to the agent-config repo
on the laptop does not reach the mini until someone `git pull`s inside
`~/dev` — wait, `~/.agent-config` — on the mini.** This is a structural
resurrection vector by omission: a coordinator who edits and pushes from the
laptop and calls agent-config "clean" without also updating the mini's
separate checkout leaves stale cass-referencing content live there,
including in files that are actively read at session start on that machine
(shell config, skills, etc.)

A broad `rg -i cass` across the mini's `~/.agent-config` returned ~230
matching files. The overwhelming majority are **historical, already-committed
`specs/*` and `thoughts/shared/handoffs/*` records** — dated investigation
logs, past spec work, review transcripts — that mention cass as a subject of
past work (e.g. `specs/074-claude-sessions-wiki/`, the
`20260814-cass-status-check/` handoff, `20260810-cass-deploy-coverage-fix.md`,
`20260810-br-cutover-verification/lanes/challenge-cass.md`, disk-janitor specs
that used cass test runs as an example workload). These are the audit trail
of work already done; editing them would rewrite history rather than remove
an active resurrection vector, per this repo's own `ground-truth-updates-plan.md`
rule. I did not enumerate all ~230 individually since almost all are inert
history — the ones below are the ones with actual current-behavior
implications:

- **`configs/shell/zshrc.local.mini:30-32`** (the already-known lead,
  confirmed at its real path — this file is the tracked *source* that
  produces the mini's deployed `~/.zshrc.local`):
  ```
  30: # CASS MacBook mirror — synced nightly from MacBook Pro
  31: export CASS_MACBOOK_DB="$HOME/cass-mirror/agent_search.db"
  32: alias cass-macbook='cass --db "$CASS_MACBOOK_DB"'
  ```
  Deployed copy confirmed live at `~/.zshrc.local:30-32` on the mini
  (identical, so a real file — not a symlink — 1,753 bytes, last written
  2026-08-17 10:03, i.e. it is actively regenerated/deployed on this
  machine).
  - **`~/cass-mirror`** is a symlink → `/Volumes/SSD-1/cass-mirror`, which is
    mounted and **exists but is empty (0 bytes, empty dir)**. So
    `CASS_MACBOOK_DB` currently points at a file that doesn't exist — the
    alias is dead in practice today, but it is live config that would
    resolve correctly the moment anything populated
    `/Volumes/SSD-1/cass-mirror/agent_search.db` again.
  - **No launchd job or cron entry performs this "nightly sync"** on the
    mini — checked `~/Library/LaunchAgents/*.plist`,
    `/Library/Launch{Agents,Daemons}/*.plist` (none matched `cass`), and
    `crontab -l` (only an unrelated `openclaw` snapshot job). So "synced
    nightly from MacBook Pro" is either stale documentation, a mechanism that
    lives on the laptop side (pushing to the mini) rather than the mini
    pulling, or a mechanism that was never actually wired up on this
    machine. This lane could not find the pull/push mechanism on the mini
    itself, so a companion laptop-side lane should check whether the laptop
    pushes to `/Volumes/SSD-1/cass-mirror` on the mini.
  - **A related dead reference in an OPEN bead**: `.agent-config-14bq`
    (open, priority 1, mini SSD backup repair) has a 2026-08-05 comment
    naming `SSD-2/SSD-1-mirror/cass-mirror/agent_search.db (6.4 GB)` as a
    stale file with no counterpart on SSD-1 that a planned mirror-mode flip
    would delete — i.e. there may be a 6.4GB cass database sitting in the
    **backup** mirror (`/Volumes/SSD-2/SSD-1-mirror/cass-mirror/`) even
    though the live `/Volumes/SSD-1/cass-mirror/` is empty. Not directly
    checked by this lane (it's inside a backup/rsync mirror path, arguably
    out of a "read-only discovery of live resurrection vectors" scope, but
    flagging since it's 6.4GB of cass data sitting on this machine that the
    known inventory never mentioned).
  - `~/.agent-config/configs/mini-host/bin/ssd-mirror.sh:5` mentions
    `cass-mirror` only in a descriptive comment listing what SSD-1 holds —
    not a functional dependency on cass itself.

- **`~/.agent-config/.beads/issues.jsonl`** (the mini's own agent-config
  bead tracker, separate JSONL from the laptop's until pulled) — no beads
  are *about* retiring cass, but three open/relevant beads mention cass as
  context:
  - `.agent-config-bn34` (**open, in_progress**, priority 1) — a real,
    independent disk-janitor bug (the "unknown-hog tripwire" undercounts
    many sub-threshold directories). Its description cites "a parallel cass
    test run leaked 16 identical fixture dirs" as the workload that exposed
    the bug. The bug itself is not about cass and should stay open
    regardless of cass's retirement — flagging only so nobody closes it
    thinking it's cass-specific, and so nobody is surprised the word "cass"
    appears in an unrelated open bead.
  - `.agent-config-beads-agreement-check-8fyh` (**open**, priority 1) — a
    feature request for a weekly bead-tracker-agreement checker, whose
    description uses the cass repo's real 2026-08-10 five-month tracker
    divergence incident as its motivating case study and cites
    `coding_agent_session_search-q4vgj` (closed, in the cass repo itself) by
    id. This is agent-config's own tooling work; the cass mention is
    historical precedent, not a live dependency.
  - `.agent-config-238` and `.agent-config-34m` (**both closed**) reference
    "CASS session history" / cass as one of several skills once quarantined
    for dedup with `jsm` — pure history, already closed, no action needed.

- **`~/.codex`** on the mini — no cass-specific skill directory exists
  (`fd cass ~/.codex/skills` empty). The `rg -i cass` hits inside `~/.codex`
  are almost entirely session rollout JSONL transcripts (agent conversation
  logs that happened to discuss cass) and node_modules/vendor files with
  unrelated incidental matches (e.g. TypeScript's French diagnostic
  messages, a vitest bundle) — none are configuration that would resurrect
  cass. Not enumerated further; none are actionable.

- **`~/.claude/skills`** on the mini — **no `cass` skill directory exists
  at all** (`ls ~/.claude/skills/cass` → No such file or directory,
  confirmed twice). This differs from the known inventory's laptop finding
  of `~/.claude/skills/cass/SKILL.md` — the mini apparently never had that
  skill symlinked/installed, or it was already removed. Confirmed absent
  in `~/.gemini/skills` too.

## Outside `.agent-config` and the cass repo: a knowledge-base (wiki) trail

`~/GBrainSources/dev-wiki/` is a separate personal knowledge-base repo/tool
on the mini (not part of agent-config or the cass repo) that has compiled
wiki pages describing cass as an active project:

- `~/GBrainSources/dev-wiki/raw/repos/coding_agent_session_search/repo-summary.md`
  (40K, no `.git`, just a raw source snapshot ingested by the wiki tool).
- `~/GBrainSources/dev-wiki/wiki/index.md:114` and `:291` — entity/source
  index entries describing `[[coding-agent-session-search]]` as `_active_`,
  "Rust CLI (`cass`) that unifies coding-agent conversation history…".
- `~/GBrainSources/dev-wiki/wiki/log.md` — several dated compile-log entries
  narrating cass's wiki history (spec counts, HEAD corrections), all
  descriptive/historical.
- `~/GBrainSources/dev-wiki/wiki/concepts/beads.md:239-240` — a short
  "notably, this is the CASS project itself" note inside a concepts page.

These are read as legitimate wiki content, not accidentally-matched
substrings (spot-checked several — "cass" only ever appears as the tool
name or inside `coding-agent-session-search`, never as a false hit off an
unrelated word). This wiki is a personal, mostly-read-only knowledge base
that describes many repos including several already-archived/retired ones
("cass-custom-disabled" is separately tracked there as `_archived_`) — it is
not itself a mechanism that would relaunch or reinstall cass, but it will
keep describing cass as `_active_` until its own compile/lint cycle re-syncs
against the repo's real (retired) state. Whether the wiki maintainer process
should be told about the retirement is a call for the coordinator; this lane
treats it as informational, not a resurrection vector requiring deletion, and
did not touch it.

## Confirmed absent on the mini (matching or extending the known inventory)

- `~/.local/bin/cass` and rollback copies — absent (mini uses Homebrew
  instead, see above).
- `~/.local/share/cass`, `~/.cass-catchup` — absent.
- `/tmp/cass-*`, `/tmp/fsq-*` — absent.
- `~/.claude/skills/cass/`, `~/.codex/skills/cass*`, `~/.gemini/skills/cass*`
  — absent.
- No `cass`-named launchd plist, and `launchctl list | grep -i cass` — no
  match.
- No cass entry in `crontab -l` (only one unrelated openclaw job).
- No other clone of `coding_agent_session_search` under `~` besides
  `~/dev/coding_agent_session_search` and the wiki's raw-source snapshot
  (checked via `fd -H -t d coding_agent_session_search ~ --max-depth 6`).
- `~/bin`, `~/openclaw/scripts`, `~/chip-voice` — only `~/bin/gj` and
  `~/bin/ssd-mirror.sh` reference cass (both covered above); no other
  scripts on the mini invoke `cass`.

## Summary of concrete resurrection vectors found on the mini (action items for the coordinator)

1. **`gj-tool` repo source** (`~/dev/gj-tool` — separately cloned on the
   laptop too, not checked by this lane): `bin/gj` (`cmd_sessions`,
   `cmd_last_sessions`), `bin/gj-config.env` + `.example`, `skill/SKILL.md`,
   `docs/AGENT-INSTRUCTIONS.md` all need the cass dependency removed/replaced,
   then pushed, then pulled and reinstalled (`install.sh`) on **both** the
   laptop and the mini so the deployed `~/bin/gj` on each machine is rebuilt.
   This is the single biggest live resurrection vector found by this lane.
2. **Homebrew**: `brew uninstall cass` and remove/untap
   `dicklesworthstone/homebrew-tap` on the mini (39M Cellar; completions
   ride along automatically).
3. **`configs/shell/zshrc.local.mini:30-32`** in agent-config (source of the
   deployed `~/.zshrc.local` on the mini) — the `cass-macbook` alias and
   `CASS_MACBOOK_DB` export need removing at the source, then the mini needs
   its `.agent-config` pulled and its shell config redeployed.
4. **`~/.agent-config` on the mini is a separate clone** — any laptop-side
   agent-config cleanup (skill file, commands, config exclusions) needs an
   explicit `git pull` on the mini to actually take effect there; it will
   not happen automatically.
5. **Possible 6.4GB of cass data inside the SSD backup mirror**
   (`/Volumes/SSD-2/SSD-1-mirror/cass-mirror/agent_search.db`, per
   `.agent-config-14bq`'s 2026-08-05 comment) — not directly verified by
   this lane; worth a follow-up check before calling mini-side data fully
   clear.
6. **The 19 open beads inside the cass repo's own tracker** — a judgment
   call on whether "no open beads" reaches into the retired repo's internal
   planning beads; flagging rather than deciding.

Everything above was observed read-only; no cass binary was executed, no
files were deleted or edited on the mini.
