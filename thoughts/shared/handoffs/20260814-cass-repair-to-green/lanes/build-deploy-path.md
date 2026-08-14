# Lane: build-deploy-path

**Role:** read-only grounding. No cargo run, no mutation, no cass invocation beyond version/help-style
reads. Writes only this file.

**Scope:** the mechanics of getting a corrected `cass` binary onto this Mac safely — build command,
toolchain, deploy/rollback ritual, install.sh side effects, CI gates.

All claims marked MEASURED (I ran the command shown, today, 2026-08-14, on this Mac) or INFERRED
(reasoned from source/docs, not directly executed).

---

## 0. Self-correction on the record

While checking install.sh for side effects, an `rg ... install.sh` call with no `cd`/absolute path ran
against the WRONG file — this tool's bash cwd resets to `/Users/dalecarman/.agent-config` between calls
(per the harness contract), so a command with no explicit `cd` silently searched agent-config's own
1700-line `install.sh` (Gemini/Codex/Claude hook wiring) and returned content that looked like it came
from cass's installer. Caught by a `pwd` sanity check before publishing the finding. Every command below
either `cd`s into the repo first or uses an absolute path; the one genuine full `Read` of cass's
`install.sh` (§3) used the absolute path throughout and is unaffected.

---

## 1. Cargo.toml / build.rs

MEASURED (`Read` on both files in full).

- Crate name: `coding-agent-search` (Cargo.toml:2). Binary name: `cass` (`[[bin]] name = "cass"`,
  Cargo.toml:184, `path = "src/main.rs"`). A second bin, `cass-pages-perf-bundle`, is unrelated to the
  deploy path.
- `default = ["qr", "encryption", "semantic"]` (Cargo.toml:143). `qr` pulls in `qrcode`+`image`.
  `encryption` is dep-free (gates HTML export crypto, deps already present). `semantic` pulls
  `fastembed` (prebuilt ONNX Runtime download) + `frankensearch/fastembed-reranker`. A fourth feature,
  `backtrace`, is also dep-free and NOT in the default set. A fifth, `strict-path-dep-validation`, gates
  build.rs's sibling-repo path validation and is intentionally opt-in.
- **build.rs DOES embed a git revision, confirmed at commit `f619a74d2d69e11b24b325cfecd2177af7ef078d`**
  ("fix(build): embed git revision in cass identity", 2026-08-12, 5 lines in build.rs + 15 in lib.rs).
  Mechanism: `emit_vergen_metadata()` (build.rs:701-719) builds a `vergen_gix::GixBuilder` and calls
  `.sha(false)`. I read the vendored source at
  `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/vergen-gix-9.1.0/src/gix/mod.rs:280` —
  **`sha(&mut self, short: bool)`, so `.sha(false)` means "emit the SHA, non-short" (i.e. the full
  40-char SHA), not "disable it."** It is consumed via `option_env!("VERGEN_GIT_SHA")` at three sites:
  `src/lib.rs:191` (the `cass --version` long-form banner: `"{version}\ngit commit: {sha}"`),
  `src/lib.rs:40777` and `src/lib.rs:69824` (doctor-run identifiers, per `src/doctor_runs.rs:78`).
- **Does a dirty tree change the embedded SHA? No — verified from source, not just inferred.** The
  `GixBuilder` in build.rs never calls `.dirty(...)`, and `derive_builder`'s generated `Default` leaves
  that field `false`/unset. So `emit_vergen_metadata()` only ever emits `VERGEN_GIT_SHA` from HEAD; a
  dirty working tree does not append `-dirty` or change the SHA at all. **Practical hazard:** if the
  coordinator builds with uncommitted changes staged, the binary will silently claim to be built from
  the parent commit while actually containing the diff — `cass --version` cannot be trusted to prove
  "no local modifications," only "built while HEAD was at this SHA." Build from a committed HEAD, not a
  dirty tree, if the version banner is meant to be evidence.
- The *other* git-state check in build.rs (`validate_strict_git_state`, build.rs:591) — the one that DOES
  check dirty — is unrelated to vergen. It only runs when the `strict-path-dep-validation` feature is
  enabled (build.rs:298 `strict_enabled`, gated by that named feature), which is off by default and not
  needed for this deploy. It won't fire on a plain `cargo build --release`.
