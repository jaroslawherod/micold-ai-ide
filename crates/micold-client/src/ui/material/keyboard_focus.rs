//! `TakesTheKeyboard` — the focus the rendering stack's controls do not have (feature 022 BUG-003,
//! feature 027 FR-030).
//!
//! Only two widgets in the rendering stack can be focused at all: its text input and its text
//! editor. Everything else — button, checkbox, and by extension every control this library builds
//! out of them — holds no focus, joins no traversal and answers no key. Feature 018 recorded that
//! as accepted fidelity gap #2 and left [`state::FOCUS_RING_WIDTH`] in the token module with no
//! caller; feature 022 gave the checkbox a keyboard by wrapping it in a widget that holds the focus
//! the box cannot; feature 027's FR-030 asks for the *whole* of a settings surface to be reachable
//! that way, which the buttons and the selects on it were not.
//!
//! This is that wrapper, with the checkbox's version generalised so a second control can use it
//! rather than grow its own. Layout-transparent: it is its child's size, at its child's position,
//! and adds no node. Every method delegates, and the four that do not are the whole of what it
//! adds — holding the focus, answering the keys, saying when that changed, and drawing the
//! indicator.
//!
//! # Two ways a control can look focused, and why both are here
//!
//! The checkbox composites its focused state layer inside `style::checkbox`, from a flag the
//! *application* supplies and gets back through [`Self::on_focus_change`]. That works because the
//! application tracks a focused field already, and it is the only route available to a control
//! whose fill is a single opaque colour with nowhere to put a quad.
//!
//! A button has no such flag and needs none. Its style is resolved when the element is built, so a
//! supplied flag would mean a `FieldId` at every call site in the application — every list row,
//! every toolbar action, every dialog button — to express something this widget already knows.
//! So for those the indicator is drawn *here*, over the child: §5's focus state layer and FR-022's
//! ring, at the control's own shape radius. One control, one focus, no call site involved.
//!
//! Drawing from outside has two costs, and 018's BUG-013 found both on screen. The child draws its
//! own hover or pressed layer from state this cannot see, so the focus layer is a top-up to the
//! heavier opacity rather than a second layer summed onto the first; and a pointer press moves the
//! keyboard here without showing it, so the ring marks keyboard focus only (FR-022a).
//!
//! [`state::FOCUS_RING_WIDTH`]: micold_core::tokens::state::FOCUS_RING_WIDTH

use iced::advanced::widget::{operation, tree, Operation, Tree, Widget};
use iced::advanced::{layout, mouse, overlay, renderer, Clipboard, Layout, Renderer as _, Shell};
use iced::touch;
use iced::{keyboard, Background, Border, Color, Element, Event, Length, Rectangle, Size, Vector};
use micold_core::tokens::{state, Rgb};

use super::style;

/// What a focused control draws to say so (FR-022, §5).
///
/// Two colours because the indicator is two things: a ring in the scheme's `secondary`, and the
/// state layer beneath it in the control's *own* content colour — a state layer is the content
/// colour over the container, so a filled button's layer is not an outlined one's.
#[derive(Debug, Clone, Copy)]
pub(super) struct Indicator {
    /// The ring's colour.
    pub outline: Rgb,
    /// The colour the state layer is composited from — the control's content colour.
    pub layer: Rgb,
    /// The control's shape radius, so the indicator follows its corners rather than boxing them.
    pub radius: f32,
}

/// A child, plus the focus it cannot hold for itself.
pub(super) struct TakesTheKeyboard<'a, M> {
    content: Element<'a, M>,
    /// The keys this control answers while focused, and what each sends.
    keys: Vec<(keyboard::key::Named, M)>,
    enabled: bool,
    indicator: Option<Indicator>,
    /// What the application says about this control's focus — see [`Focus::supplied`].
    focused: bool,
    on_focus_change: Option<Box<dyn Fn(bool) -> M + 'a>>,
}

