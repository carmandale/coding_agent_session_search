# Lane: machine sweep — cass retirement discovery

Read-only. No deletions, moves, renames, uninstalls, or cass invocations were performed.
Scope: this laptop, outside `.agent-config` (per task assignment — `.agent-config` items
are confirmed-present where I happened to touch them, but are another lane's territory).

Grand total measured cass-attributable disk footprint on this laptop: **~255 GB**
(77G data + 102G /tmp residue + 69G repo worktrees + 3.7G backup + 2.7G target/
+ 467M binaries + 295M cass-custom-disabled + 37M catchup + negligible rest).

---

## 1. Binaries

`~/.local/bin/` (this is the only PATH location with any `cass` entry — nothing in
`/usr/local/bin`, `/opt/homebrew/bin`, `~/bin`, `~/.cargo/bin`):

```
-rwxr-xr-x@ 1 dalecarman staff 51934144 Aug 16 12:29 cass                                          <- active, `which cass` resolves here, v0.6.9
-rwxr-xr-x@ 1 dalecarman staff 51900976 Aug 10 20:37 cass.coverage-floor-fix-20260810
-rwxr-xr-x@ 1 dalecarman staff 51900992 Aug 14 16:56 cass.nvq59-status-gate-20260814-165549
-rwxr-xr-x@ 1 dalecarman staff 51900992 Aug 14 16:56 cass.pre-1a7mk-fix-20260815
-rwxr-xr-x@ 1 dalecarman staff 51917584 Aug 15 12:45 cass.pre-8llb5-deploy-20260816-155012
-rwxr-xr-x@ 1 dalecarman staff 51834784 Jun  1 06:21 cass.pre-coverage-floor-20260601
-rwxr-xr-x@ 1 dalecarman staff 51917584 Aug 16 10:50 cass.pre-gen10-deploy-20260816-164709
-rwxr-xr-x@ 1 dalecarman staff 51934144 Aug 16 12:29 cass.pre-gen11-deploy-20260816-122921
-rwxr-xr-x@ 1 dalecarman staff 51901024 Aug 15 11:34 cass.pre-gen5-20260815-174600
```
9 files total, ~467 MB. `command -v cass` and `which cass` both resolve to
`/Users/dalecarman/.local/bin/cass`. No process currently running it
(`ps aux`/`pgrep -fl cass` show nothing but this discovery session's own greps and
sibling investigation sessions — see §9 below).

## 2. Data directories

| Path | Size | Notes |
|---|---|---|
| `~/Library/Application Support/com.coding-agent-search.coding-agent-search/` | **77G** | production data dir. Children: `agent_search.db` 22G, `index/` 9.5G, `raw-mirror/` 46G, plus tiny lock/wal/shm files. `watcher-heartbeat` file **absent** — watcher is not currently running. |
| `~/.local/share/cass/` | 8K | `agent_search.db` (0 bytes, dead stub), `doctor/`, `watchdog.sh` (4029B) + `watchdog.sh.bak` (2592B) |
| `~/.cass-catchup/` | 37M | 66 entries — batch files, run logs, generation evidence dirs (`gen8-*`, `gen9-*`, `gen10-*`, `gen11-*`). This looks like scratch output from the recent multi-generation debugging effort (spec p3kgr / gen8-11), not a cass runtime artifact per se. |
| `~/backups/cass/agent_search-20260814-vacuum.db` | 3.7G (3,984,084,992 bytes exactly) | single file, one backup |
| **NEW, not in original inventory:** `~/Library/Application Support/coding-agent-search/` (note: no `com.` prefix — a *second*, differently-named Application Support dir) | 284K | `macros/` subdir, 55 `cass-macro-YYYYMMDD-HHMMSS.jsonl` files. These are cass's own TUI macro-recording feature — trivial keystroke recordings (e.g. one file is literally the two keypresses "h","i"). Spans Mar 27 2026 → Aug 16 19:27 2026. Harmless, tiny, but a distinct directory from the main data dir that a coordinator sweep keyed only on `com.coding-agent-search.coding-agent-search` would miss. |
| `~/Library/Caches/*cass*` | — | none found |
| `~/Library/Logs/*cass*` | — | none found (note: `~/Library/Logs/cass-index-watch.log`, the log the watchdog script writes to, does **not** exist — consistent with the watcher never having run under launchd) |
| `~/Library/Preferences/*coding-agent*` / `*cass*` | — | none found |
| `~/Library/Application Support/CrashReporter/*coding_agent*` | — | none found |
| `~/Library/Logs/DiagnosticReports/*cass*` or `*coding_agent*` | — | none found |
| `~/Library/Saved Application State/*coding-agent*` | — | none found |
| **NEW:** `~/Library/Application Support/CrashReporter/` (plain, not the folder above) | 2 files, 240 bytes each | `cass_CDDEE1D1-...plist` (Aug 15) and `coding_agent_search-983a915ea0c0a592_CDDEE1D1-...plist` (Aug 16) — these are **DiagnosticReports lock/marker plists** for the machine's own hardware UUID (`CDDEE1D1-...` matches this Mac's `machine_id` seen elsewhere in ps output), not full crash reports. Trivial but genuinely cass-attributable and not in the original inventory list (the inventory's own `CrashReporter` line and `DiagnosticReports` line both came back empty because I checked those exact subpaths; these two plists sit directly in `~/Library/Application Support/CrashReporter/`, one directory up from where the inventory's glob was aimed). |

