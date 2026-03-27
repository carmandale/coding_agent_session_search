#!/usr/bin/env bash
# dev-install.sh — One-shot local developer setup for cass
#
# Builds from source, wires ~/.local/bin/cass, activates git hooks,
# and reloads the launchd watcher plist. Safe to re-run at any time.
#
# Usage:
#   ./dev-install.sh           # full install
#   ./dev-install.sh --no-launchd   # skip plist reload (e.g. on remote)

set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOCAL_BIN="$HOME/.local/bin"
CARGO_BIN="$HOME/.cargo/bin/cass"
LOCAL_LINK="$LOCAL_BIN/cass"
WATCHER_PLIST="$HOME/Library/LaunchAgents/com.cass.index-watch.plist"
RELOAD_LAUNCHD=1

for arg in "$@"; do
  case "$arg" in
    --no-launchd) RELOAD_LAUNCHD=0 ;;
  esac
done

ok()   { echo -e "\033[0;32m✓\033[0m $*"; }
info() { echo -e "\033[0;34m→\033[0m $*"; }
warn() { echo -e "\033[1;33m⚠\033[0m $*"; }
err()  { echo -e "\033[0;31m✗\033[0m $*" >&2; }

echo ""
echo "╔══════════════════════════════════════════════════════╗"
echo "║           cass — local developer install             ║"
echo "╚══════════════════════════════════════════════════════╝"
echo ""

# ── 1. Build & install to ~/.cargo/bin/cass ───────────────────────────────────
info "Building cass from source (cargo install --path .)…"
cd "$REPO_DIR"
cargo install --path . --quiet
ok "Installed to $CARGO_BIN ($(cass --version 2>/dev/null || "$CARGO_BIN" --version))"

# ── 2. Wire ~/.local/bin/cass → ~/.cargo/bin/cass ─────────────────────────────
mkdir -p "$LOCAL_BIN"
if [[ -L "$LOCAL_LINK" ]]; then
  current_target="$(readlink "$LOCAL_LINK")"
  if [[ "$current_target" == "$CARGO_BIN" ]]; then
    ok "$LOCAL_LINK already correct"
  else
    ln -sf "$CARGO_BIN" "$LOCAL_LINK"
    ok "Updated symlink: $LOCAL_LINK → $CARGO_BIN  (was → $current_target)"
  fi
elif [[ -e "$LOCAL_LINK" ]]; then
  warn "$LOCAL_LINK exists but is not a symlink — backing up and replacing"
  mv "$LOCAL_LINK" "${LOCAL_LINK}.backup.$(date +%Y%m%d-%H%M%S)"
  ln -sf "$CARGO_BIN" "$LOCAL_LINK"
  ok "Symlink created: $LOCAL_LINK → $CARGO_BIN"
else
  ln -sf "$CARGO_BIN" "$LOCAL_LINK"
  ok "Symlink created: $LOCAL_LINK → $CARGO_BIN"
fi

# ── 3. Activate git hooks (core.hooksPath) ───────────────────────────────────
HOOKS_DIR="$REPO_DIR/hooks"
if [[ -d "$HOOKS_DIR" ]]; then
  git -C "$REPO_DIR" config core.hooksPath hooks
  ok "Git hooks activated (core.hooksPath=hooks)"
else
  warn "hooks/ dir not found — skipping core.hooksPath (run dev-install.sh again after hooks/ is created)"
fi

# ── 4. Reload launchd watcher plist ──────────────────────────────────────────
if [[ "$RELOAD_LAUNCHD" -eq 1 ]]; then
  if [[ -f "$WATCHER_PLIST" ]]; then
    info "Reloading launchd watcher plist…"
    launchctl unload "$WATCHER_PLIST" 2>/dev/null || true
    sleep 1
    launchctl load "$WATCHER_PLIST"
    sleep 2
    # Verify watcher came up
    if pgrep -f "cass index --watch" >/dev/null 2>&1; then
      ok "Watcher is running (PID $(pgrep -f 'cass index --watch'))"
    else
      warn "Watcher plist loaded but process not yet visible — check: tail -20 ~/Library/Logs/cass-index-watch.log"
    fi
  else
    warn "Watcher plist not found at $WATCHER_PLIST — skipping reload"
    info "To install plists: cass watchdog install"
  fi
else
  info "Skipping launchd reload (--no-launchd)"
fi

# ── 5. Sanity check ──────────────────────────────────────────────────────────
echo ""
info "Sanity check…"
"$LOCAL_LINK" health --json 2>/dev/null | python3 -c "
import sys, json
d = json.load(sys.stdin)
healthy = d.get('healthy', False)
convs = d.get('state', {}).get('database', {}).get('conversations', '?')
watch = d.get('state', {}).get('pending', {}).get('watch_active', False)
print(f'  healthy={healthy}  conversations={convs}  watch_active={watch}')
" 2>/dev/null || warn "Health check failed — run: cass health --json"

echo ""
ok "dev-install complete. cass is at: $LOCAL_LINK"
echo ""
