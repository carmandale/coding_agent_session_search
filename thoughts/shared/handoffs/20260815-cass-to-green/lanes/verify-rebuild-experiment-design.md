# Lane: verify-rebuild-experiment-design — adversarial verification of `rebuild-safety`

Role: ADVERSARIAL VERIFIER. Lens: experiment-design (plus control-flow re-trace and an
independent destructive-statement census, per the lane brief).
Subject: `thoughts/shared/handoffs/20260815-cass-to-green/lanes/rebuild-safety.md`
Worktree: `/Users/dalecarman/dev/coding_agent_session_search/.claude/worktrees/cass-to-green-c6bfb589` @ `74a72233`
Read-only. Live DB opened `?mode=ro` only. No `cass index`, no `cass sources`, no git mutation.

---

## 0. Bottom line

**refuted = true**, and I want to be exact about what that does and does not mean, because on a
data-loss question a sloppy verdict in either direction is expensive.

I could **not** refute the lane's narrow technical claim. I re-ran its census independently, found
**two destruction mechanisms its method was structurally blind to**, chased both, and both are
unreachable in production. I then measured one thing the lane never measured that closes the
remaining route. On source reading, `cass index --force-rebuild` still has no way to delete a
conversation row.

What I am refuting is three things the effort would act on:

1. **The headline is unconditional and the code is not.** Six gate preconditions plus a
   "populated archive" precondition sit between `--force-rebuild` and the read-only fast path.
   The lane's finding 2 carries the condition; the headline strips it, and the headline is what
   gets quoted into the bead and the skill.
2. **The designed falsifier cannot deliver the proof it advertises.** Arms B and C have no
   working positive control, the three arms are not independent, and the fixture as written does
   not match the record shape the repo's own connector tests use.
3. **The single destructive command in the design is guarded by a check that cannot see the value
   that command uses — and the failure mode when that value is empty is the live archive.**

Nothing here says a rebuild deletes rows. I looked harder than the lane did and found no evidence
it does. What I am saying is that "proven safe by reading" is the honest label, and the experiment
that was supposed to upgrade that label would not have.

---

## 1. Independent re-trace: control flow from the entry point

### 1.1 The read-only fast path — CONFIRMED, with six preconditions the headline drops

`src/indexer/mod.rs:2091-2102`, verbatim:

```rust
fn should_try_readonly_canonical_force_rebuild(opts: &IndexOptions) -> bool {
    opts.force_rebuild
        && !opts.full
        && !opts.watch
        && !opts.semantic
        && !opts.build_hnsw
        && opts
            .watch_once_paths
            .as_ref()
            .is_none_or(|paths| paths.is_empty())
        && opts.db_path.exists()
}
```

And a seventh, inside the body (`src/indexer/mod.rs:2117-2126`): `total_conversations == 0`
closes the read-only handle and `return Ok(false)` — falls through to the writable path.

Body confirmed at `src/indexer/mod.rs:2112` (`FrankenStorage::open_readonly(&opts.db_path)`) and
the strategy string at `src/indexer/mod.rs:2142-2147`
(`force_rebuild_uses_readonly_authoritative_canonical_db_rebuild_only`). The lane's citations are
accurate.

**What the headline omits:** `cass index --force-rebuild --semantic`,
`--force-rebuild --build-hnsw`, `--force-rebuild --watch`, `--force-rebuild` with explicit
`--watch-once` paths, and `--force-rebuild` against an empty archive all take the **writable**
branch. The lane's finding 2 opens with "With a populated archive" — correct. The headline says
"`cass index --force-rebuild` cannot delete a conversation row — it opens SQLite read-only",
full stop. That sentence is what lands in the bead.

### 1.2 The fall-through path fails closed — CONFIRMED

`open_storage_for_index`, `src/indexer/mod.rs:14540-14602`. Read in full. Three `db_path.exists()`
blocks, and every existing-DB branch either returns an already-open storage or returns an error:

- `src/indexer/mod.rs:14558` and `:14590` — `"canonical db is busy/locked during index open; refusing to replace it"`
- `src/indexer/mod.rs:14547` and `:14566` and `:14593` — `canonical_archive_unhealthy_for_index_error(...)`
- `src/indexer/mod.rs:14598-14601` — the `else` (file absent) branch is the only one that can
  return `opened_fresh_for_full = true`.

Also found, which the lane did not cite and which strengthens its case
(`src/indexer/mod.rs:12380-12386`): a path that *would* have replaced canonical SQLite now bails —
`"historical salvage restart would require replacing canonical SQLite"`.

### 1.3 `--full --force-rebuild` is nearly the same experiment as bare `--force-rebuild`

`src/indexer/mod.rs:12390-12391`:

```rust
let canonical_only_full_rebuild =
    opts.force_rebuild && initial_canonical_sessions_before_salvage > 0;
```

So on a populated archive `--full --force-rebuild` **skips the filesystem rescan** and goes to
rebuild-from-DB. The lane cites this itself and then calls Arm B a "different code path". It is a
different *branch*, but for the question being asked — does an absent source file cause row
removal? — Arm B never lets the scanner see the absent file. Only Arm C (plain `--full`,
`force_rebuild = false`, so `canonical_only_full_rebuild = false`) does. See §3.5.

### 1.4 CASS #164 comment — CONFIRMED verbatim at `src/indexer/mod.rs:12690-12700`

(lane said 12691-12699; text matches exactly)

```
// NOTE: We deliberately do NOT reset either SQLite or Tantivy here.
// ... Eagerly deleting or replacing anything is both redundant and
// dangerous if the scan or rebuild OOMs / hits a constraint error. (CASS #164)
```

### 1.5 The rebuild itself opens read-only — CONFIRMED at `src/indexer/mod.rs:18134`

```rust
let storage = FrankenStorage::open_readonly(db_path).with_context(|| {
    format!("opening database for Tantivy rebuild: {}", db_path.display())
})?;
```

---

## 2. Independent destructive-statement census — two mechanisms the lane's method could not see

The lane's census was `rg 'DELETE\s+FROM\s+conversations\b'`. That grep answers a narrower
question than "can a conversation row disappear". I reproduced it, then widened.

### 2.1 The narrow census reproduces exactly

```
$ rg -n -i -U 'DELETE\s+FROM\s+conversations\b' src/
src/indexer/mod.rs:20738:         DELETE FROM conversations;
src/storage/sqlite.rs:7174:            "DELETE FROM conversations WHERE agent_id = ?1",
src/storage/sqlite.rs:20499:            .execute_compat("DELETE FROM conversations WHERE id IN (2, 4)", fparams![])
src/storage/sqlite.rs:20601:                "DELETE FROM conversations WHERE id IN (3, 5, 7, 8)",
```

4 sites, 3 test-guarded, 1 reachable (the agent purge). CONFIRMED.

### 2.2 MISS #1 — `DROP TABLE conversations` in a migration

`src/storage/sqlite.rs:3328`, inside `MIGRATION_V5`. A `DELETE FROM` grep cannot see it.

Guard verified — `src/storage/sqlite.rs:3298-3299`:

```
#[cfg(test)]
const MIGRATION_V5: &str = r"
```

Test-only. And a design note at `src/storage/sqlite.rs:4662-4667` explains the production path
uses a single combined migration (v13) that *avoids* V5's `DROP TABLE conversations` because
frankensqlite mishandles autoindex cleanup during it.

**The empirical fact that closes this, which the lane never measured:**

```
$ sqlite3 "file:.../agent_search.db?mode=ro" "SELECT key,value FROM meta WHERE key LIKE '%schema%' OR key LIKE '%version%';"
schema_version|20

$ rg -n 'CURRENT_SCHEMA_VERSION\s*[:=]' src/storage/sqlite.rs
3003:pub const CURRENT_SCHEMA_VERSION: i64 = 20;
```

Live archive is already at the current schema version. No migration runs on open. This is a
measured fact about the archive at risk, not an inference about the tree.

### 2.3 MISS #2 — file-level destruction of the entire database

`src/storage/sqlite.rs:1645`:

```rust
pub(crate) fn remove_database_files(path: &Path) -> std::io::Result<()> {
```

