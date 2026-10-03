//! The Linux desktop notification: `org.freedesktop.Notifications.Notify` on the session bus, over
//! one `zbus` connection (feature 039, research R4, contract "Backends").
//!
//! Pure functions decide everything and are tested with no bus: [`notify_request`], the call's
//! arguments; [`notify_error`], what a bus failure means; [`signal`], which of the service's
//! signals a bus message is; and [`Shown::on_signal`], what that signal means for this window.
//! [`Notifier::show`] is the call and [`Notifier::listen`] the loop that reads the signals.
//!
//! The request offers the `default` action, which the specification gives a click on the
//! notification's body. A service without the `actions` capability ignores it and still shows the
//! notification (contract N7); this window then never hears of a click.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use micold_client::features::attention::{
    DesktopNotification, DesktopNotifier, NotifierEvent, NotifyError,
};
use micold_core::session::SessionId;
use std::path::PathBuf;

/// The application name the notification service shows, as the desktop entry names it.
const APP_NAME: &str = "Micold AI IDE";
/// The desktop entry the service takes the icon and the name from (`packaging/micold-ai-ide.desktop`).
const DESKTOP_ENTRY: &str = "micold-ai-ide";
/// The action a click on the notification's body invokes, by the specification.
const DEFAULT_ACTION: &str = "default";
/// The label of that action, for a service that shows actions as buttons.
const DEFAULT_ACTION_LABEL: &str = "Open";
/// The two signals of the notification service this window reads.
const ACTION_INVOKED: &str = "ActionInvoked";
const NOTIFICATION_CLOSED: &str = "NotificationClosed";
/// Every signal of the notification service's interface.
const SIGNALS: &str = "type='signal',interface='org.freedesktop.Notifications',\
                       path='/org/freedesktop/Notifications'";
/// How long the notification service has to answer a call. A service that does not answer in
/// that time is treated as one that is not there.
const METHOD_TIMEOUT: Duration = Duration::from_secs(2);

/// The arguments of one `Notify` call, in the order of the specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NotifyRequest {
    pub app_name: &'static str,
    pub replaces_id: u32,
    pub app_icon: &'static str,
    pub summary: String,
    pub body: String,
    pub actions: Vec<String>,
    pub hints: Vec<(&'static str, String)>,
    pub expire_timeout: i32,
}

/// `text` as notification body markup: a server with the `body-markup` capability reads `&`, `<`
/// and `>` as markup, so a name that holds one is escaped to be shown as written (FR-004).
fn escape_markup(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            other => escaped.push(other),
        }
    }
    escaped
}

/// The `Notify` call for `notification`: the title as the summary, the body, the `default` action
/// and the `desktop-entry` hint; nothing else (contract N2). The action is offered whatever the
/// service can do (N7). Never replaces an earlier one (N3). The
/// summary is plain text by the specification; the body is markup, so it is escaped.
pub(super) fn notify_request(notification: &DesktopNotification) -> NotifyRequest {
    NotifyRequest {
        app_name: APP_NAME,
        replaces_id: 0,
        app_icon: "",
        summary: notification.title.clone(),
        body: escape_markup(&notification.body),
        actions: vec![
            DEFAULT_ACTION.to_string(),
            DEFAULT_ACTION_LABEL.to_string(),
        ],
        hints: vec![("desktop-entry", DESKTOP_ENTRY.to_string())],
        // The server's own default.
        expire_timeout: -1,
    }
}

/// A signal of the notification service about a notification, as far as this window reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Signal {
    /// `ActionInvoked(id, key)`: the user chose the action `key` of notification `id`.
    ActionInvoked { id: u32, key: String },
    /// `NotificationClosed(id, _)`: notification `id` is gone, whatever the reason.
    NotificationClosed { id: u32 },
}

