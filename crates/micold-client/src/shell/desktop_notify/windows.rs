//! The Windows desktop notification: a WinRT toast, through `tauri-winrt-notification`
//! (feature 039, research R4, contract "Backends").
//!
//! Three functions decide everything — [`toast_text`], the text handed to the system,
//! [`notify_error`], what a failure of the crate means, and [`on_activated`], what a toast does
//! when it is clicked — and are tested without showing a toast. [`Notifier::show`] is the call,
//! with no branch of its own.

use std::path::PathBuf;

use micold_client::features::attention::{
    DesktopNotification, DesktopNotifier, NotifierEvent, NotifyError,
};
use micold_client::notification_icon::IconFiles;
use micold_core::attention::NotificationKind;
use micold_core::session::SessionId;
use tauri_winrt_notification::IconCrop;

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

/// The kind's icon on the toast (feature 613, I6): its file in the app logo's place, named for the
/// kind for a screen reader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ToastIcon {
    pub path: PathBuf,
    pub alt: &'static str,
}

/// How the toast crops the icon: the tile is a rounded square already.
pub(super) const ICON_CROP: IconCrop = IconCrop::Square;

/// The icon of a toast of `kind`: its file, when one was written; else none, and the toast is
/// shown without one (FR-016).
pub(super) fn toast_icon(kind: NotificationKind, icons: &IconFiles) -> Option<ToastIcon> {
    icons.path(kind).map(|path| ToastIcon {
        path: path.to_path_buf(),
        alt: kind.name(),
    })
}

/// What a failure of `tauri-winrt-notification` means for the user (FR-010): the system was asked
/// and did not take the toast.
pub(super) fn notify_error(error: &tauri_winrt_notification::Error) -> NotifyError {
    NotifyError::Refused(error.to_string())
}

/// What one toast does when the system says it was activated. Each toast carries its own
/// handler, so the handler is the table: it names the one session its toast was raised for
/// (contract N9). The system calls it on a thread of its own, so it only sends: `None` — the toast
/// itself was clicked, there being no button to carry an argument — is a click on that session;
/// an argument is some button's, and this window offers none.
pub(super) fn on_activated(
    events: super::Events,
    project: PathBuf,
    session: SessionId,
) -> impl Fn(Option<String>) -> tauri_winrt_notification::Result<()> + Send + 'static {
    move |argument| {
        if argument.is_none() {
            // The window is gone when nobody receives.
            let _ = events.unbounded_send(NotifierEvent::Activated {
                project: project.clone(),
                session,
                activation: None,
            });
        }
        Ok(())
    }
}

/// The Windows notifier: the channel a click is reported on, and the kind icons' files. Each toast is built and handed to
/// the system.
pub(super) struct Notifier {
    events: super::Events,
    icons: &'static IconFiles,
}

impl Notifier {
    pub(super) fn new(events: super::Events, icons: &'static IconFiles) -> Self {
        Self { events, icons }
    }
}

impl DesktopNotifier for Notifier {
    /// Windows shows the toast only when a Start-menu shortcut carries [`APP_USER_MODEL_ID`], which
    /// is so for an installed build. Whether it reports an error otherwise is not documented
    /// (research R4), so an `Ok` here does not promise a toast on screen.
    fn show(&self, notification: DesktopNotification) -> Result<(), NotifyError> {
        let text = toast_text(&notification);
        let mut toast = tauri_winrt_notification::Toast::new(APP_USER_MODEL_ID)
            .title(&text.title)
            .text1(&text.line);
        if let Some(icon) = toast_icon(notification.kind, self.icons) {
            toast = toast.icon(&icon.path, ICON_CROP, icon.alt);
        }
        toast
            .on_activated(on_activated(
                self.events.clone(),
                notification.project,
                notification.session,
            ))
            .show()
            .map_err(|error| notify_error(&error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::futures::channel::mpsc;
    use micold_client::notification_icon::{write_files, IconFiles};
    use micold_core::attention::NotificationKind;
    use tauri_winrt_notification::{Error, IconCrop};

    fn notification(title: &str, body: &str) -> DesktopNotification {
        DesktopNotification {
            kind: NotificationKind::NeedsPermission,
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

    #[test]
    fn a_click_on_the_toast_is_an_activation_of_its_session() {
        // U174 (FR-011, N5): the system calls the handler with no argument.
        let (events, mut received) = mpsc::unbounded();
        let session = SessionId::new();
        let handler = on_activated(events, PathBuf::from("/repo"), session);
        assert!(handler(None).is_ok());
        assert_eq!(
            received.try_recv().ok(),
            Some(NotifierEvent::Activated {
                project: PathBuf::from("/repo"),
                session,
                activation: None,
            })
        );
        assert!(received.try_recv().is_err(), "one click, one event");
    }

    #[test]
    fn each_toast_reports_its_own_session() {
        // U174 (N9): the table is keyed per toast.
        let (events, mut received) = mpsc::unbounded();
        let (first, second) = (SessionId::new(), SessionId::new());
        let on_first = on_activated(events.clone(), PathBuf::from("/repo"), first);
        let on_second = on_activated(events, PathBuf::from("/other"), second);
        assert!(on_second(None).is_ok());
        assert!(on_first(None).is_ok());
        assert_eq!(
            received.try_recv().ok(),
            Some(NotifierEvent::Activated {
                project: PathBuf::from("/other"),
                session: second,
                activation: None,
            })
        );
        assert_eq!(
            received.try_recv().ok(),
            Some(NotifierEvent::Activated {
                project: PathBuf::from("/repo"),
                session: first,
                activation: None,
            })
        );
    }

    #[test]
    fn an_activation_with_an_argument_is_no_event() {
        // U174: an argument is a button's, and the toast offers none.
        let (events, mut received) = mpsc::unbounded();
        let handler = on_activated(events, PathBuf::from("/repo"), SessionId::new());
        assert!(handler(Some("open".to_string())).is_ok());
        assert!(received.try_recv().is_err());
    }

    #[test]
    fn a_click_after_the_window_stopped_listening_is_not_an_error() {
        // N9: the click is reported to nobody.
        let (events, received) = mpsc::unbounded();
        drop(received);
        let handler = on_activated(events, PathBuf::from("/repo"), SessionId::new());
        assert!(handler(None).is_ok());
    }

    #[test]
    fn the_toast_for_a_kind_with_an_icon_file_carries_it_square_with_the_kinds_name() {
        // Feature 613, I6 (FR-016): the app logo override, cropped square, named for the kind.
        let dir = tempfile::tempdir().expect("temp dir");
        let files = write_files(dir.path()).expect("a writable directory");
        for kind in NotificationKind::ALL {
            assert_eq!(
                toast_icon(kind, &files),
                Some(ToastIcon {
                    path: files.path(kind).expect("written").to_path_buf(),
                    alt: kind.name(),
                }),
                "{kind:?}"
            );
        }
        assert!(ICON_CROP == IconCrop::Square);
    }

    #[test]
    fn without_an_icon_file_the_toast_carries_no_image() {
        // I6 (FR-016): the toast is still shown; its title names the kind.
        for kind in NotificationKind::ALL {
            assert_eq!(toast_icon(kind, &IconFiles::default()), None, "{kind:?}");
        }
    }
}
