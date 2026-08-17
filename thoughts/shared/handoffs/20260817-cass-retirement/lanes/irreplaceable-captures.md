# Lane: irreplaceable-capture analysis (cass retirement, step 1)

- **Lane**: irreplaceable-captures — safety-critical, gates every deletion
- **Owner**: Claude Opus 5 (1M) subagent, session `656f2411-6418-4df9-9965-55219cd71762`
- **Date**: 2026-08-17
- **Write permissions**: this log only. Everything else read-only. No cass binary was run.
  No file was deleted, moved, renamed, edited; no bead touched.
- **Stop condition**: exact irreplaceable set established with controls, extraction recipe written.

---

## VERDICT

**6,892 files / 1,890,766,050 bytes (1.76 GiB) are irreplaceable.** They exist nowhere on
this machine except inside the cass raw-mirror. They represent **3,753 deleted sessions**:

| population | sessions | files | bytes | source mtime range (UTC) |
|---|---|---|---|---|
| Claude Code — **whole deleted sessions** | 738 | 3,877 | 1,787.28 MiB | 2026-04-23 .. 2026-07-13 |
| openclaw `feature-dev-planner` | 1,533 | 1,533 | 8.23 MiB | 2026-02-10 .. 2026-02-24 |
| openclaw `feature-dev-developer` | 1,482 | 1,482 | 7.66 MiB | 2026-02-10 .. 2026-02-24 |
| **total** | **3,753** | **6,892** | **1,803.18 MiB** | |

**Extract these before deleting anything.** The rest of the 77 GB production data
directory holds nothing that is not either still live on disk or derivable — that
includes the 23.3 GB `agent_search.db`, the 3.98 GB `~/backups/cass` vacuum, the
tantivy `index/` shards, and the 29 GB `/private/tmp/fsq-probe-data/*.db`. Proof for
that claim is in *"Do the search DBs hold anything the mirror does not?"* below.

Runnable extraction script (dry-run by default, verified against the mirror):
`/private/tmp/claude-501/-Users-dalecarman-dev-coding-agent-session-search/656f2411-6418-4df9-9965-55219cd71762/scratchpad/rescue-unique-captures.py`
— full text embedded at the end of this log, because the scratchpad is reaped.

---

## Mirror inventory (measured, full pass — not sampled)

```
manifests scanned                        147,844   (0 unparseable)
distinct blob hashes referenced          140,344
.raw blob files on disk                  140,344   (0 referenced-but-missing, 0 orphaned)
blob bytes on disk                        45.24 GB
manifests/ dir on disk                      578 MB
compression.state                        none  x147,844   (100%)
encryption.state                         none  x147,844   (100%)
verification.status                      captured x147,844
blob_size_bytes != source_size_bytes           0
mirror captured_at range                 2026-06-01 11:21 UTC .. 2026-08-16 14:51 UTC
source_mtime range across all captures   2024-08-28 22:13 UTC .. 2026-08-16 14:41 UTC
```

Because every blob is `compression=none` / `encryption=none`, **a blob's bytes ARE the
original file's bytes**. A plain `cp` is a full restoration; no cass code is needed to
read anything.

### Classification of all 147,844 captures

| class | meaning | manifests | bytes (blob_size, with dup) |
|---|---|---|---|
| A | `original_path` still exists on disk | 140,952 | 49.30 GB |
| B | `original_path` is **GONE** — irreplaceable candidate | 6,892 | 1.76 GiB |
| C | manifest unparseable, or blob missing from disk | **0** | 0 |
| C | path state undeterminable (permission / TCC) | **0** | 0 |

Class C is empty in both senses. Every manifest parsed, every referenced blob is on
disk, and every `original_path` gave a definite live-or-gone answer. `lstat` errors were
separated by `errno`: only `ENOENT`/`ENOTDIR`/`ENAMETOOLONG` count as gone — a permission
error would have been reported as `unknown` and would have blocked the verdict. None occurred.

### Class B per provider

