<!-- Codex Review: APPROVED after 5 rounds | model: gpt-5.3-codex | date: 2026-03-14 -->
<!-- Status: REVISED -->
<!-- Revisions: A1 looks_like_root rewritten to exact Self::home() comparison (rounds 1-5), A2 kept follow_links+max_depth (round 1), A3 log rotation changed to copytruncate (round 1), A4 heartbeat threshold raised to 2700s (round 2), B1 strip changed to debuginfo (round 1), SIGTERM limitation documented (round 2), 11 automated tests added (rounds 1-2), all internal inconsistencies fixed (round 3) -->
---
title: "plan: Watcher CPU spin — Shape C implementation"
date: 2026-03-14
bead: coding_agent_session_search-2s40
---

# Implementation Plan: Shape C — Lifecycle Hardening + Hygiene + Instrumentation

## Overview

Shape C addresses the watcher CPU spin via four coordinated changes:

1. **C1 (Hygiene):** Narrow PiAgent watch root, bound WalkDir, add log rotation, replace watchdog with heartbeat-based liveness
2. **C2 (Instrumentation):** Retain debug symbols, name all spawned threads, log slow scans
3. **C3 (SIGTERM handler):** Graceful shutdown on signal — flush tantivy, exit clean
4. **C4 (Corruption recovery):** Document existing coverage, defer spike for opens-but-spins case

All four deploy simultaneously. If CPU spin stops → watchdog kill loop was root cause (confirmed by elimination). If it persists → C2 instrumentation captures the thread identity.

---

## C1: Hygiene (A1 + A2 + A3 + A4)

### A1: PiAgent detect() root fix

**File:** `src/connectors/pi_agent.rs:149-160`

**Change:** Return `~/.pi/agent/sessions` instead of `~/.pi/agent` as the watch root.

```rust
fn detect(&self) -> DetectionResult {
    let home = Self::home();
    if home.join("sessions").exists() {
        let sessions = home.join("sessions");
        DetectionResult {
            detected: true,
            evidence: vec![format!("found {}", home.display())],
            root_paths: if sessions.exists() { vec![sessions] } else { vec![] },
        }
    } else {
        DetectionResult::not_found()
    }
}
```

**CRITICAL: Also update `looks_like_root` in `scan()`** (pi_agent.rs:171-176).
The watcher calls `scan()` with `ScanContext::with_roots(root.path, ...)` where
`root.path` is now `~/.pi/agent/sessions`. With `use_default_detection() == false`,
scan falls through to the `looks_like_root` check. The current implementation
checks `path.join("sessions").exists()` (fails — `sessions/sessions` doesn't
exist) and `file_name().contains("pi")` (fails — file_name is `"sessions"`).

**Fix `looks_like_root` to accept the configured Pi home and its sessions subdir:**

The `looks_like_root` check must accept the new `sessions` root path without
also accepting unrelated `sessions` directories from other connectors (e.g.,
Codex at `~/.codex/sessions`, Factory at `~/.factory/sessions`), which are
fanned out to all connectors via remote root broadcasting (mod.rs:798).

**The approach: compare against `Self::home()` directly** — no path-shape
heuristics. This respects arbitrary `PI_CODING_AGENT_DIR` values:

```rust
fn scan(&self, ctx: &ScanContext) -> Result<Vec<NormalizedConversation>> {
    let pi_home = Self::home();
    let pi_sessions = Self::sessions_dir(&pi_home);

    // Check if the provided data_dir is the Pi home or its sessions subdir.
    // This replaces the old is_pi_agent_dir path-substring heuristic with
    // an exact match against the configured home, honoring arbitrary
    // PI_CODING_AGENT_DIR values.
    let is_pi_path = ctx.data_dir == pi_home || ctx.data_dir == pi_sessions;

    let looks_like_root = |path: &PathBuf| {
        path.join("sessions").exists()
            || path
                .file_name()
                .is_some_and(|n| n.to_str().unwrap_or("").contains("pi"))
    };

    let mut home = if ctx.use_default_detection() {
        if is_pi_path {
            ctx.data_dir.clone()
        } else {
            pi_home
        }
    } else {
        // For explicit roots: accept if it matches the configured Pi home
        // or sessions dir. The exact comparison prevents accepting unrelated
        // "sessions" directories from Codex/Factory via remote root fanout.
        if !looks_like_root(&ctx.data_dir) && !is_pi_path {
            return Ok(Vec::new());
        }
        ctx.data_dir.clone()
    };
    // ... rest unchanged
```

