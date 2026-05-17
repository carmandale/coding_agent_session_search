---
title: "T8 priority canary selection readiness"
date: 2026-05-17T01:58:00Z
bead: coding_agent_session_search-1vxuf
task: T8
result: preselected-blocked-before-live-proof
---

# T8 Priority Canary Selection Readiness

## Decision

T8 remains unchecked. The identity/string selection is ready, but the required
`cass index --watch-once`, live DB `source_path`, and lexical search proof still
require the approval-gated live promotion route.

Do not run the live T8 canary against the malformed live archive.

## Selected Identities

Source:

```text
specs/016-cass-recovery-ingestion/evidence/canary/canary-selection.json
```

Selected priority canaries:

| Agent | Source path | Query string |
| --- | --- | --- |
| `claude_code` | `/Users/dalecarman/.claude/projects/-Users-dalecarman-dev-quickbooks/a5066562-e3d3-4aad-ad27-90c850fb3e0f.jsonl` | `codex-implement-chunk-9-spec039-t5t6-r1.md` |
| `codex` | `/Users/dalecarman/.codex/sessions/2026/05/16/rollout-2026-05-16T08-42-53-019e3106-8e85-7343-8a7c-0eac5cb6f39a.jsonl` | `cass-session-ingestion-recovery` |
| `pi_agent` | `/Users/dalecarman/.pi/agent/sessions/--Users-dalecarman-.clawdis-workspace--/2025-12-14T23-13-12-368Z_3235b4b5-776d-4d7d-8b06-e36a322f3a4b.jsonl` | `ATT21_COL_CFP_SceneMachine_EndCard.psd` |

## Read-Only Checks

The selected source paths are all present in their frozen manifests:

```text
claude_code  /Users/dalecarman/.claude/projects/-Users-dalecarman-dev-quickbooks/a5066562-e3d3-4aad-ad27-90c850fb3e0f.jsonl
codex        /Users/dalecarman/.codex/sessions/2026/05/16/rollout-2026-05-16T08-42-53-019e3106-8e85-7343-8a7c-0eac5cb6f39a.jsonl
pi_agent     /Users/dalecarman/.pi/agent/sessions/--Users-dalecarman-.clawdis-workspace--/2025-12-14T23-13-12-368Z_3235b4b5-776d-4d7d-8b06-e36a322f3a4b.jsonl
```

Each selected source file exists locally, and each selected query string appears
inside its source file:

```text
claude_code query present at line 3882
codex query present at line 6
pi_agent query present at line 6
```

The first probe accidentally used `path` as a zsh loop variable, which shadowed
`PATH` and prevented `rg`/`sed` lookup inside that subprocess. It was read-only
and was immediately rerun with `file_path`; no repo or live data state changed.

## Commands To Run After Approval

After the verified shadow archive is promoted live and the verified binary is
installed, run the T8 canary using the selected identities above:

```text
cass index --watch-once <selected root or source path> --json --no-progress-events
sqlite3 "$LIVE_DB" "SELECT source_path FROM conversations WHERE source_path = '<selected source path>';"
cass search "<selected query>" --agent <agent> --mode lexical --robot --fields minimal --robot-meta --limit 5
```

The exact command should use the installed verified binary and the live data dir.
Save stdout, stderr, exit codes, DB proof, and search JSON under
`evidence/canary/`.

## Consequence

Selection work is ready. T8 remains blocked because the live canary itself is
part of the approval-gated production mutation path.
