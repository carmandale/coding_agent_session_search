# Lane: verify-rebuild-destructive-statements (ADVERSARIAL VERIFIER)

Subject: the rebuild-safety lane's headline — "`cass index --force-rebuild` cannot delete a
conversation row — it opens SQLite read-only and rebuilds only Tantivy."

Mandate: refute it. Default to refuted on uncertainty. Read-only. Repo root
`/Users/dalecarman/dev/coding_agent_session_search/.claude/worktrees/cass-to-green-c6bfb589`
at 74a72233.

**VERDICT: REFUTED — as stated.** The narrow mechanism survives and was strengthened. The
headline's *scope*, its *stated evidentiary basis*, and its *falsifier* each fail independently.

---

## 1. What survived my attack (the lane is right about the mechanism)

I re-derived the census from scratch and extended it to four verb families the lane never
searched. Every extension holds.

### 1a. `DELETE FROM conversations` — census reproduced, 4 sites

```
$ rg -n -i -U 'DELETE\s+FROM\s+conversations' src/
src/indexer/mod.rs:20738:         DELETE FROM conversations;
src/storage/sqlite.rs:7174:            "DELETE FROM conversations WHERE agent_id = ?1",
src/storage/sqlite.rs:20499:  ...  "DELETE FROM conversations WHERE id IN (2, 4)"
src/storage/sqlite.rs:20601:  ...  "DELETE FROM conversations WHERE id IN (3, 5, 7, 8)"
```
Count 4. Matches the lane.

### 1b. EXTENSION — `DROP TABLE conversations`: 5 sites, ALL test-only (lane never searched this)

```
src/ui/data.rs:1682   conn.execute("DROP TABLE conversations")
src/ui/data.rs:1942   conn.execute("DROP TABLE conversations")
src/storage/sqlite.rs:3328   DROP TABLE conversations;      (inside const MIGRATION_V5)
src/storage/sqlite.rs:4662, 4765   — doc comments referring to the V5 drop, not statements
```
- Both `ui/data.rs` sites are inside `#[cfg(test)] mod tests` opening at `src/ui/data.rs:1284`.
- `MIGRATION_V5` is gated: `src/storage/sqlite.rs:3298` is `#[cfg(test)]`, immediately above
  `const MIGRATION_V5` at 3299.

Result: no production `DROP TABLE conversations`. Conclusion holds; the lane's *method* did not
cover it.

### 1c. EXTENSION — FK cascade into conversations: NONE

`src/storage/sqlite.rs:4810-4814` (current schema):
```
CREATE TABLE IF NOT EXISTS conversations (
    agent_id INTEGER NOT NULL REFERENCES agents(id),          -- no ON DELETE CASCADE
    source_id TEXT NOT NULL DEFAULT 'local' REFERENCES sources(id),  -- no ON DELETE CASCADE
```
Every `ON DELETE CASCADE` in the file points *out of* conversations/messages (children), never
into conversations. So `DELETE FROM agents` / `DELETE FROM sources` cannot reach a conversation
row. Closes a path the lane never considered.

### 1d. EXTENSION — soft delete / tombstone: NONE

```
$ rg -n -i 'is_deleted|deleted_at|tombstone|is_hidden|soft_delete' src/storage/sqlite.rs | wc -l
0
```
This matters for experiment design: it means `SELECT COUNT(*) FROM conversations` is a valid
instrument. Had a tombstone column existed, the lane's probe would have read flat through a real
loss.

### 1e. EXTENSION — replace-on-write: NONE, verified WITH a positive control

```
$ rg -i -U 'INSERT\s+OR\s+REPLACE\s+INTO\s+conversations|REPLACE\s+INTO\s+conversations' src/ | wc -l
0
$ rg -i -U 'INSERT\s+INTO\s+conversations[^;]*ON\s+CONFLICT' src/ | wc -l
0
POSITIVE CONTROL — same pattern on any table:
$ rg -i -U 'INSERT\s+OR\s+REPLACE\s+INTO' src/ | wc -l
27      (meta, conversation_tail_state, conversation_external_lookup, ...)
```
The negative is real, not a dead regex. Lane finding 3 CONFIRMED.