## 3. Build residue

### /tmp/cass-* and /tmp/fsq-*
83 entries confirmed (matches inventory estimate), **102G total** (`du -ch` sum).
Largest:

```
 29G  fsq-probe-data
 26G  cass-gen8-target
 12G  cass-759l7-forward-target
9.9G  cass-8llb5-verify-zRD4sl
7.7G  cass-fix-target
4.9G  cass-ibuuh-probe
3.0G  cass-gen8
3.0G  cass-759l7-forward
3.0G  cass-0119-test
2.7G  cass-0119-target
2.3G  cass-fix-data
1.0G  fsq-probe-034-target
984M  fsq-probe-target
952M  fsq-probe-015-target
```
(full breakdown of all 83 entries, all confirmed <200M below the cut, in scratch —
ask if the full table is needed)

**lsof check: zero handles found on any of these directories.** Spot-checked the
6 largest (`fsq-probe-data`, `cass-gen8-target`, `cass-759l7-forward-target`,
`cass-8llb5-verify-zRD4sl`, `cass-fix-target`, `cass-ibuuh-probe`) — `lsof +D <dir>`
returned empty for all six. **No live process is running.** `ps aux` / `pgrep -fl cass`
confirm nothing named cass is executing right now (see §9 — the only cass-adjacent
activity visible is other agent sessions *investigating* the residue, not cass
itself running). All /tmp residue reads as safe to delete right now, pending the
coordinator's own final lsof check at actual deletion time (state can change).

**NEW, outside the prompt's named glob:** `/var/folders/2j/3tf17x5d7pj10gdjbp1ydpf40000gn/T/cass-cf-deploy-*`
— 88 entries, **0 bytes each** (empty directories/markers). I could not identify
the producer in bounded time (not `cass`, not obviously `wrangler`/cloudflare-pages-publish
skill — no hits for the literal string `cass-cf-deploy` anywhere I checked: agent-config,
the cloudflare-pages-publish skill, the pnpm wrangler package, `/opt/homebrew`). Given they
are 0 bytes, deleting them costs nothing; I'm flagging the naming collision so nobody
assumes it's coding-agent-search residue without checking, and so nobody skips it either
since it does contain the literal substring "cass". Recommend the coordinator either
identify the real producer or just sweep these empty markers along with the rest since
they're free either way.

### Repo build residue (inside `/Users/dalecarman/dev/coding_agent_session_search`)

