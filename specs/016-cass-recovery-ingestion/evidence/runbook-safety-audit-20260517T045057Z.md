---
title: "Runbook safety audit 2026-05-17T04:50:57Z"
date: 2026-05-17T04:50:57Z
bead: coding_agent_session_search-1vxuf
status: read-only
---

# Runbook Safety Audit

## Purpose

Audit the approval-gated live promotion packet for forbidden destructive
patterns before any live mutation is approved.

No live data, binary, launchd service, session root, git commit, or remote branch
was mutated.

## Files Inspected

```text
specs/016-cass-recovery-ingestion/evidence/live-promotion-runbook.md
specs/016-cass-recovery-ingestion/evidence/operator-approval-packet.md
```

## Forbidden-Pattern Scan

Command:

```text
rg -n '\b(rm|unlink|rmdir|git reset|git clean|git checkout --|git restore|force-push|push --force|launchctl bootout|killall|pkill|dd|truncate|shred|wipe|srm)\b' specs/016-cass-recovery-ingestion/evidence/live-promotion-runbook.md specs/016-cass-recovery-ingestion/evidence/operator-approval-packet.md
```

Matches:

```text
specs/016-cass-recovery-ingestion/evidence/operator-approval-packet.md:90:- No `git reset --hard`, `git clean`, force-push, or history rewrite.
specs/016-cass-recovery-ingestion/evidence/live-promotion-runbook.md:292:# Preserve current live artifacts. No rm.
specs/016-cass-recovery-ingestion/evidence/live-promotion-runbook.md:342:launchctl bootout "gui/$(id -u)/com.cass.index-watch" 2>/tmp/cass-index-watch-bootout-"$TS".err || true
```

Classification:

| Match | Classification | Reason |
| --- | --- | --- |
| `git reset --hard`, `git clean`, force-push | Prohibition text only | These are explicitly listed as not authorized in the approval packet. |
| `No rm` | Safety comment | The runbook explicitly says not to remove files. |
| `launchctl bootout` | Approval-gated service mutation | Present only in the restore shape after an approved failed launch attempt. It stops `com.cass.index-watch`; it does not delete files. It still requires the same explicit approval boundary as the live promotion. |

No active `rm`, `unlink`, `rmdir`, `git reset`, `git clean`, `git checkout --`,
`git restore`, force-push, `killall`, `pkill`, `dd`, `truncate`, `shred`,
`wipe`, or `srm` command was found in the approval-gated command blocks.

## Preservation Shape

The live promotion uses timestamped moves for existing live artifacts:

```text
agent_search.db -> agent_search.db.PRE-SPEC016-$TS
agent_search.db-shm -> agent_search.db-shm.PRE-SPEC016-$TS
agent_search.db-wal -> agent_search.db-wal.PRE-SPEC016-$TS
index -> index.PRE-SPEC016-$TS
watch_state.json -> watch_state.json.PRE-SPEC016-$TS
```

It then copies verified shadow artifacts into place:

```text
cp -p "$SHADOW/agent_search.db" "$LIVE_DIR/agent_search.db"
cp -a "$SHADOW/index" "$LIVE_DIR/index"
```

If verification fails, the restore shape preserves failed promoted artifacts
with `FAILED-SPEC016-$TS` suffixes before moving `PRE-SPEC016-$TS` artifacts
back into place.

## Restore Sidecar Collision Check

Follow-up inspection found one edge case in the written restore shape: a failed
promoted DB open could create `agent_search.db-shm` or `agent_search.db-wal`
before rollback. If those files remained in place, restoring
`agent_search.db-shm.PRE-SPEC016-$TS` or `agent_search.db-wal.PRE-SPEC016-$TS`
could collide.

The runbook was tightened at 2026-05-17T04:52:33Z so the approval-gated restore
block now preserves failed promoted sidecars first:

```text
agent_search.db-shm -> agent_search.db-shm.FAILED-SPEC016-$TS
agent_search.db-wal -> agent_search.db-wal.FAILED-SPEC016-$TS
```

It also checks that the live sidecar paths are clear before moving the
`PRE-SPEC016` sidecars back into place. This preserves the no-delete restore
contract while avoiding restore-time collisions.

## Restore Missing-Failed-Artifact Guard

Follow-up inspection found another restore edge case: if the approved promotion
failed after moving the old live DB/index aside but before publishing every
replacement, the restore block could stop while trying to preserve a failed
artifact that did not exist yet.

The runbook was tightened at 2026-05-17T05:24:27Z:

