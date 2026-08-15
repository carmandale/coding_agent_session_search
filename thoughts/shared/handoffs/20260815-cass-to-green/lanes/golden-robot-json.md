# Lane: golden_robot_json (bead coding_agent_session_search-a4xe1)

Read-only grounding lane. Only write: this file. Append-only.
Worktree: `/Users/dalecarman/dev/coding_agent_session_search/.claude/worktrees/cass-to-green-c6bfb589`
HEAD at start: `74a72233` (branch `worktree-cass-to-green-c6bfb589`)
Started: 2026-08-15

---

## 0. Bead read in full

`br show` failed in this worktree — the worktree has no `.beads/beads.db`:

```
$ br show coding_agent_session_search-a4xe1
Error: Sync conflict: Refusing storage open because pending sync-merge state
could not be inspected under database-family authority ... the authorized database is missing
```

`br --no-db show coding_agent_session_search-a4xe1` worked (reads `.beads/issues.jsonl`).
Bead is P1 OPEN, type bug, owner dalecarman, created + updated 2026-08-14.

Its stated content (verbatim summary of the claims I then went on to test):

- 9 of 37 tests in `tests/golden_robot_json.rs` fail on main, since 2026-08-10.
- FAILING: `diag_json`, `diag_quarantine_json`, `health_json`, `health_shape`,
  `status_shape`, `status_quarantine_json`, `stats_json_happy_path`,
  `stats_json_happy_path_shape`, `status_quarantine_full_json`.
- Attribution: the whole drift is one added block, twice —
  `"connector_coverage": {"checked": false, "complete": null, "incomplete_connectors": [], "floors": []}`
- Chronology: `health.json.golden` last touched by `fb75daab` (2026-05-28);
  `status_quarantine.json.golden` by `b4463276` (2026-06-01); `e3ed01f0` is
  2026-08-10 and touched no goldens.
- CONSTRAINT: "DO NOT run the UPDATE_GOLDENS=1 regeneration ... on the laptop.
  The status goldens carry a SECOND, environmental drift component:
  `"source": "linux_sysfs"` vs `"fallback"`, and memory_total_bytes /
  memory_available_bytes as `[LIVE_BYTES]` vs null."
- Proposed fix: (a) regenerate on a Linux runner, or (b) give the host-topology
  block the same `[LIVE_*]` normalization the memory fields already have.
- Evidence cited: `thoughts/shared/handoffs/20260814-cass-repair-to-green/generation-2-log.md` §2a.

---

## 1. Paths

- Test file: `tests/golden_robot_json.rs` (2,024 lines)
- Goldens: `tests/golden/robot/*.json.golden` (37 files)
- Provenance / regeneration doc: `tests/golden/PROVENANCE.md`
- Scrubber: `tests/golden_robot_json.rs::scrub_robot_json` (ends line 826) plus
  `normalize_live_robot_values` (line 329)
- Comparator: `tests/golden_robot_json.rs::assert_golden` (line 830). On mismatch
  it writes `<golden>.actual` beside the golden and panics. `.actual` files are
  gitignored (`.gitignore:271  tests/golden/**/*.actual`), so running the test
  leaves no tracked-file dirt.

---

## 2. What e3ed01f0 actually changed (CONFIRMED)

```
$ git show --name-only --format="" e3ed01f0
src/indexer/mod.rs
src/lib.rs
src/storage/sqlite.rs

$ git show --stat e3ed01f0 -- tests/
(empty output, exit 0)
```

Three source files, 932 insertions. **No test and no golden was touched.** The
commit message itself says: "src/lib.rs: connector_coverage blocks in
`cass stats --json`, `cass status` and `cass health`".

Chronology confirmed independently:

```
$ git log -1 --format='%h %ad %s' --date=short -- tests/golden/robot/health.json.golden
fb75daab 2026-05-28 fix(cass#256): F3 follow-ups — ... + refresh JSON goldens

$ git log -1 --format='%h %ad %s' --date=short -- tests/golden/robot/status_quarantine.json.golden
b4463276 2026-06-01 fix(cass): align upstream sync with local clippy
```

Cross-check on the goldens themselves — **not one golden contains the string
`connector_coverage`**:

```
$ rg -c 'topology_budget|connector_coverage' tests/golden/robot/{health,health_shape,status_shape,status_quarantine,status_quarantine_full,diag,diag_quarantine,stats_full_payload,stats_full_payload_shape}.json.golden
tests/golden/robot/status_quarantine.json.golden:1
tests/golden/robot/status_quarantine_full.json.golden:1
tests/golden/robot/status_shape.json.golden:1
```

