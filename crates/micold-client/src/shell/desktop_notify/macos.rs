//! The macOS desktop notification: `UNUserNotificationCenter`, through `mac-usernotifications`
//! (feature 039, research R4, contract "Backends").
//!
//! Pure functions decide everything — [`banner`], the text handed to the system, [`authorised`],
//! what the user's answer to the system's permission prompt means, [`notify_error`], what a
//! failure of the crate means, [`outcome`], what `show` reports when the system has not answered,
//! [`prompt_is_open`], when a notification is not handed over at all, and [`Shown::on_response`],
//! what the user's response to a notification means — and are tested without a bundle.
//! [`Notifier::show`] and [`deliver`] are the calls.
//!
//! # The click
//!
//! The crate's delegate hands a response to whoever awaits the notification's handle. The thread
//! that delivered a notification therefore stays, parked, until the user clicks or clears it or
//! [`CLICK_WAIT`] passes; nothing wakes it in between. A notification without buttons and without
//! a timeout would instead make the crate ask the notification centre every 500 ms whether it is
//! still there, for as long as it is (`mac-usernotifications` 0.3.1, `src/send.rs`,
//! `poll_until_dismissed`), so each one carries [`CLICK_WAIT`] as its timeout.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use mac_usernotifications::{NotificationHandle, NotificationResponse};
use micold_client::features::attention::{
    DesktopNotification, DesktopNotifier, NotifierEvent, NotifyError,
};
use micold_core::session::SessionId;

/// How long [`Notifier::show`] waits for the system to answer. The request for authorisation is
/// answered at once when the user has decided before, and only when they decide while the system's
/// prompt is open: `show` does not wait for a person.
const ANSWER_WAIT: Duration = Duration::from_secs(2);
/// The reason logged when the user has not allowed the application's notifications.
const NOT_ALLOWED: &str = "notifications are not allowed for this application";
/// The reason logged when the system has not answered within [`ANSWER_WAIT`].
const NOT_ANSWERED: &str =
    "the system has not answered the request to allow notifications; it is shown once allowed";

/// How long a notification can be clicked. When it passes, the crate takes the notification out
/// of the notification centre and the thread that waited for the click ends; the session keeps
/// its unread mark.
const CLICK_WAIT: Duration = Duration::from_secs(60 * 60);

/// What the system is asked to show: a title and the message under it. Both are plain text to
/// `UNNotificationContent`, so names are passed as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Banner {
    pub title: String,
    pub message: String,
}

/// The banner for `notification`: its title and its body, and nothing else (contract N2).
pub(super) fn banner(notification: &DesktopNotification) -> Banner {
    Banner {
        title: notification.title.clone(),
        message: notification.body.clone(),
    }
}

/// What a failure of `mac-usernotifications` means for the user (FR-010): a binary outside a
/// bundle has no notification centre to ask; anything else is the system not taking the request.
pub(super) fn notify_error(error: &mac_usernotifications::Error) -> NotifyError {
    match error {
        mac_usernotifications::Error::NoBundleIdentifier => {
            NotifyError::NoService(error.to_string())
        }
        other => NotifyError::Refused(other.to_string()),
    }
}

/// What the answer to the request for authorisation means: `Ok(true)` lets the notification go
/// on; a user who refused, or a system that could not be asked, is an error (FR-010).
pub(super) fn authorised(
    answer: Result<bool, mac_usernotifications::Error>,
) -> Result<(), NotifyError> {
    match answer {
        Ok(true) => Ok(()),
        Ok(false) => Err(NotifyError::Refused(NOT_ALLOWED.to_string())),
        Err(error) => Err(notify_error(&error)),
    }
}

/// What `show` reports for a notification handed to the system: what the system answered, or,
/// when it has not answered in the time `show` waits — the user has the permission prompt open —
/// an error that says so (FR-010). The notification is still shown if the user then allows it.
pub(super) fn outcome(delivered: Option<Result<(), NotifyError>>) -> Result<(), NotifyError> {
    delivered.unwrap_or_else(|| Err(NotifyError::Refused(NOT_ANSWERED.to_string())))
}

/// Whether the system's permission prompt is taken to be open: a request for authorisation has
/// gone unanswered for as long as `show` waits. A notification raised meanwhile is not handed to
/// the system, so that the user's answer does not release a burst of banners, some of them for
/// sessions that no longer wait.
pub(super) fn prompt_is_open(unanswered_for: Option<Duration>) -> bool {
    unanswered_for.is_some_and(|waited| waited >= ANSWER_WAIT)
}

