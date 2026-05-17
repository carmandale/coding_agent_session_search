---
title: "Index chipbot symlink sessions under clawdbot/Pi roots"
date: 2026-05-17
bead: coding_agent_session_search-2d37b
---

<!-- issue:complete:v1 | harness: unknown | date: 2026-05-17T02:12:18Z -->

# Index chipbot symlink sessions under clawdbot/Pi roots

## Source (verbatim)

> "process all sessions and allow them to be searchable. sessions that matter are pi-agent, claude code, an codex, with opencode, factory and others being bonus sessions." - user, 2026-05-16

> "create a new $issue if necessary." - user, 2026-05-16

## Problem

Purpose Contract:

- Outcome: CASS has a deliberate, tested classification path for the 2,098 chipbot session files reachable through `/Users/dalecarman/.pi/agent/sessions/--clawdbot-chip--`, so they are either indexed and searchable under the correct agent slug or explicitly excluded with documented rationale.
- Done means: a focused index/search proof against that path produces conversations and lexical hits from the expected source files, and regression tests pin the connector behavior that makes it possible.
- Not done: spec 016's priority Pi Agent recovery succeeds while the chipbot symlink remains a silent zero-row hole, or CASS only counts the files in a manifest without making them searchable.

During spec 016 recovery, the shadow reconciliation showed `pi_agent` matched `2076/4174` frozen manifest paths and that all `2098` missing paths were under `--clawdbot-chip--`. The path is not random noise: spec 005 explicitly preserved the symlink `/Users/dalecarman/.pi/agent/sessions/--clawdbot-chip--` to `/Users/dalecarman/.clawdbot/agents/main/sessions` as existing functionality.

Current pinned `franken_agent_detection` behavior misses this corpus:

- `PiAgentConnector::session_files` only accepts `.jsonl` filenames containing `_`; chipbot files are mostly UUID-only names.
- `ClawdbotConnector` defaults to `~/.clawdbot/sessions` and parses top-level `{role, content}` JSONL; the chipbot files use nested Pi-style `{type, message}` events.
- A scratch CASS watch-once index of `/Users/dalecarman/.pi/agent/sessions/--clawdbot-chip--` with the spec 016 release candidate produced `0` conversations and `0` messages.
- A control scratch CASS watch-once index of a normal Pi Agent file produced `1` `pi_agent` conversation and `6` messages.

## Requirements

1. Preserve spec 016 priority recovery as the first-order outcome; this issue must not block live promotion/searchability for Pi Agent, Claude Code, and Codex.
2. Decide the correct owner slug for chipbot sessions using source evidence, not path-name vibes.
3. Make the chipbot symlink corpus discoverable, parsed, persisted, and searchable.
4. Keep symlink traversal bounded and non-recursive enough to avoid the traversal risk spec 005 addressed.
5. Add regression coverage for the filename shape, symlink/root shape, nested message format, and lexical search proof.
6. Avoid new `rusqlite` code.

## Constraint

- Do not delete or rewrite existing user session files.
- Do not mutate live CASS data during issue/spec creation.
- Do not fold this into spec 016 completion unless spec 016 explicitly takes on the bonus chipbot coverage work.
- If the fix belongs in `franken_agent_detection`, make it durable through the dependency path rather than a local-only CASS workaround.

## Acceptance Criteria

1. A focused scratch index of the chipbot symlink path produces nonzero conversations and messages.
2. `cass search` in robot lexical mode finds at least three safe strings from chipbot source files and returns the expected `source_path`.
3. Regression tests fail on the current zero-row behavior and pass with the fix.
4. The selected agent slug is documented and stable in tests/goldens where relevant.
5. Existing Pi Agent, Clawdbot, and priority spec 016 canaries do not regress.
6. The issue receipt records whether this remains bonus coverage or becomes part of a broader "all sessions" closeout gate.

## Out of Scope

- Promoting the spec 016 shadow archive to live CASS.
- Repairing the live malformed CASS DB.
- Rewriting the general session manifest policy for every connector.
- Semantic model installation or semantic search proof.

## Selected Shape

Direct root-cause connector fix with focused scratch-index and lexical-search regression coverage. Start by classifying the chipbot format and owner slug, then change the smallest connector layer that can discover and parse the corpus without creating a one-off CASS storage workaround.
