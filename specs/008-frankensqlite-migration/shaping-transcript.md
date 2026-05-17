---
shaping: true
---

<!-- shape:complete:v1 | harness: pi/claude-sonnet-4-6 | date: 2026-03-27T16:03:16Z -->

# Shaping Transcript — Spec 008 (v2)

**Driver:** FastRaven (pi/claude-sonnet-4-6)
**Challenger:** YoungUnion (crew-challenger, claude-opus-4-6)
**Date:** 2026-03-27
**Session:** Full upstream sync strategy

> Note: A previous shaping session (v1, with NiceBear) was discarded.
> That session shaped the wrong question ("which franken items do we cherry-pick?")
> The user corrected the framing: "I want to be aligned with upstream.
> What in our fork demands divergence?" The correct question is how to achieve
> full upstream parity, not selective adoption.
> The v1 transcript is preserved as `shaping-transcript-v1-discarded.md`.

---

## Round 1: Challenger verifies facts, flags a clarification

**YoungUnion confirmed:**
- All 5 unique files exist (watchdog.rs is 951 lines — non-trivial)
- `count_disk_files` + `reconciliation_notes` in connectors/mod.rs (2 matches)
- SIGTERM/heartbeat logic in indexer/mod.rs
- `franken-agent-detection` already a git dep in our Cargo.toml at rev `5b0eb1a`
- Our version: 0.1.55 / Upstream: 0.2.4

**Clarification raised:** The "all upstream path deps" claim was about upstream's Cargo.toml, not ours. Our fork already uses `franken-agent-detection` as a git dep. The other franken libs (frankensqlite, frankensearch, frankentui, asupersync) are absent from our Cargo.toml entirely — we'd be adding them fresh, not converting them.

---

## Round 2: Driver proposes requirements R0–R5

Requirements proposed. R0 listed: `crush, kimi, copilot_cli, qwen, clawdbot, openclaw, vibe, copilot` as connectors to gain.

---

## Round 3: Challenger challenges requirements — four real defects

