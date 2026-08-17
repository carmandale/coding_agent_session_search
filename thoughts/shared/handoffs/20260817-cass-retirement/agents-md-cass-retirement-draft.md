DRAFT — paragraph to insert in ~/.agent-config/instructions/AGENTS.md §10 Tooling Defaults,
placed adjacent to the ego-browser/dev-browser removal record (currently lines 1023-1025),
whose shape this follows deliberately: that entry is the repo's proven precedent for
retiring a tool so a tree-scanning runtime cannot re-offer it.

Bullet text (to be finalized against the discovery lanes' exact inventory):

- Session-history search: **cass is retired and deleted — there is no session-search tool on
  this machine until the replacement ships, and cass is not to be rebuilt, reinstalled, or
  restored from git history.** `cass` (the CLI of the Rust crate `coding-agent-search`) indexed
  Claude Code and Codex session logs. It was retired on 2026-08-17 by Dale's explicit
  direction — *"retire cass completely… nothing that would potentially ressurect it"* — after a
  five-lane assessment measured it structurally non-viable, preserved at
  `thoughts/shared/handoffs/20260817-cass-viability-assessment/`. The decisive facts: it was
  built on `fsqlite` 0.1.5 ("frankensqlite"), a from-scratch Rust reimplementation of SQLite,
  which took **7h26m without finishing** the same whole-corpus `GROUP BY` that stock `sqlite3`
  runs in **77ms** against the identical file; `stats`, `health`, `status` and `doctor` each
  ballooned to 3.8–5.8 GiB of memory to report their own status, and `health` measured 2,013ms
  against its own documented 50ms bound; it carried ~256G on this machine to search a 37.5G
  corpus, storing the text three times; and **no artifact in 4,261 commits ever demonstrated
  `cass search` returning a result against the production archive**. Thirteen generations of
  "fix it to green" continuation sessions over ~63 hours did not converge, and the same defect
  class landed three separate times.

  What that buys the next agent, stated as rules rather than history. **Do not reach for a
  session-search tool that is not there**: if you need to search session history, use `rg` over
  `~/.claude/projects` and `~/.codex/sessions` (measured worst case ~8s for the whole 37.5G on
  this machine) — but know that Dale's global instructions are embedded in nearly every
  transcript, so a phrase like "root cause" matches ~9,439 of 9,442 Claude files and naive
  matching is useless without narrowing. **Do not rebuild it**: the replacement is
  `~/dev/groove-session-search`, stock SQLite with FTS5 over extracted message text, treating
  the index as a disposable rebuildable cache. **Do not resurrect cass to compare against it** —
  `git log --diff-filter=D` in this repo and the archived GitHub repo
  `carmandale/coding_agent_session_search` hold everything, and un-archiving is a deliberate act
  by design.

  Removal was total rather than partial, and that choice is the dev-browser lesson applied a
  second time: quarantining a skill controls nothing on a tree-scanning runtime, and a binary on
  PATH ignores every listing-level control. cass had ALREADY been softly controlled and the soft
  controls had ALREADY failed — `compound-learnings/SKILL.md` declared it "quarantined… not a
  default learning source," and `configs/skills/codex-visible.txt` excluded it from every Codex
  session — while the binary sat on PATH and the skill sat in `~/.claude/skills/cass/` telling
  agents how to use it. So the skill, the commands, the binaries and their rollback copies, the
  data directories, the watchdog, the aliases, and both soft-control lines were removed together.
  **Removing a skill and leaving its name in `codex-visible.txt` breaks `install.sh`'s Codex
  verifier** — that is why the exclusion line goes in the same change, not after it.

  Session capture is a separate concern and it survives: the harnesses' own append-only JSONL is
  the capture system, and cass's mirror held verbatim copies of files the harnesses had since
  deleted, so the captures that existed nowhere else were extracted to
  `~/session-archive/rescued-from-cass/` before anything was destroyed. Do not delete that tree.

NOTES ON PLACEMENT / MECHANICS
- Insert as a new bullet in §10's tooling list, adjacent to the browser-automation bullet so the
  two removal records sit together and a reader learns the general lesson once.
- Check whether AGENTS.md is pinned by a fixture test (the context-followup section at §5 is,
  per its own note). If a test pins §10, re-adjudicate the fixture in the same commit.
- Also update, in the SAME commit: configs/skills/codex-visible.txt (drop the cass line),
  skills/meta/compound-learnings/SKILL.md:~24 (the quarantine sentence becomes a retirement
  pointer), and any other reference the discovery lanes name.
- Then re-run install.sh (or the narrower skill-symlink step) so ~/.claude/skills,
  ~/.codex/skills, ~/.agents/skills, ~/.gemini/skills, ~/.cursor/skills lose the cass entry, and
  run tests/test-quarantine-invisible.sh + scripts/skill-hygiene-check.sh to prove no dangling
  symlink or half-removed skill survives.