- Current repo state relevant to this: `git status --short` at the top of this session shows only
  untracked non-source paths (`.agent-state/`, `.grok/`, a goalbuddy board dir, `solo.yml`) — no tracked
  `src/`, `Cargo.toml`, or `build.rs` changes are pending. A build right now embeds HEAD's real SHA
  cleanly.

## 2. Release Profile

MEASURED (`Read` on AGENTS.md:147-178, cross-checked against Cargo.toml:193-198 — byte-for-byte match).

```toml
[profile.release]
lto = true          # fat LTO (whole-graph), not thin
codegen-units = 1
strip = true
panic = "abort"
opt-level = 3
```

The installed binary is 51,834,784 bytes, matching `cass.pre-coverage-floor-20260601` exactly (MEASURED,
§4). This IS a `[profile.release]` build — no other profile in the file produces a 51MB *stripped*
binary; `profiling` and `release-perf` both set `strip = false` and would be visibly larger with symbols
retained. **Build command is plain `cargo build --release`** — nothing in the coordinator's task requires
touching `--profile`.

**Wall-clock cost — do not build clean.** I did not run cargo (forbidden), so the number below is
INFERRED from the state of the existing `target/` directory, not measured directly, but the inference is
tight:

- `Cargo.lock` has 823 packages (MEASURED: `rg -c '^name = ' Cargo.lock`).
- `target/` already exists, 2.4 GB, `target/release/deps/` holds 2265 files (MEASURED, `du -sh` + `ls |
  wc -l`), and `target/release/cass` sitting there right now is **51,900,976 bytes, mtime Aug 10 20:22**
  — byte-identical in size to the preserved `cass.coverage-floor-fix-20260810` specimen. This is the
  actual leftover build artifact from the last full build (bead c7yaw: "Built from HEAD (ff3d7125)").
- Since that build, only **6 commits** landed on main, touching **4 files, 21 lines total**: `Cargo.lock`
  (+1), `Cargo.toml` (+1), `build.rs` (+5), `src/lib.rs` (+15) — all from the vergen commit `f619a74d`
  (MEASURED: `git log --oneline ff3d7125..d5cea071 | wc -l` = 6; `git diff --stat ff3d7125 d5cea071 --
  src/ Cargo.toml Cargo.lock build.rs`). The coverage-floor fix itself (`e3ed01f0`) predates that build,
  so it's already reflected in the cached `target/`.
- **Recommendation: build with `CARGO_TARGET_DIR` pointed at this repo's existing `target/` (i.e. do
  NOT override it to a fresh empty path).** Cargo's fingerprinting will see build.rs and lib.rs changed,
  re-run build.rs (fast — the vergen git-info step just shells `git`), recompile the one
  `coding-agent-search` lib target (a very large single file — line numbers in it run past 69,000, so
  this alone is nontrivial even under `-O3`), and then pay the fat-LTO relink across all 823 crates'
  cached IR. That relink is the dominant cost of *any* build under `lto = true, codegen-units = 1`, so
  even this incremental path is not instant — informally, low-to-mid single-digit minutes is a reasonable
  expectation on M-series hardware, not the 15-45+ minutes a from-scratch build of this dependency graph
  (openssl vendored, ring, syntect, hnsw_rs, a pinned frankentui git rev, the ONNX-adjacent fastembed
  path) would cost. **I have no directly measured minute-figure for either case on this machine** — flag
  this as the one number in this report that is estimate, not fact, and treat it as a floor: reusing the
  cache is strictly faster than not, by an amount bounded below by "recompile 2265 dep artifacts from
  zero," which nothing on this Mac needs to pay for a 21-line diff.
- No stale lock: `target/.cargo-lock` does not exist, and `ps aux | rg 'cargo|rustc'` returned nothing —
  the directory is idle, safe to build into right now (MEASURED).
- If the coordinator genuinely needs isolation from a *concurrent* build (not from me — I'm not running
  cargo), the right move is to **copy** `target/` to the isolated path first (`cp -a target
  /path/to/isolated-target`) rather than pointing `CARGO_TARGET_DIR` at an empty directory, so the cache
  survives the isolation.

## 3. Install path