`.claude/worktrees/` — **69G total**, 6 linked git worktrees:
```
 22G  cass-759l7-spin-wait
 97M  cass-gen5-honesty
 97M  cass-nvq59-status-hang
 17G  cass-p3kgr-gen13
 97M  cass-to-green-c6bfb589
 30G  codex-coverage-gap-2bh4a
```
`target/` (repo's own top-level build dir): **2.7G**

These worktrees are **linked worktrees sharing the main repo's `.git`**, not separate
clones — see §7. lsof was not separately run against each (bounded effort; the tmp
spot-check above and the zero-process finding in §9 apply equally here — no cass
process is running anywhere on the machine right now, so nothing is pinning them).
One caveat from a sibling session's own investigation (see §9): as of ~15h before this
sweep, `/private/tmp/cass-759l7-forward` had "8 dirty + 1 commit not on origin/main" —
i.e. **some of this worktree/tmp content may be unlanded work**, not pure build cache.
The coordinator should not blind-sweep worktrees without checking for unpushed commits
first (`git -C <worktree> log @{u}.. --oneline` / `git status`).

## 4. Autostart

- `launchctl list | rg -i 'cass|coding.agent'` → **empty**. Full unfiltered list also
  checked for `mini`/`sync-to-mini`/the bead prefix `9wz3` → only unrelated hits
  (`com.notes.sync-to-mini`, `agent-config-mini-sync-...`).
- `~/Library/LaunchAgents`, `~/Library/LaunchDaemons`, `/Library/LaunchAgents`,
  `/Library/LaunchDaemons` — every `.plist` in all four directories individually
  content-scanned for the substring `cass` (not just filename-matched). **Zero matches.**
  The watchdog is genuinely unarmed on this machine — confirmed by absence of any
  registration, not just by process absence.
- `crontab -l` → no cass entries (only a disabled qmd line, unrelated).
- `~/.local/share/cass/watchdog.sh` (4029B) + `.bak` (2592B): heartbeat-based watchdog
  script, designed to run via launchd every 10 min and kill/restart a stale
  `cass index --watch` process. Confirmed unarmed (no plist references it anywhere on
  this machine) and confirmed non-functional right now anyway (`watcher-heartbeat`
  file doesn't exist, so its own logic would fall back to `pgrep -f "cass index --watch"`,
  which also finds nothing).

**Resurrection-vector note on autostart:** `~/dev/dropbox-cli/launchd-report.md`
(line 65-70, this laptop, a different repo) **documents** two launchd labels —
`com.cass.index-watch` (`KeepAlive: true`) and `com.cass.health-watchdog`
(`StartInterval: 600`) — as if they exist/existed somewhere. I could not find either
plist on THIS machine (see the exhaustive scan above). This report may describe the
Mac mini's launchd state (dropbox-cli's report-generation may have scanned there) or
may be stale documentation of a past/aspirational laptop config. **Flagging for the
coordinator to check the mini specifically** — if `com.cass.index-watch` /
`com.cass.health-watchdog` are real, live plists on `mini-ts`, that is a genuine
autostart resurrection vector this lane cannot see (out of scope: laptop only).

## 5. Shell configuration

`~/.zshrc`, `~/.zshenv`, `~/.zshrc.local`, `~/.zprofile`: **zero matches** for `cass`
(case-insensitive). `~/.secrets/agent-keys.env`: **zero matches** (key names only
would have been reported; there was nothing to report).

`~/.zsh_history` (1056 lines total): **zero matches** for `\bcass\b` as a
whole-word token. A looser case-insensitive substring search found exactly 2 lines,
both unrelated (`cass_data` inside an unrelated `.tmp*` cleanup one-liner, not the
`cass` binary — this is a coincidental substring, part of a directory-name pattern
`$d/cass_data`, itself worth a second look but not a cass invocation).
**No direct `cass <subcommand>` invocations appear anywhere in this laptop's shell
history.** This is a genuinely surprising negative given ~467MB of rollback binaries
and 9 dated versions — it means cass has been invoked exclusively from agent sessions
(non-interactive `cass --robot`/`--json` calls issued by Claude/Codex tool calls, per
the AGENTS.md instruction text found in §7) rather than typed by hand at an
interactive prompt, or the invoking shell/tool bypasses history recording.