It `fs::remove_file`s the main DB and its `-wal` / `-shm` sidecars. **This destroys all 14,104
conversations with zero SQL statements.** Invisible to any `DELETE FROM` census.

Two production call sites, `src/storage/sqlite.rs:4631` and `:4640`, both inside
`FrankenStorage::open_or_rebuild` (`src/storage/sqlite.rs:4617`):

```rust
Ok(SchemaCheck::NeedsRebuild(reason)) => {
    let backup_path = create_backup(path)?;
    cleanup_old_backups(path, MAX_BACKUPS)?;
    remove_database_files(path)?;
    return Err(MigrationError::RebuildRequired { reason, backup_path });
}
```

`MAX_BACKUPS = 3` (`src/storage/sqlite.rs:1124`).

**Reachability — this is what saves the conclusion:**

```
$ rg -n 'open_or_rebuild' src/
src/storage/sqlite.rs:4617:    pub fn open_or_rebuild(...)          <- definition
src/storage/sqlite.rs:16388,16399,16405,16410                        <- #[cfg(test)] tests
```

**Zero production callers in `src/`** (`src/` includes `src/bin/`). Every other caller is under
`tests/`. `SqliteStorage` is an alias for `FrankenStorage` (`src/storage/sqlite.rs:3732`), so the
`SqliteStorage::open_or_rebuild` calls in `tests/` are the same function; they are still tests.

So the mechanism exists, is `pub`, and is not on any `cass` command path today. The lane's
conclusion survives. **Its method did not** — a one-string grep would have reported "safe" whether
or not this had a live caller.

### 2.4 MISS #3 — `TantivyIndex::delete_all`

`src/search/tantivy.rs:1292`. Callers:

```
$ rg -n 'delete_all\(' src/
src/indexer/mod.rs:12698:            // Same rationale — skip eager delete_all(); ...
src/search/tantivy.rs:1292:    pub fn delete_all(&mut self) -> Result<()> {
src/search/tantivy.rs:1293:        self.inner.delete_all()...
```

No production caller — the only mention outside the definition is a comment saying the eager wipe
was deliberately removed. Relevant to the lane's finding 8 (search-reachability): the live lexical
index is not eagerly emptied before a rebuild.

### 2.5 Widened census: every `DELETE FROM` target in `src/`

```
11 daily_stats   8 meta   5 message_metrics   4 token_usage   4 messages
 4 conversations 3 usage_models_daily 3 usage_hourly 3 usage_daily
 3 token_daily_stats 3 snippets 3 conversation_tail_state
 3 conversation_external_tail_lookup 3 agents 3 _schema_migrations
 2 sources 2 conversation_tags 1 workspaces 1 tags 1 sqlite_master
 1 idempotency_keys 1 conversation_external_lookup 1 bookmarks
```

Checked the two that could reach `conversations` indirectly:

- **`DELETE FROM agents` → cascade?** No. Live schema, read from `sqlite_master`:
  `agent_id INTEGER NOT NULL REFERENCES agents (id)` — **no `ON DELETE CASCADE`**. Every
  `ON DELETE CASCADE` in the schema points the other way (messages/tags/snippets → conversations),
  `src/storage/sqlite.rs:4850,4863,4877,4915,4992,5245,5330`.
- **`DELETE FROM sources`** — `delete_source`, `src/storage/sqlite.rs:9096-9107`. Deletes only the
  `sources` row and refuses `LOCAL_SOURCE_ID`. `conversations.source_id REFERENCES sources(id)`
  with no cascade, so this cannot remove conversations.
- **`sync_sources_config_to_db`** (`src/indexer/mod.rs:21612-21671`) is **upsert-only** — no
  delete anywhere in it. (It also `return`s immediately under `CASS_IGNORE_SOURCES_CONFIG`; see
  §3.7.)

### 2.6 Dynamic SQL

```
$ rg -n -i 'DELETE FROM \{|DROP TABLE \{|format!\("DELETE|format!\("DROP' src/
src/storage/sqlite.rs:25289:  conn.execute(&format!("DROP TABLE IF EXISTS {table}"))
```

