---
title: "Live promotion runbook: spec 016"
date: 2026-05-17T06:18:25Z
status: approval-required
---

# Live Promotion Runbook

This runbook was prepared from read-only inspection. It has not been executed.

Required approval phrase before running any live-mutating command:

```text
I approve live CASS promotion, frankensqlite durable fix, and branch/commit resolution.
```

## Approval-Gated Attempt Token

Immediately after the exact approval phrase and before any dependency,
promotion, install, watcher, restore, or branch/upstream block, create one
attempt token and reuse it for every approval-gated command block. Do not let
individual snippets mint their own timestamp; restore depends on the same
`PRE-SPEC016-$TS` suffixes.

```bash
set -euo pipefail

export SPEC016_TS="${SPEC016_TS:-$(date -u +%Y%m%dT%H%M%SZ)}"
printf '%s\n' "$SPEC016_TS" | tee /tmp/spec016-approved-attempt-ts.txt

# If a later block runs in a fresh shell, first restore the same token with:
# export SPEC016_TS="$(cat /tmp/spec016-approved-attempt-ts.txt)"
```

## Read-Only Facts

Live data dir:

```text
/Users/dalecarman/Library/Application Support/com.coding-agent-search.coding-agent-search
size: 165G
live DB: agent_search.db, 7.1G
live quick_check: many Freelist leaf count errors
live rows: pi_agent=1077, claude_code=2574, codex=5712, opencode=976, factory=66, messages=1055517
```

Verified shadow data dir:

```text
/Users/dalecarman/Library/Application Support/com.coding-agent-search.coding-agent-search-spec016-shadow-20260516T2025Z
size: 13G
shadow DB: agent_search.db, 7.9G
shadow integrity_check: ok
shadow rows: pi_agent=2076, claude_code=2574, codex=5713, opencode=976, factory=66, messages=1238935
```

Installed runtime:

```text
/Users/dalecarman/.local/bin/cass -> /Users/dalecarman/.local/bin/cass.real
/Users/dalecarman/.local/bin/cass.real size: 52M
installed hash: 47f0692af0fd6484e82e4b69b5512ba44b82de1d0c10d64b5a171b2ed279e691
installed version: cass 0.4.7
```

Tested local debug runtime:

```text
/tmp/cass-check-target/debug/cass size: 286M
hash: 2b560419f0b08696f2d2dbb8fe6f7f3033163f6e90a04d03688e47b95192c695
```

Approval-gated release candidate built without installing:

```text
command: env CARGO_TARGET_DIR=/tmp/cass-release-target $HOME/.cargo/bin/cargo build --release --bin cass
result: pass
duration: 5m 46s
path: /tmp/cass-release-target/release/cass
size: 52M
sha256: 423e2e4c2920ec74a38a5cb4af1f00de362a4a82e493d342b4891179f4955ada
```

Release candidate shadow proof on 2026-05-16T23:13:22Z:

```text
health --stale-threshold 86400: healthy=true, index.status=ready, checkpoint.completed=true
pi_agent lexical canary: total_matches=30, elapsed_ms=48
claude_code lexical canary: total_matches=37, elapsed_ms=35
codex lexical canary: total_matches=10, elapsed_ms=34
opencode lexical canary: total_matches=2484, elapsed_ms=31
factory lexical canary: total_matches=21, elapsed_ms=27
```

Note: the same shadow health command with `--stale-threshold 1800` reports stale because no shadow watcher is running and the shadow index age exceeded 30 minutes. That does not invalidate the lexical search proof.

Release candidate refresh after final-checkpoint fix on 2026-05-17T00:18:20Z:

```text
command: env CARGO_TARGET_DIR=/tmp/cass-release-target $HOME/.cargo/bin/cargo build --release --bin cass
result: pass
path: /tmp/cass-release-target/release/cass
version: cass 0.4.7
sha256: 077674c65899936a79885d24cf141e1ac05632e5bd201958a1a6a992fda20594
health --stale-threshold 86400: healthy=true, state.index.status=ready, checkpoint.completed=true, checkpoint.db_matches=true
pi_agent lexical canary: total_matches=30, elapsed_ms=44
claude_code lexical canary: total_matches=37, elapsed_ms=25
codex lexical canary: total_matches=10, elapsed_ms=56
opencode lexical canary: total_matches=2484, elapsed_ms=57
factory lexical canary: total_matches=21, elapsed_ms=41
```

Latest release candidate refresh after changed-file UBS critical cleanup on 2026-05-17T01:18:03Z:

```text
command: env CARGO_TARGET_DIR=/tmp/cass-release-target $HOME/.cargo/bin/cargo build --release --bin cass
result: pass
path: /tmp/cass-release-target/release/cass
version: cass 0.4.7
sha256: 423e2e4c2920ec74a38a5cb4af1f00de362a4a82e493d342b4891179f4955ada
health --stale-threshold 86400: healthy=true, state.index.status=ready, checkpoint.completed=true, checkpoint.db_matches=true
pi_agent lexical canary: total_matches=30, elapsed_ms=24, search_ms=1
claude_code lexical canary: total_matches=37, elapsed_ms=23, search_ms=0
codex lexical canary: total_matches=10, elapsed_ms=23, search_ms=0
opencode lexical canary: total_matches=2484, elapsed_ms=23, search_ms=0
factory lexical canary: total_matches=21, elapsed_ms=23, search_ms=0
```

Latest release candidate refresh after spec018 watchdog command-surface repair on 2026-05-17T04:15:33Z:

```text
command: env CARGO_TARGET_DIR=/tmp/cass-release-target $HOME/.cargo/bin/cargo build --release --bin cass
result: pass
path: /tmp/cass-release-target/release/cass
version: cass 0.4.7
sha256: a5a139ca503f0f04f2b0baba06e20b16bd7003a7e7d78306cacf94d56eaca9c2
release cass watchdog run --help: exit 0
installed cass watchdog run --help: exit 2, Could not parse arguments
health --stale-threshold 86400: healthy=true, state.index.status=ready, checkpoint.completed=true, checkpoint.db_matches=true
pi_agent lexical canary: total_matches=30
claude_code lexical canary: total_matches=37
codex lexical canary: total_matches=10
opencode lexical canary: total_matches=2484
factory lexical canary: total_matches=21
```

LaunchAgents:

```text
com.cass.index-watch plist exists but service is not loaded.
ProgramArguments: /Users/dalecarman/.local/bin/cass index --watch

com.cass.health-watchdog is loaded but broken.
ProgramArguments: /Users/dalecarman/.local/bin/cass watchdog run
last exit code observed: 2
latest observed runs: 348

com.cass.sync-to-mini is loaded and unrelated to required index-watch proof.
```

Watcher plist/runtime readiness refresh on 2026-05-16T23:26:49Z:

```text
plist: /Users/dalecarman/Library/LaunchAgents/com.cass.index-watch.plist
mode: -rw-r--r--
Label: com.cass.index-watch
RunAtLoad: true
KeepAlive: true
ProgramArguments: /Users/dalecarman/.local/bin/cass index --watch
StandardOutPath/StandardErrorPath: /Users/dalecarman/Library/Logs/cass-index-watch.log
PATH: /Users/dalecarman/.local/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin
```

Both binaries expose the required command surface:

```text
/Users/dalecarman/.local/bin/cass index --help: includes --watch, --watch-once, --watch-interval
/tmp/cass-release-target/release/cass index --help: includes --watch, --watch-once, --watch-interval
installed cass.real sha256: 47f0692af0fd6484e82e4b69b5512ba44b82de1d0c10d64b5a171b2ed279e691
release candidate sha256: a5a139ca503f0f04f2b0baba06e20b16bd7003a7e7d78306cacf94d56eaca9c2
```

Pre-approval process refresh on 2026-05-16T23:20:47Z:

