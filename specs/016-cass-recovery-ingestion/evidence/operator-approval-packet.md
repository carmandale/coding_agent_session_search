---
title: "Operator approval packet: spec 016 live recovery"
date: 2026-05-17T07:00:35Z
bead: coding_agent_session_search-1vxuf
status: approval-required
---

# Operator Approval Packet

## Current Decision

Spec 016 is ready for the next phase, but the next phase is intentionally gated
because it changes the live CASS installation and git/dependency state.

Do not proceed without this exact approval text:

```text
I approve live CASS promotion, frankensqlite durable fix, and branch/commit resolution.
```

## What Is Already Proven

- Shadow archive is healthy and searchable:
  - `pi_agent=2076`
  - `claude_code=2574`
  - `codex=5713`
  - `opencode=976`
  - `factory=66`
  - `messages=1238935`
- Release candidate exists and searches the shadow archive:
  - path: `/tmp/cass-release-target/release/cass`
  - version: `cass 0.4.7`
  - sha256: `a5a139ca503f0f04f2b0baba06e20b16bd7003a7e7d78306cacf94d56eaca9c2`
  - `cass watchdog run --help` exits `0` in the release candidate, while the installed binary still exits `2`
- Synthetic Codex marker format has been proven in scratch full-index and
  `index --watch-once` flows.
- `com.cass.index-watch.plist` exists, lints, and points to
  `/Users/dalecarman/.local/bin/cass index --watch`.
- Approval-gated shell blocks in the runbook parse with `zsh -n`.
- The live SQLite read-only probe shape has been corrected: `sqlite3 -readonly`
  failed against the current live DB with SQLite code `14`; the runbook now
  uses the encoded `mode=ro` URI shape proven in
  `evidence/continuation-audit-20260517T042428Z.md`.
- No-delete restore shape exists in `live-promotion-runbook.md`.
- Runbook safety audit is recorded in
  `evidence/runbook-safety-audit-20260517T045057Z.md`: no active
  delete/reset/clean/force-push commands were found; the only
  `launchctl bootout` is in approval-gated restore handling.
- The pre-watcher archive health check in the runbook now uses the proven
  `--stale-threshold 86400` shadow-readiness threshold; `1800` is a false
  failure before watcher startup because the verified shadow index is older
  than 30 minutes.
- Pre-install archive health/search verification now uses the approval-gated
  release candidate at `/tmp/cass-release-target/release/cass`, not the old
  installed binary. The installed `cass.real` hash still differs and installed
  `cass watchdog run --help` still exits `2`.
- The watcher marker proof now fails loudly if the marker is not searchable
  within 120 seconds; it no longer falls through after timeout.
- The watcher marker proof now requires the search hit/result `source_path` to
  equal the synthetic Codex file created by the proof attempt, so a stale or
  colliding marker hit cannot satisfy watcher proof.
- The watcher proof now saves launchctl output, process proof, marker search
  JSON, post-watcher health JSON, index-watch log tail, marker, and synthetic
  source path under `specs/016-cass-recovery-ingestion/evidence/watcher-proof/`.
- Pre-install archive health/search and post-install watcher marker search now
  pass `--data-dir "$LIVE_DIR"` explicitly, tying proof to the promoted live
  archive.
- The watcher process proof now excludes its own probe process and requires a
  non-empty `cass index --watch` process result.
- The pre-promotion process guard now fails closed before DB/index moves if
  `com.cass.index-watch` is loaded or a non-probe CASS index/search/doctor/health
  process is active.
- The runtime install and restore guards now verify release/installed binary
  executability before and after replacing `cass.real`, and the restore path can
  recover the `PRE-SPEC016` binary even if no failed replacement binary exists.
- The approval-gated install block now captures the rebuilt release hash and the
  post-copy installed `cass.real` hash, then requires exact equality before
  continuing to version/capabilities proof.
- The approval-gated dependency proof now fails before release build if the
  sibling frankensqlite checkout is still dirty or unpushed, or if CASS still
  resolves `fsqlite`/`fsqlite-types` through the local
  `../spec014-frankensqlite-fix` path patch.
- That dependency proof now refreshes the sibling `carmandale` and `origin`
  remotes before checking remote containment, and its CASS-side grep catches
  the frankensqlite `[patch]` header itself.
- The watcher proof waits up to 30 seconds for a real non-probe
  `cass index --watch` process after launchd bootstrap, then fails with
  launchctl state if the process never appears.
- The restore guard now preserves failed DB/index/watch artifacts only when
  present, so a partial failed promotion can still restore `PRE-SPEC016`
  backups even if some replacement artifacts were never created.
- The restore guard now fails before bootout or failed-artifact preservation if
  mandatory `PRE-SPEC016` DB/index backups are missing or if the
  `FAILED-SPEC016` suffix has already been used.
- Before watcher startup, the promoted archive proof now requires the exact
  verified shadow counts for Pi Agent, Claude Code, Codex, OpenCode, factory,
  and messages, plus successful lexical canaries for all five agent families
  under `--data-dir "$LIVE_DIR"`.
- Before moving live DB/index artifacts, the promotion block now calculates the
  verified shadow DB/index copy footprint and current free space, records both,
  and fails closed if available space is not greater than the copy footprint.
- Before any live DB/index move, the promotion block now verifies the required
  live DB/index, shadow DB/index, writable live destination, and executable
  release candidate, records the release hash, and proves the release
  `watchdog run` command surface.
- Before preserving live artifacts, the promotion and install blocks now verify
  their `PRE-SPEC016` backup destinations are unused, preventing a reused
  attempt token from overwriting previous-live backups.
- The runtime install block now compares the rebuilt release hash against the
  release hash used for pre-promotion archive verification when that
  pre-promotion hash exists for the same approved attempt.
