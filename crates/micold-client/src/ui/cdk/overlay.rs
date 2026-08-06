//! The single floating-surface primitive (feature 017, FR-006, FR-008, FR-009, FR-010).
//!
//! The application grew five of these — the modal, the overflow menu, the context menu, the
//! project-switcher popover and the select dropdown. Each answered the same four questions for
//! itself: where does the panel go, what stops input reaching the window beneath, when does it
//! close, and what happens when two are open at once. Five answers drifted into four different
//! ones, and the fourth question had no answer at all: stacking was whatever fell out of the order
//! `ui::view` happened to wrap them in.
//!
//! One primitive answers all four. Positioning comes from [`Anchor`], input blocking from the
//! backdrop, closing from the render-free core's rules
//! ([`micold_core::overlay::dismisses`]) and stacking from the surface's own
//! [`Layer`] — so reordering the composition cannot reorder the screen.
//!
//! **No appearance.** A scrim arrives already built from the material layer; this module decides
//! *that there is a backdrop* and what it blocks, never what it looks like.

use iced::widget::{container, mouse_area, opaque, stack, Space};
use iced::{Element, Length, Padding, Point};
use micold_core::overlay::{dismisses, stack_order, Layer, Trigger};

/// Where a panel sits in the window.
///
/// Purely geometric: pixel offsets and window edges. The numbers come from the caller, because
/// *how far below the app bar* a menu hangs is a spacing decision and spacing is appearance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Anchor {
    /// The panel's top-left corner sits at this window point. Cursor-anchored context menus; the
    /// caller is responsible for clamping the point so the panel cannot open off-screen.
    Point(Point),
    /// Pinned to the top-right of the window, inset by `top` and `end` pixels. Trigger-attached
    /// popovers that hang below the app bar.
    TopEnd { top: f32, end: f32 },
    /// Centred in the window, horizontally and vertically. Dialogs.
    Center,
    /// Centred horizontally against the window's bottom edge, inset by `bottom` pixels. Snackbars.
    ///
    /// Bottom rather than centred so a notification raised while a dialog is open does not sit over
    /// the dialog's action row — it is above the scrim by band, and out of the way by position.
    BottomCenter { bottom: f32 },
}

/// One floating surface: a panel, where it goes, and how it closes.
///
/// Builder form (Principle VIII): `Surface::new(layer, panel, anchor).scrim(c).on_dismiss(m)`,
/// handed to [`Overlay::push`].
pub struct Surface<'a, M> {
    layer: Layer,
    panel: Element<'a, M>,
    anchor: Anchor,
    scrim: Option<Element<'a, M>>,
    on_dismiss: Option<M>,
    kind: micold_core::overlay::Surface,
}

impl<'a, M: Clone + 'a> Surface<'a, M> {
    /// A `panel` in the given `layer`, positioned by `anchor`.
    ///
    /// The surface kind follows from the layer — a `Dialog` layer is a modal dialog — so a caller
    /// cannot pair dialog stacking with menu dismissal by accident. Override with
    /// [`Self::non_dismissible`] for the rare surface that must resist every implicit close.
    pub fn new(layer: Layer, panel: impl Into<Element<'a, M>>, anchor: Anchor) -> Self {
        Self {
            layer,
            panel: panel.into(),
            anchor,
            scrim: None,
            on_dismiss: None,
            kind: layer.surface(),
        }
    }

    /// Fill the backdrop with an already-built layer from the material side. Without this the
    /// backdrop is invisible but still present — it is what catches an outside click.
    pub fn scrim(mut self, layer: impl Into<Element<'a, M>>) -> Self {
        self.scrim = Some(layer.into());
        self
    }

    /// The message emitted when the user does something that the core's rules say closes this
    /// surface. Without it the surface has no implicit close at all.
    pub fn on_dismiss(mut self, message: M) -> Self {
        self.on_dismiss = Some(message);
        self
    }

    /// Declare the surface non-dismissible: it holds input that an accidental close would destroy,
    /// so nothing implicit closes it and the user must act inside it.
    pub fn non_dismissible(mut self) -> Self {
        self.kind = micold_core::overlay::Surface::NonDismissibleDialog;
        self
    }

    /// Whether `trigger` closes this surface, per the core's rules.
    ///
    /// Exposed because not every trigger is something a widget can observe: an Escape press
    /// arrives through the keyboard subscription, well outside the widget tree. Callers that own
    /// such a trigger consult the same rule rather than re-deciding it.
    pub fn dismisses_on(&self, trigger: Trigger) -> bool {
        dismisses(self.kind, trigger)
    }

    /// The backdrop layer, if this surface has one: the thing that stops input reaching the window
    /// beneath, and that turns an outside click into a dismissal.
    fn backdrop(&mut self) -> Option<Element<'a, M>> {
        let dismisser = self
            .on_dismiss
            .clone()
            .filter(|_| self.dismisses_on(Trigger::OutsideClick));

        // No tint and nothing to dismiss: a layer that neither blocks nor closes is just a wasted
        // stack frame.
        let scrim = self.scrim.take();
        if scrim.is_none() && dismisser.is_none() {
            return None;
        }
        let tinted = scrim.is_some();

        let fill = container(
            scrim.unwrap_or_else(|| Space::new().width(Length::Fill).height(Length::Fill).into()),
        )
        .width(Length::Fill)
        .height(Length::Fill);

        let catcher: Element<'a, M> = match dismisser {
            Some(message) => mouse_area(fill).on_press(message).into(),
            None => fill.into(),
        };

        // A tinted backdrop also blocks: the window beneath is dimmed precisely because it is not
        // available. An invisible backdrop only catches presses, leaving hover and scroll alone.
        Some(if tinted { opaque(catcher) } else { catcher })
    }

    /// The positioned panel layer.
    fn positioned(self) -> Element<'a, M> {
        // Over a dismissing backdrop the panel must swallow its own presses, or a click on the
        // panel's padding falls through and reads as "outside".
        let panel = match self.on_dismiss {
            Some(_) => opaque(self.panel),
            None => self.panel,
        };

        let placed = container(panel).width(Length::Fill).height(Length::Fill);
        match self.anchor {
            Anchor::Center => placed.center_x(Length::Fill).center_y(Length::Fill),
            Anchor::BottomCenter { bottom } => placed
                .center_x(Length::Fill)
                .align_y(iced::alignment::Vertical::Bottom)
                .padding(Padding {
                    bottom,
                    top: 0.0,
                    left: 0.0,
                    right: 0.0,
                }),
            Anchor::Point(point) => placed
                .align_x(iced::alignment::Horizontal::Left)
                .align_y(iced::alignment::Vertical::Top)
                .padding(Padding {
                    top: point.y,
                    left: point.x,
                    right: 0.0,
                    bottom: 0.0,
                }),
            Anchor::TopEnd { top, end } => placed
                .align_x(iced::alignment::Horizontal::Right)
                .align_y(iced::alignment::Vertical::Top)
                .padding(Padding {
                    top,
                    right: end,
                    left: 0.0,
                    bottom: 0.0,
                }),
        }
        .into()
    }
}

