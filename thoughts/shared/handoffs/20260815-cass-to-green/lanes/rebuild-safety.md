# Lane: rebuild-safety — is `cass index --force-rebuild` a data-destruction path?

Bead: coding_agent_session_search-qtn0e (P0)
Lane kind: READ-ONLY. Only write is this file. No `cass index`, no `cass sources`, no git mutations.
Worktree: /Users/dalecarman/dev/coding_agent_session_search/.claude/worktrees/cass-to-green-c6bfb589 @ 74a72233
Started: 2026-08-15

## 0. Bead read

`br show coding_agent_session_search-qtn0e` FAILED in this worktree — the worktree has no
`.beads/beads.db`:

```
$ br show coding_agent_session_search-qtn0e
warning: could not prove that no sync merge is pending (Sync conflict: Pending sync-merge
state is unknown because database '.../.claude/worktrees/cass-to-green-c6bfb589/.beads/beads.db'
is missing). Read-only command will proceed with automatic sync disabled...
Error: Sync conflict: Refusing storage open because pending sync-merge state could not be
inspected under database-family authority
```

Read the bead body from the tracked export instead (read-only, main checkout):
`rg -n 'qtn0e' /Users/dalecarman/dev/coding_agent_session_search/.beads/issues.jsonl` → line 1413.
Full description read. Key claims to check:
- `DELETE FROM conversations` at src/indexer/mod.rs:20738 is `#[cfg(test)]` → CHECKED, TRUE.
- `purge_agent_archive_data` deletes every conversation for an agent → CHECKED, TRUE.
- "I have NOT audited whether `index --full` / `--force-rebuild` themselves drop
  source-absent rows" → this lane's job.

## 1. Census: every `DELETE FROM conversations` in the tree

```
$ rg -n 'DELETE FROM conversations' src/
src/indexer/mod.rs:20738:         DELETE FROM conversations;
src/storage/sqlite.rs:7174:            "DELETE FROM conversations WHERE agent_id = ?1",
src/storage/sqlite.rs:20499:            .execute_compat("DELETE FROM conversations WHERE id IN (2, 4)", fparams![])
src/storage/sqlite.rs:20601:                "DELETE FROM conversations WHERE id IN (3, 5, 7, 8)",
$ rg -n 'DELETE FROM conversations' src/ | wc -l
4
```

Four total. Disposition:
- **indexer/mod.rs:20738** — inside `fn reset_storage`, and the line immediately above the fn
  is `#[cfg(test)]` (src/indexer/mod.rs:20721, `fn reset_storage` at 20722). Unreachable in the
  shipped binary. Bead's
  claim CONFIRMED.
- **storage/sqlite.rs:7174** — `purge_agent_archive_data`, the reachable deleter. See §2.
- **20499 / 20601** — inside the `#[cfg(test)] mod tests` that opens at src/storage/sqlite.rs:15300-15301
  (`rg -n '^mod tests|^#\[cfg\(test\)\]$' src/storage/sqlite.rs` → last entry before 20499 is
  15300/15301). Fixture setup for orphan-cleanup tests.

No other statement in the product can remove a `conversations` row. The orphan-cleanup helpers
(`delete_rows_by_i64_chunks`, sqlite.rs:5818-6062) delete only `messages`, `message_metrics`,
`token_usage`, `snippets`, `conversation_tags` — never `conversations`.

## 2. `cass sources agents exclude <agent>` — CONFIRMED DESTRUCTIVE, source-blind

`run_agents_exclude` (src/lib.rs:90634) → `purge_excluded_agent_archive_data` (src/lib.rs:90498,
called at 90666 unless `--keep-indexed-data`) → `FrankenStorage::purge_agent_archive_data`
(src/storage/sqlite.rs:7121).

The delete, verbatim (src/storage/sqlite.rs:7173-7176):

```rust
        tx.execute_compat(
            "DELETE FROM conversations WHERE agent_id = ?1",
            fparams![agent_id],
        )?;
```

