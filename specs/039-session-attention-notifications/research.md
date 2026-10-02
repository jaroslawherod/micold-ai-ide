# Research: Notify When a Session Needs Attention, and Track Unread Sessions

**Feature**: 039 | **Date**: 2026-10-02 | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

Every decision names what was chosen, why, and what was rejected. Crate versions and platform
facts were checked on 2026-10-02 against crates.io, the crates' sources and the workspace's
`Cargo.lock`; a fact that could not be checked on this Linux host is marked **unverified** and has
a task that verifies it.

## Where the code is today (read before the decisions)

- **Three crates.** `micold-core` (render-free: model, wire protocol, stores), `micold-daemon`
  (the session service), `micold-client` (the iced GUI).
- **One window is one client process.** There is no multi-window iced application and no
  single-instance mechanism. Each window connects to the one session service and names itself with
  a `ClientInstance` (`crates/micold-core/src/protocol/messages.rs:92`). A project is attached by
  at most one window (`ClientMsg::Attach`, `RefusalReason::ProjectBusy`); switching projects
  detaches the one left (`switch_daemon_attachment`,
  `crates/micold-client/src/shell/daemon_sync.rs:198`, sends `Detach` for the old project and
  `Attach` for the new). "The window in which the project is open" (FR-012) is therefore the
  client holding the project's attachment.
- **The session service owns sessions and their activity.** `ActivitySignal { Unknown, Working,
  AwaitingInput, Ended }` (`messages.rs:987`) is computed by the pure machine in
  `crates/micold-daemon/src/activity.rs` and applied in `SharedState::note_activity`
  (`crates/micold-daemon/src/state.rs:2786`), which returns whether the signal changed. Activity is
  not persisted: it is `Unknown` after the service restarts.
- **Every window receives every session.** `SessionSummary` (`messages.rs:909`) travels in the
  `CatalogSnapshot` of `Welcome` and `CatalogChanged`, for all projects. A change of activity is
  sent as a whole snapshot: both callers of `note_activity` call `broadcast_catalog`
  (`crates/micold-daemon/src/hooks.rs:202`, `state.rs:2591`). `DaemonMsg::SessionChanged`
  (`messages.rs:669`) is declared and never sent; this feature does not use it. The client mirrors it into `Workspace::sessions`
  (`crates/micold-core/src/workspace.rs:28`) in `reconcile_catalog`
  (`crates/micold-client/src/catalog_sync.rs:66`).
- **The session service is the catalog's only writer.** `StoredSession`
  (`crates/micold-core/src/store.rs:163`) is written to `projects.json` in the service's data
  directory (`JsonFileStore::default_location`, `store.rs:610`) by `Catalog::persist`
  (`crates/micold-daemon/src/catalog.rs:1072`). New fields are added with `#[serde(default)]` and
  no `schema_version` bump (`store.rs:136`, `:171`, `:186`). In a container the service's data
  directory is `/var/lib/micold-ai-ide` (`STATE_CONTAINER_DIR`,
  `crates/micold-core/src/sandbox/mod.rs:367`), which is the host's state directory mounted in
  (`mod.rs:659`): the sandboxed service writes the same `projects.json` on the user's computer as
  the host service does, so what is stored there outlives the container.
- **Settings have two owners.** `Settings` (`crates/micold-core/src/settings.rs:110`) is one file;
  the service owns the fields `DaemonSettings` (`messages.rs:1079`) carries and pushes them to
  every window with `SettingsChanged`. `tool_server_enabled` is the pattern for a service-owned
  boolean, from the wire to the Environment page
  (`crates/micold-client/src/ui/settings/environment.rs:128`).
- **Window focus is tracked.** `Message::WindowFocusChanged` sets `App::window_focused`
  (`crates/micold-client/src/main.rs:839`). Settings fills the main area when
  `state.settings.settings_draft` is `Some` (`crates/micold-client/src/ui/mod.rs:289`).