One site, and `src/storage/sqlite.rs:25390-25391` asserts the repair batch never drops tables.

### 2.7 No source-presence-driven removal anywhere

```
$ rg -n -i 'delete_conversation|remove_conversation|delete_by_source|orphan.*conversation' src/storage/sqlite.rs src/indexer/mod.rs
```
Only orphan-**message** cleanup (messages whose parent conversation is gone). Never the reverse.
`src/indexer/mod.rs:21604-21606` confirmed verbatim: *"Treat remove events as noise until
delete-aware rebuilds exist."*

---

## 3. EXPERIMENT DESIGN — attacking the falsifier (my assigned lens)

Nine defects. E1 through E3 are disqualifying as written; E4 through E6 make the run
uninformative; E7 is how a correct result gets misapplied.

### 3.1 E1 (CRITICAL, data-loss) — the guard on the destructive command cannot see the value that command uses

§6.0 defines the guard:

```bash
case "$CASS_DATA_DIR" in /tmp/cass-falsifier-*) ;; *) echo "REFUSING..."; exit 2 ;; esac
```

§6.8 runs the only destructive command in the design:

```bash
CASS_DATA_DIR="$T/data-control" cass sources agents exclude claude_code
```

That is an **inline assignment prefix on an external command**. It does not change the exported
`CASS_DATA_DIR`. So the guard inspects `$T/data` — the arms' directory — passes, and the command
proceeds using a *different value the guard never examined*. The lane's own instruction ("re-assert
the case guard immediately before it") therefore produces a false sense of protection.

Why this is the disqualifying one rather than a nitpick — the fallback chain,
`src/lib.rs:80275-80292`:

```rust
pub fn default_data_dir() -> PathBuf {
    if let Ok(dir) = dotenvy::var("CASS_DATA_DIR") { ... return PathBuf::from(trimmed); }
    if let Ok(dir) = dotenvy::var("XDG_DATA_HOME") { ... return PathBuf::from(trimmed).join("coding-agent-search"); }
    directories::ProjectDirs::from("com", "coding-agent-search", "coding-agent-search")
        .map(|p| p.data_dir().to_path_buf())
    ...
}
```

If `CASS_DATA_DIR` is empty, unset, mistyped, or lost to a fresh shell, resolution falls to
`XDG_DATA_HOME/coding-agent-search` and then to `ProjectDirs` — which **is** the live archive at
`~/Library/Application Support/com.coding-agent-search.coding-agent-search`. And
`cass sources agents exclude` has **no `--data-dir` flag** to pin it with:

```
$ cass sources agents exclude --help
Options:
      --keep-indexed-data
      --robot-format <ROBOT_FORMAT>
  -h, --help
```

`--data-dir` is not global either — `cass --help` has no data-dir line, and `src/lib.rs:88636`
says *"Override with `CASS_DATA_DIR` or command-specific `--data-dir` flags elsewhere."*
So for this one command, the env var is the **only** steering wheel, and the guard is on the wrong
variable.

**Fix — guard by row count, not by path.** A count cannot be fooled by a typo:

```bash
CTRL="$T/data-control"
case "$CTRL" in /tmp/cass-falsifier-*/data-control) ;; *) echo REFUSE; return 2 ;; esac
n=$(sqlite3 "file:$CTRL/agent_search.db?mode=ro" "SELECT COUNT(*) FROM conversations;") || return 2
[ "$n" = "2" ] || { echo "REFUSE: target holds $n conversations, expected 2"; return 2; }
CASS_DATA_DIR="$CTRL" cass sources agents exclude claude_code
```

The live archive holds 14,104. A `!= 2` check refuses it unconditionally.

### 3.2 E2 (CRITICAL) — arms B and C have no positive control; a no-op reads as a pass

P2(a) greps stderr for
`force_rebuild_uses_readonly_authoritative_canonical_db_rebuild_only|selected_lexical_population_strategy`,
and the lane's stated positive control for it is "run Arm A once on a fixture whose source is
still present." **That control exercises Arm A's path only.** Enumerating every
`record_lexical_population_strategy` call site:

