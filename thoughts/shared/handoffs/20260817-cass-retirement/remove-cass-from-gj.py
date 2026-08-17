#!/usr/bin/env python3
"""Remove the cass-dependent `gj sessions` / `gj last-sessions` subcommands.

Every edit asserts on the exact current content of its anchor lines before
touching them, and the script aborts without writing if any assertion fails.
Deletions are applied in descending line order so earlier ranges stay valid.
"""
import sys, pathlib

GJ = pathlib.Path.home() / "dev/gj-tool/bin/gj"
lines = GJ.read_text().splitlines(keepends=True)
orig_count = len(lines)


def L(n):  # 1-indexed access
    return lines[n - 1]


def expect(n, needle):
    if needle not in L(n):
        sys.exit(f"ABORT: line {n} does not contain {needle!r}\n  actual: {L(n)!r}")


# ---- verify every anchor BEFORE mutating anything -------------------------
expect(38, "gj last-sessions [-n N] [--agent <name>]")
expect(10592, "cmd_sessions() {")
expect(10619, "}")
expect(10621, "cmd_last_sessions() {")
expect(10864, "}")
expect(10866, "_resolve_usage() {")
expect(13508, "_cmd_meta_sessions() {")
expect(13515, "}")
expect(13517, "_cmd_meta_last_sessions() {")
expect(13529, "}")
expect(13531, "_cmd_meta_devices() {")
expect(13728, "cmd_sessions_usage()")
expect(13729, "cmd_last_sessions_usage()")
expect(13757, "sessions) cmd_sessions_usage ;;")
expect(13758, "last-sessions|last) cmd_last_sessions_usage ;;")
expect(13781, "last|last-sessions) _cmd_meta_last_sessions json ;;")
expect(13798, "sessions) _cmd_meta_sessions json ;;")
expect(13824, "tui agents sessions last-sessions devices sim ui scene test")
expect(13867, "gj sessions [path]")
expect(13868, "gj last-sessions [-n N] [--all] [--agent <name>]")
expect(13872, "--agent: filter by agent")
expect(14139, 'sessions) cmd_sessions "$@" ;;')
expect(14140, 'last-sessions|last) cmd_last_sessions "$@" ;;')
print(f"all 22 anchors verified against {orig_count} lines")

# ---- one in-place replacement (completion list) ---------------------------
before = L(13824)
lines[13824 - 1] = before.replace(
    "tui agents sessions last-sessions devices", "tui agents devices"
)
assert "sessions" not in lines[13824 - 1], lines[13824 - 1]

# ---- deletions, descending ------------------------------------------------
# (start, end) inclusive, 1-indexed
ranges = [
    (14139, 14140),  # main dispatch
    (13867, 13872),  # help text block for both subcommands
    (13798, 13798),  # help-json: sessions
    (13781, 13781),  # help-json: last-sessions
    (13757, 13758),  # usage dispatch
    (13728, 13729),  # usage alias definitions
    (13508, 13530),  # both _cmd_meta_* blocks + trailing blank
    (10592, 10865),  # cmd_sessions + cmd_last_sessions + trailing blank
    (38, 38),        # header comment
]
removed = 0
for start, end in ranges:
    del lines[start - 1:end]
    removed += end - start + 1

GJ.write_text("".join(lines))
print(f"removed {removed} lines; {orig_count} -> {len(lines)}")

# ---- post-conditions -----------------------------------------------------
text = "".join(lines)
for forbidden in [
    "cmd_sessions", "cmd_last_sessions", "_cmd_meta_sessions",
    "_cmd_meta_last_sessions", "CASS_TUI_DIR", "cass_tui",
    "last-sessions", "CASS TUI",
]:
    if forbidden in text:
        hits = [i + 1 for i, l in enumerate(lines) if forbidden in l]
        sys.exit(f"POST-CHECK FAILED: {forbidden!r} still present at lines {hits}")
print("post-check clean: no cass/sessions-subcommand tokens remain in bin/gj")