```text
ps -axo pid,state,rss,%cpu,etime,command | rg 'cass index|cass search|cass health|cass doctor|cass watchdog|/tmp/cass-check-target|/tmp/cass-release-target|target/(debug|release)/cass'
```

Result: no active cass index/search/health/doctor/watchdog or local debug/release cass worker matched beyond the `ps`/`rg` probe itself.

## Durable Dependency Plan

Current local recovery build uses:

```toml
[patch."https://github.com/Dicklesworthstone/frankensqlite"]
fsqlite = { path = "../spec014-frankensqlite-fix/crates/fsqlite" }
fsqlite-types = { path = "../spec014-frankensqlite-fix/crates/fsqlite-types" }
```

Before deploying, make the frankensqlite fix durable:

1. In `/Users/dalecarman/dev/spec014-frankensqlite-fix`, verify the two changed files:
   - `crates/fsqlite-pager/src/pager.rs`
   - `crates/fsqlite-wal/src/wal.rs`
2. Run the focused frankensqlite tests already used for the recovery:
   - `$HOME/.cargo/bin/cargo fmt -p fsqlite-pager -p fsqlite-wal`
   - `$HOME/.cargo/bin/cargo test -p fsqlite-wal test_append_recovers_after_external_zero_byte_truncate`
   - `$HOME/.cargo/bin/cargo test -p fsqlite-pager freelist`
3. Commit and push the sibling fix only after branch authorization is explicit.
4. Replace the local CASS patch with a durable git revision or agreed fork pin.
5. Rebuild and re-run CASS verification after the durable pin.

Current local proof on 2026-05-16T22:59:41Z:

```text
cargo fmt --check: pass
fsqlite-wal test_append_recovers_after_external_zero_byte_truncate: pass
fsqlite-pager freelist tests: 23 passed, 0 failed
```

Durable dependency state refresh on 2026-05-16T23:23:03Z:

```text
/Users/dalecarman/dev/spec014-frankensqlite-fix
branch: fix/fts5-vtab-snapshot-via-delta-journal
tracking: carmandale/fix/fts5-vtab-snapshot-via-delta-journal
HEAD: f298dfa fix(fts5): replace eager snapshot clone with O(1) reverse-delta journal
remotes: carmandale=https://github.com/carmandale/frankensqlite.git, origin=https://github.com/Dicklesworthstone/frankensqlite
dirty files: crates/fsqlite-pager/src/pager.rs, crates/fsqlite-wal/src/wal.rs
diff stat: 2 files changed, 94 insertions(+), 11 deletions(-)
```

CASS dependency state:

```text
Cargo.toml still pins fsqlite/fsqlite-types to Dicklesworthstone/frankensqlite rev eba969ec45d102071b90519d3b819ddbcecf3d61.
The local recovery build enables [patch."https://github.com/Dicklesworthstone/frankensqlite"] to use ../spec014-frankensqlite-fix/crates/fsqlite and ../spec014-frankensqlite-fix/crates/fsqlite-types.
Cargo.lock therefore removes the git source from the 0.1.3 fsqlite package entries while the patch is active.
```

Durable closeout must replace that local path patch with a committed and pushed frankensqlite revision or agreed fork pin, then rebuild/reverify CASS from that durable reference.

## Approval-Gated Durable Dependency Proof Shape

Run this only after the exact approval phrase and after the sibling
frankensqlite fix has been committed and pushed. Do not build or install the
release CASS binary while the local path patch is still active.

```bash
set -euo pipefail

: "${SPEC016_TS:?export SPEC016_TS from the approved attempt before dependency proof}"
TS="$SPEC016_TS"
FRANKEN_DIR="/Users/dalecarman/dev/spec014-frankensqlite-fix"

(
  cd "$FRANKEN_DIR"
  git fetch carmandale
  git fetch origin
  git status --short --branch > /tmp/spec016-frankensqlite-status-"$TS".txt
  git rev-parse --abbrev-ref HEAD > /tmp/spec016-frankensqlite-branch-"$TS".txt
  git rev-parse HEAD > /tmp/spec016-frankensqlite-head-"$TS".txt
  git remote -v > /tmp/spec016-frankensqlite-remotes-"$TS".txt
  $HOME/.cargo/bin/cargo fmt -p fsqlite-pager -p fsqlite-wal --check
  $HOME/.cargo/bin/cargo test -p fsqlite-wal test_append_recovers_after_external_zero_byte_truncate
  $HOME/.cargo/bin/cargo test -p fsqlite-pager freelist
  if [ -n "$(git status --porcelain)" ]; then
    git status --short >&2
    echo "frankensqlite durable fix checkout is still dirty; stop before CASS release build" >&2
    exit 1
  fi
  git branch -r --contains "$(git rev-parse HEAD)" > /tmp/spec016-frankensqlite-remote-contains-"$TS".txt
  if ! rg 'carmandale/|origin/' /tmp/spec016-frankensqlite-remote-contains-"$TS".txt >/dev/null; then
    cat /tmp/spec016-frankensqlite-remote-contains-"$TS".txt >&2
    echo "frankensqlite durable fix HEAD is not present on a remote branch; stop before CASS release build" >&2
    exit 1
  fi
)

if rg -n 'spec014-frankensqlite-fix|path = "\.\./spec014-frankensqlite-fix"|\[patch\."https://github\.com/Dicklesworthstone/frankensqlite"\]' Cargo.toml Cargo.lock; then
  echo "CASS still references the local spec014 frankensqlite path patch; replace it with a durable revision before release build" >&2
  exit 1
fi

$HOME/.cargo/bin/cargo metadata --format-version=1 > /tmp/spec016-cargo-metadata-"$TS".json
jq -r '.packages[] | select(.name == "fsqlite" or .name == "fsqlite-types") | [.name, .version, (.source // "path"), .manifest_path] | @tsv' /tmp/spec016-cargo-metadata-"$TS".json > /tmp/spec016-fsqlite-sources-"$TS".tsv
cat /tmp/spec016-fsqlite-sources-"$TS".tsv
if jq -e '.packages[] | select((.name == "fsqlite" or .name == "fsqlite-types") and (((.source // "") == "") or (.manifest_path | contains("spec014-frankensqlite-fix"))))' /tmp/spec016-cargo-metadata-"$TS".json >/dev/null; then
  echo "CASS still resolves fsqlite/fsqlite-types through a local path source; stop before release build" >&2
  exit 1
fi

$HOME/.cargo/bin/cargo tree -i fsqlite@0.1.3 > /tmp/spec016-cargo-tree-fsqlite-"$TS".txt
$HOME/.cargo/bin/cargo tree -i fsqlite-types@0.1.3 > /tmp/spec016-cargo-tree-fsqlite-types-"$TS".txt
```

Durable dependency proof guard added on 2026-05-17T05:45:13Z:

```text
The approval-gated runbook now fails before release build if the sibling
frankensqlite checkout is still dirty/unpushed or if CASS still resolves
`fsqlite`/`fsqlite-types` through the local `../spec014-frankensqlite-fix`
path patch.
```

Interpretation: the approved live path can no longer accidentally deploy a
binary that only works because this machine has a dirty sibling checkout.

Durable dependency remote-refresh and patch-header guard added on 2026-05-17T05:48:16Z:

```text
The durable dependency proof now refreshes the sibling `carmandale` and
`origin` remotes before checking whether the frankensqlite fix HEAD is on a
remote branch. Its CASS-side grep also matches the frankensqlite `[patch]`
header itself, not only the local path entries.
```

Interpretation: stale remote-tracking refs cannot create false pushed proof,
and a leftover frankensqlite patch section cannot slip through just because
the local path strings were removed.

## Promotion Capacity Preflight

Read-only capacity refresh on 2026-05-16T23:24:29Z:

```text
filesystem: /dev/disk3s5 mounted on /System/Volumes/Data
size: 3.6Ti
used: 3.4Ti
available: 175Gi
capacity: 96%
```

