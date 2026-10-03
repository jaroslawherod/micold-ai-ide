//! The operating system's notification facility (feature 039, research R4,
//! `contracts/desktop-notification.md`).
//!
//! One function, [`system`], returns the backend of the operating system the client was built for.
//! Nothing outside this directory names an operating system (Principle VI); what is shown, and
//! when, is decided in `features::attention`.

#[cfg(target_os = "linux")]
mod linux;

use micold_client::features::attention::DesktopNotifier;
#[cfg(not(target_os = "linux"))]
use micold_client::features::attention::{DesktopNotification, NotifyError};

/// The notifier of the system this client was built for.
#[cfg(target_os = "linux")]
pub fn system() -> Box<dyn DesktopNotifier> {
    Box::new(linux::Notifier::new())
}

/// The notifier of the system this client was built for. macOS and Windows have none yet (T041).
#[cfg(not(target_os = "linux"))]
pub fn system() -> Box<dyn DesktopNotifier> {
    Box::new(Unsupported)
}

/// A system with no backend yet: every notification is refused with
/// [`NotifyError::Unsupported`], which the feature logs once (FR-010).
#[cfg(not(target_os = "linux"))]
struct Unsupported;

#[cfg(not(target_os = "linux"))]
impl DesktopNotifier for Unsupported {
    fn show(&self, _notification: DesktopNotification) -> Result<(), NotifyError> {
        Err(NotifyError::Unsupported)
    }
}