(The three hits are all `topology_budget`; `connector_coverage` matches zero
files.) So every golden predates the block e3ed01f0 added.

---

## 3. WHY "do not regenerate on macOS" — the stated reason, and the code proof

### 3a. Where the constraint is actually written down

Searched `AGENTS.md`, `docs/`, `tests/golden/PROVENANCE.md`, `tests/`, and
`thoughts/`. The macOS constraint is stated in exactly ONE place in the repo:

- `thoughts/shared/handoffs/20260814-cass-repair-to-green/generation-2-log.md:210-215`

```
`"source": "linux_sysfs"` against `"fallback"`, and memory_total_bytes / memory_available_bytes
as `[LIVE_BYTES]` against `null`. The goldens were generated on a Linux host and CI runs Linux.
Running the `UPDATE_GOLDENS=1` regeneration the failure message suggests, **on macOS**, would bake
macOS topology into goldens that CI compares on Linux — turning one pre-existing red into a
permanent CI red. Regeneration belongs on a Linux runner, or the topology block needs the same
```

That is the same author and the same generation as the bead, so it is ONE
source, not two. `AGENTS.md` and `tests/golden/PROVENANCE.md` prescribe the
regeneration command but say nothing about host OS:

- `AGENTS.md:443` — "run `UPDATE_GOLDENS=1 rch exec -- env CARGO_TARGET_DIR=... cargo test --test golden_robot_json ...`"
- `tests/golden/PROVENANCE.md` "## Regeneration" — same `rch exec` form, with
  the stated reason being CPU contention, not host OS.

`rch` is the Remote Compilation Helper (`AGENTS.md:980-1003`): a fleet of 8
remote Contabo VPS workers. Prescribing `rch exec` is therefore *implicitly* a
Linux-host prescription, but AGENTS.md never says so and never mentions goldens
being host-sensitive. **The OS constraint is not written in any authoritative
repo doc.** It exists only in the 2026-08-14 handoff.

### 3b. The constraint is nonetheless TRUE, and the code proves it

`src/topology_budget.rs:138-156`:

```rust
pub(crate) fn inspect_host_topology_budget() -> TopologyBudgetPlan {
    let defaults = TopologyPlannerDefaults::from_current_process();
    #[cfg(target_os = "linux")]
    {
        let memory = read_meminfo_snapshot(Path::new("/proc/meminfo")).unwrap_or(...);
        topology_budget_for_sysfs(Path::new("/sys"), memory, defaults)
    }
    #[cfg(not(target_os = "linux"))]
    {
        fallback_plan(
            fallback_topology(None, defaults.available_parallelism),
            defaults,
            "linux sysfs topology is unavailable on this platform".to_string(),
        )
    }
}
```

On macOS the non-Linux arm is compiled and `memory` is passed as `None`.
`src/topology_budget.rs:568-588`:

```rust
fn fallback_topology(memory: Option<MemorySnapshot>, available_parallelism: usize) -> TopologySnapshot {
    let memory = memory.unwrap_or(MemorySnapshot { total_bytes: None, available_bytes: None });
    TopologySnapshot {
        source: TopologySource::Fallback,
        topology_class: TopologyClass::Unknown,
        ...
        memory_total_bytes: memory.total_bytes,      // None on macOS
        memory_available_bytes: memory.available_bytes,  // None on macOS
    }
}
```

So the macOS payload differs from the Linux payload in the `topology_budget`
block. Cross-referencing the scrubber tells you which of those differences the
test can absorb and which it cannot.

`normalize_live_robot_values` (`tests/golden_robot_json.rs:329-442`) pins by key:
`topology_class`→"many_core_single_socket", `logical_cpus`→128,
`physical_cores`→64, `sockets`/`numa_nodes`→1, `llc_groups`→8,
`smt_threads_per_core`→2. Those absorb fine.

`scrub_robot_json` rule at lines 793-805 rewrites `memory_total_bytes` /
`memory_available_bytes` only when the regex `"{key}"\s*:\s*("?\d+"?)` matches —
**digits required**. A `null` does not match, so it is left as `null`.

The unabsorbed differences, read off the golden vs. the code:

| field | golden (`status_quarantine.json.golden`) | macOS value | normalized? |
|---|---|---|---|
| `topology.source` | `"linux_sysfs"` (line 257) | `"fallback"` | **no** |
| `topology.memory_total_bytes` | `"[LIVE_BYTES]"` (line 264) | `null` | no (regex needs digits) |
| `topology.memory_available_bytes` | `"[LIVE_BYTES]"` (line 265) | `null` | no |
| `reserved_core_policy.policy` | `"max(default, locality*2_on_large_hosts, smt_width, logical/12) capped at 16"` (line 269) | `"current conservative default"` (`topology_budget.rs:597`) | **no** |
| `reserved_core_policy.reason` | `"reserve 16 of 128 logical CPUs for interactive work, IO, and NUMA/LLC service headroom"` (line 270) | `"topology could not be derived, so cass preserves existing worker and RAM defaults"` (`topology_budget.rs:598`) | no — the normalizer at `golden_robot_json.rs:424` only fires on text starting `"reserve "` and containing `" logical CPUs "` |

So the constraint's reason is real, and it is **broader than the bead states**:
the bead names 3 fields (`source`, two memory fields); the code shows at least
5, adding `reserved_core_policy.policy` and `reserved_core_policy.reason`.

### 3c. But the constraint's PREMISE is only one third true

The handoff says "CI runs Linux." That is not what the workflow says.
`.github/workflows/ci.yml:359-366`:

```yaml
  test-rust:
    name: Rust Tests (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
```

and `ci.yml:397-398` runs the whole suite (`cargo test --features "qr encryption
backtrace" --verbose -- --nocapture`) on each leg. `tests/golden_robot_json.rs`
contains **no** `cfg(target_os` gate anywhere:

```
$ rg -n 'cfg\(target_os|cfg\(not\(target_os|linux' tests/golden_robot_json.rs
(no output)
```

So the three `topology_budget`-bearing goldens (`status_quarantine`,
`status_quarantine_full`, `status_shape`) should already be failing on the
`macos-latest` and `windows-latest` legs, for the host-topology reason, and
should have been failing there since before e3ed01f0. **UNVERIFIED against a
real CI run** — I have not read a CI log; this is read from the workflow file
plus the code above.

This matters for the fix choice: option (a) "regenerate on Linux" fixes the
ubuntu leg and leaves the macOS and Windows legs red. Only option (b)
(normalize the host-topology fields) makes all three legs green.

---

## 4. The run — executed, full output captured

```
$ export PATH="$HOME/.rustup/toolchains/nightly-aarch64-apple-darwin/bin:$PATH"
$ CARGO_TARGET_DIR=/tmp/cass-lane-golden cargo test --test golden_robot_json -- --nocapture
...
test result: FAILED. 28 passed; 9 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
error: test failed, to rerun pass `--test golden_robot_json`
EXIT=101
```

Full log: `/tmp/cass-lane-golden-run.log` (871 lines; the build dominated, the
tests themselves ran in 3.09s).

HEAD **does compile** — the crate and the test target both built clean. That
rules out the "HEAD does not even compile" branch.

Failing set, verbatim from the log (lines 857-866) — **identical to the nine the
bead names**:

```
failures:
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

The run wrote nine `.actual` dumps under `tests/golden/robot/`. They are
gitignored (`.gitignore:271`) and are the test's own documented artifact, so the
tracked tree is not dirtied.

---

## 5. Per-case classification — THE DELIVERABLE

Every one of the nine is a **STALE GOLDEN or a HOST-ENVIRONMENT MISMATCH. None
is a code regression.** But they do not all have the same cause, and the bead's
attribution is wrong for two of them.

| # | test | golden | `connector_coverage` missing (STALE, caused by e3ed01f0) | macOS-host drift (NOT e3ed01f0) |
|---|---|---|---|---|
| 1 | `health_json_matches_golden` | `health.json` | yes, x2 | — |
| 2 | `health_shape_matches_golden` | `health_shape.json` | yes, x2 | — |
| 3 | `stats_json_happy_path_matches_golden` | `stats_full_payload.json` | yes, x1 | — |
| 4 | `stats_json_happy_path_shape_matches_golden` | `stats_full_payload_shape.json` | yes, x1 | — |
| 5 | `status_shape_matches_golden` | `status_shape.json` | yes, x1 | yes — 4 fields |
| 6 | `status_quarantine_json_matches_golden` | `status_quarantine.json` | yes, x1 | yes — 8 sites |
| 7 | `status_quarantine_full_json_matches_golden` | `status_quarantine_full.json` | yes, x1 | yes — 8 sites |
| 8 | `diag_json_matches_golden` | `diag.json` | **NO** | yes — os/arch only |
| 9 | `diag_quarantine_json_matches_golden` | `diag_quarantine.json` | **NO** | yes — os/arch only |

### Quoted diff, case 1 — `health.json`: STALE GOLDEN, nothing else

```diff
$ diff -u tests/golden/robot/health.json.golden tests/golden/robot/health.json.actual
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
```

That is the **entire** diff. The code is right — e3ed01f0's commit message says
it added exactly this block to `cass health` — and the golden predates it by ten
weeks. Verdict: **golden stale, code correct.**

### Quoted diff, case 6 — `status_quarantine.json`: STALE *and* host-drifted

```diff
$ diff -u tests/golden/robot/status_quarantine.json.golden tests/golden/robot/status_quarantine.json.actual
@@ -193,6 +193,12 @@
     "recommended_action": null