Read-only capacity rerun on 2026-05-16T23:51:25Z:

```text
filesystem: /dev/disk3s5 mounted on /System/Volumes/Data
size: 3.6Ti
used: 3.4Ti
available: 174Gi
capacity: 96%
shadow data dir: 13G
release candidate sha256: 423e2e4c2920ec74a38a5cb4af1f00de362a4a82e493d342b4891179f4955ada
```

Read-only capacity and release-hash rerun on 2026-05-17T04:15:33Z:

```text
filesystem: /dev/disk3s5 mounted on /System/Volumes/Data
size: 3.6Ti
used: 3.4Ti
available: 151Gi
capacity: 96%
release candidate sha256: a5a139ca503f0f04f2b0baba06e20b16bd7003a7e7d78306cacf94d56eaca9c2
```

Read-only capacity, release-hash, and watcher-state rerun on 2026-05-17T05:09:27Z:

```text
filesystem: /dev/disk3s5 mounted on /System/Volumes/Data
size: 3.6Ti
used: 3.4Ti
available: 150Gi
capacity: 96%
release candidate sha256: a5a139ca503f0f04f2b0baba06e20b16bd7003a7e7d78306cacf94d56eaca9c2
installed cass.real sha256: 47f0692af0fd6484e82e4b69b5512ba44b82de1d0c10d64b5a171b2ed279e691
com.cass.index-watch: absent
com.cass.health-watchdog: runs=353, last exit code=2
```

Current live and shadow footprints:

```text
live data dir: 165G
shadow data dir: 13G
live agent_search.db: 7.1G
live index: 8.1G
shadow agent_search.db: 7.9G
shadow index: 3.7G
```

Interpretation: the approval-gated promotion uses same-directory `mv` for old live DB/index backups and then copies about 11.6G of verified shadow DB/index into the live dir. The target volume currently has enough free space for that copy, but only about 150Gi remains on a 96%-full volume, so re-check `df -h` immediately before live promotion.

## Approval-Gated Live Promotion Shape

The promotion must not delete files. Use timestamped moves for existing live artifacts, then copy the verified shadow artifacts into place.

Suggested shell shape after approval:

```bash
set -euo pipefail

: "${SPEC016_TS:?export SPEC016_TS from the approval-attempt init block before running promotion}"
TS="$SPEC016_TS"
LIVE_DIR="$HOME/Library/Application Support/com.coding-agent-search.coding-agent-search"
SHADOW="$HOME/Library/Application Support/com.coding-agent-search.coding-agent-search-spec016-shadow-20260516T2025Z"
CASS_RELEASE="/tmp/cass-release-target/release/cass"

# Fail before any live artifact moves if a required source, destination, or
# verified runtime is missing. Later commands should prove behavior, not
# discover basic path/runtime absence after the live DB has already moved.
test -d "$LIVE_DIR"
test -w "$LIVE_DIR"
test -f "$LIVE_DIR/agent_search.db"
test -d "$LIVE_DIR/index"
test -r "$SHADOW/agent_search.db"
test -d "$SHADOW/index"
test -r "$SHADOW/index"
test -x "$SHADOW/index"
test -x "$CASS_RELEASE"
for artifact in agent_search.db agent_search.db-shm agent_search.db-wal index watch_state.json; do
  test ! -e "$LIVE_DIR/$artifact.PRE-SPEC016-$TS"
done
RELEASE_HASH="$(shasum -a 256 "$CASS_RELEASE" | awk '{print $1}')"
printf '%s\n' "$RELEASE_HASH" > /tmp/cass-release-pre-promotion-hash-"$TS".txt
printf '%s  %s\n' "$RELEASE_HASH" "$CASS_RELEASE" | tee /tmp/cass-release-pre-promotion-hash-line-"$TS".txt
"$CASS_RELEASE" --version
"$CASS_RELEASE" watchdog run --help >/tmp/cass-release-watchdog-run-help-pre-promotion-"$TS".txt

SHADOW_DB="$SHADOW/agent_search.db"
SHADOW_DB_CHECK="$(sqlite3 "$SHADOW_DB" 'PRAGMA integrity_check;')"
printf '%s\n' "$SHADOW_DB_CHECK" | tee /tmp/cass-shadow-pre-promotion-integrity-"$TS".txt
test "$SHADOW_DB_CHECK" = "ok"
EXPECTED_COUNTS="$(cat <<'EOF'
claude_code|2574
codex|5713
factory|66
opencode|976
pi_agent|2076
messages|1238935
EOF
)"
SHADOW_COUNTS="$(sqlite3 "$SHADOW_DB" "SELECT a.name, COUNT(*) FROM conversations c JOIN agents a ON c.agent_id=a.id WHERE a.name IN ('pi_agent','claude_code','codex','opencode','factory') GROUP BY a.name ORDER BY a.name; SELECT 'messages', COUNT(*) FROM messages;")"
printf '%s\n' "$SHADOW_COUNTS" | tee /tmp/cass-shadow-pre-promotion-counts-"$TS".txt
test "$SHADOW_COUNTS" = "$EXPECTED_COUNTS"
"$CASS_RELEASE" health --json --stale-threshold 86400 --data-dir "$SHADOW" > /tmp/cass-shadow-pre-promotion-health-"$TS".json
jq -e '.healthy == true and .state.index.status == "ready" and .state.index.checkpoint.completed == true and .state.index.checkpoint.db_matches == true' /tmp/cass-shadow-pre-promotion-health-"$TS".json

SHADOW_COPY_KB="$(du -sk "$SHADOW/agent_search.db" "$SHADOW/index" | awk '{sum += $1} END {print sum}')"
AVAILABLE_KB="$(df -Pk "$LIVE_DIR" | awk 'NR == 2 {print $4}')"
printf 'shadow_copy_kb=%s\navailable_kb=%s\n' "$SHADOW_COPY_KB" "$AVAILABLE_KB" | tee /tmp/cass-live-promotion-capacity-"$TS".txt
if [ "$AVAILABLE_KB" -le "$SHADOW_COPY_KB" ]; then
  echo "insufficient free space for shadow DB/index copy; stop before moving live artifacts" >&2
  exit 1
fi

# Fail closed if live CASS writers/readers are active. com.cass.index-watch is
# currently absent, but re-check before promotion and re-align if it appears.
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

# Preserve current live artifacts. No rm.
mv "$LIVE_DIR/agent_search.db" "$LIVE_DIR/agent_search.db.PRE-SPEC016-$TS"
mv "$LIVE_DIR/agent_search.db-shm" "$LIVE_DIR/agent_search.db-shm.PRE-SPEC016-$TS" 2>/dev/null || true
mv "$LIVE_DIR/agent_search.db-wal" "$LIVE_DIR/agent_search.db-wal.PRE-SPEC016-$TS" 2>/dev/null || true
mv "$LIVE_DIR/index" "$LIVE_DIR/index.PRE-SPEC016-$TS"
mv "$LIVE_DIR/watch_state.json" "$LIVE_DIR/watch_state.json.PRE-SPEC016-$TS" 2>/dev/null || true

# Publish verified shadow DB and lexical index.
cp -p "$SHADOW/agent_search.db" "$LIVE_DIR/agent_search.db"
cp -a "$SHADOW/index" "$LIVE_DIR/index"

# Verify live archive before starting watcher. Encode the space in "Application Support"
# for SQLite URI mode; unencoded file: URIs and sqlite3 -readonly path opens have failed.
LIVE_DB="$LIVE_DIR/agent_search.db"
LIVE_DB_RO_URI="file:${LIVE_DB// /%20}?mode=ro"
sqlite3 "$LIVE_DB_RO_URI" 'PRAGMA integrity_check;'
EXPECTED_COUNTS="$(cat <<'EOF'
claude_code|2574
codex|5713
factory|66
opencode|976
pi_agent|2076
messages|1238935
EOF
)"
ACTUAL_COUNTS="$(sqlite3 "$LIVE_DB_RO_URI" "SELECT a.name, COUNT(*) FROM conversations c JOIN agents a ON c.agent_id=a.id WHERE a.name IN ('pi_agent','claude_code','codex','opencode','factory') GROUP BY a.name ORDER BY a.name; SELECT 'messages', COUNT(*) FROM messages;")"
printf '%s\n' "$ACTUAL_COUNTS"
test "$ACTUAL_COUNTS" = "$EXPECTED_COUNTS"
# Pre-watcher archive readiness uses the same 86400s freshness threshold proven
# by shadow release canaries. A 1800s threshold fails before watcher startup
# because the promoted shadow archive is intentionally older than 30 minutes.
"$CASS_RELEASE" health --json --stale-threshold 86400 --data-dir "$LIVE_DIR"
"$CASS_RELEASE" search 'ATT21_COL_CFP_SceneMachine_EndCard.psd' --agent pi_agent --mode lexical --robot --fields minimal --robot-meta --limit 1 --data-dir "$LIVE_DIR" > /tmp/cass-live-promotion-canary-pi-agent-"$TS".json
jq -e '.total_matches > 0' /tmp/cass-live-promotion-canary-pi-agent-"$TS".json
"$CASS_RELEASE" search 'frankensqlite' --agent claude_code --mode lexical --robot --fields minimal --robot-meta --limit 1 --data-dir "$LIVE_DIR" > /tmp/cass-live-promotion-canary-claude-code-"$TS".json
jq -e '.total_matches > 0' /tmp/cass-live-promotion-canary-claude-code-"$TS".json
"$CASS_RELEASE" search 'freelist serializer' --agent codex --mode lexical --robot --fields minimal --robot-meta --limit 1 --data-dir "$LIVE_DIR" > /tmp/cass-live-promotion-canary-codex-"$TS".json
jq -e '.total_matches > 0' /tmp/cass-live-promotion-canary-codex-"$TS".json
"$CASS_RELEASE" search 'opencode' --agent opencode --mode lexical --robot --fields minimal --robot-meta --limit 1 --data-dir "$LIVE_DIR" > /tmp/cass-live-promotion-canary-opencode-"$TS".json
jq -e '.total_matches > 0' /tmp/cass-live-promotion-canary-opencode-"$TS".json
"$CASS_RELEASE" search 'factory' --agent factory --mode lexical --robot --fields minimal --robot-meta --limit 1 --data-dir "$LIVE_DIR" > /tmp/cass-live-promotion-canary-factory-"$TS".json
jq -e '.total_matches > 0' /tmp/cass-live-promotion-canary-factory-"$TS".json
```

