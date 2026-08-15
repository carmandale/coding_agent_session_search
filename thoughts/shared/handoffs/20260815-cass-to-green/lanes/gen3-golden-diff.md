# Lane: gen3-golden-diff (bead coding_agent_session_search-a4xe1)

Read-only measurement lane, generation 3. Only write: this file (append-only).
Repo: `/Users/dalecarman/dev/coding_agent_session_search/.claude/worktrees/cass-to-green-c6bfb589`
Branch: `worktree-cass-to-green-c6bfb589`
HEAD at start: `9d4814d2`
Started: 2026-08-15

Task: re-measure the golden_robot_json failures against the previous
generation's analysis (`lanes/golden-robot-json.md`), specifically its
section 7a repair plan. Do NOT fix, do NOT run UPDATE_GOLDENS=1.

---

## 0. HEAD drift since the previous lane

Previous lane's HEAD was `74a72233`. Current HEAD is `9d4814d2`, three commits
ahead. Diffed the commit range:

```
$ git log --oneline 74a72233..HEAD -- tests/ src/topology_budget.rs src/lib.rs src/indexer/mod.rs src/storage/sqlite.rs
8dcd245b fix(coverage): bound the whole coverage read, and stop reporting a
          failed read as complete
```

Only one commit in that range touches source relevant to golden_robot_json
(`src/lib.rs`), and its own commit message states: "golden_robot_json is
unchanged at 28 passed / 9 failed, confirming the change is
behaviour-preserving on the happy path." The other two commits in the range
touch only `thoughts/shared/handoffs/**` (agent logs, receipts, other lanes'
files) — confirmed via `git diff --name-only 74a72233 HEAD`, no `tests/` or
`src/` paths outside `src/lib.rs`. So the previous lane's chronology/attribution
findings (section 2-3) are unaffected by the HEAD move; I re-verify the actual
test run and diffs myself below rather than trusting that carry-forward.

---

## 1. The run — executed here, full output captured

```
$ export PATH="$HOME/.rustup/toolchains/nightly-aarch64-apple-darwin/bin:$PATH"
$ CARGO_TARGET_DIR=/tmp/cass-gen3-golden-target cargo test --test golden_robot_json
```

Run via a python subprocess wrapper (900s timeout, captured to
`/tmp/cass-gen3-golden-scratch/run.log`, 891 lines) rather than piping to
`tail` per the lane instructions. Result:

```
EXIT 101   ELAPSED 112.2s
```

From the log itself:

```
$ rg -n 'test result:|FAILED' /tmp/cass-gen3-golden-scratch/run.log
...
test result: FAILED. 28 passed; 9 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.98s
```

Failing set (log lines 163-171), **identical to the bead's list and to the
generation-2 lane's list**:

```
diag_json_matches_golden
diag_quarantine_json_matches_golden
health_json_matches_golden
health_shape_matches_golden
stats_json_happy_path_matches_golden
stats_json_happy_path_shape_matches_golden
status_quarantine_full_json_matches_golden
status_quarantine_json_matches_golden
status_shape_matches_golden
```

28 passed / 9 failed — matches the number cited in commit `8dcd245b`'s own
message ("golden_robot_json is unchanged at 28 passed / 9 failed"), which is
independent confirmation that the coverage-fix commit between the two lanes'
HEADs did not touch this surface.

Nine `.actual` files were written under `tests/golden/robot/`, all
timestamped `Aug 15 06:38` (this run):

```
$ /bin/ls -la tests/golden/robot/*.actual
diag_quarantine.json.actual         14519 bytes
diag.json.actual                     2280 bytes
health_shape.json.actual            37257 bytes
health.json.actual                  20932 bytes
stats_full_payload_shape.json.actual 2432 bytes
stats_full_payload.json.actual       1023 bytes
status_quarantine_full.json.actual  30249 bytes
status_quarantine.json.actual       30249 bytes
status_shape.json.actual            41926 bytes
```

These are gitignored (`.gitignore:271`); `git status --short` shows only my
own log file and the pre-existing `.agent-state/` as dirty in this worktree —
the `.actual` writes did not dirty the tracked tree.

---