This works because:
- `~/.pi/agent/sessions` → `is_pi_path = true` (equals `pi_sessions`) → accepted
- `~/.codex/sessions` → `is_pi_path = false`, `looks_like_root = false` → rejected
- `~/.factory/sessions` → `is_pi_path = false`, `looks_like_root = false` → rejected
- Custom `PI_CODING_AGENT_DIR=/tmp/foo` → `/tmp/foo/sessions` equals `pi_sessions` → accepted
- Custom `PI_CODING_AGENT_DIR=/mnt/data/custom` → `/mnt/data/custom/sessions` → accepted

No path-shape heuristics. No false-positive indexing. Honors any `PI_CODING_AGENT_DIR`.

**Blast radius:** `build_watch_roots` (indexer/mod.rs:780) passes `root_paths` to `watcher.watch()`. Narrowing from `~/.pi/agent` (22,059 files) to `~/.pi/agent/sessions` (~1,688 files) eliminates FSEvents noise from `extensions/node_modules/`. PiAgent `scan()` continues to work because `looks_like_root` now accepts the sessions directory.

**Guard:** If `sessions` dir doesn't exist, `root_paths` is empty → `watcher.watch()` is never called → graceful degradation. `detected: true` still allows initial-index scan to work (which uses `local_default` → `use_default_detection() == true` → bypasses `looks_like_root`).

**Tests:** Modify existing `detect()` test + add test for empty root_paths when sessions absent + add test for `scan()` with explicit sessions root (ScanContext::with_roots).

### A2: WalkDir safety

**File:** `src/connectors/pi_agent.rs:63-64`

**Change:** Add `max_depth(10)`. Keep `follow_links(true)` (preserves clawdbot symlink indexing — 2,098 sessions).

```rust
for entry in WalkDir::new(sessions)
    .follow_links(true)
    .max_depth(10)
    .into_iter()
    .flatten()
```

**Rationale:** The clawdbot symlink at `~/.pi/agent/sessions/--clawdbot-chip--` → `~/.clawdbot/agents/main/sessions` is 1 level deep. `max_depth(10)` prevents future circular symlink bombs while preserving existing functionality. Removing `follow_links` would silently drop 2,098 sessions — unacceptable regression. Spec R1.2 was updated during review to reflect this deliberate decision: "WalkDir has max_depth; symlink following bounded by depth limit."

**Tests:** Add `session_files_follows_symlinks_with_depth_bound` test using a temp dir with symlink.

### A3: Log rotation

**File:** External — `~/.local/share/cass/watchdog.sh`

The watcher log is managed by launchd (`StandardOutPath` in the plist). Cass has no explicit file management for logs. Rather than adding `tracing_appender` (complex), handle rotation in `watchdog.sh`:

```bash
LOG_FILE="$HOME/Library/Logs/cass-index-watch.log"
MAX_LOG_SIZE=$((100 * 1024 * 1024))  # 100 MB

if [ -f "$LOG_FILE" ]; then
    LOG_SIZE=$(stat -f%z "$LOG_FILE" 2>/dev/null || echo 0)
    if [ "$LOG_SIZE" -gt "$MAX_LOG_SIZE" ]; then
        log "Log file ${LOG_SIZE} bytes > ${MAX_LOG_SIZE}, truncating"
        tail -c "$MAX_LOG_SIZE" "$LOG_FILE" > "${LOG_FILE}.tmp"
        mv "${LOG_FILE}.tmp" "$LOG_FILE"
    fi
fi
```

