<!-- plan:complete:v1 | harness: unknown | date: 2026-05-15T22:37:12Z -->

## Phase A (Research Challenge) — Round 1

**Protocol verdict**: protocol violation — research file contained final plan-selection prose at line 94.

**Action**: removed the offending line, replaced with neutral framing pointing back to spec 015 Selected Shape.


## Phase A (Research Challenge) — Round 1.5 (resubmit)

**Protocol Check**
No Phase A protocol violation found. The context file stays in research/proposed-direction territory; I did not see task IDs, implementation sequencing, final plan prose, or a file-by-file change list.

**Challenge Text**
1. Shape A is not proven viable yet. Current `ScanContext` only scopes by roots, not arbitrary files, and FAD Pi coerces file roots to the parent directory before scanning [pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:258). A workspace-subdir scan may bound memory, but it changes `external_id`: full-root scan uses `workspace/file`, while subdir scan strips only the workspace root and likely returns just `file` [pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:322). That can affect dedupe/upsert/no-data-loss behavior.

2. The temp symlink/copy fallback is riskier than the research frames. Raw-mirror preparse rejects discovered sources under symlink components [src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:17306), and copy/scratch roots would likely store scratch `source_path`s instead of the real `~/.pi/agent/sessions/...` paths. That conflicts with source-path reconciliation and future auditability.

3. Do not generalize the streaming change into non-explicit watch without preserving watermark ordering. Current non-watch-once scans sort the whole `Vec` before chunking [src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:16269), and per-chunk watermark saves rely on that ordering [src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:16372). A streaming flush can advance `watch_state` past older unprocessed conversations after a crash.

4. The PR #233 default is misstated. The research says the persist chunk default is 8, but current code says `WATCH_INGEST_DEFAULT_CHUNK_SIZE = 32` and max 512 [src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:89). The plan should use current-tree behavior, not inherited notes.

5. Acceptance #1 needs skipped-file accounting beyond ingest OOM. FAD Pi silently skips unreadable files, malformed lines, empty messages, and empty sessions [pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:351). Existing quarantine only records single-conversation ingest OOM [src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:15717). Shape A must reconcile discovered source paths against emitted/persisted/quarantined paths.

6. Acceptance #5 is still load-bearing. Any cass-local parser or external-id repair must compare against the pinned FAD Pi parser, not a hand-rolled line counter. FAD counts non-empty flattened `message.content`, including `thinking` and `toolCall` blocks [pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:143).

7. Count-based scan chunking is too weak by itself. The existing streaming sender gates by conversations, messages, and chars [src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:8355). Pi still reads a whole file and clones raw JSON into every message extra [pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:351), and current extra compaction is codex-only [src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:17573). The scan chunk knob should consider bytes/messages, not only file count.

8. A cleaner third route may be a cass-local Pi batch/stream wrapper, not temp roots. It can enumerate with FAD `discover_source_files`, parse original files in bounded batches, preserve canonical `source_path` and `external_id`, and prove equivalence against FAD fixtures. The downside is parser duplication, so it needs stronger message-coverage tests.

9. Cross-spec framing is mostly correct: spec 014’s absolute 8 GB RSS gate is out of scope [spec.md](/Users/dalecarman/dev/coding_agent_session_search/specs/015-watch-once-streaming-scan/spec.md:50). The hidden creep risk is treating “sub-1-GB delta” as an absolute memory promise instead of `peak RSS - steady FTS5 floor` [spec.md](/Users/dalecarman/dev/coding_agent_session_search/specs/015-watch-once-streaming-scan/spec.md:56).

**What I Verified**
I read the research file, spec 015, the watch-once ingest path, the watermark helpers, the FAD Connector trait, and the pinned FAD Pi scanner. I also checked repo preflight; `br ready` failed under this sandbox with `Operation not permitted`, and process-list checks for the claimed watcher PID were blocked, so I did not verify live daemon state.

