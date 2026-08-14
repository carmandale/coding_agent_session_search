# Lane: freshness-and-skill

Read-only grounding lane. Owns "stays up to date" and the agent-facing entry
point (the `cass` skill). Does not fix anything. All claims below are tagged
MEASURED (I ran the command and quote its output) or INFERRED (I reasoned
from measured facts without an independent probe).

Repo: `/Users/dalecarman/dev/coding_agent_session_search`
Date: 2026-08-14

---

## 1. Scheduling null result — re-verified and extended

MEASURED, independently, using different commands than the original probe:

```
$ launchctl list | wc -l
     616
$ launchctl list | rg -i cass
(no output, rc=1)
$ crontab -l
PATH=/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin
# qmd daily catch-up + duration tracking (DISABLED 2026-03-26 — no new data since Mar 9)
# 0 0 * * * /Users/dalecarman/dev/qmd/scripts/qmd-daily.sh
(rc=0; the one entry is commented out and belongs to an unrelated repo, qmd)
```

Extended past the original scope (which only checked `launchctl list`) to every
plist file on disk, not just loaded services, per-directory so a permission
error on one file can't hide a real rc=1 in another:

```
$ for d in ~/Library/LaunchAgents /Library/LaunchAgents /Library/LaunchDaemons; do
    rg -l watchdog.sh "$d"
  done
--- ~/Library/LaunchAgents ---   rc=1 (no match)
--- /Library/LaunchAgents ---    rc=1 (no match)
--- /Library/LaunchDaemons ---   rc=2, but the only stderr line is
    "Permission denied" on com.microsoft.teams.TeamsUpdaterDaemon.plist —
    an unrelated file. No match among the plists rg could read.

$ fd -e plist . ~/Library/LaunchAgents /Library/LaunchAgents /Library/LaunchDaemons | wc -l
  per-dir: 32 + 12 + 22 = 66 plist files total, zero mention cass or watchdog.sh
```

Claude hooks: `python3` parse of `~/.claude/settings.json`'s `hooks` object
found zero `cass` substring in any hook command (checked structurally, not by
grep, so a match inside an unrelated JSON blob can't produce a false
positive). `rg -il cass ~/.claude` (excluding the skill directory itself,
telemetry, file-history, and job scratch dirs) turns up only skill reference
files that mention `cass` as a tool name (`rch`, `rust-unsafe-code-exorcist`)
— none of them a hook registration.

`atq` (at-jobs): empty, rc=0.

**Extension beyond the given null result:** the leftover
`~/.local/share/cass/watchdog.sh` (dated 14 Mar, pre-upstream-reset) polls a
heartbeat file at
`~/Library/Application Support/com.coding-agent-search.coding-agent-search/watcher-heartbeat`.
MEASURED: that file does not exist (`rg -i heartbeat` over a full `ls -la` of
the support dir returns nothing; the directory listing has no file by that
name). No `cass index --watch` process is running (`ps -Ao pid,etime,command`
over the whole process table has zero matches other than unrelated
`CoreMediaIO` system helpers whose names happen to contain no `cass`
substring at all — i.e. truly zero). So the watchdog script's own precondition
(a launchd-supervised `cass index --watch` process) has never existed on this
machine in the current era; the script is dead weight next to a dead
heartbeat check, not a live-but-broken watchdog.

**Verdict: the null result holds and is now measured two independent ways
(loaded services + on-disk plist content) with zero matches in both.**

---

## 2. Does cass ship a native watch mode, and why isn't it running?

**Yes — `cass index --watch` is a real, upstream/native CLI feature, not
fork-local machinery.** MEASURED from source, HEAD `d5cea071`:

- `src/lib.rs:79286` — `fn run_index_with_data(... watch: bool, watch_once: Option<Vec<PathBuf>>, watch_interval: u64, ...)`.
- `src/lib.rs:79541` (`indexer::run_index(opts_clone, None)`) spawns the
  indexer in a background thread with `IndexOptions { watch, watch_once_paths,
  watch_interval_secs, .. }` — a real code path, not a stub.
