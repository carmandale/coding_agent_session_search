---
shaping: true
---

<!-- plan:complete:v1 | harness: pi/gpt-5.3-codex | date: 2026-04-03T18:28:23Z -->

# Spec 012 — Planning Transcript (Two-Agent)

**Date:** 2026-04-03
**Driver:** DarkHawk (pi/claude-opus-4-6)
**Challenger:** ZenNova (crew-challenger)
**Scope:** Build implementation plan for `specs/012-upstream-sync-query-tui/spec.md`

## Driver research shared before plan drafting

DarkHawk shared concrete repository findings (with file/function-level insertion points):

1. **`src/lib.rs` watchdog wiring points** on upstream base:
   - `Commands` enum addition
   - `describe_command()` exhaustive arm
   - tracing subscriber command list (stderr compact)
   - main dispatch wildcard block (`_ => {}`)
   - explicitly **no change** required in async import dispatch wildcard
2. **`src/storage/sqlite.rs` cascade patch**:
   - `franken_insert_message` return type changes from `Result<i64>` to `Result<Option<i64>>`
   - 6 call sites must adopt `let Some(msg_id) = ... else { continue; }`
   - remove `LIMIT 1000` and `LIMIT 100` from fingerprint queries
3. **`src/indexer/mod.rs` WAL seed points**:
   - before entering watch mode (`set_mode(Watch)` path)
   - top of `reindex_paths()` before `classify_paths()`
4. **`Cargo.toml` facts**:
   - upstream already on `frankensqlite=ff6a114b` and `frankensearch=9961c0e7`
   - upstream FAD enables `opencode` feature (must be removed in fork)
   - upstream has no active `[patch]` section

## Challenger findings (adversarial)

ZenNova challenged the approach with concrete checks:

### Confirmed gaps

1. **Fork-only infra directories missing from upstream branch baseline**
   - Verified upstream tree lacks: `specs/`, `thoughts/`, `.claude/`, `hooks/`
   - Risk: starting from upstream branch drops local planning/operations artifacts unless explicitly overlaid.

2. **Missing `state_meta_json()` watchdog block in patch list**
   - Driver verified our `src/lib.rs` has watchdog JSON block; upstream has none.
   - Risk: `cass health --json` parity loss and test failure if omitted.

3. **Workflow normalization under-specified**
   - Upstream has 11 workflow files; fork has 7.
   - Need explicit restore strategy for fork workflow set.

### Evidence-gated decisions requested

4. **`Cargo.toml` active patch section**
   - Unknown if required after rebasing onto upstream revs.
   - Must verify empirically (`cargo check` with and without patch stanza).

5. **`asupersync` revision drift**
   - Fork uses `95476b32`; upstream uses `08dd31df`.
   - Must document and gate decision by build/test evidence.

### Additional hard dependency

6. **Ordering dependency: C6 before C7**
   - If FAD `opencode` feature is removed before stubs are in place, build fails due to unresolved import path.
   - Must encode explicit dependency in tasks.

## Driver revisions accepted by challenger

DarkHawk proposed and ZenNova approved:

- expand overlay scope to include fork-only infra dirs + AGENTS/.gitignore/workflows
- add explicit `state_meta_json()` watchdog patch step
- add hard ordering dependency: connector stubs before Cargo feature removal
- treat `[patch]` and `asupersync` decisions as evidence-gated (not pre-decided)

## Approval

ZenNova final verdict: **approved** after revisions and evidence-gated decision gates were added.
