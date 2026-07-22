//! T016 — worktree discovery: porcelain parse + classify + reconcile (FR-018/018a).

use micold_core::worktree::{classify, parse_worktrees, reconcile, WorktreeRecord, WorktreeStatus};
use std::path::{Path, PathBuf};

const PORCELAIN: &str = "\
worktree /repo
HEAD abc123
branch refs/heads/main

worktree /repo/.claude/worktrees/feat-login
HEAD def456
branch refs/heads/feat/login

worktree /repo/.claude/worktrees/fix-gone
HEAD 000000
branch refs/heads/fix/gone
prunable gitdir file points to non-existent location
";

#[test]
fn parses_all_records_with_branch_shortnames() {
    let records = parse_worktrees(PORCELAIN);
    assert_eq!(records.len(), 3);
    assert_eq!(records[0].path, PathBuf::from("/repo"));
    assert_eq!(records[0].branch.as_deref(), Some("main"));
    assert_eq!(records[1].branch.as_deref(), Some("feat/login"));
    assert!(!records[1].prunable);
    assert!(records[2].prunable);
}

#[test]
fn classify_valid_missing() {
    let present = WorktreeRecord {
        path: PathBuf::from("/x"),
        branch: Some("feat/x".into()),
        prunable: false,
    };
    assert_eq!(classify(&present, true), WorktreeStatus::Valid);
    assert_eq!(classify(&present, false), WorktreeStatus::Missing);

    let prunable = WorktreeRecord {
        prunable: true,
        ..present
    };
    assert_eq!(classify(&prunable, true), WorktreeStatus::Missing);
}

#[test]
fn reconcile_classifies_registered_and_orphan_dirs() {
    let records = parse_worktrees(PORCELAIN);
    let root = Path::new("/repo/.claude/worktrees");
    // feat-login exists; fix-gone is prunable; feat-orphan is on disk but unregistered.
    let on_disk = vec!["feat-login".to_string(), "feat-orphan".to_string()];
    let exists = |p: &Path| p == Path::new("/repo/.claude/worktrees/feat-login");

    let worktrees = reconcile(&records, root, &on_disk, &exists);

    // The top-level /repo worktree is filtered out (not under .claude/worktrees).
    let by_name = |name: &str| {
        worktrees
            .iter()
            .find(|w| w.dir_name == name)
            .unwrap_or_else(|| panic!("missing {name}"))
            .status
    };
    assert_eq!(worktrees.len(), 3);
    assert_eq!(by_name("feat-login"), WorktreeStatus::Valid);
    assert_eq!(by_name("fix-gone"), WorktreeStatus::Missing);
    assert_eq!(by_name("feat-orphan"), WorktreeStatus::Invalid);
}
