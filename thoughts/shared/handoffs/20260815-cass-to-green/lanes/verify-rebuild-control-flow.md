# Lane: verify-rebuild-control-flow — adversarial verification of the rebuild-safety lane

Role: ADVERSARIAL VERIFIER. Lens: control-flow / destructive-statements / experiment-design.
Target: lane log `thoughts/shared/handoffs/20260815-cass-to-green/lanes/rebuild-safety.md`.
Lane kind: READ-ONLY. Only write is this file. No `cass index`, no `cass sources`, no git mutations.
Worktree: /Users/dalecarman/dev/coding_agent_session_search/.claude/worktrees/cass-to-green-c6bfb589 @ 74a72233
Started: 2026-08-15

Bottom line up front: **REFUTED.** Not because I found a path where `cass index
--force-rebuild` deletes a conversation row — I did not, and I say that plainly
below. Refuted because the headline's stated *mechanism* is false for the command
the product actually prescribes, false even for the bare command under a flag the
same doc prescribes, blind by construction to the file-level archive-destruction
paths this codebase really has, resting on a test that does not pin what the lane
says it pins, and backed by a falsifier that cannot execute as written.

---

## 0. Method

I did not start from the lane's line numbers. I re-entered at
`Commands::Index` in `src/lib.rs` and traced forward, then ran an independent
destructive-operation census that was NOT restricted to `DELETE FROM
conversations` — it included dynamic/`format!`-built SQL, `DROP TABLE`, table-
rebuild migrations, FK cascade, and **file-level** removal/rename of the database
bundle. The file-level class is the one a statement census cannot see, and it is
where the real findings are.

Every lane citation I could check, I opened. Verified-correct ones are listed in
§1 so the parent does not have to re-check them.

---

## 1. What survived independent re-trace (the lane is right about these)

| lane claim | my check | verdict |
|---|---|---|
| exactly 4 literal `DELETE FROM conversations` | `rg -n -i -U 'DELETE\s+FROM\s+conversations\b' src/ \| wc -l` → `4` | CONFIRMED |
| indexer/mod.rs:20738 is test-only | `#[cfg(test)]` at 20721, `fn reset_storage` at 20722 | CONFIRMED |
| sqlite.rs:20499/20601 are test-only | `#[cfg(test)]` 15300 / `mod tests {` 15301 — both sites are inside | CONFIRMED |
| fast path runs before writable open | `try_readonly_canonical_force_rebuild` at 12190, `open_storage_for_index` at 12196 | CONFIRMED |
| gate is `force_rebuild && !full && !watch && !semantic && !build_hnsw && watch_once empty && db exists` | `fn should_try_readonly_canonical_force_rebuild` at src/indexer/mod.rs:2091-2102 | CONFIRMED |
| rebuild opens read-only | `FrankenStorage::open_readonly(db_path)` at 18134, inside `rebuild_tantivy_from_db_with_options` (18104-19105) | CONFIRMED |
| writer is additive, no upsert | `franken_insert_conversation` at sqlite.rs:12052 issues plain `INSERT INTO conversations(...)`; `rg -n -i 'INSERT OR REPLACE INTO conversations\|ON CONFLICT'` returns no conversations hit | CONFIRMED |
| exclude is source-blind | sqlite.rs:7173-7176, `DELETE FROM conversations WHERE agent_id = ?1`, no predicate, no prompt | CONFIRMED |

Three things I established that the lane did **not**, all of which make the
narrow safety claim *stronger*:

- **No dynamic-SQL escape hatch.** `rg -n -i 'DELETE\s+FROM\s+\{' src/` returns
  **0**. There is no `format!`-built delete whose table name could resolve to
  `conversations`, so the literal census is genuinely exhaustive over SQL.
- **No FK cascade.** Live schema, read read-only:
  `conversations.agent_id INTEGER NOT NULL REFERENCES agents (id)` — no
  `ON DELETE CASCADE`. So the three `DELETE FROM agents` sites cannot take
  conversations with them.
