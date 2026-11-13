//! Bug 1863: the real-runtime tests find the `micold-daemon` build whichever directory of the
//! target tree the test executable runs from.

mod daemon_bin;

use daemon_bin::built_daemon_above;

fn target_tree() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("debug/deps")).unwrap();
    std::fs::create_dir_all(root.path().join("debug/build/micold-client/abc123/out")).unwrap();
    std::fs::write(root.path().join("debug/micold-daemon"), b"").unwrap();
    root
}

#[test]
fn daemon_is_found_from_the_deps_directory() {
    let root = target_tree();
    let exe = root.path().join("debug/deps/test-1");
    assert_eq!(
        built_daemon_above(&exe),
        Some(root.path().join("debug/micold-daemon"))
    );
}

#[test]
fn daemon_is_found_from_the_build_script_out_directory() {
    let root = target_tree();
    let exe = root
        .path()
        .join("debug/build/micold-client/abc123/out/test-1");
    assert_eq!(
        built_daemon_above(&exe),
        Some(root.path().join("debug/micold-daemon"))
    );
}

#[test]
fn no_build_means_no_daemon() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(built_daemon_above(&root.path().join("deps/test-1")), None);
}