## Phase A Round 1 — refinements made by Claude before Round 2:
- Removed plan-selection prose.
- Corrected PR #233 default chunk size: 8 → WATCH_INGEST_DEFAULT_CHUNK_SIZE = 32 (verified src/indexer/mod.rs:89).
- Acknowledged Shape A external_id risk (FAD pi_agent.rs:258-262 + :322-348).
- Acknowledged temp-symlink fallback rejected by raw-mirror preparse (src/indexer/mod.rs:17306-17317).
- Added Route 4 (cass-local pi batch wrapper preserving canonical paths).
- Added watermark-ordering risk to open questions.
- Added skipped-file reconciliation requirement (Acceptance #1).
- Added message-coverage harness requirement (Acceptance #5, FAD-pinned parser).
- Noted pi extras are NOT compacted today (compactor is codex-only at :17573); the chunk-size knob must reflect this.
- Removed the now-stale 'compact + raw-mirror invariants' open question.

## Phase A (Research Challenge) — Round 2

Material gaps remain before locking the research.

1. Route 4 is framed too softly. A cass-local Pi parser is not a “thin batch wrapper”; it is a second implementation of FAD’s Pi normalization contract. Today cass’s Pi connector is only a re-export ([src/connectors/pi_agent.rs](/Users/dalecarman/dev/coding_agent_session_search/src/connectors/pi_agent.rs:1)), and FAD’s parser owns content flattening, roles, timestamps, invocations, metadata, `external_id`, and `source_path` ([pi_agent.rs](</Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:376>), [pi_agent.rs](</Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:479>), [pi_agent.rs](</Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:540>)). Message-count parity is necessary but not enough to justify this route; counts can match while searchable content, invocations, timestamps, or metadata drift. If Route 4 stays in the research, it should be labeled “cass-owned Pi connector/parser replacement,” not a wrapper.

2. The Route 4 harness wording is still wrong for Acceptance #5. The revised file says “spot-check conversation” and “runs pinned FAD `PiAgentConnector::scan()` on each file.” But FAD explicitly coerces file roots to the parent directory ([pi_agent.rs](</Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:258>)), so “scan on each file” does not mean one-file oracle. The structural check should be: run pinned FAD on the same full/root-scoped corpus, build `external_id/source_path -> message_count`, then compare DB `messages` row counts for those conversations. Spot checks can be a receipt tactic, but the acceptance contract should not be phrased as a hand-picked conversation check.

3. The non-explicit-watch watermark framing is materially unsafe. Sorting within each flushed batch does not preserve the existing global sort invariant from [src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:15674). A later streamed batch can still contain an older payload timestamp after an earlier batch has advanced `watch_state` at [src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:16372). Given Acceptance #4 says forward-capture watcher behavior is unchanged, the research should make explicit-watch-once gating mandatory for this spec, not one option among “gate or sort within batch.”

4. Chunk-size composition is still under-specified. `CASS_WATCH_INGEST_CHUNK_SIZE` is a persist knob with default 32 ([src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:89), [src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:15881)). Reusing that as a scan/file knob would not satisfy the sub-1-GB delta if one Pi file can plausibly inflate to ~250 MB. The research should say the scan-side limiter needs its own byte/message/file gates, and the persist knob remains nested inside it.

5. There is a missed cass-side blast-radius surface around progress and stats. The current path sets `progress.total` only after the full `convs` Vec exists ([src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:16273)), derives connector stats from that full Vec after ingest ([src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:16380)), and returns `total_indexed` only after the full trigger finishes ([src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:16415)). A streaming plan needs an explicit replacement accounting model. This is not just UI polish; stale detection and run receipts depend on these counters.

6. Route 3 is still accidentally reintroduced. The file rejects temp symlinks as non-viable, but later says Phase B chooses among Routes 1 / 3 / 4. That is not a negative-contract violation, but it is a coherence bug. Route 3 should stay dead unless the research has new evidence.

7. Cross-spec scope is a little muddy. Route 4 imports the shape of spec 014’s C4 cass-owned parser work, but spec 015’s selected Shape A is “discover_source_files + per-batch scan()” and its constraint says FAD trait changes are out of scope. A cass-owned Pi parser may still be cass-side, but it is no longer simply implementing Shape A as written. The research should call that scope escalation out plainly.

What I verified: I reread the revised research file, spec 015, spec 014, the pinned FAD trait and Pi connector, and the watch-once ingest path. Negative contract mostly holds: I did not find T1/T2-style task breakdowns or file-by-file change lists. `br ready` could not run in this read-only sandbox: `Operation not permitted`.

## Phase A Round 2 — refinements made by Claude before Round 3:
- Route 4 reframed: not a 'wrapper' but a full cass-owned Pi connector/parser replacement; cross-spec scope deviation called out explicitly.
- Route 4 harness wording corrected: structural full-root scan + map comparison, not per-file scan (FAD coerces file roots).
- Watermark framing strengthened: streaming change MUST be gated to explicit_watch_once only (no 'sort within batch' alternative).
- Chunk-size composition specified: separate scan-side limiter with byte/message/file gates; persist knob nested inside.
- Progress/stats accounting added as a load-bearing concern.
- Route 3 marked REJECTED inline (no more 'choose among 1/3/4' coherence bug).
- Cross-spec scope tension surfaced explicitly: cass-side-only constraint may be too strict given Route 1's external_id risk and Route 4's parser-duplication risk.

## Phase A (Research Challenge) — Round 3

Using `adversarial-review` posture.

Material gaps remain before locking research.

1. The revised file still contains the unsafe watermark option it says was removed. Line 106 correctly says sorting within a flushed batch is unsafe, but line 119 still says Phase B may choose “sort within each flushed batch.” That contradiction is blocking because it reopens the exact invalid option Round 2 supposedly killed.

2. The cross-spec tension framing is not complete. It currently implies Phase B must choose among Route 1 identity risk, Route 2 FAD change, Route 4 parser duplication, or escalate. I do not think that is proven. There is a concrete cass-only route still missing: build a per-batch canonical scratch Pi root using hardlinks or copies, not symlinks, preserving `sessions/<workspace>/<file>.jsonl`, call FAD `PiAgentConnector::scan()` against that scratch root, then remap emitted `source_path` back to the original path before cass persistence/raw-mirror linkage.

Evidence: FAD `ScanContext` only accepts directory roots, not files (`scan.rs:131-140`, `:154-165`). Pi derives `sessions_dir` as `home/sessions` when present (`pi_agent.rs:74-80`), walks only that root (`:84-105`), and computes `external_id` relative to `sessions_dir` (`:340-348`). If the scratch tree preserves the canonical relative shape, `external_id` stays canonical without reimplementing parsing. FAD still does all parsing/normalization (`pi_agent.rs:305-555`). The symlink objection only applies to symlink components (`src/indexer/mod.rs:17306-17317`); hardlinks/copies are a different route.

That route has real trade-offs: temp IO, cleanup, source-path remapping, raw-mirror capture ordering, and hardlink fallback when cross-device. But it means the research cannot yet claim the spec’s cass-side-only constraint must be relaxed. Add this as a candidate route or reject it with evidence.

3. Progress/stats framing is closer but still under-specified. It names the three affected sites, but Phase B still needs a denominator decision: is `progress.total/current` counting discovered files, emitted conversations, persisted conversations, or all accounted files including skipped/quarantined? Current watch-once uses full `convs.len()` before ingest (`src/indexer/mod.rs:16273-16277`) and increments progress during ingest (`:15538-15540`). Existing streaming accumulates totals as batches arrive (`:8836-8837`, `:8864-8867`) and writes final stats at `:9066-9075`. For this spec, because skipped files are acceptance-relevant, research should explicitly call out an accounting model with discovered/emitted/persisted/quarantined/unaccounted counts. Otherwise Phase B can easily produce a progress bar that looks right while the receipt is wrong.

4. Negative contract: I did not see task IDs or a file-by-file change list. The “Direction I’m considering” section is assertive, but still reads as research route framing rather than final `plan.md` sequencing. Not my main concern.

What I verified: revised research file lines 83-125, spec lines 36-59 and 69-78, FAD connector trait lines 62-104, FAD `ScanContext` lines 131-180, FAD Pi parser/root/external-id code at `pi_agent.rs:74-105`, `:258-262`, `:305-348`, `:540-557`, cass watch-once ingest/progress at `src/indexer/mod.rs:16248-16440`, raw-mirror symlink validation at `:17201-17317`, and existing streaming stats at `:8780-9075`.

So: revisions are improved, but not sufficient to lock research. The missing hardlink/copy scratch-root route and the stale watermark open-question are the blockers.

## Phase A Round 3 — refinements made by Claude before Round 4:
- Added Route 5: per-batch hardlink/copy scratch root with source_path remap. Preserves canonical external_id (FAD sees sessions/<workspace>/<file>) without sub-ScanContext, parser duplication, or FAD changes. Symlink rejection at :17306-17317 does NOT apply (hardlinks/copies are regular files).
- Resolved Watermark Ordering open question: streaming gated to explicit_watch_once only is the only safe option; sort-within-batch eliminated. No more 'or' wording.
- Specified progress/stats accounting denominators: progress.total = discovered file count, progress.current advances per persisted conversation, stats accumulated across flushed batches (reusing :8780-9075 pattern), total_indexed summed from per-batch outcomes, run receipt requires discovered/emitted/persisted/quarantined/unaccounted breakdown.
- Updated route ranking: Route 5 is now the likely Shape A baseline; Routes 1/3/4 stay as documented non-preferred options.
- Cross-spec tension framing softened: Route 5 may resolve the tension cleanly, but Phase B should verify.

## Phase A (Research Challenge) — Round 4

**Verdict:** not quite ready to lock. Route 5’s `external_id` equivalence claim holds, but the raw-mirror interaction is still materially under-specified. The progress/stats model is close, but one counter formula is currently wrong enough to cause receipt drift.

**Challenge Text**
1. Route 5 external-id equivalence: holds, with a condition. FAD pi resolves `sessions_dir(home)` as `home/sessions` when it exists, otherwise `home` ([pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:74)). During scan it hoists that `sessions_dir`, then computes `external_id` from `source_path.strip_prefix(&sessions_dir)` ([pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:322), [pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:340)). So:
   `~/.pi/agent/sessions/<workspace>/<file>` and `<scratch>/sessions/<workspace>/<file>` both produce `<workspace>/<file>`.
   Caveat: Phase B must pass `<scratch>` or `<scratch>/sessions`, not `<scratch>/sessions/<workspace>`.

2. Raw-mirror: still a material gap. Current watch-once calls `capture_connector_sources_before_parse` before `scan()` ([mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:16237)). That helper calls `discover_source_files(ctx)` and captures `source.source_path` directly ([mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:17209), [mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:17252)). Under a scratch ctx, FAD discovery returns scratch paths ([pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:272)). Remapping emitted `NormalizedConversation.source_path` later only fixes the post-parse capture path ([mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:17429)). It does not prevent a scratch-path preparse manifest. Raw mirror manifest identity includes original path hash, so identical bytes at scratch vs original become distinct manifests ([raw_mirror.rs](/Users/dalecarman/dev/coding_agent_session_search/src/raw_mirror.rs:844), [raw_mirror.rs](/Users/dalecarman/dev/coding_agent_session_search/src/raw_mirror.rs:2000)). Phase B needs an explicit route: pre-capture originals and skip scratch preparse capture, or add remap-aware preparse capture.

3. Progress/stats: not quite exact yet. The research says `total_indexed = processed_conversations + quarantined_conversations`, but current `WatchIngestBatchOutcome.processed_conversations` already includes the single-conversation quarantine case ([mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:15656)). Current `total_indexed` adds only `processed_conversations` ([mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:16415)). Adding quarantined separately would double-count quarantined OOM files. Also define whether `messages` means emitted messages or inserted messages: current watch stats use `inserted_messages` ([mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:16389)), while streaming stats use pre-persist message counts ([mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:8836)).

4. Watermark issue looks resolved. Gating to `explicit_watch_once` composes with the existing `!explicit_watch_once` watermark save guard ([mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:16372)).

**What I Verified**
Read the revised research, FAD pi connector root/external-id code, cass watch-once scan/persist path, raw-mirror preparse/post-parse capture, raw-mirror manifest identity, and current watch ingest counters. No files changed.

## Phase A Round 4 — refinements made by Claude before Round 5:
- Route 5 external-id equivalence: verified and documented (FAD strip_prefix(sessions_dir); scratch shape preserves the relative path; root must be <scratch> or <scratch>/sessions, not deeper).
- Route 5 raw-mirror gap closed: plan must run preparse capture against ORIGINAL ~/.pi/agent/sessions root once, NOT under scratch ctx (preparse capture uses discover_source_files which returns scratch paths under scratch ctx, breaking manifest identity).
- Progress/stats counter fixed: total_indexed accumulates processed_conversations ONLY (already includes single-conv quarantine cases at :15656); adding quarantined separately would double-count. Run receipt counters relabelled and given exact formulas.
- 'messages' field semantics resolved: use inserted_messages (post-persist) to match existing watch-mode :16389; quarantined contribute 0, which is correct.

## Phase A (Research Challenge) — Round 5

Not quite locked yet.

Route 5 is now structurally sound: original-root preparse avoids the raw-mirror manifest fork, and the scratch-root external-id reasoning matches FAD’s `sessions_dir()` + `strip_prefix()` behavior. That does not appear to break another invariant, as long as Phase B makes two requirements explicit: no scratch-context preparse capture, and remap `conversation.source_path` back to the original path before post-parse raw-mirror capture/persist.

The remaining material issue is the run-receipt counter formula in `/tmp/codex-plan-research-afc9c9d9.md:124`:

`unaccounted = discovered - emitted_by_scan - quarantined_oom - persisted_to_db`

That is still wrong. `discovered` is files, `emitted_by_scan` is conversations, and `persisted_to_db` / `quarantined_oom` are outcomes of emitted conversations. It also double-subtracts successful conversations. Example: `2073 discovered`, `1970 emitted`, `1970 persisted`, `0 quarantined` gives `-1867`.

Use split accounting instead:

`parse_unaccounted_files = discovered_files - emitted_source_files - parser_skip_records`

`ingest_unaccounted_conversations = emitted_conversations - ingest_success_conversations - quarantined_oom`

Also rename `persisted_to_db` if it means `processed_conversations - quarantined_conversations`; that is not “rows actually committed” on idempotent re-runs. It is better called `ingest_success_conversations`.

**What I Verified**

I checked the revised research file, `src/indexer/mod.rs:15552-15661`, `:16237-16419`, raw-mirror capture at `:17201-17440`, raw-mirror manifest identity in `src/raw_mirror.rs:844-853`, and FAD Pi connector behavior at `pi_agent.rs:74-105`, `:272-350`, `:508-550`, `:557-558`.

Bottom line: raw mirror and `total_indexed` are fixed enough. Counter formulas are not yet lockable because the receipt reconciliation is still mathematically ambiguous/wrong.

## Phase A Round 5 — refinements made by Claude before Round 6:
- Receipt counter formula split into file-level and conversation-level accounting (units must not mix).
- Renamed persisted_to_db → ingest_success_conversations (per Codex; not 'rows committed' on idempotent re-runs).
- Added parse_unaccounted_files = discovered - emitted - parser_skip_records (file-level).
- Added ingest_unaccounted_conversations = emitted - ingest_success - quarantined_oom (conversation-level).
- Both must be 0 at receipt time; Acceptance #1 maps to ingest_success_conversations >= 1970.

## Phase A (Research Challenge) — Round 6

Not lockable yet.

The revised receipt model in [codex-plan-research-afc9c9d9.md](/tmp/codex-plan-research-afc9c9d9.md:124) is materially correct: file-level and conversation-level counters are split, and `ingest_success_conversations = processed_conversations - quarantined_conversations` matches [mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:15553), where single-conversation OOM sets both `processed_conversations: 1` and `quarantined_conversations: 1`.

Material gap: two stale formulas still remain and contradict the corrected section:

- [codex-plan-research-afc9c9d9.md](/tmp/codex-plan-research-afc9c9d9.md:116) still says `discovered files - emitted conversations - persisted conversations - quarantined`.
- [codex-plan-research-afc9c9d9.md](/tmp/codex-plan-research-afc9c9d9.md:137) still says `discovered files - emitted conversations - quarantined`.

Those are the same units-mismatch problem Round 5 caught. Replace both with the split file/conversation formulas before lock. After that, I don’t see a remaining material blocker in the Round 6 revisions.

## Phase A Round 6 — refinements made by Claude before Round 7:
- Cleaned up the two stale references to the old broken formula (lines 116 and 137 in earlier draft).
- All formula text now points back to the canonical split-formula block under 'Progress and stats accounting'.

## Phase A (Research Challenge) — Round 7 — LOCKED

research is lockable.

**Findings**
No material gaps remain. The old broken single receipt formula is gone; the research now uses split file-level and conversation-level accounting, and explicitly calls the old one-formula version wrong at [context](/tmp/codex-plan-research-afc9c9d9.md:124) and [context](/tmp/codex-plan-research-afc9c9d9.md:137).

**What I Verified**
I checked the revised receipt definitions at [context](/tmp/codex-plan-research-afc9c9d9.md:125) and [context](/tmp/codex-plan-research-afc9c9d9.md:126) against current code: OOM quarantine returns `processed_conversations: 1` and `quarantined_conversations: 1` at [src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:15656), while `total_indexed` adds only `processed_conversations` at [src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:16419). I also verified the spec’s acceptance target at [spec.md](/Users/dalecarman/dev/coding_agent_session_search/specs/015-watch-once-streaming-scan/spec.md:55).

Two non-blocking wording cleanups can be handled while drafting `plan.md`: make the receipt terms route-neutral instead of saying `scan_with_callback` if Route 5 is chosen, and replace “metadata-only” for raw-mirror preparse because `capture_source_file` copies/hashes source bytes. Neither changes the research conclusion.

`br ready` could not run in this read-only sandbox: `Operation not permitted`.



## Phase A — locked after 7 rounds.
Verdict: research is lockable. Two non-blocking wording notes for plan.md drafting:
1. Receipt terms route-neutral (don't say scan_with_callback if Route 5 is the chosen Shape A).
2. Replace 'metadata-only' framing for raw-mirror preparse (capture_source_file copies/hashes source bytes).

## Phase B (Plan Draft Review) — Round 5 — APPROVED

No blocking issues found.

The Round 4 fixes are present:
- The three stale gate locations now use `explicit_watch_once && kind.slug() == "pi_agent" && !discovered.is_empty()` at draft lines 124, 165, and 358.
- The skipped-file requirement is now explicit at lines 319-338 and implemented as T9a at lines 417-419, with per-path JSONL records in `<data_dir>/quarantine/watch_ingest_poison.jsonl`, not just aggregate receipt counts.

**What I Verified**
- Re-read `/tmp/codex-plan-draft-afc9c9d9.md` against `specs/015-watch-once-streaming-scan/spec.md`.
- Checked spec Acceptance #1 at `spec.md:55`: skipped files must be recorded in the quarantine JSONL and reflected in the receipt.
- Checked existing OOM quarantine code at `src/indexer/mod.rs:15717-15753`; the plan correctly extends the same file.
- Checked FAD pi path/external-id assumptions at pinned `pi_agent.rs:74-80`, `272-295`, and `340-349`.
- Confirmed `DiscoveredSourceFile.size_bytes` exists at `scan.rs:205-217`, so the scan-byte batch limit is real.

Minor implementation note only: the draft’s `uuid::Uuid::new_v4()` helper detail will need either a direct dependency or, better, reuse existing `tempfile::Builder`; not a plan blocker.

VERDICT: APPROVED

## Phase B — Boundary SHA: 52e96656 — Snapshot: boundary-B-52e96656-06607ecc.md
## Phase B — North-Star Check — Round 1 — BOOTSTRAP