| provider | manifests | bytes |
|---|---|---|
| `claude_code` | 3,877 | 1.75 GB |
| `openclaw/feature-dev-planner` | 1,533 | 8.23 MB |
| `openclaw/feature-dev-developer` | 1,482 | 7.66 MB |
| every other provider (`opencode` 113,027, `codex` 10,340, `claude` 7,509, `pi_agent` 1,958, `cursor` 261, `factory` 152, `amp` 37, `hermes` 1) | **0** | 0 |

Every `opencode`, `codex`, `claude`, `pi_agent`, `cursor`, `factory`, `amp` and `hermes`
capture in the mirror still has its original on disk. Those providers contribute nothing
to the irreplaceable set.

---

## Are the bytes really nowhere else? (step 2)

blake3 is not available on this machine (no `b3sum`, no python `blake3` module), so the
content test was done exactly without it:

1. Index every live file by **exact byte size**.
2. For each class-B blob, look up its size.
   - no live file of that size → the content certainly does not exist → unique.
   - live files of that size → **sha256-compare** the blob against each candidate.

This is exact, not probabilistic. Run twice, against two different index widths.

**Run 1 — the provider roots the manifests themselves name** (derived from
`original_path` values, not guessed): `~/.claude/projects`, `~/.codex/sessions`,
`~/.local/share/opencode`, `~/.local/share/amp`, `~/.pi/agent/sessions`,
`~/.openclaw/agents`, `~/.cursor`, `~/.hermes`, `~/Library/Application Support/Cursor`,
and each `~/.factory/sessions/<project>`.

```
live files indexed        229,311   (49,059 distinct sizes)
content found elsewhere         0
unique (no live file of that size)              3,799   1,763.52 MB
unique (size collided, bytes differ)            3,093      39.65 MB
```

**Run 2 — deliberately wider**, so the negative is harder to be wrong. Added every
agent-session store on the machine plus every place a stray copy could plausibly sit:
`~/.codex/archived_sessions`, `~/.pi` (whole), `~/.openclaw` (whole), `~/.cursor`,
`~/.amp`, `~/.factory` (whole), `~/.hermes`, `~/.cass-catchup`, `~/backups/cass`,
`~/dev/coding_agent_session_search/.claude/worktrees` (123,375 files), `/tmp` and
`/private/tmp` (1.25M files each, i.e. all the cass build residue). The mirror itself was
excluded from the index — it is the thing under test.

```
live files indexed      2,906,493   (96,606 distinct sizes)
content found elsewhere         0
unique (no live file of that size)              3,467   1,728.76 MB
unique (size collided, bytes differ)            3,425      74.41 MB
```

**All 6,892 are truly unique. Zero matches, twice, the second time against 2.9 million files.**

A second, independent signal points the same way: the mirror is content-addressed, so if a
deleted file's bytes also existed at a live path *that cass had captured*, both captures
would share one blob hash. Measured across all 147,844 manifests — `hashes seen with a
GONE source path` = 6,892, `hashes seen with a LIVE source path` = 133,452, **intersection
= 0**.

### No backup copies exist either

`tmutil destinationinfo` → `No destinations configured`. There is no Time Machine backup
on this machine. Nothing else is holding these bytes.

---

## What the unique set actually is (step 3)

```
BY KIND
  claude_subagent_transcript      3,139 files    662.72 MiB
  claude_top_level_session          738 files  1,124.56 MiB
  openclaw_agent_session_file     3,015 files     15.89 MiB
```

**These are whole deleted sessions, not fragments.** All 738 unique Claude Code sessions
have their top-level `<uuid>.jsonl` captured; 155 of them additionally carry their
subagent transcripts. Zero sessions are subagent-only orphans, and zero sessions have a
parent `.jsonl` that is still live — checked per session: **738 of 738 `parent_GONE`**.
So nothing here is a stray piece of a session you still have.

The openclaw side is a whole directory tree that no longer exists: `~/.openclaw/agents/`
survives but contains only `main`, whose `sessions/` is absent, and there is no
`feature-dev-planner` or `feature-dev-developer` at all. Each of those 3,015 files is one
complete session.

Monthly distribution of the unique captures by `source_mtime`:

```
2026-02   3,015 files      15.89 MiB     (openclaw, whole tree deleted)
2026-04       1 files       0.04 MiB
2026-05     489 files     613.50 MiB
2026-06   1,546 files     547.89 MiB
2026-07   1,841 files     625.85 MiB
```

The Claude Code range stops at **2026-07-13**, 35 days before today. That is consistent
with a ~30-day retention reaper in the harness, which means **this loss is ongoing** —
sessions keep aging out of `~/.claude/projects` every day. Worth stating plainly against
Dale's larger objective ("I want to capture all sessions"): the mirror is the only thing
that has been catching them, so whatever replaces cass needs to keep doing this, and the
gap between today and the replacement is unrecoverable loss.

### Deleted Claude Code sessions by project (64 projects, top 20 by bytes)

```
 136 sessions    747.38 MiB  -Users-dalecarman-dev-PfizerOutDoCancerV3
 170 sessions    190.35 MiB  -Users-dalecarman--agent-config
  11 sessions    116.56 MiB  ...Projects-SCH-8880
  23 sessions     93.55 MiB  -Users-dalecarman-dev-groove-sight-platform
  96 sessions     92.20 MiB  ...Projects-dev-wiki
  15 sessions     69.06 MiB  -Users-dalecarman-dev-hsbc
 109 sessions     63.87 MiB  ...Projects-dac-wiki
   6 sessions     40.63 MiB  -Users-dalecarman-dev-gjdraw
   8 sessions     32.82 MiB  -Users-dalecarman-dev-operator
   5 sessions     30.64 MiB  -Users-dalecarman-dev-groove-day
  12 sessions     29.66 MiB  -Users-dalecarman-dev-gj-tool
   8 sessions     28.75 MiB  -Users-dalecarman-dev-quickbooks
   2 sessions     27.95 MiB  -Users-dalecarman-conductor-workspaces-dale-admin-app-florence
   3 sessions     25.61 MiB  -Users-dalecarman-dev-coding-agent-session-search
   6 sessions     22.77 MiB  -Users-dalecarman-dev-orchestrator
   7 sessions     21.88 MiB  -Users-dalecarman-dev-groove-sight
   6 sessions     21.33 MiB  ...Projects-razorfish
   1 sessions     14.76 MiB  ...Projects-IDSA-talk
   1 sessions     11.54 MiB  -Users-dalecarman-conductor-workspaces-PfizerOutDoCancerV3-zagreb
   1 sessions     10.00 MiB  -Users-dalecarman-conductor-workspaces-lucy-bogota
```

Per-session inventory (3,753 rows: provider, project, session id, files, bytes, first/last
mtime) was written to
`…/scratchpad/unique-sessions-inventory.csv`. Regenerate it with `group_sessions.py`
(embedded below) if the scratchpad is gone.

Largest single unique captures:

```
39.31 MiB  ~/.claude/projects/...Projects-SCH-8880/73fb29da-2bdf-4572-a892-632de6b81afd.jsonl
21.41 MiB  ~/.claude/projects/...Projects-SCH-8880/17e71497-1ace-4b26-97f7-339a611743ba.jsonl
17.41 MiB  ~/.claude/projects/-Users-dalecarman-dev-PfizerOutDoCancerV3/14ac37fc-....jsonl
16.31 MiB  ~/.claude/projects/...Projects-SCH-8880/bcbbbc62-....jsonl
15.57 MiB  ~/.claude/projects/-Users-dalecarman-conductor-workspaces-dale-admin-app-florence/34dc2426-....jsonl
15.41 MiB  ~/.claude/projects/-Users-dalecarman--agent-config/25f42ed7-....jsonl
```

---

## Do the search DBs hold anything the mirror does not?

This is the one gap that could have been missed and it had to be closed: **the mirror
started 2026-06-01, but the DB is older** (`conversations.started_at` min =
2025-09-15). A session indexed in, say, March 2026 and deleted in April would have DB text
and no mirror blob.

Measured directly. Both DBs opened read-only and immutable with plain `/usr/bin/sqlite3`
(no cass binary, no writes, no `-shm` created):

