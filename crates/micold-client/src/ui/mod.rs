//! iced rendering layer for the main window. Bin-only; compiled with the `gui` feature.

mod about;
pub mod cdk;
mod confirm_delete;
mod confirm_forget;
mod confirm_session_remove;
mod material;
pub use material::target_offset_delta;
mod project_selector;
mod rename;
mod settings_form;
mod shell;
mod sidebar;
// The application's theme — the only thing the styling layer exposes beyond the component
// library (FR-002). Defined in `material`, not here: a feature module naming the styling layer is
// exactly what the boundary test forbids, and `ui/mod.rs` is a feature module.
pub use material::theme;
pub mod terminal;
mod toolbar;
mod worktree_form;
mod worktree_rename;

use crate::app::{Message, Overlay, State};
use crate::icons::Icon;
use iced::widget::{column, container, row, Space};
use iced::{Element, Length, Subscription};
use micold_core::session::SessionId;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, spacing, Roles};

// The icon font and the two primitives that draw a glyph moved into the component library with
// everything else that decides an appearance (FR-001). Re-exported here for `main`, which
// registers the font at startup, and for the tests that assert what the font file advertises.
pub use material::glyph::{icon, icon_colored, MATERIAL_SYMBOLS, MATERIAL_SYMBOLS_BYTES};

/// The daemon-connection state, as it concerns the *active* project (US5, FR-024/027). Computed by
/// the binary (the connection is binary-owned runtime state) and passed to [`view`] so the shell can
/// show a persistent status banner. `Connected` renders nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// The service is reachable and this window holds the active project.
    Connected,
    /// The service connection is down; displayed content may be stale and is auto-reconnecting.
    Disconnected,
    /// Another window took over the active project (`by` = its build/identity). This window is
    /// read-only until the user takes it back (FR-024).
    Displaced {
        /// The taking-over window's identity string.
        by: String,
    },
    /// The running service speaks a different contract version (US6, FR-021/022). Names both versions
    /// and offers a one-click restart of the service.
    VersionMismatch {
        /// This client's protocol version.
        client: u32,
        /// The running daemon's protocol version.
        daemon: u32,
        /// The running daemon's build string.
        daemon_build: String,
    },
    /// The running service is a same-contract different build — most releases don't touch the wire
    /// protocol, so this is the common shape a `.deb` upgrade takes (US6, FR-022a, BUG-002). Offers
    /// the same one-click restart as [`ConnectionStatus::VersionMismatch`], but without implying any
    /// risk to live sessions: the contract still matches, so nothing is actually incompatible.
    BuildMismatch {
        /// This client's build string.
        client_build: String,
        /// The running daemon's build string.
        daemon_build: String,
    },
}

/// The persistent connection-status strip, shown between the toolbar and the notification stack.
/// Empty (zero-height) when connected, so it never crowds a healthy session.
fn connection_banner<'a>(status: &ConnectionStatus, roles: Roles) -> Element<'a, Message> {
    let banner = match status {
        ConnectionStatus::Connected => {
            return Space::new()
                .width(Length::Fixed(0.0))
                .height(Length::Fixed(0.0))
                .into()
        }
        ConnectionStatus::Disconnected => material::ConnectionBanner::new(
            "Not connected to the session service",
            "The displayed content may be stale. Reconnecting…",
            roles,
        ),
        ConnectionStatus::Displaced { by } => material::ConnectionBanner::new(
            "Another window took over this project",
            format!("{by} is now attached — this window is read-only until you take it back."),
            roles,
        )
        .action("Take over", Message::ConnectionTakeoverRequested),
        ConnectionStatus::VersionMismatch {
            client,
            daemon,
            daemon_build,
        } => material::ConnectionBanner::new(
            "The session service is a different version",
            format!(
                "This app speaks contract v{client}; the running service ({daemon_build}) speaks \
                 v{daemon}. Restart the service to match — running processes stop, but your \
                 sessions are preserved and resumable."
            ),
            roles,
        )
        .action(
            "Restart service",
            Message::ConnectionRestartServiceRequested,
        ),
        ConnectionStatus::BuildMismatch {
            client_build,
            daemon_build,
        } => material::ConnectionBanner::new(
            "A newer session service is installed",
            format!(
                "This app is {client_build}; the running service is still {daemon_build}. \
                 Restart the service to pick up the update — your sessions are unaffected either \
                 way and remain resumable."
            ),
            roles,
        )
        .action(
            "Restart service",
            Message::ConnectionRestartServiceRequested,
        ),
    };
    container(banner)
        .padding([spacing::SM, spacing::MD])
        .width(Length::Fill)
        .into()
}

