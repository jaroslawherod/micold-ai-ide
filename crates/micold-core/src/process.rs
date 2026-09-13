//! Background process hygiene on Windows (feature 030, FR-005, FR-009, FR-024).
//!
//! - [`no_window`]: a GUI-subsystem app that spawns a console program (`git`, `powershell`, `docker`)
//!   gets a console window flashed up for every call unless the spawn says otherwise.
//! - [`announce_running`]: the installer asks the user to close the app before it replaces the
//!   exes. It finds a running app through a named mutex, so both binaries hold one for their whole
//!   lifetime.
//!
//! Both are no-ops off Windows, so call sites stay free of `cfg`.

use std::process::Command;

/// `CREATE_NO_WINDOW` process creation flag: the child gets no console window.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Keep a spawned console program from opening a console window. Returns the command for chaining.
#[cfg(windows)]
pub fn no_window(cmd: &mut Command) -> &mut Command {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(CREATE_NO_WINDOW)
}

/// Keep a spawned console program from opening a console window. Returns the command for chaining.
#[cfg(not(windows))]
pub fn no_window(cmd: &mut Command) -> &mut Command {
    cmd
}

/// The named mutex a running app holds. `packaging/windows/micold-ai-ide.iss` names the same one in
/// `AppMutex`; `Local\` scopes it to the signed-in session, so another account's app does not block
/// this account's install.
pub const APP_MUTEX_NAME: &str = "Local\\MicoldAIIDE";

/// Proof that this process announced itself to the installer. The announcement ends on drop.
pub struct RunningMarker;

/// Announce that an app process is running, for the installer's `AppMutex` check.
pub fn announce_running() -> Option<RunningMarker> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn no_window_sets_flag() {
        // The documented value; a typo here would compile and silently open windows again.
        assert_eq!(CREATE_NO_WINDOW, 0x0800_0000);

        let status = no_window(Command::new("cmd").args(["/c", "exit 0"]))
            .status()
            .expect("spawn `cmd /c exit 0`");

        assert!(
            status.success(),
            "`cmd /c exit 0` must still run and exit 0, got {status:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn no_window_is_noop_elsewhere() {
        let status = no_window(&mut Command::new("true"))
            .status()
            .expect("spawn `true`");

        assert!(
            status.success(),
            "`true` must still run and exit 0, got {status:?}"
        );
    }
}
