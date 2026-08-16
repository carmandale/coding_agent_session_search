#!/usr/bin/env python3
"""Re-split the catch-up manifest by BYTES rather than by file count.

`catchup-run.sh` splits with `split -l 250`, which is right for the head of the
manifest and wrong for its tail. Measured 2026-08-16 on the 6,758-file manifest,
which is sorted ascending by size:

    batch  1:  0.04 GiB          batch 26:  1.38 GiB
    batch  8:  0.09 GiB          batch 27:  5.49 GiB   <- 40% of the whole corpus
    batch 16:  0.15 GiB          batch 28:  1.81 GiB

    first 8 batches: 0.57 GiB    last 8 batches: 11.37 GiB   (20x)

A fixed file count therefore hands cass a working set that grows by two orders of
magnitude across the run, and the disk headroom it needs is transient working
space rather than permanent archive growth — the previous run stopped with
31.5 GiB free and ~20 GiB came back the moment it exited.

Exit 14 is checked when cass STARTS, against total free space. So the thing that
matters is the peak each batch reaches before releasing, and that peak is what a
byte bound controls. Batches 1-3 completed against roughly 0.04-0.07 GiB of
source, so the default cap here stays inside 4x of what is already proven.

Use this when the fixed-count run stops on disk in its tail, or up front if the
disk is tight enough that you would rather trade wall-clock for headroom:

    python3 catchup-split-by-bytes.py ~/.cass-catchup/manifest.txt ~/.cass-catchup
    # then re-run catchup-run.sh, which re-globs batch-* at loop start

It writes the same `batch-*` names `split` would, so the runner needs no change.
It does NOT delete anything: existing batch files are left in place and this
refuses to run if any would be overwritten, because deleting a file needs Dale's
express permission (AGENTS.md RULE 1) and a half-overwritten batch set would
silently skip work.

ceiling: batches are emitted in manifest order and never reordered, so a single
file larger than the cap becomes its own batch rather than being split — cass
takes whole files and there is nothing smaller to hand it.
"""

from __future__ import annotations

import os
import sys

DEFAULT_CAP_BYTES = 200 * 1024 * 1024  # 0.20 GiB of source per batch


def main() -> int:
    if len(sys.argv) < 3:
        print(__doc__, file=sys.stderr)
        return 2
    manifest, workdir = sys.argv[1], sys.argv[2]
    cap = int(sys.argv[3]) if len(sys.argv) > 3 else DEFAULT_CAP_BYTES

    paths = [ln.strip() for ln in open(manifest) if ln.strip()]
    if not paths:
        print("manifest is empty — nothing to split", file=sys.stderr)
        return 1

    batches: list[list[str]] = []
    cur: list[str] = []
    cur_bytes = 0
    for p in paths:
        try:
            size = os.path.getsize(p)
        except OSError:
            size = 0
        if cur and cur_bytes + size > cap:
            batches.append(cur)
            cur, cur_bytes = [], 0
        cur.append(p)
        cur_bytes += size
    if cur:
        batches.append(cur)

    # Name them exactly as `split` does: batch-aa, batch-ab, ... so the runner's
    # existing glob picks them up unchanged.
    def name(i: int) -> str:
        return f"batch-{chr(97 + i // 26)}{chr(97 + i % 26)}"

    if len(batches) > 26 * 26:
        print(f"REFUSING: {len(batches)} batches exceeds the two-letter namespace", file=sys.stderr)
        return 1

    planned = [os.path.join(workdir, name(i)) for i in range(len(batches))]
    clashes = [p for p in planned if os.path.exists(p)]
    if clashes:
        print(
            f"REFUSING: {len(clashes)} batch files already exist "
            f"(first: {clashes[0]}). Move them aside yourself — this script "
            f"does not delete files.",
            file=sys.stderr,
        )
        return 1

    total = 0
    for path, chunk in zip(planned, batches):
        with open(path, "w") as fh:
            fh.write("\n".join(chunk) + "\n")
        total += len(chunk)

    if total != len(paths):
        print(f"REFUSING TO CLAIM SUCCESS: wrote {total} of {len(paths)} paths", file=sys.stderr)
        return 1

    largest = max(
        sum(os.path.getsize(p) for p in b if os.path.exists(p)) for b in batches
    )
    print(f"  {len(paths)} files -> {len(batches)} batches, cap {cap/2**30:.2f} GiB")
    print(f"  largest batch: {largest/2**30:.2f} GiB of source")
    print(f"  (the fixed-count split put 5.49 GiB in one batch)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
