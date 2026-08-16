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
    python3 catchup-manifest.py <output-manifest-path> --acceptance-since <ISO8601>

``--acceptance-since`` turns this from a work list into a pass/fail test, and it
exists because **unindexed == 0 is not a state this archive can hold.** New
sessions land continuously — ``claude_code`` on-disk rose from 7,682 to 7,755
during the 2026-08-16 catch-up, including the session running the catch-up — so a
test demanding zero fails forever against a system that is behaving correctly.
That is a defect in the criterion, not in cass, and it is the same shape as the
``journal.jsonl`` exclusion below: an acceptance test that can never pass tells
you nothing.

The honest bound is quiescence. A file whose mtime predates the run start has
been sitting still since before we began, so the run had every opportunity to
index it; if it is still unindexed, that is a real hole and this exits 1. A file
touched since the run started is either new or still being written, and belongs
to the next incremental pass rather than to this one's verdict.

mtime is the right clock here rather than any timestamp inside the file: it
answers "could this run have seen it", which is exactly the question, and it
needs no parse of a format that varies per connector.
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


def parse_since(raw: str) -> float:
    """ISO8601 -> POSIX seconds. Refuses rather than guessing on a bad value.

    A silently-misparsed bound would classify every stale file as fresh and
    report a clean pass over an unindexed archive, which is the one failure this
    whole flag exists to prevent.
    """
    text = raw.strip().replace("Z", "+00:00")
    try:
        when = dt.datetime.fromisoformat(text)
    except ValueError:
        raise SystemExit(
            f"REFUSING: --acceptance-since {raw!r} is not ISO8601 "
            f"(want e.g. 2026-08-16T09:14:00Z)"
        )
    if when.tzinfo is None:
        when = when.replace(tzinfo=dt.timezone.utc)
    now = dt.datetime.now(dt.timezone.utc)
    if when > now:
        raise SystemExit(
            f"REFUSING: --acceptance-since {raw!r} is in the future; every file "
            f"would classify as stale and the test would fail for the wrong reason"
        )
    return when.timestamp()


def main() -> int:
    argv = sys.argv[1:]
    since: float | None = None
    since_raw = ""
    if "--acceptance-since" in argv:
        i = argv.index("--acceptance-since")
        if i + 1 >= len(argv):
            print("REFUSING: --acceptance-since needs a value", file=sys.stderr)
            return 2
        since_raw = argv[i + 1]
        since = parse_since(since_raw)
        del argv[i : i + 2]
    if len(argv) != 1:
        print(__doc__, file=sys.stderr)
        return 2
    out = pathlib.Path(argv[0])

    stamp = dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    print(f"manifest built {stamp}", flush=True)
    if since is not None:
        print(f"acceptance bound: files quiescent since {since_raw}", flush=True)

    entries: list[tuple[int, str]] = []
    stale_holes: list[str] = []
    fresh_arrivals = 0
    for slug, (root, patterns) in TREES.items():
        rows = query(
            "SELECT source_path FROM conversations c "
            "JOIN agents a ON a.id = c.agent_id "
            f"WHERE a.slug = '{slug}'"
        )
        indexed = {r[0] for r in rows if r[0]}

        on_disk: dict[str, tuple[int, float]] = {}
        skipped = 0
        for pattern in patterns:
            for path in root.rglob(pattern):
                if path.name in NON_CONVERSATION_BASENAMES:
                    skipped += 1
                    continue
                try:
                    st = path.stat()
                    on_disk[str(path)] = (st.st_size, st.st_mtime)
                except OSError:
                    pass

        missing = [(size, p) for p, (size, _) in on_disk.items() if p not in indexed]
        entries.extend(missing)
        total_bytes = sum(size for size, _ in missing)

        stale = fresh = 0
        if since is not None:
            for _, p in missing:
                if on_disk[p][1] < since:
                    stale_holes.append(p)
                    stale += 1
                else:
                    fresh += 1
            fresh_arrivals += fresh

        print(
            f"  {slug:13s} on_disk={len(on_disk):6d} indexed={len(indexed):6d} "
            f"unindexed={len(missing):6d}  ({total_bytes / 2**30:.2f} GiB)"
            + (f"  [skipped {skipped} non-conversation]" if skipped else "")
            + (f"  -> {stale} stale, {fresh} arrived during the run" if since is not None else ""),
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

    if since is None:
        return 0

    print(flush=True)
    if stale_holes:
        print(
            f"  FAIL: {len(stale_holes)} file(s) were quiescent before "
            f"{since_raw} and are still unindexed. These are real holes.",
            flush=True,
        )
        for p in stale_holes[:10]:
            print(f"    {p}", flush=True)
        if len(stale_holes) > 10:
            print(f"    ... and {len(stale_holes) - 10} more", flush=True)
        return 1

    print(
        f"  PASS: every file quiescent before {since_raw} is indexed. "
        f"{fresh_arrivals} file(s) arrived during the run and belong to the "
        f"next incremental pass.",
        flush=True,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
