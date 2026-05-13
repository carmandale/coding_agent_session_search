**Findings**
1. The C4 section contradicts itself.  
Surface 3 says “plist rewrite ... deferred,” then immediately makes plist mutation the C4 mechanism: `KeepAlive=true` → `{SuccessfulExit=false}`. The task file has the same contradiction: Group E says plist rewrites are deferred, then T23 changes the plist ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/013-cass-rebuild-stall-asupersync/tasks.md:51), [tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/013-cass-rebuild-stall-asupersync/tasks.md:56)). Decide one: either plist mutation is in scope and must be tested/reverted, or it is out of scope and C4 needs another mechanism.

2. T12 still runs a diagnostic corpus command without `--json` before the gating fix lands.  
T5 is fixed, but T12 says `CASS_INDEX_STALL_DETECT_SECS=60 cass index --full` ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/013-cass-rebuild-stall-asupersync/tasks.md:30)). Group C runs before Group E, so current source still requires structured output: `emit_progress_events = structured_output && ...` at [src/lib.rs](/Users/dalecarman/dev/coding_agent_session_search/src/lib.rs:72261). Make T12 match the revised investigation path: `cass index --full --json`.

3. The new per-source ledger depends on a `skip_reason` surface that does not exist for raw-mirror/indexer files.  
T19a says missing files can be accepted if the raw-mirror manifest has `skip_reason` ([tasks.md](/Users/dalecarman/dev/coding_agent_session_search/specs/013-cass-rebuild-stall-asupersync/tasks.md:47)). But `RawMirrorManifestFile` has provider/path/blob/db_links fields and no skip reason ([src/raw_mirror.rs](/Users/dalecarman/dev/coding_agent_session_search/src/raw_mirror.rs:94)). The `skip_reason` hits I found are doctor/action surfaces, not source-ingest skips. Add a real durable skip ledger/table/NDJSON emitted by the indexer, or classify missing files only from actual existing evidence.

**What I Verified**
I rechecked the live launchd plist (`KeepAlive => true`), the current watchdog gating in `src/lib.rs`, the updated `tasks.md`, the raw-mirror manifest schema, and searched for `skip_reason` across `src`, `tests`, and this spec. The `--full` correction and file-level reconciliation idea are good; the remaining mechanics are not yet internally consistent.

VERDICT: REVISE