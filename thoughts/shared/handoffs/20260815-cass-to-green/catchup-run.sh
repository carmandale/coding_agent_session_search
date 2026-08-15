#!/bin/bash
# Catch-up indexer for every connector tree, not just Codex.
#
# The generation-2 backfill covered 4,895 Codex rollouts and zero Claude Code
# transcripts. Measured 2026-08-15T11:39Z, claude_code had 8,056 unindexed files
# on disk and codex a further 5,017, of which 2,653 predate `last_scan_ts`.
#
# Path-scoped `--watch-once` is used deliberately, for two properties an ordinary
# incremental run does not have:
#   1. it bypasses the mtime filter, so files older than the watermark are still
#      read — an incremental run would skip them AND advance the watermark past
#      them, closing the door permanently;
#   2. it does not advance `last_scan_ts`, so running it first is safe.
#
# Batches are idempotent and resumable: a completed batch re-runs as a no-op via
# explicit_watch_once_root_unchanged_after_last_index. If this dies, re-run it.
#
# CASS_SKIP_PREFLIGHT_CLEANUP_ORPHAN_FK_ROWS=1 is carried over from the
# generation-2 backfill: zero orphans exist, and the sweep is a no-op that wedges.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORK="${CASS_CATCHUP_DIR:-$HOME/.cass-catchup}"
BIN="${CASS_BIN:-/Users/dalecarman/.local/bin/cass}"
MANIFEST="$WORK/manifest.txt"
LOG="$WORK/run.log"
BATCH_SIZE="${CASS_CATCHUP_BATCH:-250}"

# Refuse to run concurrently with the generation-2 codex backfill. Two indexers
# writing one archive is not a supported configuration, and the codex job holds
# the write lock in long bursts.
if pgrep -f 'scratchpad/backfill.sh' >/dev/null 2>&1; then
  echo "REFUSING: the generation-2 codex backfill is still running." >&2
  echo "Wait for '=== backfill done' in its run.log, then re-run this." >&2
  exit 1
fi

if [ ! -x "$BIN" ]; then
  echo "REFUSING: no executable cass at $BIN" >&2
  exit 1
fi

mkdir -p "$WORK" || exit 1

python3 "$HERE/catchup-manifest.py" "$MANIFEST" || exit 1

count=$(wc -l < "$MANIFEST" | tr -d ' ')
if [ "$count" -eq 0 ]; then
  echo "Nothing to do — every file on disk already has a conversation row."
  exit 0
fi

cd "$WORK" || exit 1
rm -f batch-* 2>/dev/null
split -l "$BATCH_SIZE" "$MANIFEST" batch-

{
  echo "=== catchup start $(date -u +%FT%TZ) ==="
  echo "manifest: $count files"
  echo "batches : $(ls batch-* | wc -l | tr -d ' ')"
  echo "binary  : $(shasum -a 256 "$BIN" | cut -c1-16)"
} > "$LOG"

cd "$HOME" || exit 1
for b in "$WORK"/batch-*; do
  echo "=== $(basename "$b") START $(date -u +%FT%TZ) ===" >> "$LOG"
  CASS_SKIP_PREFLIGHT_CLEANUP_ORPHAN_FK_ROWS=1 \
  "$BIN" index --watch-once "$(paste -sd, "$b")" \
    --json --progress-interval-ms 60000 >> "$LOG" 2>&1
  rc=$?
  n=$(sqlite3 -readonly \
    "$HOME/Library/Application Support/com.coding-agent-search.coding-agent-search/agent_search.db" \
    "SELECT count(*) FROM conversations;" 2>/dev/null)
  echo "=== $(basename "$b") END rc=$rc conversations=$n $(date -u +%FT%TZ) ===" >> "$LOG"
done

echo "=== catchup done $(date -u +%FT%TZ) ===" >> "$LOG"
