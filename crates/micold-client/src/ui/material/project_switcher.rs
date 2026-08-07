//! `ProjectSwitcher` — the top-bar project switcher primitive (Constitution Principle VIII).
//!
//! Follows the split idiom of [`super::menu`]: a [`ProjectSwitcherTrigger`] icon+label button
//! that lives in the toolbar (placed left of the overflow [`super::MenuTrigger`]), and a
//! [`ProjectSwitcherOverlay`] that floats the project list **above** the window, so opening it
//! never reflows the bar. Rows carry an active marker, a running background-session count, and an
//! unavailable badge; a trailing "Add project…" row opens the folder browser.
//!
//! Floating, dismissal and z-order come from the one shared primitive
//! ([`cdk::overlay`](crate::ui::cdk::overlay), FR-008) — this module builds the panel and says
//! where it wants to sit.

use super::{menu, menu_panel, MenuItem, Tooltip};
use crate::icons::{icon_role, Icon, IconSurface};
use crate::ui::cdk::overlay::{Anchor, Surface};
use crate::ui::material::glyph::icon;
use crate::ui::material::style;
use crate::ui::material::{Text, TypeRole};
use iced::widget::{button, row};
use iced::{Alignment, Element, Length};
use micold_core::overlay::Layer;
use micold_core::tokens::{anatomy, shape, spacing, Roles};

/// The width of the switcher panel.
const PANEL_WIDTH: f32 = 260.0;

/// One project row rendered in the switcher panel. Generic over the message type for reuse.
pub struct ProjectRow<M> {
    /// The project's display name.
    pub label: String,
    /// Whether this is the currently active project (drawn with an active marker) (FR-006).
    pub is_active: bool,
    /// Number of running background sessions this project holds (FR-007). `0` = no badge.
    pub running_count: usize,
    /// Whether the project's folder is available; unavailable rows are badged + not selectable (FR-008).
    pub available: bool,
    /// Message emitted when an available row is selected (switch to this project).
    pub on_select: M,
    /// Message emitted when the row is right-clicked, opening its context menu (feature 015).
    /// `None` for rows that are not projects, which therefore expose no context menu. The
    /// trailing "Add project…" affordance is added by the overlay itself and never carries one.
    pub on_context: Option<M>,
}

/// The switcher trigger: an icon + active-project label button placed in the toolbar (FR-004).
/// Builder form (Principle VIII): `ProjectSwitcherTrigger::new(label, on_toggle, roles).into()`.
pub struct ProjectSwitcherTrigger<M> {
    label: String,
    on_toggle: M,
    roles: Roles,
}

impl<M> ProjectSwitcherTrigger<M> {
    /// A trigger showing `label` (the active project) that emits `on_toggle` when pressed.
    pub fn new(label: impl Into<String>, on_toggle: M, roles: Roles) -> Self {
        Self {
            label: label.into(),
            on_toggle,
            roles,
        }
    }
}

impl<'a, M: Clone + 'a> From<ProjectSwitcherTrigger<M>> for Element<'a, M> {
    fn from(t: ProjectSwitcherTrigger<M>) -> Self {
        let tint = icon_role(IconSurface::AppBarAction, t.roles);
        let content = row![
            icon(Icon::OpenProject, TypeRole::Action.size(), tint),
            Text::new(t.label, TypeRole::Action, t.roles),
        ]
        .spacing(spacing::XS)
        .align_y(Alignment::Center);
        // Every pressable here ripples, like every other pressable surface in the library
        // (FR-021, SC-005a). `shape::FULL` because that is the corner `style::text_button` draws —
        // a ripple clipped to the wrong shape is the bug `shape_bands` exists to prevent.
        let trigger = button(content)
            .padding(spacing::XS)
            .style(style::text_button(t.roles))
            .on_press(t.on_toggle);
        let trigger = super::Ripple::new(trigger, t.roles.on_surface, shape::FULL);
        // Hover tooltip identifying the control (reuses the shared Tooltip primitive, Principle VIII).
        Tooltip::new(trigger, "Project selector", t.roles).into()
    }
}