impl<'a, M: Clone + 'a> TakesTheKeyboard<'a, M> {
    /// Wrap `content`. A control that is not `enabled` is left out of the traversal entirely: a tab
    /// stop that does nothing is worse than no tab stop, and a focus indicator on something that
    /// cannot be operated says the opposite of what its disabled styling says.
    pub fn new(content: impl Into<Element<'a, M>>, enabled: bool) -> Self {
        Self {
            content: content.into(),
            keys: Vec::new(),
            enabled,
            indicator: None,
            focused: false,
            on_focus_change: None,
        }
    }

    /// A key this control answers while it holds the keyboard, and the message it sends.
    ///
    /// Data rather than a closure so the set is inspectable: which keys a control claims is a
    /// design decision — Space is the checkbox's and Enter is deliberately not — and one that a
    /// test should be able to read off the value rather than infer by pressing everything.
    pub fn key(mut self, key: keyboard::key::Named, message: M) -> Self {
        self.keys.push((key, message));
        self
    }

    /// Draw the focus indicator here, for a control whose own styling cannot carry one.
    pub fn indicator(mut self, indicator: Indicator) -> Self {
        self.indicator = Some(indicator);
        self
    }

    /// Whether the application says this control holds the keyboard — for the controls that report
    /// it back through [`Self::on_focus_change`]. See [`Focus::supplied`].
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// The message emitted when the control takes or loses the keyboard (BUG-003).
    pub fn on_focus_change(mut self, f: impl Fn(bool) -> M + 'a) -> Self {
        self.on_focus_change = Some(Box::new(f));
        self
    }

    /// The message `key` sends, if this control claims it.
    fn message_for(&self, key: &keyboard::Key) -> Option<M> {
        let keyboard::Key::Named(named) = key else {
            return None;
        };
        self.keys
            .iter()
            .find(|(claimed, _)| claimed == named)
            .map(|(_, message)| message.clone())
    }
}

/// Whether this holds the keyboard.
///
/// Implements the stack's own focus trait, so a focus traversal moves through the wrapped control
/// like any text input — which is what makes this a keyboard fix and not only a paint one.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct Focus {
    focused: bool,
    /// The application's answer, as of the last frame this saw it (BUG-004).
    ///
    /// Focus is observed here and *held* by the application, which is two copies of one fact. This
    /// is what lets the second one win when they disagree: a screen that takes the keyboard back —
    /// `State::focus_terminal()` clears `focused_field` with no press landing anywhere near this
    /// control — changes the supplied flag, and the control gives the keyboard up rather than
    /// drawing itself at rest while still answering keys.
    ///
    /// A **change** in the supplied flag, not a disagreement with it. A disagreement is also what
    /// an unreported focus looks like — the traversal in [`TakesTheKeyboard::operate`] can take
    /// focus without publishing anything — and undoing that would make this control unreachable by
    /// the very traversal it was joined to.
    supplied: bool,
    /// What was last *published* to the application, so a change can be told from a repetition.
    ///
    /// Not the same question as `supplied`, and that is why it is a third bool rather than a reuse
    /// of the second. `supplied` watches the application; this watches the control. A focus
    /// traversal moves through [`TakesTheKeyboard::operate`], which is not an event and carries no
    /// shell — so the change it makes has no `update` around it to be noticed in, and the
    /// before/after bracket that catches a press cannot see it at all. Comparing against what was
    /// last said catches it on the frame that follows, which is when there is finally a shell to
    /// say it to.
    reported: bool,
    /// Whether the focus came by keyboard — a traversal, or a key this control answers — and so is
    /// shown (FR-022a, BUG-013).
    ///
    /// A pointer press still takes the keyboard, so Enter and Space act on the button just clicked,
    /// but it clears this: Material marks keyboard focus, and a ring left on the last button clicked
    /// says the keyboard is somewhere the user never sent it.
    visible: bool,
    /// Whether the child is held down, mirrored from the rendering stack's button, which keeps its
    /// own copy private. The indicator needs it to know which layer the child has already drawn.
    pressed: bool,
}

impl Focus {
    /// The opacity of the focus layer drawn over the child, or `None` for no indicator at all.
    ///
    /// Not §5's focus opacity as it stands. The child has already drawn its own hover or pressed
    /// layer in the same colour, and §5's states are mutually exclusive — "the heaviest single
    /// layer is the measurement and layers never sum" — so this is whatever tops the child's layer
    /// up to the heavier of the two, and nothing when the child's is already heavier (FR-022a).
    /// Laying a full focus layer over it composited 17.2% hovered and 19% pressed (BUG-013).
    ///
    /// `over` is whether the pointer is over the control, which is what the child's own status is
    /// decided from.
    pub(super) fn indicator(&self, enabled: bool, over: bool) -> Option<f32> {
        if !(self.focused && self.visible) {
            return None;
        }
        let child = match (enabled && over, self.pressed) {
            (false, _) => 0.0,
            (true, false) => state::HOVER,
            (true, true) => state::PRESSED,
        };
        Some(top_up(child, state::FOCUS))
    }

    /// Whether this holds the keyboard, and whether that is shown.
    ///
    /// The two flags a sliding rail row hands from the form it parks to the form it draws (feature
    /// 030 FR-007), read and written as a pair because handing over only the first would show a ring
    /// a pointer press had hidden (FR-006).
    pub(super) fn held(&self) -> (bool, bool) {
        (self.focused, self.visible)
    }

