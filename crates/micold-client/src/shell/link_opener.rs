//! The operating system's opener (feature 031, contract link-opening §2).
//!
//! One arm per platform. Every argument is a single argv element and no shell is involved, except
//! explorer's `/select,"<path>"`, which Windows cannot receive any other way.
//!
//! # The launch window
//!
//! A launcher is waited for at most [`LAUNCH_WINDOW`]. Inside it, its exit says whether the address
//! was handed on; a launcher still running at its end counts as launched and is left to run, since
//! `xdg-open`'s generic mode runs the handler in the foreground (plan Risk 10). The request reaches
//! the operating system at spawn, before any wait, so the window costs SC-003 nothing.
//!
//! The classifiers and the Linux reveal are plain functions compiled on every OS, so the Linux gate
//! tests the macOS and Windows mappings too.

use std::ffi::OsStr;
use std::path::Path;
#[cfg(unix)]
use std::process::Child;
use std::time::Duration;

pub use micold_client::features::OpenFailure;

/// Opening an address, or showing a file in the file manager, on this machine.
pub trait LinkOpener: Send + Sync {
    /// Open a URL, or a host path, in whatever the system has set up for it.
    fn open(&self, target: &str) -> Result<(), OpenFailure>;
    /// Show `path` selected in the file manager, without running it (FR-013).
    // Called from M5 on (`shell/links.rs`, O4); the capability is whole from the start.
    #[allow(dead_code)]
    fn reveal(&self, path: &Path) -> Result<(), OpenFailure>;
}

/// How long a launcher is waited for before it counts as launched (contract §2).
#[cfg_attr(windows, allow(dead_code))]
pub const LAUNCH_WINDOW: Duration = Duration::from_secs(2);

/// How often a launcher is checked inside the window.
#[cfg(unix)]
const POLL: Duration = Duration::from_millis(10);

/// Wait for a started launcher for at most `window`, and classify how it ended.
///
/// `classify` gets the exit code and whatever the launcher wrote to stderr, when stderr was piped.
/// A launcher still running at the window's end is success, and a thread reaps it when it exits.
#[cfg(unix)]
fn launch_window(
    mut child: Child,
    window: Duration,
    classify: impl Fn(i32, &str) -> Result<(), OpenFailure>,
) -> Result<(), OpenFailure> {
    use std::io::Read;

    let deadline = std::time::Instant::now() + window;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stderr = String::new();
                if let Some(mut pipe) = child.stderr.take() {
                    let _ = pipe.read_to_string(&mut stderr);
                }
                return match status.code() {
                    Some(code) => classify(code, stderr.trim()),
                    None => Err(OpenFailure::LaunchFailed(format!(
                        "the opener was stopped ({status})"
                    ))),
                };
            }
            Ok(None) if std::time::Instant::now() >= deadline => {
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
                return Ok(());
            }
            Ok(None) => std::thread::sleep(POLL),
            Err(e) => return Err(OpenFailure::LaunchFailed(e.to_string())),
        }
    }
}

/// Linux: `xdg-open`'s exit code. 3 is its "no tool found to open it".
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn classify_xdg_open(code: i32, _stderr: &str) -> Result<(), OpenFailure> {
    match code {
        0 => Ok(()),
        3 => Err(OpenFailure::NoApplication),
        other => Err(OpenFailure::LaunchFailed(format!(
            "xdg-open exited with status {other}"
        ))),
    }
}

/// Any other launcher: zero is success, anything else a launch failure naming the program.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn classify_exit(program: &str, code: i32) -> Result<(), OpenFailure> {
    match code {
        0 => Ok(()),
        other => Err(OpenFailure::LaunchFailed(format!(
            "{program} exited with status {other}"
        ))),
    }
}

/// macOS: `open`'s exit code and what it wrote to stderr.
///
/// `open` exits 1 for every failure, so stderr is what tells "no application knows how to open
/// this" (Launch Services' `kLSApplicationNotFoundErr`, -10814) from the rest.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn classify_macos_open(exit_code: i32, stderr: &str) -> Result<(), OpenFailure> {
    if exit_code == 0 {
        return Ok(());
    }
    if stderr.to_ascii_lowercase().contains("no application") || stderr.contains("-10814") {
        return Err(OpenFailure::NoApplication);
    }
    Err(OpenFailure::LaunchFailed(if stderr.is_empty() {
        format!("open exited with status {exit_code}")
    } else {
        stderr.to_string()
    }))
}

