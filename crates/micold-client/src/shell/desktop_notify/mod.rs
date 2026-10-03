//! The operating system's notification facility (feature 039, research R4,
//! `contracts/desktop-notification.md`).
//!
//! One function, [`system`], returns the backend of the operating system the client was built for.
//! Nothing outside this directory names an operating system (Principle VI); what is shown, and
//! when, is decided in `features::attention`.
//!
//! A backend reports a click over the channel it was built with ([`Events`]). The process has one
//! such channel: [`events`] is its sending end, and [`clicks`] the subscription that delivers what
//! arrives on it to the window. A backend may send from any thread.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

use std::sync::{Mutex, OnceLock};

use iced::futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use iced::futures::stream::{self, BoxStream, StreamExt};
use micold_client::features::attention::{DesktopNotifier, NotifierEvent};

/// The channel a backend reports on.
pub type Events = UnboundedSender<NotifierEvent>;

/// The process's one channel of backend reports: the sending end, and the receiving end until the
/// subscription takes it.
type Channel = (Events, Mutex<Option<UnboundedReceiver<NotifierEvent>>>);

fn channel() -> &'static Channel {
    static CHANNEL: OnceLock<Channel> = OnceLock::new();
    CHANNEL.get_or_init(|| {
        let (events, received) = mpsc::unbounded();
        (events, Mutex::new(Some(received)))
    })
}

/// The channel to build the system's notifier with.
pub fn events() -> Events {
    channel().0.clone()
}

/// What the backends report, as the window receives it. Nothing arrives, and nothing wakes the
/// window, until a notification is clicked.
pub fn clicks() -> iced::Subscription<NotifierEvent> {
    iced::Subscription::run(received)
}

/// The receiving end of the channel. It is handed out once: the subscription lives as long as
/// the window, so a second stream would have nothing to read and is empty for ever.
fn received() -> BoxStream<'static, NotifierEvent> {
    let taken = channel()
        .1
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take();
    match taken {
        Some(received) => received.boxed(),
        None => stream::pending().boxed(),
    }
}

/// The notifier of the system this client was built for: the session bus's notification service.
/// It reports a click on `events`.
#[cfg(target_os = "linux")]
pub fn system(events: Events) -> Box<dyn DesktopNotifier> {
    Box::new(linux::Notifier::new(events))
}

/// The notifier of the system this client was built for: the notification centre. It does not
/// report a click yet (T086): `events` is dropped.
#[cfg(target_os = "macos")]
pub fn system(_events: Events) -> Box<dyn DesktopNotifier> {
    Box::new(macos::Notifier)
}

/// The notifier of the system this client was built for: a toast. It does not report a click yet
/// (T087): `events` is dropped.
#[cfg(target_os = "windows")]
pub fn system(_events: Events) -> Box<dyn DesktopNotifier> {
    Box::new(windows::Notifier)
}
