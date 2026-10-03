//! The Windows desktop notification: a WinRT toast, through `tauri-winrt-notification`
//! (feature 039, research R4, contract "Backends").
//!
//! Two pure functions decide everything — [`toast_text`], the text handed to the system, and
//! [`notify_error`], what a failure of the crate means — and are tested without showing a toast.
//! [`Notifier::show`] is the call, with no branch of its own. The click arrives with story 3.

use micold_client::features::attention::{DesktopNotification, DesktopNotifier, NotifyError};

/// The Application User Model ID the toast is shown under. Windows shows a toast only for an ID
/// that a Start-menu shortcut carries: the installer puts this same string on its shortcut
/// (`packaging/windows/micold-ai-ide.iss`), and
/// `crates/micold-core/tests/notification_registers_nothing.rs` holds the two together.
pub(super) const APP_USER_MODEL_ID: &str = "MicoldAiIde.Client";

/// What the system is asked to show: the toast's title and the first line of text under it. The
/// crate sets both as the inner text of an XML element, so names are passed as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ToastText {
    pub title: String,
    pub line: String,
}

/// The toast's text for `notification`: its title and its body, and nothing else (contract N2).
pub(super) fn toast_text(notification: &DesktopNotification) -> ToastText {
    ToastText {
        title: notification.title.clone(),
        line: notification.body.clone(),
    }
}

/// What a failure of `tauri-winrt-notification` means for the user (FR-010): the system was asked
/// and did not take the toast.
pub(super) fn notify_error(error: &tauri_winrt_notification::Error) -> NotifyError {
    NotifyError::Refused(error.to_string())
}

/// The Windows notifier. It keeps nothing: each toast is built and handed to the system.
pub(super) struct Notifier;

impl DesktopNotifier for Notifier {
    /// Windows shows the toast only when a Start-menu shortcut carries [`APP_USER_MODEL_ID`], which
    /// is so for an installed build. Whether it reports an error otherwise is not documented
    /// (research R4), so an `Ok` here does not promise a toast on screen.
    fn show(&self, notification: DesktopNotification) -> Result<(), NotifyError> {
        let text = toast_text(&notification);
        tauri_winrt_notification::Toast::new(APP_USER_MODEL_ID)
            .title(&text.title)
            .text1(&text.line)
            .show()
            .map_err(|error| notify_error(&error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use micold_core::session::SessionId;
    use std::path::PathBuf;
    use tauri_winrt_notification::Error;

    fn notification(title: &str, body: &str) -> DesktopNotification {
        DesktopNotification {
            title: title.to_string(),
            body: body.to_string(),
            project: PathBuf::from("/repo"),
            session: SessionId::new(),
        }
    }

    #[test]
    fn the_toast_carries_the_title_and_the_body_as_its_first_line() {
        // U172 (FR-004, FR-029, N2).
        assert_eq!(
            toast_text(&notification(
                "Fix the parser is waiting for input",
                "repo \u{2014} Parser work"
            )),
            ToastText {
                title: "Fix the parser is waiting for input".to_string(),
                line: "repo \u{2014} Parser work".to_string(),
            }
        );
    }

    #[test]
    fn names_are_passed_as_written_because_the_crate_sets_them_as_inner_text() {
        // U172: `SetInnerText` escapes for the toast's XML, so nothing is escaped here (compare
        // `linux.rs`).
        let text = toast_text(&notification(
            "R&D <x> is waiting for input",
            "R&D \u{2014} <x>",
        ));
        assert_eq!(text.title, "R&D <x> is waiting for input");
        assert_eq!(text.line, "R&D \u{2014} <x>");
    }

    #[test]
    fn the_application_identity_is_the_one_the_installer_puts_on_its_shortcut() {
        // U46 (FR-029). The installer's half is `micold-core/tests/notification_registers_nothing.rs`.
        assert_eq!(APP_USER_MODEL_ID, "MicoldAiIde.Client");
    }

    #[test]
    fn a_failure_of_the_system_is_a_refusal_with_its_reason() {
        // U173 (FR-010): access denied, as the system answers a caller it does not let notify.
        let error = Error::Os(std::io::Error::from_raw_os_error(5).into());
        let reason = error.to_string();
        assert_eq!(notify_error(&error), NotifyError::Refused(reason));
    }

    #[test]
    fn a_failure_to_read_a_file_is_a_refusal_with_its_reason() {
        // U173: the crate's other error.
        let error = Error::Io(std::io::Error::other("no such image"));
        match notify_error(&error) {
            NotifyError::Refused(why) => assert!(why.contains("no such image"), "{why}"),
            other => panic!("expected a refusal, got {other:?}"),
        }
    }
}
