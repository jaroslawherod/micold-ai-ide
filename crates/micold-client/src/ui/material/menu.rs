//! `Menu` — a reusable overflow/dropdown menu primitive (Constitution Principle VIII).
//!
//! Split into a [`MenuTrigger`] (an icon button that lives in the toolbar) and a [`MenuOverlay`]
//! that floats the item panel **above** the rest of the window (Angular-Material `mat-menu` style)
//! — so opening it never reflows the toolbar. Reused for the toolbar's overflow menu; any future
//! dropdown should reuse it.
//!
//! Floating is not this module's job. It builds the panel and says where the panel wants to sit;
//! [`cdk::overlay`](crate::ui::cdk::overlay) does the placing, the input blocking, the dismissal
//! and the z-order (FR-008). What is left here is appearance: the panel's width, its padding, and
//! how far below the app bar it hangs.

use crate::icons::Icon;
use crate::ui::cdk::overlay::{Anchor, Surface};
use crate::ui::material::glyph::icon;
use crate::ui::material::style;
use crate::ui::material::{menu_panel, IconButton};
use iced::widget::{button, column, row, text};
use iced::{Alignment, Element, Length};
use micold_core::overlay::Layer;
use micold_core::tokens::{spacing, type_scale, Roles};

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
/// The width of the right-click context-menu panel (narrower than the toolbar dropdown).
const CONTEXT_MENU_WIDTH: f32 = 160.0;

/// The approximate rendered size of a [`MenuOverlay`] panel holding `items` entries, as
/// `(width, height)` in pixels — the input to anchor clamping so a cursor-anchored menu cannot
/// open off-screen (feature 015).
///
/// Derived from the same tokens the panel is built from: the panel's own [`spacing::XS`] padding
/// on both sides, each item's [`spacing::SM`] button padding plus a [`type_scale::BODY`] line,
/// and [`spacing::XS`] between items. Deliberately rounds the line height **up** — erring large
/// keeps the panel comfortably inside the window rather than flush against the edge.
pub fn menu_panel_size(items: usize) -> (u16, u16) {
    /// Generous line box for a `BODY`-sized label (font size plus leading).
    const LINE: u16 = type_scale::BODY as u16 + 6;
    let item = LINE + spacing::SM as u16 * 2;
    let gaps = (items.saturating_sub(1)) as u16 * spacing::XS as u16;
    let height = spacing::XS as u16 * 2 + items as u16 * item + gaps;
    (PANEL_WIDTH as u16, height)
}

/// The vertical stack of clickable menu entries shared by [`MenuOverlay`] and [`ContextMenu`].
fn item_column<'a, M: Clone + 'a>(items: Vec<MenuItem<M>>, r: Roles) -> Element<'a, M> {
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
    list.into()
}

/// The menu trigger: an icon button (emitting `on_toggle`) placed in the toolbar. Builder form
/// (Principle VIII): `MenuTrigger::new(icon, on_toggle, roles).into()`.
pub struct MenuTrigger<M> {
    icon: Icon,
    on_toggle: M,
    roles: Roles,
}

impl<M> MenuTrigger<M> {
    /// A trigger showing `icon` that emits `on_toggle` when pressed, themed by `roles`.
    pub fn new(icon: Icon, on_toggle: M, roles: Roles) -> Self {
        Self {
            icon,
            on_toggle,
            roles,
        }
    }
}

impl<'a, M: Clone + 'a> From<MenuTrigger<M>> for Element<'a, M> {
    fn from(t: MenuTrigger<M>) -> Self {
        IconButton::new(t.icon, t.roles)
            .on_press(t.on_toggle)
            .into()
    }
}

