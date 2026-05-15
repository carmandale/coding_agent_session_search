<!-- plan:complete:v1 | harness: unknown | date: 2026-05-15T16:23:04Z -->

## Phase A (Research Challenge) — Round 1

Protocol violation in the Phase A context file. I’m refusing to continue the research challenge until it is cleaned up.

The violation is at `/tmp/codex-plan-research-5262e27d.md:89`:

`stop watcher → repro → restart watcher`

That is explicit operational sequencing. It also says the plan needs that guard around profiling and full-corpus verification, which is plan-level content, not pure research findings.

Remove the task/sequence-level language and resubmit the Phase A research context.
## Phase A (Research Challenge) — Round 2 (substantive; Round 1 was a protocol-violation reject)

Session: 019e2c56-76b4-7793-b904-47e1e1a77f1a

**Phase A Challenge**

I don’t see a negative-contract violation in `/tmp/codex-plan-research-5262e27d.md`. The research note stays at candidate/assumption level and does not contain T-style tasks, implementation sequencing, final plan prose, or file-by-file change lists.

The biggest problem in the research is that it treats `extra_bin` like a field available at the normalization boundary. It is not. FAD’s `NormalizedMessage` has `content`, `extra`, `snippets`, and `invocations`; `extra_bin` and `metadata_bin` are cass storage columns created later during persistence. So a plan that says “cap `extra_bin` in pi normalization” is already conceptually off by one layer. The real candidate field is `message.extra`, which FAD currently fills with `val.clone()` for the whole original JSONL entry.

The strongest unverified assumption is where peak RSS occurs. The research says the code “suggests” the blow-up may happen after `conn.scan()`, but the FAD pi connector does a full `fs::read_to_string`, parses each line into `serde_json::Value`, flattens content into a `String`, and then stores a clone of the entire source JSON line as `message.extra`. After that, cass clones the same data again in `map_to_internal`, serializes it to MessagePack for `extra_bin`, and clones it again for lexical packets. A post-scan compactor may reduce later storage and Tantivy pressure, but it cannot reduce scan-time peak if the `_platform_memset` frame is inside `read_to_string`, `serde_json`, or `val.clone()`.

The proposed split path is also probably weaker than the research implies. `watch_ingest_chunk_size()` limits the number of conversations passed into `ingest_watch_batch_with_oom_split`, but it does not limit per-conversation message count, per-message `extra`, scan-time `Vec<NormalizedConversation>`, Rayon mapping clones, MessagePack serialization, or lexical packet cloning. If one 72 MB pi JSONL file is the culprit, proactive batch splitting can go all the way to a single conversation and still fail. The plan needs a single-conversation stress case, not just “split the batch earlier.”

There is a major acceptance-risk mismatch: current `~/.pi/agent/sessions` appears to contain 2,073 `.jsonl` files, not ≥2,800, and `.omp/agent/sessions` is absent. If FAD maps one session file to one conversation, the spec’s `conversations >= 2,500` acceptance criterion is impossible on the currently visible corpus. Phase B should challenge the acceptance number before using it as proof of success.

A stronger alternative is to make pi ingestion avoid carrying full raw JSON in `message.extra` at all, while relying on raw mirror for full-fidelity reconstruction. That could be done either upstream in FAD or by replacing the current cass re-export with pi-specific glue. This directly attacks the repeated clone/serialization surface. The tradeoff is real: UI/export/debug consumers may expect `extra_json` to hold provider raw data, so the allowed retained subset must be explicit and tested.

Raw mirror is another missed blast-radius path. `capture_connector_sources_before_parse` captures every discovered pi source before parsing, and `attach_raw_mirror_capture` links the parsed conversation afterward. The copy path streams in 64 KiB chunks, so it is probably not the 22 GB RSS source, but on the visible corpus it is a 1.7 GB disk/time side effect. If the fix leans on raw mirror to justify dropping `message.extra`, verification must prove raw mirror is present and linked for the compacted pi rows.

The “33 conversations” theory is plausible but still too story-shaped. The first successful row plus one 32-conversation chunk fits PR #233’s default chunk size, but it does not prove the stall happens in chunk 2. It could also mean chunk 1 was the first time the pathological file hit storage/lexical serialization, or that only 33 rows became visible before a later long scan/persist phase wedged. The plan needs stage-level evidence: after scan, after compaction/raw mirror, before/after `map_to_internal`, before/after MessagePack payload creation, and before/after lexical packet creation.

