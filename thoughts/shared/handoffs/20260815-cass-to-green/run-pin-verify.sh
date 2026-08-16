#!/bin/bash
# Durable copy of the generation-7 runner for verify-fsqlite-pin.sh.
#
# It lived only in a background job's tmp directory, which is deleted with the
# job. Preserved here on 2026-08-16 by generation 8 because it carries three
# things the committed verify script does not, each of which cost real time to
# work out.
#
# 1. WHY A RAW SHA AND NOT A BRANCH NAME.
#    verify-fsqlite-pin.sh does `git checkout --detach "$REF"`, and inside its
#    clone the branch exists only as a remote-tracking ref. Git then DWIMs
#    `checkout <name>` into `checkout -b <name> --track origin/<name>`, which
#    collides with --detach:
#        fatal: '--detach' cannot be used with '-b/-B/--orphan'
#    A raw SHA cannot DWIM, so it takes the plain detach path. The verify script
#    had been staged behind a disk gate that always refused, so this bug had
#    never once fired. It is a workaround, not the fix — fix the script itself
#    when you next touch it (resolve REF to a SHA with `git rev-parse` before
#    the checkout).
#
# 2. WHY THE BUILD IS SHRUNK RATHER THAN THE DISK CLEARED.
#    This repo's AGENTS.md RULE 1 forbids deleting any file without express
#    written permission, and ~97 GiB of stale cass cargo targets in /tmp are
#    therefore off limits. CARGO_INCREMENTAL=0 drops a directory measured at
#    9.6 GiB on an existing target of this crate that a one-shot verification
#    never reuses, and CARGO_PROFILE_DEV_DEBUG=line-tables-only keeps file:line
#    in backtraces while dropping full debug info. Measured on an existing
#    target of the same crate: 26 GiB debug, of which 18 GiB deps and 9.6 GiB
#    incremental.
#
# 3. WHERE THE RESULT GOES.
#    ~/.cass-catchup/fsqlite-pin-verify.log, so a successor session can read the
#    verdict without needing the launching job to still exist.
#
# RESULT OF THE 2026-08-16 RUN: RED. See bead p3kgr. The suite does not pass at
# =0.1.14 — 4 fsqlite FTS5 tests fail outright and 16 tests hang in an
# asupersync busy-spin. Do not re-run this expecting green; the next move is a
# different revision, not another run at this one.
set -uo pipefail

REF="${1:?usage: run-pin-verify.sh <sha>}"
LOG="$HOME/.cass-catchup/fsqlite-pin-verify.log"
SCRIPT=/Users/dalecarman/dev/coding_agent_session_search/thoughts/shared/handoffs/20260815-cass-to-green/verify-fsqlite-pin.sh

{
  echo "=== pin verify START $(date -u +%Y-%m-%dT%H:%M:%SZ) ref=$REF ==="
  FSQLITE_VERIFY_REF="$REF" \
  FSQLITE_VERIFY_FLOOR_GB=45 \
  CARGO_INCREMENTAL=0 \
  CARGO_PROFILE_DEV_DEBUG=line-tables-only \
    bash "$SCRIPT"
  echo "=== pin verify END rc=$? $(date -u +%Y-%m-%dT%H:%M:%SZ) free=$(df -g / | tail -1 | awk '{print $4}')GiB ==="
} > "$LOG" 2>&1

tail -20 "$LOG"