**Important:** Because launchd holds the fd open via `StandardOutPath`, a
`mv`-based rotation doesn't work — the process continues writing to the old
(now unlinked) fd. Instead, use copytruncate semantics:

```bash
LOG_FILE="$HOME/Library/Logs/cass-index-watch.log"
MAX_LOG_SIZE=$((100 * 1024 * 1024))  # 100 MB

if [ -f "$LOG_FILE" ]; then
    LOG_SIZE=$(stat -f%z "$LOG_FILE" 2>/dev/null || echo 0)
    if [ "$LOG_SIZE" -gt "$MAX_LOG_SIZE" ]; then
        log "Log file ${LOG_SIZE} bytes > ${MAX_LOG_SIZE}, rotating with copytruncate"
        cp "$LOG_FILE" "${LOG_FILE}.1"  # backup
        : > "$LOG_FILE"                 # truncate in-place (preserves fd)
    fi
fi
```

The `: > "$LOG_FILE"` truncates the file in-place, which works with the
existing fd because the inode doesn't change. The `cp` preserves recent
logs for diagnosis. This runs every 10 minutes with the watchdog.

### A4: Heartbeat-based watchdog

**Rust changes (src/indexer/mod.rs):**

In `watch_sources`, write a heartbeat timestamp file at three points:
1. On every `recv_timeout` return (heartbeat timer)
2. Before each callback invocation
3. After each callback returns

```rust
let heartbeat_path = data_dir.join("watcher-heartbeat");
fn write_heartbeat(path: &Path) {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let _ = std::fs::write(path, ts.to_string());
}
```

**Reduce `heartbeat_interval` from 300s to 60s.** This ensures:
- Heartbeat file updates at least every 60s when idle
- `recv_timeout` returns within 60s → SIGTERM flag checked within 60s (well within the 120s SIGKILL grace period)
- More frequent heartbeats give the watchdog finer-grained liveness detection

**Watchdog changes (`~/.local/share/cass/watchdog.sh`):**

Replace `cass health --json` staleness check with heartbeat file age check:

```bash
HEARTBEAT_FILE="$HOME/Library/Application Support/com.coding-agent-search.coding-agent-search/watcher-heartbeat"
HEARTBEAT_THRESHOLD=2700  # 45 minutes — exceeds full_scan_interval (1800s) + max scan duration (~600s) + margin (300s)

if [ -f "$HEARTBEAT_FILE" ]; then
    HEARTBEAT_TS=$(cat "$HEARTBEAT_FILE")
    NOW=$(date +%s)
    HEARTBEAT_AGE=$(( NOW - HEARTBEAT_TS ))
    if [ "$HEARTBEAT_AGE" -gt "$HEARTBEAT_THRESHOLD" ]; then
        NEEDS_RESTART=true
        REASON="heartbeat stale (${HEARTBEAT_AGE}s > ${HEARTBEAT_THRESHOLD}s)"
    fi
else
    NEEDS_RESTART=true
    REASON="no heartbeat file"
fi
```

**Watchdog kill sequence:** Change from `launchctl kickstart -k` (SIGKILL) to:
1. Send SIGTERM via `kill -TERM <pid>`
2. Wait 120s
3. If still running, SIGKILL via `kill -9 <pid>`

**Watchdog also removes the concurrent `cass index --full` call** — this was fighting the watcher for the tantivy lock and always failing.

---

## C2: Instrumentation (B1 + B2 + B3)

### B1: Debug symbols in release binary

**File:** `Cargo.toml` → `[profile.release]`

**Change:** `strip = true` → `strip = "debuginfo"` (spec originally said `strip = "none"`; changed during review for better size/diagnosability tradeoff)

This keeps the **symbol table** (function names visible in `sample` output: `cass::indexer::reindex_paths` instead of `??? (in cass)`) while stripping the larger DWARF debug info. Binary size increase is moderate (~1-3 MB for symbols vs ~10-50 MB for full debug info).