| DB | conversations | source live + has manifest | source gone + has manifest | **source gone + NO manifest** |
|---|---|---|---|---|
| live `agent_search.db` (23.3 GB) | 27,441 | 20,689 | 6,751 | **1** |
| `~/backups/cass/agent_search-20260814-vacuum.db` (3.98 GB) | 12,722 | 5,970 | 6,751 | **1** |

That single row is not a real file. Its `source_path` is a pseudo-path —
`~/Library/Application Support/Cursor/User/workspaceStorage/0185d4989af481f6a9cd6a9621240f1a/state.vscdb/aichat-workbench.panel.aichat.view.aichat.chatdata`
— where `state.vscdb` is itself a SQLite file and the last component is a key inside it.
That file **exists** (39,997,440 bytes, plus a `.backup`), and the key is present with
2,916,359 bytes of value. The content is live in Cursor's own store.

**Conclusion: the DBs contain nothing irreplaceable.** Once the 6,892 blobs are extracted,
`agent_search.db`, the Aug-14 vacuum backup, the tantivy `index/` tree, and the
`/private/tmp/fsq-probe-data/*.db` copies are all derived and safe to delete.

`~/.cass-catchup/` (37 MB, 79 files) was also checked: `batch-aa`…`batch-cz` are plain
**lists of file paths** to feed the indexer, plus `manifest.txt`, `run.log`, `nohup.out`.
No session content. Safe.

---

## Sanity controls (step 4) — all pass

### C1 FIDELITY — is the mirror faithful? 10/10 PASS
Ten random class-A captures. Compared blob vs still-live original on size, sha256 of the
first 64 KB, **and full sha256**. All ten matched on all three. Sample spans three
providers and four orders of magnitude of file size (224 B → 360,927 B), e.g.:

```
PASS size=186158/186158 first64k=match full=match
     blob blobs/blake3/87/87f793c7451b77aa4f069bb2f6684750801d0b7e5489ae7ad086c4514d5054da.raw
     orig ~/.codex/sessions/2026/07/16/rollout-2026-07-16T15-30-19-019f6c9f-....jsonl
PASS size=360927/360927 first64k=match full=match
     blob blobs/blake3/11/112dccde44247b399bc441d8909ef28b8898079fd30935d7295e6d3076cc65f4.raw
     orig ~/.claude/projects/-Users-dalecarman-dev-groove-sight-platform/bc026954-.../subagents/workflows/wf_98b2d75b-c29/agent-acdf32f88eb67fb1c.jsonl
```

### C2 ABSENCE — are the class-B paths really gone? 10/10 confirmed absent
Ten random class-B paths, each stat'd individually rather than trusting the scan loop, and
cross-checked with `/bin/test -e` (a different code path from `os.lstat`). All ten:
`lexists=False`, `rc=1`. Eight had an absent parent directory, two had a live parent
directory with the file itself missing — both shapes present, which is what you would
expect from a mix of directory-level and file-level deletion.

### C3 POSITIVE CONTROL — can the uniqueness matcher find anything at all? 12/12 FOUND
This is the control that makes "0 matches" mean something. The identical size-index +
sha256 matcher was pointed at 12 random **class-A** blobs, whose originals are known to be
on disk. It found all 12, including cases with 16,736 same-size candidates to sift.
**Matcher is LIVE**, so the zero on class B is a true negative, not a dead instrument.

### C4 BLOB INVENTORY — 0 missing, 0 orphaned
140,344 hashes referenced by manifests; 140,344 `.raw` files on disk; set difference empty
in both directions.

### An instrument that FAILED, reported so it is not mistaken for evidence
I also ran `mdfind -name <basename>` over ten class-B basenames looking for copies outside
the indexed roots, and got 0 hits each. **That result is worthless.** The positive control
— `mdfind -name` on the basename of a `~/.claude/projects` file that definitely exists —
also returned **0 hits**. Spotlight does not index these dotfile directories, so mdfind
cannot answer this question in either direction. It is excluded from the evidence above;
the filesystem walk in run 2 is what carries the negative.