/// The notifications this window raised that can still be clicked: the request identifier of
/// each, against what it named. It is this window's own (contract N9).
#[derive(Debug, Default)]
pub(super) struct Shown {
    by_id: HashMap<String, (PathBuf, SessionId)>,
}

impl Shown {
    /// The system took a notification for `session` of `project` under `id`.
    pub(super) fn record(&mut self, id: String, project: PathBuf, session: SessionId) {
        self.by_id.insert(id, (project, session));
    }

    /// What the user's `response` means for this window: the default action — a click on the
    /// notification itself — of a notification it holds is a click on that notification's
    /// session; a dismissal, a timeout, another action, or a notification it does not hold is
    /// nothing (N9). Either way the notification is over, and is forgotten.
    pub(super) fn on_response(&mut self, response: &NotificationResponse) -> Option<NotifierEvent> {
        let (project, session) = self.by_id.remove(&response.notification_id)?;
        (response.close_reason.is_none() && response.is_default_action()).then_some(
            NotifierEvent::Activated {
                project,
                session,
                // The Wayland token: there is none on macOS.
                activation: None,
            },
        )
    }
}

/// When the request for authorisation that is still unanswered was made.
type Asking = Arc<Mutex<Option<Instant>>>;

fn locked<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Ask for authorisation, then hand `banner` to the notification centre. The first request shows
/// the system's own prompt and returns when the user answers it; later ones return at once. A
/// binary outside a bundle is refused by the crate's `check_bundle` before the system is touched.
/// The handle is what the user's response arrives on.
fn deliver(banner: Banner, asking: &Asking) -> Result<NotificationHandle, NotifyError> {
    locked(asking).get_or_insert_with(Instant::now);
    let answer = mac_usernotifications::blocking::request_auth();
    *locked(asking) = None;
    authorised(answer)?;
    // Not `send_blocking`: it refuses with `MainThreadNotRunning` whenever the main run loop is
    // busy at that instant. The request is completed on a queue of the system's, as
    // `blocking::send` relies on too.
    mac_usernotifications::block_on(
        mac_usernotifications::Notification::new()
            .title(banner.title)
            .message(banner.message)
            .timeout(CLICK_WAIT)
            .send(),
    )
    .map_err(|error| notify_error(&error))
}

/// The macOS notifier: the channel a click is reported on, the notifications that can still be
/// clicked, and whether the permission prompt is open. The notification centre is the system's.
pub(super) struct Notifier {
    events: super::Events,
    shown: Arc<Mutex<Shown>>,
    asking: Asking,
}

impl Notifier {
    pub(super) fn new(events: super::Events) -> Self {
        Self {
            events,
            shown: Arc::default(),
            asking: Arc::default(),
        }
    }
}