- `Commands::Index { full, force_rebuild, watch, watch_once, watch_interval, .. }`
  is wired at `src/lib.rs:5990-6010` from real CLI argument parsing (the flag
  names `watch`, `watch-once`, `watch-interval` are in the CLI flag list at
  `src/lib.rs:4250,4294-4295`).
- `README.md:2569` documents it: `| index --watch | Daemon mode: watch for
  file changes, reindex automatically |`, and `README.md:2319`: "The `--watch`
  flag enables real-time index updates as agent files change."

So the repo's own documentation calls `cass index --watch` "daemon mode" and
names it as the intended freshness mechanism. It is a **foreground,
long-running process** — the watch loop lives inside the same process that
`cass index --watch` starts (confirmed by the thread-spawn pattern above; it
does not fork or detach). It therefore needs an external supervisor to keep it
running across crashes/reboots/logouts — which is exactly the role the
(now-removed) `com.cass.index-watch` launchd plist used to play, per bead
1vxuf's own words: "the `cass watchdog install` path that used to write both
plists was fork-local machinery removed by the 2026-05-17 upstream reset, so
there is currently no supported command that installs the watcher."

**Why it isn't running:** nobody has started it and nothing supervises it.
MEASURED: `com.cass.index-watch` is not loaded (`launchctl list | rg cass` —
empty) and no plist by that name exists on disk (0/66 plists mention `cass`).
Bead 1vxuf's own May-vs-August comparison shows this has been true the whole
time: "com.cass.index-watch loaded? no -> still no."

**Bead 1vxuf** (`coding_agent_session_search-1vxuf`, IN_PROGRESS,
P2): confirms the watcher is not installed and states the direct
consequence — `cass health --json` on 2026-08-10 read `healthy=false,
errors=["index stale"]`, `last_indexed_at 2026-07-22T18:04:27Z`, age 18.6
days, "So 'keep the watcher installed so future sessions stay indexed' is not
done, and every session since mid-July is unsearchable." It also separates
three entangled jobs: (a) the March-May 2026 codex/pi coverage hole (source
files present, never indexed — a different bug from freshness), (b) the
watcher-install decision (paired with 2gif2), (c) upstream sync (460 commits
behind as of 2026-08-10). My lane's scope is (b).

**Bead 2gif2** (`coding_agent_session_search-2gif2`, IN_PROGRESS, P2):
the *health-watchdog CLI supervisor* (`cass watchdog run/install/uninstall`)
is a **different thing** from `cass index --watch` and is the fork-local
piece that got dropped. Its own 2026-08-10 triage note is explicit and I
re-confirmed the reproduction independently:

```
$ /Users/dalecarman/.local/bin/cass watchdog run --help
Could not parse arguments
```

Source confirms there is no `watchdog` subcommand at all — `watchdog` falls
through to the default `search` subcommand as a query string, which is why
`cass watchdog --help` (one token, parses as `search --help`, exits 0) behaves
differently from `cass watchdog run --help` (two tokens, `search` only takes
one positional QUERY, exits 2). The bead's own recommendation is the right
frame for this lane too: "this is no longer a repair. It is a decision —
whether to re-add fork-local watchdog machinery on top of a fork that is now
[hundreds of] commits behind upstream/main... Recommend re-scoping this bead
to 'decide the watcher supervision story for the post-reset fork.'"

