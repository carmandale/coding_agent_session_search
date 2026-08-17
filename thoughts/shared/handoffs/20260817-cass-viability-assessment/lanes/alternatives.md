# Lane: alternatives — what the job requires, and what a right-sized solution looks like

Date: 2026-08-17. Read-only lane; no cass invocation was made (none was needed — see note on gtimeout below). All measurements executed this session on this machine (Apple M4 Max, 16 cores, 128GB RAM, NVMe).

## 1. The job, per cass's own skill and README

Sources: `~/.claude/skills/cass/SKILL.md` (the operative skill; the repo has none at `.claude/skills/cass/`), `README.md:13-14` ("Unified, high-performance TUI to index and search your local coding agent history" across 20 named harnesses).

The questions Dale actually asks of it (SKILL.md frontmatter + "Goldmine Principle" + workflow sections):

- "What did I ask?" / "find that prompt" — retrieve past user prompts, especially repeated ones (ritual detection: `total_matches > 10` = a working prompt)
- Session archaeology — "when did we decide NOT to do X?", recovery moments, scope decisions
- Cross-agent history search — one query over Claude Code + Codex (+ 18 other harnesses, of which this machine uses 2)
- Follow a hit to its context — `view`/`expand` a session file at a line, message-aware
- Resume a past session in its native harness — `cass resume PATH --shell` emits the `claude --resume <id>` / `codex resume <id>` command
- Secondary/bonus per the skill: token/cost analytics, cross-machine federation, ChatGPT import, encrypted HTML export

**Honest required-capability list** for the primary job: full-text search over *message text* (user prompts, assistant text) with agent/workspace/date filters, ranked results with snippets, file+line provenance so a hit can be opened/expanded, and a resume-command emitter (a path→session-id lookup plus string formatting). Everything else in the feature list is secondary. Notably, the skill's own guidance already concedes the core index is *partial*: "Content not found → `rg` — cass skips tool outputs" (SKILL.md line 199).

The corpus measured today: `~/.claude/projects` = 8.5G / 9,442 JSONL files; `~/.codex/sessions` = 29G / 8,757 JSONL files. Median file ~273KB (Claude) / ~742KB (Codex); the largest Claude session is 95MB, the largest Codex rollout is **2.57GB** (one file).

## 2. Baseline: how far does plain ripgrep get?

Measured, not estimated:

| Probe | Wall time |
|---|---|
| `rg -c --no-ignore -m 5 "root cause"` over 8.5G Claude corpus | 2.62s (9,439 of 9,442 files matched — see below) |
| `rg -c --no-ignore <absent literal>` over 8.5G Claude corpus — full-scan worst case, cold-ish | **1.75s** |
| Same, warm | 0.38s |
| Full-scan worst case over 29G Codex corpus | **5.93s** |

So a worst-case full literal scan of the entire 37.5G corpus is **~8 seconds** on this machine. On an M4 Max with 128GB RAM (the whole corpus fits in page cache) and NVMe, "no index at all" is not a viability problem for latency. This removes "you need an index to make search *possible*" from the argument entirely; the index has to justify itself on precision and ergonomics.

rg's real limitations here, measured:

- **Precision catastrophe, not speed.** "root cause" matched 9,439/9,442 Claude files — because the user's global CLAUDE.md is embedded verbatim in essentially every transcript, and tool outputs dominate bytes. Naive grep cannot distinguish "Dale typed this" from "the harness injected this" or "a tool printed this." Field-aware extraction is the actual value an index adds.
- **Unusable raw output.** In the 95MB sample session, the average JSONL line is 9.8KB and the max is 1.36MB. A bare rg match prints the whole line; every recipe needs `-o`/`--max-columns` plus jq post-processing.
- No ranking (file order only), no cross-file dedup, no message-boundary context (`expand` semantics), date/agent filtering only via path conventions.
- Re-reads 37.5G per query — irrelevant at 8s on this machine, but it burns I/O and doesn't scale to the mini or laptops with less RAM.

Conclusion of the probe: rg is already a serviceable floor for "does this string exist and where," which is why the cass skill itself falls back to it. What it can't do is "show me only *my prompts* that mention X, ranked."

## 3. Right-sized index: sketch and honest sizing (not built)

### What fraction of the bytes is actually message text? (measured on 5 real files)

Python pass classifying every byte per line (`text_fraction.py`, scratchpad):

