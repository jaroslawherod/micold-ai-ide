//! T057 (BUG-002) — `remove_worktree_dir` reports what survived (FR-023c, FR-023d).
//!
//! `std::fs::remove_dir_all` reports the first errno and never the path that produced it, which
//! reached the user as a bare "Permission denied (os error 13)" for a tree of tens of thousands of
//! files. These drive the real filesystem — the failure being modelled is a permission the process
//! genuinely lacks, which no fake can produce.

use micold_core::worktree::remove_worktree_dir;

/// FR-023a / BUG-001 regression: the ordinary path, where git already removed the directory.
#[test]
fn an_already_absent_directory_leaves_nothing_behind() {
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("feat-gone");

    let leftovers = remove_worktree_dir(&target);

    assert!(
        leftovers.is_empty(),
        "an absent target is the success path, not a leftover: {leftovers:?}"
    );
}

/// The ordinary success path with real content: a directory the process owns is fully removed.
#[test]
fn a_removable_directory_leaves_nothing_behind() {
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("feat-ok");
    std::fs::create_dir_all(target.join("src")).unwrap();
    std::fs::write(target.join("src/main.rs"), b"fn main() {}").unwrap();

    let leftovers = remove_worktree_dir(&target);

    assert!(leftovers.is_empty(), "fully removed: {leftovers:?}");
    assert!(!target.exists(), "the directory is gone");
}

/// FR-023d: a directory that cannot be emptied names the specific paths that survived.
///
/// The unremovable case is built without privilege by clearing write permission on a parent
/// directory: unlinking an entry requires write on the *containing* directory, so this yields
/// `EACCES` exactly as a root-owned file does. The mode is restored in teardown so the temp
/// directory can still be cleaned up.
#[cfg(unix)]
#[test]
fn a_directory_that_cannot_be_emptied_names_what_survived() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("feat-blocked");
    let locked = target.join("build");
    std::fs::create_dir_all(&locked).unwrap();
    std::fs::write(locked.join("artifact.jar"), b"binary").unwrap();
    // Removable sibling, so the walk has to distinguish what actually blocked the removal.
    std::fs::create_dir_all(target.join("src")).unwrap();

    // Clear write on `build/` so its entries cannot be unlinked.
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o555)).unwrap();

    let leftovers = remove_worktree_dir(&target);

    // Restore before asserting, so a failed assertion still leaves a cleanable temp dir.
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();

    assert!(
        !leftovers.is_empty(),
        "a directory that survives removal must be reported (FR-023d)"
    );
    assert!(
        leftovers
            .iter()
            .any(|l| l.path.starts_with(&locked) || l.path == locked),
        "the report must name the path that blocked removal, got: {:?}",
        leftovers.iter().map(|l| &l.path).collect::<Vec<_>>()
    );
    assert!(
        target.exists(),
        "the precondition of this test is that the directory survived"
    );
}

/// The report is bounded: a blocked `build/` tree can hold tens of thousands of entries, and a
/// notice listing all of them informs nobody.
#[cfg(unix)]
#[test]
fn the_leftover_report_is_capped() {
    use micold_core::worktree::LEFTOVER_REPORT_CAP;
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("feat-many");
    let locked = target.join("build");
    std::fs::create_dir_all(&locked).unwrap();
    for i in 0..(LEFTOVER_REPORT_CAP * 3) {
        std::fs::write(locked.join(format!("f{i}")), b"x").unwrap();
    }
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o555)).unwrap();

    let leftovers = remove_worktree_dir(&target);

    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();

    assert!(
        leftovers.len() <= LEFTOVER_REPORT_CAP,
        "report capped at {LEFTOVER_REPORT_CAP}, got {}",
        leftovers.len()
    );
}
