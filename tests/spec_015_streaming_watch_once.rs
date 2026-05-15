//! Integration test for spec 015 watch-once streaming scan.
//!
//! Tests the load-bearing properties of the streaming branch in isolation
//! (without running the full cass indexer end-to-end, which would need
//! Tantivy + storage setup heavier than this spec needs to prove):
//!
//! 1. external_id is bit-for-bit identical between a full-root scan
//!    (`~/.pi/agent/sessions`) and a scratch-root scan
//!    (`<scratch>/sessions/...`) — the load-bearing invariant for Route 5.
//! 2. The scratch tree mirrors the canonical pi layout exactly.
//! 3. Chunking + scratch-root build round-trips cleanly across multiple
//!    workspaces.
//! 4. The forward-capture watcher (continuous --watch mode) and non-pi
//!    connectors are NOT affected by the streaming branch — implicit from
//!    `do_index_run`'s gate check.
//!
//! The "did this complete the 2,073-conv pi backfill" question is a Group E
//! task that runs against the user's real corpus; this file is the small-corpus
//! regression net that catches breakages between revisions.

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use coding_agent_search::connectors::{Connector, ScanContext, pi_agent::PiAgentConnector};
use coding_agent_search::indexer::scratch_root::{
    build_scratch_root, chunk_by_files_and_bytes, derive_pi_sessions_root,
    remap_source_path_field, ScanBatchLimits,
};
use serial_test::serial;

/// Build a tiny synthetic pi corpus: `n_workspaces` workspace directories,
/// each with `files_per_workspace` jsonl files.
fn build_pi_corpus(base: &Path, n_workspaces: usize, files_per_workspace: usize) -> Vec<PathBuf> {
    let sessions = base.join("sessions");
    fs::create_dir_all(&sessions).unwrap();
    let mut written: Vec<PathBuf> = Vec::new();
    for w in 0..n_workspaces {
        let ws = sessions.join(format!("--Users-test-ws{w}--"));
        fs::create_dir_all(&ws).unwrap();
        for f in 0..files_per_workspace {
            let file = ws.join(format!(
                "2026-05-15T10-{:02}-{:02}-000Z_session-w{w}-f{f}.jsonl",
                w, f
            ));
            // Each file is a minimal valid pi session + 2 messages.
            let session_id = format!("session-w{w}-f{f}");
            let content = format!(
                "{{\"type\":\"session\",\"id\":\"{sid}\",\"timestamp\":\"2026-05-15T10:{m:02}:{s:02}.000Z\",\"cwd\":\"/test/ws{w_val}\",\"provider\":\"anthropic\",\"modelId\":\"claude-sonnet-4-20250514\"}}\n{{\"type\":\"message\",\"timestamp\":\"2026-05-15T10:{m:02}:{ms:02}.000Z\",\"message\":{{\"role\":\"user\",\"content\":[{{\"type\":\"text\",\"text\":\"prompt w{w_val}-f{f_val}\"}}]}}}}\n{{\"type\":\"message\",\"timestamp\":\"2026-05-15T10:{m:02}:{me:02}.000Z\",\"message\":{{\"role\":\"assistant\",\"content\":[{{\"type\":\"text\",\"text\":\"answer w{w_val}-f{f_val}\"}}]}}}}\n",
                sid = session_id,
                w_val = w,
                f_val = f,
                m = w,
                s = f,
                ms = f + 1,
                me = f + 2,
            );
            fs::write(&file, content).unwrap();
            written.push(file);
        }
    }
    written
}

