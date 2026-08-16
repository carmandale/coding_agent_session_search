# Lane: gen5-promote-gate

Read-only survey lane. Bead `coding_agent_session_search-sgvg3`.

## The question

The bead claims the doctor promotion coverage gate fails OPEN when the archive is
unreadable, and left one thing explicitly unverified:

> NOT verified: whether the separate `archive_db_unreadable` critical finding raised at
> src/lib.rs:34630-34641 blocks the promotion further downstream. If it does, the
> practical severity drops and this becomes a defence-in-depth fix rather than a live
> hole.

Settle that. Verify the asserted chain hop by hop, trace `archive_db_unreadable` to
every consumer, find the function that actually performs the promotion and read its
guards, and confirm the positive control the bead cites.

## Method, and one thing that went wrong with it

Read-only. `rg` and ranged reads only. No build, no test, no `cass`, no `br`, no
archive access, no edits. One file written: this log.

**Specimen drift, caught mid-lane.** `src/lib.rs` was 91,859 lines when the lane
started and 91,940 lines partway through — another session is editing this same
worktree live (`git diff --stat` = 1 file, 81 insertions; the only hunk headers are
`@@ -15311,0 +15312,30 @@` and `@@ -15321,0 +15352,51 @@`, i.e. the `probe_state_db`
work, nowhere near the doctor code). Everything after line 15321 shifted by exactly
+81. I caught it when one `rg` returned 36512 for a line I had read at 36431
(36431 + 81 = 36512).

So I re-pinned **every** citation below to the immutable base blob
`git show 6bcc51b7:src/lib.rs`, and re-derived each line number from that blob rather
than from the worktree. **All line numbers in this log are base-commit 6bcc51b7 line
numbers.** In the current worktree, add 81 to any number above 15321.

Because the git blob cannot be opened with the file reader, ranged reads of the blob
were done with `git show <sha>:src/lib.rs | awk 'NR>=a && NR<=b'`. No `rg -n` was ever
piped into another `rg -n`; no discovery search was truncated with `head`.

## Findings

### 1. The chain the bead asserts — six of seven hops are real, one line number drifted, one hop is narrower than claimed

| hop | bead's claim | verdict |
|---|---|---|
| `Err(err)` arm → `Vec::new()` | :32312 / :32316 | **as described** |
| `total_indexed_conversations = 0` | :32154 | **as described** |
| `coverage_summary.archive_conversation_count` | :36328 | **as described** |
| gate baseline | :36372-36378 | **as described, but only the conversation half is affected — see below** |
| neither blocking branch fires | :36424-36440 | **as described** for the conversation arm |
| `promote_allowed = blocking_reasons.is_empty()` | :36471 | **line drifted — it is :36473** |
| lifecycle_status `"completed"` | :38752-38763 | **as described** (the `if` runs :38752-38764) |

- **:32312-32318** — the `Err` arm calls `build_doctor_source_inventory_report(data_dir,
  true, Some(err.message), Vec::new(), detected_roots)`. Note the second argument:
  `db_available` is passed **`true`** on the failure path, and the error is carried
  separately as `db_query_error`.
- **:32154** — `report.total_indexed_conversations += count;` sits inside
  `for row in db_rows` (:32132-32206). With an empty vec the loop body never runs and the
  field keeps its `Default` value of 0. Confirmed.
- **:36328** — `archive_conversation_count: source_inventory.total_indexed_conversations`.
  Confirmed.
- **Correction to the bead's chain.** The bead treats the whole baseline as zeroed. Only
  the conversation half is. `message_delta`'s baseline is
  `coverage_summary.archived_message_count` (:36378), and that field does **not** come
  from the source inventory — it is summed from raw-mirror backfill receipts at
  :36258-36262 (`backfill.receipts.iter().map(|r| r.message_count).sum()`). A DB read
  failure does not zero it. So the failure zeroes one of the two blocking comparisons,
  not both.
- **:36473** — `let promote_allowed = blocking_reasons.is_empty();`. The bead's :36471 is
  two lines early. Everything else about the hop is accurate.
- **:38752-38764** — `promote_allowed` is one of five `&&` conjuncts deciding
  `"completed"` vs `"blocked"`; :38755 is the conjunct.

### 2. `archive_db_unreadable` does NOT block the promotion — it is raised on a different command entirely

This is the bead's open question, and the answer is a clean no.

- It is raised at **:34630-34643**, inside the function that returns
  `DoctorArchiveScanContext` (:34694).
- It is counted at **:34651-34654** and sets `scan.status = "critical"` at **:34666-34672**.
- `build_doctor_archive_scan_context` is defined at **:34414**. Its **only two production
  call sites** are **:35220**, inside `run_doctor_archive_scan_impl` (:35209), and
  **:35291**, inside `run_doctor_archive_normalize_impl` (:35277). Every other call site
  (:62248, :62318, :62376, :62417, :62453) is inside a test module.
