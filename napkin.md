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