## 2. Per-case diff, hunk count, and classification

Diffed each golden against its `.actual` with `diff -u`, hunk count = number
of `@@` lines. Full diffs saved under `/tmp/cass-gen3-golden-scratch/<name>.diff`.

| # | file | hunks | classification |
|---|---|---|---|
| 1 | `health.json` | 2 | ONLY connector_coverage |
| 2 | `health_shape.json` | 2 | ONLY connector_coverage |
| 3 | `stats_full_payload.json` | 1 | ONLY connector_coverage |
| 4 | `stats_full_payload_shape.json` | 1 | ONLY connector_coverage |
| 5 | `status_shape.json` | 3 | MIXED — connector_coverage (1 hunk) + macOS host drift (2 hunks) |
| 6 | `status_quarantine.json` | 4 | MIXED — connector_coverage (1 hunk) + macOS host drift (3 hunks) |
| 7 | `status_quarantine_full.json` | 4 | MIXED — connector_coverage (1 hunk) + macOS host drift (3 hunks), byte-identical diff to #6 |
| 8 | `diag.json` | 1 | ONLY macOS host drift (`platform.os`/`platform.arch`) — no connector_coverage anywhere |
| 9 | `diag_quarantine.json` | 1 | ONLY macOS host drift (`platform.os`/`platform.arch`) — no connector_coverage anywhere, byte-identical diff to #8 |

None of the nine falls in the "something else" bucket — every byte of every
diff is accounted for by one of the two known causes (connector_coverage
staleness, or macOS host topology/platform values). This matches the
generation-2 lane's finding.

### Case 1 — `health.json` (2 hunks, verbatim)

```diff
@@ -45,6 +45,12 @@
     "index not initialized yet"
   ],
   "latency_ms": "[LATENCY_MS]",
+  "connector_coverage": {
+    "checked": false,
+    "complete": null,
+    "incomplete_connectors": [],
+    "floors": []
+  },
   "rebuild_progress": {
     "active": false,
     "stalled": false,
@@ -391,6 +397,12 @@
       "counts_skipped": false,
       "open_skipped": false
     },
+    "connector_coverage": {
+      "checked": false,
+      "complete": null,
+      "incomplete_connectors": [],
+      "floors": []
+    },
     "pending": {
       "sessions": 0,
       "watch_active": false,
```

Both hunks are pure additions of the identical 6-line `connector_coverage`
block. No host-specific byte anywhere in this diff.

### Case 2 — `health_shape.json` (2 hunks, verbatim)

```diff
@@ -71,6 +71,29 @@
     },
     "latency_ms": {
       "type": "string"
+    },
+    "connector_coverage": {
+      "type": "object",
+      "properties": {
+        "checked": { "type": "boolean" },
+        "complete": { "type": "null" },
+        "incomplete_connectors": { "type": "array", "items": { "type": "unknown" } },
+        "floors": { "type": "array", "items": { "type": "unknown" } }
+      }
     },
     "rebuild_progress": {
...
@@ -975,6 +998,29 @@
             }
           }
         },
+        "connector_coverage": {
+          "type": "object",
+          ... (same schema block, nested)
+        },
         "pending": {
```

(Reflowed for brevity; every added line is schema description of the same
`connector_coverage` shape, no host-specific token.) Pure addition, twice.

### Case 3 — `stats_full_payload.json` (1 hunk, verbatim)

```diff
@@ -37,5 +37,11 @@
     "oldest_source_mtime_ms": null,
     "newest_source_mtime_ms": null
   },
+  "connector_coverage": {
+    "checked": false,
+    "complete": null,
+    "incomplete_connectors": [],
+    "floors": []
+  },
   "db_path": "[TEST_HOME]/search_demo_data/agent_search.db"
 }
\ No newline at end of file
```

Pure addition. (The `\ No newline at end of file` marker here is `diff`
reporting that BOTH sides lack a trailing newline — confirmed in section 3
below — not a difference between them.)

### Case 4 — `stats_full_payload_shape.json` (1 hunk, verbatim)

```diff
@@ -93,6 +93,29 @@
         }
       }
     },
+    "connector_coverage": {
+      "type": "object",
+      "properties": { ... same 4-field schema as case 2 ... }
+    },
     "db_path": {
       "type": "string"
     }
```