- **`run_doctor_impl` (:68232) — the function that builds candidates and promotes them —
  never calls it.** The finding is not computed at all on that path.
- Nothing anywhere branches on the finding kind. `archive_db_unreadable` occurs **exactly
  once** in `src/` (:34635) and **zero times** in `tests/` or any golden fixture (counted
  with `rg -c` over the repo excluding `target`; the only other two hits are prose in this
  handoff directory). The `finding_kind` field is only ever serialized (:34715, :34735,
  :34745, :34753, :34848, :35271); the two places code matches on a kind string (:62276,
  :62299) match `missing_raw_mirror_blob`, in tests.
- Its whole structural effect is the boolean `"healthy": context.scan.critical_finding_count
  == 0` at **:35232** and **:35382**, both inside the archive-scan / archive-normalize JSON
  payloads.

**Null result, stated plainly: there is no consumer of `archive_db_unreadable` that
refuses anything.** I looked for one and there is nothing.

### 3. The promotion function, and what actually refuses

The function that would lose conversations is
**`promote_doctor_reconstruct_candidate_bundle` at :46123**. It has 13 call sites; **11 are
in test modules**. The two production ones:

- **:39858** — the backup restore-apply path. Its candidate manifest is written by
  `write_doctor_restore_candidate_manifest` (:39560), which **hardcodes
  `"coverage_gate": { "status": "backup-restore-explicit-fingerprint", "promote_allowed":
  true }` at :39581-39584**. The coverage gate is bypassed by construction on that path.
  Different path from the bead's chain; flagged here because it means the gate protects
  one of the two promotion routes, not both.
- **:69647** — the repair-apply path, inside `run_doctor_impl`. This is the bead's chain.

Its own guards (all inside :46165-46434), in order: manifest must parse; `lifecycle_status`
must be `"completed"` (:46213-46221); `coverage_promote_allowed` is read out of the manifest
with `.unwrap_or(false)` (:46197-46200) and pushes a blocker at :46222-46226;
`live_inventory_unchanged` must be `true` (:46227-46236); the recorded live inventory must
still equal the current one (:46237-46244); every DB/WAL/SHM bundle component must exist,
be confined to the staging root, and match its recorded blake3 (:46246-46321); no symlinked
ancestor (:46328-46333). Only then does it mutate, and it copies the **prior live bundle**
to a backup first (:46429-46434, `"candidate promotion prior-live bundle backup"`) with a
backup manifest (:46462-46492) and a rollback path (:69866).

**The refusal that actually stops the fail-open is `db_ok`, at :47925-47935** in
`build_doctor_repair_plan_preview`:

```
let candidate_promotion_candidate = if db_ok {
    if selected_completed_candidate.is_some() {
        warnings.push("candidate-promotion-skipped: canonical archive DB is readable; ...");
    }
    None
} else {
    selected_completed_candidate
};
```

When `db_ok` is true the promote action is never added to the plan (:47968-47989), so
`candidate_promotion_apply_requested` (:69628-69634) is false and :69647 is unreachable.

Two further gates on that path: `apply_authorized = apply_requested && fingerprint_matches
&& blocked_reasons.is_empty()` (:48181) — a two-step dry-run-then-exact-fingerprint
approval; and exactly one completed candidate, or it is refused as ambiguous
(:47877-47886 selection, :47937-47947 blocker).

### 4. Verdict: DEFENCE IN DEPTH for the bead's stated worst case, but the gate is still genuinely broken

`db_ok` and `db_query_error` are **two different probes with different timeouts**, so they
can disagree in both directions:

- `db_ok`: open with a **30-second** hard timeout, then `COUNT(*)` on conversations and on
  messages, then an integrity probe (:68625-68654). Any failure — including the timeout —
  leaves `db_ok` false (:68347 initialiser, :68724-68732 error arm).
- source inventory: open with a **1-second** timeout (:32288-32292) and then a five-way
  `GROUP BY` join across `conversations`/`agents`/`sources` (:32065-32082).

**Case A — `db_ok` true, `db_query_error` set.** This is the transient case the bead worries
about, and it is exactly what a 1-second timeout under indexer contention produces. The
baseline silently becomes 0 and the gate reports `promote_allowed`. But the promote action
is never planned, because `db_ok` is true (:47925-47932). **A transient DB read error alone
cannot promote anything.** The bead's stated worst case does not occur.

**Case B — `db_ok` false AND `db_query_error` set.** Both probes fail: genuine corruption,
or both timing out under enough contention. Now the promotion is plannable, and the gate
that is supposed to prove the candidate does not shrink coverage is comparing against a
fabricated 0. It reports pass. It is still behind two-step fingerprint approval and the live
bundle is backed up before replacement, so bytes stay recoverable under the data dir — but
the gate contributes nothing at the exact moment it is supposed to.

So: **defence in depth, not a live one-error hole — and the defence is `db_ok`, not
`archive_db_unreadable`.** The finding the bead hoped was blocking is computed on a command
that never runs here. The fix is still worth making, because the gate's value is precisely
its behaviour when the archive is unreadable, and that is the case where it currently says
pass.