- Before any live DB/index move, the promotion block now also rechecks shadow DB
  integrity, exact priority/bonus/message counts, and release-candidate health
  against the shadow archive so approval-time promotion is not relying on stale
  shadow proof.
- Before bootstrapping `com.cass.index-watch`, the watcher proof now requires
  installed `cass.real` to be executable, byte-for-byte equal to the tested
  release candidate, and able to expose the installed `watchdog run` command
  surface.
- After the marker becomes searchable, the watcher proof now also requires
  installed live health to be healthy/ready/checkpoint-complete with
  `pending.watch_active=true` at the 1800-second threshold and index-watch log
  evidence from the same run.
- The approval-gated runbook now creates one shared `SPEC016_TS` token for the
  whole approved attempt. Promotion, runtime install, watcher proof, and restore
  all require that token so backup and failed-artifact suffixes stay aligned.
- The approval-gated runbook now has an explicit upstream/branch proof block:
  after live proof, it fetches upstream/origin, captures branch/ahead-behind/
  merge-tree evidence, and fails closed before commit/push if `upstream/main`
  is not an ancestor of the final CASS `HEAD`.
- Latest read-only continuation audit is recorded at
  `evidence/continuation-audit-20260517T070035Z.md`: upstream remains
  `1f20bd576f2e77a5197783c637fcc771ab9e1867`, ahead/behind is still `19/24`,
  live CASS remains malformed and under-indexed, shadow remains healthy,
  `com.cass.index-watch` remains absent, health-watchdog has reached
  `runs=364` with last exit code `2`, and no live CASS mutation occurred.
- GoalBuddy board mechanics are valid: `T005` remains blocked on approval and
  `T008` is the active PM maintenance task; the GoalBuddy checker and absolute
  `goalbuddy prompt` surface pass. The board API exposes active task as
  `.goal.activeTask=T008`.

## Why Approval Is Required

The remaining actions touch live/operator-owned surfaces:

- live data dir:
  `/Users/dalecarman/Library/Application Support/com.coding-agent-search.coding-agent-search`
- installed binary:
  `/Users/dalecarman/.local/bin/cass.real`
- launchd service:
  `com.cass.index-watch`
- real watched session root:
  `~/.codex/sessions/YYYY/MM/DD/`
- sibling dependency checkout:
  `/Users/dalecarman/dev/spec014-frankensqlite-fix`
- git branch/finalization state:
  current checkout is `dac/main`, not `main`

## What Approval Authorizes

After the exact approval phrase, proceed with the documented runbook in this
order:

1. Initialize one `SPEC016_TS` approval-attempt token and reuse it for every
   dependency, live promotion, install, watcher, restore, and branch/upstream
   block.
2. Make the frankensqlite fix durable by committing/pushing the focused sibling
   pager/WAL fix, then replace CASS's local `../spec014-frankensqlite-fix`
   patch with a durable revision or agreed fork pin.
3. Run the durable dependency proof block so the sibling checkout is clean and
   pushed and CASS no longer resolves frankensqlite through a local path patch.
4. Rebuild and reverify CASS from that durable dependency.
5. Preserve the current live DB/index/binary with timestamped `PRE-SPEC016`
   names.
6. Copy the verified shadow DB/index into the live data dir.
7. Install the verified release binary by preserving the old `cass.real` first,
   then copying the tested release artifact into place.
8. Bootstrap `com.cass.index-watch`.
9. Create a harmless synthetic Codex marker under the real watched
   `~/.codex/sessions/YYYY/MM/DD/` tree and prove it becomes searchable within
   120 seconds.
10. Re-run the upstream/branch proof and stop before commit/push if upstream is
   not incorporated or if the `dac/main` branch target is not explicitly
   resolved.
11. Refresh the completion audit, then run `$code-verify` and `$finalize`.

## What Approval Does Not Authorize

- No file deletion.
- No `git reset --hard`, `git clean`, force-push, or history rewrite.
- No semantic model download.
- No public repository or visibility change.
- No broad health-watchdog rewrite; `coding_agent_session_search-2gif2` tracks
  that separately unless it interferes with `com.cass.index-watch`.
- No unrelated dirty/untracked files staged.

Before final staging or push, list exact staged paths and the resolved branch
target in the receipt.

## Current Blockers Without Approval

- Live DB still fails `PRAGMA quick_check` with freelist errors.
- Live DB read-only checks must use the encoded SQLite `mode=ro` URI shape
  from the runbook; the plain `sqlite3 -readonly "$LIVE_DIR/agent_search.db"`
  path-open command fails against the current live DB.
- Live `pi_agent=1077`, while verified shadow has `2076`.
- `com.cass.index-watch` is absent.
- Installed CASS health is still unhealthy/stale with `watch_active=false`;
  health-watchdog is loaded but not running, now at `runs=364` and last exit
  code `2`.
- Installed `cass watchdog run --help` still exits `2`; the approval-gated
  release candidate exits `0`.
- CASS still resolves frankensqlite through the dirty local sibling patch.
- Upstream remains unresolved:
  - `HEAD=b807ef175dcdeeb48b912a22913fbcd68fb86cb8`
  - `upstream/main=1f20bd576f2e77a5197783c637fcc771ab9e1867`
  - ahead/behind: `19/24`
- `$code-verify`, `$finalize`, bead closure, commit, and push are not done.
- UBS warning-policy closeout remains open as
  `coding_agent_session_search-2v7tv`; local criticals are clear, but the
  CI-shaped `ubs --ci --fail-on-warning` command still exits nonzero on warning
  inventory.

## Primary Detailed Runbook

Use this packet as the short approval surface. Use the detailed command source
for execution:

`specs/016-cass-recovery-ingestion/evidence/live-promotion-runbook.md`