/// The stack of dismissible global notification banners, newest last. Empty when there is
/// nothing to report, in which case it occupies no space.
fn notifications<'a>(state: &'a State, r: Roles) -> Element<'a, Message> {
    let mut stack = column![].spacing(spacing::SM);
    for (index, notification) in state.notifications.iter().enumerate() {
        let banner = row![
            material::Text::new(notification.message.clone(), material::TypeRole::Body, r)
                .width(Length::Fill),
            material::Button::with_content(
                material::Text::new("Dismiss", material::TypeRole::Label, r),
                material::ButtonVariant::Outlined,
                r
            )
            .on_press(Message::NotificationDismissed(index)),
        ]
        .spacing(spacing::SM)
        .align_y(iced::Alignment::Center);
        stack = stack.push(
            material::Surface::new(
                banner,
                material::SurfaceKind::Notification(notification.level),
                r,
            )
            .padding(spacing::MD)
            .width(Length::Fill),
        );
    }
    if state.notifications.is_empty() {
        stack.into()
    } else {
        container(stack)
            .padding([spacing::SM, spacing::MD])
            .width(Length::Fill)
            .into()
    }
}

/// Render the main window: the top app bar over the shell body (active project / empty state),
/// with any floating surface stacked on top. Every surface is styled from the active color
/// scheme's design tokens.
///
/// No animation state is passed in: every transition belongs to the component that plays it, and
/// each owns its progress in the widget tree (FR-011, FR-014). `dismissing` is the snapshot of a
/// just-closed overlay still animating out — the one thing an application still has to hold, since
/// it outlives the state that opened it (rendered instead of a live overlay when `state.overlay` is
/// already `None` — see [`crate::app::ClosingOverlay`]).
#[allow(clippy::too_many_arguments)]
pub fn view<'a>(
    state: &'a State,
    grid: Option<&'a crate::grid::GridCache>,
    selection: Option<&'a crate::selection::Selection>,
    display_offset: usize,
    dismissing: Option<&'a crate::app::ClosingOverlay>,
    env_include_outcome: &'a micold_core::env_include::EnvIncludeOutcome,
    connection: &ConnectionStatus,
) -> Element<'a, Message> {
    let scheme = state.color_scheme();
    let roles = tokens::roles(scheme);
    let bg = roles.background;

    // With a project open, show the worktree sidebar beside the main area; the main area is
    // the embedded terminal when a session is active (FR-012), else the project surface. The
    // sidebar slides in/out and is resizable; the main content fades when it changes.
    let body: Element<'a, Message> = if state.workspace.active_project().is_some() {
        let main_inner: Element<'a, Message> = if state.active_session.is_some() {
            terminal::pane(state, grid, selection, display_offset, scheme)
        } else {
            shell::view(state, scheme)
        };
        let main = material::ViewFade::new(main_inner, bg).showing(main_content_key(state));
        // The drawer owns both the panel and the rail that replaces it, so it decides which of them
        // is on screen — the last thing the binary was reading a progress value to work out. All
        // that is left to say here is whether the sidebar is open.
        let left: Element<'a, Message> = material::NavigationDrawer::new(
            sidebar::view(state, scheme),
            sidebar::collapsed_strip(scheme),
        )
        .open(!state.sidebar_hidden)
        .handle(
            material::ResizeHandle::new(roles)
                .on_resize(|x| Message::SidebarDragMoved(x.max(0.0) as u16)),
        )
        .into();
        row![left, main]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        material::ViewFade::new(shell::view(state, scheme), bg)
            .showing(main_content_key(state))
            .into()
    };

    // The global notification surface, rendered here — between the toolbar and the body, above
    // every branch that decides what the body is. Deliberately unconditional: the failures this
    // replaces were all cases where state was set correctly but the only render site sat inside
    // a branch that could not be taken. Nothing may nest this inside an `if`.
    let base: Element<'a, Message> = material::Surface::new(
        column![
            toolbar::view(state, scheme),
            connection_banner(connection, roles),
            notifications(state, roles),
            body
        ],
        material::SurfaceKind::Window,
        roles,
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into();

    // Every floating surface from here down is *pushed onto one overlay* rather than wrapped
    // around the previous one (FR-008). The order they are pushed in is not the order they are
    // drawn in: the overlay sorts by each surface's own layer, so a dialog is above a menu because
    // it is a dialog, not because this function happens to build it last (FR-010).

    // The toolbar's overflow menu (no toolbar reflow), fading in/out. Pushed whether or not it is
    // open: the panel owns its own fade, so it has to outlive the flag that opened it or there
    // would be nothing left on screen to fade out. Closed, it is inert and blocks nothing.
    let overflow_menu: cdk::overlay::Surface<'a, Message> = material::MenuOverlay::new(
        toolbar::overflow_items(state),
        Message::HelpMenuToggled,
        roles,
    )
    .open(state.help_menu_open)
    .into();

    // The project switcher panel. Rows are built purely from the workspace: active marker,
    // running-background-session count, and unavailable badge. Mutually exclusive with the
    // overflow menu (handled in the reducer).
    let switcher_rows: Vec<material::ProjectRow<Message>> = state
        .switcher_entries()
        .into_iter()
        .map(|e| material::ProjectRow {
            label: e.label,
            is_active: e.is_active,
            running_count: e.running_count,
            available: e.available,
            on_select: Message::KnownProjectReopened(e.path.clone()),
            // Right-click a project row to reach its "Forget project" menu (feature 015). The
            // trailing "Add project…" row is added by the component itself and carries none.
            on_context: Some(Message::ProjectMenuToggled(e.path)),
        })
        .collect();
    let switcher: Option<cdk::overlay::Surface<'a, Message>> =
        material::ProjectSwitcherOverlay::new(
            switcher_rows,
            Message::ProjectSelectorOpened,
            Message::ProjectSwitcherToggled,
            roles,
        )
        .open(state.project_switcher_open)
        .into();

    // The right-clicked project's context menu, at the cursor (feature 015), like a normal desktop
    // context menu: the panel's top-left corner sits at the click point. The anchor is clamped at
    // render time (not when the menu opened) so a window resize while it is showing can never leave
    // the panel hanging off the edge. The switcher stays open behind it — which is now a property
    // of the context-menu layer rather than of the order these are built in.
    let project_menu: Option<cdk::overlay::Surface<'a, Message>> =
        state.project_menu_open.as_ref().map(|menu| {
            let (x, y) = crate::app::clamp_menu_anchor(
                menu.anchor,
                material::menu_panel_size(1),
                state.window_size,
            );
            material::MenuOverlay::new(
                vec![material::MenuItem::new(
                    Icon::Delete,
                    "Forget project",
                    Message::ProjectForgetRequested(menu.path.clone()),
                )],
                Message::ProjectMenuDismissed,
                roles,
            )
            .anchor(iced::Point::new(x as f32, y as f32))
            .into()
        });

    // The worktree right-click context menu, anchored near the sidebar (feature 008, FR-013).
    // Only present while a worktree's menu is open.
    let worktree_menu: Option<cdk::overlay::Surface<'a, Message>> =
        state.worktree_menu_open.as_ref().map(|dir| {
            material::MenuOverlay::new(
                worktree_menu_items(dir, &state.worktree_display_name(dir)),
                Message::WorktreeMenuDismissed,
                roles,
            )
            .anchor(iced::Point::new(24.0, 96.0))
            .into()
        });

    // The session right-click context menu (bugfix BUG-003). Only present while a session's menu
    // is open.
    let session_menu: Option<cdk::overlay::Surface<'a, Message>> =
        state.session_menu_open.map(|id| {
            material::MenuOverlay::new(session_menu_items(id), Message::SessionMenuDismissed, roles)
                .anchor(iced::Point::new(24.0, 96.0))
                .into()
        });

    // The dialog body for whatever is open — or, if one has just closed, the snapshot it left
    // behind (captured before the core cleared its live state) so the exit has something to draw
    // (FR-002). Each of these builds only the dialog; the transition around it belongs to `Modal`,
    // which owns the three tracks that carry it in and out.
    let dialog: Option<Element<'a, Message>> = match state.overlay {
        Overlay::None => {
            dismissing.map(|closing| dismissing_dialog(closing, scheme, env_include_outcome))
        }
        Overlay::About => Some(about::modal(scheme)),
        // Overlay flagged but no live state — render nothing rather than an empty dialog.
        Overlay::ProjectSelector => state
            .selector
            .as_ref()
            .map(|selector| project_selector::modal(selector, scheme)),
        Overlay::RenameProject => state
            .rename_draft
            .as_ref()
            .map(|draft| rename::modal(draft, scheme)),
        Overlay::AddWorktree => state
            .worktree_form
            .as_ref()
            .map(|form| worktree_form::modal(form, state.worktree_error.as_deref(), scheme)),
        Overlay::Settings => state
            .settings_draft
            .as_ref()
            .map(|draft| settings_form::modal(draft, scheme, env_include_outcome)),
        Overlay::ConfirmWorktreeDelete => state.worktree_delete_target.as_ref().map(|dir| {
            let branch = state
                .worktrees
                .iter()
                .find(|w| &w.dir_name == dir)
                .and_then(|w| w.branch.as_deref());
            confirm_delete::modal(
                dir,
                &state.worktree_display_name(dir),
                branch,
                state.worktree_delete_keep_branch,
                scheme,
            )
        }),
        Overlay::RenameWorktree => state
            .worktree_rename_draft
            .as_ref()
            .map(|draft| worktree_rename::modal(draft, scheme)),
        Overlay::ConfirmSessionRemove => state
            .session_remove_target
            .and_then(|id| state.workspace.find_session(id))
            .map(|(_, session)| confirm_session_remove::modal(session.label.display(), scheme)),
        Overlay::ConfirmForgetProject => state.forget_target.as_ref().map(|path| {
            // The display name and running-session count are read from the catalog/sessions at
            // render time; the count (FR-002a) is exactly the set the binary will stop.
            let display_name = state
                .workspace
                .projects
                .iter()
                .find(|p| &p.path == path)
                .map(|p| p.display_name.clone())
                .unwrap_or_else(|| micold_core::project::default_display_name(path));
            let running = state.workspace.running_session_count(path);
            confirm_forget::modal(&display_name, running, scheme)
        }),
    };

    let modal: Option<cdk::overlay::Surface<'a, Message>> = dialog.map(|dialog| {
        // A snapshot is a dialog on its way out: nothing is open behind it, so it is not shown and
        // has nothing left to cancel.
        let mut modal = material::Modal::new(dialog, roles)
            .shown(state.overlay != Overlay::None)
            // The identity of the dialog being rendered, which for a snapshot is the one it was
            // taken of — not `Overlay::None`, which is merely where the application has got to.
            .restart_on(overlay_key(match dismissing {
                Some(closing) if state.overlay == Overlay::None => closing.overlay(),
                _ => state.overlay,
            }))
            // Once the exit finishes, the snapshot has served its purpose and is released. This is
            // the one thing about a transition the application still needs to hear about — and the
            // component says it, rather than the application watching a progress value for it.
            .on_hidden(Message::OverlayTransitionFinished);
        // Clicking the scrim closes a dialog exactly the way Escape does, so the two cannot
        // disagree: both ask `on_escape` what this dialog's cancellation is (FR-009, the unified
        // dismissal sanctioned by FR-024).
        if let Some(cancel) = crate::app::on_escape(state) {
            modal = modal.on_dismiss(cancel);
        }
        modal.into()
    });

    cdk::overlay::Overlay::new(base)
        .push(overflow_menu)
        .push_maybe(switcher)
        .push_maybe(project_menu)
        .push_maybe(worktree_menu)
        .push_maybe(session_menu)
        .push_maybe(modal)
        .into()
}