- **UI.** A session row is a `TreeItem` with an `ActivityBadge` in its badge slot
  (`crates/micold-client/src/ui/sidebar.rs:645`). The switcher's panel is a `MenuOverlay` of
  `MenuItem`s whose `trailing_text` is `"{n} running"` (`ui/mod.rs:402`), built from
  `State::switcher_entries` (`crates/micold-client/src/app.rs:506`). The switcher's button is a
  `Button::text` in `crates/micold-client/src/ui/toolbar.rs:75`. In-app notices are
  `Notification::new(Level, text)` (`crates/micold-core/src/notify.rs:57`) shown by `Snackbar`.
- **No desktop-notification code or dependency exists.** `zbus` 5.19.0, `wayland-client` 0.31,
  `wayland-protocols` 0.32, `objc2` 0.6 and `windows` 0.62.2 are already in `Cargo.lock` through
  other crates.

## R1 — The session service keeps the unread state and counts attention events (FR-008, FR-008a, FR-016, FR-020, FR-024, FR-025)

**Decision.** The session service holds two durable facts per session, in `StoredSession` and on
the wire in `SessionSummary`:

- `attention_seq: u64` — the number of attention events the session has had: changes into
  `AwaitingInput` that happened while the session was in view in no window.
- `unread: bool`.

`note_activity` is the one place a session changes into `AwaitingInput`. When the signal before is
anything else and the signal after is `AwaitingInput`, and no connected window reports the session
in view (R2), the service adds one to `attention_seq`, sets `unread`, persists the catalog and
lets the `CatalogChanged` that follows every change of activity carry both. When a window reports the session in view,
the service clears `unread`, persists and broadcasts.

**Rationale.** This is the means the clarify round asked the plan to find for FR-008. A window
cannot observe what happens while it is closed; the service runs throughout and already sees every
change. With no window open nothing is in view, so the change sets `unread`, and the next window
to open reads it from the first snapshot: scenarios 18, 21 and 22 of story 2 need no further rule.
A session that was read at closing and did not change keeps `unread = false` (scenario 20). A
session unread at closing stays so (scenario 19, FR-008a), also across a restart of the service or
the computer, because the field is in the catalog. One holder gives every window the same state
(FR-024) with no file shared between client processes. FR-007 holds: the container uses the
connection it has today and gets no new access. FR-025 holds: the catalog is a file in the
service's data directory on the user's computer, and the connection is the local socket or
loopback port that carries the session's whole terminal already.

A missing field reads as `0` and `false`, which is "first start with this feature: nothing unread"
(FR-008a). A catalog that cannot be read is recovered by the existing path (`.bak`, empty catalog),
which leaves nothing unread; this feature adds no error of its own.

**Alternatives rejected.**

- *A client file of viewed sessions plus a service timestamp of the last change.* Each window is
  a process, so the file would have several writers and need a watcher to keep windows equal
  (FR-024). It also splits one fact over two stores.
- *The service queues missed events and replays them when a window opens.* FR-008 forbids the
  notification after the fact, so the queue would carry nothing but the unread flag.
- *`attention_seq` in memory only.* After a restart of the service the counter would restart at
  zero and could reach a value a window had already seen; persisting it costs one field in a file
  that the event writes anyway.

## R2 — Each window reports whether a session is in view (FR-002, FR-016, FR-019, spec Terms)

**Decision.** A new `ClientMsg::WindowView { focused: bool, in_view: Option<SessionId> }`. The
client derives its view with a pure function in `micold-core`:

```text
in_view = window focused
          AND no screen takes the main area over (settings_draft is None)
          AND the active project has a selected session            -> that session
```

A regular terminal tab of the selected session leaves it in view (spec Terms); a dialog over the
session does too, as it does not replace the main area. The client recomputes after every update
and sends the message only when the value differs from the last one sent, and once after each
`Welcome`. The service keeps the value per connection, drops it when the connection ends, and
keeps the order in which windows last reported `focused = true` (used by R6).

