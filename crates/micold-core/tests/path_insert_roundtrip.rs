//! SC-003: quoted text, run by a real shell, names exactly the file (feature 487).
//!
//! Each shell is skipped when it is not installed; cmd is covered by the quoting tables alone, and
//! PowerShell runs where it is installed (the Windows CI job).

use std::ffi::OsStr;
use std::process::Command;

use micold_core::path_insert::quote::{quote, AWKWARD_NAMES};
use micold_core::path_insert::ShellKind;

fn installed(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Run `script` in `program` with `args` before it and return stdout.
fn run(program: &str, args: &[&str], script: &str) -> String {
    let out = Command::new(program)
        .args(args)
        .arg(script)
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{program}: {out:?}");
    String::from_utf8(out.stdout).expect("utf8")
}

/// The quoted name, handed to `printf %s` by the shell, must come back as the name.
fn check_posix_like(program: &str, kind: ShellKind) {
    for name in AWKWARD_NAMES {
        let q = quote(kind, OsStr::new(name)).unwrap();
        let got = run(program, &["-c"], &format!("printf %s {q}"));
        assert_eq!(&got, name, "{program}: {q}");
    }
}

#[cfg(unix)]
#[test]
fn sh_reads_each_quoted_name_back_exactly() {
    check_posix_like("sh", ShellKind::Posix);
}

#[cfg(unix)]
#[test]
fn bash_reads_each_quoted_name_back_exactly() {
    if installed("bash") {
        check_posix_like("bash", ShellKind::Bash);
    }
}

#[cfg(unix)]
#[test]
fn bash_reads_a_non_utf8_name_back_as_the_same_bytes() {
    use std::os::unix::ffi::OsStrExt;
    if !installed("bash") {
        return;
    }
    let name = OsStr::from_bytes(b"a\xffb'c\\d");
    let q = quote(ShellKind::Bash, name).unwrap();
    let out = Command::new("bash")
        .args(["-c", &format!("printf %s {q}")])
        .output()
        .unwrap();
    assert_eq!(out.stdout, b"a\xffb'c\\d");
}

#[cfg(unix)]
#[test]
fn zsh_reads_each_quoted_name_back_exactly() {
    if installed("zsh") {
        check_posix_like("zsh", ShellKind::Bash);
    }
}

#[cfg(unix)]
#[test]
fn fish_reads_each_quoted_name_back_exactly() {
    if installed("fish") {
        for name in AWKWARD_NAMES {
            let q = quote(ShellKind::Fish, OsStr::new(name)).unwrap();
            let got = run("fish", &["-c"], &format!("printf %s {q}"));
            assert_eq!(&got, name, "fish: {q}");
        }
    }
}

#[cfg(windows)]
#[test]
fn powershell_reads_each_quoted_name_back_exactly() {
    for program in ["pwsh", "powershell"] {
        let probe = Command::new(program)
            .args(["-NoProfile", "-Command", "1"])
            .output();
        if probe.is_err() {
            continue;
        }
        for name in AWKWARD_NAMES {
            let q = quote(ShellKind::PowerShell, OsStr::new(name)).unwrap();
            // Each name is written to stdout as UTF-8 so the comparison is exact.
            let script = format!(
                "[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); \
                 [Console]::Out.Write({q})"
            );
            let got = run(program, &["-NoProfile", "-Command"], &script);
            assert_eq!(&got, name, "{program}: {q}");
        }
        return;
    }
}
