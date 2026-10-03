//! The operating system's notification facility (feature 039, research R4,
//! `contracts/desktop-notification.md`).
//!
//! One function, [`system`], returns the backend of the operating system the client was built for.
//! Nothing outside this directory names an operating system (Principle VI); what is shown, and
//! when, is decided in `features::attention`.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

use micold_client::features::attention::DesktopNotifier;

/// The notifier of the system this client was built for: the session bus's notification service.
#[cfg(target_os = "linux")]
pub fn system() -> Box<dyn DesktopNotifier> {
    Box::new(linux::Notifier::new())
}

/// The notifier of the system this client was built for: the notification centre.
#[cfg(target_os = "macos")]
pub fn system() -> Box<dyn DesktopNotifier> {
    Box::new(macos::Notifier)
}

/// The notifier of the system this client was built for: a toast.
#[cfg(target_os = "windows")]
pub fn system() -> Box<dyn DesktopNotifier> {
    Box::new(windows::Notifier)
}