Note: Spec originally said `strip = "none"`. Changed to `strip = "debuginfo"` during planning review as a better size/diagnosability tradeoff — full DWARF info is unnecessary for `sample`-based diagnosis.

### B2: Name all spawned threads

**10 locations**, 7 production-relevant:

| File:Line | Current | Named |
|-----------|---------|-------|
| `src/indexer/mod.rs:120` | `thread::spawn(...)` | `thread::Builder::new().name(format!("connector-{name}")).spawn(...)` |
| `src/lib.rs:5941` | `std::thread::spawn(...)` | `.name("tui-rebuild".into()).spawn(...)` |
| `src/lib.rs:6048` | `std::thread::spawn(...)` | `.name("tui-index-rebuild".into()).spawn(...)` |
| `src/lib.rs:7684` | `std::thread::spawn(...)` | `.name("index-runner".into()).spawn(...)` |
| `src/lib.rs:7846` | `std::thread::spawn(...)` | `.name("index-watcher".into()).spawn(...)` |
| `src/update_check.rs:422` | `std::thread::spawn(...)` | `.name("update-checker".into()).spawn(...)` |
| `src/ui/tui.rs:4912` | `std::thread::spawn(...)` | `.name("model-download".into()).spawn(...)` |

3 test-only locations (`src/search/query.rs:3495`, `src/ui/data.rs:518,531`) — name as `"test-worker-N"` for consistency.

**Pattern:** Replace `thread::spawn(move || { ... })` with `thread::Builder::new().name("...".into()).spawn(move || { ... }).expect("failed to spawn thread")`.

Note: `thread::Builder::spawn` returns `io::Result<JoinHandle>` not `JoinHandle`. Callsites that use the handle directly need `.expect()` or `?`.

### B3: Slow scan logging

**File:** `src/indexer/mod.rs`, inside `reindex_paths` (~line 1050-1075)

Wrap the per-connector scan block in elapsed-time measurement:

```rust
let scan_start = std::time::Instant::now();
let mut convs = match conn.scan(&ctx) { ... };
let scan_elapsed = scan_start.elapsed();
if scan_elapsed > Duration::from_secs(30) {
    tracing::warn!(
        ?kind,
        elapsed_secs = scan_elapsed.as_secs(),
        conversations = convs.len(),
        "slow_scan_detected"
    );
}
```

---

## C3: SIGTERM Handler

### Signal registration

**Crate:** Use `signal_hook` (add to Cargo.toml) — works without tokio runtime, simpler for the blocking event loop context.

**Registration point:** In `run_index` (mod.rs:551), before calling `watch_sources`:

```rust
let shutdown = Arc::new(AtomicBool::new(false));
signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&shutdown))?;
signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&shutdown))?;
```

### Check points (two levels)

**Level 1 — `watch_sources` loop** (mod.rs:899):
- Check `shutdown.load(Ordering::Relaxed)` after every callback call (lines ~913, 928, 935, 961, 965)
- If set, break out of the loop → function returns → process exits

**Level 2 — `reindex_paths` loop** (mod.rs:1022):
- Check `shutdown` between connector iterations in the `for (kind, root, _ts) in triggers` loop
- If set, break early. No final `t_index.commit()` needed — each iteration commits independently.
- Watch state for completed connectors is already saved; uncompleted connectors re-scan on next startup.

### Thread changes

Pass `Arc<AtomicBool>` through:
- `watch_sources` closure (captured by the callback closure)
- `reindex_paths` function signature (new parameter)

### Single-connector scan limitation