MEASURED (full `Read` of `/Users/dalecarman/dev/coding_agent_session_search/install.sh`, 576 lines,
absolute path, confirmed via `md5`/`wc -l` cross-check after the cwd mishap in §0).

**install.sh is NOT how the currently-installed binary, or last session's deploy, got there, and should
not be used for this deploy either.** Its default path (`FROM_SOURCE=0`) downloads a tagged **GitHub
release** tarball (`https://github.com/$OWNER/$REPO/releases/download/$VERSION/cass-$TARGET.tar.gz`),
verifies a checksum, and `install -m 0755`s it to `$DEST` (default `$HOME/.local/bin`). Its
`--from-source` path does `git clone --depth 1 --branch "$VERSION" ...` into a **temp directory** and
builds *that* clone — not this working tree, and it requires `$VERSION` to resolve to an existing git
tag/release, which an unreleased local fix does not have. Neither path builds or installs the repo I'm
standing in. The actual prior deploy (bead c7yaw) was a manual `cargo build --release` + manual binary
placement, matching what's reconstructed in §4.

**Side effects beyond placing the binary — the specific thing I was asked to check:**
`install.sh` does **exactly one** thing beyond placing the binary, and it is conditional:
`maybe_add_path()` (install.sh:214-238) appends `export PATH="$DEST:$PATH"` to `~/.zshrc` **and**
`~/.bashrc`, but **only when `--easy-mode` (`EASY=1`) is passed**, and even then only if `$DEST` is not
already on `$PATH`. Confirmed `$HOME/.local/bin` **is already on this shell's `$PATH`** (MEASURED:
`echo ":$PATH:" | rg -o "$HOME/\.local/bin"` matched), so `maybe_add_path` would be a no-op here even if
`--easy-mode` were passed — the `case ":$PATH:" in *:"$DEST":*) return 0;;` branch short-circuits before
ever reaching the rc-file write. Grepped the full file for `hook|skill|plist|launchctl|launchd|settings.json`
and found zero matches beyond the two `.zshrc`/`.bashrc` lines already covered — no hook registration, no
config writes, no skill installs. `--quickstart` (separate flag, not default) would run `cass index
--full`, which this lane's constraints forbid — worth the coordinator explicitly NOT passing that flag,
though it's opt-in and would not fire by accident.

**Net: install.sh is safe from the "silently rewrites something it shouldn't" pattern this machine has
been burned by before, but it is also the wrong tool for this specific job** (no local-checkout build
path, no `--dest`-only-place-this-file-I-already-built mode). The deploy should be a manual build +
manual atomic placement, per §4.

## 4. Deploy / rollback ritual

MEASURED — reconstructed from bead 1a7mk/c7yaw text plus direct filesystem verification, not from any
committed script (the binaries are untracked; git history only holds the *beads* commits that record the
decision, e.g. `667aeb49`, `8ca0f8e0` — `git show --stat` on those shows only `.beads/*` files changed,
confirming the actual deploy commands were never captured in git and had to be reconstructed).

Current state on disk, sha256-verified just now:

| path | bytes | sha256 | identity |
|---|---|---|---|
| `~/.local/bin/cass` (live) | 51,834,784 | `3d044227...` | pre-fix (== pre-coverage-floor-20260601) |
| `~/.local/bin/cass.pre-coverage-floor-20260601` | 51,834,784 | `3d044227...` | preserved pre-fix specimen |
| `~/.local/bin/cass.coverage-floor-fix-20260810` | 51,900,976 | `d0b860eb...` | preserved fix specimen — **the one with the unbounded-read bug (1a7mk), do not redeploy as-is** |

This matches the coordinator's stated facts exactly and matches bead 1a7mk/c7yaw's own sha256 citations
byte-for-byte.

**The hazard, stated precisely from the bead:** "Re-deploy with an atomic rename, not cp over the live
path — overwriting in place gives SIGKILL from a stale signature cache even though codesign reports the
bytes valid." This is the classic macOS AMFI/Gatekeeper trust-cache trap: `cp SRC EXISTING_DST` opens the
existing destination inode and overwrites its *content* in place — same inode, new bytes — and the
kernel's cached code-signature validation for that inode can go stale, SIGKILLing the next exec even
though `codesign --verify` on the file itself reports valid. `mv`/`rename(2)` on the same filesystem
instead retargets the directory entry to a **new** inode, so there's no stale cache to collide with.
`$HOME/.local/bin` is a single filesystem, so a `cp`-to-temp-name then `mv` onto the live path is a true
atomic rename.

