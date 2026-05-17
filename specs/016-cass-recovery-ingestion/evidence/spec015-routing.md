---
title: "Spec 015 routing for spec 016 recovery"
date: 2026-05-17T00:25:43Z
bead: coding_agent_session_search-1vxuf
---

# Spec 015 Routing

## Decision

`specs/015-watch-once-streaming-scan/` is subordinate evidence for this recovery. It is not the product-level owner of Dale's current goal, and it must not be treated as a separate path to "done" for CASS session ingestion.

## Evidence

- Spec 016 owns the product-level recovery: upstream sync, priority-agent ingestion/searchability for Pi Agent, Claude Code, and Codex, OpenCode/factory non-regression, and a live `com.cass.index-watch` proof.
- Spec 016 explicitly warns not to let spec 015's implementation state substitute for product-level acceptance.
- Spec 016 plan rejects "finish spec 015 first" as the main route because completing the Pi-only watch-once slice repeats the earlier failure mode.
- Spec 015's own GoalBuddy board remains active at `T003`; it has code-verify, finalize, and final audit tasks queued.
- Spec 015 `tasks.md` still leaves full-corpus verification, watcher reload, no-regression proof, implementation receipt, implement gate, code-verify, finalize, and push unchecked.
- Current spec 016 live evidence still shows the product outcome is blocked: live DB freelist errors, live `pi_agent=1077`, `com.cass.index-watch` absent, upstream unresolved at `19/19`, and no live watcher proof.

## Routing

Spec 015 may be cited only as implementation/provenance evidence for the Pi Agent watch-once scanning slice after spec 016's live verification passes. It does not close this recovery by itself.

Do not close or supersede spec 015 from this file. Leave its own board active until its verifier/finalize path is handled or until Dale explicitly directs a closure route. For spec 016, the completion test remains the live evidence packet described in `completion-audit.md`, not spec 015 artifact completion.

## Status

`subordinate`: useful evidence, not completion.