/// The signal `message` carries, when it is one of the two this window reads.
pub(super) fn signal(message: &zbus::Message) -> Option<Signal> {
    let header = message.header();
    let body = message.body();
    match header.member()?.as_str() {
        ACTION_INVOKED => {
            let (id, key) = body.deserialize::<(u32, String)>().ok()?;
            Some(Signal::ActionInvoked { id, key })
        }
        NOTIFICATION_CLOSED => {
            let (id, _reason) = body.deserialize::<(u32, u32)>().ok()?;
            Some(Signal::NotificationClosed { id })
        }
        _ => None,
    }
}

/// The notifications this window raised that are still shown: the service's id for each, against
/// what it named. It is this window's own (contract N9): an id it does not hold is another
/// window's notification, or one raised before this window started.
#[derive(Debug, Default)]
pub(super) struct Shown {
    by_id: HashMap<u32, (PathBuf, SessionId)>,
}

impl Shown {
    /// The service showed a notification for `session` of `project` under `id`.
    pub(super) fn record(&mut self, id: u32, project: PathBuf, session: SessionId) {
        self.by_id.insert(id, (project, session));
    }

    /// Whether `id` is a notification of this window that is still shown.
    pub(super) fn holds(&self, id: u32) -> bool {
        self.by_id.contains_key(&id)
    }

    /// What `signal` means for this window: the `default` action of a notification it holds is
    /// a click on that notification's session, reported once; a closed notification is forgotten;
    /// anything else is nothing (N9).
    pub(super) fn on_signal(&mut self, signal: Signal) -> Option<NotifierEvent> {
        match signal {
            Signal::ActionInvoked { id, key } if key == DEFAULT_ACTION => {
                let (project, session) = self.by_id.remove(&id)?;
                Some(NotifierEvent::Activated { project, session })
            }
            Signal::ActionInvoked { .. } => None,
            Signal::NotificationClosed { id } => {
                self.by_id.remove(&id);
                None
            }
        }
    }
}

/// Whether `error` says the connection to the bus is no good — it could not be opened, it broke,
/// or a call on it was not answered in time — so that a kept one is dropped and the next
/// notification opens a new one.
pub(super) fn is_connection_error(error: &zbus::Error) -> bool {
    matches!(
        error,
        zbus::Error::Address(_)
            | zbus::Error::InputOutput(_)
            | zbus::Error::Handshake(_)
            | zbus::Error::Connection(..)
    )
}

/// What a bus failure means for the user (FR-010): no notification service to ask, or one that
/// did not accept the notification. A call that was not answered within [`METHOD_TIMEOUT`]
/// arrives as an I/O error of kind `TimedOut`, and is no service.
pub(super) fn notify_error(error: &zbus::Error) -> NotifyError {
    const NOBODY_THERE: [&str; 2] = [
        "org.freedesktop.DBus.Error.ServiceUnknown",
        "org.freedesktop.DBus.Error.NameHasNoOwner",
    ];
    match error {
        // No session bus to reach, or no answer in time.
        error if is_connection_error(error) => NotifyError::NoService(error.to_string()),
        // A bus with nobody serving the notification interface on it.
        zbus::Error::FDO(fdo) => match fdo.as_ref() {
            zbus::fdo::Error::ServiceUnknown(why) | zbus::fdo::Error::NameHasNoOwner(why) => {
                NotifyError::NoService(why.clone())
            }
            other => NotifyError::Refused(other.to_string()),
        },
        zbus::Error::MethodError(name, why, _) if NOBODY_THERE.contains(&name.as_str()) => {
            NotifyError::NoService(why.clone().unwrap_or_else(|| name.to_string()))
        }
        other => NotifyError::Refused(other.to_string()),
    }
}

/// The Linux notifier. The session-bus connection is opened on the first notification and kept
/// until a call on it fails with a connection error; a failed open is tried again on the next one.
/// Each connection has a thread that reads the service's signals from it ([`Self::listen`]).
pub(super) struct Notifier {
    connection: Mutex<Option<zbus::blocking::Connection>>,
    /// What this window has on screen. Shared with the listening threads.
    shown: Arc<Mutex<Shown>>,
    /// Where a click is reported.
    events: super::Events,
}

