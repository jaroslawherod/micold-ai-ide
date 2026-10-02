# Contract: desktop notification

**Feature**: 039 | Code: `crates/micold-client/src/shell/desktop_notify/` *(new)*,
`crates/micold-client/src/shell/window_raise.rs` *(new)*, `micold_core::attention` *(new)*.

## The seam

```rust
pub struct DesktopNotification {
    pub title: String,
    pub body: String,
    pub project: PathBuf,
    pub session: SessionId,
}

pub trait DesktopNotifier: Send + Sync {
    /// Show one notification. An error means the system did not accept it.
    fn show(&self, notification: DesktopNotification) -> Result<(), NotifyError>;
}

/// What a backend reports back, over the channel it was built with.
pub enum NotifierEvent {
    /// The notification for this session was clicked. `activation` is the Wayland token, when
    /// the notification service sent one.
    Activated { project: PathBuf, session: SessionId, activation: Option<String> },
}
```

`desktop_notify::system(events) -> Box<dyn DesktopNotifier>` returns the backend of the operating
system the client was built for. Tests use a recording implementation.

| # | Rule | Requirement |
|---|---|---|
| N1 | A notification is shown only for an `AttentionGranted` this window received. One grant, one `show`. | FR-001, FR-006a, FR-009 |
| N2 | `title` and `body` come from `notification_text` and from the labels the sidebar shows at that moment; no other text is passed to the system. | FR-004 |
| N3 | The backend never withdraws, replaces, groups or rate-limits a notification. | spec Edge Cases, Out of Scope |
| N4 | An `Err` from `show` is logged once per run at `warn` and otherwise ignored: no in-app notice, no retry, no effect on sessions, the activity indicator or unread state. | FR-010 |
| N5 | On `RevealSession` the window is raised (N6), then `resolve_reveal` decides: `Show` sends `ProjectMsg::Reopened` when the project is not active, then `SessionMsg::Selected`; `Unavailable` pushes the notice `That session is no longer available.` at `Level::Info` and changes no selection. | FR-011, FR-013, FR-014 |
| N6 | Raising: un-minimise, then request focus; on Wayland, activate with the token of `Activated`; on failure ask for the user's attention. | FR-011 |
| N7 | A backend that cannot learn of a click still shows the notification. | FR-015 |
| N8 | No backend registers the application to be started by a click, and the client accepts no argument that names a session. | FR-015a |

## Backends

| OS | Module | Shows with | Reports a click by | Registration |
|---|---|---|---|---|
| Linux | `linux.rs` | `zbus`: `org.freedesktop.Notifications.Notify`, action `default`, hint `desktop-entry = micold-ai-ide` | signals `ActivationToken` then `ActionInvoked("default")`, matched by notification id | none |
| macOS | `macos.rs` | `mac-usernotifications`: `UNUserNotificationCenter` | the crate's delegate response | bundle identifier; authorisation asked on first use |
| Windows | `windows.rs` | `tauri-winrt-notification`: `Toast::new("MicoldAiIde.Client")` | `on_activated(None)` | `AppUserModelID` on the Start-menu shortcut (`packaging/windows/micold-ai-ide.iss`) |

The pure part of each backend — mapping a signal or callback to `NotifierEvent`, and the id table
that maps a system notification id to `(project, session)` — is a function tested on every OS. The
call into the system is glue with no branch of its own.
