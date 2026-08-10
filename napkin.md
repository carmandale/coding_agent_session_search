# napkin — coding_agent_session_search

Inbox, not archive. Lowest authority in the repo: a line here is a claim, never a
rule. Cap one screen (~75 lines). At closeout, promote what earned it to the file
that owns it and delete what expired.

## Today

- Bead `codex-coverage-gap-2bh4a`, part 2 landed. `cass index --watch-once` is the
  path-scoped re-scan; it already existed, so it was verified rather than built.
  Measured on copies: 117 hole files / 369.8 MB took 26.8 min and grew the data dir
  874 MB. Extrapolates to ~12-15 h and ~24-30 GB for the 3,186-file tail. Not run
  against the live archive — that is Dale's call.
- The product's stall detector fires during a healthy `--watch-once` run (4 times in
  26.8 min) and the run still exits 0. Anyone running the recovery needs warning, or
  they will kill a working job.

## Corrections

| What went wrong | Correction | Marker |
|---|---|---|
| A shared `CARGO_TARGET_DIR` between two checkouts of this crate silently ran the WRONG binary. Both trees resolve to the same artifact name, so the later build clobbers the earlier, and cargo's mtime freshness check then prints `Finished in 0.41s` and re-runs the other tree's test binary. My "full lib suite" result was the pre-change clone's. | Give each checkout its own target dir, or `touch` the sources and confirm `Compiling coding-agent-search (<the path you mean>)` before believing any cross-tree comparison. The tell was a panic at a line number that does not exist in the tree I thought I was testing. | Pending promotion: `.claude/rules/` or AGENTS.md if it recurs |
| Reported `WATERMARK before=None after=None (must be unchanged)` from a probe reading `last_scan_ts` out of `cass stats --json` — a key that does not exist there. Both reads were None for the same reason, so the line proved nothing. | Read `meta` from the DB directly. Verified properly afterwards: pass 3 logs zero `updated last_scan_ts` lines and the DB still holds pass 2's value. | Expires: on next edit of this file — already fixed in the bead comment |
| `cargo clippy \| tail` and `close-check \| tail` both report tail's exit code, not the tool's. Clippy "passed" with exit 0 while emitting 3 errors. | Capture the status directly (`out=$(cmd); rc=$?`) or read the verdict line, never the pipeline's `$?`. Already in the global rules; it still caught me twice today. | Expires: 2026-09-10 |

## Hypotheses

<!-- speculative -> emerging -> promotion-due, confidence from cited Evidence: lines -->

- **speculative** — The live archive's mtime is moving without an identified writer.
  It was 2026-08-10 12:51:52 before this session ran anything (the bead and the
  handoff both assert byte-identical at a 2026-08-04 mtime, so that premise was
  already false), and moved to 13:13:11 mid-session. Size identical at every
  observation (7,927,099,392). Nothing held it open at any check, no peer agent
  session is in this repo, and this session never opened it.
  Evidence: `stat -f` at 12:52 and at 19:40 local, 2026-08-10; `lsof` empty both
  times; `ListAgents` showed 18 peers, none here.

- **emerging** — The e2e integration suite is not hermetic on the read side: it
  spawns 8 concurrent `cass index --full` that scan the operator's real `~/.codex`
  and `~/.claude` trees, isolating only the output via `--data-dir`. On this machine
  that is ~9,800 codex files each and it had not finished after 90 minutes.
  Evidence: `ps` showed 8 children of `e2e_cli_flows` at ~60% CPU each with
  `--data-dir /var/folders/...`, elapsed 01:27, 2026-08-10.
