//! iced rendering layer for the main window. Bin-only; compiled with the `gui` feature.

pub(crate) mod about;
pub mod cdk;
pub(crate) mod confirm_delete;
pub(crate) mod confirm_forget;
pub(crate) mod confirm_session_remove;
mod focus;
/// The component library. `pub(crate)` rather than private to `ui`, so the component showcase
/// (feature 020, `crate::showcase`) can compose the very same components the application renders —
/// which is the whole of FR-002. The crate's *public* API is unchanged: nothing outside
/// `micold_client` gains access, and `material::style` stays `pub(crate)` and unreachable from a call
/// site. `tests/material_boundary.rs` scans the showcase at the same zero budgets it holds these
/// feature modules to, so the wider visibility cannot become a way to style a widget by hand.
pub(crate) mod material;
/// The reference scene's ripple, for the frame probe (feature 018, FR-039b).
///
/// Named individually rather than by opening the module, which stays `pub(crate)`. The binary
/// composes the `full` measurement scene and a ripple only starts from a press, so it needs the one
/// traversal that can reach a ripple's per-instance state — and nothing else from the library.
pub use focus::{into_view as focus_into_view, scroll_focused_into_view};
pub use material::ripple_pulse;
pub use material::target_offset_delta;
pub(crate) mod project_selector;
pub(crate) mod rename;
/// The Settings sections — one module per page of the full-surface view (feature 027, FR-026).
pub(crate) mod settings;
pub(crate) mod settings_view;
mod shell;
mod sidebar;
// The sidebar list's scroll viewport, by name — the binary addresses `scroll_to` to it when a
// reveal has a row to bring into view (feature 024). Re-exported like `ripple_pulse` above: the
// binary needs this one name, not the module.
pub use sidebar::SIDEBAR_SCROLL_ID;
// The application's theme — the only thing the styling layer exposes beyond the component
// library (FR-002). Defined in `material`, not here: a feature module naming the styling layer is
// exactly what the boundary test forbids, and `ui/mod.rs` is a feature module.
pub use material::theme;
pub mod terminal;
mod toolbar;
pub(crate) mod worktree_form;
pub(crate) mod worktree_rename;

use crate::app::{Message, State};
use crate::icons::{icon_role, Icon, IconSurface};
use iced::widget::{column, container, row, Space};
use iced::{Element, Length, Subscription};
use micold_core::session::SessionId;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, spacing, Roles};

/// How a dialog builds its body from the state that opened it.
///
/// The render half of a floating surface. It is *not* on [`FloatingSurface`](crate::overlay::
/// FloatingSurface), and cannot be: FR-006 forbids a feature module naming a rendering framework,
/// and Tier 1 sited views here, beside the feature rather than inside it. So the two halves are
/// named together on the surface's registration line instead — one line still buys a whole
/// surface, which is what FR-009 asks for.
///
/// `None` means the surface is open but the live state it draws is absent: render nothing rather
/// than an empty dialog.
///
/// A bare `fn` pointer rather than a boxed closure so it can sit in a `static` registration table
/// and be `Copy`. Every dialog takes the same three arguments whether or not it needs all of them,
/// which is the price of the uniformity — an environment-include outcome is meaningless to the
/// About box, but a signature that varied per dialog could not be dispatched generically at all.
pub type DialogView = for<'a> fn(
    &'a State,
    ColorScheme,
    &'a micold_core::env_include::EnvIncludeOutcome,
) -> Option<Element<'a, Message>>;

/// Where the sidebar's own context menus open: just inside the sidebar, below the app bar.
///
/// A fixed point rather than the cursor — these menus are opened from a row whose position the view
/// does not know — and the one place it is stated, rather than the two literals it was (T107).
const SIDEBAR_MENU_ANCHOR: (u16, u16) = (24, 96);

