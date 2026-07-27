//! `ProjectSwitcher` — the top-bar project switcher primitive (Constitution Principle VIII).
//!
//! Follows the split idiom of [`super::menu`]: a [`ProjectSwitcherTrigger`] icon+label button
//! that lives in the toolbar (placed left of the overflow [`super::MenuTrigger`]), and a
//! [`ProjectSwitcherOverlay`] that floats the project list **above** the window as a true
//! overlay, so opening it never reflows the bar. Rows carry an active marker, a running
//! background-session count, and an unavailable badge; a trailing "Add project…" row opens the
//! folder browser. Reuses the shared backdrop/stack overlay machinery rather than forking one.

use super::{menu_panel, Tooltip};
use crate::icons::{icon_role, Icon, IconSurface};
use micold_core::tokens::{spacing, type_scale, Roles};
use crate::ui::{icon, style};
use iced::widget::{button, column, container, mouse_area, row, text, Space};
use iced::{Alignment, Element, Length};

/// The width of the switcher panel.
const PANEL_WIDTH: f32 = 260.0;
/// Vertical offset so the panel clears the toolbar (approx. toolbar height).
const TOP_OFFSET: f32 = 52.0;

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
            icon(Icon::OpenProject, type_scale::BODY, tint),
            text(t.label).size(type_scale::BODY),
        ]
        .spacing(spacing::XS)
        .align_y(Alignment::Center);
        let trigger = button(content)
            .padding(spacing::XS)
            .style(style::text_button(t.roles))
            .on_press(t.on_toggle);
        // Hover tooltip identifying the control (reuses the shared Tooltip primitive, Principle VIII).
        Tooltip::new(trigger, "Project selector", t.roles).into()
    }
}

/// Floats the switcher panel over `base`, anchored top-right below the toolbar. An invisible
/// full-window backdrop dismisses it on an outside click. When `open` is false the overlay is
/// absent and `base` is returned as-is. Builder form (Principle VIII):
/// `ProjectSwitcherOverlay::new(base, rows, on_add, on_dismiss, roles).open(b).into()`.
pub struct ProjectSwitcherOverlay<'a, M> {
    base: Element<'a, M>,
    rows: Vec<ProjectRow<M>>,
    on_add: M,
    on_dismiss: M,
    roles: Roles,
    open: bool,
}

impl<'a, M: Clone + 'a> ProjectSwitcherOverlay<'a, M> {
    /// A switcher panel over `base` listing `rows`, with an "Add project…" row emitting
    /// `on_add` and outside-click dismissal via `on_dismiss`. Closed by default.
    pub fn new(
        base: impl Into<Element<'a, M>>,
        rows: Vec<ProjectRow<M>>,
        on_add: M,
        on_dismiss: M,
        roles: Roles,
    ) -> Self {
        Self {
            base: base.into(),
            rows,
            on_add,
            on_dismiss,
            roles,
            open: false,
        }
    }

    /// Whether the panel is shown.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }
}

impl<'a, M: Clone + 'a> From<ProjectSwitcherOverlay<'a, M>> for Element<'a, M> {
    fn from(m: ProjectSwitcherOverlay<'a, M>) -> Self {
        let ProjectSwitcherOverlay {
            base,
            rows,
            on_add,
            on_dismiss,
            roles: r,
            open,
        } = m;
        if !open {
            return base;
        }

        let active_tint = icon_role(IconSurface::Badge, r);
        let error_tint = icon_role(IconSurface::Unavailable, r);
        let add_tint = icon_role(IconSurface::AppBarAction, r);

        let mut list = column![].spacing(spacing::XS).width(Length::Fill);
        for pr in rows {
            let mut content = row![].spacing(spacing::SM).align_y(Alignment::Center);
            if pr.is_active {
                content = content.push(icon(Icon::ActiveMarker, type_scale::LABEL, active_tint));
            }
            content = content.push(text(pr.label).size(type_scale::BODY).width(Length::Fill));
            if pr.running_count > 0 {
                content = content.push(
                    text(format!("{} running", pr.running_count))
                        .size(type_scale::LABEL)
                        .style(style::muted(r)),
                );
            }
            if !pr.available {
                content = content.push(icon(Icon::Unavailable, type_scale::LABEL, error_tint));
            }
            let mut entry = button(content)
                .width(Length::Fill)
                .padding(spacing::SM)
                .style(style::text_button(r));
            // Unavailable projects are shown but cannot be activated (FR-008).
            if pr.available {
                entry = entry.on_press(pr.on_select);
            }
            // Right-click opens the row's context menu (feature 015). Offered even for
            // unavailable projects — those are precisely the ones a user wants to forget.
            let row: Element<'_, M> = match pr.on_context {
                Some(msg) => mouse_area(entry).on_right_press(msg).into(),
                None => entry.into(),
            };
            list = list.push(row);
        }

        // Trailing "Add project…" row opens the existing folder browser (FR-009).
        list = list.push(
            button(
                row![
                    icon(Icon::OpenProject, type_scale::BODY, add_tint),
                    text("Add project…").size(type_scale::BODY),
                ]
                .spacing(spacing::SM)
                .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .padding(spacing::SM)
            .style(style::text_button(r))
            .on_press(on_add),
        );

        let panel = container(menu_panel(list, Length::Fixed(PANEL_WIDTH), r, true))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Right)
            .align_y(iced::alignment::Vertical::Top)
            .padding(iced::Padding {
                top: TOP_OFFSET,
                right: spacing::SM,
                bottom: 0.0,
                left: 0.0,
            });

        // Invisible backdrop that dismisses the switcher on any outside click.
        let backdrop = mouse_area(
            container(Space::new().width(Length::Fill).height(Length::Fill))
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .on_press(on_dismiss);

        iced::widget::stack![base, backdrop, panel].into()
    }
}
