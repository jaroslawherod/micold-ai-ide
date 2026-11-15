//! `micold-daemon remote prepare-link` (#1725, contracts/dial.md): the step the holder of the SSH
//! connection runs before the bridge, so the link's `-R` binds in the remote's owner-only runtime
//! directory and a socket a killed connection left cannot make it fail.
//!
//! Driven through `remote::prepare_link_at` on a temporary directory: the command itself resolves
//! the pinned `/tmp/micold-<uid>`, and removing a live `orchestrator.sock` there would cut the link
//! of a daemon on the machine running the suite. The `ssh_real_link` suite runs the command.

// unix-only: `prepare-link` serves Unix remotes only (feature 489); the tests bind a Unix socket and read Unix modes.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;

use micold_daemon::remote::{prepare_link_at, prepare_link_command, prepare_link_report};

fn mode(path: &std::path::Path) -> u32 {
    std::fs::symlink_metadata(path)
        .unwrap()
        .permissions()
        .mode()
        & 0o777
}

#[test]
fn a_missing_runtime_directory_is_created_owner_only_and_the_socket_path_is_returned() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("micold-test");
    let path = prepare_link_at(&dir).expect("prepared");
    assert_eq!(path, dir.join("orchestrator.sock"));
    assert_eq!(mode(&dir), 0o700, "owner only");
    assert!(!path.exists(), "nothing is bound yet");
}

#[test]
fn an_existing_orchestrator_socket_is_removed_even_while_something_accepts_on_it() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("micold-test");
    std::fs::create_dir(&dir).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    let live = std::os::unix::net::UnixListener::bind(dir.join("orchestrator.sock")).unwrap();
    let daemon_socket = dir.join("daemon.sock");
    std::fs::write(&daemon_socket, b"").unwrap();

    let path = prepare_link_at(&dir).expect("prepared");
    assert!(
        std::fs::symlink_metadata(&path).is_err(),
        "the old socket is gone"
    );
    assert!(
        daemon_socket.exists(),
        "nothing else in the directory is touched"
    );
    drop(live);
}

#[test]
fn a_directory_with_a_loose_mode_is_refused_naming_it_and_nothing_is_removed() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("micold-test");
    std::fs::create_dir(&dir).unwrap();
    std::fs::write(dir.join("orchestrator.sock"), b"").unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();

    let err = prepare_link_at(&dir).expect_err("a loose mode is refused");
    assert!(err.to_string().contains("0o755"), "{err}");
    assert!(dir.join("orchestrator.sock").exists());
    assert_eq!(mode(&dir), 0o755, "never repaired");
}

#[test]
fn the_command_takes_no_argument() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_micold-daemon"))
        .args(["remote", "prepare-link", "--extra"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("usage"));
    assert!(out.stdout.is_empty(), "nothing on stdout but the path");
}

#[test]
fn the_report_prints_the_path_as_one_line_and_exits_zero() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("micold-test");
    let (mut out, mut err) = (Vec::new(), Vec::new());
    assert_eq!(prepare_link_report(&dir, &mut out, &mut err), 0);
    let want = format!("{}\n", dir.join("orchestrator.sock").display());
    assert_eq!(String::from_utf8(out).unwrap(), want);
    assert!(err.is_empty());
}

#[test]
fn the_report_names_a_refusal_on_stderr_prints_nothing_and_exits_one() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("micold-test");
    std::fs::create_dir(&dir).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    assert_eq!(prepare_link_report(&dir, &mut out, &mut err), 1);
    assert!(out.is_empty());
    assert!(String::from_utf8(err)
        .unwrap()
        .starts_with("micold-daemon remote prepare-link: "));
}

#[test]
fn a_socket_path_that_cannot_be_removed_is_an_error_not_a_silent_success() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("micold-test");
    std::fs::create_dir(&dir).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::create_dir(dir.join("orchestrator.sock")).unwrap();
    assert!(prepare_link_at(&dir).is_err());
}

#[test]
fn the_command_prepares_the_parent_of_the_resolved_socket_path() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("micold-test");
    let socket = dir.join("daemon.sock");
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = prepare_link_command(|| Ok::<_, String>(socket), &mut out, &mut err);
    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(out).unwrap(),
        format!("{}\n", dir.join("orchestrator.sock").display())
    );
}

#[test]
fn a_resolver_that_fails_is_reported_and_exits_one() {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = prepare_link_command(
        || Err::<std::path::PathBuf, _>("no home".to_string()),
        &mut out,
        &mut err,
    );
    assert_eq!(code, 1);
    assert!(out.is_empty());
    assert_eq!(
        String::from_utf8(err).unwrap(),
        "micold-daemon remote prepare-link: no home\n"
    );
}
