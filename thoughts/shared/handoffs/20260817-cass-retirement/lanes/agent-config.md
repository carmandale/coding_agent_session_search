# Lane: agent-config sweep — cass retirement discovery

- **Lane:** agent-config sweep (read-only discovery)
- **Owner:** subagent, session 656f2411-6418-4df9-9965-55219cd71762
- **Date:** 2026-08-17
- **Scope:** `/Users/dalecarman/.agent-config` (whole repo, tracked + ignored) plus the live
  runtime surfaces `~/.claude`, `~/.codex`, `~/.gemini`, `~/.cursor`, `~/.pi`, `~/.agents`,
  launchd, cron, and `jsm`.
- **Writes performed:** this file only. No deletes, no edits, no bead mutations, no cass
  binary invocations.

---

## 1. Method and instrument honesty

Whole-repo census first, then classification. The census instrument was a word-form
enumeration rather than a line grep, because `cass` is a substring of several unrelated
words and a per-file line count understates single-line JSON/HTML blobs:

```
cd /Users/dalecarman/.agent-config && rg -i -o '\b[a-z_-]*cass[a-z_-]*\b' --no-filename \
  | tr 'A-Z' 'a-z' | sort | uniq -c | sort -rn
```

Baseline: **1821 occurrences / 1380 matched lines / 260 files** out of 8998 files searched
(`rg -i -c 'cass' --stats`).

### Instrument corrections made mid-sweep (both worth recording)

**A zero that was wrong.** `rg -i -l '\bcass\b' ~/.codex/worktrees/` returned **0 files**.
A positive control (`ls ~/.codex/worktrees/9218/.agent-config/skills/tools/cass/` → `SKILL.md`)
proved the instrument was dead: rg honours the `.gitignore` inside each worktree clone.
Re-run with `--no-ignore --hidden` → **222 files**. Had I trusted the zero, the single most
dangerous artifact in this lane (a full older agent-config clone containing the deleted cass
skill) would have been reported as absent.

**A file count that understated by 188x.** `rg -i -c 'cass' tools/codex-skill-review/index.html`
reports `1`. That file embeds the entire cass `SKILL.md` as JSON on one physical line;
`rg -i -o 'cass' … | wc -l` reports **188**. Per-file line counts are not occurrence counts.

### False positives filtered

| form | occurrences | what it really is |
|---|---|---|
| `cassette` + `cassettes` + `cassette-mode` + `cassette-path` + `use_cassette` + `cassette-vs-predicted` + `cassette-replay` + `cassette-regeneration` | 100 | spec 116/117 halt-renderer test cassettes; VCR fixtures in the dhh-rails-style skill |
| `halt-renderer-cassettes` | 53 | same family (spec 117 cut-notes) |
| `cassandra_wide-column_database_01/02` | 2 | `skills/domain/visualization/uml/stencils/alibaba_cloud.md:54,55` |
| `managed_apache_cassandra_service` | 1 | `skills/domain/visualization/uml/stencils/aws4.md:654` |
| `yykcassopb` | 1 | random token in a probe fixture |

**149 of 1821 occurrences filtered as false positives (8.2%). ~1672 genuine.**

Two forms I initially suspected as false positives and **resolved as genuine** after
inspection — recording them so nobody re-filters them:

- `ncass` (48) — this is `\n` + `cass` inside escaped JSON. All 48 are in
  `tools/codex-skill-review/index.html`, and they are the embedded cass skill body.
- `cassfixture*` (91: `cassfixture_owner_state` 45, `cassfixture_owner_is_live` 34,
  `cassfixture` 6, `shape_matches_cassfixture` 4, +2) — spec 279's "cass fixture leak"
  family in `specs/279-disk-janitor-authority-v3/artifacts/proto279.sh` and its lanes.
  Genuine cass, not "cassette fixture".

Runtime-directory false positives (outside the agent-config census): `.xcassets` in
`~/.cursor/plans/stereo_image_pipeline_d175f2c4.plan.md` and in `~/.pi/agent/sessions/**` +
`~/.pi/deck-snapshots/**` (all verified by reading context — Xcode asset catalogs, not cass);
Zxcvbn surname/password dictionaries in `~/.gemini/antigravity-browser-profile/ZxcvbnData/3/*`
and `~/.codex/worktrees/121f/netapp-rfp/.agent-state/**/ZxcvbnData/3/surnames.txt`; TypeScript
"casing" strings, vite `publicAssetUrl*`, and vitest `withAwaitAsyncAssertions` in
`~/.claude/hooks/node_modules/**`; `~/.gemini/config/plugins/science/**` reference files.

**`~/.pi` contains no genuine cass reference at all.** Every hit there is `.xcassets`.

---

## 2. TIER 1 — LIVE INSTRUCTION. These are the resurrection vectors.

### 1A. The cass skill itself — four real directories, and jsm owns them

```
/Users/dalecarman/.claude/skills/cass/     196K, 23 files, May  9 03:58
/Users/dalecarman/.codex/skills/cass/      196K, 23 files, May  9 03:58
/Users/dalecarman/.gemini/skills/cass/     196K, 23 files, May  9 03:58
/Users/dalecarman/.cursor/skills/cass/     196K, 23 files, May  9 03:58
```

Contents (identical in all four): `SKILL.md` (28115 bytes), `SELF-TEST.md`,
`references/{ANALYTICS,ANTI_PATTERNS,COMMANDS,HARNESS_EXCLUSION,INTROSPECTION,OBSERVABILITY,PAGES_AND_EXPORT,PATTERNS,PITFALLS,PROMPTS,RECIPES,RECOVERY,REMOTE_SOURCES,RESUME,SEMANTIC_AND_HYBRID,SESSION_FORMATS}.md`,
`scripts/{multi_machine_search.sh,prompt_miner.py,quick_analysis.sh,recover.sh,validate.sh}`.