### 1f. Reachable message deletes are FK-integrity only, not source-driven

`src/storage/sqlite.rs:5917` `DELETE FROM messages WHERE id IN` sits in
`delete_orphan_message_id_chunk_once` (fn at 5890). Every orphan selector is a parent-existence
test, e.g. `src/storage/sqlite.rs:6002-6005`:
```
SELECT message_id FROM message_metrics WHERE NOT EXISTS (SELECT 1 FROM messages WHERE messages.id = message_metrics.message_id)
```
No predicate anywhere references `source_path`. A message whose conversation row exists is never
an orphan, so this cannot reach a source-absent conversation.

### 1g. The product says so in its own source

`src/indexer/mod.rs:21602-21606`, verbatim:
```
// Incremental watch indexing is append-only today: once a path is gone,
// classify_paths() cannot derive a scan window from it and the ingest
// path cannot delete the stale conversation rows it previously indexed.
// Treat remove events as noise until delete-aware rebuilds exist.
notify::event::EventKind::Remove(_) => false,
```
This is the strongest single artifact in the whole question and it corroborates the lane.

### 1h. Writable open refuses to replace — CONFIRMED

`open_storage_for_index` (fn at `src/indexer/mod.rs:14540`) returns `Err` on every unhealthy
existing-DB branch — literally `"canonical db is busy/locked during index open; refusing to
replace it"` at 14557-14559 and 14586-14588 — and yields `opened_fresh_for_full = full_index`
only in the final `else` branch where the DB file does not exist (14596-14599).
(The lane cited 14558/14590/14598-14601; off by 1-4 lines, substance correct.)

### 1i. Binary/source specimen check — CLEAN, and this defused my strongest objection

The falsifier proposes testing the installed binary, not the worktree. I expected a
differential-specimen problem. There is none:
```
$ /Users/dalecarman/.local/bin/cass --version
cass 0.6.9
git commit: 447d97fe60962d1ed1f34841e508f61a6b4302c4
$ git diff --stat 447d97fe..74a72233 -- src/
(empty — no src/ changes; the 4 commits touch only .beads/ and thoughts/)
```
So the census performed on HEAD's source is valid for the installed binary. Reporting this as a
null result in the lane's favor.

### 1j. Live archive, independently measured (read-only)

```
$ sqlite3 "file:/Users/.../agent_search.db?mode=ro" "SELECT (SELECT COUNT(*) FROM conversations), (SELECT COUNT(*) FROM agents);"
14024|9      (0.07s)
```
Lane reported 13371|9. Count has RISEN by 653 during the running backfill — consistent with the
additive-writer finding, and itself a live corroboration of 1e.

---

## 2. REFUTATION 1 — the headline is true of ONE invocation, and states it of all

I read the gate myself. `should_try_readonly_canonical_force_rebuild`,
`src/indexer/mod.rs:2092-2102`:
```rust
opts.force_rebuild
    && !opts.full
    && !opts.watch
    && !opts.semantic
    && !opts.build_hnsw          // <-- the lane's recital omits this term
    && opts.watch_once_paths.as_ref().is_none_or(|paths| paths.is_empty())
    && opts.db_path.exists()
```
And inside `try_readonly_canonical_force_rebuild` (2104), a second exit at 2119-2126:
`if total_conversations == 0 { ...; return Ok(false); }`.

`Ok(false)` does not stop the run. `src/indexer/mod.rs:12190-12194`, inside `run_index` (fn at
12008):
```rust
if try_readonly_canonical_force_rebuild(&opts, &progress_bump)? { return Ok(()); }
preflight_phase!("watch_startup:open_storage");
let (mut storage, ...) = open_storage_for_index(&opts.db_path, opts.full)?;   // WRITER
```

