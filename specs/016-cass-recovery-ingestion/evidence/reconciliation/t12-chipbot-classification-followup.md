---
title: "T12 preflight: chipbot symlink classification follow-up"
date: 2026-05-17T02:12:37Z
bead: coding_agent_session_search-1vxuf
---

# T12 Preflight: Chipbot Symlink Classification Follow-Up

This is not live T12 completion. It explains the `--clawdbot-chip--` split
found by `t11-shadow-reconciliation-preflight.md` and records the follow-up
issue created from that evidence.

## Finding

The shadow `pi_agent` reconciliation showed:

```text
pi_agent manifest paths: 4174
shadow pi_agent source paths: 2076
matched manifest paths: 2076
missing manifest paths: 2098
missing shape: all 2098 under --clawdbot-chip--
```

The `--clawdbot-chip--` path is a symlink, not an ordinary Pi workspace:

```text
/Users/dalecarman/.pi/agent/sessions/--clawdbot-chip-- -> /Users/dalecarman/.clawdbot/agents/main/sessions
/Users/dalecarman/.clawdbot/agents/main/sessions contains 2098 JSONL files with Pi-style nested event records
```

Older spec evidence says this bridge was deliberately preserved:

```text
specs/005-watcher-cpu-spin/planning-transcript.md: verified the symlink and 2098 JSONL files
specs/005-watcher-cpu-spin/plan.md: removing follow_links(true) would silently drop 2098 sessions and was called an unacceptable regression
```

## Current Connector Behavior

Pinned `franken_agent_detection` at rev `5115da8e515ee8a76cf676e78bc2d351e14abc82` exposes separate connector factories for `clawdbot` and `pi_agent`.

Relevant source checks:

```text
FAD src/connectors/mod.rs: clawdbot and pi_agent are separate factories
FAD src/lib.rs: both clawdbot and pi_agent are known connector slugs
FAD src/lib.rs: clawdbot probes ~/.clawdbot and ~/.clawdbot/sessions
FAD src/lib.rs: pi_agent probes ~/.pi/agent/sessions
FAD src/connectors/pi_agent.rs: session_files accepts .jsonl only when the filename contains "_"
FAD src/connectors/clawdbot.rs: parser expects top-level role/content JSONL
```

The chipbot files are mostly UUID-only filenames and use nested Pi-style records:

```text
{"type":"session", ... "cwd":"/Users/dalecarman/chipbot"}
{"type":"model_change", ... "provider":"minimax","modelId":"MiniMax-M2.1"}
{"type":"message", ... "message":{"role":"user","content":[...]}}
```

## Scratch Index Proof

Release candidate:

```text
/tmp/cass-release-target/release/cass
sha256=423e2e4c2920ec74a38a5cb4af1f00de362a4a82e493d342b4891179f4955ada
```

Chipbot symlink scratch run:

```text
cmd: /tmp/cass-release-target/release/cass index --watch-once '/Users/dalecarman/.pi/agent/sessions/--clawdbot-chip--' --json --no-progress-events --data-dir '/tmp/cass-chip-classification-20260517T021009Z'
exit: 0
conversations: 0
messages: 0
stats by_agent: []
stdout/stderr prefix: /tmp/cass-chip-classification-20260517T021009Z.*
```

Control Pi Agent scratch run:

```text
cmd: /tmp/cass-release-target/release/cass index --watch-once '/Users/dalecarman/.pi/agent/sessions/--Users-dalecarman-.clawdis-workspace--/2025-12-14T23-13-12-368Z_3235b4b5-776d-4d7d-8b06-e36a322f3a4b.jsonl' --json --no-progress-events --data-dir '/tmp/cass-pi-control-classification-20260517T021033Z'
exit: 0
conversations: 1
messages: 6
stats by_agent: pi_agent=1
stdout/stderr prefix: /tmp/cass-pi-control-classification-20260517T021033Z.*
```

Interpretation: the current release candidate and scratch index path can ingest
a normal Pi Agent session. The chipbot symlink corpus is a separate connector
classification/parsing hole, not proof that priority Pi Agent recovery failed.

## Follow-Up Issue

Created:

```text
bead: coding_agent_session_search-2d37b
spec: specs/017-chipbot-symlink-indexing/
title: Index chipbot symlink sessions under clawdbot/Pi roots
gate: issue record/verify passed
```

Spec 016 should continue to treat this as a bonus/other-session coverage gap
unless the operator explicitly makes chipbot part of the priority live
acceptance gate. T12 remains unchecked until live promotion and live priority
reconciliation rerun.