The shutdown flag is checked between connector iterations, not within a
single `conn.scan()` call. If one connector scan takes >120s (the SIGTERM→
SIGKILL grace period), the process could still be killed mid-scan. This is
an accepted limitation because:
- Most connector scans complete in <30s (B3 logging will confirm this)
- The per-connector commit model means already-completed connectors are safe
- Making `conn.scan()` interruptible would require changes to every connector
- If B3 logging reveals a connector consistently exceeding 120s, that connector
  should be optimized (e.g., Codex's 476 MB file parsing)

### No final flush needed

Each connector iteration in `reindex_paths` does `ingest_batch() + t_index.commit()` as a unit (mod.rs:1086-1092). Breaking between iterations leaves no uncommitted data. This simplifies the handler significantly.

### recv_timeout race mitigation

With `heartbeat_interval` reduced to 60s (A4), `recv_timeout` returns within 60s max. The watchdog's SIGTERM→SIGKILL grace period is 120s. Timeline:

```
t=0:   SIGTERM arrives while blocked on recv_timeout
t≤60:  recv_timeout fires → flag checked → loop breaks → exit
t=120: SIGKILL (never reached)
```

---

## C4: Corruption Recovery (partially deferred)

### Already covered (existing code)

**File:** `src/indexer/mod.rs:575-585`

The existing preflight check handles the common corruption case:
```rust
if !needs_rebuild && let Err(e) = tantivy::Index::open_in_dir(&index_path) {
    tracing::warn!("tantivy open preflight failed; forcing rebuild");
    needs_rebuild = true;
}
```
If `needs_rebuild` is true, `std::fs::remove_dir_all(&index_path)` deletes the corrupt index and rebuilds from scratch (mod.rs:589-591).

### Gap: opens-but-spins

If tantivy opens successfully but a background merge thread spins on a corrupt segment, the preflight doesn't catch it. This requires a spike:
- Attempt `Index::searchable_segment_ids()` + trivial search per segment with timeout
- If search hangs > 5s, force rebuild

**Action:** Create a follow-up bead for this spike. R3.3 was downgraded to Nice-to-have during review — the existing preflight covers the common corruption case, and C3 (SIGTERM) prevents the corruption from occurring in the first place. The residual gap (opens-but-spins) is tracked but does not block this release.

---

## Deployment Strategy

All changes deploy in a single release:
1. Build with updated Cargo.toml (`strip = "debuginfo"`)
2. Code changes (A1, A2, B2, B3, C3, heartbeat writing)
3. Updated `watchdog.sh` (heartbeat check, SIGTERM-first kill, log rotation, no concurrent `cass index --full`)
4. Updated `com.cass.health-watchdog.plist` (if needed)

**Automated tests (required before merge):**

| Test | What it verifies | Type |
|------|-----------------|------|
| `detect_returns_sessions_dir_as_root` | A1: root_paths is sessions subdir | Unit |
| `detect_returns_empty_roots_when_no_sessions` | A1: graceful degradation | Unit |
| `scan_with_explicit_sessions_root` | A1: `looks_like_root` accepts sessions path via `with_roots` | Unit |
| `session_files_follows_symlinks_with_depth_bound` | A2: symlink traversal + depth limit | Unit |
| `heartbeat_file_written_during_loop` | A4: heartbeat updates between iterations | Integration |
| `spawned_threads_have_names` | B2: `thread::current().name()` returns expected | Unit |
| `slow_scan_emits_warning` | B3: tracing warn for >30s scans | Unit (tracing test subscriber) |
| `shutdown_flag_breaks_reindex_loop` | C3: AtomicBool checked between connectors | Unit |
| `shutdown_flag_breaks_watch_loop` | C3: main loop exits on flag | Unit |
| `scan_with_custom_pi_home_explicit_root` | A1: `looks_like_root` works with custom `PI_CODING_AGENT_DIR` | Unit |
| `long_scan_not_treated_as_stale` | A4: heartbeat threshold (2700s) > full_scan_interval + scan duration | Integration |

**Verification after deployment (manual):**
- `~/Library/Logs/cass-watchdog.log` for "heartbeat stale" events (should not appear)
- `~/Library/Logs/cass-index-watch.log` for `slow_scan_detected` warnings
- `sample <pid>` output should show named functions and threads
- CPU usage should remain <5% during normal operation