Pre-watcher health-threshold refresh on 2026-05-17T04:57:15Z:

```text
/tmp/cass-release-target/release/cass health --json --stale-threshold 1800 --data-dir "$SHADOW"
exit: 1
status: unhealthy
reason: lexical index is older than the stale threshold

/tmp/cass-release-target/release/cass health --json --stale-threshold 86400 --data-dir "$SHADOW"
exit: 0
status: healthy
checkpoint.completed: true
```

Interpretation: the pre-watcher live-archive verification must use the 86400s
threshold that the release-candidate shadow canaries already proved. The 1800s
threshold is appropriate for final freshness expectations after the watcher is
running, but it would create a false approval-time failure immediately after
copying the verified shadow archive and before starting `com.cass.index-watch`.

Pre-install binary consistency refresh on 2026-05-17T04:59:40Z:

```text
release candidate sha256: a5a139ca503f0f04f2b0baba06e20b16bd7003a7e7d78306cacf94d56eaca9c2
installed cass.real sha256: 47f0692af0fd6484e82e4b69b5512ba44b82de1d0c10d64b5a171b2ed279e691
release cass watchdog run --help: exit 0
installed cass watchdog run --help: exit 2
```

Interpretation: live archive verification before the runtime install must use
the approval-gated release candidate, not the old installed binary. The launchd
watcher proof still uses `/Users/dalecarman/.local/bin/cass`, but only after the
runtime install shape has replaced `cass.real` with the verified release
candidate.

Explicit data-dir guard added on 2026-05-17T05:02:54Z:

```text
Pre-install archive health/search verification now passes
--data-dir "$LIVE_DIR"` explicitly. The watcher marker search below also passes
--data-dir "$LIVE_DIR"` after defining the same live data dir.
```

Interpretation: approval-time proof now targets the just-promoted live archive
directly instead of relying on default data-dir resolution.

Pre-watcher counts and canary guard added on 2026-05-17T05:26:27Z:

```text
Before starting `com.cass.index-watch`, the approval-gated promotion block now
requires the promoted live archive to match the verified shadow counts:
claude_code=2574, codex=5713, factory=66, opencode=976, pi_agent=2076,
messages=1238935. It also runs the five proven release-candidate lexical
canaries for pi_agent, claude_code, codex, opencode, and factory against
`--data-dir "$LIVE_DIR"` and requires each search to return total_matches > 0.
```

Interpretation: the approval-time archive proof now covers the priority agent
outcome plus OpenCode/factory bonus non-regression before watcher startup.

## Approval-Gated Runtime Install Shape

Build a release binary from the durable dependency pin, then preserve the installed binary before replacing it.

```bash
set -euo pipefail

: "${SPEC016_TS:?export SPEC016_TS from the approval-attempt init block before running runtime install}"
TS="$SPEC016_TS"
env CARGO_TARGET_DIR=/tmp/cass-release-target $HOME/.cargo/bin/cargo build --release --bin cass
CASS_RELEASE="/tmp/cass-release-target/release/cass"
test -x "$CASS_RELEASE"
RELEASE_HASH="$(shasum -a 256 "$CASS_RELEASE" | awk '{print $1}')"
printf '%s  %s\n' "$RELEASE_HASH" "$CASS_RELEASE"
if [ -e /tmp/cass-release-pre-promotion-hash-"$TS".txt ]; then
  PRE_PROMOTION_RELEASE_HASH="$(cat /tmp/cass-release-pre-promotion-hash-"$TS".txt)"
  test "$RELEASE_HASH" = "$PRE_PROMOTION_RELEASE_HASH"
fi
"$CASS_RELEASE" --version
"$CASS_RELEASE" watchdog run --help >/tmp/cass-release-watchdog-run-help-"$TS".txt
test -x "$HOME/.local/bin/cass.real"
test ! -e "$HOME/.local/bin/cass.real.PRE-SPEC016-$TS"

mv "$HOME/.local/bin/cass.real" "$HOME/.local/bin/cass.real.PRE-SPEC016-$TS"
cp -p "$CASS_RELEASE" "$HOME/.local/bin/cass.real"
test -x "$HOME/.local/bin/cass.real"
INSTALLED_HASH="$(shasum -a 256 "$HOME/.local/bin/cass.real" | awk '{print $1}')"
printf '%s  %s\n' "$INSTALLED_HASH" "$HOME/.local/bin/cass.real"
test "$INSTALLED_HASH" = "$RELEASE_HASH"
/Users/dalecarman/.local/bin/cass --version
/Users/dalecarman/.local/bin/cass capabilities --json >/tmp/cass-capabilities-"$TS".json
```

## Approval-Gated Restore Shape

If live verification fails after promotion or install, preserve the failed promoted artifacts and move the timestamped backups back into place. Do not delete failed artifacts.

Use the same `TS` value from the failed promotion/install attempt:

```bash
set -euo pipefail

: "${SPEC016_TS:?export SPEC016_TS from the failed approval attempt before restore}"
TS="$SPEC016_TS"
LIVE_DIR="$HOME/Library/Application Support/com.coding-agent-search.coding-agent-search"

# Fail before changing restore-time state if the mandatory pre-spec016 backups
# are missing or this attempt suffix was already used for failed artifacts.
test -d "$LIVE_DIR"
test -f "$LIVE_DIR/agent_search.db.PRE-SPEC016-$TS"
test -d "$LIVE_DIR/index.PRE-SPEC016-$TS"
for artifact in agent_search.db agent_search.db-shm agent_search.db-wal index watch_state.json; do
  test ! -e "$LIVE_DIR/$artifact.FAILED-SPEC016-$TS"
done
if [ -e "$HOME/.local/bin/cass.real.PRE-SPEC016-$TS" ]; then
  test ! -e "$HOME/.local/bin/cass.real.FAILED-SPEC016-$TS"
fi

# Stop the index watcher if it was loaded during the failed attempt.
launchctl bootout "gui/$(id -u)/com.cass.index-watch" 2>/tmp/cass-index-watch-bootout-"$TS".err || true

# Preserve failed promoted live artifacts, then restore pre-spec016 live artifacts.
for artifact in agent_search.db agent_search.db-shm agent_search.db-wal index watch_state.json; do
  if [ -e "$LIVE_DIR/$artifact" ]; then
    mv "$LIVE_DIR/$artifact" "$LIVE_DIR/$artifact.FAILED-SPEC016-$TS"
  else
    echo "no failed $artifact present before restoring PRE-SPEC016 artifact" >&2
  fi
done

test ! -e "$LIVE_DIR/agent_search.db"
test ! -e "$LIVE_DIR/agent_search.db-shm"
test ! -e "$LIVE_DIR/agent_search.db-wal"
test ! -e "$LIVE_DIR/index"

mv "$LIVE_DIR/agent_search.db.PRE-SPEC016-$TS" "$LIVE_DIR/agent_search.db"
mv "$LIVE_DIR/index.PRE-SPEC016-$TS" "$LIVE_DIR/index"
mv "$LIVE_DIR/watch_state.json.PRE-SPEC016-$TS" "$LIVE_DIR/watch_state.json" 2>/dev/null || true
mv "$LIVE_DIR/agent_search.db-shm.PRE-SPEC016-$TS" "$LIVE_DIR/agent_search.db-shm" 2>/dev/null || true
mv "$LIVE_DIR/agent_search.db-wal.PRE-SPEC016-$TS" "$LIVE_DIR/agent_search.db-wal" 2>/dev/null || true

# Restore the pre-spec016 installed binary if runtime install already happened.
if [ -e "$HOME/.local/bin/cass.real.PRE-SPEC016-$TS" ]; then
  if [ -e "$HOME/.local/bin/cass.real" ]; then
    mv "$HOME/.local/bin/cass.real" "$HOME/.local/bin/cass.real.FAILED-SPEC016-$TS"
  else
    echo "no failed cass.real present to preserve before restoring PRE-SPEC016 binary" >&2
  fi
  mv "$HOME/.local/bin/cass.real.PRE-SPEC016-$TS" "$HOME/.local/bin/cass.real"
fi

LIVE_DB="$LIVE_DIR/agent_search.db"
LIVE_DB_RO_URI="file:${LIVE_DB// /%20}?mode=ro"
sqlite3 "$LIVE_DB_RO_URI" 'PRAGMA quick_check;'
/Users/dalecarman/.local/bin/cass --version
```

Expected restore result: the previous malformed-but-known live state is back in place, and all failed promoted DB, DB sidecar, index, watch-state, and binary artifacts are preserved with `FAILED-SPEC016-$TS` suffixes for later inspection. This is a rollback from a failed promotion attempt, not a completion path.

## Approval-Gated Watcher Proof Shape

After live DB/index and binary are promoted:

```bash
set -euo pipefail

: "${SPEC016_TS:?export SPEC016_TS from the approved promotion/install attempt before watcher proof}"
TS="$SPEC016_TS"
LIVE_DIR="$HOME/Library/Application Support/com.coding-agent-search.coding-agent-search"
WATCHER_EVIDENCE_DIR="specs/016-cass-recovery-ingestion/evidence/watcher-proof"
CASS_RELEASE="/tmp/cass-release-target/release/cass"
mkdir -p "$WATCHER_EVIDENCE_DIR"

test -x "$CASS_RELEASE"
test -x "$HOME/.local/bin/cass.real"
RELEASE_HASH="$(shasum -a 256 "$CASS_RELEASE" | awk '{print $1}')"
INSTALLED_HASH="$(shasum -a 256 "$HOME/.local/bin/cass.real" | awk '{print $1}')"
printf '%s  %s\n%s  %s\n' "$RELEASE_HASH" "$CASS_RELEASE" "$INSTALLED_HASH" "$HOME/.local/bin/cass.real" | tee "$WATCHER_EVIDENCE_DIR/pre-watcher-binary-hashes-$TS.txt"
test "$INSTALLED_HASH" = "$RELEASE_HASH"
/Users/dalecarman/.local/bin/cass --version > "$WATCHER_EVIDENCE_DIR/pre-watcher-installed-version-$TS.txt"
/Users/dalecarman/.local/bin/cass watchdog run --help > "$WATCHER_EVIDENCE_DIR/pre-watcher-installed-watchdog-help-$TS.txt"

launchctl bootstrap "gui/$(id -u)" "$HOME/Library/LaunchAgents/com.cass.index-watch.plist" 2>"$WATCHER_EVIDENCE_DIR/index-watch-bootstrap-$TS.err" || true
launchctl print "gui/$(id -u)/com.cass.index-watch" | tee "$WATCHER_EVIDENCE_DIR/index-watch-print-$TS.txt"
launchctl list | rg 'cass|coding-agent' | tee "$WATCHER_EVIDENCE_DIR/launchctl-list-$TS.txt"
WATCH_PROCESSES=""
watch_process_deadline=$((SECONDS + 30))
while (( SECONDS < watch_process_deadline )); do
  WATCH_PROCESSES="$(ps -axo pid,state,rss,%cpu,etime,command | awk '$0 ~ /cass/ && $0 ~ /index/ && $0 ~ /--watch/ && $0 !~ /awk/ && $0 !~ /zsh -/ && $0 !~ /ps -axo/ {print}')"
  if [ -n "$WATCH_PROCESSES" ]; then
    break
  fi
  sleep 2
done
printf '%s\n' "$WATCH_PROCESSES" | tee "$WATCHER_EVIDENCE_DIR/watch-processes-$TS.txt"
if [ -z "$WATCH_PROCESSES" ]; then
  launchctl print "gui/$(id -u)/com.cass.index-watch" > "$WATCHER_EVIDENCE_DIR/index-watch-print-failed-$TS.txt" 2>&1 || true
  cat "$WATCHER_EVIDENCE_DIR/index-watch-print-failed-$TS.txt" >&2
  echo "cass index --watch process did not appear within 30s" >&2
  exit 1
fi

MARKER="SPEC016_WATCHER_MARKER_$(date -u +%Y%m%dT%H%M%SZ)"
ISO_TS="$(date -u +%Y-%m-%dT%H:%M:%S.000Z)"
DATE_PATH="$(date -u +%Y/%m/%d)"
CODEX_PROOF_DIR="$HOME/.codex/sessions/$DATE_PATH"
SYNTH_FILE="$CODEX_PROOF_DIR/rollout-spec016-watcher-$MARKER.jsonl"

mkdir -p "$CODEX_PROOF_DIR"
test ! -e "$SYNTH_FILE"
set -o noclobber
cat > "$SYNTH_FILE" <<EOF
{"timestamp":"$ISO_TS","type":"session_meta","payload":{"id":"spec016-watcher-$MARKER","cwd":"/Users/dalecarman/dev/coding_agent_session_search","cli_version":"spec016-proof"}}
{"timestamp":"$ISO_TS","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"$MARKER"}]}}
{"timestamp":"$ISO_TS","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"text","text":"$MARKER response"}]}}
EOF
set +o noclobber

found=0
deadline=$((SECONDS + 120))
while (( SECONDS < deadline )); do
  /Users/dalecarman/.local/bin/cass search "$MARKER" --agent codex --mode lexical --robot --fields minimal --robot-meta --limit 1 --data-dir "$LIVE_DIR" > "$WATCHER_EVIDENCE_DIR/marker-search-$TS.json"
  if jq -e --arg src "$SYNTH_FILE" '.total_matches > 0 and ([((.hits // .results // [])[])?.source_path] | index($src) != null)' "$WATCHER_EVIDENCE_DIR/marker-search-$TS.json" >/dev/null; then
    echo "watcher marker searchable: $MARKER"
    echo "watcher proof source: $SYNTH_FILE"
    found=1
    break
  fi
  sleep 5
done
if [ "$found" -ne 1 ]; then
  echo "watcher marker not searchable within 120s: $MARKER" >&2
  echo "watcher proof source: $SYNTH_FILE" >&2
  exit 1
fi

/Users/dalecarman/.local/bin/cass health --json --stale-threshold 1800 --data-dir "$LIVE_DIR" > "$WATCHER_EVIDENCE_DIR/post-watcher-health-$TS.json"
jq -e '.healthy == true and .state.index.status == "ready" and .state.index.checkpoint.completed == true and .state.pending.watch_active == true' "$WATCHER_EVIDENCE_DIR/post-watcher-health-$TS.json"
tail -n 200 "$HOME/Library/Logs/cass-index-watch.log" > "$WATCHER_EVIDENCE_DIR/index-watch-tail-$TS.log"
rg "$MARKER|streaming_ingest|watch_scan|streaming_scan_complete" "$WATCHER_EVIDENCE_DIR/index-watch-tail-$TS.log"
printf '%s\n' "$SYNTH_FILE" > "$WATCHER_EVIDENCE_DIR/synthetic-source-$TS.txt"
printf '%s\n' "$MARKER" > "$WATCHER_EVIDENCE_DIR/marker-$TS.txt"
```