---

## Honest boundaries on the "nowhere else" claim

State these to Dale rather than rounding them off:

1. The wide index covered agent-session stores, the repo worktrees, cass backups,
   `~/.cass-catchup`, and all of `/tmp` + `/private/tmp` — **2.9M files**. It did **not**
   walk `~/Groove Jones Dropbox`, `~/Documents`, `~/dev` at large, or external volumes. A
   byte-identical copy of a Claude Code session `.jsonl` sitting in one of those is
   conceivable but has no known mechanism; nothing in this repo or in agent-config copies
   session files there. If the coordinator wants that closed, the same script re-run with
   those roots appended takes about five extra minutes of walking.
2. The comparison is exact for content (sha256 over full bytes), so there is no hash-
   collision hand-waving. What is inferential is only *where* I looked.
3. Class B is a snapshot as of 2026-08-17 ~10:40 local. More files will have aged out by
   the time the coordinator runs the extraction, which is why the extraction script
   re-derives the set from the mirror at run time instead of reading my JSONL.

---

## Two incidental findings the coordinator should have

- **No cass process is running.** `ps -Ao pid,etime,command` shows nothing matching cass /
  coding-agent-search / fsq / lexical. But `…/index-run.lock` and `.lock.meta` claim
  `pid=75534`, `job_kind=lexical_refresh`, `phase=index`, `db_path=/private/tmp/fsq-probe-data/prod.db`,
  updated `Aug 17 10:31`. `ps -p 75534` → **exit 1, no such process**. It is a stale lock
  from this morning's lexical-refresh work, not a live owner. Nothing to wait for.
- **Disk is at 99%.** `/System/Volumes/Data`: 3.5 Ti used, **56 Gi available**. The
  extraction needs 1.76 GiB, which fits, but do the extraction *before* anything else and
  verify it before reclaiming.
- **The mirror is not a complete capture either** — relevant to designing the replacement,
  not to deletion. Live session files with no manifest: `claude_code` 1,838 (0.51 GiB),
  `opencode` 2,708 (3.21 GiB, mostly non-session metadata like `auth.json` /
  `storage/project/*.json`), `pi_agent` 136 (0.20 GiB), `codex` 65 (0.19 GiB). The newest
  of these are from today, i.e. ordinary staleness behind the last capture at
  2026-08-16 14:51 UTC. Nothing is lost by deleting the mirror, because these files are
  live.

---

## Deletion gate — what the coordinator must do, in order

1. Run the extraction dry run, confirm it reports **6,892 files / 1,890,766,050 bytes /
   392 directories** and reports **zero** skipped-unknown, zero blob-missing, zero
   unparseable. If any of those three is non-zero, stop and re-adjudicate — a
   compressed/encrypted blob or a permission error means the plain-copy assumption does
   not hold for that file, and the script refuses rather than writing corruption.
2. Run it with `--apply`. Expected: ~1.77 GiB written under
   `~/agent-session-archive/cass-rescued-v1/`, plus `index.tsv` and `README.md`.
   Numbers may be slightly *higher* than step 1 if the harness reaped more sessions in
   between; that is correct behavior, not drift.
3. Run it with `--verify`. It must print `0 mismatched / 0 missing` and exit 0.
4. **Only then** delete the raw-mirror, `agent_search.db`, `index/`,
   `~/backups/cass/`, `~/.cass-catchup/`, and `/private/tmp/fsq-probe-data/`.
5. The archive belongs outside the repo (`~/agent-session-archive/` by default) and must
   not be added to git — 1.76 GiB of session transcripts, some client-named
   (`SCH-8880`, `hsbc`, `razorfish`, `netapp-rfp`).

Expected output, precisely:

```
files        6,892
bytes        1,890,766,050   (1.7609 GiB)
directories    392
layout       <dest>/<provider-slug>/<original path relative to $HOME>
             e.g. claude_code/.claude/projects/-Users-dalecarman--agent-config/0237cbd6-….jsonl
             e.g. openclaw-feature-dev-planner/.openclaw/agents/feature-dev-planner/sessions/7b5e5750-….jsonl
mtimes       restored from each capture's source_mtime_ms
index.tsv    archived_relpath, original_path, provider, bytes, source_mtime_ms,
             captured_at_ms, blob_blake3, sha256_verified, manifest_id
```