**Canonical mini alias** (confirmed present, exact text, at
`/Users/dalecarman/.agent-config/configs/shell/zshrc.local.mini` lines 30-32 —
this is inside `.agent-config`, flagged here only for completeness since it was
part of the named known-inventory item and I happened to resolve its exact
location/lines):
```
30:# CASS MacBook mirror — synced nightly from MacBook Pro
31:export CASS_MACBOOK_DB="$HOME/cass-mirror/agent_search.db"
32:alias cass-macbook='cass --db "$CASS_MACBOOK_DB"'
```
Numerous copies of `zshrc.local.mini` also exist under various `~/.claude-accounts/*/jobs/*/tmp/...`
and `~/.codex/worktrees/*/` scratch trees (ephemeral job/worktree checkouts of
.agent-config itself, not independent installations) — not resurrection vectors,
just artifacts of other sessions' work; not enumerated individually here.

## 6. Other repos under ~/dev — genuine references (excludes the cass repo itself and
the brand-new `groove-session-search` replacement project, per task scope)

Ran a bounded `rg -il '\bcass\b|coding[-_]agent[-_]search|coding_agent_session_search'`
across every directory in `~/dev` (excluding `target/`, `node_modules/`, `.git/`,
`*.svg`, `*.lock`, the cass repo itself, and `groove-session-search/`). **108 files
across ~27 other repos matched.** I pulled match context for every hit and triaged
each. Full raw output saved at `/tmp/dev-cass-context.txt` (this session's scratchpad,
not durable — reproduce with the command above if needed later).

### 6a. GENUINE, LIVE resurrection vectors (would actively try to invoke cass, or
instruct an agent to)

**`~/dev/gj-tool/bin/gj`** (lines 10591-10614, function `cmd_sessions`) — **this is
real, currently-working code**, not documentation. The `gj sessions [path]` subcommand
shells out to a Python venv inside `coding_agent_session_search/tui/`:
```
cmd_sessions() {
    # Launch CASS TUI for the current repo's coding agent sessions
    local workspace="${1:-$(pwd)}"
    local cass_tui_dir="${CASS_TUI_DIR:-}"
    if [[ -n "$cass_tui_dir" && -f "$cass_tui_dir/pyproject.toml" ]]; then
        : # use config path
    else
        die "CASS TUI not found. Set CASS_TUI_DIR in ~/bin/gj-config.env"
    fi
    local venv_python="$cass_tui_dir/.venv/bin/python"
    if [[ ! -f "$venv_python" ]] || ! "$venv_python" -c "import cass_tui" 2>/dev/null; then
        log_info "Setting up CASS TUI environment..."
        python3 -m venv "$cass_tui_dir/.venv"
        "$venv_python" -m pip install --quiet -e "$cass_tui_dir"
        log_success "CASS TUI environment ready"
    fi
    ...
```
I confirmed `/Users/dalecarman/dev/coding_agent_session_search/tui/` **actually
exists** with a `.venv/` and a `cass_tui/` package inside it — this is a live,
currently-functioning integration, not vestigial. `gj sessions` will die with a
clear error once the repo is gone (it already has a `die()` guard for the missing-dir
case, so it fails loud rather than silently — but it IS wired in and someone could
try to "fix" it by re-cloning cass, which is exactly the resurrection vector Dale
named). Live config: **`~/dev/gj-tool/bin/gj-config.env` line 188**:
`CASS_TUI_DIR="$DEV_ROOT/coding_agent_session_search/tui"` (`DEV_ROOT=/Users/dalecarman/dev`,
line 7 same file) — this is the **active, non-example config file**, not the
`.example` template. Also: `~/dev/gj-tool/bin/gj-config.env.example` line 200 (commented
template copy), `~/dev/gj-tool/skill/SKILL.md` line 65, and
`~/dev/gj-tool/docs/AGENT-INSTRUCTIONS.md` line 52 (both just document the `gj sessions`
command as "CASS TUI for agent history").

**Three near-identical standalone `AGENTS.md` sections, one per repo**, each
instructing every agent session in that repo to use cass for prior-session search:
- `~/dev/destructive_command_guard/AGENTS.md:1195-1199` (`## cass — Cross-Agent Session Search`)
- `~/dev/ultimate_bug_scanner/AGENTS.md:739-743` (same heading, near-identical body)
- `~/dev/asupersync/AGENTS.md:852-856` (same heading, near-identical body)

