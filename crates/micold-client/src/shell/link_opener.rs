//! The operating system's opener (feature 031, contract link-opening §2).

use std::ffi::OsStr;
use std::path::Path;
use std::process::{Child, Command};
use std::time::Duration;

pub use micold_client::features::OpenFailure;

/// Opening an address, or showing a file in the file manager, on this machine.
pub trait LinkOpener: Send + Sync {
    /// Open a URL, or a host path, in whatever the system has set up for it.
    fn open(&self, target: &str) -> Result<(), OpenFailure>;
    /// Show `path` selected in the file manager, without running it (FR-013).
    fn reveal(&self, path: &Path) -> Result<(), OpenFailure>;
}

/// How long a launcher is waited for before it counts as launched (contract §2).
pub const LAUNCH_WINDOW: Duration = Duration::from_secs(2);

/// Wait for a started launcher for at most `window`, and classify how it ended.
fn launch_window(
    _child: Child,
    _window: Duration,
    _classify: impl Fn(i32, &str) -> Result<(), OpenFailure>,
) -> Result<(), OpenFailure> {
    Err(OpenFailure::LaunchFailed("unimplemented".to_string()))
}

/// Linux: `xdg-open`'s exit code.
fn classify_xdg_open(_code: i32, _stderr: &str) -> Result<(), OpenFailure> {
    Err(OpenFailure::LaunchFailed("unimplemented".to_string()))
}

/// macOS: `open`'s exit code and what it wrote to stderr.
pub fn classify_macos_open(_exit_code: i32, _stderr: &str) -> Result<(), OpenFailure> {
    Err(OpenFailure::LaunchFailed("unimplemented".to_string()))
}

/// Windows: what `ShellExecuteW` returned.
pub fn classify_shell_execute(_ret: isize) -> Result<(), OpenFailure> {
    Err(OpenFailure::LaunchFailed("unimplemented".to_string()))
}

/// Running one launcher to its classified end.
trait CommandRunner {
    fn run(&self, program: &str, args: &[&OsStr]) -> Result<(), OpenFailure>;
}

/// Linux: select `path` in the file manager, or open its folder when that is not possible.
fn reveal_linux(_runner: &dyn CommandRunner, _path: &Path) -> Result<(), OpenFailure> {
    Ok(())
}

/// The real opener.
pub struct SystemLinkOpener;

impl LinkOpener for SystemLinkOpener {
    fn open(&self, _target: &str) -> Result<(), OpenFailure> {
        Ok(())
    }

    fn reveal(&self, _path: &Path) -> Result<(), OpenFailure> {
        Ok(())
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
        use std::process::Stdio;
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
                matches!(classify_shell_execute(ret), Err(OpenFailure::LaunchFailed(_))),
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
                args.iter().map(|a| a.to_string_lossy().into_owned()).collect(),
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
        assert_eq!(calls.len(), 2, "one ShowItems call, then one fallback: {calls:?}");
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
        assert_eq!(reveal_linux(&selected, Path::new("/home/u/bin/tool")), Ok(()));
        assert_eq!(
            selected.calls.into_inner().len(),
            1,
            "a file manager that selected the item needs no fallback"
        );
    }
}
