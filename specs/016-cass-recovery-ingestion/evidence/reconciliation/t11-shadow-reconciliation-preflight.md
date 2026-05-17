---
title: "T11 shadow reconciliation preflight"
date: 2026-05-17T02:03:00Z
bead: coding_agent_session_search-1vxuf
task: T11
result: shadow-only-preflight
---

# T11 Shadow Reconciliation Preflight

## Decision

T11 remains unchecked.

This artifact records read-only reconciliation against the verified shadow
archive. It is useful preparation, but it does not satisfy the live acceptance
requirement because the shadow DB/index has not been promoted into the installed
live CASS data dir.

## Inputs

Shadow DB:

```text
/Users/dalecarman/Library/Application Support/com.coding-agent-search.coding-agent-search-spec016-shadow-20260516T2025Z/agent_search.db
```

Frozen manifests:

```text
specs/016-cass-recovery-ingestion/evidence/manifests/claude_code.jsonl
specs/016-cass-recovery-ingestion/evidence/manifests/codex.jsonl
specs/016-cass-recovery-ingestion/evidence/manifests/pi_agent.jsonl
```

Schema facts:

```text
conversations.source_path is indexed
conversations has UNIQUE(source_id, agent_id, external_id)
agents uses slug/name; priority rows are selected by agents.name
```

No source-path quarantine or skip table exists in the shadow archive. The only
quarantine-like table is a derived FTS quarantine table:

```text
cass_quarantined_fts_messages_1778951229819_78
```

So `accounted quarantine/skip count` is `0` for this shadow preflight unless a
future live verifier records path-specific quarantine/skip evidence elsewhere.

## Shadow Counts

| Agent | Frozen manifest paths | Shadow unique DB `source_path` count | Manifest paths matched in DB | Manifest paths missing in DB | DB paths outside manifest | Duplicate DB `source_path` groups | Duplicate non-null provenance keys |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `claude_code` | 2425 | 2551 | 2413 | 12 | 138 | 23 | 0 |
| `codex` | 5868 | 5681 | 5675 | 193 | 6 | 32 | 0 |
| `pi_agent` | 4174 | 2076 | 2076 | 2098 | 0 | 0 | 0 |

## Missing-Path Shape

Claude Code has 12 manifest paths missing from shadow DB. The missing paths are
spread across a small set of project roots:

```text
3 -Users-dalecarman-Groove-Jones-Dropbox-Dale-Carman-Projects-dac-wiki
2 -Users-dalecarman-dev-quickbooks
2 -Users-dalecarman-Groove-Jones-Dropbox-Dale-Carman-Projects-dev-wiki
2 -Users-dalecarman--agent-config
1 -Users-dalecarman-dev-gj-tool
1 -Users-dalecarman-dev-coding-agent-session-search
1 -Users-dalecarman-dev-be-still
```

Codex has 193 manifest paths missing from shadow DB:

```text
170 under 2026/
21 under 2025/
2 older top-level rollout files
```

Pi Agent has 2098 manifest paths missing from shadow DB, all under one path
family:

```text
2098 --clawdbot-chip--
```

This explains why the shadow DB has exactly `2076` Pi Agent source paths while
the frozen manifest has `4174`: the manifest includes the `--clawdbot-chip--`
family, and none of those paths are represented as `pi_agent` rows in the
shadow DB.

## Sample Missing Paths

Claude Code:

```text
/Users/dalecarman/.claude/projects/-Users-dalecarman--agent-config/98268aa5-b6ce-49c4-ab07-64f9ec3ad320.jsonl
/Users/dalecarman/.claude/projects/-Users-dalecarman--agent-config/d7a715ce-e42e-4b9d-bac9-ee76acbc46e8.jsonl
/Users/dalecarman/.claude/projects/-Users-dalecarman-Groove-Jones-Dropbox-Dale-Carman-Projects-dac-wiki/5d60d787-992d-4e4d-8f69-dc2f3e444de0.jsonl
```

Codex:

```text
/Users/dalecarman/.codex/sessions/2025/08/19/rollout-2025-08-19T10-08-56-3236180b-8759-4d1e-bee7-7ee907d5dbd4.jsonl
/Users/dalecarman/.codex/sessions/2025/08/19/rollout-2025-08-19T17-20-54-c1f2c2a4-d8bf-43a2-92a0-d12d8f8c26cc.jsonl
/Users/dalecarman/.codex/sessions/2025/08/20/rollout-2025-08-20T08-31-33-a5ea264c-d1c7-4f35-a842-cf08442a5dd3.jsonl
```

Pi Agent:

```text
/Users/dalecarman/.pi/agent/sessions/--clawdbot-chip--/0006519e-5c4e-4adf-9a08-150c9eb12cab.jsonl
/Users/dalecarman/.pi/agent/sessions/--clawdbot-chip--/000fd8a7-ea62-4740-818f-d21371b1b6b6.jsonl
/Users/dalecarman/.pi/agent/sessions/--clawdbot-chip--/001de445-a07a-4280-ac72-258a20965892.jsonl
```

## Duplicate Shape

The shadow archive has no duplicate non-null provenance keys for the three
priority agents. That is the more important identity uniqueness invariant
because `conversations` has a unique index on `(source_id, agent_id,
external_id)`.

There are duplicate `source_path` groups for Claude Code and Codex:

```text
claude_code duplicate source_path groups: 23
codex duplicate source_path groups: 32
pi_agent duplicate source_path groups: 0
```

Samples show duplicate Claude Code rows are mostly main session plus subagent
paths under the same project/session directory. Codex duplicate samples are
older rollout paths with count `2`. These need live reconciliation review before
T11/T15 can be closed.

## Commands

Read-only command shape:

```text
jq -r '.path' specs/016-cass-recovery-ingestion/evidence/manifests/<agent>.jsonl | sort -u
sqlite3 "$SHADOW" "SELECT c.source_path FROM conversations c JOIN agents a ON a.id=c.agent_id WHERE a.name='<agent>' ORDER BY c.source_path;"
comm -23 manifest.paths db.paths
comm -12 manifest.paths db.paths
sqlite3 "$SHADOW" "SELECT COUNT(*) FROM (SELECT c.source_path FROM conversations c JOIN agents a ON a.id=c.agent_id WHERE a.name='<agent>' GROUP BY c.source_path HAVING COUNT(*) > 1);"
sqlite3 "$SHADOW" "SELECT COUNT(*) FROM (SELECT c.source_id, c.agent_id, c.external_id FROM conversations c JOIN agents a ON a.id=c.agent_id WHERE a.name='<agent>' AND c.external_id IS NOT NULL GROUP BY c.source_id,c.agent_id,c.external_id HAVING COUNT(*) > 1);"
```

Temporary full comparison files were written under:

```text
/tmp/spec016-shadow-reconciliation/
```

They were not promoted into the repo because T11 remains live-blocked and the
full missing-path sets should be regenerated against the promoted live archive.

## Consequence

The shadow archive is useful and searchable, but this reconciliation preflight
proves why shadow evidence cannot be treated as final live T11/T12 completion.
After approval and promotion, regenerate this same table against the live DB and
then decide whether the `--clawdbot-chip--` Pi-family paths are priority Pi
Agent input, a ClawdBot/OpenClaw-family bonus source, or path-specific skip
evidence.