The preferred proof creates a new synthetic connector-compatible Codex session file under the natural watched `~/.codex/sessions/YYYY/MM/DD/` root. That is still a live session-root mutation and should be done only after approval. If the watcher ignores the synthetic file for an unexpected connector reason, fallback to appending the marker to the current real Codex session and record the exact path plus the synthetic failure evidence.

Watcher-proof overwrite guard added on 2026-05-17T04:55:37Z:

```text
The approval-gated synthetic Codex proof now checks `test ! -e "$SYNTH_FILE"`
and enables shell `noclobber` before writing the marker JSONL. This makes the
proof fail closed instead of overwriting an existing session artifact if the
timestamped path collides or the command is rerun with the same marker.
```

Watcher-proof timeout guard added on 2026-05-17T05:01:29Z:

```text
The approval-gated watcher proof now tracks `found=1` only after search returns
`.total_matches > 0`. If the 120-second loop expires without a hit, the command
prints the marker/source path to stderr and exits 1.
```

Interpretation: the watcher proof can no longer fall through as a false success
when the marker never becomes searchable.

Watcher-proof exact source guard added on 2026-05-17T06:18:25Z:

```text
The approval-gated watcher marker loop now requires the search response to have
`total_matches > 0` and at least one hit/result whose `source_path` equals the
synthetic Codex file written by this proof attempt.
```

Interpretation: a stale or colliding marker hit from another source can no
longer satisfy the watcher proof. The proof must show the file created under the
real watched `~/.codex/sessions/YYYY/MM/DD/` tree became searchable.

Post-watcher health and log guard added on 2026-05-17T05:28:13Z:

```text
After the synthetic marker becomes searchable, the approval-gated watcher proof
now requires installed live CASS health to be healthy, lexical index ready,
checkpoint completed, and pending.watch_active=true at the 1800-second freshness
threshold. It also captures the last 200 index-watch log lines and requires the
tail to contain the marker or structured watcher ingest/scan evidence.
```

Interpretation: the final watcher proof is no longer just a search hit. It must
also prove the live installed runtime reports a fresh healthy watched archive
and that the launchd watcher emitted ingestion or scan evidence.

Watcher-proof evidence persistence added on 2026-05-17T05:50:25Z:

```text
The approval-gated watcher proof now saves launchctl output, process proof,
marker search JSON, post-watcher health JSON, index-watch log tail, marker, and
synthetic source path under
`specs/016-cass-recovery-ingestion/evidence/watcher-proof/`.
```

Interpretation: the eventual approved watcher proof leaves replayable evidence
under the spec instead of relying on transient `/tmp` files or terminal output.

Pre-watcher installed-runtime equality guard added on 2026-05-17T06:05:15Z:

```text
Before bootstrapping `com.cass.index-watch`, the approval-gated watcher proof now
checks that installed `cass.real` is executable, has the same sha256 as the
tested release candidate, and exposes the installed `watchdog run` command
surface. The hashes and command proof are saved in `evidence/watcher-proof/`.
```

Interpretation: the watcher proof can no longer launch the old installed binary
after a skipped or partial runtime install and then mistake launchd activity for
proof that the verified release candidate is watching the live archive.

Approval-attempt timestamp guard added on 2026-05-17T05:33:19Z:

```text
The approval-gated runbook now creates one `SPEC016_TS` token for the whole
approved attempt and every live-mutating block requires that token before it
can run. Promotion, runtime install, watcher proof, and restore therefore share
the same `PRE-SPEC016-$TS` and `FAILED-SPEC016-$TS` suffixes.
```

Interpretation: a failed approved attempt can no longer strand the restore path
because promotion and install used different timestamp suffixes, and the
watcher proof can no longer fail later under `set -u` because `TS` was only
defined in a previous shell snippet.

Approval-attempt token ordering guard added on 2026-05-17T05:52:55Z:

```text
The `SPEC016_TS` initialization block now appears before durable dependency,
promotion, install, watcher, restore, and branch/upstream proof sections.
```

Interpretation: every approval-gated block that requires `SPEC016_TS` now has a
single earlier init step, including the durable dependency proof that runs
before the release build.

Watcher-process self-match guard added on 2026-05-17T05:04:45Z:

```text
The approval-gated watcher proof now captures watcher processes with an awk
filter that requires cass + index + --watch and excludes the probe process. It
then asserts the captured output is non-empty with `test -n "$WATCH_PROCESSES"`.
```

Interpretation: the process proof can no longer pass just because `rg` matched
its own command line.

Watcher-process bounded wait added on 2026-05-17T05:22:40Z:

```text
The approval-gated watcher proof now waits up to 30 seconds for a real non-probe
`cass index --watch` process after launchd bootstrap. If no process appears, it
prints the launchctl service state and exits 1.
```

Interpretation: the proof avoids a launchd spawn-delay false failure while still
failing closed before writing the synthetic Codex marker.

Pre-promotion live-process guard added on 2026-05-17T05:18:12Z:

```text
The approval-gated promotion block now fails closed if `com.cass.index-watch`
is already loaded or if a non-probe live CASS index/search/doctor/health
process is active. It prints the captured process rows and exits 1 instead of
continuing into DB/index moves.
```

Interpretation: the promotion path can no longer silently continue while a live
CASS process may have the database or index open.

Pre-promotion capacity guard added on 2026-05-17T05:55:20Z:

```text
The approval-gated promotion block now calculates the verified shadow DB/index
copy footprint and current free space on the live volume, records both to
`/tmp/cass-live-promotion-capacity-$TS.txt`, and exits before moving live
artifacts if available space is not greater than the copy footprint.
```

Interpretation: the promotion path can no longer move aside the old live DB and
index before discovering there is not enough free space to publish the verified
shadow archive.

Pre-move artifact and runtime guard added on 2026-05-17T06:02:09Z:

```text
The approval-gated promotion block now verifies the live data dir, current live
DB/index, verified shadow DB/index, and executable release candidate before any
live DB/index move. It also records the release hash and proves the release
`watchdog run` command surface before promotion starts.
```