Two further consequences of the same zeroed count, both outside the bead's chain:

- **A transient read error can trigger a candidate build.** :36989 computes
  `raw_mirror_links_minus_archive = raw_mirror_db_link_count as i64 - total_indexed_conversations
  as i64`; with the archive count fabricated as 0 that is positive whenever any verified
  mirror link exists. :38243-38248 ORs that into `doctor_candidate_build_should_run`, so
  `cass doctor --fix` can stage a reconstruct candidate — a real write under
  `<data_dir>/doctor/candidates` — on a false premise, while the archive is fine. Gated by
  `fix_can_mutate` and by an available candidate authority.
- **The honesty defect the sibling lanes are chasing is here too.** :37049-37060 renders
  "archive database currently contains 0 indexed conversation(s) and is authoritative" and
  the evidence string `archive-conversation-count=0` as measured fact when the query
  failed. The one place doctor tells the truth about it is :68935-68941, and it is a
  **`warn`**, not a `fail`.

### 5. Positive control CONFIRMED — the candidate side does fail closed, and the asymmetry is real

Verified in source rather than assumed. At **:36424-36432** and **:36433-36441**:

```
None => blocking_reasons
    .push("candidate conversation coverage is unknown and cannot be promoted".to_string()),
...
None => blocking_reasons
    .push("candidate message coverage is unknown and cannot be promoted".to_string()),
```

(the two `.push` lines are :36431 and :36440). An unknown **candidate** count blocks. An
unknown **baseline** is silently 0 and passes. Same function, same paragraph, opposite
policy — that is a defect, not a design choice.

**Root cause of the asymmetry is in the types.** The candidate side is
`Option<usize>` (:36367-36370), so it can say "unknown". The baseline is
`DoctorCoverageSummary.archive_conversation_count: usize` — there is no representation for
unknown, so a failed read is indistinguishable from an empty archive.

**The fail-closed candidate branch is untested.** The gate has exactly four references in
the repo: its definition (:36364), its one production caller (:38642), and two tests
(:61422 in `doctor_coverage_comparison_gate_blocks_data_reducing_candidates`, :61472 in
`doctor_coverage_comparison_gate_warns_on_derived_only_mismatches`). Both pass `Some(..)`
for conversation and message counts. **No test passes `None`.** So the positive control the
bead relies on is source-verified but has no regression guard.

### 6. Also silent on the way out

:69340-69353 pushes a `coverage_comparison_gate` check with status `"fail"` **only when
`promote_allowed` is false**. When the gate fails open there is no check, no warning, and no
line in the doctor output. The fail-open is invisible on this surface.

## Proof boundary — what I did NOT establish

- **Nothing was executed.** No build, no test, no `cass`, no probe. Every finding is read
  from source. I have not observed a single one of these branches run.
- **I did not reproduce Case B.** That both probes can time out together under real
  contention is an inference from the two timeout constants (30s at :68628, 1s at :32291)
  and from `open_franken_cli_read_db_with_hard_timeout` returning `Err` on timeout. I did
  not measure it. What would settle it: a test that opens the archive under a held lock and
  asserts `db_ok == false` while `db_query_error.is_some()`.
- **I did not read `open_franken_cli_read_db` or `open_franken_cli_read_db_with_hard_timeout`.**
  I am relying on the names and on the `Err` arms. If the 1s value at :32291 is a connect
  timeout rather than a statement timeout, the contention story changes shape (though not
  the fail-open, which follows from the `Err` arm regardless of why it fired).
- **I did not trace `collect_doctor_candidate_staging_report`.** I established that
  candidate selection for promotion keys only on `lifecycle_status == "completed"`
  (:47883) and that the manifest's `coverage_gate.promote_allowed` is re-read at :46197.
  I did not verify how the staging report discovers or validates on-disk manifests, so I
  cannot rule out a further guard there.
- **I did not verify the restore path end to end.** I read that :39583 hardcodes
  `promote_allowed: true` and that :39858 promotes with it. I did not read
  `doctor_backup_verification` upstream, so I cannot say what that path proves instead of a
  coverage comparison. It is not the bead's chain, and it may be entirely sound.
- **Uncertain: whether Case B's fabricated 0 is materially worse than the alternative.**
  When the archive genuinely cannot be read, no honest baseline exists, so blocking on
  unknown means an unreadable archive can never be repaired by promotion. That is a product
  decision (block, or promote with an explicit "baseline unknown" receipt), not something I
  can settle from source. What I can say is that the current behaviour picks neither — it
  reports a number it did not measure.
- **The worktree was mutating under me.** All numbers are pinned to `6bcc51b7`. I did not
  re-verify any finding against the current working tree, and the sibling's in-flight edit
  is at :15311-15372 only, so it does not touch anything cited here — but if further edits
  land in the doctor region these numbers will drift.