/// The menu panel, anchored top-right below the toolbar by default, with a fade driven by
/// `progress` (0 = hidden, 1 = fully shown). Builder form (Principle VIII):
/// `MenuOverlay::new(items, on_dismiss, roles).progress(p).into()`, yielding `Option<Surface>` —
/// `None` once the fade has finished, so a closed menu leaves no trace.
pub struct MenuOverlay<'a, M> {
    items: Vec<MenuItem<M>>,
    on_dismiss: M,
    roles: Roles,
    progress: f32,
    anchor: Option<iced::Point>,
    lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a, M: Clone + 'a> MenuOverlay<'a, M> {
    /// A menu panel with `items`, dismissing via `on_dismiss`, themed by `roles`. Fully shown by
    /// default; set [`Self::progress`] to fade.
    pub fn new(items: Vec<MenuItem<M>>, on_dismiss: M, roles: Roles) -> Self {
        Self {
            items,
            on_dismiss,
            roles,
            progress: 1.0,
            anchor: None,
            lifetime: std::marker::PhantomData,
        }
    }

    /// Fade progress (0 = hidden → yields no surface; 1 = fully shown).
    pub fn progress(mut self, progress: f32) -> Self {
        self.progress = progress;
        self
    }

    /// Anchor the panel at a top-left offset (window coordinates) instead of the default
    /// toolbar-relative top-right position (feature 008 context menu). The panel's top-left
    /// corner is placed at `point`.
    ///
    /// A cursor-anchored menu is a context menu, so this also moves it into the context-menu band
    /// — right-clicking a row inside an open popover must not put the row's menu behind it.
    pub fn anchor(mut self, point: iced::Point) -> Self {
        self.anchor = Some(point);
        self
    }
}

impl<'a, M: Clone + 'a> From<MenuOverlay<'a, M>> for Option<Surface<'a, M>> {
    fn from(m: MenuOverlay<'a, M>) -> Self {
        let MenuOverlay {
            items,
            on_dismiss,
            roles: r,
            progress,
            anchor,
            ..
        } = m;
        if progress <= 0.001 {
            return None;
        }

        // Fade the panel box itself (scrim of its own surface colour). Where it lands is the
        // overlay's business; how wide and how padded it is, is this module's.
        let panel = super::fade(
            menu_panel(item_column(items, r), Length::Fixed(PANEL_WIDTH), r, true),
            progress,
            r.surface,
        );

        let (layer, anchor) = match anchor {
            Some(point) => (Layer::ContextMenu, Anchor::Point(point)),
            None => (
                Layer::Popover,
                Anchor::TopEnd {
                    top: TOP_OFFSET,
                    end: spacing::SM,
                },
            ),
        };
        Some(Surface::new(layer, panel, anchor).on_dismiss(on_dismiss))
    }
}

/// A right-click context menu: a small item panel anchored at a pane-local pixel point. Unlike
/// [`MenuOverlay`]'s default (top-right below the toolbar) the panel follows the cursor, and it is
/// narrower. Builder form (Principle VIII):
/// `ContextMenu::new(items, (x, y), on_dismiss, roles).into()`.
pub struct ContextMenu<'a, M> {
    items: Vec<MenuItem<M>>,
    origin: (u16, u16),
    on_dismiss: M,
    roles: Roles,
    lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a, M: Clone + 'a> ContextMenu<'a, M> {
    /// A context menu with `items`, anchored at pane-local pixel point `origin`, dismissing via
    /// `on_dismiss`, themed by `roles`.
    pub fn new(items: Vec<MenuItem<M>>, origin: (u16, u16), on_dismiss: M, roles: Roles) -> Self {
        Self {
            items,
            origin,
            on_dismiss,
            roles,
            lifetime: std::marker::PhantomData,
        }
    }
}

impl<'a, M: Clone + 'a> From<ContextMenu<'a, M>> for Surface<'a, M> {
    fn from(m: ContextMenu<'a, M>) -> Self {
        let ContextMenu {
            items,
            origin,
            on_dismiss,
            roles: r,
            ..
        } = m;

        let panel = menu_panel(
            item_column(items, r),
            Length::Fixed(CONTEXT_MENU_WIDTH),
            r,
            true,
        );
        Surface::new(
            Layer::ContextMenu,
            panel,
            Anchor::Point(iced::Point::new(origin.0 as f32, origin.1 as f32)),
        )
        .on_dismiss(on_dismiss)
    }
}