There is **no predicate on source_path, no existence check, no count of source-absent rows, and
no confirmation prompt**. Every conversation for the agent goes, whether or not its source file
still exists on disk. `cass sources agents exclude claude_code` therefore destroys all 4,050
claude_code rows including the 3,877 with no surviving source. Bead claim CONFIRMED verbatim.

## 3. `--force-rebuild` — traced end to end

### 3a. The fast path: DB opened READ-ONLY, only Tantivy is rebuilt

`src/indexer/mod.rs:12190`:

```rust
    if try_readonly_canonical_force_rebuild(&opts, &progress_bump)? {
        return Ok(());
    }
```

This runs **before** `open_storage_for_index` (12194) — i.e. before any writable handle on the
canonical DB exists. Its gate (src/indexer/mod.rs:2091-2102):

```rust
fn should_try_readonly_canonical_force_rebuild(opts: &IndexOptions) -> bool {
    opts.force_rebuild
        && !opts.full
        && !opts.watch
        && !opts.semantic
        && !opts.build_hnsw
        && opts.watch_once_paths.as_ref().is_none_or(|paths| paths.is_empty())
        && opts.db_path.exists()
}
```

and the body (2104-2160) opens `FrankenStorage::open_readonly(&opts.db_path)`, counts
conversations, returns `false` (falls through) only when the count is 0, and otherwise calls
`rebuild_tantivy_from_db_deferred_startup_with_progress_bump(&opts.db_path, &opts.data_dir,
total_conversations, ...)` then returns. The strategy it records is literally named
`force_rebuild_uses_readonly_authoritative_canonical_db_rebuild_only` (2145).

So for a populated archive, bare `cass index --force-rebuild` **never obtains a write handle on
SQLite at all**. The thing being rebuilt is the derived Tantivy lexical index, *from* the DB.
The DB is the input, not the output.

### 3b. The `--full --force-rebuild` path also skips the filesystem rescan

src/indexer/mod.rs:12387-12392:

```rust
    // canonical_only_full_rebuild: when --force-rebuild is set and we already
    // have canonical sessions in the DB, skip the expensive filesystem rescan
    // and go straight to rebuild_tantivy_from_db().  Plain --full continues to
    // rescan as expected (preserving the #153 fix).
    let canonical_only_full_rebuild =
        opts.force_rebuild && initial_canonical_sessions_before_salvage > 0;
```

Same shape: the canonical rows are the authority and the derived index is regenerated from them.

### 3c. Opening storage for an index run is explicitly non-destructive

`open_storage_for_index` (src/indexer/mod.rs:14540-14603): when `db_path.exists()` every branch
either returns the opened storage or returns an **error refusing to replace it** —
`canonical_archive_unhealthy_for_index_error(...)`, and for a locked DB literally
`"canonical db is busy/locked during index open; refusing to replace it"` (14558, 14590). The
third tuple element (`opened_fresh_for_full`) is `full_index` only in the `else` branch, i.e.
only when the DB file did **not** exist (14598-14601). There is no truncate/replace on an
existing archive.

A second guard sits above it (src/indexer/mod.rs:12376-12385): a full rebuild that detects
incomplete historical salvage state logs *"full rebuild detected incomplete historical salvage
state; refusing to replace the canonical archive"* and errors out.

### 3d. The write path is additive, not replace

`franken_insert_conversation` (src/storage/sqlite.rs:12052-12123) issues a plain
`INSERT INTO conversations(...) VALUES(...)` — not `INSERT OR REPLACE`, not delete-then-insert.
A duplicate provenance conflict returns `Ok(None)` so the caller merges into the existing row
(doc comment 12047-12051). `rg -n 'INSERT OR REPLACE INTO conversations|ON CONFLICT' src/storage/sqlite.rs`
matching `conversat` returns **nothing**.

### 3e. Source-absence is a first-class, non-lossy state elsewhere in the product

The raw-mirror backfill already models it: `receipt.source_missing = !source_stat.exists;`
(src/lib.rs:35598) and the action string `"source_missing_db_projection_only"` (src/lib.rs:35655)
— i.e. when the source file is gone, cass projects from the DB rather than dropping the row.
`report.source_missing_count` is counted and reported (35746-35747, 35928-35931), and surfaced in
robot output as `"source_missing_count"` (src/lib.rs:44219, 47911).