So `--force-rebuild` combined with **any** of `--full`, `--watch`, `--semantic`,
`--build-hnsw`, or explicit watch-once paths goes to the writable path. The headline —
"`cass index --force-rebuild` … opens SQLite read-only" — is false for all of those.

This is not academic. The lane's own §6.6 says `--full --force-rebuild` is "the form the cass
skill actually prescribes." The headline therefore reassures an operator about a command shape
the skill does not tell them to run, while the shape it *does* tell them to run is outside the
read-only claim entirely.

## 3. REFUTATION 2 — the stated basis ("the census is exhaustive over the tree") is false

§7 of the lane log grounds its 0.92-0.95 confidence on "the DELETE census (§1) is exhaustive over
the tree." It is a census of one verb against one table. It missed §1b/1c/1d/1e above — all of
which I had to run to know the answer — and it missed the following outright.

**`promote_staged_historical_seed` replaces the entire canonical DB by rename, and parks the only
backup in a TempDir.** `src/storage/sqlite.rs:2735-2771`:
```rust
let canonical_backup = staged_seed.tempdir.path().join("pre-seed-canonical-backup.db");
if had_canonical {
    move_database_bundle(canonical_db_path, &canonical_backup)...?;   // canonical -> tempdir
}
move_database_bundle(&staged_seed.db_path, canonical_db_path)         // staged  -> canonical
```
`move_database_bundle` (1498) is `fs::rename` of the db plus `-wal` and `-shm` (1509-1521). No
DELETE, no DROP — a whole-archive substitution that the lane's census could not have detected by
construction, with the displaced original in a `TempDir` that is reaped on drop.

**The guard holds today, and I verified it.** Sole caller is
`maybe_seed_empty_canonical_from_historical_bundle` (`src/indexer/mod.rs:14932`), whose first act
is:
```rust
let conversation_count = count_total_conversations_exact(&storage)?;
if conversation_count > 0 { return Ok((storage, None)); }
```
Its sole caller in turn is `src/indexer/mod.rs:12748`, itself nested under
`if canonical_sessions_before_salvage == 0`. `count_total_conversations_exact`
(`src/indexer/mod.rs:8024-8034`) propagates errors with `?` rather than swallowing them to 0, so
a failed count cannot masquerade as an empty archive. At 14,024 conversations this path is
unreachable.

The conclusion survives. The *stated reason* for believing it does not — the lane reached the
right answer without having looked at the most dangerous statement in the subsystem.

## 4. REFUTATION 3 — the falsifier can manufacture a false "safe"

Attacking §6 as designed:

1. **No exit-code capture on any arm.** §6.5/6.6/6.7 run `cass index ... 2>"$T/armX.stderr"` then
   `probe`. A run that dies leaves the DB untouched and the probe reads `2|1|1` — scored
   SURVIVES. §7 names this ("an arm errors out … is UNAVAILABLE, not a pass") but no step checks
   `$?`. Under the repo's own exit-code rule this is the defect that most directly produces a
   false safe on a data-loss question.
2. **Arms are not independent.** A, B, C run sequentially against one archive. If A drops the
   row, B and C measure a contaminated baseline and their unchanged `1|1|0` scores as "survived."
   Each arm needs its own `cp -a` of the post-move state.
3. **Arms B and C have no positive control that the rebuild ran.** P2(a) greps for
   `force_rebuild_uses_readonly_authoritative_canonical_db_rebuild_only` — emitted only inside
   `try_readonly_canonical_force_rebuild` (`src/indexer/mod.rs:2145`), i.e. only on the fast path
   B and C by definition do not take. P2(b) (segment mtimes) is prose, not a command. So for the
   two writable-path arms — the arms that actually carry risk — a silent no-op is
   indistinguishable from a pass. This is precisely the failure my lens asks about.
4. **The §6.0 guard cannot catch the mistake it is posted to catch.**
   `case "$CASS_DATA_DIR" in /tmp/cass-falsifier-*)` passes for `$T/data` *and* `$T/data-control`.
   At §6.8 the risk is running the one destructive command against the fixture instead of the
   copy; the guard is blind to exactly that. Its label overstates its computation. (Containment
   is real — everything is under `$T` — so the blast radius is the experiment, not the archive.)
