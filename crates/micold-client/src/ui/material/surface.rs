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
use micold_core::tokens::{Rgb, Roles};

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
            padding: None,
            width: None,
            height: None,
            center_x: false,
            center_y: false,
        }
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
        let mut widget = container(s.content).style(s.kind.style(s.roles));
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
