---
title: "Tasks: Full upstream sync"
date: 2026-03-27
bead: coding_agent_session_search-3iqk
---

<!-- plan:complete:v1 | harness: pi/claude-sonnet-4-6 | date: 2026-03-27T16:44:26Z -->

# Tasks — Spec 008: Full Upstream Sync

## Phase 1: Prepare the merge environment

- [ ] **T1.1** Verify the git worktree at `/tmp/cass-merge-base` is clean and at origin/main
  ```bash
  cd /tmp/cass-merge-base && git log --oneline -1  # should be 81f25604
  ```
- [ ] **T1.2** Add upstream remote to the worktree and fetch
  ```bash
  git remote add upstream https://github.com/Dicklesworthstone/coding_agent_session_search.git
  git fetch upstream
  ```
- [ ] **T1.3** Create the sync branch from origin/main in the worktree
  ```bash
  git checkout -b feat/008-upstream-sync
  ```
- [ ] **T1.4** Save our unique diffs from the main repo before merging
  ```bash
  cd ~/dev/coding_agent_session_search
  git diff origin/main HEAD -- src/watchdog.rs > /tmp/cass-our-diffs/watchdog.patch
  git diff origin/main HEAD -- src/connectors/codebuff.rs > /tmp/cass-our-diffs/codebuff.patch
  git diff origin/main HEAD -- src/indexer/mod.rs > /tmp/cass-our-diffs/indexer-sigterm.patch
  # Verify patches are non-empty
  wc -l /tmp/cass-our-diffs/*.patch
  ```

## Phase 2: Execute the merge

- [ ] **T2.1** Run the merge from the worktree
  ```bash
  cd /tmp/cass-merge-base && git merge upstream/main
  ```