// The icon font and the two primitives that draw a glyph moved into the component library with
// everything else that decides an appearance (FR-001). Re-exported here for `main`, which
// registers the font at startup, and for the tests that assert what the font file advertises.
pub use material::glyph::{icon, icon_colored, MATERIAL_SYMBOLS, MATERIAL_SYMBOLS_BYTES};
pub use material::{ROBOTO, ROBOTO_MEDIUM_BYTES, ROBOTO_REGULAR_BYTES};

use crate::features::connection::ConnectionStatus;

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

/// Render the main window: the top app bar over the shell body (active project / empty state),
/// with any floating surface stacked on top. Every surface is styled from the active color
/// scheme's design tokens.
///
/// No animation state is passed in: every transition belongs to the component that plays it, and
/// each owns its progress in the widget tree (FR-011, FR-014). `dismissing` is the snapshot of a
/// just-closed dialog still animating out — the one thing an application still has to hold, since
/// it outlives the state that opened it (rendered instead of a live dialog once no dialog is open —
/// see [`crate::overlay::registry::Closing`]).
#[allow(clippy::too_many_arguments)]
pub fn view<'a>(
    state: &'a State,
    grid: Option<&'a crate::grid::GridCache>,
    selection: Option<&'a crate::selection::Selection>,
    display_offset: usize,
    dismissing: Option<&'a crate::overlay::registry::Closing>,
    env_include_outcome: &'a micold_core::env_include::EnvIncludeOutcome,
    connection: &ConnectionStatus,
) -> Element<'a, Message> {
    let scheme = state.color_scheme();
    let roles = tokens::roles(scheme);
    let bg = roles.background;

    // With a project open, show the worktree sidebar beside the main area; the main area is
    // the embedded terminal when a session is active (FR-012), else the project surface. The
    // sidebar slides in/out and is resizable; the main content fades when it changes.
    // Settings takes the whole content area rather than floating over it (feature 027, FR-026).
    // Checked before the project branch, and returning early from the same `body` binding, so that
    // the app bar and the connection banner below still frame it — a full-surface view is not a
    // separate window, and the way out of it has to stay visible.
    let body: Element<'a, Message> = if let Some(draft) = state.settings_draft.as_ref() {
        material::ViewFade::new(
            settings_view::view(draft, env_include_outcome, state.focused_field, scheme),
            bg,
        )
        .showing(main_content_key(state))
        .into()
    } else if state.workspace.active_project().is_some() {
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

    // The project switcher panel — the same `MenuOverlay` the ⋮ menu is, carrying different items
    // (018 FR-029c). Rows are built purely from the workspace: active marker,
    // running-background-session count, and unavailable badge. Mutually exclusive with the
    // overflow menu (handled in the reducer).
    //
    // It was its own component until BUG-007, and every difference between the two was a difference
    // nobody chose: a 260dp panel beside a 240dp one from the same edge, and no exit transition at
    // all, because the fade lives in the shared panel.
    let mut switcher_items: Vec<material::MenuItem<Message>> = state
        .switcher_entries()
        .into_iter()
        .map(|e| material::MenuItem {
            icon: e.is_active.then_some(Icon::ActiveMarker),
            // Held on every row, marked or not, so the marker says which project is active without
            // also deciding where that row's label starts (008 FR-006a).
            reserve_icon: true,
            icon_tint: Some(icon_role(IconSurface::Badge, roles)),
            label: e.label,
            // Unavailable projects are shown but cannot be activated (008 FR-008).
            message: e
                .available
                .then(|| Message::KnownProjectReopened(e.path.clone())),
            trailing_text: (e.running_count > 0).then(|| format!("{} running", e.running_count)),
            trailing_icon: (!e.available).then_some((
                Icon::Unavailable,
                icon_role(IconSurface::Unavailable, roles),
            )),
            // Right-click a project row to reach its "Forget project" menu (feature 015). Offered
            // even for unavailable projects — those are precisely the ones a user wants to forget.
            on_context: Some(Message::ProjectMenuToggled(e.path)),
        })
        .collect();
    // Trailing "Add project…" row opens the existing folder browser (008 FR-009). A row, not a
    // project, so it carries no context menu.
    switcher_items.push(material::MenuItem {
        icon_tint: Some(icon_role(IconSurface::AppBarAction, roles)),
        ..material::MenuItem::new(
            Icon::OpenProject,
            "Add project…",
            Message::ProjectSelectorOpened,
        )
    });
    let switcher: cdk::overlay::Surface<'a, Message> =
        material::MenuOverlay::new(switcher_items, Message::ProjectSwitcherToggled, roles)
            .open(state.project_switcher_open)
            .into();

    // The right-clicked project's context menu, at the cursor (feature 015), like a normal desktop
    // context menu: the panel's top-left corner sits at the click point. The anchor is clamped at
    // render time (not when the menu opened) so a window resize while it is showing can never leave
    // the panel hanging off the edge. The switcher stays open behind it — which is now a property
    // of the context-menu layer rather than of the order these are built in.
    let project_menu: Option<cdk::overlay::Surface<'a, Message>> =
        state.project_menu_open.as_ref().map(|menu| {
            let (x, y) = crate::features::project::clamp_menu_anchor(
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
    //
    // Clamped, like the project menu above and unlike its own past self (018's BUG-003, T107). The
    // anchor is a fixed point rather than the cursor, so it never *starts* off-screen — but the
    // panel it positions is now 48dp per item where it was 36, a four-item menu having grown from
    // 152dp to 200, and nothing was stopping it running off the bottom of a short window.
    let worktree_menu: Option<cdk::overlay::Surface<'a, Message>> =
        state.worktree_menu_open.as_ref().map(|dir| {
            let included = state
                .worktrees
                .iter()
                .any(|w| &w.dir_name == dir && w.included);
            let items = worktree_menu_items(dir, &state.worktree_display_name(dir), included);
            let (x, y) = crate::features::project::clamp_menu_anchor(
                SIDEBAR_MENU_ANCHOR,
                material::menu_panel_size(items.len()),
                state.window_size,
            );
            material::MenuOverlay::new(items, Message::WorktreeMenuDismissed, roles)
                .anchor(iced::Point::new(x as f32, y as f32))
                .into()
        });

    // The session right-click context menu (feature 010's BUG-003). Only present while a session's
    // menu is open. Same anchor and the same clamping as the worktree menu above.
    let session_menu: Option<cdk::overlay::Surface<'a, Message>> =
        state.session_menu_open.map(|id| {
            let items = session_menu_items(id);
            let (x, y) = crate::features::project::clamp_menu_anchor(
                SIDEBAR_MENU_ANCHOR,
                material::menu_panel_size(items.len()),
                state.window_size,
            );
            material::MenuOverlay::new(items, Message::SessionMenuDismissed, roles)
                .anchor(iced::Point::new(x as f32, y as f32))
                .into()
        });

    // The dialog body for whatever is open — or, if one has just closed, the snapshot it left
    // behind (captured before the core cleared its live state) so the exit has something to draw
    // (FR-002). Each of these builds only the dialog; the transition around it belongs to `Modal`,
    // which owns the three tracks that carry it in and out.
    // Which dialog, from the registry; how to draw it, from the view its registration line names.
    // This was a ten-arm match over the overlay enum — every dialog's state lookup written out
    // here, hundreds of lines from the dialog it belonged to, and one more arm to remember for
    // every dialog anyone added. Each of those lookups now lives beside the dialog that needs it
    // (feature 021, T035 — FR-008, SC-001).
    let open_dialog = crate::overlay::registry::open_dialog(state);
    // A snapshot only draws once the live dialog is gone. It draws through the *same* registration
    // as the live one — its own `open_dialog`, over the state it was taken of — so the exit is the
    // enter in reverse by construction rather than by a second per-dialog list agreeing with the
    // first (feature 021, T036 — SC-001).
    let closing = dismissing.filter(|_| open_dialog.is_none());
    let dialog: Option<Element<'a, Message>> = match &open_dialog {
        Some(open) => open
            .view()
            .and_then(|view| view(state, scheme, env_include_outcome)),
        None => closing.and_then(|closing| {
            let taken_of = closing.state();
            closing
                .surface()
                .and_then(|open| open.view())
                .and_then(|view| view(taken_of, scheme, env_include_outcome))
        }),
    };
    // The identity of the dialog being rendered, which for a snapshot is the one it was taken of —
    // not "nothing open", which is merely where the application has got to.
    let drawn = open_dialog
        .as_ref()
        .map(|open| open.id())
        .or_else(|| closing.map(|closing| closing.id()));

    let modal: Option<cdk::overlay::Surface<'a, Message>> = dialog.map(|dialog| {
        // A snapshot is a dialog on its way out: nothing is open behind it, so it is not shown and
        // has nothing left to cancel.
        let mut modal = material::Modal::new(dialog, roles)
            .shown(open_dialog.is_some())
            .restart_on(drawn.map(surface_key).unwrap_or(0))
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

    // The snackbar goes through the same overlay as everything else that floats (FR-008), on its
    // own band above `Dialog`. Stacking it over the finished overlay instead would have put it
    // above the dialog just as well — and changed the root's shape, which every recorded layout
    // anchor is expressed against. The band is what the overlay exists for.
    let snackbar: Option<cdk::overlay::Surface<'a, Message>> =
        state.notify.visible().map(|visible| {
            cdk::overlay::Surface::new(
                micold_core::overlay::Layer::Snackbar,
                material::Snackbar::new(visible, roles).on_dismiss(Message::NotificationDismissed),
                cdk::overlay::Anchor::BottomCenter {
                    bottom: spacing::LG,
                },
            )
        });

    cdk::overlay::Overlay::new(base)
        .push(overflow_menu)
        .push(switcher)
        .push_maybe(project_menu)
        .push_maybe(worktree_menu)
        .push_maybe(session_menu)
        .push_maybe(modal)
        .push_maybe(snackbar)
        .into()
}

/// The items in a worktree's right-click context menu (feature 008, FR-013; "Copy name" added
/// for cross-application clipboard access to labels the app doesn't render as selectable text).
fn worktree_menu_items(
    dir: &str,
    display_name: &str,
    included: bool,
) -> Vec<material::MenuItem<Message>> {
    let mut items = vec![
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
    ];
    // 016 BUG-002 (FR-030): inclusion is reversible from the row it produced. Offered only for the
    // rows it produced — "stop showing" means nothing for a worktree the app created, whose place
    // in the list follows from where it lives.
    if included {
        items.push(material::MenuItem::new(
            Icon::Close,
            "Stop showing",
            Message::WorktreeExcludeRequested(dir.to_string()),
        ));
    }
    items.push(material::MenuItem::new(
        Icon::Unavailable,
        "Delete",
        Message::WorktreeDeleteRequested(dir.to_string()),
    ));
    items
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
    // Settings is a content state like any other, so switching into and out of it crossfades the
    // way opening a project does. A sentinel rather than the next small integer: the session arm
    // below mixes a real id into the low bits, and a key that a session could collide with would
    // suppress the fade on exactly the transition it is for.
    if state.settings_draft.is_some() {
        return u64::MAX;
    }
    match (state.workspace.active_project(), state.active_session) {
        (None, _) => 0,
        (Some(_), None) => 1,
        // Offset past the two fixed states; the id itself distinguishes one session from another.
        (Some(_), Some(id)) => 2 ^ id.0.as_u64_pair().0,
    }
}

/// Which dialog is being drawn, so that switching straight from one to another replays the
/// entrance instead of inheriting a transition the previous dialog had already finished.
///
/// A surface's identity is a name and a transition's is a number, so the name is hashed. FNV-1a,
/// written out rather than reached for, because the value only has to tell two surfaces apart
/// within one run — and this way nothing about it depends on a hasher's defaults.
fn surface_key(id: crate::overlay::SurfaceId) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in id.as_str().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Keyboard subscription: while anything is floating, Esc dismisses it.
///
/// Feature 006 (FR-009): while the embedded terminal holds focus, the app binds NO global
/// keyboard shortcuts — every key is owned by the focused terminal widget (so Esc and any app
/// chord reach the `claude` process instead of driving the app).
///
/// # This function no longer knows what Escape closes (feature 021, T034)
///
/// It used to. It was a nine-arm match over the overlay enum, with the sidebar filter panel
/// hand-checked ahead of it because a popover is not an `Overlay` — a second copy of both the
/// per-dialog cancellations and the priority between bands, kept in step with `app::on_escape` by
/// hand. It now reports *that Escape happened* and lets the reducer ask the registry which
/// surface that reaches, exactly as the scroll trigger has worked since feature 017.
///
/// Three things fall out of that, none of them incidental:
///
/// - **The macro is gone.** `Subscription::filter_map` requires a zero-sized closure and derives
///   the subscription's identity from its `TypeId`, so naming the message per overlay meant one
///   distinct closure *expression* per overlay — otherwise iced kept the previous overlay's
///   recipe alive across a switch and Esc emitted the wrong message. A message that does not name
///   its target cannot be stale, so one shared closure is now correct.
/// - **The priority is the band ordering**, not a guard written above a match. A dialog outranks
///   a popover because `Layer` says so (contract D1).
/// - **The decision is made when Escape lands**, not when the frame was last rendered.
///
/// What is left here is the one thing that genuinely belongs to the view layer: whether to hold a
/// keyboard listener open at all. It is held only while Escape has something to close, so pressing
/// it with nothing open stays as inert as it was.
///
/// # Tab belongs here for the same reason (feature 027, FR-030)
///
/// Moving the keyboard's focus is the other thing that is about *whether a listener is open at
/// all* rather than about any one surface, and it is subject to the same terminal rule: inside a
/// session Tab is a tab character the shell wants, not a request to leave the field.
///
/// It is a second subscription rather than a second arm of the same closure because of the
/// `TypeId` rule above — one closure, one identity, and this one has to stay open in states where
/// Escape's does not (nothing floating, but there is still a form to tab through).
pub fn subscription(state: &State) -> Subscription<Message> {
    // Feature 006 (FR-009): the focused terminal owns every key, Tab and Escape included.
    if state.terminal_focused() {
        return Subscription::none();
    }
    let mut subs = vec![focus_traversal()];
    // Held only while Escape has something to close, so pressing it with nothing open stays as
    // inert as it was.
    if crate::app::on_escape(state).is_some() {
        subs.push(escape_dismissal());
    }
    Subscription::batch(subs)
}

/// Escape, reported as having happened; the reducer asks the registry what it reaches.
fn escape_dismissal() -> Subscription<Message> {
    iced::keyboard::listen().filter_map(escape_message)
}

/// Tab and Shift+Tab, as a request to move the keyboard's focus.
fn focus_traversal() -> Subscription<Message> {
    iced::keyboard::listen().filter_map(focus_move_message)
}

/// [`Message::EscapePressed`] for a press of Escape, and nothing for anything else.
///
/// A free function, not a closure written inline: `Subscription::filter_map` takes its identity
/// from the mapper's `TypeId`, so each of the two listeners needs a distinct named one.
fn escape_message(event: iced::keyboard::Event) -> Option<Message> {
    use iced::keyboard::{key::Named, Event, Key};
    matches!(
        event,
        Event::KeyPressed {
            key: Key::Named(Named::Escape),
            ..
        }
    )
    .then_some(Message::EscapePressed)
}

/// Which way Tab moves the focus, if this event is a Tab at all.
///
/// Public to the crate so `tests/` can state the mapping without a runtime: the defect this
/// answers (T075) was that *nothing* was listening, which no assertion about focus order could
/// have caught — the order was fine, the key never arrived.
pub fn focus_move_message(event: iced::keyboard::Event) -> Option<Message> {
    use iced::keyboard::{key::Named, Event, Key};
    match event {
        Event::KeyPressed {
            key: Key::Named(Named::Tab),
            modifiers,
            ..
        } => Some(Message::FocusMoved {
            forward: !modifiers.shift(),
        }),
        _ => None,
    }
}