## 3f. Corroboration from the product's own words

- `cass index --help` (run against the installed 0.6.9 binary): *"--force-rebuild: Force
  **Tantivy index** rebuild even if schema matches"*. The shipped help already says the object of
  the rebuild is the derived lexical index, not the archive.
- src/indexer/mod.rs:12690-12700, the `--full` branch, verbatim:

```rust
        if opts.full && !opened_fresh_for_full && initial_canonical_sessions_before_salvage == 0 {
            // NOTE: We deliberately do NOT reset either SQLite or Tantivy here.
            // An empty canonical archive can be populated in place, and the
            // Tantivy index will be atomically replaced by rebuild_tantivy_from_db()
            // at the end of a successful --full rebuild. Eagerly deleting or
            // replacing anything is both redundant and dangerous if the scan or
            // rebuild OOMs / hits a constraint error. (CASS #164)
        } else if opts.full {
            // Same rationale — skip eager delete_all(); rebuild_tantivy_from_db()
            // handles starting fresh after the scan succeeds.  (CASS #164)
        }
```

- `rebuild_tantivy_from_db_with_options` opens the archive **read-only**
  (src/indexer/mod.rs:18134: `FrankenStorage::open_readonly(db_path)`). The whole lexical rebuild
  family is a DB reader.
- Watch mode deliberately ignores file removal — src/indexer/mod.rs:21605-21606:
  `// Treat remove events as noise until delete-aware rebuilds exist.` followed by
  `notify::event::EventKind::Remove(_) => false,`. cass has **no delete-aware path at all**.
- `doctor_provider_prune_risk` (src/lib.rs:31561-31566) already encodes the bead's premise as
  product knowledge: for `claude_code`, level `"high"`, note *"Claude Code may prune local harness
  logs; cass archive rows can be the durable copy."*

## 4. Existing tests that pin this behavior

### 4a. For `sources agents exclude`: YES — and it already proves source-absent rows are deleted

`tests/e2e_sources.rs:321 sources_agents_exclude_purges_local_archive_data_by_default` seeds two
conversations with `seed_archive_conversation` (tests/e2e_sources.rs:46), whose fixture writes
**only to the DB** and sets `source_path: format!("/tmp/{agent_slug}-{marker}.jsonl")` — a path it
never creates. It then runs `cass sources agents exclude openclaw` and asserts the openclaw
conversation is gone (`conversations.len() == 1`, remaining slug `codex`) and that searching for
its marker returns 0 hits.

So the *existing suite already contains an executed falsifier for the exclude path*, and it pins
the destructive behavior as the intended contract: a conversation whose source file does not
exist is deleted, and the test asserts that it should be. Two golden docs say the same in
operator-facing words — `tests/golden/robot_docs/examples.txt.golden:79`:
`cass sources agents exclude openclaw    # block future indexing, purge archived local data`.

CAVEAT: I read this test's source; I did **not** run it (that needs a full cargo build, out of
scope for a read-only lane). Its current pass/fail state is UNVERIFIED by me.

### 4b. For `--force-rebuild` preserving source-absent rows: NO SUCH TEST EXISTS

The force-rebuild tests pin *search/doc-count stability*, not row survival past a vanished source:

- `tests/e2e_search_index.rs:904 force_rebuild_preserves_search_results_and_reader_surface_during_atomic_publish`
- `tests/e2e_search_index.rs:1247 ..._during_federated_atomic_publish`
- `tests/e2e_search_index.rs:1614 repeated_force_rebuild_preserves_federated_reader_and_search_stability`
- `tests/atomic_swap_publish_crash_window.rs:325` — *"forced federated --force-rebuild on
  unchanged content must preserve the doc count"*