Each says roughly: "`cass` indexes prior agent conversations (Claude Code, Codex,
Cursor, Gemini, ChatGPT, etc.)... **Rules:** Never run bare `cass` (TUI). Always use
`--robot` or `--json`." This is a template block that was copy-pasted into at least
3 separate repos' global agent instructions — genuinely the exact "resurrection
vector" shape Dale described: any future agent session reading any of these three
`AGENTS.md` files will be told to run `cass --robot ...` as standing instruction.
Companion `SKILL.md` table rows in the same two of three repos:
`~/dev/destructive_command_guard/SKILL.md:453` and `~/dev/ultimate_bug_scanner/SKILL.md:307`.

**`~/dev/agent_flywheel_clawdbot_skills_and_integrations/`** — an entire separate
skill-manager repo that ships cass as one of its installable skills:
- `skills/cass/SKILL.md` (full skill definition, `name: cass`, "NEVER run bare `cass`")
- `skills/cm/SKILL.md` ("CASS Memory System" — references `~/.local/share/cass/` directly
  as its "Episodic Memory" layer path)
- `skills/ntm/SKILL.md:354-358` — a DIFFERENT tool (`ntm`) with CASS integration flags:
  `--robot-cass-search=`, `--cass-since=`
- `README.md:25-26,134` — table rows advertising cass + cm, and a config block
  `"cass": { "enabled": true }`
- `install.sh:36-37,58` — array entries registering cass and cm as installable

If this repo's `install.sh` is ever run, or its skills symlinked into a Claude/Codex
skills directory, cass comes back as a live skill. This is a real repo (not a fork of
agent-config), separate from anything in `.agent-config`.

**`~/dev/gstack/setup-gbrain/memory.md`** — a "## CASS is optional" section
(already gracefully degradable: "If CASS is broken or too stale, leave it out of the
startup path") — low risk of actually re-triggering cass, since it's written
defensively, but it's live user-facing documentation that still describes cass as an
available integration and should be updated/removed once cass is gone. Companion:
`gstack/test/brain-sync.test.ts:108` — this one is **not** a real dependency; it's a
test asserting `'cass'` is correctly *rejected* as an unrecognized
`transcript_ingest_mode` value (confirms cass was never wired into gstack's own
config enum). Noise, not a vector — no action needed.

### 6b. Historical/documentation-only references (won't execute anything, but

mention cass as precedent, evidence, or citation — lower priority, but Dale asked for
exhaustive so listing them):

- `~/dev/apple-notes-export/specs/003-remote-notes-access/{spec,plan,shaping,planning-transcript}.md`
  and its `thoughts/shared/handoffs/.../finalize.yaml` — cites the cass nightly-sync
  pattern (`~/.local/bin/cass-sync-to-mini.sh` + `com.cass.sync-to-mini.plist`) as
  design precedent for an already-shipped feature (apple-notes-export has its own,
  separate sync mechanism now). Historical design rationale; doesn't invoke cass.
- `~/dev/Continuous-Claude-v3/thoughts/shared/plans/2026-01-13-unified-artifact-system.md`
  and its handoff — mentions `cass search "..." --robot` as a technique example.
- `~/dev/PfizerOutDoCancerV3/specs/014-portrait-option-change-crash/{spec.md,artifacts/boundary-B-5bb93067-859816d1.md}` —
  cites "the CASS research doc" as evidence for a load-bearing "explicitly NOT safe
  to revert" decision about a memory-crash fix. **Note for coordinator:** this
  doesn't need cass to keep running (it's citing a past finding), but if "the CASS
  research doc" itself only exists as raw session content inside cass's now-being-deleted
  index rather than as its own durable file, that citation becomes unverifiable later.
  Worth a quick check by whoever owns Pfizer before the 77G data dir is deleted.
- `~/dev/gj-tool/specs/007-set-e-safety-cleanup/{plan.md,tasks.md,shaping-transcript.md}`
  and `~/dev/gj-tool/specs/017-dropbox-to-dev-migration/plan.md` — older, already-closed
  spec docs containing the same `CASS_TUI_DIR="$DEV_ROOT/coding_agent_session_search/tui"`
  line as historical planning text (the live version is the config file in §6a, not
  these).