- **`open_storage_for_index` can never report a rebuild.** Every branch returns
  `canonical_storage_rebuilt = false` (src/indexer/mod.rs:14573, 14587, 14597).
  So the `canonical_storage_rebuilt ||` arm of
  `should_salvage_historical_databases` (src/indexer/mod.rs:8273) is dead on an
  existing DB.
- **Production migrations never rebuild the table.** `MIGRATION_V1`–`V10` are all
  `#[cfg(test)]` (that is where the `DROP TABLE conversations` at sqlite.rs:3328
  lives). The shipped post-V13 set — `MIGRATION_V14` (3517), `V15_TAIL_STATE_TABLE`
  (3527), `V16` (3538), `V17` (3545), `V18` (3553), `V19` (3576), `V20` (3595) —
  contains no `DROP TABLE conversations`, no table copy-and-rename, and no
  `DELETE` against conversations.
- **The whole-archive replacement is double-gated on an empty archive.**
  `maybe_seed_empty_canonical_from_historical_bundle` returns early when
  `count_total_conversations_exact(&storage) > 0` (src/indexer/mod.rs:14936-14939),
  and its only call site is itself inside `if canonical_sessions_before_salvage == 0`
  (src/indexer/mod.rs:12749-12751). At 14,142 rows it cannot fire.

So the **narrow** proposition — *no SQL statement reachable from any `cass index`
invocation can delete a conversation row from a populated archive* — held up under
an independent trace and a wider census. I want that on the record, because the
refutation below is not "the rebuild eats your data."

---

## 2. REFUTATION A — the headline's mechanism does not describe the command the
## product tells operators to run

The fast-path gate requires `!opts.full` (src/indexer/mod.rs:2093).

The shipped guidance prescribes `--full --force-rebuild`, twice:

```
SKILL.md:127:  cass index --full --force-rebuild
SKILL.md:422:  | 5 | Data corruption | Run `cass index --full --force-rebuild` |
```

`--full` fails the gate, so the prescribed command **skips the read-only fast path
entirely** and goes down `open_storage_for_index` → writable handle → writable
preflight (`UPDATE meta SET value = value…`, src/indexer/mod.rs:12200) → orphan
cleanup → salvage decision → scan. That pipeline is safe for conversation rows by
census, but it is not "opens SQLite read-only and rebuilds only Tantivy."

The lane knows this — Arm B of its falsifier is exactly this branch. But the
headline states the read-only mechanism unconditionally, and the headline is what
travels. On a data-loss question a mechanism claim that is true only of a command
nobody is told to run is a refutable claim, because the next reader will apply it
to the command they were told to run.

## 3. REFUTATION B — "never obtains a write handle on SQLite at all" is false even
## for the bare form

`run_index_with_data` opens the database **writable, before `run_index` is ever
called**, whenever `--idempotency-key` is passed (src/lib.rs:79435-79455):

```rust
    if let Some(key) = &idempotency_key {
        let cached = with_frankensqlite_connection(
            &db_path,
            "checking index idempotency cache",
            |conn| {
                if let Err(e) = conn.execute(
                    "CREATE TABLE IF NOT EXISTS idempotency_keys ( … )",
                ) { … }
                …
                if let Err(e) = conn.execute_compat(
                    "DELETE FROM idempotency_keys WHERE expires_at < ?1",
                    frankensqlite::params![now_ms],
                ) { … }
```

