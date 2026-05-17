---
title: "Frankensqlite fix proof for spec 016"
date: 2026-05-16T22:59:41Z
status: verified-local
---

# Frankensqlite Fix Proof

Sibling checkout:

```text
/Users/dalecarman/dev/spec014-frankensqlite-fix
branch: fix/fts5-vtab-snapshot-via-delta-journal
tracking: carmandale/fix/fts5-vtab-snapshot-via-delta-journal
```

Changed files:

```text
crates/fsqlite-pager/src/pager.rs | 55 +++++++++++++++++++++++++++++++--------
crates/fsqlite-wal/src/wal.rs     | 50 +++++++++++++++++++++++++++++++++++
2 files changed, 94 insertions(+), 11 deletions(-)
```

Working tree state:

```text
## fix/fts5-vtab-snapshot-via-delta-journal...carmandale/fix/fts5-vtab-snapshot-via-delta-journal
 M crates/fsqlite-pager/src/pager.rs
 M crates/fsqlite-wal/src/wal.rs
```

Commands run:

```text
$HOME/.cargo/bin/cargo fmt -p fsqlite-pager -p fsqlite-wal --check
env CARGO_TARGET_DIR=/tmp/frankensqlite-spec016-target $HOME/.cargo/bin/cargo test -p fsqlite-wal test_append_recovers_after_external_zero_byte_truncate
env CARGO_TARGET_DIR=/tmp/frankensqlite-spec016-target $HOME/.cargo/bin/cargo test -p fsqlite-pager freelist
```

Results:

```text
cargo fmt --check: pass
fsqlite-wal::wal::tests::test_append_recovers_after_external_zero_byte_truncate: ok
fsqlite-pager freelist tests: 23 passed, 0 failed
```

Interpretation:

- The WAL zero-byte truncate regression is covered by a focused passing test.
- The pager freelist regression is covered by `freelist_max_leaf_entries_respects_reserved_page_bytes`, which asserts default page size with `12` reserved bytes allows `1019` leaf entries instead of the invalid `1022` count that damaged the live cass DB.
- This proof is local only. It does not make the CASS dependency durable until the sibling fix is committed/pushed and CASS stops relying on a local `[patch]` path.