**Rationale.** "In view" is a fact about every window, and the service is the only party that
sees them all. The same report clears unread (FR-019) for each of its six causes — selecting,
a notification click, a project switch, the window regaining focus, the application opening with
the session selected, and leaving Settings — because each of them changes the derived value.
The client hides the mark of the session it has in view without waiting for the service's answer,
so the 1-second limit of FR-019 does not depend on a round trip.

**Alternatives rejected.**

- *Reuse `SetViewedSession`.* It says which session receives the terminal stream, per project,
  and stays set while the window is unfocused or shows Settings. It is not "in view".
- *The client decides alone.* A window cannot know that the session is in view in another window,
  so two windows would disagree about FR-002 and FR-016.

## R3 — A window claims an attention event; the service grants each one once (FR-001, FR-003, FR-005, FR-006, FR-006a, FR-009)

**Decision.** The client keeps, per process and in memory, the `attention_seq` and activity it last
saw for each session (`AttentionTracker` in `micold-core`). On every catalog snapshot (`Welcome`,
`CatalogChanged`):

| Situation | Rule |
|---|---|
| Session not seen before by this process (first snapshot, or a session that appears later) | Adopt its values. No claim (FR-005). |
| `attention_seq` greater than the one seen, connection unbroken | Claim it. |
| First snapshot after a lost connection | Claim only when the activity last seen was not `AwaitingInput`, it is `AwaitingInput` now, `attention_seq` is greater than the one seen, and the session is not in view in this window (FR-006). Otherwise adopt. |
| `attention_seq` lower than the one seen | Adopt (a recovered catalog). |

A claim is `ClientMsg::AttentionClaim { session, seq }`. The service grants it with
`DaemonMsg::AttentionGranted { session, seq }`, sent to the claimer only, when `seq` is greater
than the last sequence granted for that session and not greater than the session's current one.
The window that is granted raises the notification. The last granted sequence is kept in memory.

**Rationale.** Every window sees the same change, so something must pick one (FR-006a); the
service can, and a claim costs one round trip on a local connection. One mechanism covers the
change seen at once and the change found after reconnecting, where only the client knows that it
had a window open and what it last saw. `attention_seq` only moves on a real change into
`AwaitingInput` — `note_activity` reports no change for a repeated waiting signal — so FR-003
needs no rule of its own. Each session has its own sequence and its own claim, so ten changes in
one second give ten grants (FR-009, SC-005), and two quick changes of one session give two.

Which window raises the notification does not matter to the user: the click is routed by the
service (R6).

**Alternatives rejected.**

- *The service pushes the event to one chosen window.* It cannot tell a window that lost its
  connection from no window at all, so FR-006 would still need the claim.
- *Each window decides by "I hold the project".* Sessions of a project open in no window would
  never be notified.

**Known limit.** A window whose connection has stalled is still counted as viewing its session
until the service drops the connection, at most `LIVENESS_DEADLINE` (9 s,
`crates/micold-core/src/protocol/keepalive.rs:22`) later. If the user selects another session in
that window during those seconds and the first session changes to awaiting input in the same
interval, that change raises no notification and no mark. A closed or crashed window drops its
connection at once and is not affected.

## R4 — Showing the notification: one trait, the system's own facility per OS (FR-004, FR-010, FR-015, FR-029)

**Decision.** `shell/desktop_notify/` in the client holds a trait and three `cfg` modules; nothing
outside it names an operating system (Principle VI).

| OS | Crate | Why |
|---|---|---|
| Linux | `zbus` 5.19 (already in `Cargo.lock`, `async-io` as today), calling `org.freedesktop.Notifications.Notify` with the `default` action and the `desktop-entry` hint `micold-ai-ide` | One connection and one signal stream for `ActionInvoked`, `ActivationToken` and `NotificationClosed`. |
| macOS | `mac-usernotifications` 0.3.1 (`MIT OR Apache-2.0`, pure `objc2` 0.6, no build script) | `UNUserNotificationCenter`: banners are shown and clicks reported to the running application. The bundle is `io.github.cumulocity-iot.micold-ai-ide` and `scripts/macos-bundle.sh` ad-hoc signs it, which is what the API needs. |
| Windows | `tauri-winrt-notification` 0.8.1 (`MIT OR Apache-2.0`, on `windows` 0.62 already in `Cargo.lock`) | `Toast::on_activated` reports a body click in-process, with no COM activator. |