Pure addition.

### Case 5 — `status_shape.json` (3 hunks, verbatim)

```diff
@@ -270,10 +270,10 @@
               "type": "integer"
             },
             "controller_loadavg_high_watermark_1m": {
-              "type": "number"
+              "type": "null"
             },
             "controller_loadavg_low_watermark_1m": {
-              "type": "number"
+              "type": "null"
             },
             "runtime": {
               "type": "null"
@@ -578,6 +578,29 @@
         }
       }
     },
+    "connector_coverage": {
+      "type": "object",
+      "properties": { ... same 4-field schema ... }
+    },
     "policy_registry": {
       "type": "object",
@@ -684,10 +707,10 @@
               "type": "integer"
             },
             "memory_total_bytes": {
-              "type": "integer"
+              "type": "null"
             },
             "memory_available_bytes": {
-              "type": "integer"
+              "type": "null"
             }
           }
         },
```

Hunk 1 (`controller_loadavg_*_watermark_1m`: `"number"`→`"null"`) and hunk 3
(`memory_total_bytes`/`memory_available_bytes`: `"integer"`→`"null"`) are
macOS host drift — on macOS these values come back `None` (see
`src/topology_budget.rs:568-588`, cited by the prior lane) so the shape
inference sees `null` instead of a number/integer. Hunk 2 is the pure
connector_coverage schema addition. **This is the case named by the previous
lane as one of the three "connector_coverage-only-hunk" files — confirmed
correct: hunk 2, and only hunk 2, is the connector_coverage change; hunks 1
and 3 are unrelated host drift that a wholesale replace would wrongly bake
in.**

### Case 6 — `status_quarantine.json` (4 hunks, verbatim)

```diff
@@ -193,6 +193,12 @@
     "quarantine_files": [],
     "newest_last_attempt_at_ms": null,
     "recommended_action": null
+  },
+  "connector_coverage": {
+    "checked": false,
+    "complete": null,
+    "incomplete_connectors": [],
+    "floors": []
   },
   "policy_registry": {
     "schema_version": "1",
@@ -254,7 +260,7 @@
   "topology_budget": {
     "schema_version": "1",
     "topology": {
-      "source": "linux_sysfs",
+      "source": "fallback",
       "topology_class": "many_core_single_socket",
       "logical_cpus": 128,
       "physical_cores": 64,
@@ -262,13 +268,13 @@
       "numa_nodes": 1,
       "llc_groups": 8,
       "smt_threads_per_core": 2,
-      "memory_total_bytes": "[LIVE_BYTES]",
-      "memory_available_bytes": "[LIVE_BYTES]"
+      "memory_total_bytes": null,
+      "memory_available_bytes": null
     },
     "reserved_core_policy": {
       "reserved_cores": "[LIVE_COUNTER]",
-      "policy": "max(default, locality*2_on_large_hosts, smt_width, logical/12) capped at 16",
-      "reason": "reserve 16 of 128 logical CPUs for interactive work, IO, and NUMA/LLC service headroom"
+      "policy": "current conservative default",
+      "reason": "topology could not be derived, so cass preserves existing worker and RAM defaults"
     },
     "advisory_budgets": {
       "shard_builders": "[LIVE_COUNTER]",
@@ -287,12 +293,11 @@
       "cache_cap_bytes": "[LIVE_BYTES]",
       "max_inflight_bytes": "[LIVE_BYTES]"
     },
-    "fallback_active": false,
-    "decision_reason": "planned from ManyCoreSingleSocket: 128 logical CPUs, 64 physical cores, 1 socket(s), 1 NUMA node(s), 8 LLC group(s)",
+    "fallback_active": true,
+    "decision_reason": "using conservative defaults: linux sysfs topology is unavailable on this platform",
     "proof_notes": [
-      "advisory only: live controllers keep current conservative settings until explicitly wired",
-      "CPU budgets prefer physical cores and LLC/NUMA locality over SMT oversubscription",
-      "RAM caps scale only when MemAvailable is large enough to preserve broad host headroom"
+      "fallback is intentionally isomorphic to current defaults for live rebuild budgets",
+      "no /sys-derived CPU locality assumptions are made in fallback mode"
     ]
   },
   "doctor_summary": {
```