Finally, the current fixture coverage is nowhere near the risk. There is one pi fixture, and the existing OOM split test uses Amp with synthetic OOM injection. That will not catch the real failure class: large pi `message.extra` duplication and MessagePack allocation. A credible plan needs a pi-specific regression with large `toolCall.arguments`, large `thinking`, or large tool-result content that proves message count and searchable content survive while peak retained payload is bounded.

**What I Verified**

I read the Phase A research note and found no negative-contract violation.

I checked the spec at [spec.md](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/spec.md:1): it requires source-line allocation proof, full pi watch-once completion, RSS under 8 GB, PR #233 behavior preserved, and no reduction in pi message coverage.

I confirmed PR #233 changed only [src/indexer/mod.rs](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:16333) to always use `watch_ingest_chunk_size()`.

I confirmed cass’s pi connector is only a re-export at [src/connectors/pi_agent.rs](/Users/dalecarman/dev/coding_agent_session_search/src/connectors/pi_agent.rs:3), with FAD pinned in [Cargo.toml](/Users/dalecarman/dev/coding_agent_session_search/Cargo.toml:85).

I confirmed FAD’s pi scanner reads whole files and stores full source entries in `message.extra` at [pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:351) and [pi_agent.rs](/Users/dalecarman/.cargo/git/checkouts/franken_agent_detection-1ef1b003c0d2ba55/5115da8/src/connectors/pi_agent.rs:485).

I counted 2,073 visible pi `.jsonl` files and 1.7 GB total under `/Users/dalecarman/.pi/agent/sessions`; largest file was about 71.9 MB.
## Phase A (Research Challenge) — Round 3 (revised research, second substantive challenge)

**Protocol Check**

I don’t see a negative-contract violation in the revised research file: no task breakdown, no implementation sequence, no final-plan prose, and no file-by-file change list. Continuing with the Phase A challenge.

**Challenge Text**

C1 is cass-local only in the narrow ownership sense. Replacing the bare re-export in [src/connectors/pi_agent.rs](/Users/dalecarman/dev/coding_agent_session_search/src/connectors/pi_agent.rs:1) with a wrapper that mutates FAD’s public `NormalizedMessage` after `scan()` returns is not “patching franken-agent-detection.” But it also is not a scan-time memory fix. FAD has already done `fs::read_to_string(&file)` and `extra: val.clone()` before cass can strip anything, at the FAD source lines I read in the cargo checkout. A C1 wrapper can reduce lifetime/persist pressure after [conn.scan(&ctx)](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:16248), but if profiling says the hot allocation is in FAD’s file scan or `val.clone()`, C1 is a mitigation, not the root fix. The stronger cass-local alternative still missing is a cass-owned pi streaming connector/parser that avoids the FAD `read_to_string` + cloned raw line path while preserving the normalized output contract.

There are more `message.extra` consumers than just `extra_bin`. The direct flow is `NormalizedMessage.extra` → packet `extra_json` at [conversation_packet.rs:513](/Users/dalecarman/dev/coding_agent_session_search/src/model/conversation_packet.rs:513), and that same value participates in packet hashes at [conversation_packet.rs:668](/Users/dalecarman/dev/coding_agent_session_search/src/model/conversation_packet.rs:668). It also becomes internal `Message.extra_json`, with redaction applied, at [indexer/mod.rs:18763](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:18763). Storage-side analytics extract token/model info from `msg.extra_json` at [storage/sqlite.rs:10309](/Users/dalecarman/dev/coding_agent_session_search/src/storage/sqlite.rs:10309). Pages export derives model and attachment refs from `extra_json` at [pages/export.rs:351](/Users/dalecarman/dev/coding_agent_session_search/src/pages/export.rs:351). So stripping pi extras needs an explicit preservation story for model, attachments, token usage if pi has any, and hash/dedupe behavior. The current compactor preserves model and attachments, not token usage, and it is currently codex-only at [indexer/mod.rs:17552](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:17552).

Raw mirror is solid as byte retention, not yet solid as an exact `message.extra` reconstruction contract. The manifest records blob path/hash/size and db links at [raw_mirror.rs:769](/Users/dalecarman/dev/coding_agent_session_search/src/raw_mirror.rs:769), and capture is content-addressed at [raw_mirror.rs:794](/Users/dalecarman/dev/coding_agent_session_search/src/raw_mirror.rs:794). But `RawMirrorDbLink` only stores conversation-level linkage, message count, source path, and started time at [raw_mirror.rs:48](/Users/dalecarman/dev/coding_agent_session_search/src/raw_mirror.rs:48). It does not store per-message line offsets or the exact FAD skip/filter decisions. A future rebuild can probably replay the JSONL and reconstruct equivalent extras, but that is an algorithmic claim that needs proof against FAD’s current parsing rules, especially because FAD skips empty flattened content and uses session/model-change lines as context.