/// The items in a worktree's right-click context menu (feature 008, FR-013; "Copy name" added
/// for cross-application clipboard access to labels the app doesn't render as selectable text).
fn worktree_menu_items(dir: &str, display_name: &str) -> Vec<material::MenuItem<Message>> {
    vec![
        material::MenuItem::new(
            Icon::Copy,
            "Copy name",
            Message::TextCopyRequested(display_name.to_string()),
        ),
        material::MenuItem::new(
            Icon::Rename,
            "Rename",
            Message::WorktreeRenameStarted(dir.to_string()),
        ),
        material::MenuItem::new(
            Icon::Unavailable,
            "Delete",
            Message::WorktreeDeleteRequested(dir.to_string()),
        ),
    ]
}

/// The items in a session's right-click context menu (bugfix BUG-003): "Close" archives (kept,
/// hidden, never resurrected by reconciliation — FR-015a/FR-020c); "Remove" permanently deletes,
/// behind a confirm dialog (FR-015c).
fn session_menu_items(id: SessionId) -> Vec<material::MenuItem<Message>> {
    vec![
        material::MenuItem::new(Icon::Close, "Close", Message::SessionCloseRequested(id)),
        material::MenuItem::new(
            Icon::Unavailable,
            "Remove",
            Message::SessionRemoveRequested(id),
        ),
    ]
}