Hunk 1 is the pure connector_coverage addition. Hunks 2-4 are all inside
`topology_budget` and are macOS host drift: 8 distinct fields differ
(`source`, `memory_total_bytes`, `memory_available_bytes`,
`reserved_core_policy.policy`, `reserved_core_policy.reason`,
`fallback_active`, `decision_reason`, `proof_notes`) — matches the prior
lane's "8 sites" count exactly. **Confirmed: taking only the connector_coverage
hunk (hunk 1) and leaving the rest is the correct, and only correct, edit.**

### Case 7 — `status_quarantine_full.json` (4 hunks)

Diff is **byte-for-byte identical in content** to case 6. Verified by diffing
the two `.diff` files directly, capturing `diff`'s own exit status rather
than a pipeline tail's (the zsh `$?`-after-a-pipeline trap this repo's own
rules warn about — first attempt piped through `head` and silently reported
`head`'s rc=0, which would have been a false "confirmed identical"; redone
without the pipe):

```
$ out=$(diff status_quarantine.diff status_quarantine_full.diff); rc=$?
$ echo "rc=$rc"; echo "$out"
rc=1
1,2c1,2
< --- status_quarantine.json.golden	2026-08-15 06:00:17
< +++ status_quarantine.json.actual	2026-08-15 06:38:01
---
> --- status_quarantine_full.json.golden	2026-08-15 06:00:17
> +++ status_quarantine_full.json.actual	2026-08-15 06:38:01
```

rc=1 because the two `.diff` files differ — but ONLY in the unified-diff
header lines (source filenames and mtimes). Nothing beyond line 2 differs, so
every hunk (all 4) and every line of hunk content is identical between the
two files. Also both `.actual` files are exactly 30249 bytes. Same
classification and same confirmation as case 6.

### Case 8 — `diag.json` (1 hunk, verbatim)

```diff
@@ -1,8 +1,8 @@
 {
   "version": "[VERSION]",
   "platform": {
-    "os": "linux",
-    "arch": "x86_64"
+    "os": "macos",
+    "arch": "aarch64"
   },
```

That is the entire diff. **No `connector_coverage` anywhere in this file's
diff.** Confirms the prior lane: this failure is pure host-platform drift
(`platform.os`/`platform.arch`), unrelated to e3ed01f0, and touching this
golden is out of scope for a4xe1.

### Case 9 — `diag_quarantine.json` (1 hunk)

**Correction to an over-strong claim I first drafted here**: this is NOT
byte-identical to case 8. Diffing the two `.diff` files directly (same
non-piped exit-status method as case 7):

```
$ out=$(diff diag.diff diag_quarantine.diff); rc=$?
$ echo "rc=$rc"; echo "$out"
rc=1
1,2c1,2
< --- diag.json.golden	2026-08-15 06:00:17
< +++ diag.json.actual	2026-08-15 06:38:01
---
> --- diag_quarantine.json.golden	2026-08-15 06:00:17
> +++ diag_quarantine.json.actual	2026-08-15 06:38:01
13c13
<      "data_dir": "[TEST_HOME]/coding-agent-search",
---
>      "data_dir": "[TEST_HOME]/cass-data",
```