```text
for artifact in agent_search.db agent_search.db-shm agent_search.db-wal index watch_state.json; do
  if [ -e "$LIVE_DIR/$artifact" ]; then
    mv "$LIVE_DIR/$artifact" "$LIVE_DIR/$artifact.FAILED-SPEC016-$TS"
  else
    echo "no failed $artifact present before restoring PRE-SPEC016 artifact" >&2
  fi
done
```

This preserves failed artifacts when present but still lets the restore path
recover the known previous `PRE-SPEC016` DB/index/watch state when a failed
replacement was never created.

## Approval Boundary

This audit does not authorize execution. The following remain live/operator-owned
mutations and still require the exact approval phrase:

```text
I approve live CASS promotion, frankensqlite durable fix, and branch/commit resolution.
```

Approval-gated mutations include:

- moving live DB/index/watch-state artifacts aside;
- copying shadow DB/index into the live CASS data dir;
- replacing the installed `cass.real` binary;
- bootstrapping or booting out `com.cass.index-watch`;
- writing the synthetic Codex marker under `~/.codex/sessions`;
- committing/pushing frankensqlite and CASS branch/dependency resolution.

## Reference Integrity Follow-Up

After the restore-sidecar update, a repo-relative reference scan over the active
T008 surfaces checked eight files and found 86 unique repo-relative references.
It reported five missing references:

```text
specs/016-cass-recovery-ingestion/code-verify.md
specs/016-cass-recovery-ingestion/evidence/live-promotion-runbook.md:342:launchctl
specs/016-cass-recovery-ingestion/evidence/operator-approval-packet.md:90:-
specs/019
tests/build
```

Classification:

| Reference | Classification | Reason |
| --- | --- | --- |
| `specs/016-cass-recovery-ingestion/code-verify.md` | Expected absent | `$code-verify` is blocked until live promotion/watcher proof exists. |
| `specs/016-cass-recovery-ingestion/evidence/live-promotion-runbook.md:342:launchctl` | Parser false positive | The scanner over-captured a path plus line/command evidence string from this audit. |
| `specs/016-cass-recovery-ingestion/evidence/operator-approval-packet.md:90:-` | Parser false positive | The scanner over-captured a path plus line/punctuation evidence string from this audit. |
| `specs/019` | Prose shorthand | Concrete paths exist under `specs/019-ubs-warning-policy-closeout/`. |
| `tests/build` | Prose shorthand | The text says `focused tests/build`; it is not meant as a file path. |

No missing first-read recovery evidence artifact was found.

## Watcher-Proof Overwrite Guard

Follow-up inspection of the approval-gated watcher proof found that the
synthetic Codex marker used a plain `cat > "$SYNTH_FILE"` redirect. The marker
name includes a timestamp and should be unique, but the proof should still fail
closed rather than overwriting an existing session file if a command is rerun
with the same marker.

The runbook was tightened at 2026-05-17T04:55:37Z:

```text
test ! -e "$SYNTH_FILE"
set -o noclobber
cat > "$SYNTH_FILE" <<EOF
...
EOF
set +o noclobber
```

This keeps the approved watcher proof as a new synthetic session artifact and
does not authorize deleting or overwriting any existing session artifact.

## Pre-Watcher Health Threshold Check

Follow-up inspection found that the live-promotion section used:

```text
cass health --json --stale-threshold 1800
```

immediately after copying the verified shadow archive and before starting
`com.cass.index-watch`. That would fail for a known, expected reason: the shadow
archive is not watched and is older than 30 minutes.

Read-only proof against the verified shadow archive on 2026-05-17T04:57:15Z:

```text
release cass health --stale-threshold 1800 --data-dir "$SHADOW": exit 1, unhealthy, lexical index is older than the stale threshold
release cass health --stale-threshold 86400 --data-dir "$SHADOW": exit 0, healthy, checkpoint.completed=true
```

The runbook now uses `--stale-threshold 86400` for the pre-watcher archive
readiness check. Final live freshness still depends on starting
`com.cass.index-watch` and proving a new synthetic session marker becomes
searchable.

## Pre-Install Binary Consistency Check

Follow-up inspection found that the live archive verification command was still
calling `/Users/dalecarman/.local/bin/cass` before the runtime install section.
That installed binary is not the verified release artifact:

```text
release candidate sha256: a5a139ca503f0f04f2b0baba06e20b16bd7003a7e7d78306cacf94d56eaca9c2
installed cass.real sha256: 47f0692af0fd6484e82e4b69b5512ba44b82de1d0c10d64b5a171b2ed279e691
release cass watchdog run --help: exit 0
installed cass watchdog run --help: exit 2
```

The runbook now sets:

```text
CASS_RELEASE="/tmp/cass-release-target/release/cass"
```