#[test]
#[serial]
fn spec_015_external_id_invariant_under_scratch_root() {
    // The load-bearing invariant: scanning <scratch>/sessions/<ws>/<file>
    // produces the same external_id as scanning ~/.pi/agent/sessions/<ws>/<file>
    // directly. If this breaks, dedupe/upsert against existing DB rows is
    // wrong and the no-data-loss clause fails.
    let dir = TempDir::new().unwrap();
    let original = dir.path().join("agent");
    fs::create_dir_all(&original).unwrap();
    let files = build_pi_corpus(&original, 2, 3);
    assert_eq!(files.len(), 6);

    // Scan the original root via PiAgentConnector to harvest canonical
    // external_ids.
    unsafe {
        std::env::set_var("PI_CODING_AGENT_DIR", &original);
    }
    let conn = PiAgentConnector::new();
    let ctx = ScanContext::with_roots(
        original.clone(),
        Vec::new(),
        None,
    );
    let original_convs = conn.scan(&ctx).expect("original scan");
    let original_eids: Vec<String> = original_convs
        .iter()
        .filter_map(|c| c.external_id.clone())
        .collect();
    assert_eq!(original_eids.len(), 6, "expected 6 conversations from original scan");

    // Build a scratch root mirroring the canonical layout.
    let workdir = dir.path().join("scratch");
    let original_sessions = derive_pi_sessions_root(&original);
    let discovered: Vec<_> = conn.discover_source_files(&ctx).expect("discover");
    assert_eq!(discovered.len(), 6);
    let limits = ScanBatchLimits {
        max_files: 10,
        max_bytes: u64::MAX,
    };
    let batches = chunk_by_files_and_bytes(&discovered, limits);
    assert_eq!(batches.len(), 1, "expected single batch for 6 files under generous limits");
    let (guard, skips) = build_scratch_root(batches[0], &workdir, &original_sessions).unwrap();
    assert!(skips.is_empty(), "expected no scratch skips on clean fixture");

    // Re-scan via the scratch root.
    unsafe {
        std::env::set_var("PI_CODING_AGENT_DIR", guard.path());
    }
    let scratch_ctx = ScanContext::with_roots(
        original.clone(),
        Vec::new(),
        None,
    );
    let scratch_convs = conn.scan(&scratch_ctx).expect("scratch scan");
    let scratch_eids: Vec<String> = scratch_convs
        .iter()
        .filter_map(|c| c.external_id.clone())
        .collect();
    assert_eq!(scratch_eids.len(), 6, "expected 6 conversations from scratch scan");

    // The external_id sets MUST match (sorted, since scan order may differ).
    let mut a = original_eids;
    let mut b = scratch_eids;
    a.sort();
    b.sort();
    assert_eq!(
        a, b,
        "external_id mismatch between original and scratch scan"
    );

    // Cleanup env var to avoid leaking into other tests
    unsafe {
        std::env::remove_var("PI_CODING_AGENT_DIR");
    }
}

#[test]
#[serial]
fn spec_015_scratch_layout_mirrors_canonical_pi_shape() {
    let dir = TempDir::new().unwrap();
    let original = dir.path().join("agent");
    fs::create_dir_all(&original).unwrap();
    let files = build_pi_corpus(&original, 3, 2);
    assert_eq!(files.len(), 6);

    let workdir = dir.path().join("scratch");
    let original_sessions = derive_pi_sessions_root(&original);
    unsafe {
        std::env::set_var("PI_CODING_AGENT_DIR", &original);
    }
    let conn = PiAgentConnector::new();
    let ctx = ScanContext::with_roots(original.clone(), Vec::new(), None);
    let discovered = conn.discover_source_files(&ctx).expect("discover");

    let limits = ScanBatchLimits { max_files: 100, max_bytes: u64::MAX };
    let batches = chunk_by_files_and_bytes(&discovered, limits);
    let (guard, skips) = build_scratch_root(batches[0], &workdir, &original_sessions).unwrap();
    assert!(skips.is_empty());

    // Every original file should appear under <scratch>/sessions/<ws>/<file>
    let scratch_sessions = guard.path().join("sessions");
    for orig in &files {
        let rel = orig.strip_prefix(&original_sessions).unwrap();
        let mirrored = scratch_sessions.join(rel);
        assert!(
            mirrored.is_file(),
            "expected scratch mirror file at {} (from original {})",
            mirrored.display(),
            orig.display()
        );
    }

    unsafe {
        std::env::remove_var("PI_CODING_AGENT_DIR");
    }
}