The text (FR-004) is built by a pure function in `micold-core` from the three names the sidebar
shows: title `<session label> is waiting for input`, body `<project> — <worktree>`. The session
label is `Session::label.display()` (the row's own label, placeholders included); the worktree
name is `worktree_display_name` or the sidebar's `Default`. Nothing else enters the text.

A failure to show — no notification service on the bus, permission refused, no bundle identifier,
no registered application identity — is returned as an error by the backend. The feature logs it
with `tracing::warn!` the first time in a run and stays silent after that; no in-app notice is
shown and nothing else depends on the result (FR-010).

Registration each system needs:

- **Linux**: nothing; the `.desktop` file that packaging installs gives the name and icon.
- **macOS**: the first notification asks the system for authorisation; the prompt is the system's
  own. An unbundled binary (`cargo run`, `mise run app` staged unsigned) gets an error and shows
  nothing.
- **Windows**: a toast needs an Application User Model ID that the system knows. The installer's
  Start-menu shortcut gets `AppUserModelID: "MicoldAiIde.Client"` in
  `packaging/windows/micold-ai-ide.iss`, and the client passes the same string to `Toast::new`.
  Microsoft's quickstart for desktop toasts says a toast with an ID no shortcut carries "will not
  be displayed"; whether `show` then returns an error is not documented, so the client does not
  rely on one. Checked in the crate's source (`src/lib.rs`, 0.8.1): `on_activated` takes
  `Fn(Option<String>) -> Result<()> + Send + 'static`, called with `None` for a click on the body;
  the handler is called on a thread of the system's, not the window's, so the backend only sends
  a `NotifierEvent` over its channel; `show(&self)` attaches the handler and drops the
  `ToastNotification` when it returns. **Unverified**: whether a body click reaches
  `on_activated` once the banner has gone and the toast sits in the notification centre.
  Microsoft documents the `Activated` event for "apps that are running" and says nothing about
  the notification centre, and no primary source was found (searched 2026-10-02); the system may
  drop such a toast, or start the shortcut's target instead, which is an ordinary start
  (FR-015a). A task of story 3 checks it on an installed build (quickstart §C2) and records the
  result in the user guide. FR-015 holds either way because the notification is shown.

**Alternatives rejected.**

- *`notify-rust` 4.18.1 on all three.* It reports clicks on all three since 4.18.0, but it does not
  expose the `ActivationToken` signal that R7 needs on Wayland, its wait calls block one thread
  per notification, and on macOS it always compiles `mac-notification-sys`, whose `build.rs` builds
  Objective-C with `cc` and whose `NSUserNotification` backend draws no banner while the sending
  application is active.
- *`objc2-user-notifications` directly.* Raw bindings: the delegate `mac-usernotifications`
  already provides would be written and maintained here.
- *`winrt-toast-reborn`, `win-toast-notify`.* Less maintained than `tauri-winrt-notification`.
- *`user-notify`.* `LGPL-3.0-or-later`; the workspace is `Apache-2.0`.
- *The session service shows the notification.* In a container it has no desktop (FR-007).

## R5 — Counting unread sessions (FR-021, FR-022, FR-023)

**Decision.** Two pure functions on `Workspace` beside `running_session_count`
(`crates/micold-core/src/workspace.rs:319`): `unread_session_count(&project, in_view)` counts
the project's sessions with `unread`, less the session the window has in view, and
`other_projects_unread(&active)` sums it over every project but the active one. Both read `Workspace::sessions`, which holds every session of every known
project, whatever the sidebar's tag filter hides and including the Default entry (FR-022).
`SwitcherEntry` gains `unread_count`. The count of the session in view is taken as read at once
(R2).

**Rationale.** The counts are derived, never stored, so they cannot disagree with the marks.

**Alternatives rejected.** *The service sends counts.* The client has the sessions; a second
source could only drift.

## R6 — A click is routed by the session service to the right window (FR-011 to FR-015a)

**Decision.** The window that raises a notification keeps its project path and session id against
the system's id for it. On activation it sends `ClientMsg::SessionReveal { project, session,
activation }`, where `activation` is the Wayland token of the click when there is one. The service picks the
target — the window attached to `project`; when none is, the window that most recently reported
focus (R2); when none did, the sender — and sends it `DaemonMsg::RevealSession { project,
session, activation }`. The target brings its window to the front (R7) and resolves the request with a pure
function in `micold-core`:

- the project is known and available and the session exists → make the project active through the
  existing switch (`ProjectMsg::Reopened`) and select the session (`SessionMsg::Selected`);
- otherwise → leave the selection as it is and push the in-app notice `That session is no longer
  available.` at `Level::Info` (FR-013).

Only those two existing messages are sent, so nothing is stopped, interrupted, restarted or typed
into (FR-014), and the session comes into view by the ordinary path, which clears its mark
(FR-019, story 3 scenario 3). The request is handled only by a running window. A notification
clicked after the application has quit either does nothing or, where the system starts the
application, starts it with no argument, which is an ordinary start (FR-015a).

**Rationale.** A window is a process; FR-012 names a window that may not be the one that raised
the notification. The service already knows who holds each project.

**Known limit.** A click is reported by the system to the process that raised the notification:
the D-Bus signals name an id only that process can resolve, and the macOS delegate and the Windows
handler live in it. When that window has been closed and another is still open, the click reaches
no window: nothing changes, and the session keeps its unread mark, which is how the user finds it.
This is the case FR-015 excepts — the notification service cannot report the click to the
application — and the spec's Edge Cases name it. Any open window can raise a notification, so the
limit applies only to notifications older than the window that raised them.

**Alternatives rejected.** *The raising window handles the click itself.* It would have to take
the project from the window that holds it, which is the takeover FR-012 rules out. *The service
keeps every notification's system id, and every window listens for clicks on ids it did not
raise.* One more message and a table in the service, for Linux notification services that send
the click to every listener only; macOS and Windows would gain nothing.

## R7 — Bringing the window to the front (FR-011)

**Decision.** `shell/window_raise.rs` issues `iced::window::minimize(id, false)` and then
`iced::window::gain_focus(id)` for the window from `iced::window::latest()`. Which steps are
issued is decided by the pure, tested `raise_plan` in `features/attention.rs`; `window_raise.rs`
only carries them out (Principle I). That is
`_NET_ACTIVE_WINDOW` on X11, `activateIgnoringOtherApps` on macOS and winit's foreground switch on
Windows. On Wayland winit's `focus_window` does nothing, so the Linux arm also activates the
surface with the `xdg_activation_v1` protocol, using the token the notification service sent in
its `ActivationToken` signal — carried to the window that is raised in `RevealSession`, as that
may be another process — and the `wl_display` and `wl_surface` handles from
`iced::window::run`, through `wayland-client` and `wayland-protocols` (both in `Cargo.lock`). When
there is no token, no such protocol, or the compositor declines, it falls back to
`iced::window::request_user_attention`, and the session is still selected.

**Rationale.** The spec asks for keyboard focus wherever the notification service reports the
click. The token is the one sanctioned way on Wayland.

**Checked in the sources (2026-10-02).** winit 0.30.13 leaves `focus_window` empty on Wayland
(`src/platform_impl/linux/wayland/window/mod.rs:629`) and takes an activation token only when a
window is created (`:176`). It selects `wayland-backend` with `client_system` and
`wayland-protocols` with `staging` (its `Cargo.toml`), so `Backend::from_foreign_display`
(`wayland-backend` 0.3.17, `src/sys/client_impl/mod.rs:271`) and the `xdg_activation_v1` bindings
are compiled in already. A second connection over the application's own `wl_display` is what
`smithay-clipboard` 0.7.3 does today in this application (`src/lib.rs:35`, through iced's
clipboard). `iced::window::run` hands the closure a `&dyn Window` with the display and window
handles (`iced_runtime` 0.14.0, `src/window.rs:463`). The client must name `wayland-backend` with
`client_system` and `wayland-protocols` with `client` and `staging` itself.

**Risk.** **Unverified**: whether a compositor gives keyboard focus for the token a notification
service hands out; that is the compositor's policy and differs between them. The Wayland arm's
first task is a probe on a Wayland session (the development host runs one). If the probe fails,
the fallback is what ships on Wayland and the limit is written into the user guide and recorded in
the ledger as a follow-up. X11, macOS and Windows do not depend on it.

**Alternatives rejected.** *A second window, or restarting the window with a token.* Destroys the
user's state. *Only `request_user_attention` everywhere.* Does not meet FR-011 where focus is
possible.

## R8 — The switch is a service-owned setting (FR-026, FR-027, FR-028)

**Decision.** `Settings::desktop_notifications: bool`, default `true`, service-owned: it joins
`DaemonSettings` and `ClientMsg::SettingsSet` exactly as `tool_server_enabled` does, and is shown
on the Environment page as **Desktop notifications** with the `Checkbox` and `field_note` its
neighbours use. While it is off the service grants no claim (R3).

**Rationale.** A window that still believed the setting on would claim and notify, so the value
must be one value for all windows at once; `SettingsChanged` already does that. Refusing at the
grant makes "off" hold for sessions already running and for the next change, with no restart, and
a change that happened while it was off is never granted later: while the switch is off the
service records each attention event as granted (`Views::note_event`), which also covers a window
that was disconnected at the time and claims after reconnecting (FR-027). There is one field and no per-CLI field (FR-028). An unreadable
settings file yields the defaults, so the switch is on (spec Edge Cases).

**Alternatives rejected.** *A client-owned field.* Each window would read it on its own schedule.

## R9 — The unread mark and count are one shared component (FR-018, FR-021, FR-023, FR-030, FR-032)

**Decision.** A new `ui/material/unread_mark.rs`: `UnreadMark::new(roles)`, a filled 8dp circle in
the `primary` role, with `.count(n)` to put the number after it and `.worded(true)` to add the
word `unread`. It is used in three places and nowhere restyled:

| Place | Use | How it differs from what is there |
|---|---|---|
| Session row | `TreeItem::unread(true)`: the mark at the row's trailing edge, and the label in the emphasised weight | The activity indicator stays in the badge slot at the leading edge; the mark differs by position and by the label's weight, not by colour alone (FR-018). |
| Switcher row | `MenuItem::trailing_mark`: `● 2 unread` after the `3 running` text | The word tells it from the running count (FR-021). |
| Switcher button | `Button::trailing_mark`: `● 3` after the project's name, tooltip `3 unread sessions in other projects` | The symbol tells it from the name (FR-023). |

The component showcase gets three entries (FR-030). The activity indicator, the running count and
the notices are not edited (FR-032); the contrast and anatomy gates cover the new component in
both schemes.

**Alternatives rejected.** *Recolouring the activity indicator.* Colour alone, and it changes an
existing indicator (FR-018, FR-032). *A text suffix built at each call site.* Three private copies
(Principle VIII).

## R10 — One wire change per milestone

**Decision.** Each milestone that changes the wire bumps `PROTOCOL_VERSION` once, in one edit, as
`crates/micold-core/src/protocol/version.rs` requires: 21 for the view report and the sequence; 22
for the claim and the grant; 23 for `unread`; 24 for the reveal pair; 25 for the activation token
on the reveal pair; 26 for the setting. Each milestone ships only the wire it uses. No `#[serde(default)]` on wire
types: peers that differ are refused at the handshake, as today.

**Alternatives rejected.** *One bump for the whole feature in the first milestone.* It would ship
`unread`, the reveal pair and the setting on `main` with nothing using them. *The activation token
with the reveal pair.* The token is used only by the Wayland path, which starts with a probe (R7);
if the probe fails the field would stay on the wire unused.