/// `SE_ERR_ASSOCINCOMPLETE`: the association is incomplete or invalid.
const SE_ERR_ASSOCINCOMPLETE: isize = 27;
/// `SE_ERR_NOASSOC`: no application is associated with the file name extension.
const SE_ERR_NOASSOC: isize = 31;

/// Windows: what `ShellExecuteW` returned. Above 32 is success.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn classify_shell_execute(ret: isize) -> Result<(), OpenFailure> {
    match ret {
        ret if ret > 32 => Ok(()),
        SE_ERR_NOASSOC | SE_ERR_ASSOCINCOMPLETE => Err(OpenFailure::NoApplication),
        other => Err(OpenFailure::LaunchFailed(format!(
            "the system could not open it (error {other})"
        ))),
    }
}

/// Running one launcher to its classified end.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
trait CommandRunner {
    fn run(&self, program: &str, args: &[&OsStr]) -> Result<(), OpenFailure>;
}

/// Linux: select `path` in the file manager, or open its folder when that is not possible.
///
/// `ShowItems` is the freedesktop file-manager interface. `--print-reply` makes `dbus-send` wait for
/// the answer, so a session with no file manager providing it fails here and falls back, rather
/// than succeeding with nothing shown. The fallback opens the folder, never the file (FR-013).
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn reveal_linux(runner: &dyn CommandRunner, path: &Path) -> Result<(), OpenFailure> {
    let item = format!("array:string:{}", file_uri(path));
    let args = [
        OsStr::new("--session"),
        OsStr::new("--print-reply"),
        OsStr::new("--dest=org.freedesktop.FileManager1"),
        OsStr::new("--type=method_call"),
        OsStr::new("/org/freedesktop/FileManager1"),
        OsStr::new("org.freedesktop.FileManager1.ShowItems"),
        OsStr::new(&item),
        // The startup-notification id, which a reveal has none of.
        OsStr::new("string:"),
    ];
    match runner.run("dbus-send", &args) {
        Ok(()) => Ok(()),
        Err(selecting) => match path.parent() {
            Some(folder) => runner.run("xdg-open", &[folder.as_os_str()]),
            None => Err(selecting),
        },
    }
}

/// `file://` and `path`, percent-encoding every byte outside RFC 3986's unreserved set and `/`, so
/// a comma or a space cannot split the `array:string:` list `dbus-send` parses. On Unix the path's
/// own bytes are encoded, so a name that is not UTF-8 still names the file.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn file_uri(path: &Path) -> String {
    #[cfg(unix)]
    let bytes = std::os::unix::ffi::OsStrExt::as_bytes(path.as_os_str()).to_vec();
    #[cfg(not(unix))]
    let bytes = path.to_string_lossy().into_owned().into_bytes();
    let mut uri = String::from("file://");
    for byte in bytes {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                uri.push(byte as char)
            }
            other => uri.push_str(&format!("%{other:02X}")),
        }
    }
    uri
}

/// The real opener.
pub struct SystemLinkOpener;

