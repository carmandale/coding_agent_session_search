//! Scratch-root infrastructure for spec 015 watch-once streaming scan.
//!
//! When `cass index --watch-once ~/.pi/agent/sessions` runs on the pi connector,
//! the legacy bulk path materialises every NormalizedConversation in memory
//! before any persist happens. This module supports the streaming alternative
//! described in spec 015: cass enumerates pi files via the connector's
//! `discover_source_files`, then for each batch builds a real (non-symlink)
//! scratch directory tree mirroring the canonical `sessions/<workspace>/<file>`
//! layout. The pi connector scans the scratch root; FAD computes the same
//! `external_id` it would for a full-root scan because the relative path is
//! preserved. After scan, cass remaps `source_path` back to the original.
//!
//! See `specs/015-watch-once-streaming-scan/plan.md` for the full design and
//! `specs/015-watch-once-streaming-scan/planning-transcript.md` for the 21
//! rounds of adversarial review that produced it.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::connectors::DiscoveredSourceFile;

/// A single file that could not be added to a scratch root during streaming
/// watch-once. Each entry is destined for `<data_dir>/quarantine/watch_ingest_poison.jsonl`
/// with `reason: "scratch_build_failure"`.
#[derive(Debug, Clone)]
pub struct ScratchBuildSkip {
    pub source_path: PathBuf,
    pub error_message: String,
}

/// RAII handle for a per-batch scratch directory. Drops via `remove_dir_all`
/// so a panic mid-scan still cleans up.
pub struct ScratchRootGuard {
    path: PathBuf,
}

impl ScratchRootGuard {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScratchRootGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// FAD-equivalent sessions-root resolution. Spec 015 plan section "Scratch-root
/// builder helper" requires this to match `franken_agent_detection::connectors::pi_agent`'s
/// `sessions_dir(home)` resolution (`pi_agent.rs:74-105`): if `scan_root/sessions`
/// exists, return that; else return `scan_root`. This lets users run either
/// `--watch-once ~/.pi/agent` or `--watch-once ~/.pi/agent/sessions` and have
/// the scratch tree align with FAD's expectations.
pub fn derive_pi_sessions_root(scan_root: &Path) -> PathBuf {
    for ancestor in scan_root.ancestors() {
        if ancestor.file_name().is_some_and(|name| name == "sessions") {
            return ancestor.to_path_buf();
        }
    }

    let candidate = scan_root.join("sessions");
    if candidate.is_dir() {
        candidate
    } else if scan_root.is_file() {
        scan_root.parent().unwrap_or(scan_root).to_path_buf()
    } else {
        scan_root.to_path_buf()
    }
}

/// Build a per-batch scratch root mirroring the canonical pi layout. Hardlinks
/// each source file into `<scratch>/sessions/<rel>`; falls back to `fs::copy`
/// on cross-device errors. Per-file failures (missing source, permission, etc.)
/// are recorded in the returned `Vec<ScratchBuildSkip>` and the function
/// continues with the remaining files; only systemic errors (workdir create,
/// scratch root permission, etc.) propagate via `Result::Err`.
pub fn build_scratch_root(
    batch: &[DiscoveredSourceFile],
    workdir: &Path,
    original_sessions_root: &Path,
) -> Result<(ScratchRootGuard, Vec<ScratchBuildSkip>)> {
    std::fs::create_dir_all(workdir)
        .with_context(|| format!("creating scratch workdir at {}", workdir.display()))?;

    let scratch_root = tempfile::Builder::new()
        .prefix("watch-once-streaming-")
        .tempdir_in(workdir)
        .with_context(|| format!("creating per-batch scratch dir under {}", workdir.display()))?
        // `into_path()` releases tempfile's cleanup so OUR `ScratchRootGuard` owns the lifetime.
        // ScratchRootGuard::Drop removes the directory.
        .keep();

    let sessions = scratch_root.join("sessions");
    std::fs::create_dir_all(&sessions)
        .with_context(|| format!("creating scratch sessions dir at {}", sessions.display()))?;

    let mut skips: Vec<ScratchBuildSkip> = Vec::new();
    for src in batch {
        let rel = match src.source_path.strip_prefix(original_sessions_root) {
            Ok(r) => r,
            Err(_) => {
                skips.push(ScratchBuildSkip {
                    source_path: src.source_path.clone(),
                    error_message: format!(
                        "source path not under sessions root {}",
                        original_sessions_root.display()
                    ),
                });
                continue;
            }
        };
        let dest = sessions.join(rel);
        if let Some(parent) = dest.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            skips.push(ScratchBuildSkip {
                source_path: src.source_path.clone(),
                error_message: format!("create_dir_all({}): {e}", parent.display()),
            });
            continue;
        }
        // EXDEV = 18 on macOS, Linux, BSDs (POSIX-standard cross-device error).
        // We hardcode the constant rather than depend on libc just for this one
        // check; the value has been stable for decades.
        const EXDEV: i32 = 18;
        match std::fs::hard_link(&src.source_path, &dest) {
            Ok(()) => {}
            Err(e) if e.raw_os_error() == Some(EXDEV) => {
                if let Err(copy_err) = std::fs::copy(&src.source_path, &dest) {
                    skips.push(ScratchBuildSkip {
                        source_path: src.source_path.clone(),
                        error_message: format!("copy fallback: {copy_err}"),
                    });
                }
            }
            Err(e) => {
                skips.push(ScratchBuildSkip {
                    source_path: src.source_path.clone(),
                    error_message: format!("hard_link: {e}"),
                });
            }
        }
    }

