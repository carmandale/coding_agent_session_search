#!/bin/bash
# Verify the frankensqlite 0.1.5 -> 0.1.14 pin bump (bead p3kgr) before it lands.
#
# WHAT IS ALREADY KNOWN, so this run does not re-establish it:
#   - it BUILDS: generation 6 produced binary 572ae86d from this tree
#   - it FIXES the real defect: a controlled A/B on one specimen with both
#     controls firing showed 0.1.5 emitting `not implemented: reloading populated
#     WITHOUT ROWID table` and reporting coverage UNKNOWN, while 0.1.14 emits no
#     WARN and reports complete, with identical row counts from both
#   - the crates are vendored locally, so no network is needed
#
# WHAT HAS NEVER RUN is the test suite at that pin. That is the whole job here.
#
# IT HAS NOW RUN, AND THE ANSWER IS RED (2026-08-16, generation 8, bead p3kgr).
# Do not re-run this expecting green — the next move is a different revision,
# not another run at this one. The suite fails at =0.1.14 on two independent
# axes, each reproduced isolated with only the pin varying:
#
#   fsqlite FTS5   4 tests fail. Two are semantic — an already-healthy FTS table
#                  is reported as needing a rebuild, and a one-row incremental
#                  catch-up degrades to a full rebuild. Two are hard open
#                  failures: "database disk image is malformed: FTS5 table
#                  `fts_messages` is missing required content shadow table
#                  `fts_messages_content`" on a database 0.1.5 opens fine, which
#                  puts the FTS repair path out of reach for exactly the
#                  databases that need it.
#
#   asupersync     16 tests hang, not fail: 4 search::model_download::* and 12
#                  update_check::integration_*. Threads pin at 100% in
#                  run_future_with_budget -> yield_now under block_on, a
#                  busy-spin rather than blocked I/O. The fixture server is
#                  local, so no network is involved. The suite cannot terminate.
#
# The live archive is NOT exposed by the FTS half: checked read-only with a
# positive control first, it carries `messages` and no `fts_messages` at all, so
# there is no FTS5 virtual table for 0.1.14 to refuse.
#
# This is a trade rather than a regression against a clean baseline: 0.1.5 has
# its own FTS5 shadow-table defect in the opposite direction ("not implemented:
# reloading populated WITHOUT ROWID table fts_messages_idx into MemDatabase").
# Refusing 0.1.14 is safe today only because the incremental path is already
# unwedged in production by CASS_SKIP_PREFLIGHT_CLEANUP_ORPHAN_FK_ROWS=1.
#
# Why it needs disk, and why it is gated: the bump moves fsqlite, fsqlite-types
# and asupersync together, so cargo rebuilds the dependency graph rather than
# just this crate. Measured 2026-08-16, only 3 of 4,810 dep files in
# cass-repair-target still reference fsqlite-core-0.1.14 — generation 6's build
# was displaced by a later 0.1.5 rebuild in the same target, so there is nothing
# warm to reuse.
#
# The gate exists because cass refuses to START indexing below ~32 GiB and exits
# 14. A build that eats into that floor stops a running catch-up. Default floor
# here is deliberately well above it.
#
# Usage:
#   ./verify-fsqlite-pin.sh                  # refuses unless the disk clears
#   FSQLITE_VERIFY_FLOOR_GB=45 ./verify-fsqlite-pin.sh
set -uo pipefail

REPO="${CASS_REPO:-/Users/dalecarman/dev/coding_agent_session_search}"
REF="${FSQLITE_VERIFY_REF:-worktree-cass-gen5-honesty}"
FLOOR_GB="${FSQLITE_VERIFY_FLOOR_GB:-60}"
WORK="${FSQLITE_VERIFY_DIR:-/tmp/fsqlite-pin-verify}"
TARGET="${CARGO_TARGET_DIR:-/tmp/cass-fsqlite014-target}"
NIGHTLY="$HOME/.rustup/toolchains/nightly-aarch64-apple-darwin/bin"

free_gb() { df -g / | tail -1 | awk '{print $4}'; }

echo "=== frankensqlite pin verification ==="
echo "  ref        : $REF"
echo "  free now   : $(free_gb) GiB   (floor $FLOOR_GB GiB)"

if [ "$(free_gb)" -lt "$FLOOR_GB" ]; then
  cat >&2 <<MSG
REFUSING: $(free_gb) GiB free, floor is $FLOOR_GB GiB.

This is not a formality. A full dependency rebuild here will push free space
through cass's own ~32 GiB indexing floor, and any catch-up or index run then
fails at startup with exit 14. Bead jck92 records 116 GiB of stale cargo targets
in /tmp; freeing them is Dale's decision and NOT this script's to make.

