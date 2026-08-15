# Corpus Ground Truth — Read-Only Lane

Role: log-only, read-only. DB opened exclusively via `sqlite3 -readonly` (or
`file:...?mode=ro`, both used). No writes to the live DB, no `cass index`, no
git mutations. Only this file was written.

A codex backfill was actively running against the DB the entire time this
lane worked. Every number below is timestamped at the point it was measured;
treat any two numbers from different timestamps as **not simultaneous**.

## Headline answer

`connector_coverage.complete` cannot be the acceptance signal — confirmed
independently. It is not even a stored/persisted field (the DB has no
coverage table; `sources` holds exactly one row, `id=local`), so it must be
computed at runtime by the binary from data that itself lags the real
filesystem. The real gap, measured by diffing the disk directly against
`conversations.source_path`, as of **2026-08-15T11:12:49Z**:

| source tree | on disk | indexed | never-indexed gap | indexed-but-source-now-gone |
|---|---:|---:|---:|---:|
| `~/.codex/sessions` (both layouts) | 10,313 | 4,011 (11:12:40Z) | ~6,302 | 0 |
| `~/.claude/projects` (*.jsonl only) | 8,189 | 4,050 (static) | 8,016 | 3,877 |

Claude Code coverage is far worse than Codex's: only ~173 of 8,189
currently-existing transcripts (2.1%) are both indexed *and* still have
their source file present. Codex is being actively backfilled right now and
0% of its indexed rows point to a missing file. Claude Code has **no active
backfill** in this session's task list, and the previous session's hole
manifest never covered it at all (0 of 4,895 manifest lines are under
`.claude/projects` — verified below).

The single most valuable disagreement with the previous session's manifest:
**2,338 pre-existing, never-indexed Codex `.jsonl` files were not in the
4,895-line manifest at all**, and they are not new — their mtimes predate
the manifest's own creation time. The manifest's flat-layout (`.json`)
enumeration was 100% complete (1,645 of 1,645); its nested-layout (`.jsonl`)
enumeration undercounted the true nested backlog by at least 41.8%
(2,338 missed out of a true ≥5,588). Detail in "Manifest cross-check" below.

---

## 1. Connector registry (source of "what other trees exist")

`franken-agent-detection` v0.1.8 is pulled as a git dependency
(`Cargo.toml:94`, rev `b62d859`), vendored at
`~/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/b62d859`.
cass's own `src/connectors/mod.rs` is a re-export shim; the real connector
logic lives in that checkout.

`get_connector_factories()` in the checkout
(`src/connectors/mod.rs:200-232`, matching cass's enabled Cargo features
`connectors, cursor, chatgpt, opencode, crush, hermes`) registers **20**
connector slugs: `codex, cline, gemini, claude, clawdbot, vibe, amp, aider,
pi_agent, factory, kimi, openclaw, copilot, copilot_cli, qwen, opencode,
chatgpt, cursor, crush, hermes`.

Command:
```
sqlite3 -readonly ".../agent_search.db" "SELECT id, slug FROM agents ORDER BY id;"
```
Output (2026-08-15T11:04:xxZ — small, static table, safe to treat as a
config-time read):
```
1|claude_code
2|amp
3|codex
4|pi_agent
5|openclaw/feature-dev-developer
6|factory
7|cursor
8|openclaw/feature-dev-planner
9|opencode
```
Only 9 of the 20 registered connector slugs have ever produced an indexed
row on this machine. `openclaw` further splits into two role-scoped agent
identities (`feature-dev-developer`, `feature-dev-planner`) rather than one
`openclaw` slug — this is a real DB fact, not a guess.

Existence check for the remaining connector home directories (cheap,
non-recursive `ls -d`, 2026-08-15T11:0xZ):
- EXIST: `~/.amp`, `~/.copilot`, `~/.factory`, `~/.gemini`, `~/.openclaw`
  (`~/.clawdbot` is a **symlink to `~/.openclaw`** — confirmed via `ls -d`,
  not assumed)
- ABSENT: `~/.vibe`, `~/.aider`, `~/.pi_agent`, `~/.kimi`, `~/.qwen`