/// The identity of the main content area, so the view fades when what it shows changes rather than
/// on every re-render — the same question the old `MotionKey::Main` reset answered, now asked of
/// the component that owns the track.
fn main_content_key(state: &State) -> u64 {
    match (state.workspace.active_project(), state.active_session) {
        (None, _) => 0,
        (Some(_), None) => 1,
        // Offset past the two fixed states; the id itself distinguishes one session from another.
        (Some(_), Some(id)) => 2 ^ id.0.as_u64_pair().0,
    }
}

/// Which dialog is open, so that switching straight from one to another replays the entrance
/// instead of inheriting a transition the previous dialog had already finished.
fn overlay_key(overlay: Overlay) -> u64 {
    overlay as u64
}

/// Render the snapshot of a just-closed overlay so it can keep fading out after the pure core
/// has already cleared its live state (see [`crate::app::ClosingOverlay`]). Delegates to the same
/// per-overlay `modal` render functions as the live path, so the exit is the enter in reverse.
fn dismissing_dialog<'a>(
    closing: &'a crate::app::ClosingOverlay,
    scheme: ColorScheme,
    env_include_outcome: &'a micold_core::env_include::EnvIncludeOutcome,
) -> Element<'a, Message> {
    use crate::app::ClosingOverlay;
    match closing {
        ClosingOverlay::About => about::modal(scheme),
        ClosingOverlay::Selector(selector) => project_selector::modal(selector, scheme),
        ClosingOverlay::Rename(draft) => rename::modal(draft, scheme),
        ClosingOverlay::Worktree(form, error) => {
            worktree_form::modal(form, error.as_deref(), scheme)
        }
        ClosingOverlay::Settings(draft) => settings_form::modal(draft, scheme, env_include_outcome),
        ClosingOverlay::ConfirmDelete(dir) => {
            // Fading-out snapshot: the override may already be gone, so fall back to the derived
            // name for the exit animation. The live state (branch, checkbox choice) is gone by
            // now too — this non-interactive snapshot fades the dialog without the branch
            // checkbox rather than reconstructing it.
            let friendly = micold_core::naming::display_name(dir);
            confirm_delete::modal(dir, &friendly, None, false, scheme)
        }
        ClosingOverlay::WorktreeRename(draft) => worktree_rename::modal(draft, scheme),
        ClosingOverlay::ConfirmSessionRemove(label) => confirm_session_remove::modal(label, scheme),
        ClosingOverlay::ConfirmForget(display_name, running) => {
            confirm_forget::modal(display_name, *running, scheme)
        }
    }
}