Every one of them is on **unchanged content**. Cross-referencing the two populations:
`rg -l 'remove_file' tests/` returns 9 files (util/e2e_log.rs, recovery/key_slots.rs,
recovery/disaster.rs, fs_errors.rs, pages_preview_integration.rs, pages_error_handling_e2e.rs,
pages_bundle.rs, doctor_mutate_auditor.rs, search_asset_harness.rs) and **none** of them
mentions `force_rebuild`. The only deletion inside `tests/cli_index.rs` is
`fs::remove_dir_all(&index_path)` at line 1597 — wiping the *lexical index directory*, not a
source file.

Adjacent tests that touch source-absence but do not answer this question:
`src/indexer/mod.rs:26808 raw_mirror_capture_handles_deleted_after_discovery_source_without_manifest`
and `src/lib.rs:60396 doctor_source_inventory_counts_missing_sources_without_calling_them_lost`.

**Plainly: nothing pins "a conversation row whose source file is gone survives a rebuild."**

## 5. VERDICT

**`cass index --force-rebuild` is not a data-destruction path for the canonical archive.**
Confidence high, on a census argument rather than a single trace: there are exactly four
`DELETE FROM conversations` statements in the tree (case-insensitive, multi-line-aware:
`rg -n -i -U 'DELETE\s+FROM\s+conversations\b' src/ | wc -l` → 4), three are `#[cfg(test)]`, and
the one reachable statement is the agent purge behind `cass sources agents exclude`. No index,
scan, rebuild, watch, or salvage path can remove a `conversations` row, whatever it decides about
a missing file. Independently, the populated-archive force-rebuild opens SQLite **read-only** and
regenerates only Tantivy from it.

**`cass sources agents exclude <agent>` IS the data-destruction path**, exactly as the bead says,
and the existing test suite pins that as intended behavior. On the live archive today that means
`cass sources agents exclude claude_code` deletes **4,050** conversations, of which the bead's
2026-08-14 measurement says 3,877 have no surviving source file.

Live-archive state at the time of this lane (read-only, cheap):

```
$ sqlite3 "file:$LIVE?mode=ro" "SELECT (SELECT COUNT(*) FROM conversations), (SELECT COUNT(*) FROM agents);"
13371|9
$ sqlite3 "file:$LIVE?mode=ro" "SELECT a.slug, COUNT(c.id) FROM agents a LEFT JOIN conversations c ON c.agent_id=a.id GROUP BY a.slug ORDER BY 2 DESC;"
claude_code|4050
codex|3712
pi_agent|1876
openclaw/feature-dev-developer|1482
openclaw/feature-dev-planner|1392
opencode|764
factory|66
amp|33
cursor|1
```

(`$LIVE` = `/Users/dalecarman/Library/Application Support/com.coding-agent-search.coding-agent-search/agent_search.db`.
`claude_code|4050` matches the bead's measured 4,050 exactly. Total is 13,371 against the bead's
12,722 — the running backfill has added rows since 2026-08-14.)

Residual risk that source-reading cannot close, and that the falsifier below exists to close: a
row can survive in SQLite and still become **unreachable** — dropped from the rebuilt Tantivy
index, or orphaned if its `agents` row goes. That would look like data loss to every operator
even though `SELECT COUNT(*)` is unchanged. This is why the falsifier asserts a **search hit**,
not only a row count.

## 6. FALSIFIER DESIGN (designed, NOT run)

Scope: tiny and synthetic. Two ~1 KB JSONL files in a temp dir. Seconds, not the 40-minute wedge
the previous session hit against a 7.4 GB copy. Nothing under `~/Library/Application Support/` or
`~/backups/cass/` is opened at all.

### 6.0 Isolation (load-bearing — this is what keeps it tiny)

Connectors fall back to HOME-derived default roots when `scan_roots` is empty
(src/indexer/mod.rs:21714), and the operator's configured sources are skipped when
`CASS_IGNORE_SOURCES_CONFIG` is set (src/indexer/mod.rs:11478, 21613, 21718). Without that env
var the run would reach into the real `~/.claude/projects` — 8,008 files, 6 GB — which is
precisely how a "tiny" experiment becomes another wedge.