These are **real directories, not agent-config symlinks** — `skills/tools/cass/` no longer
exists in agent-config (deleted at `aa98f6996 chore(skills): resolve jsm collisions and add
hygiene check`). So `install.sh` never creates them and no install.sh re-run is needed to
clear a symlink.

**The resurrection vector is `jsm`.** `jsm list` reports cass as an installed, managed skill:

```
NAME                       VERSION   STATUS     INSTALLED
cass                       7         ? unknown  2026-05-09
gcloud                     1         ? unknown  2026-05-08
rch                        5         ? unknown  2026-05-09
rust-unsafe-code-exorcist  8         ? unknown  2026-05-15
vercel                     4         ? unknown  2026-05-16
```

Deleting the four directories without `jsm uninstall cass` leaves jsm believing cass is
installed and able to restore it. (No jsm LaunchAgent is currently loaded — `launchctl list`
and `~/Library/LaunchAgents` are both clean of jsm and cass — so there is no *automatic*
daily reinstall today, contrary to what `.claude/rules/installer-hygiene.md` describes. But
any manual `jsm install-all` / `jsm update` would bring it back.)

**This skill is currently advertised to every Claude Code session.** It appears in this
subagent's own skill listing as `cass: Mine past agent sessions for working prompts,
decisions, and patterns…`.

### 1B. agent-config allowlists — the reason it is visible

| path:line | content | class |
|---|---|---|
| `configs/skills/claude-curation.txt:66` | `cass\|external\|visible\|external kept visible (dispatch/reference evidence)` | **INSTRUCTION** — this row is what keeps cass in the Claude Code listing |
| `configs/skills/codex-visible.txt:44-45` | `# CASS quarantined 2026-05-25: keep it out of default agent startup until` / `# upstream search/index/watch are reliable again.` | **INSTRUCTION** — "until reliable again" explicitly invites revival |
| `configs/skills/codex-visible.txt:200` | `exclude:cass` | coupling anchor — see Tier 2 |

The curation-list header (lines 60-72 of `claude-curation.txt`) states that
`external`/`bundled` rows are **never deleted, only flipped `hide`<->`visible`**, because
"row retention IS the ownership ledger". The sanctioned edit is therefore flipping line 66 to
`hide` (with a retirement reason) and running
`/usr/bin/python3 scripts/skill-curation-apply.py` — homebrew python3 lacks yaml.

### 1C. Live agent-config skills that instruct an agent to run cass

| path:line | content | note |
|---|---|---|
| `skills/tools/testflight/SKILL.md:385` | `cass search "upload-to-testflight-complete" --robot --limit 10 --days 365` | recipe line inside a live skill |
| `skills/tools/gj-tool/SKILL.md:59` | `gj sessions [path]        # CASS TUI for agent history` | documents a `gj` subcommand that launches cass — the gj-tool repo is a separate owner, flag to that lane |
| `skills/meta/compound-learnings/SKILL.md:24` | `**CASS is quarantined:** Do not invoke \`cass\` for this workflow unless the user explicitly asks for a CASS-specific check. It is not a default learning source while search/index/watch are unreliable.` | leaves a door open on both clauses ("unless the user asks", "while … unreliable") — rewrite to unconditional retirement |
| `skills/_quarantine/tools/cm/SKILL.md.quarantined` (12 lines: 3, 7, 9, 21, 360, 730, 749, 751, 755, 780, 803, 815) | whole CASS-backed memory system; frontmatter description reads `QUARANTINED: CASS-backed memory system. Do not invoke … until CASS search/index/watch are reliable again` | correctly suppressed from Codex by the `SKILL.md.quarantined` rename, but see 1D — its Gemini command wrapper is NOT suppressed |

### 1D. Generated command wrappers — live, and some are orphaned

Source in agent-config:

- `commands/debug-plus.md:69` and `:99` — `query_cass=true`

Deployed copies (these are the ones an agent actually reads):

| path:line | content |
|---|---|
| `~/.claude/commands/debug-plus.md:69,99` | `query_cass=true` (regular file copy, Jul 7 20:07) |
| `~/.gemini/commands/debug-plus.toml:70,100` | `query_cass=true` |
| `~/.gemini/commands/skills/compound-learnings.toml:34` | the quarantine paragraph from 1C |
| `~/.gemini/commands/skills/gj-tool.toml:69` | `gj sessions [path]  # CASS TUI for agent history` |
| `~/.gemini/commands/skills/testflight.toml:395` | `cass search "upload-to-testflight-complete" …` |
| `~/.gemini/commands/skills/cm.toml` — 13 lines (4, 13, 17, 19, 31, 370, 740, 759, 761, 765, 790, 813, 825) | the **entire** CM/CASS memory system body, including `\| **Episodic Memory** \| Raw session transcripts \| ~/.local/share/cass/ \| cass \|` and `Run cass reindex to rebuild session index` |
| `~/.gemini/commands/skills/agent-ergonomics-and-agent-intuitiveness-maximization-for-cli-tools.toml` — ~16 lines (33, 70, 142, 148, 314, 332, 373, 374, 376, 377, 550, 590, 592, 593, 595, 662, 684, 700) | heavy cass dependency: a `cass-miner` subagent, "Deep CASS mining recipes (38+ targeted queries)", cass as the canonical exemplar for `capabilities --json` / `robot-docs guide` / `--robot-meta` |

**Two of these are orphans whose source skill no longer exists.** `cm` is quarantined in
agent-config; `agent-ergonomics-and-agent-intuitiveness-maximization-for-cli-tools` exists
**nowhere** on this machine as a skill —
`fd -H -t d -i 'agent-ergonomics' ~/.claude ~/.codex ~/.gemini ~/.cursor ~/.agent-config ~/.pi`
returns nothing. Only the live Gemini command wrapper survives. This is the quarantine gap
`installer-hygiene.md` describes, reaching a surface nobody checked: **the `SKILL.md.quarantined`
rename suppresses Codex discovery but does not remove an already-generated `.toml` command.**
`~/.gemini/commands/skills/` holds 237 wrappers.