#[test]
#[serial]
fn spec_015_remap_source_path_restores_canonical_path_after_scratch_scan() {
    // Spec 015 Phase B Round 1 caught a bug where the remap helper would
    // produce ~/.pi/agent/sessions/sessions/<ws>/<file>. This test guards
    // against that regression: after remap, the source_path must be
    // bit-for-bit equal to the original disk path.
    let dir = TempDir::new().unwrap();
    let original = dir.path().join("agent");
    fs::create_dir_all(&original).unwrap();
    let files = build_pi_corpus(&original, 1, 2);

    let workdir = dir.path().join("scratch");
    let original_sessions = derive_pi_sessions_root(&original);
    unsafe {
        std::env::set_var("PI_CODING_AGENT_DIR", &original);
    }
    let conn = PiAgentConnector::new();
    let ctx = ScanContext::with_roots(original.clone(), Vec::new(), None);
    let discovered = conn.discover_source_files(&ctx).expect("discover");
    let limits = ScanBatchLimits { max_files: 10, max_bytes: u64::MAX };
    let batches = chunk_by_files_and_bytes(&discovered, limits);
    let (guard, _skips) = build_scratch_root(batches[0], &workdir, &original_sessions).unwrap();
    let scratch_sessions = guard.path().join("sessions");

    // Scan the scratch root.
    unsafe {
        std::env::set_var("PI_CODING_AGENT_DIR", guard.path());
    }
    let scratch_ctx = ScanContext::with_roots(original.clone(), Vec::new(), None);
    let mut scratch_convs = conn.scan(&scratch_ctx).expect("scratch scan");
    assert_eq!(scratch_convs.len(), 2);

    // Before remap, source_path should be under <scratch>/sessions
    for c in &scratch_convs {
        assert!(
            c.source_path.starts_with(&scratch_sessions),
            "pre-remap path should be under scratch_sessions, got {}",
            c.source_path.display()
        );
    }

    // Apply remap.
    for c in &mut scratch_convs {
        remap_source_path_field(&mut c.source_path, &scratch_sessions, &original_sessions);
    }

    // After remap, every source_path must equal one of the original disk files.
    let original_paths: std::collections::HashSet<_> = files.iter().cloned().collect();
    for c in &scratch_convs {
        assert!(
            original_paths.contains(&c.source_path),
            "remapped path {} not in original set",
            c.source_path.display()
        );
    }

    unsafe {
        std::env::remove_var("PI_CODING_AGENT_DIR");
    }
}

#[test]
#[serial]
fn spec_015_scratch_build_skip_record_carries_through_quarantine() {
    // Mix of real + non-existent source files. Builder should skip the
    // missing file (record ScratchBuildSkip) and continue with the rest.
    let dir = TempDir::new().unwrap();
    let original = dir.path().join("agent");
    fs::create_dir_all(&original).unwrap();
    let files = build_pi_corpus(&original, 1, 1);
    let real_file = &files[0];
    let original_sessions = derive_pi_sessions_root(&original);
    let workspace_dir = real_file.parent().unwrap();
    let missing_file = workspace_dir.join("nonexistent.jsonl");

    // Manually construct DiscoveredSourceFile for both real and missing.
    use coding_agent_search::connectors::DiscoveredSourceFile;
    use coding_agent_search::connectors::DiscoveredSourceRole;
    use coding_agent_search::sources::provenance::Origin;
    let dsf = |p: PathBuf| DiscoveredSourceFile {
        provider_slug: "pi_agent".to_string(),
        scan_root: original_sessions.clone(),
        source_path: p,
        role: DiscoveredSourceRole::PrimarySessionLog,
        origin: Origin::local(),
        platform: None,
        required_for_reconstruction: true,
        size_bytes: Some(100),
        modified_at_ms: None,
    };

    let batch = vec![dsf(real_file.clone()), dsf(missing_file.clone())];
    let workdir = dir.path().join("scratch");
    let (guard, skips) = build_scratch_root(&batch, &workdir, &original_sessions).unwrap();
    assert_eq!(skips.len(), 1);
    assert_eq!(skips[0].source_path, missing_file);
    assert!(skips[0].error_message.contains("hard_link"));

    // The real file still landed in scratch despite the missing-file failure.
    let rel = real_file.strip_prefix(&original_sessions).unwrap();
    let mirrored = guard.path().join("sessions").join(rel);
    assert!(mirrored.is_file());
}