- [ ] **T2.2** Resolve Cargo.toml conflict — keep git dep strategy
  - Take upstream's deps as the base
  - Convert every `path = "../..."` dep to its `git =` equivalent (see plan.md §A2 table)
  - Bump ftui revs: `18facd3` → `7a91089`
  - Bump frankensearch rev: `8b319e9` → `3eec663`
  - Bump franken-agent-detection to include "crush" feature
  - Add: `toon`, `portable-pty` (upstream's new deps)
  - Remove: `[patch."https://github.com/Dicklesworthstone/asupersync"]` section
  - Verify: `cargo metadata --no-deps` exits 0 after resolution
- [ ] **T2.3** Resolve `src/connectors/mod.rs` conflict
  - Take upstream's 59-line version entirely
  - Do NOT re-apply our PathTrie implementation (it's in FAD now)
  - Do NOT add count_disk_files here (goes in src/doctor.rs — see Phase 3)
- [ ] **T2.4** Resolve `src/indexer/mod.rs` conflict
  - Take upstream's version as base
  - Check if upstream already added SIGTERM/shutdown logic (grep for "signal_hook\|SIGTERM")
  - If not present: apply the SIGTERM/heartbeat patch (`git apply /tmp/cass-our-diffs/indexer-sigterm.patch`)
  - If apply fails due to context mismatch: manually insert the ~20 lines from the patch
- [ ] **T2.5** Resolve `src/lib.rs` conflict — take upstream's version entirely
- [ ] **T2.6** Resolve `src/storage/sqlite.rs` conflict — take upstream's version entirely
- [ ] **T2.7** Resolve all remaining conflicts — take upstream's version for every other file
  ```bash
  git diff --name-only --diff-filter=U  # list unresolved files
  # For each: git checkout upstream/main -- <file>
  ```
- [ ] **T2.8** Stage and commit the merge
  ```bash
  git add -A && git commit -m "chore: merge upstream/main v0.2.4 into feat/008-upstream-sync"
  ```

## Phase 3: Re-apply our unique additions

- [ ] **T3.1** Add watchdog.rs (new file)
  ```bash
  cd /tmp/cass-merge-base
  git apply /tmp/cass-our-diffs/watchdog.patch
  # Or: copy directly: cp ~/dev/coding_agent_session_search/src/watchdog.rs src/
  ```
- [ ] **T3.2** Add codebuff.rs connector (new file)
  ```bash
  git apply /tmp/cass-our-diffs/codebuff.patch
  # Register in src/connectors/mod.rs: add `pub mod codebuff;`
  # Register in indexer/mod.rs connector list if not already present
  ```
- [ ] **T3.3** Create `src/doctor.rs` with DoctorConnector extension trait
  ```rust
  // src/doctor.rs
  /// Extension trait for connectors that support disk-vs-DB reconciliation.
  /// Separate from Connector (which lives in FAD) to avoid upstream divergence.
  pub trait DoctorConnector {
      fn count_disk_files(&self) -> Option<usize>;
      fn reconciliation_notes(&self) -> Option<String> { None }
  }
  // Implement for CodebuffConnector here or in codebuff.rs
  ```
- [ ] **T3.4** Update the doctor subcommand in lib.rs to use `DoctorConnector` instead of
  looking for `count_disk_files` on the base `Connector` trait. Connectors that don't
  implement `DoctorConnector` show "N/A" in the doctor output.
- [ ] **T3.5** Add `pub mod doctor;` to `src/lib.rs` or `src/main.rs`
- [ ] **T3.6** Commit additions
  ```bash
  git add -A && git commit -m "feat: re-apply watchdog, codebuff, doctor extension trait"
  ```

## Phase 4: Build verification

- [ ] **T4.1** Verify Cargo.toml resolves cleanly
  ```bash
  cargo metadata --no-deps 2>&1 | head -5
  ```
- [ ] **T4.2** Check compilation
  ```bash
  cargo check --all-targets 2>&1 | head -50
  ```
- [ ] **T4.3** Fix any compilation errors from:
  - Missing features or API changes in frankensqlite/frankensearch/frankentui
  - fad_adapter references in indexer or lib.rs (should be gone after merge, but verify)
  - DoctorConnector wiring issues
  - codebuff connector registration
- [ ] **T4.4** Full release build
  ```bash
  cargo build --release 2>&1 | tail -20
  ```
- [ ] **T4.5** Verify binary runs
  ```bash
  ./target/release/cass --version
  ./target/release/cass health --json 2>/dev/null | python3 -c "import sys,json; d=json.load(sys.stdin); print('healthy:', d['healthy'])"
  ```

## Phase 5: Database migration (8.8GB live DB)

- [ ] **T5.1** Take mandatory pre-migration backup
  ```bash
  DB="$HOME/Library/Application Support/com.coding-agent-search.coding-agent-search/agent_search.db"
  sqlite3 "$DB" "VACUUM INTO '${DB%.db}-backup-pre-v14.db'"
  echo "Backup size: $(du -h "${DB%.db}-backup-pre-v14.db")"
  ```
- [ ] **T5.2** Inspect what columns our live conversations table currently has
  ```bash
  sqlite3 "$DB" "PRAGMA table_info(conversations);" | awk '{print $2, $3}'
  ```
- [ ] **T5.3** Run surgical gap-fill to add columns that MIGRATION_FRESH_SCHEMA assumes
  (SQLite ALTER TABLE ADD COLUMN is idempotent — wrap in shell to skip duplicates):
  ```bash
  for col_sql in \
    "ALTER TABLE conversations ADD COLUMN metadata_bin BLOB" \
    "ALTER TABLE conversations ADD COLUMN total_input_tokens INTEGER" \
    "ALTER TABLE conversations ADD COLUMN total_output_tokens INTEGER" \
    "ALTER TABLE conversations ADD COLUMN total_cache_read_tokens INTEGER" \
    "ALTER TABLE conversations ADD COLUMN total_cache_creation_tokens INTEGER" \
    "ALTER TABLE conversations ADD COLUMN grand_total_tokens INTEGER" \
    "ALTER TABLE conversations ADD COLUMN estimated_cost_usd REAL" \
    "ALTER TABLE conversations ADD COLUMN primary_model TEXT" \
    "ALTER TABLE messages ADD COLUMN extra_bin BLOB"; do
    sqlite3 "$DB" "$col_sql" 2>/dev/null && echo "OK: $col_sql" || echo "SKIP (exists): $col_sql"
  done
  ```
- [ ] **T5.4** Run the migration by starting the new binary once
  ```bash
  ./target/release/cass health --json 2>/dev/null | python3 -c "
  import sys, json; d = json.load(sys.stdin)
  print('healthy:', d['healthy'])
  print('conversations:', d['state']['database']['conversations'])
  "
  ```
  The startup triggers `transition_from_meta_version()` + MigrationRunner (V13, V14).
- [ ] **T5.5** Verify conversation count is preserved (should still be ≥24,340)
  ```bash
  sqlite3 "$DB" "SELECT COUNT(*) FROM conversations;"
  ```
- [ ] **T5.6** Verify new schema tables exist
  ```bash
  sqlite3 "$DB" ".tables" | tr ' ' '\n' | sort
  # Should include: token_usage, message_metrics, embedding_jobs, etc.
  ```
- [ ] **T5.7** **Fallback (if migration fails):** Restore from backup and run full rebuild
  ```bash
  cp "${DB%.db}-backup-pre-v14.db" "$DB"
  ./target/release/cass index --full --force-rebuild
  ```

## Phase 6: Integration verification

- [ ] **T6.1** Watcher starts correctly
  ```bash
  launchctl unload ~/Library/LaunchAgents/com.cass.index-watch.plist
  launchctl load ~/Library/LaunchAgents/com.cass.index-watch.plist
  sleep 5 && pgrep -af "cass index --watch" && echo "watcher running"
  ```
- [ ] **T6.2** Watchdog subcommand works
  ```bash
  ./target/release/cass watchdog run
  ./target/release/cass watchdog install --dry-run
  ```
- [ ] **T6.3** Search returns results
  ```bash
  ./target/release/cass search "authentication" --robot --limit 5
  ```
- [ ] **T6.4** Codebuff connector detected (if Codebuff sessions exist)
  ```bash
  ./target/release/cass stats --json | python3 -c "
  import sys, json; d = json.load(sys.stdin)
  agents = [a['agent'] for a in d.get('by_agent', [])]
  print('codebuff present:', any('codebuff' in a for a in agents))
  "
  ```
- [ ] **T6.5** Run existing test suite
  ```bash
  cargo test 2>&1 | tail -30
  ```

## Phase 7: Deploy and clean up

- [ ] **T7.1** Install the new binary
  ```bash
  cargo install --path . --quiet
  ln -sf ~/.cargo/bin/cass ~/.local/bin/cass  # verify symlink
  ```
- [ ] **T7.2** Push sync branch to origin
  ```bash
  git push origin feat/008-upstream-sync
  ```
- [ ] **T7.3** Update Cargo.toml version to match upstream (0.2.4) or bump to 0.2.5
- [ ] **T7.4** Close bead
  ```bash
  br close coding_agent_session_search-3iqk --reason="Upstream sync complete: v0.2.4 merged, watchdog/codebuff/doctor preserved"
  br sync --flush-only
  ```
- [ ] **T7.5** Clean up worktree
  ```bash
  cd ~/dev/coding_agent_session_search && git worktree remove /tmp/cass-merge-base
  ```
- [ ] **T7.6** Update napkin.md with merge learnings
  - Document that fad_adapter pattern was replaced by native FAD connectors
  - Document the gap-fill migration pattern for future schema upgrades
  - Document the git object corruption workaround

## Dependency graph

```
T1 → T2 → T3 → T4 → T5 → T6 → T7
          T3.3 requires T3.4 (doctor.rs before lib.rs wiring)
          T4.3 may require iteration (fix compile errors)
          T5 requires T4.4 (must have release binary)
          T7.1 requires T6 (don't install until verified)
```

## Estimated effort

- Phase 1-2 (setup + merge): 1-2 hours (merge conflict resolution is the main work)
- Phase 3 (re-apply additions): 30-60 minutes
- Phase 4 (build verification): 30-90 minutes (compile errors expected; frankensqlite API changes)
- Phase 5 (DB migration): 30-60 minutes including verification
- Phase 6-7 (integration + deploy): 30 minutes

Total: ~4-6 hours, with most time in Phases 2 and 4.