#[cfg(unix)]
fn spawn(program: &str, args: &[&OsStr], pipe_stderr: bool) -> Result<Child, OpenFailure> {
    use std::process::{Command, Stdio};
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(if pipe_stderr {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .spawn()
        .map_err(|e| OpenFailure::LaunchFailed(format!("couldn't start {program}: {e}")))
}

#[cfg(target_os = "linux")]
impl CommandRunner for SystemLinkOpener {
    fn run(&self, program: &str, args: &[&OsStr]) -> Result<(), OpenFailure> {
        // Null stderr: a handler `xdg-open` runs in the foreground inherits it and outlives the
        // window, and a pipe nobody reads any more would break it on its first write.
        let child = spawn(program, args, false)?;
        if program == "xdg-open" {
            launch_window(child, LAUNCH_WINDOW, classify_xdg_open)
        } else {
            launch_window(child, LAUNCH_WINDOW, |code, _| classify_exit(program, code))
        }
    }
}

#[cfg(target_os = "linux")]
impl LinkOpener for SystemLinkOpener {
    fn open(&self, target: &str) -> Result<(), OpenFailure> {
        self.run("xdg-open", &[OsStr::new(target)])
    }

    fn reveal(&self, path: &Path) -> Result<(), OpenFailure> {
        reveal_linux(self, path)
    }
}

#[cfg(target_os = "macos")]
impl LinkOpener for SystemLinkOpener {
    fn open(&self, target: &str) -> Result<(), OpenFailure> {
        // `open` hands the address to Launch Services and exits; the application it starts does
        // not inherit this pipe, so reading stderr at exit cannot block on it.
        let child = spawn("open", &[OsStr::new(target)], true)?;
        launch_window(child, LAUNCH_WINDOW, classify_macos_open)
    }

    fn reveal(&self, path: &Path) -> Result<(), OpenFailure> {
        let child = spawn("open", &[OsStr::new("-R"), path.as_os_str()], true)?;
        launch_window(child, LAUNCH_WINDOW, classify_macos_open)
    }
}

#[cfg(windows)]
impl LinkOpener for SystemLinkOpener {
    fn open(&self, target: &str) -> Result<(), OpenFailure> {
        use windows_sys::Win32::System::Com::{
            CoInitializeEx, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
        };
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let wide = |text: &str| -> Vec<u16> { text.encode_utf16().chain(Some(0)).collect() };
        let verb = wide("open");
        let file = wide(target);
        // `ShellExecuteW` can hand the verb to a shell extension that needs COM, so its documentation
        // asks for COM on the calling thread. A blocking-pool thread may already have it (then this
        // is S_FALSE) or have it in another mode (RPC_E_CHANGED_MODE); either way the call proceeds,
        // and COM stays up for the thread's life rather than being torn down per open.
        // SAFETY: the reserved pointer must be null.
        let _ = unsafe {
            CoInitializeEx(
                std::ptr::null(),
                (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32,
            )
        };
        // SAFETY: both strings are NUL-terminated UTF-16 buffers that outlive the call, and the
        // null window, parameters and directory are documented as optional.
        let ret = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                verb.as_ptr(),
                file.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            )
        };
        classify_shell_execute(ret as isize)
    }

    fn reveal(&self, path: &Path) -> Result<(), OpenFailure> {
        use std::os::windows::process::CommandExt;
        use std::process::Command;
        // Rust's own quoting would wrap the whole `/select,…` argument, which explorer does not
        // parse; a Windows path cannot contain `"`, so quoting it by hand is safe.
        // `no_window` for the rule every spawn follows (feature 030, FR-024); explorer is a GUI
        // program, so the flag changes nothing for it.
        micold_core::process::no_window(&mut Command::new("explorer.exe"))
            .raw_arg(format!("/select,\"{}\"", path.display()))
            .spawn()
            .map(|_| ())
            .map_err(|e| OpenFailure::LaunchFailed(format!("couldn't start explorer.exe: {e}")))
    }
}

/// An opener that opens nothing, so no test can reach the system's.
#[cfg(test)]
pub(crate) struct NoopLinkOpener;

#[cfg(test)]
impl LinkOpener for NoopLinkOpener {
    fn open(&self, _target: &str) -> Result<(), OpenFailure> {
        Ok(())
    }

    fn reveal(&self, _path: &Path) -> Result<(), OpenFailure> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[cfg(unix)]
    mod launch {
        use super::super::*;
        use std::process::{Command, Stdio};
        use std::time::Instant;

        fn sh(script: &str) -> Child {
            Command::new("sh")
                .args(["-c", script])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
                .expect("sh starts")
        }

        /// U101
        #[test]
        fn a_launcher_exiting_zero_inside_the_window_is_success() {
            assert_eq!(
                launch_window(sh("exit 0"), LAUNCH_WINDOW, classify_xdg_open),
                Ok(()),
                "exit 0 means the opener handed the address on (FR-015)"
            );
        }

        /// U102
        #[test]
        fn xdg_open_exiting_three_inside_the_window_is_no_application() {
            assert_eq!(
                launch_window(sh("exit 3"), LAUNCH_WINDOW, classify_xdg_open),
                Err(OpenFailure::NoApplication),
                "xdg-open's exit 3 is its 'no tool found' (contract §2)"
            );
        }

        /// U103
        #[test]
        fn another_non_zero_exit_inside_the_window_is_a_launch_failure() {
            let result = launch_window(sh("exit 4"), LAUNCH_WINDOW, classify_xdg_open);
            assert!(
                matches!(&result, Err(OpenFailure::LaunchFailed(reason)) if reason.contains('4')),
                "any other failure is a launch failure naming the status, got {result:?}"
            );
        }

        /// U104
        #[test]
        fn a_launcher_still_running_at_the_end_of_the_window_is_success_and_keeps_running() {
            assert_eq!(
                LAUNCH_WINDOW,
                Duration::from_secs(2),
                "the contract's launch window"
            );
            let dir = tempfile::tempdir().expect("a temp dir");
            let marker = dir.path().join("still-ran");
            let script = format!("sleep 1; touch '{}'", marker.display());
            let started = Instant::now();
            let result = launch_window(sh(&script), Duration::from_millis(200), classify_xdg_open);
            assert!(
                started.elapsed() < Duration::from_millis(900),
                "the wait ends with the window, not with the handler"
            );
            assert_eq!(
                result,
                Ok(()),
                "a foreground handler still running is a launched one (plan Risk 10)"
            );
            let deadline = Instant::now() + Duration::from_secs(5);
            while !marker.exists() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(50));
            }
            assert!(
                marker.exists(),
                "the launcher is left to run, not killed at the window's end"
            );
        }
    }

    /// U155
    #[cfg(unix)]
    #[test]
    fn a_file_name_that_is_not_utf8_is_encoded_from_its_own_bytes() {
        use std::os::unix::ffi::OsStrExt;
        let path = Path::new(OsStr::from_bytes(b"/home/u/a b,\xff"));
        assert_eq!(
            file_uri(path),
            "file:///home/u/a%20b%2C%FF",
            "ShowItems names the file as it is on disk, not a lossy copy of its name"
        );
    }

    /// U139
    #[test]
    fn macos_open_failing_names_no_application_only_when_stderr_says_so() {
        assert_eq!(classify_macos_open(0, ""), Ok(()), "exit 0 is success");
        assert_eq!(
            classify_macos_open(
                1,
                "No application knows how to open URL foo://x (Error Domain=NSOSStatusErrorDomain Code=-10814)"
            ),
            Err(OpenFailure::NoApplication),
            "stderr naming no application is NoApplication (contract §2)"
        );
        assert!(
            matches!(
                classify_macos_open(1, "The file /tmp/x does not exist."),
                Err(OpenFailure::LaunchFailed(reason)) if reason.contains("does not exist")
            ),
            "any other failure is LaunchFailed with open's own words"
        );
    }

    /// U144
    #[test]
    fn shell_execute_returns_map_to_success_no_application_and_launch_failure() {
        assert_eq!(classify_shell_execute(33), Ok(()), "above 32 is success");
        assert_eq!(
            classify_shell_execute(31),
            Err(OpenFailure::NoApplication),
            "SE_ERR_NOASSOC"
        );
        assert_eq!(
            classify_shell_execute(27),
            Err(OpenFailure::NoApplication),
            "SE_ERR_ASSOCINCOMPLETE"
        );
        for ret in [32, 2] {
            assert!(
                matches!(
                    classify_shell_execute(ret),
                    Err(OpenFailure::LaunchFailed(_))
                ),
                "{ret} is at or below 32 and no association error, so a launch failure"
            );
        }
    }

    /// Records each command, and fails the ones whose program is listed.
    struct StubRunner {
        failing: &'static [&'static str],
        calls: RefCell<Vec<(String, Vec<String>)>>,
    }

    impl CommandRunner for StubRunner {
        fn run(&self, program: &str, args: &[&OsStr]) -> Result<(), OpenFailure> {
            self.calls.borrow_mut().push((
                program.to_string(),
                args.iter()
                    .map(|a| a.to_string_lossy().into_owned())
                    .collect(),
            ));
            if self.failing.contains(&program) {
                Err(OpenFailure::LaunchFailed(format!("{program} failed")))
            } else {
                Ok(())
            }
        }
    }

    /// U138
    #[test]
    fn linux_reveal_opens_the_folder_when_the_file_manager_cannot_select_the_item() {
        let runner = StubRunner {
            failing: &["dbus-send"],
            calls: RefCell::new(Vec::new()),
        };
        let result = reveal_linux(&runner, Path::new("/home/u/bin/tool"));
        let calls = runner.calls.into_inner();
        assert_eq!(result, Ok(()), "the fallback opened, so the reveal worked");
        assert_eq!(
            calls.len(),
            2,
            "one ShowItems call, then one fallback: {calls:?}"
        );
        assert_eq!(calls[0].0, "dbus-send");
        assert!(
            calls[0]
                .1
                .iter()
                .any(|a| a == "array:string:file:///home/u/bin/tool"),
            "ShowItems names the file itself: {:?}",
            calls[0].1
        );
        assert_eq!(
            calls[1],
            ("xdg-open".to_string(), vec!["/home/u/bin".to_string()]),
            "the fallback opens the containing folder, never the file (FR-013)"
        );

        let selected = StubRunner {
            failing: &[],
            calls: RefCell::new(Vec::new()),
        };
        assert_eq!(
            reveal_linux(&selected, Path::new("/home/u/bin/tool")),
            Ok(())
        );
        assert_eq!(
            selected.calls.into_inner().len(),
            1,
            "a file manager that selected the item needs no fallback"
        );
    }
}
