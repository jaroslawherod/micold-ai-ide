//! `Surface` — the library's wrapper around a container used as a *surface* (Principle VIII).
//!
//! A container serves two unrelated purposes in this codebase. Most of the time it is layout: pad
//! this, align that, give it a width. Sometimes it is a **surface** — a background with an edge
//! that content sits on, which is a piece of the design system with a colour role, a corner radius
//! and (in feature 018) an elevation.
//!
//! Layout containers stay unwrapped; they have nothing to style (FR-003). This wraps the second
//! kind, so that when 018 gives surfaces elevation, "which containers are surfaces?" is answered by
//! the type rather than by reading every `.style(...)` in the tree.
//!
//! Parity: each kind resolves to exactly the style function its call sites use today (FR-005).

use crate::app::NoticeLevel;
use crate::ui::material::style;
use iced::widget::container;
use iced::{Element, Length, Padding};
use micold_core::tokens::{anatomy, elevation, shape, Rgb, Roles};

/// A boxed container style function — each `impl Fn` from the style layer is a distinct opaque
/// type, so the kinds are boxed behind one signature to be chosen at runtime.
type ContainerStyleFn = Box<dyn Fn(&iced::Theme) -> container::Style>;

/// Which surface this is. The distinction is about *role in the interface*, not about colour:
/// two kinds may resolve to the same colour today and diverge in 018 without a call site changing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    /// The window's own background, behind everything.
    Window,
    /// A plain raised surface — the default for grouped content.
    Plain,
    /// A modal dialog's box.
    Dialog,
    /// The worktree sidebar.
    Sidebar,
    /// An app bar or a pane's toolbar.
    Toolbar,
    /// A floating menu or popover panel.
    Menu,
    /// A row in a list, distinguished from the surface it sits on.
    ListItem,
    /// A notification banner at the given severity.
    Notification(NoticeLevel),
    /// A tag chip, tinted with its own accent.
    Chip(Rgb),
}

impl Kind {
    /// The elevation level this kind sits at, from contract §4's assignment table.
    ///
    /// A property of *what the surface is*, not of the call site — a dialog is at level 3 because
    /// it is a dialog. `.elevation()` exists for the rare surface that needs to differ, not so each
    /// call site can pick its own depth.
    fn elevation(self) -> Option<u8> {
        match self {
            Kind::Window => Some(elevation::PAGE),
            Kind::Plain => Some(elevation::CARD),
            Kind::Dialog => Some(elevation::DIALOG),
            Kind::Sidebar => Some(elevation::CARD),
            Kind::Toolbar => Some(elevation::APP_BAR_REST),
            Kind::Menu => Some(elevation::MENU),
            // Colour-driven rather than elevation-driven: a list row is distinguished by its own
            // fill, a notification by severity and a chip by its accent. Overriding elevation on
            // these would replace the fill that identifies them, so they have no default level and
            // `.elevation()` is refused for them below.
            Kind::ListItem | Kind::Notification(_) | Kind::Chip(_) => None,
        }
    }

    /// The tone this kind actually paints, for a wrapper that has to composite against it.
    ///
    /// Public for the same reason as [`Self::shape`], and it closes the same class of bug from the
    /// other side. `fade` has no true opacity to work with — the rendering stack offers none — so it
    /// composites a veil of *the surface behind the content* over it. A veil in the wrong tone is a
    /// visible rectangle for the length of every transition, and three call sites were passing
    /// `roles.surface` while covering something else entirely: the sidebar is elevation 1
    /// (`surface_container_low`), a menu panel elevation 2, a dialog elevation 3. On the sidebar
    /// that is tone 6 painted over tone 10 — four tones too dark, and plainly visible as a block
    /// around the row actions while they fade.
    ///
    /// Asking the surface is the difference between one answer and one guess per call site.
    pub fn tone(self, roles: Roles) -> Rgb {
        match self.elevation() {
            Some(level) => roles.elevation_surface(level),
            // Colour-driven kinds are identified by their own fill rather than by depth, and that
            // fill is not a surface tone. What sits *behind* one of these is the surface it was
            // placed on, which is what a compositing wrapper needs — a row fading out reveals the
            // list beneath it, not the row's own colour.
            None => roles.surface,
        }
    }