impl Notifier {
    pub(super) fn new(events: super::Events) -> Self {
        Self {
            connection: Mutex::new(None),
            shown: Arc::new(Mutex::new(Shown::default())),
            events,
        }
    }

    /// Read the notification service's signals from `connection` on a thread of their own, and
    /// report what [`Shown::on_signal`] makes of each. The thread ends with the connection, or
    /// when nobody reads the events any more. A connection that cannot be listened on still
    /// shows notifications (contract N7), so every failure here is passed over.
    fn listen(&self, connection: &zbus::blocking::Connection) {
        let connection = connection.clone();
        let shown = Arc::clone(&self.shown);
        let events = self.events.clone();
        let _ = std::thread::Builder::new()
            .name("desktop-notify-clicks".to_string())
            .spawn(move || {
                let Ok(signals) =
                    zbus::blocking::MessageIterator::for_match_rule(SIGNALS, &connection, None)
                else {
                    return;
                };
                for message in signals {
                    let Ok(message) = message else {
                        return;
                    };
                    let event = signal(&message).and_then(|signal| {
                        shown
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .on_signal(signal)
                    });
                    if event.is_some_and(|event| events.unbounded_send(event).is_err()) {
                        return;
                    }
                }
            });
    }

    fn connection(&self) -> Result<zbus::blocking::Connection, NotifyError> {
        let mut held = self
            .connection
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(connection) = held.as_ref() {
            return Ok(connection.clone());
        }
        let connection = zbus::blocking::connection::Builder::session()
            .map(|builder| builder.method_timeout(METHOD_TIMEOUT))
            .and_then(zbus::blocking::connection::Builder::build)
            .map_err(|error| notify_error(&error))?;
        self.listen(&connection);
        *held = Some(connection.clone());
        Ok(connection)
    }

    /// Drop the kept connection, so the next notification opens a new one.
    fn forget_connection(&self) {
        *self
            .connection
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    }
}