- `~/dev/orchestrator/Docs/Testing/gj-tool-navigate-scenes.md` and
  `~/dev/PfizerOutDoCancerV3/Docs/Testing/gj-tool-navigate-scenes.md` — identical doc
  (copied into two repos) documenting `gj sessions` as "CASS TUI".
- `~/dev/gj-tool/thoughts/shared/handoffs/.../checkpoint.yaml` and
  `.../last-sessions-complete.yaml` — old handoffs mentioning CASS TUI as a
  design-comparison reference for gj-tool's own log viewer.
- `~/dev/dan-notion-workflow/docs/research/2026-07-22-precedent-mining.md:49` — one
  line, "(e) Prior-session findings (cass)".
- `~/dev/groove-sight-platform/` (6 files: `docs/gj-brain-decision-record.md`,
  `docs/gj-brain-brainstorm.md`, and 4 files under
  `thoughts/shared/handoffs/20260707-gj-brain-brainstorm/lanes/` and
  `.../20260816-cortex-vision-alignment/lanes/read-doctrine.md`) — architecture
  documentation that lists cass as one of "five disjoint personal memory stores" and
  explicitly scopes it **out** of the company brain ("Dale-personal corpus: stays
  out"). These docs will read as stale once cass is retired (they discuss it as a
  live, available-but-excluded system) but nothing here executes cass — a
  documentation-freshness issue for that repo's own owners, not a resurrection risk.
  Also two data files that show the mini's dev-wiki has **already compiled and
  indexed pages about the cass repos** as wiki entities:
  `groove-sight-platform/specs/002-gj-brain-phase0-falsifier/manifest/presence-manifest.jsonl`
  (lines 10594, 10600, 10735 — entities for both `cass-custom-disabled` and
  `coding_agent_session_search`) and two `eval/receipts*/query/C-REC-07.json` files
  citing `wiki/entities/cass-custom-disabled.md` as a retrieval source. This means
  the **dev-wiki itself holds compiled content about cass** — out of scope for this
  laptop sweep (wiki lives on the mini / is a separate maintained corpus) but worth
  the coordinator flagging to whoever owns `wiki-maintainer` for that wiki.
- `~/dev/gstack/setup-gbrain/memory.md` — already covered in 6a.

### 6c. False positives / noise — confirmed NOT cass-the-tool, no action needed

- `~/dev/goalbuddy/` (12 files under `specs/003-purpose-contract-gate/` +
  `internal/test/check-goal-state.test.mjs` +
  `thoughts/shared/handoffs/bd-k9m-.../finalize.yaml`) — "cass" here is used purely
  as an **illustrative example name** for an unrelated goalbuddy failure-mode pattern
  ("cass-shaped artifact-green/outcome-red completion" — i.e. "spec looks done but the
  live outcome is false," a pattern goalbuddy borrowed from an earlier cass incident
  as a memorable case study). No functional or even documentary dependency on cass
  existing; these are goalbuddy's own permanent test/spec vocabulary now. Confirmed
  by reading full context — not just a title match.
- `~/dev/agent-observer/` (18 files, mostly under
  `thoughts/shared/handoffs/20260817-blocked-on-dale/lanes/*` and
  `specs/059-*/`, `specs/060-*/`, `specs/058-*/`, `specs/037-*/`) — this is a
  **currently-running sibling investigation** into the same disk-usage/wedged-process
  situation this sweep is part of (dated today, 2026-08-17). Two of its own test
  fixture files (`lib.test.mjs`, `ui-logic.test.mjs`) and `lib.mjs` also use the real
  `coding_agent_session_search` repo path as example/fixture data in agent-observer's
  own test suite — not a functional dependency, just realistic test data. See §9 for
  what this parallel investigation found (it corroborates several items in this
  report independently — including a hung/wedged 328-minute cass search process it
  observed earlier, which is not running anymore per my own `ps`/`pgrep` check).
- `~/dev/cmux/ghostty/src/stb/stb_image.h:109` and
  `~/dev/ghostty/src/stb/stb_image.h:109` — "Cass Everitt" is a person's name in a
  vendored third-party C library's author-credits list (`stb_image.h`). Totally
  unrelated.