    /// The corner size this kind takes, from contract §3.
    ///
    /// Public so a wrapper that has to match this surface's shape can ask instead of restating it.
    /// `Ripple` is the case: it clips its circle to the shape of whatever it wraps, and a wrapper
    /// cannot see the radius its child draws with — asking is the difference between the two
    /// staying in step and drifting the moment a kind is re-shaped.
    pub fn shape(self) -> f32 {
        match self {
            Kind::Window | Kind::Sidebar | Kind::Toolbar => shape::NONE,
            Kind::Plain | Kind::ListItem | Kind::Notification(_) => shape::MEDIUM,
            Kind::Dialog => shape::EXTRA_LARGE,
            Kind::Menu => shape::EXTRA_SMALL,
            Kind::Chip(_) => shape::FULL,
        }
    }

    fn style(self, roles: Roles) -> ContainerStyleFn {
        match self {
            Kind::Window => Box::new(style::window_bg(roles)),
            Kind::Plain => Box::new(style::surface(roles)),
            Kind::Dialog => Box::new(style::dialog(roles)),
            Kind::Sidebar => Box::new(style::sidebar_surface(roles)),
            Kind::Toolbar => Box::new(style::toolbar_surface(roles)),
            Kind::Menu => Box::new(style::menu_surface(roles)),
            Kind::ListItem => Box::new(style::list_item(roles)),
            Kind::Notification(level) => Box::new(style::notification(roles, level)),
            Kind::Chip(accent) => Box::new(style::chip(accent)),
        }
    }
}

/// Content on a design-system surface. Builder form (Principle VIII):
/// `Surface::new(content, Kind::Dialog, roles).padding(spacing::LG).width(420.0).into()`.
pub struct Surface<'a, M> {
    content: Element<'a, M>,
    kind: Kind,
    roles: Roles,
    elevation: Option<u8>,
    corner: Option<f32>,
    padding: Option<Padding>,
    width: Option<Length>,
    height: Option<Length>,
    center_x: bool,
    center_y: bool,
}

impl<'a, M: 'a> Surface<'a, M> {
    /// `content` on a surface of the given `kind`, themed by `roles`.
    pub fn new(content: impl Into<Element<'a, M>>, kind: Kind, roles: Roles) -> Self {
        Self {
            content: content.into(),
            kind,
            roles,
            elevation: None,
            corner: None,
            padding: None,
            width: None,
            height: None,
            center_x: false,
            center_y: false,
        }
    }

    /// Raise this surface to an explicit elevation level, overriding its kind's own.
    ///
    /// For the surface whose depth genuinely differs from its kind — an app bar that elevates once
    /// content scrolls under it (FR-025a) is the motivating case. Not a per-call-site depth dial:
    /// a kind that always sits at a level states it in [`Kind::elevation`] instead.
    ///
    /// # Panics
    ///
    /// On a colour-driven kind (`ListItem`, `Notification`, `Chip`), whose fill identifies it —
    /// applying an elevation tone would replace exactly the colour the call site asked for. Failing
    /// loudly beats silently discarding it.
    pub fn elevation(mut self, level: u8) -> Self {
        assert!(
            self.kind.elevation().is_some(),
            "{:?} is a colour-driven surface: its fill is what identifies it, so an elevation tone \
             would overwrite the very thing the call site chose",
            self.kind
        );
        self.elevation = Some(level);
        self
    }

    /// Give this surface an explicit corner size, overriding its kind's own.
    ///
    /// The argument is a value from the shape scale (`tokens::shape`), never a literal — the scale
    /// is what makes "one size larger" mean something.
    pub fn shape(mut self, corner: f32) -> Self {
        self.corner = Some(corner);
        self
    }

    /// Inset the content from the surface's edge.
    ///
    /// **A parity affordance.** Padding is a property of a surface kind, and in feature 018 each
    /// kind gets one from the shape and density scales. Until then the call sites differ and
    /// reproducing them exactly is what keeps this feature reviewable.
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = Some(padding.into());
        self
    }

    /// Lay the surface out at a given width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Lay the surface out at a given height.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// Centre the content horizontally within the surface.
    pub fn center_x(mut self) -> Self {
        self.center_x = true;
        self
    }

    /// Centre the content vertically within the surface.
    pub fn center_y(mut self) -> Self {
        self.center_y = true;
        self
    }
}