+  },
+  "connector_coverage": {
+    "checked": false,
+    "complete": null,
+    "incomplete_connectors": [],
+    "floors": []
   },
@@ -254,7 +260,7 @@
   "topology_budget": {
     "topology": {
-      "source": "linux_sysfs",
+      "source": "fallback",
       "topology_class": "many_core_single_socket",     <- absorbed by the normalizer
       "logical_cpus": 128,                             <- absorbed
       ...
-      "memory_total_bytes": "[LIVE_BYTES]",
-      "memory_available_bytes": "[LIVE_BYTES]"
+      "memory_total_bytes": null,
+      "memory_available_bytes": null
     },
     "reserved_core_policy": {
-      "policy": "max(default, locality*2_on_large_hosts, smt_width, logical/12) capped at 16",
-      "reason": "reserve 16 of 128 logical CPUs for interactive work, IO, and NUMA/LLC service headroom"
+      "policy": "current conservative default",
+      "reason": "topology could not be derived, so cass preserves existing worker and RAM defaults"
     },
-    "fallback_active": false,
-    "decision_reason": "planned from ManyCoreSingleSocket: 128 logical CPUs, ...",
+    "fallback_active": true,
+    "decision_reason": "using conservative defaults: linux sysfs topology is unavailable on this platform",
     "proof_notes": [
-      "advisory only: live controllers keep current conservative settings until explicitly wired",
-      "CPU budgets prefer physical cores and LLC/NUMA locality over SMT oversubscription",
-      "RAM caps scale only when MemAvailable is large enough to preserve broad host headroom"
+      "fallback is intentionally isomorphic to current defaults for live rebuild budgets",
+      "no /sys-derived CPU locality assumptions are made in fallback mode"
     ]
```

Two independent causes in one file. The first hunk is the same stale-golden
defect as case 1. Everything from `topology_budget` down is the macOS host, and
would vanish on any Linux host — `plan_for_topology` sets `policy` to the fixed
string at `src/topology_budget.rs:518` and `proof_notes` to the fixed vector at
`:469`, and the normalizer folds the size-dependent parts (`logical_cpus` -> 128
at `golden_robot_json.rs:366`, `decision_reason` via the `"planned from "`
prefix rule at `:418`). So the scrubber is *Linux-complete* and
*macOS-incomplete* by construction.

The bead states this drift as 3 fields. It is **8 sites**: `source`,
`memory_total_bytes`, `memory_available_bytes`, `reserved_core_policy.policy`,
`reserved_core_policy.reason`, `fallback_active`, `decision_reason`,
`proof_notes`.

`status_shape.json` carries the same host drift in schema form — 4 fields:
`memory_total_bytes` and `memory_available_bytes` `"type": "integer"` ->
`"type": "null"`, and `controller_loadavg_high_watermark_1m` /
`controller_loadavg_low_watermark_1m` `"type": "number"` -> `"type": "null"`.

### Quoted diff, case 8 — `diag.json`: this one refutes the bead

```diff
$ diff -u tests/golden/robot/diag.json.golden tests/golden/robot/diag.json.actual
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

That is the whole diff — byte-for-byte identical shape for
`diag_quarantine.json`. **No `connector_coverage` anywhere.** e3ed01f0 never
touched `cass diag`.

There is no scrubber for these fields:

```
$ rg -n '"os"|"arch"|x86_64|aarch64|target_arch' tests/golden_robot_json.rs
(no output, exit 0)
```

So `diag_json` and `diag_quarantine_json` are **pure host failures, red on any
non-Linux-x86_64 machine, and green on Linux CI**. They are not attributable to
e3ed01f0 and regenerating them is not part of fixing a4xe1.

### What this means for the bead's headline number

The bead says "9 of 37 tests fail on main." That is a **macOS** count. On CI's
`ubuntu-latest` leg the e3ed01f0 damage is **7 tests** (rows 1-7); rows 8-9 are
green there. Conversely on macOS and Windows, rows 5-9 fail for host reasons
regardless of e3ed01f0, so the pre-existing laptop red was 5, not 0.

---

## 6. `rch` — the prescribed Linux regeneration path is NOT AVAILABLE here

`AGENTS.md:984` says "RCH is installed at `~/.local/bin/rch`". It is not:

```
$ command -v rch ; echo "rc=$?"
rc=1
$ /bin/ls -la ~/.local/bin/rch
ls: /Users/dalecarman/.local/bin/rch: No such file or directory
$ /bin/ls ~/.config/rch
"/Users/dalecarman/.config/rch": No such file or directory (os error 2)
```

So fix option (a) as the bead words it — "regenerate on a Linux runner" — has no
runner on this machine today. Executing it needs rch installed, a Linux
container/VM, or a Linux box; CI only verifies goldens, it never regenerates
them.

---

## 7. The correct fix

**Scope discipline first.** Bead a4xe1 is about the `connector_coverage`
staleness. The macOS/Windows host drift is a *different, older* defect that
a4xe1 discovered in passing. Fixing them together is fine; conflating them is
what produced the bead's wrong attribution for `diag`.

### 7a. For a4xe1 — hand-apply the `connector_coverage` block to 7 goldens

Neither of the bead's two options is what I would do, and hand-editing is not a
hack here: the failing run has already written the exact bytes.

- **4 goldens can be replaced wholesale from their `.actual`**, because their
  diff contains nothing host-dependent — proven above, the full diff is the
  `connector_coverage` hunk(s) and nothing else:
  `health.json`, `health_shape.json`, `stats_full_payload.json`,
  `stats_full_payload_shape.json`.
  Trailing-byte parity checked, so a `cp` is byte-safe:
  ```
  $ tail -c 20 ...stats_full_payload.json.golden | od -c   ->  b " \n }   (no trailing newline)
  $ tail -c 20 ...stats_full_payload.json.actual | od -c   ->  b " \n }   (identical)
  ```
- **3 goldens must take only the `connector_coverage` hunk**, leaving the
  `topology_budget` block untouched: `status_shape.json`,
  `status_quarantine.json`, `status_quarantine_full.json`. Six added lines each
  (twelve for `status_shape`, which is the schema form). Copying `.actual`
  wholesale for these three is exactly the mistake the "do not regenerate on
  macOS" constraint exists to prevent.
- **2 goldens must not be touched at all**: `diag.json`, `diag_quarantine.json`.

That lands a4xe1 without a Linux host, without `rch`, and without baking macOS
topology into a Linux contract. After the edit all seven should pass on
`ubuntu-latest`; on this laptop 5 will still be red for host reasons (rows 5-9).

### 7b. For the cross-host red (recommend a separate bead)

`.github/workflows/ci.yml:359-366` runs the whole suite on
`[ubuntu-latest, macos-latest, windows-latest]` with `fail-fast: false`, no
`continue-on-error` on `test-rust`, and `tests/golden_robot_json.rs` has no
`cfg(target_os)` gate. So five of these tests are structurally red on two of the
three CI legs. **UNVERIFIED against an actual CI run** — I read the workflow and
the code, not a CI log.

Recommended: extend the existing normalizer rather than cfg-gating the tests.
`normalize_live_robot_values` already folds host-derived values to Linux
sentinels by key (`logical_cpus` -> 128 at `:366`) and even folds a host-derived
*narrative string* by prefix (`"planned from "` -> the ManyCoreSingleSocket
sentence at `:418`). Extending that same mechanism to `source`,
`fallback_active`, `policy`, `reason`, `decision_reason` (the
`"using conservative defaults: "` prefix), `proof_notes`, the `null` memory
case, and `platform.os` / `platform.arch` is the pattern the file already uses —
not new machinery. cfg-gating is the alternative and it is worse: it silently
drops the contract assertion on two of three CI legs.

### 7c. Also worth filing: the regeneration instructions are a trap

`assert_golden`'s own panic text (`golden_robot_json.rs:864`), `AGENTS.md:443`,
and `tests/golden/PROVENANCE.md` all tell the reader to run
`UPDATE_GOLDENS=1 ... cargo test --test golden_robot_json`. Nothing in any of
them says the goldens are host-sensitive. A macOS agent that follows the printed
instruction converts one stale-golden red into a permanent CI red on the ubuntu
leg. The constraint currently lives only in a handoff file
(`thoughts/shared/handoffs/20260814-cass-repair-to-green/generation-2-log.md:210-215`),
which no future agent will read. It belongs in `PROVENANCE.md` and in the panic
string.

---

## 8. Status

Lane complete. Read-only; the only file I wrote is this log. Nine gitignored
`.actual` dumps were produced by the test run itself under
`tests/golden/robot/`. No git mutation, no cass invocation, no write to the live
archive.
