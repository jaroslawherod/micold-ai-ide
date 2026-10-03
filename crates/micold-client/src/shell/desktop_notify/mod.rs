//! The operating system's notification facility (feature 039, research R4,
//! `contracts/desktop-notification.md`).
//!
//! One function, [`system`], returns the backend of the operating system the client was built for.
//! Nothing outside this directory names an operating system (Principle VI); what is shown, and
//! when, is decided in `features::attention`.

use micold_client::features::attention::{DesktopNotification, DesktopNotifier, NotifyError};

/// The notifier of the system this client was built for.
pub fn system() -> Box<dyn DesktopNotifier> {
    Box::new(Unsupported)
}

/// A system with no backend yet: every notification is refused with
/// [`NotifyError::Unsupported`], which the feature logs once (FR-010).
struct Unsupported;

impl DesktopNotifier for Unsupported {
    fn show(&self, _notification: DesktopNotification) -> Result<(), NotifyError> {
        Err(NotifyError::Unsupported)
    }
}