    Ok((ScratchRootGuard { path: scratch_root }, skips))
}

/// Limits for a single scan batch. Spec 015 plan section "Scan-batch sizing"
/// — files count and cumulative byte size, both env-tunable. Whichever fires
/// first triggers a flush.
#[derive(Debug, Clone, Copy)]
pub struct ScanBatchLimits {
    pub max_files: usize,
    pub max_bytes: u64,
}

impl ScanBatchLimits {
    /// Load limits from env. Falls back to spec defaults: 50 files / 64 MB.
    pub fn from_env() -> Self {
        let max_files = std::env::var("CASS_WATCH_SCAN_BATCH_FILES")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|n| *n > 0)
            .unwrap_or(50);
        let max_bytes = std::env::var("CASS_WATCH_SCAN_BATCH_BYTES")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|n| *n > 0)
            .unwrap_or(64 * 1024 * 1024);
        Self {
            max_files,
            max_bytes,
        }
    }
}

/// Group a slice of discovered files into batches no larger than `limits.max_files`
/// or `limits.max_bytes` (whichever fires first). Bytes drawn from each file's
/// `size_bytes` field (defaults to 0 if unknown, so files limit still gates).
pub fn chunk_by_files_and_bytes(
    discovered: &[DiscoveredSourceFile],
    limits: ScanBatchLimits,
) -> Vec<&[DiscoveredSourceFile]> {
    let mut batches: Vec<&[DiscoveredSourceFile]> = Vec::new();
    if discovered.is_empty() {
        return batches;
    }
    let mut start = 0usize;
    let mut accumulated_bytes: u64 = 0;
    for (i, f) in discovered.iter().enumerate() {
        let file_bytes = f.size_bytes.unwrap_or(0);
        // If this single file alone exceeds the byte limit and the current
        // batch is non-empty, flush the current batch first so the oversized
        // file lands in its own batch (degenerate but safe).
        if i > start
            && (i - start >= limits.max_files
                || accumulated_bytes.saturating_add(file_bytes) > limits.max_bytes)
        {
            batches.push(&discovered[start..i]);
            start = i;
            accumulated_bytes = 0;
        }
        accumulated_bytes = accumulated_bytes.saturating_add(file_bytes);
    }
    if start < discovered.len() {
        batches.push(&discovered[start..]);
    }
    batches
}