Interpretation: the promotion path can no longer move the current live DB aside
and only then discover that the shadow source, destination, or tested release
runtime is missing or unusable.

Pre-move shadow integrity and readiness guard added on 2026-05-17T06:09:56Z:

```text
Before any live DB/index move, the approval-gated promotion block now verifies
the shadow DB integrity, exact priority/bonus/message counts, and release
candidate health against the shadow archive at the 86400-second threshold.
```

Interpretation: the promotion path can no longer rely on stale shadow proof and
only discover after moving live artifacts that the shadow DB/index is no longer
healthy, count-matched, or ready.

Runtime-install and restore guard added on 2026-05-17T05:21:01Z:

```text
The approval-gated runtime install block now tests that the rebuilt release
candidate and existing installed binary are executable before moving
`cass.real`, verifies the release command surface before install, and checks
that the copied replacement is executable before continuing. The restore block
now restores the `PRE-SPEC016` binary even if no failed replacement binary is
present to preserve.
```

Interpretation: a failed approved install cannot strand the restore path on a
missing `cass.real` after the pre-spec016 binary has already been preserved.

Runtime-install hash equality guard added on 2026-05-17T05:42:52Z:

```text
The approval-gated install block now captures the rebuilt release hash and the
post-copy installed `cass.real` hash, then requires exact equality before
continuing to version/capabilities proof.
```

Interpretation: the install proof can no longer pass merely because hashes were
printed. The installed binary must be byte-for-byte the tested release artifact.

Restore missing-failed-artifact guard added on 2026-05-17T05:24:27Z:

```text
The approval-gated restore block now preserves failed DB, DB sidecar, index, and
watch-state artifacts only when those paths exist, and logs missing failed
artifacts before restoring the corresponding `PRE-SPEC016` backups.
```

Interpretation: if an approved promotion fails after moving old live artifacts
away but before copying every replacement, the restore path still moves the
known previous DB/index/watch state back into place.

Restore preflight and suffix-collision guard added on 2026-05-17T06:12:03Z:

```text
The approval-gated restore block now verifies the mandatory `PRE-SPEC016-$TS`
DB and index backups exist, and verifies no `FAILED-SPEC016-$TS` destination
already exists, before bootout or failed-artifact preservation begins.
```

Interpretation: a restore attempt can no longer move failed promoted artifacts
aside and only then discover that the required previous DB/index backups are
missing, nor can a rerun overwrite failed-artifact evidence with the same suffix.

Pre-backup suffix-collision guard added on 2026-05-17T06:13:54Z:

```text
The approval-gated promotion block now verifies no live DB/index/watch-state
`PRE-SPEC016-$TS` backup destination exists before preserving current live
artifacts. The runtime install block similarly verifies no
`cass.real.PRE-SPEC016-$TS` backup exists before preserving the installed binary.
```

Interpretation: rerunning an approved attempt with the same `SPEC016_TS` can no
longer overwrite or collide with prior `PRE-SPEC016` backups before the restore
path has a reliable previous-live artifact to move back.

Release hash continuity guard added on 2026-05-17T06:16:09Z:

```text
The approval-gated promotion block now writes the release hash it used for
pre-promotion archive verification. If the runtime install block later rebuilds
the release binary in the same approved attempt, it compares the rebuilt hash to
that pre-promotion hash before installing.
```

Interpretation: the approved path can no longer prove the promoted archive with
one release binary and then install a different rebuilt binary without an
explicit stop.

Synthetic Codex proof-format preflight on 2026-05-16T23:27:50Z:

```text
scratch root: /tmp/cass-spec016-synth-proof-20260516T232750Z
synthetic file: /tmp/cass-spec016-synth-proof-20260516T232750Z/.codex/sessions/2026/05/16/rollout-spec016-synthetic-20260516T232750Z.jsonl
marker: SPEC016_SYNTH_FORMAT_PROOF_20260516T232750Z
binary: /tmp/cass-release-target/release/cass
index command: CODEX_HOME="$SCRATCH/.codex" HOME="$SCRATCH" cass index --full --data-dir "$SCRATCH/data" --json --no-progress-events
index result: success=true, conversations=1, messages=2, codex conversations=1, codex messages=2
search command: CODEX_HOME="$SCRATCH/.codex" HOME="$SCRATCH" cass search "$MARKER" --agent codex --mode lexical --robot --fields minimal --robot-meta --limit 5 --data-dir "$SCRATCH/data"
search result: total_matches=2, count=2, both hits source_path="$SYNTH_FILE", agent=codex
```

Interpretation: the synthetic JSONL shape is connector-compatible and searchable with the release candidate in a scratch data dir. This validates the proof fixture format only; it does not prove the live launchd watcher until the approved live service processes a synthetic file under the real `~/.codex/sessions/` tree.

Synthetic Codex watch-once preflight on 2026-05-16T23:33:14Z:

```text
scratch root: /tmp/cass-spec016-watchonce-proof-20260516T233309Z
synthetic file: /tmp/cass-spec016-watchonce-proof-20260516T233309Z/.codex/sessions/2026/05/16/rollout-spec016-watchonce-20260516T233309Z.jsonl
marker: SPEC016_WATCHONCE_FORMAT_PROOF_20260516T233309Z
binary: /tmp/cass-release-target/release/cass
index command: CODEX_HOME="$SCRATCH/.codex" HOME="$SCRATCH" cass index --watch-once "$SYNTH_FILE" --data-dir "$SCRATCH/data" --json --no-progress-events
index result: success=true, entrypoint.kind=watch_once, watch_once_path_count=1, conversations=1, messages=2, agents_discovered=[codex]
search command: CODEX_HOME="$SCRATCH/.codex" HOME="$SCRATCH" cass search "$MARKER" --agent codex --mode lexical --robot --fields minimal --robot-meta --limit 5 --data-dir "$SCRATCH/data"
search result: total_matches=2, count=2, both hits source_path="$SYNTH_FILE", agent=codex, index.status=ready
```

Interpretation: the release candidate can process the proposed synthetic marker through the one-shot watch ingestion entrypoint and make it searchable. This is closer to the approval-gated launchd watcher proof than a full-index fixture, but it still does not prove live `com.cass.index-watch` until the approved service sees a real file event under `~/.codex/sessions/`.

Runbook shell syntax refresh on 2026-05-17T04:24:28Z:

```text
extracted bash code blocks to /tmp/spec016-runbook-shell-blocks.sh
zsh -n /tmp/spec016-runbook-shell-blocks.sh
result: pass
```

Interpretation: the approval-gated shell blocks in this runbook are parseable by zsh, including the synthetic Codex heredoc. This is only a syntax check; it does not execute or validate live side effects.

Restore-shape safety refresh on 2026-05-17T04:52:33Z:

```text
The approval-gated restore block now preserves failed promoted
agent_search.db-shm and agent_search.db-wal sidecars with
FAILED-SPEC016-$TS suffixes before restoring PRE-SPEC016 sidecars.
```

Interpretation: a failed promoted DB open cannot leave `agent_search.db-shm` or
`agent_search.db-wal` in place to collide with restoring the pre-spec016
sidecars. This remains an approval-gated restore path, not a live action and not
a completion path.

Runbook tool availability refresh on 2026-05-17T04:22:01Z:

```text
sqlite3=/usr/bin/sqlite3
jq=/opt/homebrew/bin/jq
launchctl=/bin/launchctl
plutil=/usr/bin/plutil
rg=/opt/homebrew/bin/rg
shasum=/usr/bin/shasum
date=/bin/date
mkdir=/bin/mkdir
cp=/bin/cp
mv=/bin/mv
ls=/bin/ls
```

