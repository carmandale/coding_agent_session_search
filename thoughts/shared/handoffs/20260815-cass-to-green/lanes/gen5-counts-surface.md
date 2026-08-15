# gen5-counts-surface — StateDbSnapshot count-field surface (read-only survey)

**Lane:** log-only-evidence, read-only. No src/tests edited. Only this file was written.
**Bead:** coding_agent_session_search-0gzok part 2.

## Question

What breaks if `StateDbSnapshot.conversation_count` and `.message_count`
(currently `i64`) change to `Option<i64>`, where `None` means "unknown"?
Enumerate every direct read of those two fields on a `StateDbSnapshot` value,
`state_db_count_json`'s callers, `refresh_state_database_counts_if_needed`'s
"improved" heuristic, every downstream reader of the
`database.conversations`/`database.messages`/`database.counts_skipped` JSON
keys, and the `fresh_franken_count_retry` fallback trigger.

## Method

`rg -n` for `conversation_count`, `message_count`, `StateDbSnapshot`,
`probe_state_db`, `state_db_count_json`, `refresh_state_database_counts_if_needed`,
`.get("database")`, `index_empty_with_messages`, and the three
`response_schema_*_database`/`response_schema_health_db` functions across
`src/lib.rs`. Every cited line was opened with `Read` at the offset `rg`
reported, in the same pass where possible, and content (not just the line
number) was re-confirmed at the final snapshot below. Field-vs-struct
identity was established by tracing each local variable back to its
assignment (`db_snapshot = ... probe_state_db(...)` or `let snapshot =
probe_state_db(...)` in tests) rather than by name alone, because
`conversation_count`/`message_count` are field names reused on ~15 unrelated
structs in this file (semantic-tier stats, archive-coverage structs,
doctor-remote-sync structs, indexer progress structs — none of those are
enumerated below; they don't touch `StateDbSnapshot`).

## Important caveat established mid-investigation: the file is a live target

`git status --short -- src/lib.rs` showed `M src/lib.rs` (uncommitted,
modified) for this entire investigation, and `wc -l` grew from 91,859 lines
at the start of this lane to 91,989 lines by the end — a concurrent session
in this same worktree is actively landing commits against
`src/lib.rs` (visible in `git log --oneline -3 -- src/lib.rs`:
`8dcd245b fix(coverage): bound the whole coverage read...`,
`447d97fe fix(status): bound the raw-mirror walk...`) and has an in-progress
edit on top of that, matching the orchestrator's own task list item #2,
"Fix -nao4q: bound the whole probe_state_db read". Mid-investigation I
caught this directly: an `rg` result and the `Read` immediately after it
disagreed on what line 73291 contained (one said
`fn response_schema_state_database`, the other showed unrelated JSON-schema
lines) because the file shifted between the two calls. I re-ran `rg` +
`Read` together as a final synchronized pass (timestamped
2026-08-15T17:10:53Z–17:11:12Z) for every citation below, and the *shape* of
every finding was independently re-verified against two full snapshots
(the file's state at investigation start, and this final one) with
identical content, only shifted by a constant ~81-line offset — so the
causal chain below is solid, but **treat every specific line number as
correct only as of 2026-08-15T17:11:12Z; re-`rg` before editing.**

One concrete, on-topic consequence of that concurrent edit: `probe_state_db`
itself was rewritten during this lane (bead `coding_agent_session_search-nao4q`)
into a bounding wrapper (`src/lib.rs:15342-15388`) around a new
`probe_state_db_blocking` (`src/lib.rs:15393` on) that does the real work on a
worker thread with a `recv_timeout`. That fix is doc-commented
(`src/lib.rs:15312-15341`) as explicitly aware of this lane's exact concern —
its own comment says "the counts elided rather than invented —
`state_db_count_json` already renders JSON null when `counts_skipped`" — but
it did **not** touch the `.unwrap_or(0)` count-query bug this lane is
scoped to. Confirmed at `src/lib.rs:15451-15481` (below): unchanged.

## Findings

### 1. Every direct read of `.conversation_count` / `.message_count` on a `StateDbSnapshot` value

Struct definition: `src/lib.rs:15292-15294`.
```
15292: struct StateDbSnapshot {
15293:     conversation_count: i64,
15294:     message_count: i64,
```

| site | file:line | what it does | assumes today |
|---|---|---|---|
| write (query result) | `src/lib.rs:15451-15457` | `snapshot.conversation_count = franken_query_row_map_retry(&conn, "SELECT COUNT(*) FROM conversations", ...).unwrap_or(0)` | `.unwrap_or(0)` — a failed query is indistinguishable from a real zero-row table. **This is the bug the bead names.** |
| write (query result) | `src/lib.rs:15458-15462` | same for `message_count` / `SELECT COUNT(*) FROM messages` | same |
| read (retry gate) | `src/lib.rs:15463` | `if snapshot.conversation_count == 0 {` — gates the `fresh_franken_count_retry` fallback | reads 0 as "worth retrying," but 0 is now overloaded: real-empty-table, failed-query, *and* "about to retry" all look identical |
| write (fallback result) | `src/lib.rs:15464-15471` | `snapshot.conversation_count = fresh_franken_count_retry(...).unwrap_or(0)` | same `.unwrap_or(0)` bug, one level deeper |
| read (retry gate) | `src/lib.rs:15473` | `if snapshot.message_count == 0 {` | same as conversation_count's gate |
| write (fallback result) | `src/lib.rs:15474-15481` | `snapshot.message_count = fresh_franken_count_retry(...).unwrap_or(0)` | same bug |
| read | `src/lib.rs:16173` | `let conversation_count = db_snapshot.conversation_count;` in `state_meta_json_inner` | copies the (possibly-lying) `i64` into a local that feeds JSON + the index-doc-count gate below |
| read | `src/lib.rs:16174` | `let message_count = db_snapshot.message_count;` | same |
| read (gate) | `src/lib.rs:16451` | `if db_opened && message_count > 0 && lexical.exists` — decides whether to probe the live Tantivy doc count at all | treats "0" as "no messages, skip the probe" — a failed count silently *skips* the one check (`index_empty_with_messages`) that would otherwise flag a real problem |
| read (gate) | `src/lib.rs:16464` | `.map(|docs| docs == 0 && message_count > 0)` computes `index_empty_with_messages` | same false-zero risk, one hop downstream |
| read (JSON emit) | `src/lib.rs:16512` | `state_db_count_json(conversation_count, counts_skipped)` | passes the local `i64` straight through — see item 2 |
| read (JSON emit) | `src/lib.rs:16513` | `state_db_count_json(message_count, counts_skipped)` | same |
| read (heuristic) | `src/lib.rs:16804` | `refreshed.conversation_count > 0` inside `refresh_state_database_counts_if_needed`'s "improved" check | `refreshed` is a second, independent `probe_state_db` result — see item 3 |
| read (heuristic) | `src/lib.rs:16805` | `refreshed.message_count > 0` | same |
| read (JSON emit) | `src/lib.rs:16819` | `state_db_count_json(refreshed.conversation_count, refreshed.counts_skipped)` | same pass-through as 16512 |
| read (JSON emit) | `src/lib.rs:16823` | `state_db_count_json(refreshed.message_count, refreshed.counts_skipped)` | same |
| test assertion | `src/lib.rs:66607-66621` (`probe_state_db_reads_meta_without_count_scan`) | asserts `snapshot.conversation_count == 0` and `snapshot.message_count == 0` when `include_counts=false` | **must change.** This call passes `include_counts: false`, so `counts_skipped` is `true` by construction (`src/lib.rs:15404`, `counts_skipped: !include_counts`) — under the `None`-means-unknown design this is exactly the "deliberately not counted" case, so the correct post-change assertion is `assert_eq!(snapshot.conversation_count, None)` / `None` for `message_count`, not `Some(0)`. |

**What each site must become**, at minimum, for the field to compile as `Option<i64>`:
- `15451-15462`: the `.unwrap_or(0)` must become `.ok()` (so a failed `franken_query_row_map_retry` yields `None` instead of `Some(0)`).
- `15463`/`15473`: `if snapshot.conversation_count == 0` cannot compile against `Option<i64>` — must become something that means "we don't yet have a confirmed nonzero count," e.g. `if snapshot.conversation_count.is_none_or(|n| n == 0)` (or, if the intent is *only* "retry on a real empty read but not on a failed one," `if snapshot.conversation_count == Some(0)`, which is the sharper, bug-fixing reading and is a genuine behavior change: today a failed first COUNT(*) query — reported as `Some(0)` — always triggers the fallback; under `None`, a failed query would need its own explicit branch to decide whether to retry at all).
- `15464-15481`: fallback assignment gets the same `.unwrap_or(0)` → `.ok()` change.
- `16451`/`16464`: `message_count > 0` does not compile against `Option<i64>`; must become `message_count.is_some_and(|n| n > 0)` — and this is a real semantic fork the implementer must pick, not a mechanical rename: `None` (unknown) could either skip the live-doc-count probe (current behavior for `0`, extended) or *always* run it defensively since "unknown" is not "zero." The comment at `16447-16450` ("Probe the live lexical document count when the DB has messages") suggests the honest behavior for `None` is closer to "we don't know, so don't claim the index is empty-with-messages," i.e. keep treating `None` like `0` here — but that should be a stated decision, not a silent fallthrough of `.unwrap_or(0) > 0`.
- `16804`/`16805`: `refreshed.conversation_count > 0` — same fix as above, `refreshed.conversation_count.is_some_and(|n| n > 0)`.
- `66607-66621`: test assertions must become `None`, and the test's premise ("counts_skipped=true means the field IS meaningfully zero") already contradicts the file's own comment two call sites away — see item 2.

### 2. `state_db_count_json` (`src/lib.rs:16761-16767`) — callers and null-vs-number behavior

```
16761: fn state_db_count_json(count: i64, counts_skipped: bool) -> serde_json::Value {
16762:     if counts_skipped {
16763:         serde_json::Value::Null
16764:     } else {
16765:         serde_json::Value::from(count)
16766:     }
16767: }
```

**Callers (exactly two call *sites*, four invocations):**
1. `src/lib.rs:16512` — `state_db_count_json(conversation_count, counts_skipped)`, inside `state_meta_json_inner`'s `"database"` JSON object (`16509-16518`). `conversation_count`/`counts_skipped` are the `db_snapshot` locals from item 1.
2. `src/lib.rs:16513` — same, `message_count`.
3. `src/lib.rs:16819` — `state_db_count_json(refreshed.conversation_count, refreshed.counts_skipped)`, inside `refresh_state_database_counts_if_needed` (item 3), writing the refreshed value back into the live `serde_json::Value` state object.
4. `src/lib.rs:16823` — same, `refreshed.message_count`.

**What it assumes today:** the *only* signal for "this number is not real" is the separate `counts_skipped` bool. It never looks at the count value itself. That is exactly the bug: when `include_counts=true` and the `COUNT(*)` query fails, `counts_skipped` stays `false` (nothing at the `probe_state_db_blocking` call sites sets it on query failure — confirmed by re-reading `15450-15483`: the `if include_counts { ... }` block only ever writes into `.conversation_count`/`.message_count`, never into `.counts_skipped`), so `state_db_count_json` takes the `else` branch and emits `serde_json::Value::from(0)` — a literal JSON `0`, presented with exactly the same shape as a genuine "we counted and there are zero rows."

**What it must become:** the signature has to accept `Option<i64>` (`fn state_db_count_json(count: Option<i64>, counts_skipped: bool) -> serde_json::Value`), and the body has a real design choice to make, not a mechanical one: should it emit `Null` whenever `count.is_none()`, *regardless* of `counts_skipped`? That is the reading consistent with `None` meaning "unknown" and is the minimal fix — `counts_skipped` stops being the sole null-trigger and becomes redundant with `count.is_none()` in the honest-skip case, while still being independently `true`/`false` for callers that want to know *why* it's null (deliberately skipped vs. attempted-and-failed). The struct doc comment already anticipates this exact distinction for a sibling field: `src/lib.rs:15306-15309` on `connector_scan_floors: Option<BTreeMap<String, i64>>` — *"`None` when this probe never opened the database and so did not check. `Some(empty)` means checked and complete; the two are not interchangeable."* The same two-state read applies here: `counts_skipped=true` should mean "we deliberately never asked," and `count: None` under `counts_skipped=false` should mean "we asked and failed" — two distinguishable failure/skip modes state_db_count_json currently cannot represent because it only carries one bit (`counts_skipped`) instead of the `Option`'s own state.

### 3. `refresh_state_database_counts_if_needed` (`src/lib.rs:16769-...`) — the "improved" heuristic

Read at `16769-16843` (confirmed in full at this offset). Relevant block:

```
16784: let current_conversations = state.get("database").and_then(|db| db.get("conversations")).and_then(|v| v.as_i64()).unwrap_or(0);
16789: let current_messages = state.get("database").and_then(|db| db.get("messages")).and_then(|v| v.as_i64()).unwrap_or(0);
16795: let needs_refresh = !current_counts_skipped && (!current_opened || current_conversations <= 0 || current_messages <= 0);
16801: let refreshed = probe_state_db(db_path, reason, Duration::from_secs(30), true);
16802: let improved = (!current_opened && refreshed.opened)
16803:     || (current_counts_skipped && !refreshed.counts_skipped)
16804:     || (current_conversations <= 0 && refreshed.conversation_count > 0)
16805:     || (current_messages <= 0 && refreshed.message_count > 0);
```

Two separate zero-ambiguity problems, one on each side of this function:

- **`current_conversations`/`current_messages` (JSON side, lines 16784/16789):** these already read a `serde_json::Value` that *can* be `Null` (whenever `counts_skipped` was true when the JSON was built), and `.as_i64()` on a `Null` returns `None`, collapsed by `.unwrap_or(0)` into the same `0` as a real zero-row read. This ambiguity exists **today**, independent of the `StateDbSnapshot` field type — it's a second site with the identical "0 conflates unknown-and-real-zero" shape, just one layer up (JSON rather than the struct). Fixing `state_db_count_json`/the struct does not fix this line by itself; `current_conversations`/`current_messages` would need to become `Option<i64>` locals too (e.g. `.and_then(|v| v.as_i64())` without the `.unwrap_or(0)`), and every place that reads them (`16795`, `16804`, `16805`) needs to handle three states — "known 0," "known >0," "unknown" — instead of two.
- **`refreshed.conversation_count > 0` / `refreshed.message_count > 0` (struct side, lines 16804-16805):** these are direct `StateDbSnapshot` reads (already listed in item 1) and will not compile once the fields are `Option<i64>`. The mechanical fix is `refreshed.conversation_count.is_some_and(|n| n > 0)`, but there's a real semantic question sitting underneath the mechanics: right now "improved" fires when a retry produces a confirmed nonzero count. Under the `Option` model, should "improved" *also* fire when a retry turns a previous failure into a confirmed **zero** (`current_conversations` was unknown, `refreshed.conversation_count` is now `Some(0)`)? That is a real state transition ("unknown → known-empty") that the current `> 0` comparison cannot see and arguably should count as "improved" — going from "we don't know" to "we know it's genuinely empty" is strictly better information, but the current boolean-OR chain has no term for it.

`needs_refresh` at `16795` has the same "0 means try again" ambiguity as item 1's retry gates (`15463`/`15473`), for the same reason: it triggers a refresh attempt on `current_conversations <= 0`, which today conflates "the JSON said a real zero" with "the JSON said null and `.unwrap_or(0)` hid it" (though in the current, unfixed code `counts_skipped` gates the whole expression via `!current_counts_skipped`, so today `current_conversations`/`current_messages` only matter when `counts_skipped` was already `false` — meaning today's `<=0` really is "we tried, and got zero," honest zero or the query-failure bug's false zero, not the deliberate-skip case). Once the JSON can carry a real `Null` for "counted and failed" (item 2's fix), that guard (`!current_counts_skipped`) stops being sufficient to keep `current_conversations`/`current_messages` numeric.

### 4. Downstream readers of `database.conversations` / `database.messages` / `database.counts_skipped`

Traced every `.get("database")` in the file (30 occurrences at the final snapshot) plus the two hard-coded `"conversations": state.get("database")...` / `"messages": state.get("database")...` pass-throughs. Grouped by function:

**`state_meta_json_inner`** (produces the envelope; already covered in items 1-2).

**`refresh_state_database_counts_if_needed`** (`src/lib.rs:16769` on) — covered in item 3.

**`readiness_recommended_commands`** (`src/lib.rs:17056-17072` region) — reads `database.exists` (`17062`), `database.opened` (`17067`), `database.open_retryable` (`17072`), plus `index.empty_with_messages` (`17056`) feeding a check at `17169`. **Does not read `conversations`/`messages` numerically at all.** No change required here beyond whatever `index_empty_with_messages`'s upstream computation needs (item 1).

**`run_status`** (`fn run_status`, contains lines ~65092-65450 at this snapshot):
- `65092-65117`: reads `database.exists`, `.opened`, `.open_error`, `.open_retryable`, `.counts_skipped`, `.open_skipped` — all booleans/strings, no numeric read.
- `65376-65377`: builds a `"database"` sub-object for JSON output by cloning `state.get("database")...get("conversations")`/`"messages"` straight through (`Value::Null` passes through unchanged; a `Value::Number(0)` also passes through unchanged) — this is a pure pass-through, so whatever `state_meta_json_inner` emits (item 2's fix target) is what ships here unmodified. No logic change needed at this site itself once item 2 is fixed; it inherits the fix for free.
- `65470-65481`: the human-readable `println!` path, gated by `if counts_skipped { "Counts skipped..." } else { if let Some(conversations) = ...as_i64() { println!("  Conversations: {conversations}") } }`. This is the exact site the bead's context calls out: today, on a failed query, `counts_skipped` is `false` and the value is a JSON `0`, so this prints `Conversations: 0` — a confident false claim, not a "skipped" message. Once `state_db_count_json` can emit `Null` on failure independent of `counts_skipped`, `.as_i64()` here returns `None` and the `if let Some(...)` silently prints nothing for that line — which is honest but silent. The task doesn't ask me to design the message; flagging that "prints nothing" and "prints skipped" read differently to an operator, and this call site currently has no third branch for "tried and failed" (only "skipped" and "have a number").

**`run_triage`** (`fn run_triage`, ~65572 on) — reads `database.exists` (`65572`), `.opened` (`65577`), `.open_retryable` (`65582`), and `index.empty_with_messages` (feeds `healthy`/`status`). Also embeds the whole `database` object verbatim into its JSON payload's `"readiness"` field (`65680`: `"database": state.get("database").cloned()...`). **No direct numeric read of conversations/messages** — `run_triage`'s health/status computation is entirely boolean-flag-driven (`db_exists`, `db_available`, `index_exists`, `index_fresh`, `!rebuild_active`, `!index_empty_with_messages`). The embedded pass-through inherits whatever `state_meta_json_inner` produces, same as `run_status`.

**`run_health`** (`fn run_health`, ~65773 on) — uses `state_meta_json_for_health`, which forces `skip_db_open=true` (confirmed via the doc comment at the site — health is documented as the <50ms fast path that elides the DB open entirely). Under that path `db_snapshot` is the *synthesized* `StateDbSnapshot { opened: true, counts_skipped: true, open_skipped: true, ..Default::default() }` branch (item 1's context at `16161-16167`), so `conversation_count`/`message_count` are always the struct's default — currently `0`, and under the `Option` change would be `None` — and `counts_skipped` is unconditionally `true`. `run_health` reads `database.exists`/`.opened`/`.open_error` as booleans/strings (`65773-65784`) and never reads `conversations`/`messages` as numbers for its `healthy`/`status`/`warnings`/`errors` logic — only `index_empty_with_messages` (item 1's downstream effect) feeds an error string (`"index empty but database has messages — run 'cass index --full'"`, confirmed present in this function). It does pass the `conversations`/`messages` JSON values through verbatim into its own `"db"` object (two sites, `66038-66039`), same pattern as `run_status`/`run_triage` — pure pass-through, inherits the item-2 fix.

**Readiness computation itself** (`cass_not_initialized`, called at `16302-16306`/`65599`/`65850` etc.) — takes `db_exists`, `lexical_index_initialized`, `rebuild_active` as its three inputs. **Never reads `conversation_count`/`message_count` at all**, confirmed by grep — readiness (initialized vs. not) is entirely independent of the counts. The only readiness-adjacent thing the counts influence is `index_empty_with_messages`, which is a warning/error signal layered on top of readiness, not readiness itself.

**Tests:**
- `probe_state_db_reads_meta_without_count_scan` (`66607-66621`) — covered in item 1, must change its `0`/`0` assertions to `None`/`None`.
- `refresh_state_database_counts_keeps_large_db_counts_skipped` (`66765-...`) — constructs a state JSON with `"conversations": Null, "messages": Null, "counts_skipped": true` directly (bypassing `StateDbSnapshot` entirely) and asserts the refreshed state still shows `counts_skipped: true` and both fields still `is_null()`. This test does not touch the `StateDbSnapshot` field type at all — it operates purely on the JSON envelope — so it should be unaffected by the `Option<i64>` change *unless* `refresh_state_database_counts_if_needed`'s `needs_refresh`/`improved` logic (item 3) changes in a way that alters whether a refresh is attempted for this fixture. Worth a rerun, not a rewrite, once item 3 lands.

**JSON schemas** (introspection contracts) — three schema-builder functions all describe `database.conversations`/`.messages` as **already nullable**:
- `response_schema_state_database()` — `src/lib.rs:73333-73347`: `"conversations": { "type": ["integer", "null"] }`, `"messages": { "type": ["integer", "null"] }`.
- `response_schema_status_database()` — `src/lib.rs:73349-73357`: derives from the above, adds `"path"`.
- `response_schema_health_db()` — `src/lib.rs:73359-73372`: same nullable pair, independently declared (not derived from the other two — a second, structurally-identical copy; flagging as a `structural-coupling.md`-shaped drift risk but out of scope for this lane's question).

Callers (i.e. which `--json` surfaces publish these schemas): `response_schema_state_database()` at `73697` (feeds `response_schema_state_meta`) and again at `73... ` inside the `"triage"` schema's nested `"readiness.database"` (matched via the second `response_schema_state_database()` call site in the schema-registry block, confirmed present); `response_schema_status_database()` at two call sites (the `"status"` schema and the `"state"` schema, both in the schema-registry `HashMap` builder); `response_schema_health_db()` at one site inside the `"health"` schema's `"db"` field.

**Conclusion for item 4's wire contract:** the JSON *schema* already promises `integer | null` for every one of `cass status --json`, `cass triage --json`, `cass health --json`, and the generic `"state"` introspection schema. **No schema change is required** for the `Option<i64>` change — the contract was already honest about nullability; only the *runtime* behavior (when null actually gets emitted) is currently dishonest, which is exactly the bug being fixed. Any consumer of these `--json` surfaces that only ever saw `null` on the documented "deliberately skipped, large DB" path and assumed a `null` never appears for a small/fast DB will start seeing `null` more often (on query failure too) — a behavior change within an already-declared type, not a breaking schema change.

### 5. `fresh_franken_count_retry` fallback trigger (`src/lib.rs:15463-15481`)

```
15463: if snapshot.conversation_count == 0 {
15464:     snapshot.conversation_count = fresh_franken_count_retry(...).unwrap_or(0);
...
15473: if snapshot.message_count == 0 {
15474:     snapshot.message_count = fresh_franken_count_retry(...).unwrap_or(0);
```

**What it assumes today:** "the first `COUNT(*)` came back 0" is the sole trigger for retrying with a fresh connection (`fresh_franken_count_retry` — presumably a "maybe the cached/pooled connection is stale, try a brand-new one" fallback). Because the first read's `0` is already ambiguous (real-empty vs. failed-query, per item 1), the retry fires in both cases today — which is arguably *accidentally* reasonable (a failed query gets one free retry on a fresh connection), but it's an accident, not a designed behavior, and it's silent either way.

**What it must become:** once `snapshot.conversation_count` is `Option<i64>`, the trigger needs to name which of the following it means, because they're no longer the same value:
- `Some(0)` — a genuine empty-table read; retrying on a fresh connection is genuinely questionable (why would a fresh connection see rows a live one didn't, for a real empty table?) but is presumably here to rule out a stale/cached view.
- `None` — the first query failed outright; retrying on a fresh connection is the obviously correct thing to do (the failure could be transient — lock contention, a stale handle), and doing so is likely the *actual intended* purpose of this fallback, going by its name.

The minimal, most defensible trigger change is `if snapshot.conversation_count.is_none_or(|n| n == 0)` (retry on either "failed" or "confirmed empty," preserving today's *effective* behavior exactly) — but the honest, bug-fixing alternative is to retry only on `None` (`if snapshot.conversation_count.is_none()`) and trust a confirmed `Some(0)` as a real answer, which is a behavior change: today a real empty conversations table always pays for a second full connection + query cycle; under `is_none()`-only, it would not. I did not find anything in the surrounding code or comments that states which of these two readings is intended — this is a genuine design decision, not something derivable from the current source.

## Proof boundary — what I did NOT establish

- **I did not build or run the code.** Hard limits forbid `cargo build`/`cargo test`; every claim above about what "must change" to compile is a manual read of Rust semantics (`Option<i64>` vs. `i64` comparisons, `.unwrap_or` vs. `.ok()`), not a compiler-verified fact. A `cargo check` (by whoever owns write access) would catch anything I missed.
- **The exact line numbers above are a single timestamped snapshot** (final synchronized re-check completed 2026-08-15T17:11:12Z) of a file under active, uncommitted, concurrent edit by another session in this same worktree (`git status --short -- src/lib.rs` showed `M src/lib.rs` throughout; `wc -l` grew 91,859 → 91,989 lines during this investigation). I directly observed one instance of the file shifting between an `rg` call and the immediately following `Read` call and treat that as proof the file was in motion the whole time, not just at that moment. Re-`rg` every citation before editing on top of it.
- **`response_schema_state_database()`'s second call site** inside the `"triage"` schema's nested `readiness.database` field — I confirmed by grep that a second `response_schema_state_database()` invocation exists in the schema-registry block (distinct from the `73697` site) but did not re-open and re-cite its exact line number in the final synchronized pass; I'm reporting its existence, not a verified current line number for it.
- **`run_stats` (`src/lib.rs:23902` on) and `run_diag` (`src/lib.rs:24229` on)** have their own, structurally identical `.unwrap_or(0)`-on-`COUNT(*)` bugs (confirmed present at `23953-23978` roughly, and their own JSON schemas — `"stats"` and `"diag"` in the introspection registry — declare `conversations`/`messages` as **non-nullable** `"type": "integer"`, unlike the three `StateDbSnapshot`-backed schemas in item 4). These are **not** `StateDbSnapshot` fields — they're independent local `i64` variables computed by separate SQL query logic in separate functions — so they are outside this lane's exact question ("what breaks if `StateDbSnapshot`'s fields change type"), and changing `StateDbSnapshot` will not touch them at all. I'm flagging them because the orchestrator's own task list (visible to me as inherited context) names "Fix the sibling honesty defects: a failed read must never render as good news" as pending work, and these two are exactly that shape — but I did not enumerate their downstream consumers with the same rigor as items 1-4, since they're a different struct-free code path entirely.
- **I did not determine the "right" semantic answer** for the two forks flagged as genuine design decisions: (a) whether `message_count == None` should keep skipping the live Tantivy doc-count probe at `16451`/`16464` (treating unknown like zero) or force the probe defensively, and (b) whether the `fresh_franken_count_retry` trigger at `15463`/`15473` should retry on `None` only or on `None`-or-`Some(0)`. Both are stated as open questions above, not resolved.