**Important nuance for the freshness fix, not previously stated in the
established facts:** the coverage-floor hang (bead 1a7mk,
`read_connector_scan_floors_bounded` at `src/lib.rs:15099-15108`) has exactly
**one call site** in the entire source tree — `src/lib.rs:65457`, inside
`fn run_health` (`src/lib.rs:65364`). MEASURED: `rg -n
'read_connector_scan_floors_bounded\(' src/*.rs` returns one hit. `run_health`
is invoked from exactly two places (`src/lib.rs:5680` and `src/lib.rs:6886`),
both under `Commands::Health` dispatch — not from `Commands::Index`/watch,
`Commands::Status`, `Commands::Triage`, or `Commands::Stats`. **`cass index
--watch`'s own loop does not call this function.** So the coverage-floor hang
and the freshness/watch-mode gap are two separate, independently-fixable
problems: getting `cass index --watch` running under supervision does not, on
this evidence, require the coverage-floor bug to be fixed first, and fixing
the coverage-floor bug does not by itself make `cass index --watch` start
running. (The established facts state the hang also reproduces on `cass
status`/`triage`/`stats` on the live archive; I did not re-derive that
mechanism here — it's outside this lane's assignment — but the specific claim
"only `run_health` calls the bounded-floor function" is mine, from a direct
`rg`, and narrows where the coverage-floor lane needs to look.)

---

## 3. The repo has already decided against custom launchd wrappers

Both citations verified at the exact line numbers given, quoted verbatim:

**`specs/017-watch-once-lexical-oom/spec.md:65`** (Out of Scope):
> Replacing the documented watcher with custom launchd/scripts.

**`specs/018-lexical-refresh-finalization/spec.md:38`** (Requirements):
> Keep the fix upstream-clean. Do not rely on local launchd scripts, manual DB edits, index deletion, or environment-variable operator rituals.

Two more instances of the same decision, found while confirming what "the
documented watcher" means:

**`specs/017-watch-once-lexical-oom/spec.md:50`**:
> Do not restart the documented watcher against live data until the targeted failure is fixed and verified in a temp/live-clone path.

**`specs/018-lexical-refresh-finalization/spec.md:65`** (its own Out of Scope):
> Rewriting the watcher or replacing the documented watcher with custom infrastructure.

**`docs/goals/cass-lexical-refresh-finalization/goal.md:31`** (Purpose
Contract, tangible outcome):
> Dale can actually use cass against recent and historical sessions through documented or upstream-compatible behavior, without custom local watcher/index band-aids.

