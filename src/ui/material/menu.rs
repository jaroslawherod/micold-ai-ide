//! `Menu` — a reusable overflow/dropdown menu primitive (Constitution Principle VIII).
//!
//! Split into a [`menu_trigger`] (an icon button that lives in the toolbar) and a
//! [`menu_overlay`] that floats the item panel **above** the rest of the window as a true
//! overlay (Angular-Material `mat-menu` style) — so opening it never reflows the toolbar.
//! Reused for the toolbar's overflow menu; any future dropdown should reuse it.

use crate::ui::material::icon_button;
use crate::ui::{icon, style};
use iced::widget::{button, column, container, mouse_area, row, text, Space};
use iced::{Alignment, Element, Length};
use micold_ai_ide::icons::Icon;
use micold_ai_ide::tokens::{spacing, type_scale, Roles};

/// One entry in a menu. Generic over the message type for reuse.
pub struct MenuItem<M> {
    /// Optional leading icon.
    pub icon: Option<Icon>,
    /// The item label.
    pub label: String,
    /// Message emitted when the item is activated.
    pub message: M,
}

impl<M> MenuItem<M> {
    /// A labeled item with a leading icon.
    pub fn new(icon: Icon, label: impl Into<String>, message: M) -> Self {
        Self {
            icon: Some(icon),
            label: label.into(),
            message,
        }
    }
}

/// The width of the dropdown panel.
const PANEL_WIDTH: f32 = 220.0;
/// Vertical offset so the panel clears the toolbar (approx. toolbar height).
const TOP_OFFSET: f32 = 52.0;

/// The menu trigger: an icon button (emitting `on_toggle`) placed in the toolbar.
pub fn menu_trigger<'a, M: Clone + 'a>(trigger: Icon, on_toggle: M, r: Roles) -> Element<'a, M> {
    icon_button(trigger, type_scale::BODY, r.on_surface, r, Some(on_toggle))
}

/// Float the menu panel over `base`, anchored top-right below the toolbar, with a fade driven
/// by `progress` (0 = hidden, 1 = fully shown). An invisible full-window backdrop beneath the
/// panel emits `on_dismiss` on an outside click, so the menu closes without reflowing any
/// layout. At `progress` 0 the overlay is absent and `base` is returned as-is.
pub fn menu_overlay<'a, M: Clone + 'a>(
    base: Element<'a, M>,
    progress: f32,
    items: Vec<MenuItem<M>>,
    on_dismiss: M,
    r: Roles,
) -> Element<'a, M> {
    if progress <= 0.001 {
        return base;
    }

    let mut list = column![].spacing(spacing::XS).width(Length::Fill);
    for item in items {
        let mut content = row![].spacing(spacing::SM).align_y(Alignment::Center);
        if let Some(glyph) = item.icon {
            content = content.push(icon(glyph, type_scale::BODY, r.on_surface));
        }
        content = content.push(text(item.label).size(type_scale::BODY));
        list = list.push(
            button(content)
                .width(Length::Fill)
                .padding(spacing::SM)
                .style(style::text_button(r))
                .on_press(item.message),
        );
    }

    // Fade the panel box itself (scrim of its own surface color), then anchor it top-right.
    let panel_box = super::fade(
        container(list)
            .padding(spacing::XS)
            .width(Length::Fixed(PANEL_WIDTH))
            .style(style::menu_surface(r)),
        progress,
        style::color(r.surface),
    );
    let panel = container(panel_box)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Right)
        .align_y(iced::alignment::Vertical::Top)
        .padding(iced::Padding {
            top: TOP_OFFSET,
            right: spacing::SM as f32,
            bottom: 0.0,
            left: 0.0,
        });

    // Invisible backdrop that dismisses the menu on any outside click.
    let backdrop = mouse_area(
        container(Space::new(Length::Fill, Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(on_dismiss);

    iced::widget::stack![base, backdrop, panel].into()
}