/// Floats zero or more [`Surface`]s over a base element.
///
/// Builder form (Principle VIII): `Overlay::new(base).push(surface).push(surface).into()`. With no
/// surfaces pushed it returns `base` untouched, so a closed overlay costs nothing.
pub struct Overlay<'a, M> {
    base: Element<'a, M>,
    surfaces: Vec<Surface<'a, M>>,
}

impl<'a, M: Clone + 'a> Overlay<'a, M> {
    /// An overlay host over `base`, with nothing open.
    pub fn new(base: impl Into<Element<'a, M>>) -> Self {
        Self {
            base: base.into(),
            surfaces: Vec::new(),
        }
    }

    /// Add a floating surface. Where it lands in the z-order is decided by its layer, not by when
    /// it was added — see [`Self::stacking`].
    pub fn push(mut self, surface: Surface<'a, M>) -> Self {
        self.surfaces.push(surface);
        self
    }

    /// Add a surface only when there is one, so callers can chain an `Option` without an `if`.
    pub fn push_maybe(self, surface: Option<Surface<'a, M>>) -> Self {
        match surface {
            Some(surface) => self.push(surface),
            None => self,
        }
    }

    /// The resolved z-order, bottom-to-top.
    ///
    /// Public because it *is* the contract FR-010 states — "given two open surfaces the order is
    /// deterministic and independent of composition order" is only checkable if the order can be
    /// read. It exposes the ordering decision, not the surfaces.
    pub fn stacking(&self) -> Vec<Layer> {
        let keyed: Vec<(Layer, usize)> = self
            .surfaces
            .iter()
            .enumerate()
            .map(|(i, s)| (s.layer, i))
            .collect();
        stack_order(&keyed).into_iter().map(|(l, _)| l).collect()
    }
}

impl<'a, M: Clone + 'a> From<Overlay<'a, M>> for Element<'a, M> {
    fn from(overlay: Overlay<'a, M>) -> Self {
        let Overlay { base, surfaces } = overlay;
        if surfaces.is_empty() {
            return base;
        }

        // Resolve the order first, then move the surfaces out in that order. Indices rather than
        // the surfaces themselves, because ordering is a decision about layers and `stack_order`
        // has no business borrowing a widget tree.
        let keyed: Vec<(Layer, usize)> = surfaces
            .iter()
            .enumerate()
            .map(|(i, s)| (s.layer, i))
            .collect();
        let order: Vec<usize> = stack_order(&keyed).into_iter().map(|(_, i)| i).collect();

        let mut surfaces: Vec<Option<Surface<'a, M>>> = surfaces.into_iter().map(Some).collect();
        let mut layers = vec![base];
        for index in order {
            let mut surface = surfaces[index].take().expect("each surface placed once");
            layers.extend(surface.backdrop());
            layers.push(surface.positioned());
        }
        stack(layers).into()
    }
}
