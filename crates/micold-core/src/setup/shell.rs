//! Which shell runs a setup command (feature 1289, research R6, FR-011).
//!
//! The choice is pure: the daemon passes its platform (`cfg!(windows)` at the call site, so core
//! does not branch on the OS) and whether `bash` is on the run's `PATH`. The flags match
//! environment-include's shells, without `-i`: setup sources no startup file.

use std::ffi::{OsStr, OsString};

use super::SetupCommand;

/// The platform whose shell runs the command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellPlatform {
    Unix,
    Windows,
}

/// The program and arguments that run `command`: `bash --noprofile --norc -c` when `bash` is on the
/// run's `PATH`, else `sh -c`; `powershell.exe -NoProfile -NonInteractive -Command` on Windows.
#[must_use]
pub fn shell_line(
    platform: ShellPlatform,
    bash_on_path: bool,
    command: &SetupCommand,
) -> (OsString, Vec<OsString>) {
    let (program, flags): (&str, &[&str]) = match (platform, bash_on_path) {
        (ShellPlatform::Windows, _) => (
            "powershell.exe",
            &["-NoProfile", "-NonInteractive", "-Command"],
        ),
        (ShellPlatform::Unix, true) => ("bash", &["--noprofile", "--norc", "-c"]),
        (ShellPlatform::Unix, false) => ("sh", &["-c"]),
    };
    let args = flags
        .iter()
        .map(OsString::from)
        .chain(std::iter::once(OsString::from(command.as_str())))
        .collect();
    (OsString::from(program), args)
}

/// Whether `bash` resolves on `path`, by the same rules a session's command lookup uses.
#[must_use]
pub fn bash_on_path(path: &OsStr) -> bool {
    crate::provider::resolve_on_path("bash", path).is_some()
}
