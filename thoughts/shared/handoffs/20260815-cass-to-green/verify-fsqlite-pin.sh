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
  git fetch origin "$REF" 2>&1 | tail -1
else
  git clone --local --no-checkout "$REPO" "$WORK" 2>&1 | tail -1 || exit 1
  cd "$WORK" || exit 1
fi
git checkout --detach "$REF" 2>&1 | tail -1 || exit 1

echo "  HEAD       : $(git rev-parse --short HEAD)"
pin=$(rg -N 'frankensqlite = ' Cargo.toml | head -1)
echo "  pin        : $pin"
case "$pin" in
  *'=0.1.14'*) ;;
  *) echo "REFUSING: $REF does not pin fsqlite =0.1.14 — wrong ref?" >&2; exit 1 ;;
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