    /// See [`Self::held`].
    pub(super) fn hold(&mut self, focused: bool, visible: bool) {
        self.focused = focused;
        self.visible = visible;
    }
}

/// The opacity of a layer that, laid over one of `under` in the same colour, leaves `target`.
fn top_up(under: f32, target: f32) -> f32 {
    if under >= target {
        0.0
    } else {
        (target - under) / (1.0 - under)
    }
}

impl operation::Focusable for Focus {
    fn is_focused(&self) -> bool {
        self.focused
    }

    /// Only a traversal calls this — a press sets the flag directly — so focus arriving here is
    /// keyboard focus, and shown.
    fn focus(&mut self) {
        self.focused = true;
        self.visible = true;
    }

    fn unfocus(&mut self) {
        self.focused = false;
        self.visible = false;
    }
}

/// The indicator's two rectangles, given the control's bounds.
///
/// A free function so the geometry can be asserted without a renderer: the ring is drawn *inside*
/// the control rather than around it, because a ring outside these bounds is a ring over whatever
/// is next to them — the rail rows in a settings surface sit `spacing::XS` apart, which is less
/// than the width of two rings.
fn ring_bounds(bounds: Rectangle) -> Rectangle {
    let inset = state::FOCUS_RING_WIDTH / 2.0;
    Rectangle {
        x: bounds.x + inset,
        y: bounds.y + inset,
        width: (bounds.width - state::FOCUS_RING_WIDTH).max(0.0),
        height: (bounds.height - state::FOCUS_RING_WIDTH).max(0.0),
    }
}

