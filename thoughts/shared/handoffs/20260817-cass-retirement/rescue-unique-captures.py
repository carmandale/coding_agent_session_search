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

    # optional: also rescue captures whose bytes exist nowhere live even though
    # their path still exists (not needed; default is path-gone only)

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