- `~/dev/operator/specs/037-mini-dev-local-checkouts/artifacts/implementation/{group-a,group-b,group-c}/*`
  (7 files) — references to a directory literally named `data/markdown/cass/` inside
  a **different person's** (Chip's) personal notes repo,
  `chip-memory` at `/Users/chipcarman/Library/CloudStorage/Dropbox-GrooveJones/Chip/dev/chip-memory`.
  This is Chip's own topic-organized markdown notes about "cass" as a subject (or an
  unrelated abbreviation) — not a functional dependency on the cass binary, and not
  reachable/actionable from this laptop sweep (it's Chip's Dropbox path, referenced
  only inside `operator`'s migration-planning artifacts). Flagging only because it
  matched the search; no action recommended.
- `~/dev/hsbc/refactor/artifacts/2026-05-08T152208-codex-simplify/{skill-inventory.log,skill_inventory.json}` —
  a historical point-in-time snapshot from HSBC repo's own skill audit showing "cass
  present" at `~/.claude/skills/cass` as of 2026-05-08. Just a dated log; not live.
- `~/dev/pi-messenger/specs/002-multi-runtime-support/claude-stream-format.jsonl` —
  one line inside a captured raw Claude Code session JSON dump where "cass" appears
  in that session's full skills-list array (evidence the skill was loaded then, not
  a functional reference).
- `~/dev/claude-usage/specs/005-jobs-store-reader/lanes/{r10-conformance,r10-correctness,r10-size}.md` —
  test fixtures that use `~/dev/coding_agent_session_search` as one of several real
  repo paths to validate claude-usage's own job-store reader against; no dependency
  on cass itself.

## 7. Git origin and other clones