/// Keyboard subscription. While a modal overlay is open, Esc dismisses it — the About
/// dialog (FR-011) or the project selector. Mirrors [`crate::app::on_escape`].
///
/// Feature 006 (FR-009): while the embedded terminal holds focus, the app binds NO global
/// keyboard shortcuts — every key is owned by the focused terminal widget (so Esc and any app
/// chord reach the `claude` process instead of driving the app).
pub fn subscription(state: &State) -> Subscription<Message> {
    /// One Esc-dismiss subscription, for `$message`.
    ///
    /// This is a macro rather than a helper `fn` because `Subscription::filter_map` requires a
    /// **zero-sized** closure and derives the subscription's identity from that closure's
    /// `TypeId`. A function taking the message would capture it (non-zero-sized, rejected at
    /// compile time); a single shared closure would give every overlay the same identity, so
    /// iced would keep the previous overlay's recipe alive across a switch and Esc would emit
    /// the wrong message. Each macro expansion is a distinct closure expression, hence a
    /// distinct type — exactly what the 0.13 `on_key_press(fn)` form gave us per call site.
    macro_rules! on_escape {
        ($message:expr) => {
            iced::keyboard::listen().filter_map(|event| {
                use iced::keyboard::{key::Named, Event, Key};
                matches!(
                    event,
                    Event::KeyPressed {
                        key: Key::Named(Named::Escape),
                        ..
                    }
                )
                .then_some($message)
            })
        };
    }

    if state.terminal_focused {
        return Subscription::none();
    }
    // The sidebar filter panel (feature 009) is a lightweight popover, not a modal `Overlay`,
    // so it's checked ahead of the `Overlay` match below (mirrors `on_escape`'s priority).
    if state.overlay == Overlay::None && state.sidebar_filter_open {
        return on_escape!(Message::SidebarFilterMenuToggled);
    }
    match state.overlay {
        Overlay::None => Subscription::none(),
        Overlay::About => on_escape!(Message::AboutClosed),
        Overlay::ProjectSelector => on_escape!(Message::ProjectSelectorClosed),
        Overlay::RenameProject => on_escape!(Message::RenameCancelled),
        Overlay::AddWorktree => on_escape!(Message::AddWorktreeCancelled),
        Overlay::Settings => on_escape!(Message::SettingsCancelled),
        Overlay::ConfirmWorktreeDelete => on_escape!(Message::WorktreeDeleteCancelled),
        Overlay::RenameWorktree => on_escape!(Message::WorktreeRenameCancelled),
        Overlay::ConfirmSessionRemove => on_escape!(Message::SessionRemoveCancelled),
        Overlay::ConfirmForgetProject => on_escape!(Message::ProjectForgetCancelled),
    }
}