| File | Size | user+assistant text | thinking/reasoning | tool traffic | structure/other |
|---|---|---|---|---|---|
| Claude subagent (median-size) | 0.27MB | 5.4% | 0% | 35.1% | 59.5% |
| Claude mainline session | 6.8MB | 3.1% | 0% | 27.0% | 69.9% |
| Claude largest session | 94.9MB | **0.3%** | 0% | **87.0%** | 12.8% |
| Codex median rollout | 0.74MB | 20.2% | 0% | 52.8% | 27.1% |
| Codex mid-size rollout | 9.7MB | 5.7% | 0% | 51.8% | 42.5% |

Two structural facts fall out:

1. **Claude Code stores every tool result twice** — once as a `tool_result` content block and again as a top-level `toolUseResult` field. In the 95MB session those two categories are 42.6% + 43.4% = 86% of the file. The raw corpus is itself ~2x-inflated for Claude sessions.
2. Byte-weighted (large files dominate, and large files are tool-noise-heavy), **actual message text is roughly 1-4% of corpus bytes**. Call it 2-3%: the searchable prose in this 37.5G corpus is on the order of **0.8-1.5GB**; add tool-call *inputs* (commands run — genuinely worth searching) and it stays ≤ ~2.5GB.

### The sketch

Single SQLite file, stock SQLite, FTS5:

- `messages(id, file_id, line_no, role, agent, workspace, ts)` + `files(id, path, mtime, size, agent)` + `fts` as an FTS5 external-content table over the extracted text (`tokenize='porter unicode61'`, `detail=full` for phrase queries).
- Indexer: walk the two roots, re-extract any file whose (mtime,size) changed — both formats are append-only per session, so incremental = re-parse changed files only, delete rows for vanished files. No daemon, no watcher, no atomic-swap generations: one SQLite transaction per file is the whole durability story.
- Query: `SELECT ... snippet(fts) ... WHERE fts MATCH ? AND role='user' AND ts > ? ORDER BY bm25(fts)`. `--role user` *is* the "find my prompts" feature, done properly instead of the skill's `line_number <= 3` heuristic.
- view/expand: open path, seek to line, print ±N messages. Resume: read session id from the file's first lines, print the harness command.

### Sizing, using standard FTS5 overhead (30-60% of indexed text)

- Extracted text stored: ~1.0-2.5GB. FTS5 index: ~0.5-1.5GB. **Total ~2-4GB**, one file.
- Against cass's current footprint: **77G** (46G raw-mirror — a *copy* of source files that are already on local disk and readable by path; 22G SQLite on a pinned custom "frankensqlite" engine; 9.5G tantivy). The right-sized design is ~20-40x smaller and its largest component (the 46G mirror) simply has no reason to exist for a local-only corpus.

### Throughput, measured

Single-threaded Python (json.loads per line + full byte classification) processed the 95MB session in **0.38s ≈ 250MB/s**. Even at a pessimistic 5x slowdown for FTS5 inserts, a **full-corpus rebuild is ~15-40 minutes single-core, minutes if parallelized**. Against the napkin's measured cass rate: 369.8MB in 26.8 min = **0.23MB/s** — three orders of magnitude slower — with an extrapolated 12-15 hours and +24-30GB for the remaining codex backlog. The gap is not Rust-vs-Python; it is architecture (raw mirroring, dual engines, semantic enrichment, per-generation publishing) versus parse-and-insert.

### LOC, honestly

Extractors for the two formats ~250; walker + mtime incremental ~80; schema/ingest ~120; query CLI with filters/snippets/JSON ~200; view/expand + resume ~100. **≈ 700-900 lines** in one Python file (or the same shape in Rust/rusqlite). cass is **393,912 LOC** in src/ (sqlite.rs alone ~20k) plus 225 test files — roughly **500x** the code for the capability set the skill actually exercises. "Hundreds, not thousands" holds.

## 4. Prior art