The dry run has already been executed and reproduces those numbers independently of the
analysis pass above (it re-scans all 147,844 manifests itself, 41 s).

---

## Method / reproducibility

Scripts, all read-only, in
`/private/tmp/claude-501/-Users-dalecarman-dev-coding-agent-session-search/656f2411-6418-4df9-9965-55219cd71762/scratchpad/`:

| file | what it does | runtime |
|---|---|---|
| `scan_manifests.py` | streams all 147,844 manifests → `manifests.jsonl`, classifying path + blob state | 87 s |
| `summarize.py` | class/provider/byte tallies, hash-overlap analysis, emits `manifests.jsonl.gone` | 30 s |
| `uniqueness.py` | size index over manifest-named roots + sha256 compare | 25 s |
| `uniqueness_wide.py` | same over 2.9M files incl. `/tmp` and worktrees | 110 s |
| `group_sessions.py` | session grouping, parent-liveness, date ranges, `unique-sessions-inventory.csv` | 5 s |
| `controls.py` | C1–C4 | 6 min |
| `rescue-unique-captures.py` | the extraction recipe (dry-run default, `--apply`, `--verify`) | 41 s dry |

Data products: `manifests.jsonl` (147,844 rows), `manifests.jsonl.gone` (6,892),
`gone-verdicts.jsonl`, `wide-verdicts.json`, `unique-sessions-inventory.csv`,
`db-live-paths.txt`, `db-bak-paths.txt`.

No full pass exceeded 15 minutes, so no statistical sampling was substituted for any
answer — every number above is an exact full-population count.

---

## Appendix: `rescue-unique-captures.py` (full text)