/// The switcher panel, anchored top-right below the toolbar. Builder form (Principle VIII):
/// `ProjectSwitcherOverlay::new(rows, on_add, on_dismiss, roles).open(b).into()`, yielding
/// `Option<Surface>` — `None` while closed, so a closed switcher leaves no trace.
pub struct ProjectSwitcherOverlay<'a, M> {
    rows: Vec<ProjectRow<M>>,
    on_add: M,
    on_dismiss: M,
    roles: Roles,
    open: bool,
    lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a, M: Clone + 'a> ProjectSwitcherOverlay<'a, M> {
    /// A switcher panel listing `rows`, with an "Add project…" row emitting `on_add` and
    /// outside-click dismissal via `on_dismiss`. Closed by default.
    pub fn new(rows: Vec<ProjectRow<M>>, on_add: M, on_dismiss: M, roles: Roles) -> Self {
        Self {
            rows,
            on_add,
            on_dismiss,
            roles,
            open: false,
            lifetime: std::marker::PhantomData,
        }
    }

    /// Whether the panel is shown.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }
}

/// The vertical stack of project rows, with the trailing "Add project…" affordance.
///
/// `pub(super)` for the §7.5 anatomy tests, which need a row's laid-out box and cannot get it from
/// the public entry point: that yields a `cdk::overlay::Surface` rather than an `Element`, and wraps
/// the rows in a panel whose padding would have to be subtracted back out by hand. Same reason
/// `menu::item_column` is reachable.
/// A project row **is** a menu item (FR-029b): a leading marker, a label, a trailing count, a
/// trailing badge and a right-press. It used to be a hand-built copy of [`menu::item_column`] — the
/// same `button`, the same `text_button` style, the same `Ripple`, written out a second time — and
/// what that cost is BUG-003. §7.5's 48dp item height was applied to the original and the copy
/// stayed at 36dp, so the two panels hanging off the app bar disagreed about how tall a row is, and
/// every remaining figure in §7.5 would have had to be applied twice to reach both.
pub(super) fn row_column<'a, M: Clone + 'a>(
    rows: Vec<ProjectRow<M>>,
    on_add: M,
    r: Roles,
) -> Element<'a, M> {
    let active_tint = icon_role(IconSurface::Badge, r);
    let error_tint = icon_role(IconSurface::Unavailable, r);
    let add_tint = icon_role(IconSurface::AppBarAction, r);

    let mut items: Vec<MenuItem<M>> = rows
        .into_iter()
        .map(|pr| MenuItem {
            icon: pr.is_active.then_some(Icon::ActiveMarker),
            icon_tint: Some(active_tint),
            label: pr.label,
            // Unavailable projects are shown but cannot be activated (FR-008 of feature 008).
            message: pr.available.then_some(pr.on_select),
            trailing_text: (pr.running_count > 0).then(|| format!("{} running", pr.running_count)),
            trailing_icon: (!pr.available).then_some((Icon::Unavailable, error_tint)),
            // Right-click opens the row's context menu (feature 015). Offered even for unavailable
            // projects — those are precisely the ones a user wants to forget.
            on_context: pr.on_context,
        })
        .collect();

    // Trailing "Add project…" row opens the existing folder browser (FR-009).
    items.push(MenuItem {
        icon_tint: Some(add_tint),
        ..MenuItem::new(Icon::OpenProject, "Add project…", on_add)
    });

    menu::item_column(items, r)
}

impl<'a, M: Clone + 'a> From<ProjectSwitcherOverlay<'a, M>> for Option<Surface<'a, M>> {
    fn from(m: ProjectSwitcherOverlay<'a, M>) -> Self {
        let ProjectSwitcherOverlay {
            rows,
            on_add,
            on_dismiss,
            roles: r,
            open,
            ..
        } = m;
        if !open {
            return None;
        }

        let panel = menu_panel(
            row_column(rows, on_add, r),
            Length::Fixed(PANEL_WIDTH),
            r,
            true,
            menu::panel_padding(),
        );
        Some(
            Surface::new(
                Layer::Popover,
                panel,
                // §7.1's bottom edge, read rather than restated (FR-029a). This module carried its
                // own `TOP_OFFSET = 52.0`, word for word the same guess `menu.rs` did, so the
                // switcher opened 13dp inside the bar exactly as the overflow menu did (BUG-003).
                Anchor::TopEnd {
                    top: anatomy::app_bar::BOTTOM_EDGE,
                    end: spacing::SM,
                },
            )
            .on_dismiss(on_dismiss),
        )
    }
}