```bash
# X's must be TRAILING on macOS mktemp, and capture the status: a template with a
# suffix is taken literally and leaves $T empty on the failing second call.
T=$(mktemp -d /tmp/cass-falsifier-XXXXXX) || exit 1
mkdir -p "$T/home/.claude/projects/demo" "$T/config" "$T/data" "$T/moved-away"

# One env block, reused by every command below.
export HOME="$T/home"
export XDG_CONFIG_HOME="$T/config"
export XDG_DATA_HOME="$T/data"
export CASS_DATA_DIR="$T/data"
export CASS_IGNORE_SOURCES_CONFIG=1
export CODING_AGENT_SEARCH_NO_UPDATE_PROMPT=1
DB="$T/data/agent_search.db"        # <data-dir>/agent_search.db (src/lib.rs:16710)

# GUARD before every cass invocation — refuse to run if the env still points at the real archive.
case "$CASS_DATA_DIR" in /tmp/cass-falsifier-*) ;; *) echo "REFUSING: data dir is not the fixture"; exit 2 ;; esac
```

Env-var names verified against `tests/lifecycle_matrix.rs:41-44` (`XDG_DATA_HOME`, `HOME`,
`CASS_IGNORE_SOURCES_CONFIG`) and `tests/e2e_sources.rs:333-334` (`XDG_CONFIG_HOME`,
`CASS_DATA_DIR`). Binary under test: `/Users/dalecarman/.local/bin/cass`, `cass 0.6.9`,
`git commit: 447d97fe` — 4 commits behind this worktree's 74a72233. Use the installed binary,
because the question is about what an operator following the skill would actually run.

### 6.1 Fixture

```bash
cat > "$T/home/.claude/projects/demo/keep.jsonl" <<'EOF'
{"role":"user","content":"FALSIFIERMARKERKEEP alpha"}
{"role":"assistant","content":"FALSIFIERMARKERKEEP beta"}
EOF
cat > "$T/home/.claude/projects/demo/vanish.jsonl" <<'EOF'
{"role":"user","content":"FALSIFIERMARKERVANISH alpha"}
{"role":"assistant","content":"FALSIFIERMARKERVANISH beta"}
EOF
```

Two files, not one: `keep.jsonl` is the internal reference that separates "the vanished row was
dropped" from "the whole archive was emptied".

### 6.2 The probe (read-only, used identically at every step)

```bash
probe() {   # prints:  <total> <keep-rows> <vanish-rows>
  sqlite3 "file:$DB?mode=ro" \
    "SELECT (SELECT COUNT(*) FROM conversations),
            (SELECT COUNT(*) FROM conversations WHERE source_path LIKE '%keep.jsonl'),
            (SELECT COUNT(*) FROM conversations WHERE source_path LIKE '%vanish.jsonl');"
}
```

### 6.3 Baseline, and the NEGATIVE control that stops a vacuous pass

```bash
cass index --full --data-dir "$T/data"
probe            # REQUIRE exactly: 2|1|1
```

If this is not `2|1|1`, **stop**. A fixture that indexed nothing would sail through every arm
below with the same "nothing was dropped" reading. This is the check the previous session's
wedged run did not have: its archive never got past phase `preparing`, so "delta 0" was measuring
an archive that had not been touched.

### 6.4 Make the source vanish — by MOVING, never deleting

```bash
mv "$T/home/.claude/projects/demo/vanish.jsonl" "$T/moved-away/vanish.jsonl"
[ -f "$T/moved-away/vanish.jsonl" ] || { echo "move failed"; exit 2; }
[ ! -e "$T/home/.claude/projects/demo/vanish.jsonl" ] || { echo "source still present"; exit 2; }
```

Both assertions matter: the second is what makes the arm about source-absence at all.

### 6.5 Arm A — bare `--force-rebuild` (the read-only fast path, src/indexer/mod.rs:2091-2160)

```bash
cass index --force-rebuild --data-dir "$T/data" --verbose 2>"$T/armA.stderr"
probe
cass search FALSIFIERMARKERVANISH --json --limit 5
```

- SURVIVES ⇒ probe is `2|1|1` **and** the search returns ≥1 hit.
- DROPPED ⇒ probe is `1|1|0`, or probe holds at `2|1|1` while search returns 0 (the
  survived-in-SQLite-but-unreachable case §5 names).