```text
/Users/dalecarman/.local/bin/cass -> /Users/dalecarman/.local/bin/cass.real
/Users/dalecarman/.local/bin/cass.real exists, mode -rwxr-xr-x, size 54342720
/tmp/cass-release-target/release/cass exists, mode -rwxr-xr-x, size 54458736
/Users/dalecarman/Library/LaunchAgents/com.cass.index-watch.plist exists, mode -rw-r--r--, size 1223
plutil -lint com.cass.index-watch.plist: OK
installed cass.real sha256: 47f0692af0fd6484e82e4b69b5512ba44b82de1d0c10d64b5a171b2ed279e691
release candidate sha256: a5a139ca503f0f04f2b0baba06e20b16bd7003a7e7d78306cacf94d56eaca9c2
```

Watcher log failure context refresh on 2026-05-16T23:31:54Z:

```text
/Users/dalecarman/Library/Logs/cass-index-watch.log
size: 39M
mtime: 2026-05-15 18:27:13 local
OOM-related watcher entries: 184
last timestamp in tail: 2026-05-15T23:27:13.316918Z
tail pattern: watch ingest batch/single Codex conversation ran out of memory; some conversations were quarantined and watch progress advanced
```

```text
/Users/dalecarman/Library/Logs/cass-watchdog.log
size: 15K
mtime: 2026-05-16 18:24:02 local
"Could not parse arguments" entries: 448
last line: Could not parse arguments
```

Interpretation: historical logs support the current diagnosis. The old index watcher failed on Codex OOM/quarantine behavior and is not currently writing fresh log lines because the service is absent. The health watchdog is still failing at argument parsing, matching the loaded plist's unsupported `cass watchdog run` command. Health-watchdog remains nonblocking for this recovery unless it interferes with `com.cass.index-watch` proof.

Health-watchdog command-surface refresh on 2026-05-16T23:45:59Z:

```text
/tmp/cass-release-target/release/cass watchdog run --help
exit: 2
output: Could not parse arguments

/Users/dalecarman/.local/bin/cass watchdog run --help
exit: 2
output: Could not parse arguments
```

Historical interpretation at 2026-05-16T23:45:59Z: the then-current release
candidate did not fix `com.cass.health-watchdog`. This was superseded by the
2026-05-17T04:15:33Z refresh below. Current truth: the approval-gated release
candidate now supports `cass watchdog run`, while the installed binary and live
launchd service remain broken until approved install and launchd smoke.

Health-watchdog command-surface refresh on 2026-05-17T04:15:33Z:

```text
/tmp/cass-release-target/release/cass watchdog run --help
exit: 0
output: Run a one-shot health check (heartbeat + log rotation + restart if stale)

/Users/dalecarman/.local/bin/cass watchdog run --help
exit: 2
output: Could not parse arguments

launchctl print gui/501/com.cass.health-watchdog
state: not running
runs: 348
last exit code: 2
```

Interpretation: the approval-gated release candidate now fixes the watchdog
command surface, but live `com.cass.health-watchdog` remains broken until the
verified binary is installed and launchd smoke proves the plist no longer exits
with the parse failure. Direct `com.cass.index-watch` proof is still the
required watcher outcome for spec 016.

Path permission preflight on 2026-05-16T23:35:19Z:

```text
live data dir: exists, readable, writable, searchable
live agent_search.db: exists, readable, writable
live index dir: exists, readable, writable, searchable
shadow data dir: exists, readable, writable, searchable
shadow agent_search.db: exists, readable, writable
shadow index dir: exists, readable, writable, searchable
~/.local/bin: exists, readable, writable, searchable
~/.local/bin/cass.real: exists, readable, writable, executable
~/Library/LaunchAgents: exists, readable, writable, searchable
~/Library/LaunchAgents/com.cass.index-watch.plist: exists, readable, writable
~/Library/Logs: exists, readable, writable, searchable
```

Interpretation: the approval-gated backup/publish/install/launchd proof paths are currently accessible to this session. Re-check immediately before live promotion if much time has passed.

## Post-Promotion Gates

After live proof:

1. Refresh `completion-audit.md` with live counts and watcher proof.
2. Run `$code-verify`.
3. Run `$finalize`.
4. Stage only scoped files by name.
5. Resolve the `dac/main` branch authorization before committing or pushing.

## Approval-Gated Upstream And Branch Resolution Shape

The user outcome includes being deliberately in sync with upstream. Do not treat
live promotion as final if `upstream/main` is still not an ancestor of the
final CASS commit, unless the blocker is recorded with current evidence.

Run this only after the exact approval phrase and after live proof is captured:

```bash
set -euo pipefail

: "${SPEC016_TS:?export SPEC016_TS from the approved attempt before branch/upstream resolution}"
TS="$SPEC016_TS"

git fetch upstream main
git fetch origin
git status --short --branch > /tmp/spec016-git-status-before-finalize-"$TS".txt
git rev-parse --abbrev-ref HEAD > /tmp/spec016-branch-before-finalize-"$TS".txt
git rev-parse HEAD > /tmp/spec016-head-before-finalize-"$TS".txt
git rev-parse upstream/main > /tmp/spec016-upstream-main-before-finalize-"$TS".txt
git merge-base HEAD upstream/main > /tmp/spec016-merge-base-before-finalize-"$TS".txt
git rev-list --left-right --count HEAD...upstream/main > /tmp/spec016-ahead-behind-before-finalize-"$TS".txt
git merge-tree --write-tree HEAD upstream/main > /tmp/spec016-merge-tree-before-finalize-"$TS".txt

if git merge-base --is-ancestor upstream/main HEAD; then
  echo "upstream/main is already an ancestor of HEAD"
else
  echo "upstream/main is not yet incorporated; stop before commit/push and record the branch/upstream blocker or approved incorporation path" >&2
  exit 1
fi

git diff --name-only > /tmp/spec016-diff-files-before-finalize-"$TS".txt
git diff --name-only --cached > /tmp/spec016-staged-files-before-finalize-"$TS".txt
```

If this exits because upstream is not incorporated, do not paper over it with a
generic "merge later" note. Either incorporate upstream through the approved
non-destructive branch path, then rerun the proof above, or keep
`full_outcome_complete=false` and record the blocker in `completion-audit.md`,
GoalBuddy state, and the final handoff.

If the current branch is still `dac/main`, the final receipt must list the exact
branch target and staged paths before any commit or push. Do not stage unrelated
dirty or untracked files, and do not push to upstream.

## Scoped Finalization Boundary

Before any commit, re-run:

```bash
git status --short --branch
git diff --name-only
git diff --name-only --cached
```

Only these paths are currently in scope for this recovery and should be considered for staging:

```text
.beads/issues.jsonl
.beads/last-touched
Cargo.lock
Cargo.toml
docs/goals/cass-session-ingestion-recovery/
specs/016-cass-recovery-ingestion/
src/indexer/mod.rs
src/indexer/redact_secrets.rs
src/indexer/scratch_root.rs
src/lib.rs
src/main.rs
src/search/asset_state.rs
src/storage/sqlite.rs
src/ui/app.rs
src/watchdog.rs
specs/018-health-watchdog-command-surface/
specs/019-ubs-warning-policy-closeout/
tests/cli_robot.rs
tests/golden/robot/capabilities.json.golden
tests/golden/robot/introspect.json.golden
tests/golden/robot_docs/commands.txt.golden
tests/golden/robot_docs/robot_help.txt.golden
tests/spec_015_streaming_watch_once.rs
```

The sibling frankensqlite checkout has its own durable-fix scope:

```text
/Users/dalecarman/dev/spec014-frankensqlite-fix/crates/fsqlite-pager/src/pager.rs
/Users/dalecarman/dev/spec014-frankensqlite-fix/crates/fsqlite-wal/src/wal.rs
```

Do not stage unrelated untracked files such as older notes, screenshots, `.pi/`, `.ralph-o/`, `.tldr/`, historical specs unrelated to this recovery, or local installer/LaunchAgent experiments unless they become explicitly tied to the approved live proof.
