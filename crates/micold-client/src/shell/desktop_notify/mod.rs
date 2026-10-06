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

use std::path::Path;
use std::sync::{Mutex, OnceLock};

use iced::futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use iced::futures::stream::{self, BoxStream, StreamExt};
use micold_client::features::attention::{DesktopNotifier, NotifierEvent};
use micold_client::notification_icon::{write_files, IconFiles};

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

/// The kind icons as files, written once per run under the data directory (feature 613, I6). A
/// failure is logged here, once, and leaves no paths: the notifications are shown without icons.
/// Linux passes the pixels in the request instead and never asks for these.
#[cfg(any(target_os = "macos", target_os = "windows"))]
fn icon_files() -> &'static IconFiles {
    static FILES: OnceLock<IconFiles> = OnceLock::new();
    FILES.get_or_init(|| {
        // The data directory as `shell/startup.rs` resolves it.
        let dir = directories::ProjectDirs::from("", "", "micold-ai-ide")
            .map(|dirs| dirs.data_dir().to_path_buf());
        icon_files_in(dir.as_deref())
    })
}

/// The kind icons written under `data_dir`; none, logged, when there is no such directory or it
/// cannot be written.
#[cfg_attr(target_os = "linux", allow(dead_code))]
fn icon_files_in(data_dir: Option<&Path>) -> IconFiles {
    let Some(dir) = data_dir else {
        eprintln!(
            "micold-ai-ide: no data directory for the notification icons; \
             notifications are shown without them"
        );
        return IconFiles::default();
    };
    write_files(dir).unwrap_or_else(|error| {
        eprintln!(
            "micold-ai-ide: the notification icons could not be written under {}: {error}; \
             notifications are shown without them",
            dir.display()
        );
        IconFiles::default()
    })
}

/// The notifier of the system this client was built for: the session bus's notification service.
/// It reports a click on `events`.
#[cfg(target_os = "linux")]
pub fn system(events: Events) -> Box<dyn DesktopNotifier> {
    Box::new(linux::Notifier::new(events))
}

/// The notifier of the system this client was built for: the notification centre. It reports a
/// click on `events`.
#[cfg(target_os = "macos")]
pub fn system(events: Events) -> Box<dyn DesktopNotifier> {
    Box::new(macos::Notifier::new(events, icon_files()))
}

/// The notifier of the system this client was built for: a toast. It reports a click on `events`.
#[cfg(target_os = "windows")]
pub fn system(events: Events) -> Box<dyn DesktopNotifier> {
    Box::new(windows::Notifier::new(events, icon_files()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use micold_core::attention::NotificationKind;

    #[test]
    fn a_writable_data_directory_gives_the_four_icon_files() {
        // Feature 613, I6.
        let dir = tempfile::tempdir().expect("temp dir");
        let files = icon_files_in(Some(dir.path()));
        for kind in NotificationKind::ALL {
            assert!(files.path(kind).is_some_and(Path::is_file), "{kind:?}");
        }
    }

    #[test]
    fn a_data_directory_that_cannot_be_written_gives_no_icon_files() {
        // I6 (FR-016): no paths, so the backends show the notification without an icon.
        let dir = tempfile::tempdir().expect("temp dir");
        let blocked = dir.path().join("blocked");
        std::fs::write(&blocked, b"").expect("a file where the directory should be");
        assert_eq!(icon_files_in(Some(&blocked)), IconFiles::default());
    }

    #[test]
    fn no_data_directory_gives_no_icon_files() {
        assert_eq!(icon_files_in(None), IconFiles::default());
    }
}