impl<'a, M: 'a> From<Surface<'a, M>> for Element<'a, M> {
    fn from(s: Surface<'a, M>) -> Self {
        // An override composes the elevation scale directly; otherwise the kind's own style
        // function applies, which is what keeps every existing call site byte-identical.
        let style: ContainerStyleFn = match (s.elevation, s.corner) {
            (None, None) => s.kind.style(s.roles),
            (level, corner) => Box::new(style::elevated(
                s.roles,
                level.or_else(|| s.kind.elevation()).unwrap_or(0),
                corner.unwrap_or_else(|| s.kind.shape()),
            )),
        };
        let mut widget = container(s.content).style(style);
        // §7.4's width bounds, applied by the *kind* rather than by each dialog. A dialog narrower
        // than 280dp reads as a tooltip; one that stretches to a wide window puts its action row
        // an inch from its title. Neither is something a call site should have to remember, and
        // the seven that build dialogs were each free to forget it.
        if matches!(s.kind, Kind::Dialog) {
            // Only the ceiling: the rendering stack's container has `max_width` and no matching
            // floor, and the *floor* is the one a dialog reaches on its own — its content is a
            // title and an action row, both wider than 280dp in every dialog this application has.
            // Recorded rather than silently dropped: `anatomy::dialog::MIN_WIDTH` is asserted
            // against the contract in `tokens_anatomy.rs`, so the number stays true even though
            // nothing applies it here.
            widget = widget.max_width(anatomy::dialog::MAX_WIDTH);
        }
        if let Some(padding) = s.padding {
            widget = widget.padding(padding);
        }
        if let Some(width) = s.width {
            widget = widget.width(width);
        }
        if let Some(height) = s.height {
            widget = widget.height(height);
        }
        // Applied after width/height so an explicit size still wins — `center_x` sets a width of
        // its own, which would otherwise silently override the caller's.
        if s.center_x {
            widget = widget.align_x(iced::alignment::Horizontal::Center);
        }
        if s.center_y {
            widget = widget.align_y(iced::alignment::Vertical::Center);
        }
        widget.into()
    }
}

#[cfg(test)]
mod tone_tests {
    use super::*;
    use micold_core::theme::ColorScheme;

    fn roles(scheme: ColorScheme) -> Roles {
        micold_core::tokens::roles(scheme)
    }

    /// Every elevation-driven kind reports the tone its own level paints.
    ///
    /// The property, rather than nine transcribed colours: a wrapper compositing against a surface
    /// must get *that* surface's tone, and the elevation table is where that already lives.
    #[test]
    fn each_kind_reports_the_tone_its_elevation_paints() {
        for scheme in [ColorScheme::Light, ColorScheme::Dark] {
            let r = roles(scheme);
            for (kind, level) in [
                (Kind::Window, elevation::PAGE),
                (Kind::Plain, elevation::CARD),
                (Kind::Sidebar, elevation::CARD),
                (Kind::Toolbar, elevation::APP_BAR_REST),
                (Kind::Menu, elevation::MENU),
                (Kind::Dialog, elevation::DIALOG),
            ] {
                assert_eq!(
                    kind.tone(r),
                    r.elevation_surface(level),
                    "{kind:?} in {scheme:?} reports a tone its own elevation does not paint"
                );
            }
        }
    }

    /// The three kinds that were being veiled with `roles.surface` do not paint `roles.surface`.
    ///
    /// This is the regression, stated as the thing that was false. A screen recording of the
    /// sidebar's row actions showed the sidebar case: `surface` is neutral tone 6 in dark and the
    /// sidebar is tone 10, so the fade painted a rectangle four tones too dark around the icons for
    /// the length of every reveal. The dialog and the menu had the same mistake and are less
    /// obvious only because they sit over a scrim.
    #[test]
    fn the_veiled_surfaces_are_not_the_plain_surface_tone() {
        for scheme in [ColorScheme::Light, ColorScheme::Dark] {
            let r = roles(scheme);
            for kind in [Kind::Sidebar, Kind::Menu, Kind::Dialog] {
                assert_ne!(
                    kind.tone(r),
                    r.surface,
                    "{kind:?} in {scheme:?} paints the plain `surface` tone, so veiling it with \
                     `roles.surface` would be correct — if that ever becomes true, the call sites \
                     that now ask `tone()` are carrying a guess again"
                );
            }
        }
    }

    /// A kind's tone and its shape answer for the same surface, so a wrapper that asks for both
    /// gets a consistent pair. Cheap, and it is the pairing every compositing wrapper relies on.
    #[test]
    fn tone_and_shape_are_available_for_every_kind() {
        let r = roles(ColorScheme::Dark);
        for kind in [
            Kind::Window,
            Kind::Plain,
            Kind::Dialog,
            Kind::Sidebar,
            Kind::Toolbar,
            Kind::Menu,
            Kind::ListItem,
        ] {
            let _ = kind.tone(r);
            assert!(
                kind.shape() >= 0.0,
                "{kind:?} reports a negative corner size"
            );
        }
    }
}