"The documented watcher" resolves unambiguously to `cass index --watch`:
`README.md:2569` is the only place in the repo that documents watcher/daemon
behavior as a named feature, and it names exactly that flag ("Daemon mode:
watch for file changes, reindex automatically").

**This constrains the freshness fix as directly as the task brief states it:**
four separate documents (two specs' scope sections, one spec's own
requirement text, one goal's purpose contract) all forbid solving this with a
hand-rolled shell/launchd wrapper. The only repo-sanctioned shape for a
freshness fix is getting cass's own `cass index --watch` running and kept
running — which is a supervision problem (something needs to start it, keep
it alive, and restart it if it dies), not a reason to write new indexing
logic. A plain launchd plist that runs `cass index --watch` as its `Program`
and sets `KeepAlive` (or `RunAtLoad` + `KeepAlive`) is supervision of the
documented mechanism, not "custom launchd scripts" replacing it — the banned
thing is a shell script that re-implements watching/scanning/restart logic
(exactly what the old `watchdog.sh` did with its own heartbeat polling and
kill/restart sequence). That distinction is not stated explicitly in any of
the four quotes above; it is my reading of "replacing the documented watcher"
vs. supervising it, and the coordinator should treat it as INFERRED, not
settled — worth a one-line confirmation from Dale before landing a plist,
since the old watchdog.sh's launchd plist (now gone) may be exactly the shape
that was rejected here for reasons this lane didn't excavate.

---

## 4. The skill problem — confirmed exactly as briefed, plus root cause data

MEASURED:

```
$ ls -la ~/.claude/skills/cass/
drwxr-xr-x@ - dalecarman  9 May 03:58 references
drwxr-xr-x@ - dalecarman  9 May 03:58 scripts
.rw-rw-r--@ 3.7k dalecarman  9 May 03:58 SELF-TEST.md
.rw-rw-r--@  28k dalecarman  9 May 03:58 SKILL.md
$ [ -L ~/.claude/skills/cass ] && echo SYMLINK || echo "REAL DIRECTORY"
REAL DIRECTORY
$ wc -l ~/.claude/skills/cass/SKILL.md
     520 SKILL.md
```

Real directory (not a symlink), dated 2026-05-09, 520 lines — matches the
brief exactly.

Line numbers for `cass status --json && cass index --json` (or a variant
consuming its output), MEASURED via `rg -n`:

| line | context |
|---|---|
| 59 | `## THE EXACT PROMPT — Discovery Workflow`, step 1: "Bootstrap: Check health, refresh index, get project overview / `cass status --json && cass index --json`" |
| 121 | `cass status --json \| jq '{healthy, fresh: .index.fresh, ...}'` |
| 124 | `if [ "$(cass status --json \| jq -r '.index.stale')" = "true" ]; then` |
| 133 | prose referencing `cass status --json` for interactive-agent staleness decisions |
| 163 | `## Quick Reference` → `# Health + refresh (ALWAYS first)` → `cass status --json && cass index --json` |
| 515 | `cass status --json \| jq '.index.fresh'` |

All six sites confirmed by direct `rg -n` against the live file — the brief's
"~59 and ~163... three more sites at ~121, ~124, ~133, ~515" is accurate (six
sites total, two of them literally labeled "Bootstrap" step 1 and "ALWAYS
first").

**This makes the hanging command the mandated first action of the skill's own
documented workflow**, twice over, in the two places most likely to be read
first (the worked example at the top, and the "Quick Reference" cheat-sheet
most agents will grep for). Given the established fact that `cass status
--json` never returns on the currently-installed binary (43% CPU walking
125,601 raw-mirror blobs with no sqlite file open), any agent that follows
this skill literally — which is its entire purpose — hangs before doing
anything useful.

**Installer-hygiene.md's decision rule, applied here:** the rule (`~/.agent-
config/.claude/rules/installer-hygiene.md`) gives two branches for an
agent-config-vs-jsm name collision: "Keep jsm version → delete agent-config's
copy" or "Keep agent-config version → `jsm uninstall <name>` and re-run
`./install.sh` to restore the symlink." Neither branch applies verbatim here
because **agent-config currently has no cass skill at all** (confirmed below)
— this is not yet a collision, it's a gap. The rule's underlying logic still
answers the question: an in-place edit to a jsm-owned real directory is
explicitly the pattern the rule warns against elsewhere in the same file (the
topview-skill precedent: "A workaround copy made to route around a hidden
skill is still a second source, and it drifts backwards"), and
`single-source.md` states the same principle generally. The durable fix is
therefore: **author the corrected skill inside agent-config, `jsm uninstall
cass`, run `./install.sh` so agent-config's symlink occupies the now-empty
`~/.claude/skills/cass` path.** `install.sh:1230` ("CC skill collision... real
content exists, skipping") is exactly the collision-skip logic the rule
refers to, confirming that today, with jsm's real directory still present, an
agent-config install would silently no-op rather than deploy a fix — so the
`jsm uninstall` step is not optional, it's the precondition that makes
`install.sh` do anything at that path.

---

## 5. agent-config's existing cass coverage, and jsm's actual update behavior

**Agent-config has no cass skill.** MEASURED:

```
$ fd -t d cass ~/.agent-config/skills
(no output)
$ rg -il cass ~/.agent-config/skills --glob '*.md'
(8 files — all incidental: dhh-rails-style, gj-tool, testflight, a UML stencil,
compound-learnings, and a quarantined "cm" skill. None is a cass skill; "cass"
appears in unrelated prose/identifiers in each.)
```

Confirms and re-verifies the prior probe's finding.

**`jsm list` output:**

```
Installed Skills (5 total, 0 updates available)
NAME                       VERSION   STATUS     INSTALLED
cass                       7         ? unknown  2026-05-09
gcloud                     1         ? unknown  2026-05-08
rch                        5         ? unknown  2026-05-09
rust-unsafe-code-exorcist  8         ? unknown  2026-05-15
vercel                     4         ? unknown  2026-05-16
```

`jsm info cass` reports the catalog's current version as **v8** — one ahead
of the installed **v7**. The "0 updates available" summary is **not** a
currency check that passed; it's a check that never ran: every single
installed skill shows `? unknown` in the STATUS column (not just cass), and
the legend confirms `? unknown` is a real state distinct from `✓ current` or
`⬆ update available`.

**Correction to `~/.agent-config/.claude/rules/installer-hygiene.md`:** that
file states jsm "auto-updates daily via launchd." MEASURED, this is currently
false on this machine, and has been false for over five weeks:

```
$ cat "$HOME/Library/Application Support/jsm/config.toml"
[auto_update]
enabled = false
schedule = "daily"
time = "03:00"
...

$ cat "$HOME/Library/Application Support/jsm/auto-update-state.json"
{
  "enabled": false,
  "backend": "launchd",
  "last_run_at": "2026-07-04T08:58:51.208378+00:00",
  "last_run_status": "network_error",
  "last_run_message": "...TLS handshake failed: invalid peer certificate: UnknownIssuer..."
}

$ tail -3 "$HOME/Library/Application Support/jsm/auto-update-log.jsonl"
2026-07-02  network_error (same TLS error)
2026-07-03  success — "upgraded 1 skill(s)" (dueling-idea-wizards)
2026-07-04  network_error (same TLS error) — last entry in the file, period.
```

So auto-update ran daily and successfully through 2026-07-03, hit a TLS error
on 2026-07-04, and has not attempted since — `enabled: false` today, and zero
log entries in the 41 days between 2026-07-04 and today (2026-08-14). I also
confirmed no launchd job is currently registered for it (same plist search as
§1: zero `jsm` matches across all 66 plists), even though the state file
records `"backend": "launchd"` as its intended mechanism — consistent with
`enabled: false` meaning the plist itself was unloaded/removed, not merely
paused. I confirmed `jeffreys-skills.md` is reachable right now (`curl`,
`http_code=200`), so the TLS failure that triggered the disable was
transient/dated, not a standing network block — which matters because it
means "auto-update is currently off" is not a permanent property of this
machine, just its current state.

**What this means for durability of an in-place edit:** editing
`~/.claude/skills/cass/SKILL.md` directly would survive *today*, because
nothing is currently scheduled to overwrite it. It would **not** survive (a)
Dale or an agent running `jsm update`/`jsm sync` manually — the catalog
already has v8 sitting one version ahead of the installed v7, ready to pull
the moment anyone asks — or (b) `auto_update.enabled` being flipped back to
`true`, which given the log history is the normal state, not an edge case.
The five-week gap looks like an unresolved TLS incident sitting on top of an
otherwise-working daily job, not a deliberate, durable "off." **The fix must
not depend on jsm staying disabled.** This directly supports the §4
recommendation: move the corrected content into agent-config and `jsm
uninstall cass`, rather than editing the jsm-owned copy in place.

---

## 6. The mini — no scheduling, and its own cass has never even been initialized

MEASURED, `ssh mini-ts` (read-only commands only, as instructed):

```
$ cass --version
cass 0.6.23
$ launchctl list | grep -i cass
(nothing, rc=1)
$ crontab -l
*/15 * * * * /Users/chipcarman/openclaw/scripts/snapshot-sessions.sh >> /tmp/snapshot-sessions.log 2>&1
(rc=0 — one entry, confirmed unrelated: it's an OpenClaw usage-tracker script
 that reads OpenClaw's own trajectory files into an OpenClaw SQLite DB.
 `grep -i cass` against the script itself: rc=1, zero matches.)
$ grep -l -i cass ~/Library/LaunchAgents/*.plist
(nothing, rc=1)
```

No scheduling of any kind touches cass on the mini.

**Additional, unrequested but load-bearing finding:** the mini's own default
cass installation has never been run. `cass status --json` at the default
data dir (`/Users/chipcarman/Library/Application Support/com.coding-agent-
search.coding-agent-search`) returns in 12ms (`"elapsed_ms": 12`, not a hang —
this machine is on 0.6.23, past the coverage-floor regression window) with:

```json
"status": "not_initialized",
"database": {"exists": false, "opened": false, "conversations": 0, "messages": 0}
```

So "the mini holds 4,878 sessions" (established fact) is raw source material,
not an indexed archive on the mini — I confirmed the arithmetic directly:

```
$ ssh mini-ts 'find ~/.codex/sessions -name "*.jsonl" | wc -l'      # 1,293
$ ssh mini-ts 'find ~/.claude/projects -name "*.jsonl" | wc -l'     # 3,585
# 1,293 + 3,585 = 4,878 — exact match to the established figure
```

I found two other `agent_search.db` files on the mini, neither at the default
path and neither live:

```
/Users/chipcarman/rescue-20260805/agent_search.db          6,403,723,264 bytes, mtime Aug  5 03:44
/Volumes/SSD-2/SSD-1-mirror/cass-mirror/agent_search.db    6,403,723,264 bytes, mtime May 17 02:02
```

Byte-identical sizes, two different mtimes 80 days apart, both static (no
scheduling touches them, per the crontab/launchctl checks above, which
covered the whole machine, not just the default path). The path name
`SSD-1-mirror/cass-mirror` reads as a laptop→mini backup destination
(`SSD-1` is very likely the laptop's disk, mirrored onto the mini's `SSD-2`
external volume), not the mini indexing its own sessions. I did not open
either DB or trace the mirror job that produced them — that's outside this
lane's read-only-commands-only mandate for the mini and outside the
freshness/skill scope; flagging the paths and mtimes as a fact for whichever
lane owns cross-machine sync.

**Verdict for the cross-machine plan: the mini does not index itself in any
live sense.** Its own cass has never been initialized at its default path,
nothing schedules it, and the only DB files present are two frozen backup
snapshots. This does **not** change the cross-machine plan toward "the mini
handles itself" — it confirms the opposite: all 4,878 mini-side sessions are
exactly as dependent on a remote-source-sync or a similar cross-machine
mechanism as the established facts already assumed. `cass status --json`'s own
`remote_source_sync` block on the mini reports
`"configured_remote_source_count": 0` — nothing is configured on the mini side
either.

---

## Summary — durable fixes this lane can hand off

**Freshness:** the repo bans hand-rolled launchd/shell watcher machinery
(§3, four citations) and already ships a real, native long-running watch mode
(`cass index --watch`, §2) that the coverage-floor hang does not touch (its
one hang-triggering call site is exclusive to `cass health`, §2). The
supervision gap — nothing starts or restarts `cass index --watch` — is the
actual freshness bug, and it is a decision (per bead 1vxuf/2gif2, both
already IN_PROGRESS and owned outside this lane) about how to supervise the
documented command, not a new indexing mechanism to build. A plain launchd
plist whose `Program`/`ProgramArguments` is `cass index --watch` with
`KeepAlive` is the smallest thing that satisfies "documented watcher, not
custom infrastructure" — flagged above as my inference, worth a one-line
confirmation before landing.

**Skill:** `~/.claude/skills/cass/SKILL.md` is jsm's real (non-symlinked)
directory, v7 against a catalog v8, with jsm's auto-update currently disabled
by an unresolved five-week-old TLS incident rather than by durable
configuration — so an in-place edit is not safe against a future `jsm
update`/`jsm sync` or a re-enabled schedule. Agent-config has no cass skill
today (this is a gap, not yet a collision). The durable fix: write the
corrected skill into agent-config, `jsm uninstall cass`, then `./install.sh`
so the agent-config symlink takes the now-vacant path (`install.sh:1230`'s
collision-skip logic otherwise silently refuses to deploy over jsm's live
directory). The fix content itself must stop `cass status --json` (or any
command that shares its hang) from being the mandated first step at all six
measured line numbers — that's a content decision for whichever lane owns
the coverage-floor/hang fix, since the safe replacement command depends on
what that lane ships.