Nothing has been built or written.
MSG
  exit 14
fi

if pgrep -f 'catchup-run.sh' >/dev/null 2>&1; then
  echo "WARNING: a catch-up indexer is running. This build will compete for CPU" >&2
  echo "         and disk. Continuing because the floor above already cleared." >&2
fi

[ -d "$NIGHTLY" ] || { echo "REFUSING: no nightly toolchain at $NIGHTLY" >&2; exit 1; }
export PATH="$NIGHTLY:$PATH"
export CARGO_TARGET_DIR="$TARGET"

# --local hardlinks objects, so the clone costs ~0 GiB on the same filesystem.
# A clone rather than a checkout so main and any running indexer are untouched.
if [ -d "$WORK" ]; then
  echo "  reusing existing clone at $WORK"
  cd "$WORK" || exit 1
  # A SHA is not a fetchable refspec, so this fails for the normal case and that
  # is fine — the --local clone already has every object. Output is kept rather
  # than piped into `tail`, because tailing a FAILING command shows the summary
  # and eats the diagnosis.
  git fetch origin "$REF" 2>&1 | sed 's/^/    fetch: /' || true
else
  if ! git clone --local --no-checkout "$REPO" "$WORK" 2>&1 | sed 's/^/    clone: /'; then
    echo "REFUSING: clone of $REPO into $WORK failed" >&2
    exit 1
  fi
  cd "$WORK" || exit 1
fi

# Resolve to a commit BEFORE detaching, and never hand `checkout` a bare name.
#
# In this clone the work branch exists only as a remote-tracking ref, so git
# DWIMs `checkout <name>` into `checkout -b <name> --track origin/<name>`, which
# collides with the flag we passed:
#     fatal: '--detach' cannot be used with '-b/-B/--orphan'
# A raw SHA cannot DWIM, so it takes the plain detach path. This bug shipped
# undetected because the disk gate above always refused before reaching it —
# generation 7 hit it the first time the gate ever cleared, and worked around it
# by passing a SHA from the caller. This is the fix.
if ! REF_SHA=$(git rev-parse --verify --quiet "${REF}^{commit}"); then
  if ! REF_SHA=$(git rev-parse --verify --quiet "origin/${REF}^{commit}"); then
    echo "REFUSING: cannot resolve '$REF' to a commit in $WORK." >&2
    echo "          Tried '$REF' and 'origin/$REF'. Pass a SHA or a ref this clone has." >&2
    exit 1
  fi
  echo "  resolved   : $REF -> origin/$REF"
fi
if ! git checkout --detach "$REF_SHA" 2>&1 | sed 's/^/    checkout: /'; then
  echo "REFUSING: checkout of $REF_SHA failed" >&2
  exit 1
fi

echo "  HEAD       : $(git rev-parse --short HEAD)"
# Read the RESOLVED version from Cargo.lock, not the requirement from
# Cargo.toml. `version = "0.1.5"` is a caret requirement and `"=0.1.14"` is an
# exact one, so a pattern over the requirement string answers a different
# question than "what will actually be linked".
pin=$(rg -N 'frankensqlite = ' Cargo.toml | head -1)
locked=$(rg -A2 '^name = "fsqlite"$' Cargo.lock | rg -N -o '^version = "[^"]+"' | head -1)
echo "  requirement: $pin"
echo "  locked     : fsqlite $locked"
case "$locked" in
  *'"0.1.14"'*) ;;
  '') echo "REFUSING: could not read fsqlite's version from Cargo.lock — unknown, not safe" >&2; exit 1 ;;
  *) echo "REFUSING: $REF locks fsqlite $locked, not 0.1.14 — wrong ref?" >&2; exit 1 ;;
esac

echo
echo "=== building and testing (this is the part that has never run) ==="
cargo test --lib -j 10; lib=$?
echo "--- lib rc=$lib   free=$(free_gb) GiB ---"

cargo test --test golden_robot_json -j 10; gold=$?
echo "--- golden rc=$gold   free=$(free_gb) GiB ---"

echo
echo "=== VERDICT ==="
echo "  lib suite    : rc=$lib"
echo "  golden suite : rc=$gold"
echo "  free after   : $(free_gb) GiB"
if [ "$lib" -eq 0 ] && [ "$gold" -eq 0 ]; then
  echo "  GREEN — the pin bump is safe to land. Merge $REF to main and push"
  echo "  main plus main:master, then redeploy by atomic rename (never cp over"
  echo "  the live path — a stale signature cache gives SIGKILL)."
  exit 0
fi
echo "  RED — do NOT land. The pin bump changes fsqlite, fsqlite-types and"
echo "  asupersync together; read which suite failed before blaming the engine."
exit 1
