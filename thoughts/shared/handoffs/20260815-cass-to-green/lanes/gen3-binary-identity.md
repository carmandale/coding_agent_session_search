# Lane gen3-binary-identity — what binary is actually installed

Read-only lane. Owner: gen3-binary-identity. Append-only.
Started 2026-08-15. Repo: `.claude/worktrees/cass-to-green-c6bfb589` (branch
`worktree-cass-to-green-c6bfb589`, HEAD `9d4814d2` at lane start).

Subject: settle the contradiction between bead `coding_agent_session_search-1a7mk`
(hangs "after the coverage-floor fix", implying the installed binary CONTAINS it)
and `backfill.sh:12` ("Runs on the installed PRE-FIX binary").

Nothing was written outside this file. No git mutation, no build, no index, no
`cass health/status/triage/stats` run against the live archive (the backfill is
in flight; those surfaces are the ones 1a7mk says do not return, so running one
would have burned minutes for evidence the string markers already give).

---

## 1. The live binary

```
$ shasum -a 256 /Users/dalecarman/.local/bin/cass
5b3344fd94f93cd4ba0357a4c2d5b9de5733ead94ab404ff0963fdec29d01644
$ /usr/bin/stat -f %z  -> 51900992
$ date -r ... -u       -> 2026-08-14T21:56:50Z
```

Regular file, not a symlink (`ls -la ~/.local/bin`). `which -a cass` returns
`/Users/dalecarman/.local/bin/cass` (twice — a duplicated PATH entry, same path).

## 2. Every cass binary found on disk

Searched `~/.local/bin`, `/usr/local/bin`, `~/bin`, the main checkout's
`target/release`, the worktree's `target/release`, and the backfill scratchpad.

| path | sha256 (first 16) | size | mtime (UTC) |
|---|---|---|---|
| `~/.local/bin/cass` (LIVE) | `5b3344fd94f93cd4` | 51,900,992 | 2026-08-14T21:56:50Z |
| `~/.local/bin/cass.nvq59-status-gate-20260814-165549` | `5b3344fd94f93cd4` | 51,900,992 | 2026-08-14T21:56:50Z |
| `~/dev/coding_agent_session_search/target/release/cass` | `5b3344fd94f93cd4` | 51,900,992 | 2026-08-14T21:54:41Z |
| `~/.local/bin/cass.coverage-floor-fix-20260810` | `d0b860eb6a8ef366` | 51,900,976 | 2026-08-11T01:37:41Z |
| `~/.local/bin/cass.pre-coverage-floor-20260601` | `3d04422759268c17` | 51,834,784 | 2026-06-01T11:21:13Z |

Three of the five are byte-identical (`5b3344fd…`): the live binary, the dated
nvq59 specimen, and the leftover build artifact in the main checkout's
`target/release`. `/usr/local/bin/cass*` and `~/bin/cass*` do not exist
(`no matches found`). The worktree has no `target/` at all
(`os error 2`). Nothing cass-binary-shaped in the backfill scratchpad.

## 3. Does the live binary contain the coverage-floor code?

Markers derived from the feature commit, not guessed:

```
$ git show e3ed01f0 | rg '^\+' | rg -o '"[^"]{16,}"' | sort -u
```

Kept only literals that live in production code (verified by `rg -n` against
`src/`, each outside a `#[cfg(test)]` module): `src/indexer/mod.rs:10734`,
`src/indexer/mod.rs:11576`, `src/lib.rs:24074`, `src/storage/sqlite.rs:60`.

Counts are byte-occurrence counts over the whole file (python `bytes.count`),
not `rg` line counts — a stripped release binary has few newlines, so line
counts understate.

| marker | LIVE | nvq59-0814 | target/release | covfix-0810 | pre-cov-0601 |
|---|---|---|---|---|---|
| CONTROL `last_scan_ts` | 15 | 15 | 15 | 15 | 15 |
| CONTROL `fts_messages_content` | 5 | 5 | 5 | 5 | 5 |
| NEGATIVE CONTROL `zzz-nonexistent-marker-9931` | 0 | 0 | 0 | 0 | 0 |
| `recording connector scan coverage floor` | **1** | 1 | 1 | 1 | **0** |
| `widening connector scan window back to its recorded coverage floor` | **1** | 1 | 1 | 1 | **0** |
| `Scan Coverage: INCOMPLETE` | **1** | 1 | 1 | 1 | **0** |
| `or_scan_floors` (meta-key tail, see §4) | **1** | 1 | 1 | 1 | **0** |
| `connector_scan_floors` (meta key, whole) | 0 | 0 | 0 | 0 | 0 |
| `Scan Coverage: UNKNOWN` (hang fix `8dcd245b`) | **0** | 0 | 0 | 0 | 0 |
| `connector coverage read exceeded its bound` (hang fix `8dcd245b`) | **0** | 0 | 0 | 0 | 0 |

