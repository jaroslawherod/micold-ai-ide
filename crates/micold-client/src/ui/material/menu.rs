//! `Menu` — a reusable overflow/dropdown menu primitive (Constitution Principle VIII).
//!
//! Both panels that hang from the app bar are this one: the overflow menu and the project
//! switcher's list differ in the items they carry and in nothing else (018 FR-029c). The switcher
//! had its own panel and its own trigger until BUG-007, and every difference between the two was a
//! difference nobody chose — a 260dp panel beside this 240dp one, a 28dp trigger beside a 48dp one,
//! and no exit fade, because the fade is written here.
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

use crate::icons::{icon_role, Icon, IconSurface};
use crate::ui::cdk::overlay::{Anchor, Surface};
use crate::ui::material::glyph::icon;
use crate::ui::material::style;
use crate::ui::material::{menu_panel, IconButton, Text, TypeRole};
use iced::widget::{button, column, opaque, row, Space};
use iced::{Alignment, Element, Length};
use micold_core::overlay::Layer;
use micold_core::tokens::{anatomy, density, motion::duration, shape, spacing, Rgb, Roles};

/// One entry in a menu — a plain record the caller fills in, generic over the message type.
///
/// Everything past the first three fields is here because the **project switcher's rows are these
/// rows** (FR-029b). Its list used to be a hand-built copy of [`item_column`] one module over, and
/// what that cost is BUG-003: §7.5's 48dp item height was applied to this file and the copy stayed
/// at 36, so two panels hanging off the same app bar shipped 12dp apart. A marker, a count, a badge
/// and a right-press are four fields; they are not a second implementation.
pub struct MenuItem<M> {
    /// Optional leading icon, drawn at [`anatomy::menu::ITEM_ICON`].
    pub icon: Option<Icon>,
    /// Hold the leading slot's width even when [`Self::icon`] is `None`, so every label in the
    /// panel starts at the same x (FR-006a of feature 008).
    ///
    /// For a list where the glyph means something *about that row* — the switcher's active marker —
    /// rather than identifying what the row does. A marker that appeared and also moved the label
    /// would be doing two jobs, and the sideways shift is the louder of the two: a quiet indicator
    /// of which project is active reads instead as a list that cannot keep its rows in line. The
    /// type-ahead's rows have always reserved theirs, for the reason stated there.
    pub reserve_icon: bool,
    /// Tint for the leading icon. `None` takes §7.5's `on_surface_variant`; the switcher's active
    /// marker states its own (FR-006 of feature 008).
    pub icon_tint: Option<Rgb>,
    /// The item label.
    pub label: String,
    /// Message emitted when the item is activated. `None` shows the item without making it
    /// pressable — an unavailable project is listed precisely so it can be seen and forgotten.
    pub message: Option<M>,
    /// Trailing supporting text, muted (the switcher's running-session count).
    pub trailing_text: Option<String>,
    /// Trailing badge glyph and its tint (the switcher's unavailable marker).
    pub trailing_icon: Option<(Icon, Rgb)>,
    /// What a right-press on the item becomes, built from the **press point** in window pixels —
    /// the item's own context menu (feature 015; the point since BUG-008, 018 FR-029d).
    ///
    /// `'static` rather than borrowed: the closure captures what the message names (a project's
    /// path, cloned), never the state it was read from.
    pub on_context: Option<Box<dyn Fn((u16, u16)) -> M>>,
}

impl<M> MenuItem<M> {
    /// A labeled item with a leading icon — the common case, and every field a menu itself uses.
    pub fn new(icon: Icon, label: impl Into<String>, message: M) -> Self {
        Self::labeled(label, message).icon(icon)
    }

    /// A labeled item with no leading icon (the terminal's copy/paste menu).
    pub fn labeled(label: impl Into<String>, message: M) -> Self {
        Self {
            icon: None,
            reserve_icon: false,
            icon_tint: None,
            label: label.into(),
            message: Some(message),
            trailing_text: None,
            trailing_icon: None,
            on_context: None,
        }
    }

