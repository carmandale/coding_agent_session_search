<!-- plan:complete:v1 | harness: pi/claude-sonnet-4-6 | date: 2026-04-01T10:23:12Z -->

# Planning Transcript — Spec 011

**Session**: 2026-03-31  
**Planner**: FastNova (pi/claude-sonnet-4-6)  
**Challenger**: YoungNova (crew-challenger/claude-opus-4-6)

---

## Round 1: Research Findings → Challenge

**FastNova presented**:
- Copy-forward approach: `git checkout upstream/main -- src/...` then restore watchdog
- Path dep → git dep conversion for asupersync, frankensqlite, franken-agent-detection
- Version `0.2.6-gj.1` 
- 5 watchdog wiring sites in lib.rs
- Drop sessions.rs and message_render.rs (ratatui → ftui migration)

**YoungNova challenged** [PHASE:challenge] — verified against actual source files:

### C1 (CRITICAL): `state_meta_json` signature changed
Upstream's `state_meta_json` (line 4570) takes 4 args: `(data_dir, db_path, stale_threshold, allow_db_open: bool)`. Our watchdog.rs test at line 941 calls it with 3 args. This is a guaranteed compile failure after the copy-forward. **This is a 6th wiring site — missed in original analysis.**

Evidence: `grep -n "fn state_meta_json" upstream/main:src/lib.rs` → line 4570 with 4 args. `grep -n "state_meta_json" src/watchdog.rs` → line 941 with 3 args.

### C2 (CRITICAL): Missing `libc` dependency
Upstream Cargo.toml does NOT include `libc`. watchdog.rs uses 9 separate `libc::*` symbols: `kill`, `flock`, `getuid`, `SIGTERM`, `SIGKILL`, `ESRCH`, `EPERM`, `LOCK_EX`, `LOCK_NB`. After taking upstream's Cargo.toml without adding `libc = "*"`, the build will fail.

Evidence: `grep -n "libc::" src/watchdog.rs` returned 9 hits. `grep "libc" upstream/main:Cargo.toml` → no results.

### C3 (CRITICAL): `fsqlite-types` dev-dep also needs git redirect
Original analysis missed the dev-dep: `fsqlite-types = { path = "../frankensqlite/crates/fsqlite-types" }`. Also a path dep that needs `git = "https://github.com/Dicklesworthstone/frankensqlite", package = "fsqlite-types"`.

### S1 (SIGNIFICANT): SemVer pre-release ordering is backwards
`0.2.6-gj.1` is a pre-release. In SemVer, `0.2.6-gj.1 < 0.2.6`. If upstream bumps to 0.2.6, our version appears older. Correct choice: `0.2.7-gj.1` — above upstream's 0.2.5 and any immediate 0.2.6.

### S2: Step 1+2 ordering is a footgun
`git checkout upstream/main -- src/` deletes `src/watchdog.rs`. Step 2 restores it. **Chain the commands to prevent accidental commit between the two.** Also: back up watchdog.rs to /tmp first.

### S3: asupersync needs a rev pin
Without a rev pin, `asupersync = { git = "..." }` pulls HEAD at every `cargo update`, which could break silently. Pin to current HEAD: `rev = "95476b32"`. (Verified: YoungNova cloned asupersync, confirmed it's a workspace with franken_kernel, franken_evidence, franken_decision as members — sub-crates resolve from same git repo without needing [patch].)

### S4: Vendored OpenSSL adds 2-5 minutes to first build
Upstream has `openssl = { version = "*", features = ["vendored"] }`. Requires C compiler + Perl. Document in plan.

### S5: `state_meta_json` insertion is surgical
Upstream's version is 200+ lines. The "watchdog" JSON block must be inserted between the `"pending"` block and the `"_meta"` block. Simple pattern-matching on `"_meta"` as anchor works since the watchdog block has no dependencies on local variables.

---

## Round 2: Approval

**FastNova verified** [PHASE:approved]:
- C1 confirmed: upstream signature is `(data_dir, db_path, stale_threshold, allow_db_open: bool)`
- C2 confirmed: 9 libc usages in watchdog.rs
- C3 confirmed: fsqlite-types must be in dev-deps
- S1 agreed: `0.2.7-gj.1` is correct
- S2 agreed: chain the commands
- S3: asupersync rev = 95476b32 added to plan
- S4-S5 noted

**YoungNova approved** [COMPLETE] with summary of 7 issues surfaced (3 critical build-breakers, 4 significant improvements).

---

## Final Delta Summary

The challenger surfaced changes that turned a 3-critical-failure plan into a solid one:

| Issue | Severity | Impact |
|-------|----------|--------|
| 6th wiring site: state_meta_json 3→4 args | CRITICAL | Build failure without fix |
| libc dep missing from Cargo.toml | CRITICAL | Build failure without fix |
| fsqlite-types dev-dep conversion missing | CRITICAL | Build failure without fix |
| Version: 0.2.7-gj.1 not 0.2.6-gj.1 | Significant | Wrong SemVer ordering |
| Chain git checkouts + backup first | Significant | Silent data loss risk |
| asupersync rev pin | Significant | Silent breakage risk |
| Vendored OpenSSL build time warning | Minor | User surprise |