| site | reason string | which arm reaches it |
|---|---|---|
| `src/indexer/mod.rs:2142` | `force_rebuild_uses_readonly_authoritative_canonical_db_rebuild_only` | **Arm A only** (read-only fast path) |
| `src/indexer/mod.rs:12641` | `resume_incomplete_authoritative_db_rebuild_from_checkpoint` | only on a resumed incomplete rebuild |
| `src/indexer/mod.rs:12864` | `full_rebuild_uses_authoritative_canonical_db_rebuild_only` | **Arm B** — a *different string* from the one P2(a) names first |
| `src/indexer/mod.rs:12922` | `repair_plan.reason` | only when a lexical repair is planned |

Arm C is plain `--full` with `force_rebuild = false`, so `canonical_only_full_rebuild` is false
(`src/indexer/mod.rs:12390-12391`) and the `:12864` branch is not taken. **Arm C can legitimately
emit no strategy line at all.** P2(a) then cannot distinguish "Arm C rescanned and preserved
everything" from "Arm C errored, short-circuited, or did nothing". That is exactly the
control-exercises-a-different-code-path failure the lane brief warns about, committed inside the
control the lane wrote to prevent it.

**Fix:** derive a distinct expected marker per arm from the table above, and run the
source-still-present positive control **once per arm**, not once for Arm A.

### 3.3 E3 (CRITICAL) — P2(b) is unrunnable as written

P2(b) is prose only: *"capture the index dir's segment file set + mtimes before and after each
arm; require it to change."* No code is given, and none of the arm scripts in §6.5–6.7 takes a
before-snapshot. Run literally, the design produces a P2(b) that can never fire — the
after-snapshot has nothing to compare against. A control that cannot produce a positive is
indistinguishable from a dead one.

### 3.4 E4 — the three arms are not independent

A, B, C run sequentially against the single archive at `$T/data`. Consequences:

- If Arm A drops the row, Arms B and C are untestable — the row is already gone, and every
  subsequent reading is `1|1|0` regardless of what B and C would have done.
- If any arm re-indexes `keep.jsonl`, tail/merge state moves under the later arms.
- Arm C, the only arm that performs a real filesystem rescan (§1.3), runs **third**, on an archive
  two rebuilds removed from the state §6.4 established.

**Fix:** `cp -a "$T/data" "$T/data-armA"` … `-armB` … `-armC` immediately after §6.4, and run each
arm against its own copy. Costs kilobytes.

### 3.5 E5 — Arm B is nearly a duplicate of Arm A; the arm that matters is the least controlled

The lane justifies Arm B as a "different code path", citing the `!opts.full` gate. True at the
branch level. But `src/indexer/mod.rs:12390-12391` — which the lane cites itself — means Arm B on a
populated archive **skips the filesystem rescan** and rebuilds from the DB, same as Arm A. Neither
arm ever lets the scanner observe that `vanish.jsonl` is absent. Only Arm C does. So the design
spends two of its three arms on the same behavior and gives the decisive arm the weakest position
(last) and no working positive control (E2).

### 3.6 E6 — the fixture does not match the connector's own test-fixture record shape

The repo's own e2e fixture writer, `tests/e2e_cli_flows.rs:55-56`:

```
{"type": "user", "timestamp": "2024-12-01T10:00:00Z", "message": {"role": "user", "content": "..."}}
{"type": "assistant", "timestamp": "2024-12-01T10:01:00Z", "message": {"role": "assistant", "content": "..."}}
```

The lane's §6.1 fixture:

```
{"role":"user","content":"FALSIFIERMARKERKEEP alpha"}
```

No `type`, no `timestamp`, and `role`/`content` at the top level rather than nested under
`message`. Whether the claude_code connector tolerates the flat shape is **UNVERIFIED** — I could
not run an index to find out, and that is precisely the constraint this lane operates under. But
the repo never writes the lane's shape anywhere in `tests/`, and no timestamp means no
`started_at`.

Failure here is *safe* — §6.3's `REQUIRE exactly 2|1|1` stops the run — so this is a runnability
defect, not a false-safety one. The hazard is second-order and worth naming anyway: an operator
whose fixture indexes 0 rows is one impatient step from pointing the experiment at real data.