**Scope limitation, stated plainly:** I did not do a full on-disk file count
for all 20 connectors (cline/opencode/copilot_cli use `~/.config` or
`~/.local`, which are broad and risky to walk under the "never search
unbounded from $HOME" constraint; gemini/amp/copilot/factory exist but were
out of the task's explicit "at minimum" list). What I have for those:
existence of the home dir, and the DB's own indexed-row count per slug
(table above/below). A full on-disk audit of all 20 connectors is real
follow-up work, not something I inferred a "clean" answer for.

---

## 2. Codex on-disk corpus, both layouts, counted separately

Discovery logic verified in the vendored checkout,
`src/connectors/codex.rs:100-121` (`rollout_files()`, `WalkDir` over
`~/.codex/sessions`, matches `rollout-*.jsonl` or `rollout-*.json` at any
depth) — confirms flat and nested layouts are walked by the same code path
(so "undiscovered" is a downstream bug, not a missing walk). cass's own
`src/connectors/mod.rs:158-185` (`collect_codex_rollout_files`) does the
same walk independently for its explicit-file-root preflight.

Commands (2026-08-15T11:04:15Z — 11:04:22Z):
```
find "$HOME/.codex/sessions" -maxdepth 1 -type f -name 'rollout-*.json'   → 1645
find "$HOME/.codex/sessions" -maxdepth 1 -type f -name 'rollout-*.jsonl'  → 2
find "$HOME/.codex/sessions" -mindepth 2 -type f \( -name 'rollout-*.jsonl' -o -name 'rollout-*.json' \)  → 8666
find "$HOME/.codex/sessions" -type f \( -name 'rollout-*.jsonl' -o -name 'rollout-*.json' \)              → 10313
```
Depth histogram confirms there is no third layout:
```
find "$HOME/.codex/sessions" -type f \( -name 'rollout-*.jsonl' -o -name 'rollout-*.json' \) -print \
  | sed "s#^$HOME/.codex/sessions/##" | awk -F/ '{print NF}' | sort | uniq -c
   1647 1     ← flat: rollout-*.json(l) directly under sessions/
   8666 4     ← nested: sessions/YYYY/MM/DD/rollout-*.jsonl
```
So: **flat layout = 1,647** (1,645 `.json` + 2 `.jsonl`), **nested layout =
8,666** (all `.jsonl`), **total on disk = 10,313**, re-confirmed stable at
2026-08-15T11:12:49Z (unchanged across the whole lane — no new Codex
sessions were created while I measured).

This exactly matches bead
`coding_agent_session_search-codex-flat-layout-undiscovered-kfaid`'s claim
of "1,647 flat-layout rollouts" — confirmed independently, not copied.

## 3. Claude Code on-disk corpus

Discovery logic verified in the checkout,
`src/connectors/claude_code.rs:106-119` (`session_files()`, `WalkDir` over
`~/.claude/projects`, accepts extensions `jsonl`, `json`, **and** `claude`).

Command (2026-08-15T11:04:40Z):
```
find "$HOME/.claude/projects" -type f -name '*.jsonl' → 8182
find "$HOME/.claude/projects" -type f -name '*.json'  → 6474
find "$HOME/.claude/projects" -type f -name '*.claude' → 0
```
**Important correction to a naive reading of the connector's own extension
filter:** the connector's `session_files()` really does walk `.json` files
too, so a naive "on-disk corpus" count would be 8182+6474=14,656. That is
wrong. Sampled the `.json` files by name
(`sessions-index.json` ×16, `wf_*.json` workflow files, `agent-*.meta.json`
subagent metadata) — none of these are conversation transcripts, they are
per-project index/metadata files that live in the same tree. Verified this
is not a guess by checking what the DB actually holds:
```
sqlite3 -readonly ".../agent_search.db" \
  "SELECT CASE WHEN source_path LIKE '%.jsonl' THEN 'jsonl' WHEN source_path LIKE '%.json' THEN 'json' ELSE 'other' END, COUNT(*) FROM conversations WHERE agent_id=1 GROUP BY 1;"
→ jsonl|4050
```
All 4,050 indexed `claude_code` rows have a `.jsonl` source_path; **zero**
come from a `.json` file. So the honest on-disk denominator for Claude Code
is the `.jsonl` count, not `.jsonl + .json`. Re-measured paired with the
final gap computation at 2026-08-15T11:11:48Z: **8,189** `.jsonl` files
(grew by 7 from the 11:04:40Z count — plausible: this very session and any
other live Claude Code sessions keep writing new transcripts while this
lane ran).

## 4. Real gap, computed by direct set-diff (not by any coverage field)

Method: read the on-disk path list, read `conversations.source_path` for
the agent via `sqlite3 -readonly ... | python3` (single subprocess call,
no writes), diff as sets. Every number below has its own timestamp pair
printed by the command itself.

### Codex (agent_id=3), 2026-08-15T11:10:19Z–11:10:20Z
```
find "$HOME/.codex/sessions" -type f \( -name 'rollout-*.jsonl' -o -name 'rollout-*.json' \) \
  | python3 -c "...disk vs sqlite3 -readonly ... SELECT source_path FROM conversations WHERE agent_id=3..."
on_disk_total 10313
indexed_rows_total 3927
overlap 3927                       ← every indexed codex row's file still exists
gap_on_disk_not_indexed 6386       ← real, current backfill-needed count
indexed_but_not_on_disk_here 0
```
Re-run later (backfill still running) at 2026-08-15T11:12:40Z–49Z:
indexed rows had grown to 4,011 against a still-stable 10,313 on disk
→ remaining gap ≈ **6,302**. (I did not re-run the full set-diff a second
time to avoid hammering the live DB during an active backfill; the
subtraction is valid here because 0 indexed rows point to missing files for
codex, confirmed twice.)

### Claude Code (agent_id=1), 2026-08-15T11:11:48Z–11:11:50Z
```
find "$HOME/.claude/projects" -type f -name '*.jsonl' \
  | python3 -c "...disk vs sqlite3 -readonly ... SELECT source_path FROM conversations WHERE agent_id=1..."
on_disk_jsonl 8189
indexed_rows 4050
overlap_still_present 173          ← only 173 indexed rows' files still exist
gap_on_disk_never_indexed 8016     ← never indexed at all
indexed_rows_missing_source_file 3877
```
Claude Code has **no active backfill** running (only Codex is, per this
lane's own task brief). 8,016 of 8,189 currently-existing transcripts
(97.9%) have never been seen by cass at all.

## 5. Indexed-but-source-file-gone, recomputed for every agent (not copied)

Command pattern per agent (`WHERE agent_id=N`, single sqlite3→python
pipeline, no writes), each timestamped individually between
2026-08-15T11:08Z and 11:09Z:

| agent_id | slug | indexed | source file missing | % missing |
|---:|---|---:|---:|---:|
| 3 | codex | 3,831→4,011 (grew during measurement) | **0** | 0% |
| 1 | claude_code | 4,050 | **3,877** | 95.7% |
| 2 | amp | 33 | 0 | 0% |
| 4 | pi_agent | 1,876 | 0 | 0% |
| 5 | openclaw/feature-dev-developer | 1,482 | **1,482** | 100% |
| 6 | factory | 66 | 0 | 0% |
| 7 | cursor | 1 | 1 | 100% |
| 8 | openclaw/feature-dev-planner | 1,392 | **1,392** | 100% |
| 9 | opencode | 764 | 0 | 0% |

Sum of the "missing" column = 3877+1482+1+1392 = **6,752**, which exactly
matches the aggregate query run separately:
```
sqlite3 -readonly ".../agent_search.db" "SELECT source_path FROM conversations;" \
  | python3 -c "...os.path.exists per line..."
→ 13479 6752        (2026-08-15T11:07:xx–11:08Z; total conversations, missing count)
```
This is an exact match to the handoff's cited "6,752 of 12,722" for the
*missing* half — the total grew (13,479 vs 12,722) purely from the
in-flight codex backfill, which contributes 0 to the missing count, so the
missing figure itself is stable. **This independently confirms 6,752 is
correct and current**, not stale.

The `claude_code` figure of 3,877 also exactly matches the number already
named in this repo's own git history (commit `770c1d8b`, "cass is the only
copy of 3,877 Claude Code conversations") — confirmed here independently
via direct file-existence checks, not copied from the commit message.

**New finding, not previously recorded anywhere I found:** `openclaw`'s two
role-scoped identities are **100% missing source file** — 2,874
conversations combined. Checked whether this is "expected ephemeral scratch
gets cleaned up" or real data loss:
```
sqlite3 -readonly ".../agent_search.db" "SELECT source_path FROM conversations WHERE agent_id=5 LIMIT 3;"
→ /Users/dalecarman/.openclaw/agents/feature-dev-developer/sessions/<uuid>.jsonl  (×3)

ls -d "$HOME/.openclaw/agents/feature-dev-developer/sessions"
→ No such file or directory

find "$HOME/.openclaw/agents" -maxdepth 3 -type d
→ /Users/dalecarman/.openclaw/agents
→ /Users/dalecarman/.openclaw/agents/main
→ /Users/dalecarman/.openclaw/agents/main/agent
```
The entire `feature-dev-developer` and (by the same evidence pattern)
`feature-dev-planner` directory trees are gone; only an `agents/main/agent`
tree exists now. This reads as an openclaw reorganization (old role-scoped
layout retired in favor of `main`), not a partial/random deletion — but I
have no timeline evidence for *when* that happened, so I'm reporting the
fact (2,874 conversations whose only surviving copy is cass) and flagging
the interpretation as UNVERIFIED rather than asserting a cause.

## 6. Manifest cross-check — the most valuable disagreement

Manifest: `/private/tmp/claude-501/-Users-dalecarman--agent-config/a91c2501-1830-4d3d-9430-3c9afe08a63c/scratchpad/cass-hole-manifest.txt`
- `wc -l` → 4,895 (matches the handoff's stated count)
- 100% Codex paths: `grep -c '.claude/projects'` → 0, `grep -c '.codex/sessions'` → 4895
- Extension split: `.json` → 1,645, `.jsonl` → 3,250 (sums to 4,895)
- **True UTC mtime** of the manifest file (per the instrument-labels rule —
  `stat -f '%Sm'` prints *local* time; used `date -r <file> -u
  +%Y-%m-%dT%H:%M:%SZ` instead): **2026-08-14T21:29:27Z**
  (the raw `stat -f '%Sm'` value read `16:29:27`, a 5-hour local/UTC gap —
  consistent with this Mac's documented drift; using the wrong one would
  have misdated every comparison below by 5 hours)

Three-way diff — manifest ∩ current-gap, manifest ∩ now-indexed, current-gap
∖ manifest — computed 2026-08-15T11:10:47Z:
```
disk 10313
manifest 4895
indexed 3951
gap 6362
manifest_still_in_gap 4002              ← manifest entries still un-indexed
manifest_already_indexed_since 893      ← manifest entries the backfill already cleared
manifest_entries_not_on_disk_now 0      ← nothing in the manifest was deleted
gap_entries_not_in_manifest 2360        ← current gap the manifest never listed
```
`manifest_entries_not_on_disk_now 0` matters: it rules out "the manifest is
stale because files were deleted" — every manifest entry still exists on
disk. The disagreement runs the other way: the manifest **undercounted**.

Split `gap_entries_not_in_manifest` (2,360) by mtime against the manifest's
own creation time (2026-08-15T11:11:14Z–11:11:26Z):
```
gap_not_in_manifest_total 2360
older_than_manifest_creation (should have been listed) 2338
newer_than_manifest_creation (genuinely new since) 22
older_total 2338
flat_layout_among_older 0
nested_layout_among_older 2338
ext_json_among_older 0
ext_jsonl_among_older 2338
```
**2,338 of the 2,360 files the manifest omitted already existed, unindexed,
before the manifest was built** (mtimes as old as 2026-07-23, sample
included in the transcript) — this is not "the corpus grew since," it is a
methodology gap in whatever produced the manifest. All 2,338 are
**nested-layout `.jsonl`** files (none flat, none `.json`). So:
- The manifest's flat-layout (`.json`) enumeration was **complete**:
  1,645 listed = 1,645 on disk, exactly.
- The manifest's nested-layout (`.jsonl`) enumeration **undercounted the
  true backlog by at least 41.8%** (2,338 missed out of a true nested
  backlog of ≥ 3,250+2,338 = 5,588 at manifest-creation time).

I do not know what method produced the manifest (I did not find its
generating command in this lane, and reconstructing it was out of scope
for a read-only grounding pass) — I can only report the measured
disagreement, not its cause. Whatever generates the next backfill manifest
should be re-derived from a direct disk-walk-vs-DB diff like the one in
this log, not reused as-is, since it visibly missed real, pre-existing,
un-indexed files.

## 7. Caveats / limitations, stated plainly

- Every count in this log moved while I worked (an active Codex backfill).
  I paired on-disk and indexed counts as tightly in time as I could
  (typically within 1-2 seconds) and named the exact timestamp for each;
  do not combine numbers across sections with different timestamps as if
  simultaneous.
- I did not attempt a full on-disk file audit for the 11 other connectors
  that exist as directories but were not in this task's explicit "at
  minimum" list (gemini, amp, copilot, factory, openclaw/main, cline,
  opencode, copilot_cli, chatgpt, cursor, crush, hermes) beyond the
  existence check and the DB's own indexed-row counts already shown in
  section 5. That is real follow-up scope, not a checked-and-clean result.
- The `openclaw` 100%-missing-source finding (section 5) is reported as a
  fact with an unverified interpretation (reorg vs. deletion) — I did not
  find timeline evidence to settle which.
- I did not run the `cass` binary at all (no `cass status`, no `cass
  sources`) to avoid any risk of contending with the live backfill or
  writing to the DB; every number here comes from direct `sqlite3
  -readonly` reads and direct filesystem walks, which is a stronger
  ground-truth source than the binary's own self-reported coverage field
  would have been anyway.
- One `sqlite3 ... | wc -l` query hit `database is locked (5)` once,
  transiently, consistent with the concurrent backfill writer; the retry a
  few seconds later succeeded. Noted for whoever reads DB-lock errors in
  the backfill's own logs — a locked read is expected background noise
  here, not evidence of DB corruption.
- Tooling note for whoever runs `sqlite3 "file:...?mode=ro" ... | <cmd>` in
  this sandboxed session type: piping a command whose argument list
  contains the literal substring `file:` was intermittently refused by
  this harness's worktree-isolation guard ("too complex to verify..."),
  even though the identical command run *without* a pipe succeeded
  repeatedly. Switching to `sqlite3 -readonly <plain-path> ...` (same
  read-only guarantee, no `file:` URI) made every piped query reliable.
  Recording this so a future lane doesn't waste time re-diagnosing it.