**Positive control fires** in all five binaries (`last_scan_ts` 15, and a second
independent control `fts_messages_content` 5). **Negative control is 0** in all
five. So the method can produce both answers, and the zeros in the `pre-cov-0601`
column are real absences rather than a dead instrument.

Verdict from §3: the live binary **contains the coverage-floor feature**
(`e3ed01f0`) and **does not contain the fix for the hang it caused**
(`8dcd245b`, committed today 2026-08-15T06:29:06-05:00, after the binary was
built).

## 4. Why the `connector_scan_floors` literal reads 0 everywhere — explain the empty

`connector_scan_floors` (`src/storage/sqlite.rs:60`,
`CONNECTOR_SCAN_FLOORS_META_KEY`) is **absent as contiguous bytes from all five
release binaries, including the ones that demonstrably carry the feature.**
It is not a discriminator. The bytes exist, split:

```
02882e00  6f 72 5f 73 63 61 6e 5f 66 6c 6f 6f 72 73 00 00  or_scan_floors..
02882e10  00 00 00 00 00 00 00 80 15 63 6f 6e 6e 65 63 74  .........connect
```

`0x15` = 21 = `len("connector_scan_floors")`, with the first 7 bytes (`connect`)
stored inline and the 14-byte tail (`or_scan_floors`) held separately — a
compact/inline string representation, not one contiguous rodata literal. I did
not identify which type produces this layout and am not claiming one; what is
measured is that the whole literal is not searchable and the 14-byte tail is,
1 vs 0, in exactly the expected columns.

This independently reproduces a correction already on the record: bead
`coding_agent_session_search-c7yaw`, comment 2026-08-11T01:39:31Z — *"The
verification method in this bead is INVALID and should not be re-run. `strings
<binary> | grep -cF 'connector_scan_floors'` returns 0 on a release binary that
HAS the fix."* The `54` in that bead's body is a source-occurrence count, not a
binary measurement. Anyone re-running c7yaw's stated method today would get 0
against a binary that carries the fix and could read it as "not deployed".

## 5. `cass --version` — reported, and unreliable per the vergen gap

```
~/.local/bin/cass                            rc=0  "cass 0.6.9 / git commit: 447d97fe60962d1ed1f34841e508f61a6b4302c4"
~/.local/bin/cass.coverage-floor-fix-20260810 rc=0  "cass 0.6.9"          (no git commit line)
~/.local/bin/cass.pre-coverage-floor-20260601 rc=0  "cass 0.6.9"          (no git commit line)
```

The version string alone cannot separate them: all three say `0.6.9`, because
`Cargo.toml` was never bumped. The git-sha line only exists on the newest build —
bead `coding_agent_session_search-il0e9` (closed) records that `vergen` was
configured without the `git` feature so every earlier build reported `unknown`;
commit `ff3d7125` (2026-08-10T20:16:52-05:00) is the beads commit that filed that
gap. So the absent line on the two older binaries is the vergen gap, and the
present line on the live one is a later build with the gap closed.

Treated as unreliable per instruction, but it is corroborated rather than
contradicted here:

```
$ git merge-base --is-ancestor e3ed01f0 447d97fe   -> rc=0
$ git merge-base --is-ancestor 419437e6 447d97fe   -> rc=0   (the coverage-floor merge)
447d97fe  2026-08-14T16:47:15-05:00 = 21:47:15Z   (binary mtime 21:56:50Z, 9m later)
```