impl<'a, M: Clone + 'a> Widget<M, iced::Theme, iced::Renderer> for TakesTheKeyboard<'a, M> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        // A rebuild is where the application's answer can be seen changing, and the only place: a
        // frame carries no event of its own, so `update` alone would miss a flag that went true and
        // back between two keystrokes (BUG-004).
        //
        // Only for the controls that asked to be told. A control drawing its own indicator supplies
        // nothing and must not have its focus reset to `false` on every rebuild — which is every
        // frame, and would undo the traversal that had just landed on it.
        if self.on_focus_change.is_some() {
            let state = tree.state.downcast_mut::<Focus>();
            if state.supplied != self.focused {
                state.supplied = self.focused;
                state.focused = self.focused;
                // The application is the one that just said this, so saying it back would be an
                // echo — and one that arrives a frame late, after the next `update` compares.
                state.reported = self.focused;
            }
        }
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Focus>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Focus::default())
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style_: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style_,
            layout,
            cursor,
            viewport,
        );

        // Nothing of its own unless the control asked for it. A checkbox's focused layer is
        // composited into its fill by `style::checkbox` — drawing a second one here would be the
        // same state said twice, in two tones.
        let Some(indicator) = self.indicator else {
            return;
        };
        let bounds = layout.bounds();
        let Some(opacity) = tree
            .state
            .downcast_ref::<Focus>()
            .indicator(self.enabled, cursor.is_over(bounds))
        else {
            return;
        };
        // §5's state layer: the content colour over the container, topped up to the focus opacity
        // over whatever layer the child drew. Over rather than composited into, because a button's
        // background belongs to the child.
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: Border {
                    radius: indicator.radius.into(),
                    ..Border::default()
                },
                ..renderer::Quad::default()
            },
            Background::Color(Color {
                a: opacity,
                ..style::color(indicator.layer)
            }),
        );
        // And FR-022's ring on top of it. A quad with a transparent fill is its border.
        renderer.fill_quad(
            renderer::Quad {
                bounds: ring_bounds(bounds),
                border: Border {
                    color: style::color(indicator.outline),
                    width: state::FOCUS_RING_WIDTH,
                    radius: (indicator.radius - state::FOCUS_RING_WIDTH / 2.0)
                        .max(0.0)
                        .into(),
                },
                ..renderer::Quad::default()
            },
            Background::Color(Color::TRANSPARENT),
        );
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        let before = tree.state.downcast_ref::<Focus>().focused;

        // Before the child sees it, and captured: nothing this wraps answers a key at all, so there
        // is nothing to wait for and nothing underneath that wanted the press.
        if before {
            if let Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = event {
                if let Some(message) = self.message_for(key) {
                    // Keyboard use, so from here the focus is shown — whatever put it here.
                    tree.state.downcast_mut::<Focus>().visible = true;
                    shell.publish(message);
                    shell.capture_event();
                }
            }
        }

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        // A press takes the keyboard, and a press anywhere else gives it up — the same rule the
        // stack's text input follows, so a surface holding both behaves as one thing rather than as
        // two controls with different ideas about what a click means. It does not *show* the
        // keyboard, though: the pointer is what the user is using (FR-022a).
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
            let now = self.enabled && cursor.is_over(layout.bounds());
            let state = tree.state.downcast_mut::<Focus>();
            state.focused = now;
            state.visible = false;
        }

        // The child's pressed state, by the rendering stack's button's own rule: taken by a press
        // over it, released by any release, lost with the finger.
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) => {
                if self.enabled && cursor.is_over(layout.bounds()) {
                    tree.state.downcast_mut::<Focus>().pressed = true;
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerLifted { .. } | touch::Event::FingerLost { .. }) => {
                tree.state.downcast_mut::<Focus>().pressed = false;
            }
            _ => {}
        }

        // Against what was last reported, not against `before`: focus can also have been taken by
        // a traversal since the previous frame, and that happened outside any event this bracket
        // spans (BUG-004's neighbour, found by the T075 visual pass — tabbing onto a credential
        // opt-in changed not one pixel, and Space then toggled it).
        let state = tree.state.downcast_mut::<Focus>();
        if state.reported != state.focused {
            state.reported = state.focused;
            let now = state.focused;
            if let Some(on_focus_change) = &self.on_focus_change {
                shell.publish(on_focus_change(now));
            }
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        if self.enabled {
            operation.focusable(
                None,
                layout.bounds(),
                tree.state.downcast_mut::<Focus>() as &mut dyn operation::Focusable,
            );
            // The same state again, whole: `Focusable` can say "focused" but not whether that is
            // shown, and a sliding rail row hands both to the form it draws (feature 030 FR-007).
            // Under the same condition, so the n-th state offered is the n-th focusable.
            operation.custom(None, layout.bounds(), tree.state.downcast_mut::<Focus>());
        }
        operation.traverse(&mut |operation| {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout,
                renderer,
                operation,
            );
        });
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, iced::Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, M: Clone + 'a> From<TakesTheKeyboard<'a, M>> for Element<'a, M> {
    fn from(w: TakesTheKeyboard<'a, M>) -> Self {
        Element::new(w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ring is drawn inside the control's own box, so a focused rail row does not paint over
    /// the row above it — they sit `spacing::XS` apart, which is narrower than two rings.
    #[test]
    fn the_ring_stays_inside_the_control() {
        let bounds = Rectangle {
            x: 10.0,
            y: 20.0,
            width: 200.0,
            height: 40.0,
        };
        let ring = ring_bounds(bounds);
        let half = state::FOCUS_RING_WIDTH / 2.0;
        // The ring is stroked centred on its own edge, so its outer edge is this rectangle grown by
        // half the width — which is exactly `bounds`.
        assert_eq!(ring.x - half, bounds.x);
        assert_eq!(ring.y - half, bounds.y);
        assert_eq!(ring.width + state::FOCUS_RING_WIDTH, bounds.width);
        assert_eq!(ring.height + state::FOCUS_RING_WIDTH, bounds.height);
    }

    /// A control smaller than the ring is not a reason to panic or to draw a negative rectangle.
    #[test]
    fn a_control_narrower_than_its_ring_is_representable() {
        let ring = ring_bounds(Rectangle {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        });
        assert_eq!((ring.width, ring.height), (0.0, 0.0));
    }

    /// One layer of `top` laid over another of `under`, both in the same colour — which is what
    /// the indicator and a button's own hover or pressed fill are (`Variant::content`).
    fn over(under: f32, top: f32) -> f32 {
        1.0 - (1.0 - under) * (1.0 - top)
    }

    /// §5's states are mutually exclusive, so a focused control that is also hovered or pressed
    /// shows the heavier of the two layers — never both (FR-022a, BUG-013). The focus layer is
    /// drawn over a child that has already drawn its own, so what it adds is the difference.
    #[test]
    fn the_focus_layer_tops_the_childs_layer_up_rather_than_adding_to_it() {
        let focus = Focus {
            focused: true,
            visible: true,
            ..Focus::default()
        };
        for (over_child, pressed, child) in [
            (false, false, 0.0),
            (true, false, state::HOVER),
            (true, true, state::PRESSED),
        ] {
            let focus = Focus { pressed, ..focus };
            let top = focus
                .indicator(true, over_child)
                .expect("a keyboard-focused control draws its indicator");
            let drawn = over(child, top);
            assert!(
                (drawn - child.max(state::FOCUS)).abs() < 1e-6,
                "child layer {child}: drew {drawn}, §5 allows the heaviest single layer, \
                 {}",
                child.max(state::FOCUS),
            );
        }
    }
}