### 1E. Other jsm skills with cass wired in — 4 runtime copies each

**`rust-unsafe-code-exorcist`** (jsm v8; live and in this subagent's own skill listing).
25 files reference cass. Paths below are the `~/.claude/skills/` copy; identical trees exist
under `~/.codex/skills/`, `~/.gemini/skills/`, `~/.cursor/skills/`.

| file | cass occurrences | note |
|---|---|---|
| `references/source/CASS-QUERY-PACK.md` | 131 | whole file is a cass query pack |
| `references/methodology/CASS-MINING-DEEP.md` | 81 | |
| `references/methodology/CASS-MINING.md` | 78 | |
| `subagents/cass-miner.md` | 16 | lines 2, 10, 12, 17, 21, 30, 32, 35, 36 — `cass search "$query" --robot --limit 30`, `--host "$host"` over `css csd ts1 ts2` |
| `SKILL.md` | 14 | lines 86, 118, 135, 137, 141, 208, 717, 763, 764, 783, 835, 896, 934, 962. Line 137: *"**CASS available?** If `/cass` is installed and indexed, run `subagents/cass-miner.md` BEFORE Phase 1"* |
| `scripts/cass-mine.sh` | 8 | executable; `HOSTS=(localhost css csd ts1 ts2)` |
| `references/methodology/SOUNDNESS-ARCHEOLOGY.md` | 7 | |
| `references/methodology/AGENT-PROMPTS.md` | 5 | |
| `references/methodology/{SKILL-FALLBACKS,SOURCE-CORPUS}.md` | 4 each | `SKILL-FALLBACKS.md:150-156` |
| `references/methodology/PHASES.md`, `subagents/archeologist.md` | 3 each | |
| `references/methodology/{PREREQUISITES,GLOSSARY,MODEL-DIFFERENCES}.md` | 2 each | `PREREQUISITES.md:57` |
| `assets/intake-prompt.md`, `scripts/check-skills.sh:22`, `scripts/validate-corpus.py:2`, `references/source/EXEMPLAR-CATALOG.md`, `references/methodology/{KICKOFF-PROMPTS,DECISION-TREE,COOKBOOK,QUICK-REFERENCE,PLATFORM-NOTES,MENTAL-MODEL}.md` | 1 each | |

Mitigating fact, measured: `references/methodology/SKILL-FALLBACKS.md:154` states *"Skip Phase
0.5 mining; rely entirely on the exemplar-miner … no other phase depends on cass output."* So
the skill degrades gracefully — but it still tells agents to run cass, and `:156` says *"If
`cass` binary is installed but the skill isn't, use it directly per the query pack."*

**`rch`** (jsm v5; 4 runtime copies):

| path:line | content |
|---|---|
| `~/.claude/skills/rch/SKILL.md:225` | `**cass** — search prior agent sessions; the skill ships scripts/mine_rch_history.sh as a fallback when cass index has dead pointers` |
| `~/.claude/skills/rch/scripts/mine_rch_history.sh:4,84,85` | `cass search "$PATTERN" --robot --limit 30` |
| `~/.claude/skills/rch/references/MACHINE_INTROSPECTION.md:193` | `cass robot-docs guide is the cass equivalent (used by the cass skill)` |
| `~/.claude/skills/rch/references/TELEMETRY_RECOVERY.md:14` | `(cass evidence: 30+ incidents under "Telemetry database integrity check failed")` — historical cite only |

Both are vendor (jsm) content. Editing them forks vendor bytes that a `jsm update` can
overwrite; `jsm uninstall` is the clean lever but removes the whole skill, and
`rust-unsafe-code-exorcist` is a skill Dale uses. **This one needs a decision, not a
mechanical delete.**

### 1F. The sessions-wiki pipeline — live, registered, and cass-fed

This is the vector the earlier inventory did not name, and it is reachable from any session.

| path:line | content | class |
|---|---|---|
| `configs/wiki-registry.json` | registers `"name": "sessions-wiki"`, `"path": "$HOME/Projects/sessions-wiki"`, description *"Compiled knowledge from all agent sessions across all harnesses and workspaces"* — queryable from any repo via `/wiki-query --wiki sessions-wiki` | **INSTRUCTION** (entry point) |
| `~/Projects/sessions-wiki/CLAUDE.md` | live auto-loaded CLAUDE.md. Frontmatter schema field `cass_session_ids: [session-id-1, session-id-2]`. Section `## Compile Pipeline`: *"Knowledge is extracted nightly at 3 AM on mini via `compile.py`. The pipeline: 1. **Reads new sessions from cass**"*. `Manual trigger: /sessions-compile` | **INSTRUCTION** — highest-priority rewrite outside agent-config |
| `~/Projects/sessions-wiki/wiki/outputs/what-patterns-exist.md:22` | *"The wiki will grow substantially after the cass backfill processes ~10K historical sessions."* | INSTRUCTION-adjacent (states pending cass work) |
| `specs/074-claude-sessions-wiki/scripts/cass_client.py` | 11 hits — `"""Thin wrapper around cass CLI for sessions-wiki pipeline."""`, `REQUIRED_API = "v1"`, `REQUIRED_CONTRACT = "v1"` | **the only copy of the pipeline's cass client** |
| `specs/074-claude-sessions-wiki/scripts/backfill.py` | 9 hits | imports cass_client |
| `specs/074-claude-sessions-wiki/scripts/compile.py` | 6 hits | imports cass_client |
| `specs/074-claude-sessions-wiki/scripts/{write_wiki.py,test_write_wiki.py}` | 1 each | |

`/sessions-compile` was curation-deleted from agent-config — only
`commands/sessions-compile.md.curation-backup.20260710-165559` remains (itself cass-free, and
untracked: `git ls-files` returns nothing for it). **But the live command file survives inside
a stale worktree clone — see 1I.**

**Cross-lane hazard:** the CLAUDE.md says the compile runs *nightly at 3 AM on mini*. Nothing
in this lane's scope can see the mini's launchd. Flag to whoever owns the mini.

### 1G. Shell alias (mini profile, tracked in agent-config)

| path:line | content |
|---|---|
| `configs/shell/zshrc.local.mini:30` | `# CASS MacBook mirror — synced nightly from MacBook Pro` |
| `configs/shell/zshrc.local.mini:31` | `export CASS_MACBOOK_DB="$HOME/cass-mirror/agent_search.db"` |
| `configs/shell/zshrc.local.mini:32` | `alias cass-macbook='cass --db "$CASS_MACBOOK_DB"'` |

The **laptop** profiles are clean — verified `rg -i 'cass' ~/.zshrc.local ~/.zshenv ~/.zshrc`
returns nothing, and `configs/shell/zshrc.local` has zero cass hits.

### 1H. disk-janitor cass-specific machinery (live scripts — protective, do not just delete)

| path:line | content |
|---|---|
| `scripts/disk-janitor.sh:457` | `"$HOME/.local/share/cass"` — a **denylist** entry protecting the cass index from deletion |
| `tests/test-disk-janitor.sh:2563` | `for root in "$HOME/.Trash" "$HOME/.codex/sessions" "$HOME/.local/share/cass"; do` |
| `tests/test-disk-janitor.sh:2569` | `fail "guard denies the remaining spec-150 denylist roots" "one of Trash/codex-sessions/cass was allowed"` |
| `docs/automations/disk-janitor.md:23` | documents `~/.local/share/cass` in the denylist |

These three are coupled: removing the denylist entry without editing the test turns
`tests/test-disk-janitor.sh` red. And the denylist is a *protection* — if the coordinator
intends to delete `~/.local/share/cass` as part of retirement, the deletion must happen
**before or independently of** removing the guard, and the janitor guard should probably keep
protecting the path until the data is gone.

### 1I. Stale agent-config clones — resurrection by checkout

`~/.codex/worktrees/` — 661M total, five entries, `.metadata_never_index` present:

| worktree | size | cass files | what it is |
|---|---|---|---|
| `9218/.agent-config` | 38M | **45** | full agent-config at `ffaa5e854 fix: align north-star halt and spec templates` (May 8). **Contains `skills/tools/cass/SKILL.md`** — the deleted cass skill, with `# CASS - Coding Agent Session Search` and `**NEVER run bare cass** - it launches an interactive TUI that blocks your session!`. Also `commands/sessions-compile.md` (the live command) and `configs/claude/hooks/src/installed-binary-guard.ts:27` → `` [`${HOME}/.local/bin/cass`]: { `` |
| `8096/.agent-config` | 171M | **176** | full agent-config at `0a056470b spec 266: correct proof sequence before review` (Aug 12) — carries `configs/skills/{codex-visible,claude-curation}.txt`, `configs/shell/zshrc.local.mini`, `napkin.md`, `docs/automations/disk-janitor.md`, `.claude/rules/instrument-labels.md`, `configs/mini-host/*`, `.beads/issues.jsonl` |
| `121f/netapp-rfp` | 422M | 1 | **false positive** (Zxcvbn `surnames.txt`) |
| `0fd5/quickbooks` | 14M | 0 | clean |
| `643d/quickbooks` | 14M | 0 | clean |

Worth noting for its own sake: the **current** `configs/claude/hooks/src/installed-binary-guard.ts`
maps only `` `${HOME}/bin/gj` `` — verified cass-free, as are the deployed
`~/.claude/hooks/src/installed-binary-guard.ts` and `~/.claude/hooks/dist/installed-binary-guard.mjs`
and `configs/pi/extensions/installed-binary-guard.ts`. The cass entry exists **only** in the
9218 clone. That is exactly the danger: an agent that `cd`s into that clone inherits a hook
config, a cass skill, and a sessions-compile command that the live tree has already retired.

### 1J. `.agent-state` clones (gitignored, 1.3G, 1086 cass-referencing files)

`.agent-state/` is gitignored (`.gitignore:4`). It holds seven full agent-config clones:

```
161 files  .agent-state/spec257-r11-state-current/
161 files  .agent-state/spec257-r11-state-control/
161 files  .agent-state/spec257-r11-state/
161 files  .agent-state/spec257-r11-skill/
161 files  .agent-state/spec257-r11-agents/
161 files  .agent-state/r13-mutants/
108 files  .agent-state/spec224-review-clone/
  5 files  .agent-state/dirtiness/
  2 files  .agent-state/verification/
  2 files  .agent-state/tldraw-health/
  1 file   .agent-state/{bear-routing-codex-verify,ac5-evidence,daily-bug-scan-topview-venv}/
```

Same shape of hazard as 1I but session-local and disposable.

---

## 3. TIER 2 — ORDERING HAZARDS AND STRUCTURAL COUPLING

**Read this section before touching any allowlist.**

### 2.1 `exclude:cass` is load-bearing in three places at once

Removing `configs/skills/codex-visible.txt:200` alone **breaks the allowlist validator**.
`scripts/agent-skill:42` sets `CODEX_VISIBLE_V3 = DEFAULT_SPEC_DIR / "codex-visible-allowlist-v3.md"`,
`parse_v3_seed()` (line 417) reads the second ```` ```text ```` block of that file as the
required exclusion seed, and line 579-583 raises
`f"{path}: missing V3 exclusions: {', '.join(missing_excluded)}"`.

The seed row is `specs/133-configurable-codex-skills/codex-visible-allowlist-v3.md:87`:

```
cass        # Quarantined until further notice; search/index/watch behavior is unreliable.
```

Measured now (read-only): `python3 scripts/agent-skill validate codex --enforce-v3` →
`ok: True`, `excluded_count: 13`,
`excluded_names: ['agent-mail','beads-bv','beads-triage','beads-workflow','browser','cass','chrome','commit','dev-browser','explore','fix','help','make-pdf']`.

So the allowlist line and the spec-133 seed line must move in the same commit.

### 2.2 `tests/test-agent-skill.sh` hardcodes cass as its test fixture

Eleven sites: lines **552, 554, 555, 645, 865, 1126, 1142, 1230, 1268, 1409, 1499, 1553**.

The assertions that will fail:

```
552:  assert not (stage / "cass").exists()
554:  assert "cass" not in manifest["external_names"]
555:  assert "cass" in manifest["excluded_names"]
865:  assert "cass/SKILL.md" in config_text
1142: assert payload["visible_excluded"] == ["cass"], payload
```

And the comment at 1139-1141 explains *why* it was chosen, which is the whole trap:

```
# cass replaces shape as the excluded-name fixture: shape was deleted from the
# repo (559e4b5c, 2026-08-07) and dropped from the V3 excluded block, so it no
# longer trips the excluded check; cass is a real, durable excluded entry.
```

The suite deliberately depends on cass being a *durable* exclusion. Retiring cass must
substitute another durable excluded name — twelve remain (list in 2.1) — in the same commit.
Lines 645, 1230, 1409, 1499 also use `cass` as an **external-fixture directory name**
(`for external_name in cass beads-br …`), which is a separate use and can be renamed freely.

### 2.3 `tests/test-skill-curation.sh` will NOT break — verified

It uses generic fixtures (`extrow`, `extkeep`), never cass:
`rg -i -n 'cass' tests/test-skill-curation.sh` → no cass hits. Flipping the
`claude-curation.txt:66` row is safe from that suite's perspective.

### 2.4 `~/.codex/config.toml` carries a managed entry that will dangle

`~/.codex/config.toml:891`:

```toml
[[skills.config]]
path = "/Users/dalecarman/.codex/skills/cass/SKILL.md"
enabled = false
```

One of 234 `[[skills.config]]` entries; written by `agent-skill apply codex`. Deleting the
directory without re-applying leaves a config entry pointing at a missing path.

### 2.5 install.sh / bootstrap.sh have no cass-specific handling

Verified: `rg -i 'cass' install.sh bootstrap.sh scripts/agent-skill` → **zero cass hits**
(the only `scripts/` hit anywhere is `disk-janitor.sh:457`).

Consequence for the symlink question in the brief: because `skills/tools/cass/` does not exist
in agent-config, `install.sh` creates **no** cass symlink into `~/.claude/skills`,
`~/.codex/skills`, `~/.gemini/skills`, `~/.cursor/skills`, or `~/.agents/skills`. The four
runtime cass dirs are real jsm content. So **no install.sh re-run is required to clear a
symlink** — but one *is* required after editing the two allowlists, via
`/usr/bin/python3 scripts/skill-curation-apply.py` (Claude listing) and
`agent-skill apply codex` (Codex surface). `~/.agents/skills` is a whole-tree symlink to
`/Users/dalecarman/.agent-config/skills` (`~/.agents/skills -> …/.agent-config/skills`), so it
inherits whatever the repo tree holds and needs nothing separate.

### 2.6 A stale measurement comment goes out of date (no test asserts it)

`scripts/lib/collision-check.sh:339-341`:

```
# Measured 2026-08-12 across the 7 real directories in ~/.claude/skills, the
# only marked one is the vendor-app install (tldraw-offline); every
# skill-manager drop (cass, gcloud, goalbuddy, rch, rust-unsafe-code-exorcist,
# vercel) is unmarked.
```

Removing the cass dir makes it 6 of 7 and drops a named example. I checked for a test
asserting that count — there is none. Comment-only, but it is a *measured* claim in a live
guard, so it should be re-measured rather than silently left wrong.

---

## 4. TIER 3 — HISTORICAL RECORD. Preserve; some want a retirement note.

### 4.1 Live low-authority files carrying cass as evidence for general lessons

| path:line | what it is | recommendation |
|---|---|---|
| `napkin.md:20` (`## Today`) | the `cp`-over-a-live-binary → `Killed: 9` lesson, evidenced by `cass.pre-coverage-floor-20260601 --version → ok, while cass --version → …` | keep — the lesson is general; cass is only its evidence |
| `napkin.md:42` (`## Corrections` table) | the br-cutover-verification row: *"the installed binary was the pre-fix build …"*, `Measured while deploying cass (coding_agent_session_search-c7yaw)`, and *"deployment IS proven: **present is not working.** The fix shipped a `cass health` hang"*. Row's last cell reads `Pending promotion: .claude/rul…` | keep the lesson; **check whether retirement discharges the `Pending promotion:` marker** per the §1 napkin grammar / `close-check` |
| `.claude/rules/instrument-labels.md:128` | *"The differential-specimen section was promoted 2026-08-10 from the cass deployment verification"* | keep — provenance cite, not an instruction |

Negative confirmations (all measured, all zero cass hits): `instructions/AGENTS.md`,
`configs/wiki-registry.json` (registers sessions-wiki but the word "cass" never appears),
`configs/br-version.txt`, `configs/gitignore-agent-infra`, `configs/dcg/**`,
`configs/claude/**`, `configs/codex/**`, `PROJECT_MEMORY.md`, `TENETS.md`, `docs/adr/**`,
`tools-bin/**`, `configs/skills/claude-curation-originals.json`, `~/.claude/settings.json`,
`~/.claude/settings.local.json`, `~/.codex/AGENTS.md`, `~/.codex/prompts/`,
`~/.claude/plugins/`, `~/.claude/agents/`, `~/.agents/.skill-lock.json`,
`~/.agents/skills.backup.20260303-161237/`, `crontab -l`, `~/Library/LaunchAgents/*.plist`,
`/Library/LaunchAgents`, `/Library/LaunchDaemons`, `launchctl list`.
`~/.claude/CLAUDE.md` is a symlink to `.agent-config/instructions/AGENTS.md` — clean.

### 4.2 Claude Code project memory — auto-loaded, and currently a *guard*

```
~/.claude/projects/-Users-dalecarman-dev-coding-agent-session-search/memory/MEMORY.md
~/.claude/projects/-Users-dalecarman-dev-coding-agent-session-search/memory/cass-viability-verdict.md
```

Both written today (Aug 17 10:01) by this retirement effort's own assessment session. The
verdict file already tells future agents *"do not start new 'fix cass to green' campaigns
without Dale explicitly re-deciding"* and *"Do not run `cass mirror prune`"*. **Do not delete
these** — they are the anti-resurrection record. They should be *updated* from "replacement
recommended / don't re-run rescue campaigns" to "retired 2026-08-17 by Dale's decision", and
the `MEMORY.md` index line with it. Leaving them as-is is the second-best option; deleting
them is the worst.

### 4.3 Historical specs and handoffs (preserve as history)

Dedicated cass records:

- `thoughts/shared/handoffs/20260817-cass-viability-assessment/` — 6 files: `agent-log.md` (11),
  `lanes/live-probe.md` (54), `lanes/resource-forensics.md` (35), `lanes/architecture-audit.md` (18),
  `lanes/alternatives.md` (18), `lanes/defect-ledger.md` (15). **Today's assessment — the basis for
  the decision. Preserve.**
- `thoughts/shared/handoffs/20260814-cass-status-check/` — 8 files: `lanes/deployed-binary.md` (74),
  `lanes/hang-repro.md` (71), `lanes/scheduling.md` (46), `lanes/repair-history.md` (46),
  `lanes/cross-machine.md` (38), `agent-log.md` (37), `lanes/index-contents.md` (20),
  `lanes/corpus-coverage.md` (17)
- `thoughts/shared/handoffs/20260810-cass-deploy-coverage-fix.md` (25) and
  `…-cass-deploy-coverage-fix.md.launch-receipt.md` (1)
- `thoughts/shared/handoffs/20260810-br-cutover-verification/` — `lanes/adjudicate.md` (22),
  `raw-test-logs/challenge-cass-full-lib.log` (21), `raw-test-logs/adjudicate-m2-full.log` (21),
  `lanes/challenge-cass.md` (17), `agent-log.md` (9), `lanes/challenge-beads.md` (1)

Specs with substantial cass content:

- `specs/279-disk-janitor-authority-v3/` — ~25 files, `plan.md` (36), `tasks.md` (19),
  `lanes/r6-d-devicesupport-devil.md` (19), `lanes/r6-a-quarantine-refute.md` (18),
  `lanes/r12-verify-c.md` (16), `lanes/r12-c-citation-resolution.md` (13),
  `lanes/r12-a-superseded-statements.md` (13), `artifacts/proto279.sh` (12),
  `lanes/r13-c-residue-map.md` (11), `lanes/r12-verify-a.md` (11), `log.md` (10),
  `lanes/r13-e-rev18-citations.md` (10), `spec.md` (6), `lanes/r13-coordinator.md` (6), + 11 more
- `specs/133-configurable-codex-skills/` — ~15 files, `codex-visible-allowlist-v2.md` (8),
  `code-verify-transcript.md` (8), `probe-receipts/codex.json` (9), `plan.md` (6), `spec.md` (4),
  `codex-visible-allowlist-v3.md` (4 — **but see 2.1, line 87 is live coupling**),
  `log.md` (3), `codex-review.md` (3), `code-verify.md` (3), `implement-receipt.md` (2), + artifacts
- `specs/116-precheck-halt-translator/` — `implement-receipt.md` (44), `tasks.md` (10),
  `spec.md` (7), `code-verify-transcript.md` (6), `plan.md` (5), `codex-review.md` (2),
  `code-verify.md` (1), `artifacts/smoke-test-2026-05-07.md` (5) — **largely the `cassette` false
  positive family**; check per-file before touching
- `specs/074-claude-sessions-wiki/` — `spec.md` (13), `plan.md` (9), `shaping-transcript.md` (6),
  `tasks.md` (5), `code-verify-transcript.md` (3), `log.md` (1), `implement-receipt.md` (1)
  (its `scripts/` are Tier 1F)
- `specs/021-cross-agent-compound-learnings/` (spec.md 9, shaping.md 6, plan.md 3, tasks.md 1),
  `specs/064-skill-budget-audit/` (shaping-transcript 8, tasks 2, spec 2, skill-triage.html 2,
  plan/log/audit-report.tsv/user-confirmed-keep.txt/planning-transcript 1 each),
  `specs/117-halt-cleanup-and-sanity-gate/` (artifacts/boundary-B 6, plan 5, planning-transcript 3,
  cut-notes/old-halt-renderer-assets.yaml 2, tasks/spec/implement-receipt/codex-review 1 each),
  `specs/150-disk-janitor-launchd-sweep/`, `specs/154-janitor-sim-worktree-prune/`,
  `specs/174-claude-context-slimming/`, `specs/177-skill-hygiene-charbudget/`,
  `specs/179-skill-description-budget-diet/`, `specs/186-design-skill-grounding/`,
  `specs/198-os-probe-evidence-review/`, `specs/211-janitor-application-support-off-limits/`,
  `specs/214-web-structure-skills/`, `specs/260-closing-read-recommendation/`,
  `specs/134/135/136/003/013/048/051/073/077/140`
- `docs/goals/configurable-codex-skill-surface/state.yaml` — lines 92, 234, 272, 306, 345, 395, 402
- `thoughts/shared/audits/agent-flywheel-comparison-2026-05-08.md` (8) + `.html` (3),
  `workflow-bloat-2026-05-08.md` (4) + `.html` (2)
- ~35 further handoff directories with 1-17 hits each (`20260812-brain-foundation/lanes/session-mining.md`
  17, `20260809-buzz-fresh-look/lanes/prior-look.md` 16, `20260804-mini-reliability/*`,
  `20260805-mini-remediation/*`, `20260806-skill-budget-recovery/*`, `20260807-skill-dedup-audit/*`,
  `20260809-br-schema-cutover/*`, `20260810-wake-source-standard/*`, `20260812-*`, `20260814-*`,
  `20260816-*`, `20260817-*`)

### 4.4 A generated tool that embeds the whole cass skill

`tools/codex-skill-review/index.html` — **git-tracked**, `188` cass occurrences on one
physical line (file-level line count of `1` is misleading). It carries `"id": "198-cass"`,
`"name": "cass"`, `exclude:cass` twice, and the full `cm` description
(`"summary": "QUARANTINED: CASS-backed memory system…"`). It is a generated snapshot of the
skill surface, so regenerating it after the allowlist edits removes the content without a
hand edit.

### 4.5 Mini SSD mirror docs (mini lane owns the hardware)

- `configs/mini-host/README.md:179` — *"`SSD-2/SSD-1-mirror/cass-mirror/agent_search.db` (6.4 GB)
  has no counterpart on …"*
- `configs/mini-host/bin/ssd-mirror.sh:5` — *"SSD-1 (/Volumes/SSD-1) = live working set: dev,
  models, cass-mirror,"*
- Mirrored copies at `thoughts/shared/handoffs/20260804-mini-reliability/mini-host-snapshot/bin/gj`
  (25), `.../bin/ssd-mirror.sh` (1), `.../bin/gj-config.env` (1),
  `thoughts/shared/handoffs/20260805-mini-remediation/ssd-mirror.sh.{fixed,before}` (1 each)

---

## 5. TIER 4 — INCIDENTAL (no action, listed for completeness)

| path | what |
|---|---|
| `~/.claude.json` → `skillUsage.cass` | `{"usageCount": 7, "lastUsedAt": 1784719338263}` — telemetry counter |
| `~/.claude.json:905-907, 1422, 5849` | project-path entries for `coding_agent_session_search` (incl. a Dropbox path and `carmandale/coding_agent_session_search`) |
| `~/.codex/config.toml:2557` | `[projects."/Users/dalecarman/dev/coding_agent_session_search"]` / `trust_level = "trusted"` |
| `~/.claude/telemetry/1p_failed_events.*.json` (7 files) | queued event payloads |
| `~/.claude/projects/**/*.jsonl` | session transcripts — thousands of files, pure history, not enumerated |
| `skills/domain/compound/dhh-rails-style/references/{gems,testing}.md:184,197` | `VCR.use_cassette` — false positive |

---

## 6. Beads in `.agent-config/.beads/issues.jsonl`

722 beads total; **11 match** on `(?<![a-z])cass(?![a-z])|cass[-_]|coding.agent.search|coding_agent_session_search`.

### Open / in_progress (6)

| id | status | pri | title | cass relevance |
|---|---|---|---|---|
| `.agent-config-mpsd` | **open** | 1 | `cass-*-target cargo caches are a TMPDIR family spec 279 does not reach (88 GB live)` | **directly about the retirement's disk residue.** Records Dale's own 2026-08-14 question and its answer: the `/private/tmp/cass-*-target` dirs are *compiler output* (`debug/`, `release/`, `tmp/`, `.rustc_info.json`, `CACHEDIR.TAG`) — "No test data and no live data." 10 directories, 135.7 GiB apparent / 117.6 GiB `du`. **Retiring cass largely resolves this bead; recommend closing with the deletion as the reason, or re-scoping to the general janitor gap.** |
| `.agent-config-u2y6` | in_progress | 1 | `disk-janitor authority v3: Trash (24h age guard), cass fixture family, stale DeviceSupport` | spec 279. **One of three scopes is the cass fixture family**; carries Dale's verbatim 24h Trash decision and the 2026-07-27 media-loss hard constraint. **Do not close** — re-scope the cass family only. |
| `.agent-config-bn34` | in_progress | 1 | `disk-janitor: unknown-hog tripwire is per-entry, so 43.5GiB of sub-threshold TMPDIR leak is invisible` | cass is the *example* leak (16 × 2.72GiB fixture dirs); the defect is general. **Keep open.** |
| `.agent-config-io58` | open | 2 | `disk-janitor: --report-only cannot preview the three new floor-recovery delete families` | one of the three families is "leaked cass fixtures (TMPDIR)". **Keep open**, re-scope. |
| `.agent-config-14bq` | in_progress | 1 | `Repair and manage mini SSD backup` | matches only via `cass-mirror` on SSD-1. **Keep open**; the mirror path may need updating once the mini's cass-mirror is retired. |
| `.agent-config-beads-agreement-check-8fyh` | open | 1 | `Weekly check that each repo's beads database and JSONL still agree` | cites `coding_agent_session_search` as the incident that motivated it ("128 issues, ten of them live engineering work, were invisible to `br ready`"). Historical cite. **Keep open, do not edit.** |

### Closed (5) — historical, no action

`.agent-config-238` (skill budget audit — "Audit every skill against CASS session history"),
`.agent-config-34m` (skill discovery bloat — names cass among 15 jsm duplicates),
`.agent-config-pyw` (centralize AGENTS.md — "Moved common guidance (beads, cass, agent-mail, …)"),
`.agent-config-xw9` (generalize compound-learnings — "Input should use CASS/CM"),
`.agent-config-tldraw-offline-collision-oxig` (matched incidentally).

**No bead in agent-config is *about* cass as a product.** `.agent-config-mpsd` is the only one
the retirement should discharge. The live cass defect beads named in the memory file
(`jy8v8`, `k69vx`, `1a7mk`, `pfar8`) live in the `coding_agent_session_search` tracker, not
here — another lane's scope.

Repo state at sweep time: `git status --short` **clean**; no dirty cass paths; `.beads` clean.

---

## 7. Recommended action per Tier-1 item (coordinator's call, not executed)

| # | artifact | action |
|---|---|---|
| 1A | 4 × `*/skills/cass/` | `jsm uninstall cass` **first** (so jsm stops believing it is installed), then verify all four directories are gone; `/usr/bin/trash` any survivor |
| 1B | `claude-curation.txt:66` | flip `visible` → `hide` with a retirement reason (row retention is the ledger — do not delete the row), then `/usr/bin/python3 scripts/skill-curation-apply.py` |
| 1B | `codex-visible.txt:44-45, :200` | delete the two comment lines and `exclude:cass` **in the same commit as** `codex-visible-allowlist-v3.md:87` and the `tests/test-agent-skill.sh` fixture swap (see 2.1, 2.2) |
| 1C | `testflight/SKILL.md:385` | delete the recipe line |
| 1C | `gj-tool/SKILL.md:59` | delete the line; file a bead against `gj-tool` for the `gj sessions` subcommand itself |
| 1C | `compound-learnings/SKILL.md:24` | rewrite to unconditional: cass is retired, not quarantined-pending-repair |
| 1C | `skills/_quarantine/tools/cm/SKILL.md.quarantined` | delete the skill outright (its only reason to exist was cass) |
| 1D | `commands/debug-plus.md:69,99` + all deployed wrappers | remove `query_cass=true`; regenerate the Claude/Gemini/Codex wrappers |
| 1D | `~/.gemini/commands/skills/cm.toml` | delete — orphaned wrapper for a quarantined skill |
| 1D | `~/.gemini/commands/skills/agent-ergonomics-…-cli-tools.toml` | delete — orphaned wrapper for a skill that exists nowhere |
| 1E | `rust-unsafe-code-exorcist`, `rch` (×4 runtimes) | **needs Dale's decision.** Vendor jsm content; `jsm uninstall` removes a skill he uses. Options: (a) leave and accept that the cass instructions fail loudly once the binary is gone, (b) fork-patch the cass sections and accept jsm-update drift, (c) uninstall. Recommend (a) plus a note, since `SKILL-FALLBACKS.md:154` says no phase depends on cass output. |
| 1F | `~/Projects/sessions-wiki/CLAUDE.md` | rewrite the `## Compile Pipeline` section and drop `cass_session_ids` from the schema; decide whether `sessions-wiki` stays in `configs/wiki-registry.json` at all |
| 1F | `specs/074-claude-sessions-wiki/scripts/*.py` | leave as spec history, or delete `cass_client.py` and mark the spec superseded — but note this is the **only** copy of the pipeline |
| 1G | `configs/shell/zshrc.local.mini:30-32` | delete the three lines; coordinate with the mini lane so the deployed `~/.zshrc.local` on mini is updated |
| 1H | `disk-janitor.sh:457` + its test + doc | **do not remove until `~/.local/share/cass` is actually gone.** The entry is protective. |
| 1I | `~/.codex/worktrees/{9218,8096}` | delete both (`209M` combined) — stale clones carrying a live cass skill, a cass hook entry, and a live sessions-compile command. Confirm no live Codex session owns them first. |
| 1J | `.agent-state/{spec257-r11-*,r13-mutants,spec224-review-clone}` | delete — gitignored session-local clones, 1.3G |
| 4.2 | `~/.claude/projects/…/memory/cass-viability-verdict.md` + `MEMORY.md` | **update, do not delete** — this is the anti-resurrection guard |

---

## 8. Commands run (audit trail)

```
git log --oneline -10; git status --short
rg -i -c 'cass' --stats                                     # 1821 / 1380 / 260 / 8998
rg -i -o '\b[a-z_-]*cass[a-z_-]*\b' --no-filename | sort | uniq -c | sort -rn
rg -i -c 'cass' | sort -t: -k2 -rn                          # full file list
rg -i -n 'cass' <each live surface>                          # exact lines
sed -n '<ranges>' tests/test-agent-skill.sh                   # fixture contexts
python3 scripts/agent-skill validate codex --enforce-v3       # ok:True, cass in excluded_names
python3 - <<parse .beads/issues.jsonl>                        # 722 beads, 11 matches
jsm list                                                      # cass v7, installed 2026-05-09
launchctl list | rg -i 'cass|jsm'; crontab -l                 # both clean
rg -i -l --no-ignore --hidden '\bcass\b' ~/.codex/worktrees/  # 222 (0 without --no-ignore)
rg -i -o 'cass' tools/codex-skill-review/index.html | wc -l   # 188
fd -H -t d -i '^cass$|^cm$|^rch$' ~/.{claude,codex,gemini,cursor}/skills
```

---

## 9. Uncertainty and out-of-scope leads

- **The 3 AM mini compile.** `~/Projects/sessions-wiki/CLAUDE.md` claims a nightly cass-fed
  compile on the Mac mini. This lane cannot see the mini's launchd. Unverified from here.
- **`gj sessions`.** `skills/tools/gj-tool/SKILL.md:59` documents a `gj` subcommand that
  launches the cass TUI. The `gj` implementation lives in the gj-tool repo — a separate owner.
- **`~/.local/bin/cass` and its 8 rollback copies, `~/.local/share/cass/watchdog.sh`,
  `~/.cass-catchup/`, `~/backups/cass/`, the 77G production data dir, `/tmp/cass-*`** — named in
  the brief, owned by other lanes. This lane only confirms that
  `scripts/disk-janitor.sh:457` currently *protects* `~/.local/share/cass` from deletion.
- **The `rust-unsafe-code-exorcist` / `rch` decision is genuinely Dale's.** Both are vendor
  skills he uses for other purposes; both instruct cass. I have not recommended a unilateral
  uninstall.
