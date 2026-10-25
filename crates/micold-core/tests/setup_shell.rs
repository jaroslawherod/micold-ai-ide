//! Feature 1289, A4 (shell): which shell runs a setup command (FR-011, data-model `shell_line`).

use std::ffi::OsString;

use micold_core::setup::shell::{bash_on_path, shell_line, ShellPlatform};
use micold_core::setup::SetupCommand;

fn os(args: &[&str]) -> Vec<OsString> {
    args.iter().map(OsString::from).collect()
}

fn command() -> SetupCommand {
    SetupCommand::parse("mise trust && mise install").unwrap()
}

#[test]
fn unix_with_bash_runs_bash_without_startup_files() {
    assert_eq!(
        shell_line(ShellPlatform::Unix, true, &command()),
        (
            OsString::from("bash"),
            os(&["--noprofile", "--norc", "-c", "mise trust && mise install"])
        )
    );
}

#[test]
fn unix_without_bash_falls_back_to_sh() {
    assert_eq!(
        shell_line(ShellPlatform::Unix, false, &command()),
        (
            OsString::from("sh"),
            os(&["-c", "mise trust && mise install"])
        )
    );
}

#[test]
fn windows_runs_powershell_whatever_bash_is() {
    let expected = (
        OsString::from("powershell.exe"),
        os(&[
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "mise trust && mise install",
        ]),
    );
    assert_eq!(
        shell_line(ShellPlatform::Windows, true, &command()),
        expected
    );
    assert_eq!(
        shell_line(ShellPlatform::Windows, false, &command()),
        expected
    );
}

#[cfg(unix)]
#[test]
fn bash_is_found_on_a_path_that_holds_it_and_not_on_an_empty_one() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let stub = dir.path().join("bash");
    std::fs::write(&stub, "#!/bin/sh\n").unwrap();
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();

    assert!(bash_on_path(dir.path().as_os_str()));
    assert!(!bash_on_path(std::ffi::OsStr::new("")));
    let empty = tempfile::tempdir().unwrap();
    assert!(!bash_on_path(empty.path().as_os_str()));
}
