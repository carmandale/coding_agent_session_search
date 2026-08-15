#!/usr/bin/env python3
"""Build the catch-up manifest: every file on disk that has no conversation row.

Read-only. Opens the archive with ``mode=ro`` and never writes it.

Why a set-diff and not ``connector_coverage.complete``: that field is computed
from the connector scan floors, the archive has no ``connector_scan_floors``
meta row at all, and an empty floor map renders as ``"complete": true``. It is
structurally incapable of reporting this hole — bead
``coding_agent_session_search-1a7mk``. Counting files is the only honest signal.

Why file paths and never day-directories: a day-directory argument mints
non-canonical external_ids that silently duplicate against
``idx_conversations_provenance`` on any later scan. The generation-2 backfill
learned this; the note is in ``backfill.sh``'s own header.

Smallest file first, so cheap batches prove the path before the expensive tail.

Usage:
    python3 catchup-manifest.py <output-manifest-path>
"""

from __future__ import annotations

import datetime as dt
import os
import pathlib
import sqlite3
import sys
import time

DB = os.path.expanduser(
    "~/Library/Application Support/"
    "com.coding-agent-search.coding-agent-search/agent_search.db"
)

# slug -> (root, glob patterns). Matches the connectors that actually hold rows
# in this archive; adding a tree here is safe, it just widens the diff.
TREES: dict[str, tuple[pathlib.Path, tuple[str, ...]]] = {
    "claude_code": (pathlib.Path.home() / ".claude" / "projects", ("*.jsonl",)),
    "codex": (
        pathlib.Path.home() / ".codex" / "sessions",
        ("rollout-*.jsonl", "rollout-*.json"),
    ),
}

# Basenames that live under a connector root, match its glob, and are NOT
# conversations. Excluding them is not a convenience: leaving them in makes the
# acceptance test unreachable, because they can never acquire a conversation row
# however many times they are scanned, so the unindexed count never reaches zero.
#
# `journal.jsonl` is the Claude Code workflow bookkeeping stream. Measured
# 2026-08-15: its records are `{type, key, agentId}` with no message, no
# timestamp and no conversation content, against `agent-*.jsonl` which carries
# message/timestamp/sessionId. 0 of the 4,050 indexed `claude_code` rows come
# from a `journal.jsonl`, while 3,268 of them come from sibling `agent-*.jsonl`
# transcripts in the same `subagents/workflows/` directories — so the connector
# is discriminating by content, not skipping the directory.
NON_CONVERSATION_BASENAMES = {"journal.jsonl"}


def query(sql: str, tries: int = 6):
    """Read-only query with retry.

    A concurrent indexer holds the write lock in bursts, so ``database is
    locked`` here is expected background noise rather than corruption.
    """
    for attempt in range(tries):
        try:
            conn = sqlite3.connect(f"file:{DB}?mode=ro", uri=True, timeout=30)
            try:
                return conn.execute(sql).fetchall()
            finally:
                conn.close()
        except sqlite3.OperationalError:
            if attempt == tries - 1:
                raise
            time.sleep(3)
    raise AssertionError("unreachable")


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    out = pathlib.Path(sys.argv[1])

    stamp = dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    print(f"manifest built {stamp}", flush=True)

    entries: list[tuple[int, str]] = []
    for slug, (root, patterns) in TREES.items():
        rows = query(
            "SELECT source_path FROM conversations c "
            "JOIN agents a ON a.id = c.agent_id "
            f"WHERE a.slug = '{slug}'"
        )
        indexed = {r[0] for r in rows if r[0]}

        on_disk: dict[str, int] = {}
        skipped = 0
        for pattern in patterns:
            for path in root.rglob(pattern):
                if path.name in NON_CONVERSATION_BASENAMES:
                    skipped += 1
                    continue
                try:
                    on_disk[str(path)] = path.stat().st_size
                except OSError:
                    pass

        missing = [(size, p) for p, size in on_disk.items() if p not in indexed]
        entries.extend(missing)
        total_bytes = sum(size for size, _ in missing)
        print(
            f"  {slug:13s} on_disk={len(on_disk):6d} indexed={len(indexed):6d} "
            f"unindexed={len(missing):6d}  ({total_bytes / 2**30:.2f} GiB)"
            + (f"  [skipped {skipped} non-conversation]" if skipped else ""),
            flush=True,
        )

    # A comma is the --watch-once separator, so a path containing one cannot be
    # passed through that interface. Refuse loudly instead of silently splitting
    # one path into two nonexistent ones.
    commas = [p for _, p in entries if "," in p]
    if commas:
        print(
            f"REFUSING: {len(commas)} path(s) contain a comma, which is the "
            f"--watch-once separator. First: {commas[0]}",
            file=sys.stderr,
        )
        return 1

    entries.sort()  # smallest file first
    out.write_text("\n".join(p for _, p in entries) + "\n")
    total = sum(size for size, _ in entries)
    print(
        f"  wrote {len(entries)} paths ({total / 2**30:.2f} GiB) to {out}",
        flush=True,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