impl DesktopNotifier for Notifier {
    /// Delivers on a thread of its own and waits [`ANSWER_WAIT`] for the result, so that an open
    /// permission prompt holds neither the caller nor, at exit, the runtime's blocking pool. The
    /// thread then waits for the user's response and reports a click (module docs).
    fn show(&self, notification: DesktopNotification) -> Result<(), NotifyError> {
        if prompt_is_open(locked(&self.asking).map(|since| since.elapsed())) {
            return outcome(None);
        }
        let banner = banner(&notification);
        let DesktopNotification {
            project, session, ..
        } = notification;
        let (events, shown, asking) = (
            self.events.clone(),
            Arc::clone(&self.shown),
            Arc::clone(&self.asking),
        );
        let (answer, answered) = mpsc::channel();
        std::thread::Builder::new()
            .name("desktop-notify".to_string())
            .spawn(move || {
                let delivered = deliver(banner, &asking);
                // Nobody listens once `show` has stopped waiting.
                let _ = answer.send(delivered.as_ref().map(drop).map_err(Clone::clone));
                let Ok(handle) = delivered else { return };
                locked(&shown).record(handle.notification_id().to_string(), project, session);
                // An error is the crate's delegate gone: nobody can click any more.
                let Ok(response) = mac_usernotifications::block_on(handle.response()) else {
                    return;
                };
                if let Some(event) = locked(&shown).on_response(&response) {
                    // The window is gone when nobody receives.
                    let _ = events.unbounded_send(event);
                }
            })
            .map_err(|error| NotifyError::Refused(error.to_string()))?;
        outcome(answered.recv_timeout(ANSWER_WAIT).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mac_usernotifications::{CloseReason, Error};

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

    /// The identifiers the system gives the two responses it makes up itself (Apple's
    /// `UNNotificationDefaultActionIdentifier` and `UNNotificationDismissActionIdentifier`).
    const DEFAULT_ACTION: &str = "com.apple.UNNotificationDefaultActionIdentifier";
    const DISMISS_ACTION: &str = "com.apple.UNNotificationDismissActionIdentifier";

    fn response(id: &str, action: &str, close_reason: Option<CloseReason>) -> NotificationResponse {
        NotificationResponse {
            notification_id: id.to_string(),
            action_identifier: action.to_string(),
            reply_text: None,
            close_reason,
        }
    }

    fn shown_for(id: &str, session: SessionId) -> Shown {
        let mut shown = Shown::default();
        shown.record(id.to_string(), PathBuf::from("/repo"), session);
        shown
    }

    #[test]
    fn the_default_action_of_a_notification_in_the_table_is_an_activation_of_its_session() {
        // U169 (FR-011, N5): a click on the notification itself.
        let session = SessionId::new();
        let other = SessionId::new();
        let mut shown = shown_for("a", session);
        shown.record("b".to_string(), PathBuf::from("/other"), other);
        assert_eq!(
            shown.on_response(&response("a", DEFAULT_ACTION, None)),
            Some(NotifierEvent::Activated {
                project: PathBuf::from("/repo"),
                session,
                activation: None,
            })
        );
        assert_eq!(
            shown.on_response(&response("b", DEFAULT_ACTION, None)),
            Some(NotifierEvent::Activated {
                project: PathBuf::from("/other"),
                session: other,
                activation: None,
            })
        );
    }

    #[test]
    fn a_click_is_reported_once() {
        // U169 (N9).
        let mut shown = shown_for("a", SessionId::new());
        assert!(shown
            .on_response(&response("a", DEFAULT_ACTION, None))
            .is_some());
        assert_eq!(
            shown.on_response(&response("a", DEFAULT_ACTION, None)),
            None
        );
    }

    #[test]
    fn a_dismissal_is_no_event_and_ends_the_notification() {
        // U170: cleared by the user, as the system says it and as the crate makes it up.
        for action in [DISMISS_ACTION, ""] {
            let mut shown = shown_for("a", SessionId::new());
            let dismissed = response("a", action, Some(CloseReason::Dismissed));
            assert_eq!(shown.on_response(&dismissed), None);
            assert_eq!(
                shown.on_response(&response("a", DEFAULT_ACTION, None)),
                None
            );
        }
    }

    #[test]
    fn a_timeout_is_no_event_and_ends_the_notification() {
        // U170: `CLICK_WAIT` passed; the crate has taken the notification down.
        let mut shown = shown_for("a", SessionId::new());
        let expired = response("a", "", Some(CloseReason::Expired));
        assert_eq!(shown.on_response(&expired), None);
        assert_eq!(
            shown.on_response(&response("a", DEFAULT_ACTION, None)),
            None
        );
    }

    #[test]
    fn another_action_is_no_event() {
        // U170: the notification offers no button, so no other action is this window's.
        let mut shown = shown_for("a", SessionId::new());
        assert_eq!(shown.on_response(&response("a", "open", None)), None);
    }

    #[test]
    fn an_id_the_table_does_not_hold_is_no_event() {
        // U171 (N9), and the table is left as it was.
        let session = SessionId::new();
        let mut shown = shown_for("a", session);
        assert_eq!(
            shown.on_response(&response("b", DEFAULT_ACTION, None)),
            None
        );
        assert!(shown
            .on_response(&response("a", DEFAULT_ACTION, None))
            .is_some());
    }

    #[test]
    fn the_prompt_is_open_once_a_request_has_gone_unanswered_for_as_long_as_show_waits() {
        // M3 review A follow-up: no thread and no banner per notification behind an open prompt.
        assert!(!prompt_is_open(None));
        assert!(!prompt_is_open(Some(Duration::from_millis(50))));
        assert!(prompt_is_open(Some(ANSWER_WAIT)));
        assert!(prompt_is_open(Some(ANSWER_WAIT * 30)));
    }
}
