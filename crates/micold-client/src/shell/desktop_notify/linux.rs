//! The Linux desktop notification: `org.freedesktop.Notifications.Notify` on the session bus, over
//! one `zbus` connection (feature 039, research R4, contract "Backends").
//!
//! Two pure functions decide everything — [`notify_request`], the call's arguments, and
//! [`notify_error`], what a bus failure means — and are tested with no bus. [`Notifier::show`] is
//! the call, with no branch of its own. No action is offered yet: the click arrives with story 3.

use std::collections::HashMap;
use std::sync::Mutex;

use micold_client::features::attention::{DesktopNotification, DesktopNotifier, NotifyError};

/// The application name the notification service shows, as the desktop entry names it.
const APP_NAME: &str = "Micold AI IDE";
/// The desktop entry the service takes the icon and the name from (`packaging/micold-ai-ide.desktop`).
const DESKTOP_ENTRY: &str = "micold-ai-ide";

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

/// The `Notify` call for `notification`: the title as the summary, the body, and the
/// `desktop-entry` hint; nothing else (contract N2). Never replaces an earlier one (N3).
pub(super) fn notify_request(notification: &DesktopNotification) -> NotifyRequest {
    NotifyRequest {
        app_name: APP_NAME,
        replaces_id: 0,
        app_icon: "",
        summary: notification.title.clone(),
        body: notification.body.clone(),
        actions: Vec::new(),
        hints: vec![("desktop-entry", DESKTOP_ENTRY.to_string())],
        // The server's own default.
        expire_timeout: -1,
    }
}

/// What a bus failure means for the user (FR-010): no notification service to ask, or one that
/// did not accept the notification.
pub(super) fn notify_error(error: &zbus::Error) -> NotifyError {
    const NOBODY_THERE: [&str; 2] = [
        "org.freedesktop.DBus.Error.ServiceUnknown",
        "org.freedesktop.DBus.Error.NameHasNoOwner",
    ];
    match error {
        // No session bus to reach.
        zbus::Error::Address(_)
        | zbus::Error::InputOutput(_)
        | zbus::Error::Handshake(_)
        | zbus::Error::Connection(..) => NotifyError::NoService(error.to_string()),
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

/// The Linux notifier. The session-bus connection is opened on the first notification and kept;
/// a failed open is tried again on the next one.
pub(super) struct Notifier {
    connection: Mutex<Option<zbus::blocking::Connection>>,
}

impl Notifier {
    pub(super) fn new() -> Self {
        Self {
            connection: Mutex::new(None),
        }
    }

    fn connection(&self) -> Result<zbus::blocking::Connection, NotifyError> {
        let mut held = self
            .connection
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(connection) = held.as_ref() {
            return Ok(connection.clone());
        }
        let connection =
            zbus::blocking::Connection::session().map_err(|error| notify_error(&error))?;
        *held = Some(connection.clone());
        Ok(connection)
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
        self.connection()?
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
            .map_err(|error| notify_error(&error))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use micold_core::session::SessionId;
    use std::path::PathBuf;

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
                actions: Vec::new(),
                hints: vec![("desktop-entry", "micold-ai-ide".to_string())],
                expire_timeout: -1,
            }
        );
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