/// Remap an emitted `NormalizedConversation`'s `source_path` from the scratch
/// tree back to the canonical original tree. No-op if the path doesn't have
/// the scratch prefix (defensive: keeps the helper safe to call on
/// already-canonical paths during refactors).
pub fn remap_source_path_field(
    source_path: &mut PathBuf,
    scratch_sessions_root: &Path,
    original_sessions_root: &Path,
) {
    if let Ok(rel) = source_path.strip_prefix(scratch_sessions_root) {
        *source_path = original_sessions_root.join(rel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectors::{DiscoveredSourceFile, DiscoveredSourceRole};
    use crate::sources::provenance::Origin;
    use std::fs;

    fn discovered(source_path: PathBuf, size_bytes: Option<u64>) -> DiscoveredSourceFile {
        DiscoveredSourceFile {
            provider_slug: "pi_agent".to_string(),
            scan_root: source_path
                .parent()
                .unwrap_or_else(|| Path::new("/"))
                .to_path_buf(),
            source_path,
            role: DiscoveredSourceRole::PrimarySessionLog,
            origin: Origin::local(),
            platform: None,
            required_for_reconstruction: true,
            size_bytes,
            modified_at_ms: None,
        }
    }

    #[test]
    fn derive_pi_sessions_root_prefers_subdir_when_exists() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("sessions")).unwrap();
        let result = derive_pi_sessions_root(tmp.path());
        assert_eq!(result, tmp.path().join("sessions"));
    }

    #[test]
    fn derive_pi_sessions_root_returns_input_when_subdir_missing() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = derive_pi_sessions_root(tmp.path());
        assert_eq!(result, tmp.path().to_path_buf());
    }

    #[test]
    fn derive_pi_sessions_root_handles_both_canonical_user_inputs() {
        // Spec 015 supports both `--watch-once ~/.pi/agent` and
        // `--watch-once ~/.pi/agent/sessions`. Both must resolve to the same
        // sessions directory.
        let tmp = tempfile::TempDir::new().unwrap();
        let home = tmp.path().join("agent");
        let sessions = home.join("sessions");
        fs::create_dir_all(&sessions).unwrap();
        let a = derive_pi_sessions_root(&home);
        let b = derive_pi_sessions_root(&sessions);
        assert_eq!(a, sessions);
        assert_eq!(b, sessions);
    }

    #[test]
    fn derive_pi_sessions_root_uses_sessions_ancestor_for_explicit_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let sessions = tmp.path().join("agent").join("sessions");
        let workspace = sessions.join("--Users-dale-project--");
        fs::create_dir_all(&workspace).unwrap();
        let file = workspace.join("2026-01-19T20-14-17-509Z_session-id.jsonl");
        fs::write(&file, "{}\n").unwrap();

        let result = derive_pi_sessions_root(&file);

        assert_eq!(result, sessions);
    }

    #[test]
    fn derive_pi_sessions_root_uses_sessions_ancestor_for_workspace_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let sessions = tmp.path().join("agent").join("sessions");
        let workspace = sessions.join("--Users-dale-project--");
        fs::create_dir_all(&workspace).unwrap();

        let result = derive_pi_sessions_root(&workspace);

        assert_eq!(result, sessions);
    }

    #[test]
    fn chunk_by_files_and_bytes_empty_input() {
        let result = chunk_by_files_and_bytes(
            &[],
            ScanBatchLimits {
                max_files: 5,
                max_bytes: 1000,
            },
        );
        assert!(result.is_empty());
    }

    #[test]
    fn chunk_by_files_and_bytes_files_limit() {
        let files: Vec<_> = (0..10)
            .map(|i| discovered(PathBuf::from(format!("/x/{i}.jsonl")), Some(10)))
            .collect();
        let result = chunk_by_files_and_bytes(
            &files,
            ScanBatchLimits {
                max_files: 3,
                max_bytes: u64::MAX,
            },
        );
        assert_eq!(result.len(), 4);
        assert_eq!(result[0].len(), 3);
        assert_eq!(result[1].len(), 3);
        assert_eq!(result[2].len(), 3);
        assert_eq!(result[3].len(), 1);
    }

    #[test]
    fn chunk_by_files_and_bytes_bytes_limit() {
        let files: Vec<_> = (0..5)
            .map(|i| discovered(PathBuf::from(format!("/x/{i}.jsonl")), Some(40)))
            .collect();
        let result = chunk_by_files_and_bytes(
            &files,
            ScanBatchLimits {
                max_files: 1000,
                max_bytes: 100,
            },
        );
        // 40+40 = 80 fits; 40+40+40 = 120 exceeds. So [0,1] / [2,3] / [4]
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].len(), 2);
        assert_eq!(result[1].len(), 2);
        assert_eq!(result[2].len(), 1);
    }

    #[test]
    fn chunk_by_files_and_bytes_oversized_single_file() {
        let files = vec![
            discovered(PathBuf::from("/x/a.jsonl"), Some(10)),
            discovered(PathBuf::from("/x/b.jsonl"), Some(10)),
            discovered(PathBuf::from("/x/c.jsonl"), Some(10_000)), // exceeds limit by itself
            discovered(PathBuf::from("/x/d.jsonl"), Some(10)),
        ];
        let result = chunk_by_files_and_bytes(
            &files,
            ScanBatchLimits {
                max_files: 1000,
                max_bytes: 100,
            },
        );
        // a+b together (<100), c alone (>100, degenerate), d alone after
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].len(), 2);
        assert_eq!(result[1].len(), 1);
        assert_eq!(result[2].len(), 1);
    }

    #[test]
    fn build_scratch_root_hardlinks_files_into_canonical_layout() {
        let tmp = tempfile::TempDir::new().unwrap();
        let orig = tmp.path().join("agent").join("sessions");
        let ws = orig.join("--Users-test-foo--");
        fs::create_dir_all(&ws).unwrap();
        let f1 = ws.join("a.jsonl");
        let f2 = ws.join("b.jsonl");
        fs::write(&f1, b"line1\n").unwrap();
        fs::write(&f2, b"line2\n").unwrap();

        let batch = vec![
            discovered(f1.clone(), Some(6)),
            discovered(f2.clone(), Some(6)),
        ];
        let workdir = tmp.path().join("scratch");
        let (guard, skips) = build_scratch_root(&batch, &workdir, &orig).unwrap();
        assert!(skips.is_empty());

        let scratch_ws = guard.path().join("sessions").join("--Users-test-foo--");
        assert!(scratch_ws.join("a.jsonl").is_file());
        assert!(scratch_ws.join("b.jsonl").is_file());
        // Hardlink should share inode
        let st1 = fs::metadata(&f1).unwrap();
        let st2 = fs::metadata(scratch_ws.join("a.jsonl")).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            assert_eq!(st1.ino(), st2.ino());
        }
    }

    #[test]
    fn build_scratch_root_records_missing_file_as_skip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let orig = tmp.path().join("agent").join("sessions");
        let ws = orig.join("--Users-test-foo--");
        fs::create_dir_all(&ws).unwrap();
        let real_file = ws.join("a.jsonl");
        fs::write(&real_file, b"line\n").unwrap();
        let missing_file = ws.join("nonexistent.jsonl");

        let batch = vec![
            discovered(real_file.clone(), Some(5)),
            discovered(missing_file.clone(), Some(5)),
        ];
        let workdir = tmp.path().join("scratch");
        let (guard, skips) = build_scratch_root(&batch, &workdir, &orig).unwrap();
        assert_eq!(skips.len(), 1);
        assert_eq!(skips[0].source_path, missing_file);
        assert!(skips[0].error_message.contains("hard_link"));

        // The real file still got linked through
        let scratch_ws = guard.path().join("sessions").join("--Users-test-foo--");
        assert!(scratch_ws.join("a.jsonl").is_file());
    }

    #[test]
    fn build_scratch_root_records_path_outside_root_as_skip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let orig = tmp.path().join("agent").join("sessions");
        fs::create_dir_all(&orig).unwrap();
        let outside = tmp.path().join("elsewhere").join("a.jsonl");
        fs::create_dir_all(outside.parent().unwrap()).unwrap();
        fs::write(&outside, b"data").unwrap();
        let batch = vec![discovered(outside.clone(), Some(4))];
        let workdir = tmp.path().join("scratch");
        let (_guard, skips) = build_scratch_root(&batch, &workdir, &orig).unwrap();
        assert_eq!(skips.len(), 1);
        assert_eq!(skips[0].source_path, outside);
        assert!(
            skips[0].error_message.contains("not under sessions root"),
            "unexpected error_message: {}",
            skips[0].error_message
        );
    }

    #[test]
    fn scratch_root_guard_drops_directory() {
        let tmp = tempfile::TempDir::new().unwrap();
        let orig = tmp.path().join("sessions");
        fs::create_dir_all(orig.join("ws")).unwrap();
        let f = orig.join("ws").join("a.jsonl");
        fs::write(&f, b"x").unwrap();
        let batch = vec![discovered(f, Some(1))];
        let workdir = tmp.path().join("scratch");

        let (guard, _skips) = build_scratch_root(&batch, &workdir, &orig).unwrap();
        let scratch_path = guard.path().to_path_buf();
        assert!(scratch_path.is_dir());
        drop(guard);
        // After drop, the directory must be gone (NotFound).
        assert!(
            !scratch_path.exists(),
            "scratch path still exists after guard drop: {}",
            scratch_path.display()
        );
    }

    #[test]
    fn remap_source_path_field_translates_scratch_to_original() {
        let scratch_sessions = PathBuf::from("/tmp/scratch-abc/sessions");
        let original_sessions = PathBuf::from("/Users/me/.pi/agent/sessions");
        let mut path = PathBuf::from("/tmp/scratch-abc/sessions/--ws--/2026-05-15.jsonl");
        remap_source_path_field(&mut path, &scratch_sessions, &original_sessions);
        assert_eq!(
            path,
            PathBuf::from("/Users/me/.pi/agent/sessions/--ws--/2026-05-15.jsonl")
        );
    }

    #[test]
    fn remap_source_path_field_is_noop_when_prefix_absent() {
        let scratch_sessions = PathBuf::from("/tmp/scratch-abc/sessions");
        let original_sessions = PathBuf::from("/Users/me/.pi/agent/sessions");
        let mut path = PathBuf::from("/somewhere/else/file.jsonl");
        let before = path.clone();
        remap_source_path_field(&mut path, &scratch_sessions, &original_sessions);
        assert_eq!(path, before);
    }
}