**Exact deploy commands** (after a build lands `target/release/cass`):

```bash
cd /Users/dalecarman/dev/coding_agent_session_search
STAMP=$(date +%Y%m%d-%H%M%S)

# 1. Preserve a permanent, never-touched-again specimen of the NEW binary — new file, new inode,
#    nothing live has this path open, so a plain cp here is fine (the hazard is only about
#    overwriting an EXISTING live path).
cp target/release/cass ~/.local/bin/cass.coverage-floor-fix-bounded-${STAMP}
chmod +x ~/.local/bin/cass.coverage-floor-fix-bounded-${STAMP}

# 2. Stage the same bytes under a temp name ON THE SAME DIRECTORY (same filesystem is what makes
#    the next mv an atomic rename rather than a cross-device copy).
cp target/release/cass ~/.local/bin/.cass.new.$$
chmod +x ~/.local/bin/.cass.new.$$

# 3. Atomic rename onto the live path — this is the step that avoids the stale-signature SIGKILL.
mv -f ~/.local/bin/.cass.new.$$ ~/.local/bin/cass

# 4. Verify.
shasum -a 256 ~/.local/bin/cass target/release/cass   # must match
~/.local/bin/cass --version                            # should print the new VERGEN_GIT_SHA
```

**Exact rollback command (one step, matches "deploy AND roll back in one step"):**

```bash
cp ~/.local/bin/cass.pre-coverage-floor-20260601 ~/.local/bin/.cass.rollback.$$
chmod +x ~/.local/bin/.cass.rollback.$$
mv -f ~/.local/bin/.cass.rollback.$$ ~/.local/bin/cass
shasum -a 256 ~/.local/bin/cass   # must read 3d044227...
```

Never `cp ... ~/.local/bin/cass` directly (steps 2-3 / rollback both avoid this), and never delete either
preserved specimen — per repo rule, nothing gets removed, only superseded by a newer timestamped
specimen alongside it.

## 5. Toolchain

MEASURED.

- `rust-toolchain.toml`: `channel = "nightly"` (floating channel name, **not** a dated pin like
  `nightly-2025-12-10`), `components = ["rustfmt", "clippy"]`, `profile = "default"`.
- Bare `rustc --version` / `cargo --version` on this shell's default `$PATH` resolve to **Homebrew's
  stable 1.96.0** (`/opt/homebrew/bin/{rustc,cargo}` -> `../Cellar/rust/1.96.0/bin/...`). `rustup` is
  **not on `$PATH` at all** — `command -v rustup` fails, and `which -a rustc`/`which -a cargo` each
  return exactly one hit (the Homebrew one), confirming `~/.cargo/bin` is absent from `$PATH`.
- `~/.cargo/bin/` DOES exist and holds the rustup proxy shims (`cargo -> rustup`, `rustc -> rustup`,
  etc.) plus the `rustup` binary itself. `~/.rustup/toolchains/` has both `nightly-aarch64-apple-darwin`
  and `stable-aarch64-apple-darwin` installed.
- **Verified the fix directly:** `PATH="$HOME/.cargo/bin:$PATH" rustc --version` →
  `rustc 1.96.0-nightly` — wait, actual measured output: `rustc 1.94.0-nightly (f52090008 2025-12-10)`
  and `cargo 1.94.0-nightly (2c283a9a5 2025-12-04)`. So prefixing `$HOME/.cargo/bin` onto `$PATH` makes
  the rustup shim take over, read `rust-toolchain.toml`, and correctly resolve to the pinned nightly
  channel — which on this machine is presently a **~8-month-old nightly** (2025-12-10, today is
  2026-08-14), **older** than the Homebrew stable that would otherwise be picked up silently. This
  matches bead c7yaw's own note that the prior successful build was "PATH-prefixed to the rustup
  nightly the rust-toolchain.toml pins" — same mechanism, same resolved toolchain (nobody has run
  `rustup update nightly` since, or `~/.cargo/bin` would have moved off `$PATH` entirely rather than
  just being absent).
