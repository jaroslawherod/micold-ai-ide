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

    /// The app mutex is machine-session-wide, so the two cases below take turns rather than see each
    /// other's marker.
    #[cfg(windows)]
    static APP_MUTEX_CASES: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Whether a mutex named [`APP_MUTEX_NAME`] exists right now, as the installer's `AppMutex` asks.
    #[cfg(windows)]
    fn app_mutex_exists() -> bool {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::{OpenMutexW, SYNCHRONIZATION_SYNCHRONIZE};

        let name: Vec<u16> = APP_MUTEX_NAME.encode_utf16().chain(Some(0)).collect();
        // SAFETY: `name` is NUL-terminated and outlives the call; a non-null handle is closed at once.
        unsafe {
            let handle = OpenMutexW(SYNCHRONIZATION_SYNCHRONIZE, 0, name.as_ptr());
            if handle.is_null() {
                return false;
            }
            CloseHandle(handle);
            true
        }
    }

    #[cfg(windows)]
    #[test]
    fn announce_running_holds_the_app_mutex() {
        // E7.1: the installer finds a running app by this mutex and asks the user to close it.
        let _turn = APP_MUTEX_CASES.lock().unwrap_or_else(|e| e.into_inner());
        let marker = announce_running();

        assert!(
            app_mutex_exists(),
            "while the marker is held, OpenMutexW({APP_MUTEX_NAME:?}) must succeed"
        );
        drop(marker);
    }

    #[cfg(windows)]
    #[test]
    fn dropping_the_marker_releases_the_app_mutex() {
        // E7.1: once the app exits, the installer must not keep asking the user to close it.
        let _turn = APP_MUTEX_CASES.lock().unwrap_or_else(|e| e.into_inner());
        let marker = announce_running();
        assert!(
            marker.is_some(),
            "announce_running must return a marker on Windows"
        );
        drop(marker);

        assert!(
            !app_mutex_exists(),
            "after the marker is dropped, OpenMutexW({APP_MUTEX_NAME:?}) must fail"
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