Line 13 is a **context line** (unchanged on both sides of each file's own
diff, part of `diff -u`'s 3-line context window, not a `+`/`-` line) — the two
fixtures have different `data_dir` values (`coding-agent-search` vs
`cass-data`) because `diag_quarantine` is a different CLI invocation/test
fixture than `diag`. The actual **hunk** (`@@ -1,8 +1,8 @@`, the
`platform.os`/`platform.arch` swap) is identical between the two files;
only a context line adjacent to it differs, for an unrelated and expected
reason (different `--home` fixture path). Classification and confirmation
against the handoff instruction (section 4) are unaffected: still pure
host-platform drift, still zero connector_coverage.

---

## 3. Trailing-byte (final newline) parity, per file

Checked `tail -c N | od -c` on golden vs `.actual` for every file, not just
the four wholesale-replace candidates:

| file | golden tail | actual tail | match |
|---|---|---|---|
| `health.json` | `}` (no trailing `\n`) | `}` (no trailing `\n`) | yes |
| `health_shape.json` | `}` (no trailing `\n`) | `}` (no trailing `\n`) | yes |
| `stats_full_payload.json` | `db"` `\n` `}` (no trailing `\n` after final `}`) | same | yes |
| `stats_full_payload_shape.json` | `}` (no trailing `\n`) | `}` (no trailing `\n`) | yes |
| `status_shape.json` | `}` (no trailing `\n`) | `}` (no trailing `\n`) | yes |
| `status_quarantine.json` | `}` (no trailing `\n`) | `}` (no trailing `\n`) | yes |
| `status_quarantine_full.json` | `}` (no trailing `\n`) | `}` (no trailing `\n`) | yes |
| `diag.json` | `]` `\n` `}` (no trailing `\n`) | same | yes |
| `diag_quarantine.json` | `}` (no trailing `\n`) | `}` (no trailing `\n`) | yes |

All nine match. For the 4 files a wholesale `.actual`→`.golden` copy is
being considered for (health, health_shape, stats_full_payload,
stats_full_payload_shape), a plain `cp` would be byte-safe with respect to
trailing bytes. (Whether a wholesale copy is otherwise safe is answered in
section 2 above — for these four, yes; the trailing-byte check is necessary
but not the only condition.)

---

## 4. Re-check against the handoff's instruction (step 4 deliverable)

The generation-2 lane's section 7a instructed:

```
replace wholesale from .actual (4): health.json, health_shape.json,
  stats_full_payload.json, stats_full_payload_shape.json
take ONLY the connector_coverage hunk (3): status_shape.json,
  status_quarantine.json, status_quarantine_full.json
do not touch (2): diag.json, diag_quarantine.json
```

| file | instruction | my measurement | verdict |
|---|---|---|---|
| `health.json` | replace wholesale | 2 hunks, both pure connector_coverage, zero host bytes | **CONFIRMS** |
| `health_shape.json` | replace wholesale | 2 hunks, both pure connector_coverage schema, zero host bytes | **CONFIRMS** |
| `stats_full_payload.json` | replace wholesale | 1 hunk, pure connector_coverage, zero host bytes | **CONFIRMS** |
| `stats_full_payload_shape.json` | replace wholesale | 1 hunk, pure connector_coverage schema, zero host bytes | **CONFIRMS** |
| `status_shape.json` | take ONLY the connector_coverage hunk | 3 hunks total; hunk 2 is connector_coverage, hunks 1 and 3 are macOS-host `"number"`/`"integer"`→`"null"` shape drift | **CONFIRMS** |
| `status_quarantine.json` | take ONLY the connector_coverage hunk | 4 hunks total; hunk 1 is connector_coverage, hunks 2-4 (8 fields) are macOS host topology drift | **CONFIRMS** |
| `status_quarantine_full.json` | take ONLY the connector_coverage hunk | same as `status_quarantine.json`, byte-identical diff | **CONFIRMS** |
| `diag.json` | do not touch | 1 hunk, pure `platform.os`/`platform.arch` host drift, no connector_coverage | **CONFIRMS** |
| `diag_quarantine.json` | do not touch | 1 hunk, pure `platform.os`/`platform.arch` host drift, no connector_coverage | **CONFIRMS** |

**All nine confirm the generation-2 lane's 7a instruction exactly.** For the
four "replace wholesale" files I specifically checked whether ANY
macOS-host-specific byte was present in the diff (the task's stated
poison-the-Linux-contract concern) — none was found in any of the four; every
added byte in all four is the identical 4-key `connector_coverage` value or
its schema description, which contains no host-derived value.

---

## 5. Status

Lane complete, read-only. Only file written: this log. Nine gitignored
`.actual` files exist under `tests/golden/robot/` as the test's own
documented artifact (`.gitignore:271`); `git status --short` at close
confirms they are not tracked-tree dirt. No git mutation, no `UPDATE_GOLDENS`
run, no `cass` invocation, no edit to any golden or source file.