    /// Give the item a leading icon.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
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
/// The width of the right-click context-menu panel (narrower than the toolbar dropdown).
const CONTEXT_MENU_WIDTH: f32 = 160.0;

/// The tint of an item's leading glyph: whatever the item states, or §7.5's default.
///
/// A function rather than an expression inside the loop so it can be asserted directly
/// (`menu_anatomy`). The tint is not observable from the laid-out tree and rasterising a glyph to
/// read one colour would be a heavy way to ask a one-line question.
pub(super) fn leading_tint<M>(item: &MenuItem<M>, r: Roles) -> Rgb {
    item.icon_tint
        .unwrap_or_else(|| icon_role(IconSurface::MenuItem, r))
}

/// §7.5's panel padding, as a menu wants it: 8dp above the first item and below the last, and
/// nothing at either side, so an item's state layer runs the full width of the panel.
pub(super) fn panel_padding() -> iced::Padding {
    iced::Padding {
        top: anatomy::menu::VERTICAL_PADDING,
        bottom: anatomy::menu::VERTICAL_PADDING,
        left: 0.0,
        right: 0.0,
    }
}

/// The approximate rendered size of a [`MenuOverlay`] panel holding `items` entries, as
/// `(width, height)` in pixels — the input to anchor clamping so a cursor-anchored menu cannot
/// open off-screen (feature 015).
///
/// Derived from the same tokens the panel is built from: [`anatomy::menu::VERTICAL_PADDING`] above
/// the first item and below the last, and [`density::MENU_ITEM_BASE`] per item. Nothing between
/// them — §7.5 gives an item a height and a divider, and gives the space between two items nothing.
///
/// Every figure is read rather than rebuilt. While §7.5's 48dp went unapplied this estimate
/// reproduced the *actual* 36dp by restating the arithmetic that produced it, so it stayed right by
/// tracking the defect — and would have gone wrong the moment the height was fixed.
/// `menu_anatomy::the_clamping_estimate_matches_the_panel_it_estimates` is what now holds the two
/// together, rather than the care of whoever edits one of them.
pub fn menu_panel_size(items: usize) -> (u16, u16) {
    let item = density::MENU_ITEM_BASE as u16;
    let height = anatomy::menu::VERTICAL_PADDING as u16 * 2 + items as u16 * item;
    (PANEL_WIDTH as u16, height)
}

/// The vertical stack of clickable menu entries shared by [`MenuOverlay`] and [`ContextMenu`].
///
/// `pub(super)` for the anatomy-size gate, which needs an item's laid-out height and cannot get it
/// from either public entry point: both yield a `cdk::overlay::Surface` rather than an `Element`,
/// and both wrap the items in a panel whose padding would have to be subtracted back out by hand.
/// Measuring the column directly is the difference between asserting §7.5's figure and asserting
/// arithmetic about it. Same reason `terminal_pane::scrollbar_metrics` is reachable from tests.
pub(super) fn item_column<'a, M: Clone + 'a>(items: Vec<MenuItem<M>>, r: Roles) -> Element<'a, M> {
    // No `spacing`: §7.5 gives an item a height and a divider, and gives the gap between two items
    // nothing, because there is none. The 4dp that used to sit here read as a loose panel while
    // items were 36dp and became 20dp of dead space per five items once they were 48.
    let mut list = column![].width(Length::Fill);
    for item in items {
        // `height(Fill)` is what makes `align_y` mean what it says here. A `Row` centres its
        // children against *each other*, inside the cross size the flex computes from them, and
        // that band is then laid against the top of the node `button` stretched to 48dp — so the
        // label came out 8.4dp high of centre while every other figure in §7.5 was right. Filling
        // the height makes the row's own box the 48dp, and the centring then happens inside it.
        // BUG-001's app bar, one component over: an unstated alignment is not "centred by default",
        // it is "against the top edge" (FR-030a).
        let mut content = row![]
            .spacing(spacing::SM)
            .align_y(Alignment::Center)
            .height(Length::Fill);
        if let Some(glyph) = item.icon {
            // §7.5's 24dp, not the label's size. The glyph used to be `TypeRole::Action.size()` —
            // 14dp — so it took its size from the text beside it rather than from the row that
            // states one, and `anatomy::menu::ITEM_ICON` was read by nothing.
            let tint = leading_tint(&item, r);
            content = content.push(icon(glyph, anatomy::menu::ITEM_ICON, tint));
        } else if item.reserve_icon {
            // The slot without the glyph — same width, same gap after it, so the label lands where
            // a marked row's does (FR-006a). Not `Space::new()` alone: the width has to be §7.5's
            // icon rather than a number that merely matches it today.
            content = content.push(Space::new().width(Length::Fixed(anatomy::menu::ITEM_ICON)));
        }
        // A menu item is something you press, so its label is `Action` — Material's `label_large`,
        // the same 14dp at medium weight. It fills, so trailing content sits at the trailing edge
        // and the item's two 12dp ends are the same 12dp.
        content = content.push(Text::new(item.label, TypeRole::Action, r).width(Length::Fill));
        if let Some(text) = item.trailing_text {
            content = content.push(Text::new(text, TypeRole::Label, r).muted());
        }
        if let Some((glyph, tint)) = item.trailing_icon {
            content = content.push(icon(glyph, TypeRole::Label.size(), tint));
        }
        // Menu items are buttons built here rather than through `material::Button`, so the ripple
        // is composed explicitly — FR-024c wants every interactive surface to ripple, and "it is
        // not a `Button` type" is not a reason a user would accept for one row not responding.
        //
        // §7.5's 48dp height with §7.5's 12dp ends, and its content centred in both — see the row
        // above for why `align_y` alone was not enough to deliver the second of those.
        let mut pressable = button(content)
            .width(Length::Fill)
            .height(Length::Fixed(density::MENU_ITEM_BASE))
            .padding(iced::Padding {
                top: 0.0,
                bottom: 0.0,
                left: anatomy::menu::ITEM_PADDING,
                right: anatomy::menu::ITEM_PADDING,
            })
            .style(style::text_button(r));
        // An item with no message is shown and not pressable — `on_press` is what makes a `button`
        // interactive, so withholding it is the whole of "listed but not selectable".
        let inert = item.message.is_none();
        if let Some(message) = item.message {
            pressable = pressable.on_press(message);
        }
        let entry = super::Ripple::new(pressable, r.on_surface, shape::FULL);
        // `cdk::ContextArea`, not `mouse_area`: the latter's `on_right_press` takes a bare message
        // and drops the press point, which is how a context menu comes to open somewhere other
        // than where it was asked for (BUG-008).
        let entry: Element<'a, M> = match item.on_context {
            Some(build) => crate::ui::cdk::context_area::ContextArea::new(entry)
                .on_secondary_press(build)
                .into(),
            None => entry.into(),
        };
        // …and "not pressable" has to mean the press stops here. A button without `on_press`
        // declines the event, so it went on to whatever was behind the panel — the defect
        // `picker::row_element` carries the full account of (016 BUG-002, FR-035). Same shape here,
        // same answer, so the two kinds of inert row cannot diverge.
        let entry = if inert { opaque(entry) } else { entry };
        list = list.push(entry);
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
const FADE: Duration = Duration::from_millis(duration::SHORT_3);

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
            menu_panel(
                item_column(items, r),
                Length::Fixed(PANEL_WIDTH),
                r,
                true,
                panel_padding(),
            ),
            open,
            FADE,
            super::SurfaceKind::Menu.tone(r),
        )
        // Tone and corners both taken from the panel's own surface kind — a menu is elevation 2,
        // and veiling it with `r.surface` left a rectangle two tones too dark over it while it
        // faded.
        .rounded(super::SurfaceKind::Menu.shape())
        .animate_in();

        let (layer, anchor) = match anchor {
            Some(point) => (Layer::ContextMenu, Anchor::Point(point)),
            // Read, not restated: the bar's own height plus its divider (FR-029a). This was
            // `TOP_OFFSET = 52.0 // approx. toolbar height`, written against a content-sized bar in
            // feature 005 and left alone when §7.1 pinned the bar at 64 — so the panel opened 13dp
            // inside the bar it hangs from, over the trigger it was opened from (BUG-003).
            None => (
                Layer::Popover,
                Anchor::TopEnd {
                    top: anatomy::app_bar::BOTTOM_EDGE,
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
    rising_above: Option<f32>,
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
            rising_above: None,
            on_dismiss,
            roles,
            lifetime: std::marker::PhantomData,
        }
    }

    /// Open **upward**, with the panel's bottom edge `bottom` above the window's bottom edge, at the
    /// press point's x. For a menu opened from a control in a bar pinned to that edge.
    ///
    /// The default hangs the panel *down* from the press point, which `Anchor::Point` says plainly
    /// and leaves the caller to clamp. A bottom bar cannot clamp it: the room below any press in the
    /// bar is at most the bar's own height, and a one-item panel is already taller than that. The
    /// 2026-08-19 visual pass found the terminal tab menu opening into 27px of window with its
    /// single item cut in half, and a second item would have been off-screen entirely — the same
    /// shape as the defect that moved the item into this menu in the first place: present in the
    /// tree, correctly conditioned, impossible to press.
    pub fn rising_above(mut self, bottom: f32) -> Self {
        self.rising_above = Some(bottom);
        self
    }
}

impl<'a, M: Clone + 'a> From<ContextMenu<'a, M>> for Surface<'a, M> {
    fn from(m: ContextMenu<'a, M>) -> Self {
        let ContextMenu {
            items,
            origin,
            rising_above,
            on_dismiss,
            roles: r,
            ..
        } = m;

        let panel = menu_panel(
            item_column(items, r),
            Length::Fixed(CONTEXT_MENU_WIDTH),
            r,
            true,
            panel_padding(),
        );
        let anchor = match rising_above {
            Some(bottom) => Anchor::BottomStart {
                bottom,
                start: origin.0 as f32,
            },
            None => Anchor::Point(iced::Point::new(origin.0 as f32, origin.1 as f32)),
        };
        Surface::new(Layer::ContextMenu, panel, anchor).on_dismiss(on_dismiss)
    }
}