- **Native harness pickers already cover resume.** `claude --resume` opens a session picker with a search/filter mode, `claude -c` continues, `/history` lists in-session; `codex resume` has a picker, `--last`, and id-addressed resume, plus `fork`/`archive`/`delete` subcommands (measured from `--help` output today). What natives lack is *cross-project, cross-harness full-text search* — that delta is exactly the §3 sketch, nothing more.
- **Community tools occupy the same niche at small size:** [search-sessions](https://github.com/sinzin91/search-sessions) (sub-second CLI search over all Claude Code history, resumable via session UUID), Claude Explorer ([overview](https://easyclaw.com/blog/knowledge/claude-code-history-viewer-compared/)), plus how-to prior art ([LLMnesia](https://www.llmnesia.com/blog/search-claude-code-conversation-history), [codeagentswarm guide](https://www.codeagentswarm.com/en/guides/claude-code-history-complete-guide), [Definite's search skill](https://www.definite.app/blog/claude-code-search-skill), [Raymond Peck series](https://medium.com/@raymondpeck/unlocking-your-claude-history-part-1-f19000c05655)). Codex side: [codex-trace](https://github.com/PixelPaw-Labs/codex-trace) (session viewer over `~/.codex/sessions`), a [Codex session-history skill](https://mcpmarket.com/tools/skills/codex-session-history-manager), and [resume docs](https://deepwiki.com/openai/codex/4.4-session-resumption). None found that unifies both harnesses in one index — that is cass's genuine differentiator, and also the part the §3 sketch reproduces in ~250 LOC of extractors.

## 5. Migration cost: what actually depends on cass here

Searched today — `~/.agent-config` (scripts, skills, configs, commands), crontab, `~/Library/LaunchAgents`, `~/.claude/settings.json` hooks, `~/.zsh_history`, sibling repos' AGENTS.md:

- **Nothing scheduled or hooked invokes cass.** No cron entry, no launchd agent, no Claude Code hook.
- **The fleet has already routed around it.** `~/.agent-config/skills/meta/compound-learnings/SKILL.md:24`: "**CASS is quarantined:** Do not invoke `cass` for this workflow... not a default learning source while search/index/watch are unreliable." `configs/skills/codex-visible.txt:200`: `exclude:cass` (hidden from every Codex session). The skill remains visible to Claude Code per `claude-curation.txt:66`.
- **Zero `cass` commands in `~/.zsh_history`.**
- Remaining real touchpoints, all small: one recipe line in `skills/tools/testflight/SKILL.md:385` (`cass search "upload-to-testflight-complete"`); a `cass-macbook` alias + `CASS_MACBOOK_DB` in `configs/shell/zshrc.local.mini:31-32` (mini queries a mirrored laptop DB); `disk-janitor.sh:457` knows cass's data path for cleanup; sibling Dicklesworthstone-ecosystem repos (`asupersync`, `destructive_command_guard`, `ultimate_bug_scanner`) carry boilerplate "use cass" sections in their vendored AGENTS.md — upstream authored, not local dependencies.

**What would genuinely be lost by dropping it:** the cross-harness unified index (reproducible small, §3); `resume` cross-harness emission (a lookup + format string); token/cost analytics (partially covered by the `claude-usage`/cusage skill for Claude; Codex-side analytics would lapse); semantic search (never a load-bearing path — the skill itself says lexical is the default and semantic silently falls back); cross-machine federation (the mini alias is its one observed consumer); encrypted HTML export/pages (no observed local use).

**Carrying cost of keeping it green, for contrast** (this lane's context + today's shared measurements): 77G data dir on a disk with 29Gi free, 69G of worktrees inside the repo, `cass stats --json` at 5.2GB RSS not returning in 3.5 min, a search probe past 4h48m, 0.23MB/s indexing with a 12-15h backlog, a pinned custom SQLite engine that "cannot run the GROUP BY" (p3kgr), and a 394k-LOC codebase to diagnose it all in. The switching cost is editing one skill recipe, one alias, and retiring one skill directory.

## Note on environment

`gtimeout`/GNU `timeout` is **not installed on this machine** (checked PATH, `/opt/homebrew/bin`, coreutils gnubin). The cass SKILL.md lists it as a dependency ("recommended; cass index can hang under contention") and its recovery scripts wrap every index call in `timeout` — those wrappers are currently no-ops-by-absence fleet-wide. This lane therefore ran no cass command at all (none was required by its questions).

## Structural vs incidental

- **Structural (design-inherent):** the 46G raw mirror duplicating local files; dual index engines (custom-pinned SQLite + tantivy) with generation publishing/backups/quarantine machinery; semantic/HNSW enrichment; 394k LOC to maintain — all serve an architecture sized for a product, not for the two-harness local-search job. The 1000x indexing-throughput gap vs a naive parse is architectural, not a bug to patch.
- **Incidental (fixable in principle):** individual hangs (issue #196), the GROUP BY failure on the pinned engine, the stall-detector false-firing, stats blowing up to 5.2GB RSS. Each is fixable — inside a codebase whose size makes each fix a project.

## Bottom line

The job cass performs for this machine — ranked full-text search over ~1-1.5GB of actual message text with role/agent/date filters, plus context view and resume emission — is an ~800-line SQLite-FTS5 tool over a ~2-4GB single-file index, rebuildable from scratch in well under an hour; rg alone already covers the existence-check floor in ~8s per full-corpus scan, and nothing on this machine except one skill recipe and one mini alias would notice cass's absence, because the fleet's own instructions already quarantined it.