### 6.6 Arm B — `--full --force-rebuild`, the form the cass skill actually prescribes

Different code path: `!opts.full` in the fast-path gate (src/indexer/mod.rs:2093) sends this arm
through the writable branch with `canonical_only_full_rebuild = true`
(src/indexer/mod.rs:12391-12392). Arm A does not cover it.

```bash
cass index --full --force-rebuild --data-dir "$T/data" --verbose 2>"$T/armB.stderr"
probe; cass search FALSIFIERMARKERVANISH --json --limit 5
```

### 6.7 Arm C — plain `--full` (a real filesystem rescan with the file absent)

```bash
cass index --full --data-dir "$T/data" --verbose 2>"$T/armC.stderr"
probe; cass search FALSIFIERMARKERVANISH --json --limit 5
```

### 6.8 POSITIVE CONTROL P1 — prove the probe can see a real deletion

Without this, "the row survived" and "the command was a no-op" are the same reading. Run it on a
**copy** of the fixture archive so the arms above stay intact:

```bash
cp -a "$T/data" "$T/data-control"
CASS_DATA_DIR="$T/data-control" cass sources agents exclude claude_code   # NO --keep-indexed-data
DB="$T/data-control/agent_search.db" probe    # REQUIRE 0|0|0
```

The known-destructive command on the same DB shape, read by the same probe. If this does not go
to zero, the instrument is dead and **every result above is void**. (This is the control the
previous session lacked.) Guard: the `CASS_DATA_DIR` override must be on the same line as the
command, and re-assert the `/tmp/cass-falsifier-*` case guard immediately before it — this is the
one command in the whole experiment that destroys data, so pointing it at the wrong archive is
the single unrecoverable mistake available here.

### 6.9 POSITIVE CONTROL P2 — prove the rebuild actually ran

A rebuild that short-circuits to "nothing to do" reads exactly like a rebuild that preserved
everything.

```bash
# (a) the run took the force-rebuild path at all:
rg -c 'force_rebuild_uses_readonly_authoritative_canonical_db_rebuild_only|selected_lexical_population_strategy' "$T/armA.stderr"
# (b) the lexical index was really regenerated: capture the index dir's segment
#     file set + mtimes before and after each arm; require it to change.
```

`--verbose` is required and `--json` alone is not enough: robot mode hard-codes the log filter to
`error` and ignores `RUST_LOG` (src/lib.rs:5769-5775), so a bare `--json` run would show an empty
stderr and (a) would read as "the path never ran" when it merely could not print. Before trusting
a zero from (a), prove the instrument can emit a one — run Arm A once on a fixture whose source is
still present and confirm the line appears there.

### 6.10 Cost and safety

Two 1 KB files; the archive is 2 conversations. Every arm should finish in seconds. Disk cost is
kilobytes against the 131 GB free / 150 GB floor. Nothing opens the live archive or
`~/backups/cass/`. The only destructive command in the design is §6.8, and it runs against
`$T/data-control`.

## 7. PREDICTION (so the experiment can refute me)

| arm | prediction | confidence |
|---|---|---|
| A — bare `--force-rebuild` | row SURVIVES; search still hits | 0.95 |
| B — `--full --force-rebuild` | row SURVIVES; search still hits | 0.93 |
| C — plain `--full` | row SURVIVES; search still hits | 0.92 |
| P1 — `sources agents exclude` on the control copy | row is DELETED (probe `0|0|0`) | 0.97 |

Basis: the DELETE census (§1) is exhaustive over the tree, and the one reachable deleter is not on
any of arms A/B/C. Confidence is not 1.0 because reachability was established by reading, and
because the search-hit half of the claim depends on the Tantivy rebuild covering every row rather
than only on SQLite retention.

What would refute me, in order of plausibility: (1) the row survives in SQLite but the rebuilt
lexical index omits it, so search returns 0 — a real operator-visible loss that the row count
alone would hide; (2) an arm errors out rather than completing, which is UNAVAILABLE, not a pass;
(3) plain `--full` interacts with historical salvage or the coverage ledger in a way the census
does not cover.