and uses `"$CASS_RELEASE"` for pre-install archive health/search verification.
The launchd watcher proof still uses `/Users/dalecarman/.local/bin/cass`, but
only after the runtime install shape copies the verified release candidate into
`cass.real`.

## Watcher-Proof Timeout Guard

Follow-up inspection found that the 120-second watcher marker loop could finish
without finding the marker and still let the shell block continue, because there
was no assertion after the loop.

The runbook was tightened at 2026-05-17T05:01:29Z:

```text
found=0
...
if search finds marker; then
  found=1
  break
fi
...
if [ "$found" -ne 1 ]; then
  echo "watcher marker not searchable within 120s: $MARKER" >&2
  echo "watcher proof source: $SYNTH_FILE" >&2
  exit 1
fi
```

This makes the approval-gated watcher proof fail loudly unless the marker is
actually searchable.

## Post-Watcher Health And Log Guard

Follow-up inspection found that a synthetic marker search hit was necessary but
not enough as final live watcher proof. The runbook was tightened at
2026-05-17T05:28:13Z:

```text
/Users/dalecarman/.local/bin/cass health --json --stale-threshold 1800 --data-dir "$LIVE_DIR" > /tmp/cass-live-post-watcher-health-"$TS".json
jq -e '.healthy == true and .state.index.status == "ready" and .state.index.checkpoint.completed == true and .state.pending.watch_active == true' /tmp/cass-live-post-watcher-health-"$TS".json
tail -n 200 "$HOME/Library/Logs/cass-index-watch.log" > /tmp/cass-index-watch-tail-"$TS".log
rg "$MARKER|streaming_ingest|watch_scan|streaming_scan_complete" /tmp/cass-index-watch-tail-"$TS".log
```

This requires final health/freshness, an active watcher signal, and watcher-log
evidence after the marker becomes searchable.

## Explicit Live Data-Dir Guard

Follow-up inspection found that archive health/search proof relied on default
data-dir resolution. The defaults should resolve to the live archive, but
approval-time proof is clearer and safer when it names the archive being proved.

The runbook was tightened at 2026-05-17T05:02:54Z:

```text
"$CASS_RELEASE" health --json --stale-threshold 86400 --data-dir "$LIVE_DIR"
"$CASS_RELEASE" search ... --data-dir "$LIVE_DIR"
/Users/dalecarman/.local/bin/cass search "$MARKER" ... --data-dir "$LIVE_DIR"
```

The watcher-proof block now defines the same `LIVE_DIR` before bootstrap/search.
This ties pre-install archive verification and post-install marker search to the
just-promoted live CASS archive.

## Watcher-Process Self-Match Guard

Follow-up inspection found that the watcher process proof used:

```text
ps -axo ... | rg 'cass index --watch|cass.*--watch'
```

That can match the `rg` probe command itself. The runbook was tightened at
2026-05-17T05:04:45Z:

```text
WATCH_PROCESSES="$(ps -axo pid,state,rss,%cpu,etime,command | awk '$0 ~ /cass/ && $0 ~ /index/ && $0 ~ /--watch/ && $0 !~ /awk/ && $0 !~ /zsh -/ {print}')"
printf '%s\n' "$WATCH_PROCESSES"
test -n "$WATCH_PROCESSES"
```

This requires an actual non-probe `cass index --watch` process before the
approved watcher proof can continue.

## Watcher-Process Bounded Wait

Follow-up inspection found that the watcher proof checked for a process
immediately after `launchctl bootstrap`. That could false-fail if launchd took a
moment to spawn the process. The runbook was tightened at
2026-05-17T05:22:40Z:

```text
WATCH_PROCESSES=""
watch_process_deadline=$((SECONDS + 30))
while (( SECONDS < watch_process_deadline )); do
  WATCH_PROCESSES="$(ps -axo pid,state,rss,%cpu,etime,command | awk '$0 ~ /cass/ && $0 ~ /index/ && $0 ~ /--watch/ && $0 !~ /awk/ && $0 !~ /zsh -/ && $0 !~ /ps -axo/ {print}')"
  if [ -n "$WATCH_PROCESSES" ]; then
    break
  fi
  sleep 2
done
printf '%s\n' "$WATCH_PROCESSES"
if [ -z "$WATCH_PROCESSES" ]; then
  launchctl print "gui/$(id -u)/com.cass.index-watch" >&2 || true
  echo "cass index --watch process did not appear within 30s" >&2
  exit 1
fi
```

This keeps the proof fail-closed while allowing a short launchd spawn window.

## Pre-Promotion Live-Process Guard

