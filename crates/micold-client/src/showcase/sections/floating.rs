//! Floating surfaces and the controls that open them (feature 020, T021).
//!
//! A dialog, a menu panel, a context menu and the project switcher cover the page when open, so each
//! is opened from its own section and dismissed without leaving the page (FR-007): press its trigger,
//! then press Escape or click the scrim. The page stays scrollable underneath, and because
//! `Showcase::open` holds one surface at a time, no pair of them can trap it (spec, Edge Cases).
//!
//! Every surface goes onto the same `cdk::overlay::Overlay` the application uses. That host is what
//! the two exemptions in the catalogue name: it decides *where* a panel sits and what dismisses it,
//! and draws nothing itself, so it is exercised by this section rather than posed in it.

use iced::{Element, Length};
use micold_core::tokens::{anatomy, spacing, Roles};

use crate::icons::{icon_role, Icon, IconSurface};
use crate::showcase::catalogue::Layout;
use crate::showcase::gallery::{arrange, posed};
use crate::showcase::samples;
use crate::showcase::state::{Floating, Message, Showcase};
use crate::ui::cdk;
use crate::ui::material::{self, SurfaceKind, TypeRole};

/// The items every menu instance in this section shows.
fn menu_items() -> Vec<material::MenuItem<Message>> {
    vec![
        material::MenuItem::new(Icon::Copy, "Copy name", Message::NoOp),
        material::MenuItem::new(Icon::Rename, "Rename", Message::NoOp),
        material::MenuItem::new(Icon::Delete, "Delete", Message::NoOp),
    ]
}

/// The invented project rows the switcher shows: an active one, one with running sessions, and one
/// whose folder is gone.
///
/// `MenuItem`s, because that is what the switcher's list is made of — it stopped being a component
/// of its own at BUG-007, and the rows here are built exactly as `ui::view` builds them.
fn project_rows(roles: Roles) -> Vec<material::MenuItem<Message>> {
    samples::PROJECTS
        .iter()
        .enumerate()
        .map(|(row, (label, running, available))| material::MenuItem {
            icon: (row == 0).then_some(Icon::ActiveMarker),
            reserve_icon: true,
            icon_tint: Some(icon_role(IconSurface::Badge, roles)),
            label: (*label).to_string(),
            message: available.then_some(Message::NoOp),
            trailing_text: (*running > 0).then(|| format!("{running} running")),
            trailing_icon: (!*available).then_some((
                Icon::Unavailable,
                icon_role(IconSurface::Unavailable, roles),
            )),
            on_context: Some(Message::NoOp),
        })
        .collect()
}

/// A button that opens `surface`, labelled so the page says what pressing it will do.
fn opener<'a>(label: &'a str, surface: Floating, roles: Roles) -> Element<'a, Message> {
    material::Button::with_content(
        material::Text::new(label, TypeRole::Body, roles),
        material::ButtonVariant::Outlined,
        roles,
    )
    .on_press(Message::Opened(surface))
    .into()
}

/// Every floating surface the gallery can show, in a **stable, non-empty set**.
///
/// Non-empty is the load-bearing word. `cdk::overlay::Overlay` returns its base untouched when nothing
/// is pushed and wraps it in a `stack` when something is — so a set that empties and refills inserts and
/// removes a level *above* the page, and iced reallocates the state of everything beneath it. The
/// visible symptom is the page jumping to the top every time a surface opens, because a scrollable's
/// offset is widget-tree state.
///
/// The application never meets this: it pushes its overflow menu unconditionally ("pushed whether or
/// not it is open: the panel owns its own fade"), so its set is never empty. The gallery now does the
/// same. A count that changes *after* index 0 is harmless — only the base changing depth resets state.
///
/// The comparisons below map a value to an element, which is what a renderer does. The *rule* about
/// which surfaces may be open at once is not here — it is the reducer's `Option`, where it is tested.
pub fn surfaces<'a>(
    showcase: &'a Showcase,
    roles: Roles,
) -> Vec<cdk::overlay::Surface<'a, Message>> {
    let open = showcase.open;
    let mut out: Vec<cdk::overlay::Surface<'a, Message>> = Vec::new();

    // Always present, exactly as the application pushes it. A closed menu is inert and blocks nothing.
    out.push(
        material::MenuOverlay::new(menu_items(), Message::Dismissed, roles)
            .open(open == Some(Floating::Menu))
            .into(),
    );

    if open == Some(Floating::ContextMenu) {
        out.push(
            material::ContextMenu::new(menu_items(), (120, 220), Message::Dismissed, roles).into(),
        );
        // The same panel on the other anchor: `rising_above` puts its **bottom** edge above the
        // window's bottom edge instead of its top edge at the cursor. Posed beside the default so
        // the difference is visible by comparison — a menu opened from a control in a bottom bar
        // has no room to hang downward, which is how the terminal's tab menu came to be drawn
        // half outside the window (012 BUG-004, the 2026-08-19 visual pass).
        out.push(
            material::ContextMenu::new(menu_items(), (420, 220), Message::Dismissed, roles)
                .rising_above(anatomy::app_bar::HEIGHT)
                .into(),
        );
    }

    // The project switcher's list — the same `MenuOverlay` above, carrying the switcher's rows
    // instead of the overflow menu's. Posed under the same entry for that reason.
    if open == Some(Floating::ProjectSwitcher) {
        out.push(
            material::MenuOverlay::new(project_rows(roles), Message::Dismissed, roles)
                .open(true)
                .into(),
        );
    }

    if open == Some(Floating::Modal) {
        out.push(modal_surface(roles));
    }

    out
}

