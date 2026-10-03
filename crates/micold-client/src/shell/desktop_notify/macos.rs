//! The macOS desktop notification: `UNUserNotificationCenter`, through `mac-usernotifications`
//! (feature 039, research R4, contract "Backends").
//!
//! Four pure functions decide everything — [`banner`], the text handed to the system,
//! [`authorised`], what the user's answer to the system's permission prompt means,
//! [`notify_error`], what a failure of the crate means, and [`outcome`], what `show` reports when
//! the system has not answered — and are tested without a bundle.
//! [`Notifier::show`] is the calls, with no branch of its own. The click arrives with story 3.

use micold_client::features::attention::{DesktopNotification, NotifyError};

/// What the system is asked to show: a title and the message under it. Both are plain text to
/// `UNNotificationContent`, so names are passed as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Banner {
    pub title: String,
    pub message: String,
}

/// The banner for `notification`: its title and its body, and nothing else (contract N2).
pub(super) fn banner(notification: &DesktopNotification) -> Banner {
    let _ = notification;
    todo!("T039")
}

/// What a failure of `mac-usernotifications` means for the user (FR-010).
pub(super) fn notify_error(error: &mac_usernotifications::Error) -> NotifyError {
    let _ = error;
    todo!("T039")
}

/// What the answer to the request for authorisation means: `Ok(true)` lets the notification go
/// on; a user who refused, or a system that could not be asked, is an error (FR-010).
pub(super) fn authorised(
    answer: Result<bool, mac_usernotifications::Error>,
) -> Result<(), NotifyError> {
    let _ = answer;
    todo!("T039")
}

/// What `show` reports for a notification handed to the system: what the system answered, or,
/// when it has not answered in the time `show` waits — the user has the permission prompt open —
/// an error that says so (FR-010). The notification is still shown if the user then allows it.
pub(super) fn outcome(delivered: Option<Result<(), NotifyError>>) -> Result<(), NotifyError> {
    let _ = delivered;
    todo!("T039")
}

#[cfg(test)]
mod tests {
    use super::*;
    use mac_usernotifications::Error;
    use micold_core::session::SessionId;
    use std::path::PathBuf;

    fn notification(title: &str, body: &str) -> DesktopNotification {
        DesktopNotification {
            title: title.to_string(),
            body: body.to_string(),
            project: PathBuf::from("/repo"),
            session: SessionId::new(),
        }
    }

    #[test]
    fn the_banner_carries_the_title_and_the_body_as_the_message() {
        // U167 (FR-004, FR-029, N2).
        assert_eq!(
            banner(&notification(
                "Fix the parser is waiting for input",
                "repo \u{2014} Parser work"
            )),
            Banner {
                title: "Fix the parser is waiting for input".to_string(),
                message: "repo \u{2014} Parser work".to_string(),
            }
        );
    }

    #[test]
    fn names_are_passed_as_written_because_the_system_reads_plain_text() {
        // U167: no markup on this system, so nothing is escaped (compare `linux.rs`).
        let banner = banner(&notification(
            "R&D <x> is waiting for input",
            "R&D \u{2014} <x>",
        ));
        assert_eq!(banner.title, "R&D <x> is waiting for input");
        assert_eq!(banner.message, "R&D \u{2014} <x>");
    }

    #[test]
    fn a_binary_outside_a_bundle_has_no_notification_service() {
        // U168 (FR-010): `cargo run`, or the bundle staged unsigned.
        match notify_error(&Error::NoBundleIdentifier) {
            NotifyError::NoService(why) => assert!(why.contains("bundle"), "{why}"),
            other => panic!("expected no service, got {other:?}"),
        }
    }

    #[test]
    fn a_request_the_system_rejected_is_a_refusal() {
        // U168.
        assert!(matches!(
            notify_error(&Error::NotificationRejected),
            NotifyError::Refused(_)
        ));
    }

    #[test]
    fn any_other_failure_of_the_crate_is_a_refusal_with_its_reason() {
        // U168: the crate's error is `non_exhaustive`.
        match notify_error(&Error::MainThreadNotRunning) {
            NotifyError::Refused(why) => assert_eq!(why, Error::MainThreadNotRunning.to_string()),
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[test]
    fn a_granted_authorisation_lets_the_notification_go_on() {
        // U168.
        assert_eq!(authorised(Ok(true)), Ok(()));
    }

    #[test]
    fn a_refused_authorisation_is_a_refusal_that_says_so() {
        // U168 (FR-010): the user answered the system's prompt with "Don't Allow", or turned the
        // application's notifications off in System Settings.
        match authorised(Ok(false)) {
            Err(NotifyError::Refused(why)) => assert!(why.contains("not allowed"), "{why}"),
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[test]
    fn what_the_system_answered_in_time_is_what_show_reports() {
        // U168.
        assert_eq!(outcome(Some(Ok(()))), Ok(()));
        let refused = NotifyError::Refused("blocked".to_string());
        assert_eq!(outcome(Some(Err(refused.clone()))), Err(refused));
    }

    #[test]
    fn a_system_that_has_not_answered_in_time_is_a_refusal_that_says_so() {
        // U168 (FR-010): the permission prompt is open and nobody has answered it. `show` must
        // not wait for the user.
        match outcome(None) {
            Err(NotifyError::Refused(why)) => assert!(why.contains("not answered"), "{why}"),
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[test]
    fn an_authorisation_that_could_not_be_asked_for_maps_as_any_other_failure() {
        // U168.
        assert!(matches!(
            authorised(Err(Error::NoBundleIdentifier)),
            Err(NotifyError::NoService(_))
        ));
        assert!(matches!(
            authorised(Err(Error::NotificationRejected)),
            Err(NotifyError::Refused(_))
        ));
    }
}
