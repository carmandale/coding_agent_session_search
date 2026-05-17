---
title: "Evidence hygiene pass: spec 016"
date: 2026-05-17T00:02:48Z
bead: coding_agent_session_search-1vxuf
---

# Evidence Hygiene Pass

## Scope

Reviewed spec 016 evidence before any eventual commit/stage decision. This pass did not delete, redact in place, or mutate live CASS state.

## Checks Run

```text
rg --files specs/016-cass-recovery-ingestion/evidence | sort
du -sh specs/016-cass-recovery-ingestion/evidence specs/016-cass-recovery-ingestion/evidence/*
refined credential-pattern scan over evidence files and spec receipts
find specs/016-cass-recovery-ingestion/evidence -type f -size +1M -print
head -5 <oversized evidence files>
rg -n 'snippet|content|message|messages|source_path|/Users/dalecarman|Groove Jones|Dropbox' <stdout evidence>
```

## Findings

- Evidence tree size is about `34M`.
- Largest evidence groups are `recovery-runs` (`17M`), `canary` (`6.6M`), `runtime-refresh` (`6.5M`), and `manifests` (`3.2M`).
- Refined credential/key scan returned no matches for API keys, bearer headers, key material, OAuth-style token fields, or password assignment patterns.
- Broad text scan had expected false positives for fields such as `max_tokens`, `token_budget`, and code symbols such as `redact_secrets`.
- Oversized files are mostly macOS sample/vmmap summaries and manifest JSONL files. Sample files showed binary/process paths already redacted by tooling as `/Users/USER/*/cass`.
- Several raw stdout/candidate files contain absolute local paths and project names, including Claude/Codex/Pi source paths and shadow data-dir paths. These are operational evidence, not credentials, but they are local-environment metadata.
- Search probe strings currently recorded in summary receipts are non-secret technical markers or file names used for recovery proof, such as `frankensqlite`, `freelist serializer`, `opencode`, `factory`, `ATT21_COL_CFP_SceneMachine_EndCard.psd`, and synthetic `SPEC016_*` markers.

## Commit Guidance

Commit-ready summary artifacts:

- `specs/016-cass-recovery-ingestion/evidence/live-promotion-runbook.md`
- `specs/016-cass-recovery-ingestion/evidence/release-candidate-shadow-proof.md`
- `specs/016-cass-recovery-ingestion/evidence/frankensqlite-fix-proof.md`
- `specs/016-cass-recovery-ingestion/evidence/upstream-blocker.md`
- `specs/016-cass-recovery-ingestion/evidence/upstream-working-tree-overlap.md`
- `specs/016-cass-recovery-ingestion/evidence/upstream-reconciliation-map.md`
- `specs/016-cass-recovery-ingestion/evidence/spec015-routing.md`
- `specs/016-cass-recovery-ingestion/evidence/evidence-hygiene.md`
- spec-level receipts/audits: `implement-receipt.md`, `completion-audit.md`, `tasks.md`, `log.md`, and GoalBuddy state/handoff files.

Keep local unless an explicit verifier needs raw replay detail:

- `specs/016-cass-recovery-ingestion/evidence/canary/*.stdout`
- `specs/016-cass-recovery-ingestion/evidence/canary/*.sample.txt`
- `specs/016-cass-recovery-ingestion/evidence/canary/*.ps-samples.txt`
- `specs/016-cass-recovery-ingestion/evidence/canary/*.kill-log.txt`
- `specs/016-cass-recovery-ingestion/evidence/recovery-runs/*.stdout`
- `specs/016-cass-recovery-ingestion/evidence/recovery-runs/*.stderr`
- `specs/016-cass-recovery-ingestion/evidence/recovery-runs/*.sample*.txt`
- `specs/016-cass-recovery-ingestion/evidence/recovery-runs/*.ps-samples.txt`
- `specs/016-cass-recovery-ingestion/evidence/recovery-runs/*.kill-log`
- `specs/016-cass-recovery-ingestion/evidence/runtime-refresh/*.stdout`
- `specs/016-cass-recovery-ingestion/evidence/runtime-refresh/*.stderr`
- `specs/016-cass-recovery-ingestion/evidence/runtime-refresh/*.sample*.txt`
- `specs/016-cass-recovery-ingestion/evidence/runtime-refresh/*.ps-samples.txt`
- full path manifests `specs/016-cass-recovery-ingestion/evidence/manifests/*.jsonl`, unless source-path reconciliation needs the exact frozen list in review.

## Status

`pass`: no credential material found. Raw local-path and bulky telemetry files are identified for local-only handling unless needed for verifier replay.