```python
#!/usr/bin/env python3
"""rescue-unique-captures.py -- extract ONLY the irreplaceable captures out of the
cass raw-mirror into a plain, cass-free archive tree.

An irreplaceable capture = a raw-mirror manifest whose original_path no longer
exists on disk. All 147,844 manifests are compression=none / encryption=none, so a
blob's bytes ARE the original file's bytes: a plain copy is a full restoration.

This script is SELF-CONTAINED. It re-derives the set from the mirror at run time
(it does not trust any earlier JSONL), so it can be re-run right before deletion
to pick up anything the harnesses reaped in the meantime.

    # 1. see what it would do (writes nothing)
    python3 rescue-unique-captures.py

    # 2. do it
    python3 rescue-unique-captures.py --apply

    # 3. prove the archive matches the mirror, before deleting the mirror
    python3 rescue-unique-captures.py --verify

Default destination: ~/agent-session-archive/cass-rescued-v1
Override with --dest <path>.

Expected as measured 2026-08-17: 6,892 files / 1,890,766,050 bytes (1.76 GiB)
across 392 directories -- 738 whole deleted Claude Code sessions (with 3,139 of
their subagent transcripts) and 3,015 deleted openclaw agent sessions.
"""
import argparse
import errno
import hashlib
import json
import os
import shutil
import sys
import time

MIRROR = os.path.expanduser(
    "~/Library/Application Support/com.coding-agent-search.coding-agent-search/raw-mirror/v1"
)
MANIFESTS = os.path.join(MIRROR, "manifests")
HOME = os.path.expanduser("~")
GONE_ERRNOS = {errno.ENOENT, errno.ENOTDIR, errno.ENAMETOOLONG}


def slug(provider):
    return (provider or "unknown").replace("/", "-").replace(os.sep, "-")


def rel_from_home(p):
    if p.startswith(HOME + "/"):
        return p[len(HOME) + 1:]
    return p.lstrip("/")


def sha256_file(p):
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for c in iter(lambda: f.read(1 << 20), b""):
            h.update(c)
    return h.hexdigest()


def source_is_gone(p):
    """True only when the path is provably absent. A permission error is NOT
    absence -- those are reported and skipped, never archived as 'gone'."""
    if not p:
        return None
    try:
        os.lstat(p)
        return False
    except OSError as e:
        if e.errno in GONE_ERRNOS:
            return True
        return None


def collect():
    rows = []
    skipped_unknown = []
    blob_missing = []
    unparseable = []
    n = 0
    t0 = time.time()
    with os.scandir(MANIFESTS) as it:
        for e in it:
            if not e.name.endswith(".json"):
                continue
            n += 1
            try:
                with open(e.path, "rb") as f:
                    m = json.load(f)
            except Exception as ex:
                unparseable.append((e.path, repr(ex)))
                continue
            g = source_is_gone(m.get("original_path"))
            if g is None:
                skipped_unknown.append(m.get("original_path"))
                continue
            if not g:
                continue
            comp = (m.get("compression") or {}).get("state")
            enc = (m.get("encryption") or {}).get("state")
            if comp != "none" or enc != "none":
                # a compressed/encrypted blob is not a plain copy -- refuse rather
                # than write a corrupt archive file
                skipped_unknown.append(
                    "%s (compression=%s encryption=%s -- NOT a plain copy)"
                    % (m.get("original_path"), comp, enc))
                continue
            bp = os.path.join(MIRROR, m["blob_relative_path"])
            if not os.path.exists(bp):
                blob_missing.append(m.get("original_path"))
                continue
            rows.append(m)
            if n % 20000 == 0:
                print("  scanned %d manifests (%.0fs), gone-so-far=%d"
                      % (n, time.time() - t0, len(rows)), flush=True)
    print("  scanned %d manifests in %.0fs" % (n, time.time() - t0))
    return rows, skipped_unknown, blob_missing, unparseable, n


def dest_for(dest, m):
    return os.path.join(dest, slug(m.get("provider")),
                        rel_from_home(m["original_path"]))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dest", default=os.path.join(HOME, "agent-session-archive",
                                                  "cass-rescued-v1"))
    ap.add_argument("--apply", action="store_true", help="actually write files")
    ap.add_argument("--verify", action="store_true",
                    help="re-check an existing archive against the mirror")
    args = ap.parse_args()
    dest = os.path.abspath(args.dest)

    if not os.path.isdir(MANIFESTS):
        sys.exit("mirror not found: %s" % MANIFESTS)

    print("mirror : %s" % MIRROR)
    print("dest   : %s" % dest)
    print("mode   : %s" % ("VERIFY" if args.verify else ("APPLY" if args.apply else "DRY RUN")))
    print("\nscanning manifests for captures whose original_path is gone...")
    rows, unknown, blob_missing, unparseable, total = collect()

    nbytes = sum(m.get("blob_size_bytes") or 0 for m in rows)
    dirs = {os.path.dirname(dest_for(dest, m)) for m in rows}
    prov = {}
    for m in rows:
        p = slug(m.get("provider"))
        a = prov.setdefault(p, [0, 0])
        a[0] += 1
        a[1] += m.get("blob_size_bytes") or 0

    print("\nTO RESCUE: %d files, %d bytes (%.2f GiB) into %d directories"
          % (len(rows), nbytes, nbytes / 1024 ** 3, len(dirs)))
    for p in sorted(prov):
        print("   %-34s %6d files  %10.2f MiB" % (p, prov[p][0], prov[p][1] / 1024 ** 2))
    if unknown:
        print("\n!! %d captures SKIPPED because their path state could not be "
              "determined (permission error, or a compressed/encrypted blob). "
              "These are NOT archived and the mirror must not be deleted until "
              "they are adjudicated:" % len(unknown))
        for u in unknown[:20]:
            print("     %s" % u)
    if blob_missing:
        print("\n!! %d gone-source captures whose BLOB is missing -- already "
              "unrecoverable:" % len(blob_missing))
        for u in blob_missing[:20]:
            print("     %s" % u)
    if unparseable:
        print("\n!! %d unparseable manifests:" % len(unparseable))
        for u in unparseable[:10]:
            print("     %s  %s" % u)

    if args.verify:
        bad = miss = ok = 0
        for m in rows:
            d = dest_for(dest, m)
            if not os.path.exists(d):
                miss += 1
                print("  MISSING %s" % d)
                continue
            b = os.path.join(MIRROR, m["blob_relative_path"])
            if os.path.getsize(d) != os.path.getsize(b) or sha256_file(d) != sha256_file(b):
                bad += 1
                print("  MISMATCH %s" % d)
            else:
                ok += 1
        print("\nVERIFY: %d ok / %d mismatched / %d missing" % (ok, bad, miss))
        sys.exit(0 if (bad == 0 and miss == 0) else 1)

    if not args.apply:
        print("\nDRY RUN -- nothing written. Examples of the output layout:")
        for m in rows[:8]:
            print("   %s\n     <- %s" % (dest_for(dest, m), m["original_path"]))
        print("\nre-run with --apply to write.")
        return

    os.makedirs(dest, exist_ok=True)
    idx_path = os.path.join(dest, "index.tsv")
    written = skipped_existing = failed = 0
    t0 = time.time()
    with open(idx_path, "w") as idx:
        idx.write("\t".join(["archived_relpath", "original_path", "provider",
                             "bytes", "source_mtime_ms", "captured_at_ms",
                             "blob_blake3", "sha256_verified", "manifest_id"]) + "\n")
        for i, m in enumerate(rows, 1):
            d = dest_for(dest, m)
            b = os.path.join(MIRROR, m["blob_relative_path"])
            os.makedirs(os.path.dirname(d), exist_ok=True)
            try:
                if os.path.exists(d) and os.path.getsize(d) == (m.get("blob_size_bytes") or -1):
                    skipped_existing += 1
                else:
                    tmp = d + ".part"
                    shutil.copyfile(b, tmp)
                    if os.path.getsize(tmp) != (m.get("blob_size_bytes") or -1):
                        os.remove(tmp)
                        raise OSError("size mismatch after copy")
                    os.replace(tmp, d)
                    written += 1
                smt = m.get("source_mtime_ms")
                if smt:
                    os.utime(d, (smt / 1000.0, smt / 1000.0))
                idx.write("\t".join([
                    os.path.relpath(d, dest), m["original_path"],
                    str(m.get("provider")), str(m.get("blob_size_bytes")),
                    str(m.get("source_mtime_ms")), str(m.get("captured_at_ms")),
                    str(m.get("blob_blake3")), sha256_file(d),
                    str(m.get("manifest_id"))]) + "\n")
            except OSError as ex:
                failed += 1
                print("  FAILED %s: %s" % (d, ex))
            if i % 1000 == 0:
                print("  ...%d/%d (%.0fs)" % (i, len(rows), time.time() - t0), flush=True)

    with open(os.path.join(dest, "README.md"), "w") as r:
        r.write(
            "# Rescued agent session captures\n\n"
            "Plain copies of coding-agent session files that no longer exist at their\n"
            "original locations. The harnesses (Claude Code, openclaw) deleted the\n"
            "originals; these bytes survived only inside the cass raw-mirror, which was\n"
            "retired on %s. Nothing here needs cass to read.\n\n"
            "Layout: `<provider>/<path the file had, relative to $HOME>`\n\n"
            "Each file is a byte-exact copy of the original; mtime is restored from the\n"
            "capture's recorded `source_mtime_ms`. `index.tsv` carries, per file, the\n"
            "original absolute path, provider, size, source mtime, capture time, the\n"
            "mirror's blake3 and a sha256 computed at extraction time.\n\n"
            "Files: %d   Bytes: %d (%.2f GiB)\n"
            % (time.strftime("%Y-%m-%d"), written + skipped_existing, nbytes,
               nbytes / 1024 ** 3))

    print("\nDONE written=%d already_present=%d failed=%d in %.0fs"
          % (written, skipped_existing, failed, time.time() - t0))
    print("index: %s" % idx_path)
    print("\nNow run --verify before deleting the mirror.")


if __name__ == "__main__":
    main()
```