**Fix:** copy the record shape from `tests/e2e_cli_flows.rs:50-59` rather than inventing one.

### 3.7 E7 — scope transfer: a pass on this fixture does not license a rebuild on the live archive

The fixture and the subject differ on every axis that could matter:

| | fixture | live archive |
|---|---|---|
| rows | 2 | **14,104** (measured now; 13,371 when the lane measured — the backfill is running) |
| sources config | **disabled** (`CASS_IGNORE_SOURCES_CONFIG=1`) | active |
| `sync_sources_config_to_db` | **skipped** — `src/indexer/mod.rs:21613-21615` returns early | runs |
| writer contention | none | a live backfill is writing right now |
| who wrote the rows | the binary under test | older binaries, over months |

`CASS_IGNORE_SOURCES_CONFIG` is load-bearing for keeping the experiment small — the lane is right
about that — but it also switches off a function that writes to the DB on every real run. The
falsifier therefore does not exercise the operator's actual code path, only a subset of it.

The lane's *recommendation* says plainly "Do NOT run any rebuild against the live archive" — good,
and I endorse it. But its *headline* says the rebuild is safe, and a headline outlives the caveat
under it. Whoever reads the bead next will read the headline.

### 3.8 E8 (minor) — `exit 2` in a design meant to be pasted

Every guard uses `exit 2`. Pasted into an interactive zsh session, that kills the operator's shell
rather than aborting a step. Use `return 2` inside a function, or run the whole design as a script.

### 3.9 E9 (minor) — `.env` participates in resolution

`default_data_dir` uses `dotenvy::var`, not `std::env::var` (`src/lib.rs:80276,80282`). A `.env`
in the working directory takes part in the lookup. Exported vars still win in dotenvy's default
mode, so this is low risk — but run the experiment from a directory with no `.env`, and say so.

---

## 4. Suspicions I raised and then REFUTED by measurement (honest nulls)

Recording these because a verifier who only reports hits is not measuring, and each of these was
a plausible-looking defect that turned out not to be real.

### R1 — REFUTED: `DB="..." probe` does not leak the assignment

I predicted §6.8's `DB="$T/data-control/agent_search.db" probe` would permanently rebind `$DB`
(POSIX says assignment prefixes on *functions* persist), silently pointing every later probe at
the emptied control DB and manufacturing a false "DROPPED" reading. Measured:

```
$ zsh -c 'DB=original; probe() { print "probe sees DB=$DB"; }; DB=control probe; print "AFTER: DB=$DB"'
probe sees DB=control
AFTER: DB=original

$ bash -c 'DB=original; probe() { echo "probe sees DB=$DB"; }; DB=control probe; echo "AFTER: DB=$DB"'
probe sees DB=control
AFTER: DB=original
```

No leak in either shell. The design is fine here.

### R2 — REFUTED: the installed binary is not source-drifted from the censused tree

The design tests `/Users/dalecarman/.local/bin/cass` (`cass 0.6.9`, `git commit: 447d97fe`) while
the census reads the worktree at `74a72233`. I expected a differential-specimen problem — a result
attributed to code that was never the code that produced it. Measured:

```
$ git diff --name-only 447d97fe..74a72233
.beads/issues.jsonl
.beads/last-touched
thoughts/shared/handoffs/20260814-cass-repair-to-green/backfill-continuation-prompt.md
thoughts/shared/handoffs/20260814-cass-repair-to-green/backfill-continuation-prompt.md.launch-receipt.md
thoughts/shared/handoffs/20260814-cass-repair-to-green/lanes/backfill-falsifier.md

$ git diff --stat 447d97fe..74a72233 -- src/ tests/ Cargo.toml Cargo.lock
(empty)
```

Zero source or test changes across the gap. The censused source **is** the binary's source. The
lane's version note is fine and my objection was wrong.

### R3 — REFUTED: `CASS_DATA_DIR` does take precedence over `XDG_DATA_HOME`

