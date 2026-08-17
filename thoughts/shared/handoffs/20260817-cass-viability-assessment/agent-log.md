# Coordinator log — cass viability assessment (2026-08-17)

Session: 656f2411-6418-4df9-9965-55219cd71762 (Claude Code, coordinator).
Goal: honest assessment for Dale — is cass a valid solution, or does it need replacing?
Mode: research (no fixes). Workflow run: wf_fec7a1dd-4ec (ultracode dynamic workflow).

## Lane declaration

All lanes: Claude Code workflow subagents, internal-only visibility surfaced via
/workflows plus these durable logs; read-only toward production data; every cass
invocation gtimeout-bounded ≤120s; forbidden to touch PID 75534 (another
session's probe), forbidden to run `cass index` or any repair verb; stop
condition = single-report completion; write permission ONLY to their assigned
log below. Coordinator owns synthesis and this log.

| lane | purpose | log path |
|---|---|---|
| defect-ledger | open/closed defects, generations of rescue effort, recurrence | lanes/defect-ledger.md |
| architecture-audit | LOC split core vs periphery, storage design, frankensqlite pin | lanes/architecture-audit.md |
| resource-forensics | attribute the 69G worktrees + 77G data dir + memory blowup | lanes/resource-forensics.md |
| live-probe | bounded executed probes of shipping binary (search/stats/freshness) | lanes/live-probe.md |
| alternatives | job-to-be-done, rg baseline, right-sized design, prior art | lanes/alternatives.md |

## Coordinator's own measurements (pre-lane, 2026-08-17)

- Source corpus: ~/.claude/projects 8.5G + ~/.codex/sessions 29G ≈ 37.5G.
- Prod data dir 77G (raw-mirror 46G, agent_search.db 22G, tantivy index 9.5G).
- Repo .claude/worktrees 69G. Disk: 29Gi available.
- `cass stats --json` (PATH binary): >3.5min, 5.2GB RSS, no return; killed by me.
- Sibling probe `/tmp/cass-fix-target/release/cass … search frankensqlite --limit 5`
  running 4h48m+ with no result (PID 75534, left alone).
- src 161 files / 393,912 LOC; tests 225 files; 4,261 commits; CrashReporter
  plist present for coding_agent_search.

## Synthesis (verdict delivered to Dale in chat, 2026-08-17)

All five lanes returned with executed evidence; logs in lanes/. One lane claim
was dropped after failed verification: alternatives' assertion that the cass
SKILL.md wraps index calls in gtimeout (no such reference in
~/.claude/skills/cass/SKILL.md; what does hold is that gtimeout/timeout are not
installed on this machine at all).

**Verdict: cass as implemented is not a valid solution, and the evidence says
stop rescuing it.** The underlying need is real and the data is safe; the
implementation is structurally unsound in four independent ways:

1. **It does not work today.** Search fails exit-7 (index-busy) behind a
   machine-global lock held 5h by a zero-progress job against a throwaway DB
   (new bead jy8v8); `stats`/`health`/`status`/`doctor --check` balloon to
   3.8–5.8 GiB RSS and hang or blow their own documented bounds (1a7mk
   reopened); `status` exits 0 over "could not be opened" (new bead k69vx). No
   artifact anywhere demonstrates end-to-end search returning a result on the
   production archive.
2. **The engine is the defect.** fsqlite 0.1.5 ("frankensqlite", a from-scratch
   Rust SQLite): whole-corpus GROUP BY 77ms in stock sqlite3 vs 7h26m+
   unfinished; MAX(id) as full table scan; the whole stack (823 crates) is
   same-author rewrites of SQLite/tokio/ratatui/tantivy. The storage layer is
   woven with fsqlite specifics — migrating back to rusqlite is a project, not
   a patch.
3. **Resource profile is design, not bug.** ~256G cass-attributable on a
   29Gi-free disk: text stored 3x (46.6G uncompressed raw mirror by design with
   no off switch, 10.3G messages table, dual FTS5+tantivy indexes), 8.2GiB
   freelist, ~190G residue produced by the rescue campaign itself. Indexing
   throughput 0.23MB/s vs 250MB/s for a naive single-threaded parse (~1000x).
4. **The fix loop does not converge.** 13 continuation generations, 83 commits,
   ~63h ending only at the 95% weekly usage cap; the same fixed-then-recurred
   class landed three times; the one fix that makes the rebuild complete (57s)
   is still unlanded on main; 6,452/10,283 codex conversations silently lost
   all tool messages at rc=0; doctor recommends the prune that deletes
   irreplaceable blobs (pfar8).

The fleet has already voted: compound-learnings quarantined cass, the Codex
surface excludes the skill, nothing in cron/launchd/shell history invokes it.

**Salvage facts:** archive is stock-SQLite-compatible; raw mirror is a
blake3-verified complete second copy — nothing is lost. Real searchable prose
is ~0.8–1.5GB of the 37.5G corpus. A right-sized replacement (rusqlite+FTS5,
external-content, mtime-incremental, existing schema shape) is ~1k LOC with a
2–4GB index; rg full-scan worst case is ~8s on this machine.

Beads filed/updated this session: jy8v8 (new, P1), k69vx (new, P2), 1a7mk
(reopened with evidence). Decision on retirement/replacement is Dale's.
