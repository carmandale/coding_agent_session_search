#!/usr/bin/env python3
"""Recover the issues that exist only in coding_agent_session_search's
pre-cutover database and are absent from the live tracker.

The two sides diverged in March 2026 when the bd->br migration normalized dots
out of issue ids and br 0.1.24 then refused to export ("stale database that
would lose issues"), so neither side has been authoritative since.

Twin matching runs dotted->normalized ONLY. The reverse is not a function:
nothing in `0lyd3` says where the dot was, and counting that direction reports
every twin as a loss (it produced both the 130 and the 245 figures earlier).

Emits candidate JSONL lines on stdout. It does not touch the repository; the
caller stages the result through a scratch database first.
"""
import json
import sqlite3
import sys
from datetime import datetime, timezone
from pathlib import Path

REPO = Path(sys.argv[1] if len(sys.argv) > 1 else ".")
BACKUP = REPO / ".beads/.pre-migration-backup/beads.db"
LIVE = REPO / ".beads/beads.db"

# br omits empty fields on export rather than writing nulls (measured: only
# 10 of 27 keys are present on all 1758 live records), so the generated lines
# follow that policy or the rebuild produces a spurious diff on every issue.
ALWAYS = ("id", "title", "status", "priority", "issue_type",
          "created_at", "updated_at", "source_repo",
          "compaction_level", "original_size")
OPTIONAL = ("description", "design", "acceptance_criteria", "notes",
            "assignee", "owner", "created_by", "closed_at", "close_reason",
            "deleted_at", "deleted_by", "delete_reason", "original_type")


def ro(path):
    return sqlite3.connect(f"file:{path}?mode=ro", uri=True)


def ts(value):
    """Normalize to the seconds-precision Z form the live export uses.

    The backup carries nanosecond offsets (`...579678659+00:00`) from the bd
    era. 0.2.x validates timestamps on import where 0.1.24 accepted silently,
    so hand the importer the shape 1758 live records already use.
    """
    if not value:
        return None
    raw = str(value).strip()
    cleaned = raw.replace("Z", "+00:00")
    if "." in cleaned:  # trim sub-second precision to microseconds for fromisoformat
        head, _, tail = cleaned.partition(".")
        digits = "".join(c for c in tail if c.isdigit())
        offset = tail[len(digits):]
        cleaned = f"{head}.{digits[:6]}{offset}"
    try:
        dt = datetime.fromisoformat(cleaned)
    except ValueError:
        return raw  # unparseable: pass through and let br reject it loudly
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)
    return dt.astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def main():
    backup, live = ro(BACKUP), ro(LIVE)
    live_ids = {r[0] for r in live.execute("SELECT id FROM issues")}
    twins = {i.replace(".", "d") for i in live_ids if "." in i}
    backup_rows = {r["id"]: r for r in
                   (dict(zip([c[0] for c in cur.description], row))
                    for cur in [backup.execute("SELECT * FROM issues")]
                    for row in cur)}

    missing = sorted(set(backup_rows) - live_ids - twins)
    merged = live_ids | set(missing)

    labels, deps, comments = {}, {}, {}
    for iid, label in backup.execute("SELECT issue_id,label FROM labels"):
        labels.setdefault(iid, []).append(label)
    cols = [c[1] for c in backup.execute("PRAGMA table_info(dependencies)")]
    for row in backup.execute("SELECT * FROM dependencies"):
        d = dict(zip(cols, row))
        deps.setdefault(d["issue_id"], []).append(d)
    ccols = [c[1] for c in backup.execute("PRAGMA table_info(comments)")]
    for row in backup.execute("SELECT * FROM comments"):
        c = dict(zip(ccols, row))
        comments.setdefault(c["issue_id"], []).append(c)

    dropped_deps = []
    for iid in missing:
        src = backup_rows[iid]
        rec = {}
        for k in ALWAYS:
            v = src.get(k)
            if k in ("created_at", "updated_at"):
                v = ts(v)
            if k == "compaction_level":
                v = v or 0
            if k == "original_size":
                v = v or 0
            if k == "source_repo":
                v = v or ""
            rec[k] = v
        for k in OPTIONAL:
            v = src.get(k)
            if k in ("closed_at", "deleted_at"):
                v = ts(v)
            if v not in (None, "", 0):
                rec[k] = v
        if labels.get(iid):
            rec["labels"] = sorted(labels[iid])
        if comments.get(iid):
            rec["comments"] = [
                {"id": c["id"], "issue_id": c["issue_id"], "author": c["author"],
                 "text": c["text"], "created_at": ts(c["created_at"])}
                for c in sorted(comments[iid], key=lambda c: c["id"])
            ]
        kept = []
        for d in deps.get(iid, []):
            # A dependency whose target is absent aborts the whole import
            # (measured in Continuous-Claude-v3: rc=4 on the first bad ref),
            # so drop it here and report rather than losing the other 127.
            if d["depends_on_id"] not in merged:
                dropped_deps.append((iid, d["depends_on_id"]))
                continue
            kept.append({"issue_id": d["issue_id"],
                         "depends_on_id": d["depends_on_id"],
                         "type": d["type"] or "blocks",
                         "created_at": ts(d["created_at"]),
                         "created_by": d["created_by"] or "import",
                         "metadata": d["metadata"] or "{}",
                         "thread_id": d["thread_id"] or ""})
        if kept:
            rec["dependencies"] = kept
        print(json.dumps(rec, ensure_ascii=False))

    print(f"generated {len(missing)} records", file=sys.stderr)
    print(f"dropped {len(dropped_deps)} unresolvable dependencies", file=sys.stderr)
    for pair in dropped_deps:
        print(f"  {pair[0]} -> {pair[1]}", file=sys.stderr)


if __name__ == "__main__":
    main()
