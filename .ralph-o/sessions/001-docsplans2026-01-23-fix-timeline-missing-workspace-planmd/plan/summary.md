# Planning Summary: Fix Timeline Missing Workspace

**Date:** 2026-01-23
**Session:** 001-docsplans2026-01-23-fix-timeline-missing-workspace-planmd

---

## Artifacts Created

| File | Purpose |
|------|---------|
| `plan/rough-idea.md` | Initial concept and problem statement |
| `plan/idea-honing.md` | Requirements clarification (minimal - idea was well-defined) |
| `plan/research/codebase-analysis.md` | Verification of code locations and patterns |
| `plan/design/detailed-design.md` | Comprehensive design document |
| `plan/implementation/plan.md` | Step-by-step implementation checklist |
| `plan/summary.md` | This summary document |

---

## Problem Summary

The `cass timeline` command returns `"workspace": null` for all Claude Code sessions even though the database correctly stores workspace information. This breaks downstream tools like `gj last` that filter sessions by workspace.

---

## Solution Summary

**6 code changes** in `src/lib.rs` to add workspace to timeline output:

1. **SQL Query** - Add `LEFT JOIN workspaces` and `w.path as workspace`
2. **Row Extraction** - Add workspace field at index 10
3. **Tuple Type** - Add `Option<String>` as 11th element
4. **JSON None Mode** - Add workspace to destructuring and output
5. **JSON Hour/Day Mode** - Add workspace to destructuring and output
6. **Non-JSON Mode** - Add `_workspace` for tuple destructuring

---

## Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| LEFT JOIN | Matches existing codebase pattern; handles null workspace_id |
| Option<String> | Rust idiom for nullable database columns |
| Add to tuple (not struct) | Minimal change; avoids scope creep |
| No non-JSON display change | Not requested; keeps fix focused |

---

## Implementation Approach

- **Scope:** Single file (`src/lib.rs`)
- **Risk:** Low - follows existing patterns
- **Dependencies:** None
- **Backwards Compatible:** Yes - adds field, doesn't change existing

---

## Verification Steps

```bash
# Compilation
cargo check --all-targets
cargo clippy --all-targets -- -D warnings

# Tests
cargo test

# Manual verification
cass timeline --since 1d --json | jq '.sessions[0] | {agent, workspace}'
```

---

## Next Steps

1. Review the implementation plan at `plan/implementation/plan.md`
2. Run `ralph-o task` to generate structured code tasks
3. Run `ralph-o run` to execute the implementation

---

## Areas for Future Consideration

- Add workspace to non-JSON timeline display (optional enhancement)
- Add timeline-specific tests (currently no coverage)
- Simplify `gj last` to use workspace field directly instead of source_path matching
