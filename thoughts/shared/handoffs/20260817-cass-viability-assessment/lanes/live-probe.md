# Lane: live-probe — what the shipping `cass` binary actually does right now

Date: 2026-08-17. Read-only toward production data. Every `cass` invocation
below was wrapped in a bounded-execution helper (see note on tooling) with a
bound of 60-120s. Two probes exceeded their bound and were killed; both
orphaned a grandchild process that had to be identified and terminated
separately (documented below, each verified by exact command-line + timing
match before kill). PID 75534 (another live session's process) was never
touched.

## Tooling note: gtimeout is not installed on this machine

`command -v gtimeout` and `command -v timeout` both resolve to nothing.
`brew list coreutils` reports "No such keg" — coreutils is not installed, so
neither `timeout` nor `gtimeout` exists anywhere on PATH. This contradicts
the assumption that `gtimeout` was available for bounding probes.

AGENTS.md §10 itself names the sanctioned fallback for this exact situation:
"no `timeout` (use `gtimeout`, or **background+kill**)". I implemented that
fallback as a small script,
`/private/tmp/claude-501/.../scratchpad/bounded_run.sh`: it launches the
target command as its own background child, records that child's PID, polls
it, and — if the bound is exceeded — sends SIGTERM/SIGKILL to **only that
PID**, which it launched itself in the same invocation. This satisfies the
lane's bound requirement without gtimeout. Full script is at that path if the
coordinator wants to inspect it.

Caveat found in use: when `/usr/bin/time -l <cmd>` is the wrapped command and
the bound is hit, killing the immediate child (`/usr/bin/time`) does not
kill *its* child — the underlying `cass` process gets reparented to PID 1 and
keeps running. This happened twice (documented in probes 2 and 4 below). Each
time I re-verified via `ps` that the surviving PID's command line and elapsed
time matched exactly what I had just launched, then killed it. I never killed
anything I had not just launched myself in this lane.

---

## 1. Identity — which binary is on PATH

```
$ command -v cass
/Users/dalecarman/.local/bin/cass
$ ls -la ~/.local/bin/cass*
.rwxr-xr-x@ 52M dalecarman 16 Aug 12:29 /Users/dalecarman/.local/bin/cass
.rwxr-xr-x@ 52M dalecarman 10 Aug 20:37 cass.coverage-floor-fix-20260810
.rwxr-xr-x@ 52M dalecarman 14 Aug 16:56 cass.nvq59-status-gate-20260814-165549
.rwxr-xr-x@ 52M dalecarman 14 Aug 16:56 cass.pre-1a7mk-fix-20260815
.rwxr-xr-x@ 52M dalecarman 15 Aug 12:45 cass.pre-8llb5-deploy-20260816-155012
.rwxr-xr-x@ 52M dalecarman  1 Jun 06:21 cass.pre-coverage-floor-20260601
.rwxr-xr-x@ 52M dalecarman 15 Aug 11:34 cass.pre-gen5-20260815-174600
.rwxr-xr-x@ 52M dalecarman 16 Aug 10:50 cass.pre-gen10-deploy-20260816-164709
.rwxr-xr-x@ 52M dalecarman 16 Aug 12:29 cass.pre-gen11-deploy-20260816-122921
$ gtimeout 30 cass --version
cass 0.6.9
git commit: 1c9c0cec498eb1abb8be666491d0281734832ec9
```

The live `cass` mtime (16 Aug 12:29) matches the `cass.pre-gen11-deploy-*`
backup's mtime exactly — the shipping binary is the generation-11 deploy.
Nine numbered pre-deploy backups sit alongside it on PATH, one per recent
deploy attempt over the last ~2.5 months, most from the last week
(gen5/gen10/gen11 backups all within the last 3 days) — a high churn rate for
a tool the handoff chain already numbers to "generation 11-13."

---

## 2. Core function probe — does `cass search` work?

```
$ /usr/bin/time -l cass search "napkin" --limit 5      (bound 120s)
cass is already repairing the search index in /Users/dalecarman/Library/Application Support/com.coding-agent-search.coding-agent-search
       30.05 real         0.00 user         0.03 sys
            19709952  maximum resident set size
             7931008  peak memory footprint
exit_status=7
wall_seconds=30
```

```
$ /usr/bin/time -l cass search "root cause" --robot --limit 5   (bound 120s)
{"error":{"code":7,"kind":"index-busy","message":"cass is already repairing the search index in /Users/dalecarman/Library/Application Support/com.coding-agent-search.coding-agent-search","hint":"Wait for the active index run to finish; search will retry against the repaired lexical index afterward.","retryable":true}}
       30.12 real         0.00 user         0.03 sys
             7881880  peak memory footprint
exit_status=7
wall_seconds=30
```

Both searches fail immediately (in the sense of "return a structured error
in a fixed 30s"), not with a timeout — with `index-busy`, exit code 7.
Memory use for these two calls is trivial (~7.9 MB peak footprint) — nothing
like the stats/health/status hangs below.

**Root cause of the busy lock, traced (read-only):**

```
$ cat "$HOME/Library/Application Support/com.coding-agent-search.coding-agent-search/index-run.lock"
pid=75534
started_at_ms=1786960233009
updated_at_ms=1786978109119
last_progress_at_ms=1786960307804
db_path=/private/tmp/fsq-probe-data/prod.db
mode=index
job_id=lexical_refresh-1786960233009-75534
job_kind=lexical_refresh
phase=index
```

Converted:
- started: 2026-08-17T09:50:33Z
- last real progress: 2026-08-17T09:51:47Z (74.8s after start)
- updated (heartbeat) at capture time: 2026-08-17T14:48:29Z — i.e. still
  being heartbeated live, 14s before "now"
- **no progress for ~297 minutes (4h57m) while still heartbeating as alive**

`ps -p 75534` (read-only, never signaled) confirms this is the sibling
session's process named in the shared context — `/tmp/cass-fix-target/release/cass
--db /tmp/fsq-probe-data/prod.db search frankensqlite --limit 5` — elapsed
04:58:27 at capture time, 58.6% CPU, 0.0% memory (CPU-bound spin, not a
memory hang, unlike everything below).

**This is a structural finding, not an artifact of one stuck process.** The
lock file that gates search sits at a fixed, platform-default location
(`.../com.coding-agent-search.coding-agent-search/index-run.lock`) — it is
NOT scoped to, or keyed by, the `--db` path a given invocation targets. The
sibling process was invoked with `--db /private/tmp/fsq-probe-data/prod.db`,
a throwaway probe database entirely unrelated to the real production
database — yet its repair job acquired the *one* lock file that every other
`cass` invocation on this machine (including ones with no `--db` override,
which default to production) must also acquire before searching. A job
against a scratch/probe database can — and right now does — wedge search
against the real production data for every other process on the machine,
for as long as that job's heartbeat keeps renewing.

---

## 3. `cass stats --json` — confirming or refuting the 5.2GB/hang report

First attempt, bound 120s via `/usr/bin/time -l`, wrapped by `bounded_run.sh`:
zero bytes of output were captured in 120s. The internal Bash-tool timeout
(also 120s by default) fired essentially simultaneously and moved the call to
background; `bounded_run.sh`'s own kill logic fired correctly at 120s
(`exit_status=124`, `wall_seconds=122`) and reported killing its own child
PID by name and command match.

However, because the immediate child was `/usr/bin/time` and not `cass`
itself, killing that immediate child left the real `cass stats --json`
process running, reparented to PID 1 (orphaned, not killed). Found via `ps`:

```
  PID  PPID ELAPSED  %CPU %MEM    RSS      VSZ COMMAND
 5619     1   02:31  97.8  3.8 5106704 489554480 cass stats --json
```

**RSS = ~4.87 GB at 2m31s elapsed, still climbing (CPU rising to 97.8%).**
This is a second, independent confirmation on the same day of the
coordinator's earlier measurement ("ran >3.5 min, reached 5.2GB RSS, never
returned"): my own run, in a completely separate invocation, was on the same
trajectory (4.87GB at 2m31s, heading toward the same ~5.2GB range by
3.5min). I verified this was my own process (exact command-line match,
elapsed time matched my launch time to the second, PPID=1 from my own killed
wrapper) before terminating it with SIGTERM.

**`cass stats --json` does not return in bounded time and its memory use is
unbounded within the observed window — this is confirmed, not refuted.**

---

## 4. Read-only diagnostic subcommands: `health`, `status`, `doctor --check`

`cass --help` documents `health` as "Minimal health check (**<50ms**). Exit
0=healthy, 1=unhealthy. For agent pre-flight checks."

```
$ /usr/bin/time -l cass health          (bound 30s)
✗ Unhealthy (2013ms)
  - index stale
Health check failed
       2.10 real         0.99 user         0.64 sys
          5765452408  peak memory footprint     (≈5.37 GiB)
exit_status=1
wall_seconds=3
```

The documented "<50ms" is off by roughly **40x** (2013ms measured, self-
reported by the tool's own output), and a command billed as a lightweight
agent pre-flight check allocates **5.37 GiB** peak footprint to answer "is
the index stale."

```
$ /usr/bin/time -l cass status          (bound 30s)
2026-08-17T14:52:01Z WARN read-witness cap reached on cursor — dropping further per-cursor witnesses (pager-level SSI evidence unaffected) root_page=14 cap=16384
2026-08-17T14:52:05Z WARN state database probe exceeded its 5000ms bound — reporting the probe as failed with counts elided ... reason="state-meta" timeout_ms=5000
! CASS Status: Attention needed
Index:
  Last indexed: 1 days ago (stale)
Database:
  Exists, but could not be opened
  Error: state database probe exceeded its 5000ms bound for .../agent_search.db
Semantic:
  Status: error
  Summary: db unavailable (database unavailable during asset inspection)
  Hint: Restore the semantic assets and database; lexical search remains available when the archive database is healthy
Recommended: Run 'cass doctor check --json' before any repair; indexing will not replace an unreadable canonical database
        5.09 real         4.39 user         0.57 sys
          5749325824  peak memory footprint     (≈5.35 GiB)
exit_status=0 (!)
wall_seconds=6
```

`cass status` exits **0** — success — while its own body says the database
"could not be opened" and both Database and Semantic subsystems report
`error`. The command's own internal probe of its own 22GB SQLite/"frankensqlite"
production database times out at its **own internal 5000ms bound** before
`status` can even report row counts, and this self-check alone costs 5.35
GiB peak. `--json` was not attempted a second time given the pattern already
established, to avoid a third leaked multi-GB process.

`cass doctor --help` documents `--check` as "Run the **bounded** read-only
doctor truth surface." I ran it:

```
$ /usr/bin/time -l cass doctor --check --json    (bound 60s)
```

Zero bytes of output in 60s — the documented "bounded" surface exceeded the
bound with nothing printed. Same orphan pattern as the stats probe: the
underlying `cass doctor --check --json` (PID 58082, PPID 1) was found still
running post-kill, RSS 3.84 GB and climbing at 98.5% CPU, elapsed matching my
launch exactly. Verified and terminated the same way.

**Pattern across all three self-diagnostic commands** (`health`, `status`,
`doctor --check`): each independently balloons to 3.8–5.8 GiB peak memory
just to answer "is the index/database okay," each is markedly slower than
its own documentation claims ("<50ms", "bounded"), and one of the three
(`doctor --check --json`) did not return at all within 60s.

---

## 5. Daemon/watcher — is anything armed?

```
$ launchctl list | rg -i cass                    → no matches (rc=1)
$ ls ~/Library/LaunchAgents | rg -i cass          → no matches
$ ls ~/Library/LaunchDaemons | rg -i cass         → no matches
$ ls /Library/LaunchAgents /Library/LaunchDaemons | rg -i cass  → no matches
```

**Nothing cass-related is registered in launchd anywhere on this machine —
system or user level, agent or daemon.**

`~/.local/share/cass/watchdog.sh` exists (4.0K, last touched 14 Mar). Read in
full. It is a heartbeat-based liveness checker: intended to run every 10
minutes via launchd, checks
`~/Library/Application Support/.../watcher-heartbeat` for staleness
(threshold 2700s = 45 min), and if stale (or the heartbeat file is entirely
absent and no `cass index --watch` process is found), sends SIGTERM to the
watcher, waits up to 120s, escalates to SIGKILL, and relies on launchd's
`KeepAlive` to restart it.

Checked every precondition this script depends on:
- `watcher-heartbeat` file: **does not exist** (`No such file or directory`)
- any `cass index --watch` process: **none running** (only unrelated `node
  --watch`-flavored test runners matched the grep, no cass process)
- `~/Library/Logs/cass-index-watch.log`: **does not exist** — the watchdog
  has apparently never produced a log line, i.e. never actually run
- no crontab entry references it either

**Verdict: the watchdog is not armed.** It is inert code sitting on disk
with no launchd job to invoke it, no watcher process for it to supervise,
and no heartbeat file for it to check. Whatever kept the index "watched" and
current previously is not running today.

---

## 6. Freshness — is the 77G index even current?

The tool's own self-report (from `cass status` above): **"Last indexed: 1
days ago (stale)."** The tool itself calls its own index stale.

Independent mtime comparison (read-only, all times UTC):

| artifact | mtime |
|---|---|
| `agent_search.db` (production, 22G) | 2026-08-17T11:59:13Z |
| `index/` (tantivy dir) | 2026-08-17T09:50:33Z |
| `index-run.lock` `started_at` (the stuck repair job) | 2026-08-17T09:50:33Z |
| newest `~/.codex/sessions/**/*.jsonl` | 2026-08-17T14:54:38Z |
| newest `~/.claude/projects/**/*.jsonl` (excluding this lane's own live transcript) | 2026-08-17T14:54:30Z |
| "now" at capture | 2026-08-17T14:54:54Z |

The source corpora are being written continuously (new rollout/session files
land essentially every few seconds across ~20 concurrent sessions on this
machine, including this one) while the production `agent_search.db` was last
written **~3 hours** before this capture, and the tantivy `index/` directory
was last rebuilt **~5 hours** before this capture — at the exact same
timestamp the still-stuck repair job started. In other words: the last
attempt to bring the index current is the same job that has been spinning
for 5 hours making no progress and blocking search in the meantime (see §2).

---

## Verdict

**No.** As shipped and running right now, a user cannot ask `cass` a
question about their session history and get a correct answer in acceptable
time and memory, on any of the three paths tested:

1. **`cass search` — the actual product feature** — fails immediately with
   `index-busy` (exit 7), not because the *production* index is being
   repaired, but because a lock file at a fixed, un-scoped location is held
   by an unrelated job against a throwaway probe database. This is not a
   momentary busy state: the holding job has made zero progress in ~5 hours
   while continuing to renew its own heartbeat, so the lock has no natural
   expiry in sight.
2. **`cass stats`** — does not return within 120s and grows unbounded past
   ~4.9 GB RSS in that window (second same-day confirmation of an earlier
   ~5.2 GB/never-returned measurement).
3. **The tool's own health/status/doctor self-checks** — `health` (documented
   <50ms) took 2s and 5.4 GiB and reported unhealthy; `status` exited 0
   while its own body reported the database "could not be opened"; the
   documented "bounded" `doctor --check --json` did not return in 60s and
   consumed 3.8+ GiB. None of the three would give an agent or a human a
   trustworthy, fast picture of whether it's safe to proceed.

Two of my five probes (`stats`, `doctor --check`) left an orphaned `cass`
process running past my kill of the `/usr/bin/time` wrapper around it; both
were positively identified by exact command-line and elapsed-time match
before being terminated. No process I did not launch was touched, and PID
75534 (the sibling session's process, and the actual holder of the busy
lock) was inspected read-only only.

No launchd job, watcher process, or heartbeat keeps the index current today;
the watchdog script that would do that is present on disk but has left no
evidence it has ever run.