`447d97fe` has no new string literals in `src/lib.rs`
(`git show 447d97fe -- src/lib.rs | rg '^\+' | rg -o '"[^"]{12,}"'` → empty), so
there is no string discriminator for that commit specifically. `archive_coverage_state`
appears 2x in **all five** binaries including the June 1 one, so it is NOT a
discriminator and proves nothing — recording that here so nobody reuses it.
The binary's identity is settled by sha256 equality with the dated
`cass.nvq59-status-gate-20260814-165549` specimen, which does not depend on vergen.

## 6. What the record says, and when each claim was true

- `coding_agent_session_search-c7yaw` body: "installed cass is the 2026-06-01
  pre-fix binary" — true when filed 2026-08-10.
- `c7yaw` comment 2026-08-11T01:39:31Z: deploy attempted and **ROLLED BACK**;
  installed binary reverted. Consistent with `cass.coverage-floor-fix-20260810`
  (`d0b860eb…`) existing as a preserved sibling and not being live.
- `coding_agent_session_search-status-json-hang-nvq59` comment 2026-08-14T21:49:02Z:
  "the fix is in main but NOT deployed. ~/.local/bin/cass is still the pre-fix
  0.6.9 (sha256 3d044227..)" — true at 21:49Z.
- **7 minutes later the deploy happened.** Live binary mtime 21:56:50Z, sha
  `5b3344fd…`. Every statement above became stale at that moment.
- `backfill.sh:12-13` (written 2026-08-15T05:53 local): "Runs on the installed
  PRE-FIX binary. A HEAD build would reintroduce the coverage-floor regression
  (bead 1a7mk)." That is **false**, and it is contradicted by the script's own
  output: `backfill/run.log:4` records `binary  : 5b3344fd94f93cd4`, the
  post-fix build. `backfill.sh:18` sets `BIN=/Users/dalecarman/.local/bin/cass`.
- `coding_agent_session_search-1a7mk` title ("hang … after the coverage-floor
  fix") describes the currently installed binary correctly. Its body line "the
  build was rolled back rather than left installed" was true on 2026-08-11 and
  is stale now.
- Commit `b0780df0` (2026-08-15T06:29:26-05:00, this handoff's own generation-2
  evidence commit) already reached the same conclusion: "447d97fe IS deployed …
  the -1a7mk coverage regression has been live on this machine, not rolled back."
  This lane is an independent confirmation with a different instrument
  (feature-derived literals + negative control), not a new finding.

## 7. Verdict

**Not ambiguous.** The live binary contains the coverage-floor code: it is
byte-identical (sha256 `5b3344fd…`) to the dated post-fix specimen
`cass.nvq59-status-gate-20260814-165549`, three production literals added by
`e3ed01f0` are present in it and absent from the preserved 2026-06-01 pre-fix
binary under a control that fires 15/15 in both and a negative control that fires
0/5, and the sha `447d97fe` it reports descends from `e3ed01f0`. Bead 1a7mk's
framing is the one that matches reality; `backfill.sh`'s header comment is wrong,
and its own `run.log` line 4 records the post-fix sha.

Two consequences for the imminent deploy, stated because they follow directly
from the measurement:

1. The 1a7mk coverage regression is **live right now** — the hang fix
   `8dcd245b` is absent from the installed binary (0 occurrences of both its
   production literals, same controls). "Don't deploy HEAD, it would reintroduce
   the regression" is not a reason to hold: the regression is already installed
   and a HEAD build is what removes it.
2. The running backfill is executing under the post-fix binary, so its scans can
   write and clear per-connector coverage floors. Anyone reading
   `connector_scan_floors` out of the archive's `meta` table after this run must
   not compare it against c7yaw's 2026-08-11 note that the key was absent.

## Commands run (all read-only)

`shasum -a 256`, `/usr/bin/stat -f %z`, `date -r … -u`, `ls -la`, `which -a`,
`rg -a -F -c`, `strings -a`, python `bytes.count` / `re.finditer` over the
binaries, `head` on `backfill.sh` and `backfill/run.log`, `git log/show/
merge-base/cat-file`, python read of `.beads/issues.jsonl`, and
`cass --version` on three binaries via python `subprocess` with `timeout=25`
(all returned rc=0 well inside the bound).

`br show` was NOT usable: the worktree has no `.beads/beads.db`, so
`br show coding_agent_session_search-1a7mk` exits with
`Sync conflict: Refusing storage open … the authorized database is missing`.
Bead text was read from `.beads/issues.jsonl` in the main checkout instead.