Follow-up inspection found that the approval-gated promotion block previously
printed process state with an `rg ... || true` probe and then continued. That
could self-match and, more importantly, it did not stop the promotion if a live
CASS process had the DB or index open.

The runbook was tightened at 2026-05-17T05:18:12Z:

```text
if launchctl print "gui/$(id -u)/com.cass.index-watch" >/tmp/cass-index-watch-print-"$TS".txt 2>&1; then
  cat /tmp/cass-index-watch-print-"$TS".txt >&2
  echo "com.cass.index-watch is loaded; stop and re-align before promotion" >&2
  exit 1
fi
ACTIVE_CASS_PROCESSES="$(ps -axo pid,state,rss,%cpu,etime,command | awk '$0 ~ /cass/ && ($0 ~ /index/ || $0 ~ /search/ || $0 ~ /doctor/ || $0 ~ /health/) && $0 !~ /awk/ && $0 !~ /zsh -/ && $0 !~ /ps -axo/ {print}')"
printf '%s\n' "$ACTIVE_CASS_PROCESSES"
if [ -n "$ACTIVE_CASS_PROCESSES" ]; then
  echo "active CASS process detected; stop and re-align before promotion" >&2
  exit 1
fi
```

This makes the approved promotion path fail closed before DB/index moves if a
live watcher or CASS process is present.

## Runtime Install And Binary Restore Guard

Follow-up inspection found a restore-edge gap: if the approved runtime install
failed after moving `cass.real` to `cass.real.PRE-SPEC016-$TS` but before
copying the replacement, the restore block would try to preserve a failed
`cass.real` path that might not exist, then stop before restoring the
pre-spec016 binary.

The runbook was tightened at 2026-05-17T05:21:01Z:

```text
CASS_RELEASE="/tmp/cass-release-target/release/cass"
test -x "$CASS_RELEASE"
shasum -a 256 "$CASS_RELEASE"
"$CASS_RELEASE" --version
"$CASS_RELEASE" watchdog run --help >/tmp/cass-release-watchdog-run-help-"$TS".txt
test -x "$HOME/.local/bin/cass.real"

mv "$HOME/.local/bin/cass.real" "$HOME/.local/bin/cass.real.PRE-SPEC016-$TS"
cp -p "$CASS_RELEASE" "$HOME/.local/bin/cass.real"
test -x "$HOME/.local/bin/cass.real"
```

The binary restore block now preserves the failed replacement only when it
exists, then restores `cass.real.PRE-SPEC016-$TS` either way.

This keeps the install/restore path reversible without deleting files.

## Pre-Watcher Counts And Canary Guard

Follow-up inspection found that the approval-gated pre-watcher archive proof
checked integrity, health, and one Codex search. That was not enough to cover
the user's stated priority agents plus OpenCode/factory non-regression.

The runbook was tightened at 2026-05-17T05:26:27Z:

```text
EXPECTED_COUNTS:
claude_code|2574
codex|5713
factory|66
opencode|976
pi_agent|2076
messages|1238935

canaries:
pi_agent    ATT21_COL_CFP_SceneMachine_EndCard.psd
claude_code frankensqlite
codex       freelist serializer
opencode    opencode
factory     factory
```

The approved promotion block now compares live SQL counts against that expected
set and requires every canary search under `--data-dir "$LIVE_DIR"` to return
`total_matches > 0` before watcher startup.

## Decision

The runbook passes the no-deletion/no-reset safety audit for the current written
command shape, and the restore shape now preserves failed DB sidecars before
restoring `PRE-SPEC016` sidecars. The watcher proof also now fails closed instead
of clobbering an existing synthetic-session path. The pre-watcher health check
now uses the proven shadow-readiness threshold instead of a false-failing
30-minute freshness threshold, and the pre-install archive verification now uses
the verified release candidate instead of the old installed binary. The watcher
marker loop now exits 1 on timeout instead of falling through. Archive health,
archive search, and watcher marker search now pass `--data-dir "$LIVE_DIR"`
explicitly. The watcher process proof now excludes its own probe and asserts a
non-empty result after a bounded 30-second launchd spawn wait. The pre-promotion
live-process check now fails closed before DB/index moves if a watcher or CASS
process is active. The runtime install block now verifies release and installed
binary executability before/after replacement, and the binary restore block no
longer depends on a failed replacement binary being present. The DB/index
restore block now also tolerates missing failed replacement artifacts while
preserving them when present. The pre-watcher archive proof now checks all
priority/bonus counts and lexical canaries before watcher startup. The final
watcher proof now also requires healthy fresh live status with
`pending.watch_active=true` plus watcher-log evidence. It remains approval-gated
and does not complete the live CASS recovery outcome.
