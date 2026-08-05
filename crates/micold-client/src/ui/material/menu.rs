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

use std::time::Duration;

use crate::icons::Icon;
use crate::ui::cdk::overlay::{Anchor, Surface};
use crate::ui::material::glyph::icon;
use crate::ui::material::style;
use crate::ui::material::{menu_panel, IconButton, Text, TypeRole};
use iced::widget::{button, column, row};
use iced::{Alignment, Element, Length};
use micold_core::overlay::Layer;
use micold_core::tokens::{shape, spacing, Roles};

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
///
/// Widened from 220 at T017–T021. The panel's own padding, an item's padding, the leading icon and
/// its gap take 46dp, so 220 left a label 174dp — and the longest item, "Session service
/// diagnostics", shaped to 172.8dp. It fit by about a pixel, and only because menu labels were
/// rendering at the body weight; giving them Material's `label_large` (the same size at weight 500,
/// roughly 4% wider) pushed it over and wrapped the item onto two lines, which also silently
/// invalidated [`menu_panel_size`]'s one-line-per-item height estimate and with it the anchor
/// clamping that keeps a cursor-anchored menu on screen.
///
/// 240 leaves the same label about 14dp of headroom. That the old value was one character from
/// wrapping was a latent bug, not something this change introduced — `tests/layout_snapshot.rs` is
/// what surfaced it, and is what will surface the next label that outgrows the panel.
const PANEL_WIDTH: f32 = 240.0;
/// Vertical offset so the panel clears the toolbar (approx. toolbar height).
const TOP_OFFSET: f32 = 52.0;
/// The width of the right-click context-menu panel (narrower than the toolbar dropdown).
const CONTEXT_MENU_WIDTH: f32 = 160.0;

/// The approximate rendered size of a [`MenuOverlay`] panel holding `items` entries, as
/// `(width, height)` in pixels — the input to anchor clamping so a cursor-anchored menu cannot
/// open off-screen (feature 015).
///
/// Derived from the same tokens the panel is built from: the panel's own [`spacing::XS`] padding
/// on both sides, each item's [`spacing::SM`] button padding plus one line of its label, and
/// [`spacing::XS`] between items. Deliberately rounds the line height **up** — erring large keeps
/// the panel comfortably inside the window rather than flush against the edge.
pub fn menu_panel_size(items: usize) -> (u16, u16) {
    // The label's own line height, so the estimate follows the type scale rather than restating
    // it. This used to be the font size plus a guessed 6dp of leading; the role states the real
    // number, which happens to be the same 20dp — an estimate that was right by luck is now right
    // by construction, and follows if the role is ever re-valued.
    let line = TypeRole::Action.line_height_dp().ceil() as u16;
    let item = line + spacing::SM as u16 * 2;
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
            content = content.push(icon(glyph, TypeRole::Action.size(), r.on_surface));
        }
        // A menu item is something you press, so its label is `Action` — Material's `label_large`,
        // the same 14dp at medium weight. It used to state a size and get the default weight.
        content = content.push(Text::new(item.label, TypeRole::Action, r));
        // Menu items are buttons built here rather than through `material::Button`, so the ripple
        // is composed explicitly — FR-024c wants every interactive surface to ripple, and "it is
        // not a `Button` type" is not a reason a user would accept for one row not responding.
        list = list.push(super::Ripple::new(
            button(content)
                .width(Length::Fill)
                .padding(spacing::SM)
                .style(style::text_button(r))
                .on_press(item.message),
            r.on_surface,
            shape::FULL,
        ));
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

/// How long the panel takes to fade in or out.
const FADE: Duration = Duration::from_millis(90);

/// The menu panel, anchored top-right below the toolbar by default, fading itself in and out.
/// Builder form (Principle VIII): `MenuOverlay::new(items, on_dismiss, roles).open(flag).into()`.
///
/// A closed menu still yields a surface, and that is deliberate: the panel has to outlive the
/// state that opened it or there would be nothing left on screen to fade out. It costs an inert,
/// zero-size layer — the panel stops drawing and stops accepting input the moment the fade
/// finishes — and it carries no dismissal while closed, so nothing beneath it is blocked.
pub struct MenuOverlay<'a, M> {
    items: Vec<MenuItem<M>>,
    on_dismiss: M,
    roles: Roles,
    open: bool,
    anchor: Option<iced::Point>,
    lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a, M: Clone + 'a> MenuOverlay<'a, M> {
    /// A menu panel with `items`, dismissing via `on_dismiss`, themed by `roles`. Open by
    /// default — a context menu is built because it was just summoned.
    pub fn new(items: Vec<MenuItem<M>>, on_dismiss: M, roles: Roles) -> Self {
        Self {
            items,
            on_dismiss,
            roles,
            open: true,
            anchor: None,
            lifetime: std::marker::PhantomData,
        }
    }

    /// Whether the menu is open. Going from `true` to `false` plays the fade out.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
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

impl<'a, M: Clone + 'a> From<MenuOverlay<'a, M>> for Surface<'a, M> {
    fn from(m: MenuOverlay<'a, M>) -> Self {
        let MenuOverlay {
            items,
            on_dismiss,
            roles: r,
            open,
            anchor,
            ..
        } = m;

        // Fade the panel box itself (scrim of its own surface colour). Where it lands is the
        // overlay's business; how wide and how padded it is, is this module's.
        let panel = super::fade(
            menu_panel(item_column(items, r), Length::Fixed(PANEL_WIDTH), r, true),
            open,
            FADE,
            r.surface,
        )
        // Rounded to the panel it covers, or the veil squares off the panel's corners for the
        // length of the fade.
        .rounded(super::SurfaceKind::Menu.shape())
        .animate_in();

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
        let surface = Surface::new(layer, panel, anchor);
        // A closed menu carries no dismissal: there is nothing left to close, and a backdrop that
        // outlived the menu would swallow the next click for the length of the fade.
        if open {
            surface.on_dismiss(on_dismiss)
        } else {
            surface
        }
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
