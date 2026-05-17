---
title: "Release candidate shadow proof"
date: 2026-05-17T03:59:00Z
bead: coding_agent_session_search-1vxuf
---

# Release Candidate Shadow Proof

This proof runs the release candidate binary against the verified shadow archive. It does not install the binary and does not touch the live cass data dir.

Previous release candidate from the 2026-05-17T00:47:53Z refresh:

```text
path: /tmp/cass-release-target/release/cass
version: cass 0.4.7
sha256: db3dbb0a9652bc5cadfa9a7d824da13a529d9cd2ad6ad85dc169a0760b0a7f1c
```

Latest verifier refresh after changed-file UBS critical cleanup:

```text
path: /tmp/cass-release-target/release/cass
version: cass 0.4.7
sha256: 423e2e4c2920ec74a38a5cb4af1f00de362a4a82e493d342b4891179f4955ada
```

Latest release rebuild after spec018 watchdog command-surface repair and
`tests/cli_robot.rs` UBS critical cleanup:

```text
env CARGO_TARGET_DIR=/tmp/cass-release-target "$HOME/.cargo/bin/cargo" build --release --bin cass
result: pass

path: /tmp/cass-release-target/release/cass
size: 52M
version: cass 0.4.7
sha256: a5a139ca503f0f04f2b0baba06e20b16bd7003a7e7d78306cacf94d56eaca9c2
```

Release watchdog command-surface proof:

```text
/tmp/cass-release-target/release/cass watchdog run --help
result: exit 0
observed: "Run a one-shot health check (heartbeat + log rotation + restart if stale)"
```

Shadow data dir:

```text
/Users/dalecarman/Library/Application Support/com.coding-agent-search.coding-agent-search-spec016-shadow-20260516T2025Z
```

Health:

```text
/tmp/cass-release-target/release/cass health --json --stale-threshold 1800 --data-dir "$SHADOW"
status=unhealthy
index.status=stale
checkpoint.completed=true
checkpoint.db_matches=true
```

Interpretation: the 30-minute threshold reports stale because no shadow watcher is running and `last_indexed_at` has aged. This is expected for a stopped shadow archive and is not the same as a corrupt or missing lexical index.

Bounded readiness check:

```text
/tmp/cass-release-target/release/cass health --json --stale-threshold 86400 --data-dir "$SHADOW"
status=healthy
healthy=true
index.status=ready
index.fresh=true
checkpoint.present=true
checkpoint.completed=true
checkpoint.db_matches=true
pending.sessions=0
pending.watch_active=false
```

Latest bounded readiness check after release rebuild:

```text
/tmp/cass-release-target/release/cass health --json --stale-threshold 86400 --data-dir "$SHADOW"
healthy=true
status=healthy
index.status=ready
index.fresh=true
checkpoint.completed=true
checkpoint.db_matches=true
```

Lexical search canaries:

```text
pi_agent    "ATT21_COL_CFP_SceneMachine_EndCard.psd" total_matches=30   elapsed_ms=24 search_ms=1
claude_code "frankensqlite"                         total_matches=37   elapsed_ms=23 search_ms=0
codex       "freelist serializer"                   total_matches=10   elapsed_ms=23 search_ms=0
opencode    "opencode"                              total_matches=2484 elapsed_ms=23 search_ms=0
factory     "factory"                               total_matches=21   elapsed_ms=23 search_ms=0
```

Latest release lexical canaries after release rebuild:

```text
pi_agent    "ATT21_COL_CFP_SceneMachine_EndCard.psd" total_matches=30   first.source_path=/Users/dalecarman/.pi/agent/sessions/--Users-dalecarman-dev-dropbox-cli--/2026-04-01T08-29-10-484Z_8e2e91b7-b93d-42ce-9545-f99d648c6abc.jsonl
claude_code "frankensqlite"                         total_matches=37   first.source_path=/Users/dalecarman/.claude/projects/-Users-dalecarman-dev-coding-agent-session-search/99ea43d3-aa1d-4d45-be56-6fac233c3723.jsonl
codex       "freelist serializer"                   total_matches=10   first.source_path=/Users/dalecarman/.codex/sessions/2026/05/15/rollout-2026-05-15T20-32-54-019e2e6a-3bdc-7313-9917-ddc766a1eb9d.jsonl
opencode    "opencode"                              total_matches=2484 first.source_path=/Users/dalecarman/.local/share/opencode/storage/session/5b0cb53f3d1aca39a750a401f9e5d51a0c3fed55/ses_4d17229b6ffeYWnMOFmKq28T9F.json
factory     "factory"                               total_matches=21   first.source_path=/Users/dalecarman/.factory/sessions/-Users-dalecarman-Groove Jones Dropbox-Dale Carman-Projects-dev-PfizerOutdoCancerV2/f3cbeb11-6447-4b95-a20c-4fd527b4334d.jsonl
```

Conclusion:

The release candidate can read and search the verified shadow archive, and the
release binary now exposes the watchdog command surface. It is ready for
approval-gated install testing, but it is not installed and therefore does not
satisfy live acceptance yet.