That is a `CREATE TABLE` and a `DELETE` against the canonical archive, on the
`index` path, before any gate the lane examined. It is harmless to conversations.
It is fatal to the sentence "for a populated archive, bare `cass index
--force-rebuild` **never obtains a write handle on SQLite at all**."

And this is not a hypothetical flag: SKILL.md:130 prescribes
`cass index --full --idempotency-key "build-$(date +%Y%m%d)"`.

The correction matters more than the fact. **The archive's safety does not come
from read-only-ness. It comes from the statement census.** The lane led with the
weaker of its two arguments, and the weaker one is the one that breaks.

## 4. REFUTATION C — a statement census is blind to the file-level destruction
## paths this codebase actually has

Three production functions remove or replace the entire `conversations` table
without executing a single `DELETE`. A census keyed on SQL reports "4 deletes,
3 test-only" for all three.

**C1 — `FrankenStorage::open_or_rebuild` deletes the database file**
(src/storage/sqlite.rs:4617-4648):

```rust
                Ok(SchemaCheck::NeedsRebuild(reason)) => {
                    let backup_path = create_backup(path)?;
                    cleanup_old_backups(path, MAX_BACKUPS)?;
                    remove_database_files(path)?;
```

`remove_database_files` (sqlite.rs:1644-1667) `fs::remove_file`s the db plus
`-wal`/`-shm`. **Not reachable today** — `rg -n 'open_or_rebuild' src/` returns
the definition plus two tests (16399, 16410) and no production caller. I am
reporting it because it is one call site away from being reachable, and the
lane's method would still read "safe" the day someone adds it. Note also that
`create_backup` returns `Ok(None)` when `copyable_bundle_file_exists` is false
(sqlite.rs:1393-1400) — in that case the `?` does not fire and the delete
proceeds with **no backup at all**.

**C2 — `cass doctor --fix` moves the whole archive aside** (src/lib.rs:69812,
inside `run_doctor_impl` which begins at src/lib.rs:68094):

```rust
            if !db_ok {
                let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
                let backup_path = db_path.with_extension(format!("corrupt.{ts}"));
                match crate::storage::sqlite::move_database_bundle(&db_path, &backup_path) {
```

Guarded, and I checked the guard rather than assuming it:
`safe_auto_archive_rebuild_refused = safe_auto_run_requested && needs_rebuild &&
fix_can_mutate && !db_ok` (src/lib.rs:69713-69717); `cass doctor --fix` resolves
to `DoctorExecutionMode::SafeAutoFix` (src/doctor.rs:275-276), which sets
`safe_auto_run_requested` (src/lib.rs:68259-68260), which forces
`derived_rebuild_attempted = false` (69723-69724) and so never reaches the move.
So **legacy `cass doctor --fix` refuses**, correctly.

What is **UNVERIFIED** is every other doctor surface that can set
`fix_can_mutate = true` while `execution_mode != SafeAutoFix` — for those,
`safe_auto_run_requested` is false, the refusal does not fire, and with
`!db_ok` the else-branch is live. I did not enumerate the surfaces. This matters
because `docs/planning/RECOVERY_RUNBOOK.md:108` and `:114` prescribe
`cass doctor --fix --json` / `cass doctor --fix --force-rebuild --json` as the
recovery path, and the archive is currently in a state where `db_ok` is exactly
the thing in doubt.

**C3 — historical seeding destroys the pre-seed archive with no surviving backup.**
`promote_staged_historical_seed` (src/storage/sqlite.rs:2735-2769) moves the live
canonical bundle to `canonical_backup`, which is
`staged_seed.tempdir.path().join("pre-seed-canonical-backup.db")` — inside a
`tempfile::TempDir` (2698-2701, 2712-2713). A `TempDir` deletes its contents on
drop. So on a *successful* promotion the previous archive is not backed up, it is
**erased**. As established in §1 this is double-gated on a zero-conversation
archive and cannot fire at 14,142 rows. But "no index/scan/rebuild/watch/salvage
path can remove a conversation row" is not an accurate description of a function
that renames the archive out from under itself into a directory scheduled for
deletion.

## 5. REFUTATION D — the exclude blast radius is three commands wide, not one

`archive_agent_slug_for_exclusion` (src/lib.rs:90559-90564):

```rust
    match agent.trim().to_ascii_lowercase().as_str() {
        "claude" | "claude-code" | "claude_code" => "claude_code".to_string(),
        other => other.to_string(),
    }
```

`cass sources agents exclude claude`, `… claude-code`, and `… claude_code` all
destroy the same 4,050 rows. The shortest spelling is the one a human types. Any
guard added to the exclude path must key on the **resolved** slug, after this
mapping, not on the typed argument — otherwise the guard covers the spelling
nobody uses.

## 6. REFUTATION E — the "existing test already proves it" claim is an over-read

`tests/e2e_sources.rs:321 sources_agents_exclude_purges_local_archive_data_by_default`
seeds via `seed_archive_conversation` (tests/e2e_sources.rs:46), which sets
`source_path: format!("/tmp/{agent_slug}-{marker}.jsonl")` and never creates it —
that part of the lane's reading is correct, and I confirmed the path is absent on
this machine:

```
$ ls -la /tmp/openclaw-purge-me.jsonl
"/tmp/openclaw-purge-me.jsonl": No such file or directory (os error 2)
```

But the test **never asserts, creates, stats, or mentions the source file**. It
would pass byte-identically if the file existed. So it pins *"exclude purges
archived local data"*; it does not pin *"exclude purges rows whose source file is
gone,"* and it is not, as the lane says, "an executed falsifier for the exclude
path" on the source-absence question.

The practical conclusion survives — the code has no predicate at all, so absence
is irrelevant to what it deletes — but the *evidence* claim does not, and the
difference is load-bearing for the fix. The lane tells the implementer this test
will "turn red by design" and must be updated. It will turn red for the wrong
reason: not because it asserts source-absent deletion, but because it asserts
deletion at all. A guard that refuses when sources are missing breaks it as
collateral, and the honest repair is a new test that creates one source file and
omits the other, not a re-arming of this one.

## 7. REFUTATION F — the designed falsifier cannot run as written

The fixture (lane §6.1) writes lines of this shape:

```
{"role":"user","content":"FALSIFIERMARKERKEEP alpha"}
```

The Claude Code parser is vendored:
`franken-agent-detection` rev `b62d859` (Cargo.toml:94), file
`~/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/b62d859/src/connectors/claude_code.rs`.
Its acceptance test, lines 404-415:

```rust
                    let entry_type = val.get("type").and_then(|v| v.as_str());
                    let role_hint = val
                        .get("message")
                        .and_then(|m| m.get("role"))
                        .and_then(|v| v.as_str())
                        .or_else(|| val.get("role").and_then(|v| v.as_str()));
                    let is_user_assistant = matches!(entry_type, Some("user" | "assistant"))
                        || (entry_type == Some("message")
                            && matches!(role_hint, Some("user" | "assistant")));
                    if !is_user_assistant {
                        continue;
                    }
```

A line with no `"type"` key gives `entry_type = None`; both disjuncts are false;
every line is skipped. Then, lines 541-546:

```rust
            if messages.is_empty() {
                …
                continue;
            }
```

No conversation is emitted. **The baseline probe reads `0|0|0`, not the required
`2|1|1`**, and the design's own negative control stops the run at step one. That
control is the best thing in the design and it works — but the experiment as
written produces no answer at all.

The parser's own fixture (same file, :805-806) gives the correct shape:

```
{"type":"user","timestamp":"2025-12-01T10:00:00Z","message":{"role":"user","content":"MARKER"}}
{"type":"assistant","timestamp":"2025-12-01T10:00:01Z","message":{"role":"assistant","content":"MARKER"}}
```

One adjacent hazard I checked and **cleared**, because it looked decisive and is
not: the active-session filter skips any `.jsonl` modified within 120 s
(src/indexer/mod.rs:143-160, 175-183), which would silently swallow a
just-written fixture. It is disabled on non-watch runs —
`ActiveSessionSourceFilter::new(opts.watch && …)` at src/indexer/mod.rs:11585,
11731, 20800 — so it does not affect Arms A/B/C. Recording it because the next
person to write this fixture will trip over it the moment they add `--watch`.

## 8. Two more experiment-design defects

**P1 is a probe control, not a path control.** `cass sources agents exclude`
shares no code with the rebuild. It proves the sqlite probe can observe a
deletion. It proves nothing about whether Arms A/B/C executed, and the lane's
"if this does not go to zero… every result above is void" invites the converse
reading — that P1 going to zero validates the arms. It does not. P2 is the only
thing separating a real pass from a silent no-op, and P2(b) (index-directory
segment set + mtimes) is left as a sentence rather than a procedure.

**Scale-shape mismatch.** The rebuild plans staged shards against byte budgets —
`LEXICAL_REBUILD_STAGED_SHARD_MESSAGE_BYTES_FLOOR` 16 MB / `_DEFAULT` 64 MB /
`_CEILING` 128 MB (src/indexer/mod.rs:8930-8932) — and carries single-conversation
guardrail fallbacks (test `rebuild_tantivy_from_db_falls_back_for_single_conversation_guardrail_outlier`,
src/indexer/mod.rs:40447). A 2-conversation, ~2 KB archive lands in one trivial
shard; the live archive plans many. A green result on the fixture is real evidence
about the *statements* on the path and weak evidence about the *live* run, and the
write-up must say so rather than reporting "rows survive."

## 9. One finding useful for the fix, not a refutation

`purge_agent_archive_data` (src/storage/sqlite.rs:7157-7184) deletes
`conversation_external_lookup`, `conversation_external_tail_lookup`,
`conversations`, and the emptied `agents` row — it **does not delete `messages`**.
Message bodies are left orphaned and are reaped later by
`cleanup_orphan_fk_rows` (sqlite.rs:4453-4473 → `DELETE FROM messages WHERE id IN`
at 5917) on the next writable index run.

So an accidental `exclude` is **partially recoverable until the next
`cass index` writes** — the text is still in the `messages` table, keyed by a
`conversation_id` whose parent is gone. That is worth building into the undo/guard
design, and it is worth telling an operator who has just made the mistake.

## 10. Live archive state, measured this session

```
$ DB="/Users/dalecarman/Library/Application Support/com.coding-agent-search.coding-agent-search/agent_search.db"
$ sqlite3 "file:$DB?mode=ro" "SELECT (SELECT COUNT(*) FROM conversations),(SELECT COUNT(*) FROM agents);"
14142|9
$ sqlite3 "file:$DB?mode=ro" "SELECT COUNT(*) FROM conversations c JOIN agents a ON a.id=c.agent_id WHERE a.slug='claude_code';"
4050
$ # positive control — a slug that cannot exist must return 0, proving the join discriminates
$ sqlite3 "file:$DB?mode=ro" "SELECT COUNT(*) FROM conversations c JOIN agents a ON a.id=c.agent_id WHERE a.slug='zzz-no-such-agent';"
0
```

14,142 against the lane's 13,371 — the backfill is live, which is itself the
positive control that this instrument reads current state rather than a cache.
`claude_code` is unchanged at 4,050.

## 11. Verdict

**refuted = true.**

The narrow safety proposition survived and got stronger: I found no statement and
no reachable path by which `cass index` in any form removes a conversation row
from a populated archive, and I found two structural reasons the lane did not cite
(no dynamic-SQL delete, salvage double-gated on an empty archive).

What is refuted is the headline as written and the confidence it licenses:

1. its mechanism does not apply to `--full --force-rebuild`, the form SKILL.md
   prescribes twice;
2. "never obtains a write handle" is false under `--idempotency-key`, which the
   same file prescribes;
3. the census method is blind to three file-level archive-destruction functions,
   one of which (`doctor --fix` with `!db_ok`) is guarded only on the surface I
   checked and is prescribed by the recovery runbook;
4. the test cited as an existing falsifier does not test the property it is cited
   for;
5. the experiment designed to close the residual cannot execute — its fixture
   parses to zero conversations.

Recommended posture for the parent: keep the lane's *conclusion* about the
rebuild, discard its *reason*. The correct one-line statement is "no reachable SQL
statement can delete a conversation row, in any index mode" — which is stronger,
survives `--full`, and does not collapse the moment someone passes a flag. Then
treat the file-level paths in §4 as their own question, because they are the ones
that can empty this archive and nothing in the lane's method could have seen them.