I suspected P1 might hit `$T/data` (the arms' archive) instead of `$T/data-control`, because it
sets `CASS_DATA_DIR` inline while `XDG_DATA_HOME` stays exported at `$T/data`. `src/lib.rs:80275-80292`
checks `CASS_DATA_DIR` **first** and returns it verbatim. P1 targets the control copy correctly.

(E1 still stands and is a different objection: the *guard* cannot see that value, and the
fall-through when it is absent is the live archive.)

---

## 5. Findings-by-finding adjudication of the lane

| # | lane claim | my verdict | note |
|---|---|---|---|
| 1 | 4 `DELETE FROM conversations`, 3 test-only, 1 reachable | **CONFIRMED, method insufficient** | reproduced exactly; but the grep is blind to §2.2/§2.3, both of which I had to chase separately |
| 2 | force-rebuild opens SQLite read-only | **CONFIRMED, headline over-claims** | gate has 6 conditions + a populated-archive condition (§1.1) |
| 3 | `--full` refuses to reset/replace; open errors rather than replaces | **CONFIRMED** | read `open_storage_for_index` 14540-14602 in full; plus the extra bail at 12380-12386 |
| 4 | exclude deletes with no source predicate, no confirmation | **CONFIRMED, and worse** | `src/storage/sqlite.rs:7121-7190`; shipped help says only *"Exclude an agent/connector from future indexing runs"* — it never mentions deletion, and deletion is the default |
| 5 | an existing test pins that exclude deletes a source-absent row | **CONFIRMED by reading** | `tests/e2e_sources.rs:321`, seed at `:46`; lane's own UNVERIFIED note on its pass state is correct and I did not run it either |
| 6 | no test pins rebuild preserving source-absent rows | **NOT INDEPENDENTLY RE-RUN** | plausible and consistent with §2.7; I spent my budget on the destructive census and the design instead |
| 7 | live counts 13,371 / 9 agents | **CONFIRMED then SUPERSEDED** | now `14104\|9`; `claude_code` still exactly 4,050, `codex` 3,712 → 4,441 (backfill running) |
| 8 | rows could survive SQLite but be unreachable in search | **still UNVERIFIED, but weakened as a hazard** | `delete_all` has no production caller (§2.4) and the publish path stages then atomically exchanges (`publish_staged_lexical_index`, `src/indexer/mod.rs:16254-16336`, `atomic_exchange_paths` at `:16298`, retained backups at `:16283`), so an interrupted rebuild should not leave the live index wiped |

---

## 6. What I could not do

- **Did not run any arm.** Forbidden by the lane constraints (`cass index` writes). So every
  statement about runtime behavior here is source-reading or a shell/`git`/`sqlite3` measurement,
  never an observed rebuild.
- **Did not verify the fixture indexes.** E6 is a shape mismatch against the repo's own test
  fixtures; whether the connector tolerates the flat shape is **UNAVAILABLE** without an index run.
- **Did not re-run finding 6's test census.**
- **Did not build.** No `cargo check` was needed for any claim above.

---

## 7. Recommendation

1. **Rewrite the headline with its preconditions.** "On a populated archive, and only when none of
   `--full`, `--semantic`, `--build-hnsw`, `--watch`, or `--watch-once` is set, `cass index
   --force-rebuild` opens SQLite read-only and rebuilds only Tantivy. No `cass index` form has a
   statement that can delete a conversation row. Established by source reading, not by execution."
2. **Keep the P0 exactly where the lane put it.** `cass sources agents exclude <agent>` is the
   destruction path, its help text does not say so, and it defaults to destroying. That is the
   urgent fix and my census only strengthens it.
3. **Do not run the falsifier as written.** Fix E1 first (count-based guard on the exclude target),
   then E2/E3 (per-arm marker + per-arm positive control, with a real before-snapshot), then E4
   (one `cp -a` per arm), then E6 (fixture shape from `tests/e2e_cli_flows.rs:50-59`).
4. **Add `remove_database_files` to whatever guard or test comes out of this.** It is `pub(crate)`,
   it deletes the whole archive file plus WAL, it has no production caller **today**, and nothing
   in the tree pins that. That is one careless call site away from being the worst bug in the
   product, and it is invisible to the census everyone has been running.
5. **Do not treat a fixture pass as licence to rebuild the live archive** (§3.7).