`/Users/dalecarman/dev/coding_agent_session_search` has two remotes:
```
origin    https://github.com/carmandale/coding_agent_session_search.git (fetch/push)
upstream  https://github.com/Dicklesworthstone/coding_agent_session_search.git (fetch/push)
```
`gh repo view carmandale/coding_agent_session_search --json isFork,isPrivate,visibility,parent`:
```json
{"isFork":true,"isPrivate":false,"visibility":"PUBLIC",
 "parent":{"name":"coding_agent_session_search","owner":{"login":"Dicklesworthstone"}}}
```
**This is a PUBLIC fork of an upstream repo Dale does not own**
(`Dicklesworthstone/coding_agent_session_search`), not a private Dale-owned repo. This
matters for the coordinator's archive decision — per `.agent-config`'s
`open-source-fork-setup` convention, the correct disposition for a fork being retired
is generally different from a private repo (e.g. leaving the fork as-is / deleting
Dale's fork only, never touching upstream) — that decision is the coordinator's, not
mine, but the fork/public/upstream facts above are load-bearing for it. (First `gh
repo view` attempt returned `HTTP 503`, transient GitHub API issue; retry succeeded.)

**Other clones on this machine:** none found. `fd` for `Cargo.toml` files anywhere
under `~/dev` whose `name = "coding-agent-search"` returned only the 6 linked-worktree
copies under `.claude/worktrees/` (which share the main repo's `.git`, not independent
clones — see below) plus the main repo's own `Cargo.toml`. No `.git/config` anywhere
else under `~/dev` references this origin. The Dropbox fallback path some old
`gj-tool` spec docs mention
(`~/Groove Jones Dropbox/Dale Carman/Projects/dev/coding_agent_session_search`)
**does not exist** — confirmed via direct `ls`. **Conclusion: exactly one real
checkout of this repo exists on this laptop**, at
`/Users/dalecarman/dev/coding_agent_session_search`, plus its 6 linked git worktrees
(`.claude/worktrees/{cass-759l7-spin-wait, cass-gen5-honesty, cass-nvq59-status-hang,
cass-p3kgr-gen13, cass-to-green-c6bfb589, codex-coverage-gap-2bh4a}`, 69G total, see
§3) which are not separable clones — removing the main repo removes them (or they need
explicit `git worktree remove`/manual deletion first, since a bare `rm -rf` on the
worktree dirs without `git worktree prune` afterward will leave dangling
`.git/worktrees/*` metadata in the main repo's `.git`, which won't matter once the
whole repo is archived/deleted but is worth naming for the coordinator's runbook).

I did not find and did not search for a second full-machine clone outside `~/dev`
(Documents/Desktop bounded check via the second `fd` command in my transcript
returned only the same repo and its two Application Support data dirs — no other
clone).

## 8. Additional finds not in the original known-inventory list

- **`~/dev/cass-custom-disabled/`** — **295M**, 3 timestamped subdirs from
  2026-05-17 (`20260517T-upstream-install/`, `20260517T051314/`,
  `20260517T051409/`). Contains: 6 old rollback Mach-O binaries named things like
  `cass.real.pre-upstream-0.4.8`, `cass.real.BROKEN-PATCH-OVERRIDE-20260516-005800`,
  `cass.v042-prev` (these predate the current `~/.local/bin/` naming scheme —
  evidence of an even earlier cass version history than the 9 binaries in §1); a
  `cass-sync-to-mini.sh` script (confirmed **not** currently armed via any launchd
  plist on this machine — see §4); generic `br` (beads) git hooks unrelated to
  cass specifically; and old `.ralph-o`/`.pi/messenger`/goalbuddy-board scratch data
  from unrelated sessions that happened to be captured in this quarantine snapshot,
  including three old goalbuddy boards for goals literally named
  `cass-session-ingestion-recovery`, `pi-agent-memset-stall`, and
  `watch-once-streaming-scan`. This whole directory reads as a **quarantine/backup
  snapshot taken during an old cass reinstall (~May 17)**, analogous in spirit to
  the `.local/bin/cass.pre-*` rollback pattern but from an earlier "custom install"
  era predating the current `.local/bin/cass` location. Full resurrection-vector
  weight is low (nothing here is armed/wired in), but it is 295M of cass-attributable
  disk that the original inventory list did not mention at all.
- **`~/Library/Application Support/coding-agent-search/`** (no `com.` prefix) — see
  §2, the macro-recording directory, 284K, not in the original inventory.
- **`~/Library/Application Support/CrashReporter/{cass,coding_agent_search}*.plist`** —
  see §2, two 240-byte marker plists, not in the original inventory.
- **`/var/folders/.../T/cass-cf-deploy-*`** — see §3, 88 empty (0-byte) dirs of
  unidentified origin, not in the original inventory, not clearly cass-the-tool at all.
- **`~/dev/cass-custom-disabled/`** itself as a top-level `~/dev` entry — not in the
  original inventory (which named repos to check *for references*, not this as a
  target).

## 9. Live-process / concurrent-session context (informational, not a finding to act on)

At the time of this sweep, **no cass process is running** (`ps aux`, `pgrep -fl cass`
both confirm this — the only matches are this session's own `rg` invocations and two
unrelated sibling Claude Code sessions' shell wrappers that happen to contain the
literal substring "cass" inside quoted `rg` patterns and job-ID variable names).
However, several **other live/recent sessions are independently investigating this
exact same cass footprint right now**:
- `~/dev/agent-observer/thoughts/shared/handoffs/20260817-blocked-on-dale/` (today,
  in-progress, multiple lane files) — appears to be a disk-usage/hung-process
  investigation that independently found and documented: a previously-running
  `cass search frankensqlite` process that had accumulated 328 minutes of CPU before
  presumably being killed (not running now, per my own check), the same
  `cass-759l7-forward` worktree/tmp pair with "8 dirty + 1 commit not on origin/main"
  (a genuine unlanded-work flag — see §3's worktree caveat), and a `coding_agent_session_search`
  background job in state `blocked`. This is corroborating, independent evidence for
  several items above, not something I verified myself — cite the agent-observer
  handoff directly if the coordinator wants the underlying detail.
- Two sibling shell processes visible in `ps aux` at scan time (pids 2248/2250,
  5057/5059) were themselves running greps very similar to this sweep's own commands
  (`rg -i -c '\bcass\b|...'` against `.agent-config`, and a `state.json` lookup for a
  list of cass-related background job IDs) — consistent with parallel lanes of this
  same retirement effort running concurrently, not a separate concern.

None of this changes any deletion-safety conclusion in this report (nothing is
currently running or holding a handle), but the coordinator should be aware other
sessions are actively looking at the same paths right now and may have fresher
process-state information at actual execution time.