- Checked both crate roots (`src/main.rs`, `src/lib.rs`) for `#![feature(...)]` unstable-language gates —
  none found. So the nightly requirement isn't (visibly, from the two roots checked) load-bearing for
  language features; it's the documented/pinned contract and it's what the last verified-working build
  used, so match it rather than deviate on a hunch.
- **The exact build command, folding in §2 and this section:**

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo build --release
# target dir defaults to ./target inside the repo — see §2 for why that's the right choice here, not
# an override to an isolated empty path.
```

- `ubs`: installed at `~/.local/bin/ubs`, reports `UBS Meta-Runner v5.0.7 (git 37d52925)`. CI's pin file
  `.github/workflows/ubs-version.txt` contains the single word `latest` — there is no specific version to
  compare against; "latest" cannot drift out of sync with itself, so this is a non-issue, not a gap.

## 6. CI gates (`.github/workflows/ci.yml`, 963 lines)

MEASURED (`rg` for job headers and `cargo`/`ubs` invocations, spot-read the `lint` job in full).

Trigger: `push: branches: [main]` and `pull_request: branches: [main]`, plus `workflow_dispatch`. Given
this repo's working style is direct push to `main` (no PR gate blocking the push itself), **CI runs
*after* the push lands, not before** — it's a report, not a lock. It still matters: if it goes red on
`main`, that's a visible public-repo signal to fix, not something that blocks this deploy from
proceeding locally.

Jobs, in dependency order: `no-mock-audit` → `lint` → `ubs-changed-files`, `test-rust`, `ssh-sync-docker`,
`crypto-vectors`, `security` → `build` (needs all five of the previous non-lint jobs).

What each actually gates:

- **no-mock-audit**: `./scripts/validate_ci.sh --artifact-hygiene-only`, then greps the diff for
  `Mock[A-Z]|Fake[A-Z]|Stub[A-Z]|mock_|fake_|stub_` patterns against an allowlist
  (`tests/policies/no_mock_allowlist.json`). Not relevant to this fix (no new mock/fake code expected in
  a bounded-read fix).
- **lint**: `cargo fmt --all -- --check`, then
  **`cargo clippy --all-targets --features "qr encryption backtrace" -- -D warnings`**. Note this is
  *not* identical to the plain `cargo clippy --all-targets -- -D warnings` AGENTS.md documents as the
  local gate — CI's invocation additionally activates `backtrace` (dep-free feature, on top of the
  default `qr, encryption, semantic`) and explicitly avoids `--all-features` so it doesn't also pull in
  `strict-path-dep-validation` (the comment in the workflow says exactly why: CI clones sibling repos at
  HEAD, not the pinned revs that feature's build.rs validation expects). **Coordinator implication: a
  clean local `cargo clippy --all-targets -- -D warnings` does not by itself prove CI's clippy job will
  be clean** — the `backtrace` feature combination isn't exercised locally unless explicitly added. Also
  installs a **fresh nightly toolchain via `dtolnay/rust-toolchain@... # nightly`** on the CI runner
  (not this machine's possibly-8-months-stale local nightly), so CI's clippy could in principle diverge
  from a local run for toolchain-version reasons too, independent of the feature-set gap.
  The `lint` job also clones four sibling repos (`asupersync`, `frankensqlite`, `franken_agent_detection`,
  `frankensearch`) at `HEAD` via `git clone --depth 1` before running — did not chase why this happens
  unconditionally in a job that avoids the feature needing it; not relevant to this local deploy.
- **ubs-changed-files**: runs `ubs --format=json --ci <changed files>` and diffs base-vs-current findings
  — this is the same UBS pre-merge gate AGENTS.md documents; running it locally before push
  (`ubs --format=json --ci <changed files>`) is the direct local equivalent.
- **test-rust**: `cargo test --features "qr encryption backtrace" --verbose -- --nocapture`,
  `cargo test --doc`, plus an E2E variant with `E2E_LOG=1`. Same feature-set note as `lint` applies.
- **ssh-sync-docker**: `cargo test --features "qr encryption backtrace" --test ssh_sync_integration
  -- --ignored --test-threads=1 --nocapture` and an `e2e_ssh_sources` equivalent — both `--ignored`, so
  these don't run under a plain `cargo test`.
- **crypto-vectors**: `cargo test --test crypto_vectors -- --nocapture`.
- **security**: `cargo audit`.
- **build**: `cargo build --release --target ${{ matrix.target }}` — this is the cross-platform release
  **artifact** build (Linux targets per `[workspace.metadata.dist] targets =
  ["x86_64-unknown-linux-gnu"]` in Cargo.toml), gated on all five jobs above passing. Unrelated to this
  Mac's arm64 local deploy; it's what produces the tarballs `install.sh`'s default path downloads.

None of this blocks the local build/deploy in §2/§4. The two things worth doing before pushing, beyond
what AGENTS.md already prescribes, are: (a) run clippy with the CI feature set at least once
(`--features "qr encryption backtrace"`) rather than trusting the bare default-features clippy run, and
(b) run `ubs --format=json --ci` against the exact changed files, matching the `ubs-changed-files` job.

---

## Deliver (summary for the coordinator)

**Build:**
```bash
cd /Users/dalecarman/dev/coding_agent_session_search
PATH="$HOME/.cargo/bin:$PATH" cargo build --release
```
- No `CARGO_TARGET_DIR` override — reuse the repo's existing `target/` (2.4 GB, idle, no lock, last
  built from a commit only 21 lines removed from HEAD). This turns the build into an incremental
  recompile-one-crate-plus-fat-LTO-relink rather than a from-scratch 823-package build.