impl DesktopNotifier for Notifier {
    fn show(&self, notification: DesktopNotification) -> Result<(), NotifyError> {
        let request = notify_request(&notification);
        let hints: HashMap<&str, zbus::zvariant::Value<'_>> = request
            .hints
            .iter()
            .map(|(key, value)| (*key, zbus::zvariant::Value::from(value.as_str())))
            .collect();
        let reply = self
            .connection()?
            .call_method(
                Some("org.freedesktop.Notifications"),
                "/org/freedesktop/Notifications",
                Some("org.freedesktop.Notifications"),
                "Notify",
                &(
                    request.app_name,
                    request.replaces_id,
                    request.app_icon,
                    request.summary.as_str(),
                    request.body.as_str(),
                    &request.actions,
                    hints,
                    request.expire_timeout,
                ),
            )
            .map_err(|error| {
                if is_connection_error(&error) {
                    self.forget_connection();
                }
                notify_error(&error)
            })?;
        // The service's id for the notification, by which its signals name it. A reply that
        // carries none leaves the notification shown and its click unheard (N7).
        if let Ok(id) = reply.body().deserialize::<u32>() {
            self.shown
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .record(id, notification.project, notification.session);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notification() -> DesktopNotification {
        DesktopNotification {
            title: "Fix the parser is waiting for input".to_string(),
            body: "repo \u{2014} Parser work".to_string(),
            project: PathBuf::from("/repo"),
            session: SessionId::new(),
        }
    }

    #[test]
    fn notify_request_carries_the_app_name_the_text_and_the_desktop_entry_and_nothing_else() {
        // U159 (FR-004, N2, N3).
        assert_eq!(
            notify_request(&notification()),
            NotifyRequest {
                app_name: "Micold AI IDE",
                replaces_id: 0,
                app_icon: "",
                summary: "Fix the parser is waiting for input".to_string(),
                body: "repo \u{2014} Parser work".to_string(),
                actions: vec!["default".to_string(), "Open".to_string()],
                hints: vec![("desktop-entry", "micold-ai-ide".to_string())],
                expire_timeout: -1,
            }
        );
    }

    #[test]
    fn the_request_offers_the_default_action_whatever_the_service_can_do() {
        // U161 (FR-015, N7). `notify_request` is not told the service's capabilities, so the
        // request is the same for a service that lists `actions` and for one that does not: a
        // text-only service ignores the pair and still shows the notification.
        assert_eq!(
            notify_request(&notification()).actions,
            ["default", "Open"],
            "one action: the key the specification gives a click on the body, and its label"
        );
    }

    const SHOWN: u32 = 7;
    const NOT_OURS: u32 = 8;

    fn shown_for(session: SessionId) -> Shown {
        let mut shown = Shown::default();
        shown.record(SHOWN, PathBuf::from("/repo"), session);
        shown
    }

    fn invoked(id: u32, key: &str) -> Signal {
        Signal::ActionInvoked {
            id,
            key: key.to_string(),
        }
    }

    #[test]
    fn the_default_action_of_a_notification_in_the_table_is_an_activation_of_its_session() {
        // U162 (FR-011).
        let session = SessionId::new();
        let mut shown = shown_for(session);
        assert_eq!(
            shown.on_signal(invoked(SHOWN, "default")),
            Some(NotifierEvent::Activated {
                project: PathBuf::from("/repo"),
                session,
            })
        );
    }

    #[test]
    fn a_click_is_reported_once_however_many_times_the_service_says_it() {
        // U162: a second connection of this window hears the same signal.
        let mut shown = shown_for(SessionId::new());
        assert!(shown.on_signal(invoked(SHOWN, "default")).is_some());
        assert_eq!(shown.on_signal(invoked(SHOWN, "default")), None);
    }

    #[test]
    fn an_id_the_table_does_not_hold_is_no_event() {
        // U163 (FR-015, N9): another window's notification, or one older than this window.
        let mut shown = shown_for(SessionId::new());
        assert_eq!(shown.on_signal(invoked(NOT_OURS, "default")), None);
        assert!(shown.holds(SHOWN), "this window's own is untouched");
    }

    #[test]
    fn another_action_key_is_no_event() {
        // U164 (FR-011): only the action this window offered opens a session.
        let mut shown = shown_for(SessionId::new());
        assert_eq!(shown.on_signal(invoked(SHOWN, "dismiss")), None);
        assert!(shown.holds(SHOWN), "and the notification is still shown");
    }

    #[test]
    fn a_closed_notification_leaves_the_table() {
        // U165 (FR-015).
        let mut shown = shown_for(SessionId::new());
        assert_eq!(
            shown.on_signal(Signal::NotificationClosed { id: SHOWN }),
            None
        );
        assert!(!shown.holds(SHOWN));
        assert_eq!(
            shown.on_signal(invoked(SHOWN, "default")),
            None,
            "a click the service reports after the close finds nothing"
        );
    }

    fn message<B>(member: &str, body: &B) -> zbus::Message
    where
        B: serde::Serialize + zbus::zvariant::DynamicType,
    {
        zbus::Message::signal(
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
            member,
        )
        .expect("a signal header")
        .build(body)
        .expect("a signal")
    }

    #[test]
    fn the_two_signals_are_read_from_the_bus_messages_that_carry_them() {
        // U162, U165: the id and the action key of `ActionInvoked`; the id of
        // `NotificationClosed`, whose second argument is the reason.
        assert_eq!(
            signal(&message("ActionInvoked", &(SHOWN, "default"))),
            Some(invoked(SHOWN, "default"))
        );
        assert_eq!(
            signal(&message("NotificationClosed", &(SHOWN, 2u32))),
            Some(Signal::NotificationClosed { id: SHOWN })
        );
    }

    #[test]
    fn any_other_signal_and_a_signal_with_another_body_is_not_read() {
        // U164: `ActivationToken` is read with the Wayland slice; a body of another shape is not
        // a signal of the specification.
        assert_eq!(signal(&message("ActivationToken", &(SHOWN, "token"))), None);
        assert_eq!(signal(&message("ActionInvoked", &(SHOWN,))), None);
    }

    #[test]
    fn the_body_is_escaped_as_markup_and_the_summary_is_left_as_plain_text() {
        // Review A F2 (FR-004): the body is markup on a server with `body-markup`; the summary
        // is plain text by the specification.
        let request = notify_request(&DesktopNotification {
            title: "R&D <x> is waiting for input".to_string(),
            body: "R&D \u{2014} <x>".to_string(),
            project: PathBuf::from("/repo"),
            session: SessionId::new(),
        });
        assert_eq!(request.body, "R&amp;D \u{2014} &lt;x&gt;");
        assert_eq!(request.summary, "R&D <x> is waiting for input");
    }

    #[test]
    fn a_call_that_timed_out_is_no_notification_service() {
        // Review A F1c: `zbus` reports a method call past its timeout as an I/O error of kind
        // `TimedOut`.
        let error = zbus::Error::InputOutput(std::sync::Arc::new(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "timed out",
        )));
        match notify_error(&error) {
            NotifyError::NoService(why) => assert!(why.contains("timed out"), "{why}"),
            other => panic!("expected no service, got {other:?}"),
        }
    }

    #[test]
    fn a_broken_bus_is_a_connection_error_and_a_services_answer_is_not() {
        // Review A F3: the kept connection is dropped after the first kind, not after the second.
        assert!(is_connection_error(&zbus::Error::InputOutput(
            std::sync::Arc::new(std::io::Error::from(std::io::ErrorKind::BrokenPipe))
        )));
        assert!(is_connection_error(&zbus::Error::InputOutput(
            std::sync::Arc::new(std::io::Error::from(std::io::ErrorKind::TimedOut))
        )));
        assert!(!is_connection_error(&zbus::Error::FDO(Box::new(
            zbus::fdo::Error::ServiceUnknown("nobody".to_string())
        ))));
        assert!(!is_connection_error(&zbus::Error::FDO(Box::new(
            zbus::fdo::Error::AccessDenied("blocked".to_string())
        ))));
    }

    #[test]
    fn no_session_bus_is_no_notification_service() {
        // U160 (FR-010, Edge: system refuses).
        for error in [
            zbus::Error::Address("DBUS_SESSION_BUS_ADDRESS is not set".to_string()),
            zbus::Error::InputOutput(std::sync::Arc::new(std::io::Error::from(
                std::io::ErrorKind::ConnectionRefused,
            ))),
            zbus::Error::Handshake("no common mechanism".to_string()),
        ] {
            assert!(
                matches!(notify_error(&error), NotifyError::NoService(_)),
                "{error:?} is no notification service"
            );
        }
    }

    #[test]
    fn nobody_serving_the_interface_is_no_notification_service() {
        // U160: a bus with no notification daemon on it answers `ServiceUnknown`.
        let error = zbus::Error::FDO(Box::new(zbus::fdo::Error::ServiceUnknown(
            "The name org.freedesktop.Notifications was not provided by any .service files"
                .to_string(),
        )));
        assert_eq!(
            notify_error(&error),
            NotifyError::NoService(
                "The name org.freedesktop.Notifications was not provided by any .service files"
                    .to_string()
            )
        );
    }

    #[test]
    fn any_other_bus_failure_is_a_refusal_with_the_buses_reason() {
        // U160.
        let error = zbus::Error::FDO(Box::new(zbus::fdo::Error::AccessDenied(
            "notifications are blocked".to_string(),
        )));
        match notify_error(&error) {
            NotifyError::Refused(why) => {
                assert!(why.contains("notifications are blocked"), "{why}")
            }
            other => panic!("expected a refusal, got {other:?}"),
        }
    }
}