/// The dialog the modal entry opens.
fn modal_surface<'a>(roles: Roles) -> cdk::overlay::Surface<'a, Message> {
    material::Modal::new(
        material::Surface::new(
            iced::widget::column![
                material::Text::new("A modal dialog", TypeRole::Title, roles),
                material::Text::new(samples::BODY, TypeRole::Body, roles),
                material::Button::with_content(
                    material::Text::new("Close", TypeRole::Body, roles),
                    material::ButtonVariant::Filled,
                    roles,
                )
                .on_press(Message::Dismissed),
            ]
            .spacing(spacing::MD),
            SurfaceKind::Dialog,
            roles,
        )
        .padding(spacing::LG)
        .width(Length::Fixed(420.0)),
        roles,
    )
    .on_dismiss(Message::Dismissed)
    .into()
}

/// `Modal` — opened from here, dismissed with Escape, the scrim, or its own Close button.
pub fn modal<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    arrange(
        vec![posed(
            "open it",
            opener("Open the dialog", Floating::Modal, roles),
            roles,
        )],
        Layout::Inline,
    )
}

/// `MenuOverlay` — the toolbar's overflow panel, and the project switcher's list, which is the
/// same panel carrying different items (018 FR-029c). Two openers under one entry so the pair can
/// be compared where they used to be two components that could not be.
pub fn menu_overlay<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    arrange(
        vec![
            posed(
                "open it",
                opener("Open the menu panel", Floating::Menu, roles),
                roles,
            ),
            posed(
                "the same panel, the switcher's rows",
                opener(
                    "Open the project switcher",
                    Floating::ProjectSwitcher,
                    roles,
                ),
                roles,
            ),
        ],
        Layout::Inline,
    )
}

/// `ContextMenu` — the cursor-anchored menu a right-click opens.
pub fn context_menu<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    arrange(
        vec![posed(
            "open it",
            opener("Open a context menu", Floating::ContextMenu, roles),
            roles,
        )],
        Layout::Inline,
    )
}

/// `MenuTrigger` — the icon button that opens a menu panel.
pub fn menu_trigger<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    arrange(
        vec![posed(
            "default",
            material::MenuTrigger::new(Icon::Menu, Message::Opened(Floating::Menu), roles),
            roles,
        )],
        Layout::Inline,
    )
}

/// `Tooltip` — hover-driven, so it has nothing to pose: point at an instance and wait.
pub fn tooltip<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    arrange(
        vec![
            posed(
                "below (the default)",
                material::Tooltip::new(
                    material::IconButton::new(Icon::Settings, roles).on_press(Message::NoOp),
                    "Settings",
                    roles,
                ),
                roles,
            ),
            posed(
                "to the left",
                material::Tooltip::new(
                    material::IconButton::new(Icon::Menu, roles).on_press(Message::NoOp),
                    "More actions",
                    roles,
                )
                .position(material::TooltipPosition::Left),
                roles,
            ),
        ],
        Layout::Inline,
    )
}

/// `Snackbar` — the visible notification, at both severities.
///
/// Posed at rest rather than driven by the queue: the queue lives in `micold-core` and is tested
/// there without a renderer, so what the gallery has to show is the *surface* — the one place in
/// the application that is deliberately inverted, and the only use of the `inverse_*` roles.
///
/// `Anchor::BottomCenter` is the band it floats on: above a dialog and its scrim, but bottom-
/// aligned so it does not sit over the dialog's action row.
pub fn snackbar<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    use micold_core::notify::{Level, Notification};
    use std::sync::LazyLock;

    // Statics rather than values built here: `Snackbar` borrows the notification it draws, and a
    // render function has nowhere to own one. Leaking a `Box` would satisfy the borrow and lose a
    // little memory on **every frame** the gallery draws, which is exactly the kind of cost that
    // never announces itself.
    static INFO: LazyLock<Notification> =
        LazyLock::new(|| Notification::new(Level::Info, samples::LABEL));
    static ERROR: LazyLock<Notification> = LazyLock::new(|| {
        Notification::new(
            Level::Error,
            "Could not create the worktree — that branch is already checked out.",
        )
    });
    let (info, error) = (&*INFO, &*ERROR);

    arrange(
        vec![
            posed(
                "info, dismissible",
                material::Snackbar::new(info, roles).on_dismiss(Message::NoOp),
                roles,
            ),
            posed(
                "error, dismissible",
                material::Snackbar::new(error, roles).on_dismiss(Message::NoOp),
                roles,
            ),
            posed(
                "no action",
                material::Snackbar::<Message>::new(info, roles),
                roles,
            ),
        ],
        Layout::FullWidth,
    )
}