- **Cost: not directly measured.** Reusing the cache is a floor-bounded win (strictly cheaper than
  rebuilding 2265 cached dependency artifacts from zero); no minute-figure for either case is backed by
  an executed timer in this session — say so plainly if reporting a number upstream.
- The `$HOME/.cargo/bin` PATH prefix is load-bearing: without it, bare `cargo` on this Mac silently
  builds with Homebrew's stable 1.96.0 instead of the `rust-toolchain.toml`-pinned nightly
  (verified: prefixed → `rustc 1.94.0-nightly (2025-12-10)`; unprefixed → `rustc 1.96.0` stable).

**Deploy (atomic, avoids the stale-codesign SIGKILL bead 1a7mk warned about):**
```bash
STAMP=$(date +%Y%m%d-%H%M%S)
cp target/release/cass ~/.local/bin/cass.coverage-floor-fix-bounded-${STAMP}   # permanent specimen
chmod +x ~/.local/bin/cass.coverage-floor-fix-bounded-${STAMP}
cp target/release/cass ~/.local/bin/.cass.new.$$
chmod +x ~/.local/bin/.cass.new.$$
mv -f ~/.local/bin/.cass.new.$$ ~/.local/bin/cass
shasum -a 256 ~/.local/bin/cass target/release/cass    # must match
```

**Rollback (one step, live path back to the known-good pre-fix binary):**
```bash
cp ~/.local/bin/cass.pre-coverage-floor-20260601 ~/.local/bin/.cass.rollback.$$
chmod +x ~/.local/bin/.cass.rollback.$$
mv -f ~/.local/bin/.cass.rollback.$$ ~/.local/bin/cass
shasum -a 256 ~/.local/bin/cass   # must read 3d044227...
```

**Gates before pushing:**
1. `cargo fmt --all -- --check`
2. `PATH="$HOME/.cargo/bin:$PATH" cargo clippy --all-targets --features "qr encryption backtrace" -- -D warnings`
   (the CI feature set — not the narrower default-only local invocation AGENTS.md shows)
3. `ubs --format=json --ci <changed files>` (base-vs-current regression check)
4. Whatever test scope the coordinator judges proportional to the fix (this lane took no position on
   that — it's outside this lane's scope)
5. `git push origin main`, then `git push origin main:master` per this repo's AGENTS.md convention

**install.sh is not part of this deploy** — it fetches tagged GitHub releases or clones-and-builds a
fresh temp checkout, neither of which installs *this* working tree's build. Its only side effect beyond
placing a binary (a conditional `~/.zshrc`/`~/.bashrc` PATH append) is inert on this machine anyway,
because `~/.local/bin` is already on `$PATH` and the flag that would trigger it (`--easy-mode`) isn't
part of any command above.