5. **P1 hardcodes the agent slug `claude_code`.** If the connector assigns a different slug to
   `$T/home/.claude/projects/demo`, P1 purges nothing, reads `2|1|1`, and by the lane's own rule
   "every result above is void" — a false-negative control voiding a sound experiment. Derive the
   slug from `SELECT slug FROM agents` instead.
6. **P1 validates the probe, not the subject** — which is correct and worth stating plainly: no
   control can prove the rebuild path *could* have deleted. That residual is irreducible, so the
   experiment's ceiling is lower than §7's 0.92-0.95 implies.

## 5. REFUTATION 4 — citation accuracy on the destructive path itself

The destructive path is the half a fix will be written against, and two of its four citations are
wrong:

| lane cited | actually contains | real site |
|---|---|---|
| `src/lib.rs:90666` calls `purge_excluded_agent_archive_data` | error-handling tail of that fn | **90741**, in `run_agents_exclude` (fn at 90709) |
| `src/lib.rs:90524` calls `storage.purge_agent_archive_data` | `cass models install` output | **90599**, in `purge_excluded_agent_archive_data` (fn at 90573) |
| `src/storage/sqlite.rs:7173-7176` DELETE | correct | 7174, in fn at 7121 |
| `tests/e2e_sources.rs:321` | correct | correct |

The substance is right — I read `run_agents_exclude` at 90709-90775 and confirm there is **no
confirmation prompt, no dry-run, no pre-count**: `config.save()` then
`purge_excluded_agent_archive_data(agent, cli)?` then the count is *printed after the fact*
(90755-90759). One hazard the lane did not note: the purge runs on the `changed == false` branch
too, so re-excluding an already-excluded agent purges again.

## 6. Minor — the "existing test proves it" claim is one step stronger than the test

`tests/e2e_sources.rs:321` + `seed_archive_conversation` (:46) are as described:
`source_path: format!("/tmp/{agent_slug}-{marker}.jsonl")`, never created; post-exclude assertion
`conversations.len() == 1`. But `DELETE FROM conversations WHERE agent_id = ?1` has no source
predicate, so source-absence is *incidental* to the test, not pinned by it. The lane's downstream
inference — that adding a guard turns this test red — is still correct (either the command exits
non-zero or the count reads 2). It just is not a test *about* source absence.

---

## 7. Bottom line

| lane claim | my verdict |
|---|---|
| No reachable statement deletes a conversation row on any index/rebuild path | **CONFIRMED, and broadened** (drop/cascade/tombstone/replace all checked) |
| Bare `cass index --force-rebuild` uses a read-only SQLite handle | **CONFIRMED** |
| "`cass index --force-rebuild`" is read-only *as a general statement* | **REFUTED** — `--full`/`--watch`/`--semantic`/`--build-hnsw`/empty-DB all fall through to the writer |
| Census is exhaustive over the tree | **REFUTED** — missed the whole-DB rename in `promote_staged_historical_seed` |
| `sources agents exclude` is source-blind and unguarded | **CONFIRMED** |
| Bead item 1 can be downgraded to a confirmation experiment | **REJECT as written** — narrow the claim first, then confirm |
| The §6 falsifier is ready to run | **REFUTED** — 6 defects, at least two of which yield a false "safe" |

**Recommended disposition.** Keep the operational advice (do not run rebuilds against the live
archive; the P0 is the exclude guard). Rewrite the headline to name the invocation: *bare*
`cass index --force-rebuild` on a populated archive takes a read-only SQLite path; every other
`--force-rebuild` combination goes through the writable open, which is still non-deleting but by
a different and less direct argument. Fix the falsifier's exit-code capture, per-arm isolation,
and B/C run-proof before executing it. And record `promote_staged_historical_seed` as a watched
path — it is the one place in the subsystem where the whole archive can be substituted, its guard
is a single `> 0` comparison, and no test pins that guard.