The candidate space still has a spec mismatch around acceptance counts. The spec says the corpus has “≥ 2,800 jsonl files” and acceptance requires at least 2,500 conversations at [spec.md:44](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/spec.md:44) and [spec.md:66](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/spec.md:66). My current NUL-safe check gets 2,073 `.jsonl` files, and `du -ch` reports 1.7G total, not 1.0G. That is not just a documentation nit; it can invalidate the success threshold unless the missing files are elsewhere or the acceptance criterion is amended.

Be careful with C3: extending `compact_large_connector_extras()` to pi can help persist-time pressure, but it cannot satisfy the spec’s profiling requirement by itself. Requirement 1 asks for the exact Rust allocation site and field at [spec.md:50](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/spec.md:50), and acceptance requires pre/post `sample` evidence at [spec.md:69](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/spec.md:69). Also, any content truncation to survive a single 72 MB conversation risks violating “No reduction in pi-agent message coverage” at [spec.md:70](/Users/dalecarman/dev/coding_agent_session_search/specs/014-pi-agent-memset-stall/spec.md:70). Shrinking raw `extra` is different from reducing searchable message content.

One more coupling point: raw mirror attachment happens after compaction in the watch scan loop at [indexer/mod.rs:16263](/Users/dalecarman/dev/coding_agent_session_search/src/indexer/mod.rs:16263). That order is probably okay if raw mirror linkage depends only on source path/provenance, but the research should explicitly prove the affected pi rows still get manifest links after `message.extra` is stripped. Otherwise the fidelity argument becomes “the source file exists somewhere,” not a verified cass recovery path.
### Phase A → Phase B transition

The research direction is now settled. The candidate space (C1 cass-local glue / C2 upstream FAD fix / C3 extend codex compactor to pi / C4 cass-owned streaming pi parser) is explicit with peak-RSS-vs-mitigation trade-offs named. Codex's round-3 concerns — message.extra caller preservation contract, raw-mirror fidelity formalization, spec acceptance amendment, order-of-operations between compaction and raw-mirror attach — are Phase B territory and will be addressed in plan.md sections. Proceeding to Phase B draft.

Verified disk numbers (round-3 reconciliation):
- pi corpus: 2,073 .jsonl files, 1.70 GB total, biggest 72 MB
- spec.md:44 says "≥ 2,800" (off by 727)
- spec.md:66 / acceptance #1 says "≥ 2,500" (impossible given corpus ceiling 2,073)
- Required amendment to spec.md acceptance #1 will be surfaced as part of Phase B output before final.


## Phase B (Plan Draft Review) — Round 1 — VERDICT: REVISE

Codex flagged 5 blockers: FAD-scan path not closed, missing spec (b) pre-split branch, missing PR #233 regression test, missing raw-mirror reconstruction test, mid-run sample timing wrong.

## Phase B (Plan Draft Review) — Round 2 — VERDICT: REVISE

Codex flagged 4 internal-consistency drifts after the Round 1 fixes: Shape X still referenced the old "C3 + parallel FAD PR" path, Plan Sanity Evidence Decision impact stale, T9a allowed downgrade-to-C3, T5 didn't include C5.

## Phase B (Plan Draft Review) — Round 3 — VERDICT: REVISE

Codex caught two residual drift echoes after Round 2 fixes: Shape X closing sentence and T9 C2 bullet.

## Phase B (Plan Draft Review) — Round 4 — VERDICT: APPROVED

The drift cleanup is good. The remaining references now consistently bind FAD-scan to C4 or C2 landed in-cycle, and the T9/T9a path no longer permits downgrading to C3 as a deferred acceptance-#2 workaround.

I re-checked the Shape X close, Decision impact, T5, T9, T9a, and the remaining `defer/follow-up/C3 mitigation` search hits. No blocking inconsistency remains.

VERDICT: APPROVED
## Phase B — Boundary SHA: 05ba881b — Snapshot: boundary-B-bootstrap.md
## Phase B — North-Star Check — Round 1 — BOOTSTRAP