**Challenge 1: R0's connector list had phantom connectors.**
`crush`, `kimi`, `copilot_cli`, `qwen` — grepped our codebase: zero hits. These are upstream-only connectors (confirmed in upstream's connectors/mod.rs). R0 was conflating "what upstream has" with "what we have." Reframe: we WANT these, we don't HAVE them.

**Challenge 2: fad_adapter removal is not free.**
indexer/mod.rs has 10 direct call sites to `fad_adapter::copilot`, `fad_adapter::clawdbot`, `fad_adapter::openclaw`, `fad_adapter::vibe`. R1 understated migration cost.

Resolution: taking upstream's indexer/mod.rs wholesale means those call sites are replaced by upstream's native connector wiring. fad_adapter removal is actually implicit in taking upstream's files — not a separate task.

**Challenge 3: count_disk_files blast radius is 14 files.**
`grep -l count_disk_files src/connectors/*.rs` returned 14 files. Taking upstream's connector files replaces all 14 implementations. The Connector trait definition (from upstream) won't have count_disk_files. Every connector file will fail to compile unless we decide on a strategy. Options: re-add to all 14 files, extension trait, or drop doctor command.

**Challenge 4: R3 "zero data loss" is an aspiration without a rollback story.**
R3 needed: explicit VACUUM INTO backup before migration, incremental migration path (v8→v9→…→v13), and acknowledgment that migration is one-way (backup is the rollback mechanism).

**All four challenges accepted.** Requirements updated.

---

## Round 4: Driver proposes revised requirements + three shapes

**Revised requirements (R0–R5):**

| ID | Requirement | Status |
|----|-------------|--------|
| R0 | Full parity with upstream v0.2.4: frankensqlite, frankensearch, frankentui, all 19 upstream connectors | Core goal |
| R1 | Our unique additions survive: watchdog.rs, codebuff.rs, watcher SIGTERM/heartbeat. Doctor trait (count_disk_files) — extension trait or drop, decided during merge conflict resolution | Must-have |
| R2 | Build self-contained via git deps — origin/main's git dep Cargo.toml is the foundation; bump revs, add franken sub-crates if needed | Must-have |
| R3 | 8.8GB DB: VACUUM INTO backup before migration; schema migrates v8→v13 incrementally; one-way (backup is rollback) | Must-have |
| R4 | Watcher, watchdog, launchd plist continue working post-merge | Must-have |
| R5 | History clean: merge commit clearly marks upstream sync boundary | Nice-to-have |

**Three shapes proposed:**
- **A**: Clean branch from upstream/main + force-push
- **B**: File-by-file overlay on current branch
- **C**: Clone sibling repos, use upstream Cargo.toml path deps verbatim

---

## Round 5: Challenger finds the session-defining error — shared history EXISTS

**Critical finding from YoungUnion (verified with grep):**

```
$ git merge-base origin/main upstream/main
81f25604bcf9f1aa3520e672b7e4e6cfe4ce56ec
```

The merge-base IS commit `81f25604`: "chore: convert path deps to git deps for v0.2.0 release (closes #89)". This is our origin/main. Upstream is 274 commits ahead of this fork point. There ARE 0 commits on origin/main past this point — we are exactly at the fork.

**Consequences:**
- Shape A ("force-push gymnastics") solves a problem that doesn't exist — normal merge/rebase is available
- Shape B (file-by-file overlay) is massively over-engineered when `git merge` handles 182 files automatically
- Shape C violates R2 AND mischaracterizes upstream's Cargo.toml (upstream already uses git deps for ftui and frankensearch)

**Additional finding:** upstream's Cargo.toml has 8 path deps remaining — but for sub-crates of asupersync (`franken-decision`, `franken-evidence`, `franken-kernel`) and the frankensqlite sub-crate (`fsqlite-types`). R2 needs to account for converting these too.

**Proposed Shape D (YoungUnion):** Standard git merge — `git checkout origin/main -b sync/upstream-v0.2.4` → `git merge upstream/main` → resolve ~5 conflict files → cherry-pick/re-apply our unique additions → `cargo build --release`.

---

## Round 6: Driver confirms, proposes Shapes D and E

Driver verified:
- origin/main Cargo.toml already has all franken libs as git deps (frankensqlite, frankensearch, ftui, asupersync, franken-agent-detection) — this was the v0.2.0 release state
- Upstream reverted to path deps for their private monorepo development — we never need to follow that revert
- git diff origin/main HEAD works for all changed files (tree objects intact)

**Shape E** proposed (merge + rebase our spec commits as patch series) — rejected as strictly harder than D given the git object corruption.

---

## Round 7: Challenger surfaces the corruption issue as a real obstacle

**Git object corruption:**
Object `cb78850fe136a1a1154b71f06c5d834a754a0703` is missing from our repo. It's in the ancestry chain of our `feat/007-watchdog-subcommand` branch. This breaks:
- `git merge-base HEAD upstream/main` (can't walk our history past the corruption)
- `git cherry-pick` from HEAD
- Any ancestry traversal of HEAD

**However:** Tree objects are intact. `git diff origin/main HEAD -- <path>` works:
- `src/watchdog.rs`: 957 lines of diff available
- `src/connectors/codebuff.rs`: 527 lines
- `src/indexer/mod.rs`: 2,534 lines
- `src/connectors/mod.rs`: 1,224 lines

35 commits reachable from HEAD before corruption — our spec work is accessible via tree diffs.

**Shape D amendment:** D4 must use `git diff origin/main HEAD -- <path>` to extract changes as patches rather than `git cherry-pick`. Cherry-pick traverses commit objects (broken); diff extracts from tree objects (intact).

---

## Final Fit Check

| Req | Requirement | Status | D |
|-----|-------------|--------|---|
| R0 | Full parity with upstream v0.2.4: frankensqlite, frankensearch, frankentui, all 19 upstream connectors | Core goal | ✅ |
| R1 | Watchdog.rs, codebuff.rs, watcher SIGTERM/heartbeat survive; doctor trait strategy decided at merge time | Must-have | ✅ |
| R2 | Self-contained git deps; origin/main deps as foundation; sub-crate deps added | Must-have | ✅ |
| R3 | VACUUM INTO backup; v8→v13 incremental; one-way with backup as rollback | Must-have | ✅ |
| R4 | Watcher, watchdog, launchd plist continue working | Must-have | ✅ |
| R5 | Merge commit marks upstream sync boundary clearly | Nice-to-have | ✅ |

---

## Selected Shape: D — Standard git merge with diff-based patch extraction

| Part | Mechanism |
|------|-----------|
| D1 | `git checkout origin/main -b sync/upstream-v0.2.4` — start at clean fork point (81f25604) |
| D2 | `git merge upstream/main` — 3-way merge; git auto-resolves ~177 files, flags ~5 conflicts |
| D3 | Resolve conflicts: Cargo.toml (keep git dep strategy, bump revs, add franken-decision/evidence/kernel + fsqlite-types as git deps), connectors/mod.rs (extension trait or drop for count_disk_files), indexer/mod.rs (preserve SIGTERM/heartbeat), lib.rs, main.rs |
| D4 | Extract our unique changes via `git diff origin/main HEAD -- <path>` for: watchdog.rs (new file), codebuff.rs (new file), indexer/mod.rs SIGTERM+heartbeat diff, connectors/mod.rs doctor trait diff; apply to merged branch |
| D5 | `cargo build --release` — verify clean build |
| D6 | `sqlite3 live.db "VACUUM INTO 'backup.db'"` then run schema migration v8→v13 |

**Rationale:** Upstream alignment is the goal, and we have a clean merge path from origin/main. The git object corruption on feat/007 is worked around by operating from origin/main and extracting our unique additions as tree diffs. Shape D uses standard git mechanics for 99% of the work and only requires manual extraction for the 4 files where we have unique additions.

---

## Key Decisions Made During Shaping

1. **fad_adapter.rs is dropped** — upstream's native connectors replace it entirely; taking upstream's indexer/mod.rs makes this automatic
2. **Doctor trait strategy is deferred** — to be decided during D3 conflict resolution (extension trait preferred; drop is acceptable)
3. **Cargo.toml strategy** — origin/main's git dep versions are the baseline; bump to upstream's current pinned revs, add missing sub-crate deps
4. **frankentui is IN scope** — full parity means frankentui; prior shaping session was wrong to exclude it
5. **frankensqlite is IN scope** — same reasoning

---

## What Demands Divergence (the user's original question)

Almost nothing. Specifically:
- **watchdog.rs** — launchd-aware watchdog subcommand; upstream has nothing like it
- **codebuff.rs** — Codebuff connector; not in upstream
- **indexer/mod.rs SIGTERM/heartbeat** — ~30 lines; watcher reliability improvements upstream lacks
- **connectors/mod.rs doctor trait** — 2 methods; either extension trait or dropped entirely

Everything else: take upstream's version verbatim.
