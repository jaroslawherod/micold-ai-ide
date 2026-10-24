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
//! A key another widget has already captured is not keyboard use of this control either (BUG-016):
//! a clicked terminal-bar tab still holds this focus while the terminal pane takes every key, and
//! answering those drew the tab's ring beside the pane's and re-sent the tab's press.
//!
//! [`state::FOCUS_RING_WIDTH`]: micold_core::tokens::state::FOCUS_RING_WIDTH

use crate::ui::cdk::motion::Progress;
use crate::ui::cdk::passthrough::forward_to_child;
use iced::advanced::widget::{operation, tree, Operation, Tree, Widget};
use iced::advanced::{mouse, renderer, Clipboard, Layout, Renderer as _, Shell};
use iced::touch;
use iced::{keyboard, Background, Border, Color, Element, Event, Rectangle};
use micold_core::tokens::motion::STANDARD;
use micold_core::tokens::{state, Rgb};
use std::time::Duration;

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

/// The parts of a control's bounds the indicator outlines instead of the whole: given the control's
/// size, rectangles relative to its top-left (a link's line fragments inside a paragraph).
pub(super) type Regions = std::rc::Rc<dyn Fn(iced::Size) -> Vec<Rectangle>>;

/// A child, plus the focus it cannot hold for itself.
pub(super) struct TakesTheKeyboard<'a, M> {
    content: Element<'a, M>,
    /// The keys this control answers while focused, and what each sends.
    keys: Vec<(keyboard::key::Named, Option<keyboard::Modifiers>, M)>,
    enabled: bool,
    indicator: Option<Indicator>,
    /// What the application says about this control's focus — see [`Focus::supplied`].
    focused: bool,
    /// Whether the supplied focus is the keyboard's, and so drawn (feature 803, D6): the
    /// application moved the keyboard here, no press or traversal did.
    shown: bool,
    on_focus_change: Option<Box<dyn Fn(bool) -> M + 'a>>,
    /// Told when the focus is held *and shown* changes: keyboard focus only (feature 1121, U1).
    on_shown_change: Option<Box<dyn Fn(bool) -> M + 'a>>,
    /// A stable name, so the application can move the keyboard here (feature 813, R12/R13).
    id: Option<iced::advanced::widget::Id>,
    /// Whether a pointer press on it takes the keyboard (feature 1122, U1): a link clicked on a
    /// card leaves the keyboard where it was.
    press_takes_focus: bool,
    /// Whether the child draws its own hover and pressed layers, which the focus layer tops up;
    /// `false` for a bare child, so the focus layer is the whole of it (feature 1122, U1).
    child_layers: bool,
    /// Outline these parts of the child rather than all of it; `None` outlines the whole.
    regions: Option<Regions>,
    /// How long the indicator takes to fade in or out (feature 1299, D1); `None` draws it at
    /// once, as every control did before.
    fade: Option<Duration>,
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
            shown: false,
            on_focus_change: None,
            on_shown_change: None,
            id: None,
            press_takes_focus: true,
            child_layers: true,
            regions: None,
            fade: None,
        }
    }

    /// Fade the indicator in and out over `over` on the standard curve rather than drawing it at
    /// once (feature 1299, FR-001, FR-002); `None`, the default, is instant.
    pub fn fade(mut self, over: Option<Duration>) -> Self {
        self.fade = over;
        self
    }

    /// Draw the indicator around each rectangle `regions` gives, not around the whole child: one
    /// link in a paragraph, or each line fragment of a link that wraps.
    pub fn regions(mut self, regions: Regions) -> Self {
        self.regions = Some(regions);
        self
    }

    /// Name this control for `focus(id)`.
    pub fn id(mut self, id: iced::advanced::widget::Id) -> Self {
        self.id = Some(id);
        self
    }

    /// A key this control answers while it holds the keyboard, and the message it sends.
    ///
    /// Data rather than a closure so the set is inspectable: which keys a control claims is a
    /// design decision — Space is the checkbox's and Enter is deliberately not — and one that a
    /// test should be able to read off the value rather than infer by pressing everything.
    pub fn key(mut self, key: keyboard::key::Named, message: M) -> Self {
        self.keys.push((key, None, message));
        self
    }

    /// A key together with exactly these modifiers (Shift+F10, feature 972, R5): the same key with
    /// any other modifier set, or none, does not match. Data like [`Self::key`].
    pub fn chord(
        mut self,
        key: keyboard::key::Named,
        modifiers: keyboard::Modifiers,
        message: M,
    ) -> Self {
        self.keys.push((key, Some(modifiers), message));
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

    /// Say the supplied focus is keyboard focus, so the indicator is drawn for it (feature 803,
    /// D6): a row the application gave the keyboard to has had no traversal or key to show it.
    /// Only meaningful with [`Self::on_focus_change`].
    pub fn shown(mut self, shown: bool) -> Self {
        self.shown = shown;
        self
    }

    /// The message emitted when the control takes or loses the keyboard (BUG-003).
    pub fn on_focus_change(mut self, f: impl Fn(bool) -> M + 'a) -> Self {
        self.on_focus_change = Some(Box::new(f));
        self
    }

    /// The message emitted when the control's *shown* focus changes, `focused && visible`: a Tab,
    /// an arrow move or a `focus(id)` hand-back report `true`; a pointer press takes the keyboard
    /// without showing it and reports nothing, and a press on a control whose focus was shown
    /// reports `false` (feature 1121, U1, FR-002).
    pub fn on_shown_change(mut self, f: impl Fn(bool) -> M + 'a) -> Self {
        self.on_shown_change = Some(Box::new(f));
        self
    }

    /// Whether a pointer press on this control takes the keyboard; `true` unless said otherwise
    /// (feature 1122, U1). With `false` a press on it leaves the keyboard where it was, held or
    /// not, and a press elsewhere still drops the keyboard it holds.
    pub fn press_takes_focus(mut self, takes: bool) -> Self {
        self.press_takes_focus = takes;
        self
    }

    /// Say the child draws no hover or pressed layer of its own, so the focus layer is drawn at
    /// full focus opacity under the pointer too (feature 1122, U1: a text link).
    pub fn bare_child(mut self) -> Self {
        self.child_layers = false;
        self
    }

    /// The message `key` sends, if this control claims it.
    fn message_for(&self, key: &keyboard::Key, held: keyboard::Modifiers) -> Option<M> {
        let keyboard::Key::Named(named) = key else {
            return None;
        };
        self.keys
            .iter()
            .find(|(claimed, modifiers, _)| {
                claimed == named && modifiers.is_none_or(|exact| exact == held)
            })
            .map(|(_, _, message)| message.clone())
    }
}

/// Whether this holds the keyboard.
///
/// Implements the stack's own focus trait, so a focus traversal moves through the wrapped control
/// like any text input — which is what makes this a keyboard fix and not only a paint one.
#[derive(Debug, Clone, Copy)]
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
    /// What `on_shown_change` was last told, `focused && visible` as of then (feature 1121, U1):
    /// compared after every event, as `reported` is, so a traversal is seen on the next frame.
    shown_reported: bool,
    /// Whether the child is held down, mirrored from the rendering stack's button, which keeps its
    /// own copy private. The indicator needs it to know which layer the child has already drawn.
    pressed: bool,
    /// How much of the indicator is drawn, `0.0` none to `1.0` full (feature 1299, D1): eased
    /// toward the shown focus when the control fades, held at it when it does not.
    fade: Progress,
}

impl Default for Focus {
    /// Unfocused, and the indicator at none.
    fn default() -> Self {
        Self {
            focused: false,
            supplied: false,
            reported: false,
            visible: false,
            shown_reported: false,
            pressed: false,
            fade: faded(0.0),
        }
    }
}

/// A fade track resting at `at`, on the standard curve (feature 1299, R5).
fn faded(at: f32) -> Progress {
    Progress::new(at).easing(STANDARD.x1, STANDARD.y1, STANDARD.x2, STANDARD.y2)
}

impl Focus {
    /// The opacity of the focus layer drawn over the child at full strength.
    ///
    /// Not §5's focus opacity as it stands. The child has already drawn its own hover or pressed
    /// layer in the same colour, and §5's states are mutually exclusive — "the heaviest single
    /// layer is the measurement and layers never sum" — so this is whatever tops the child's layer
    /// up to the heavier of the two, and nothing when the child's is already heavier (FR-022a).
    /// Laying a full focus layer over it composited 17.2% hovered and 19% pressed (BUG-013).
    ///
    /// `over` is whether the pointer is over the control, which is what the child's own status is
    /// decided from. Worked out whether or not the focus is shown, so a fading indicator can be
    /// drawn at a part of it (feature 1299, D1).
    fn layer(&self, enabled: bool, over: bool) -> f32 {
        let child = match (enabled && over, self.pressed) {
            (false, _) => 0.0,
            (true, false) => state::HOVER,
            (true, true) => state::PRESSED,
        };
        top_up(child, state::FOCUS)
    }

    /// Whether the focus is held and shown: what the indicator is drawn for.
    fn shown(&self) -> bool {
        self.focused && self.visible
    }

    /// How much of the indicator is drawn, `0.0` none to `1.0` full (feature 1299, D1): the
    /// fade's value for a control that `fades`, else the shown focus itself.
    pub(super) fn strength(&self, fades: bool) -> f32 {
        if fades {
            self.fade.value()
        } else {
            f32::from(u8::from(self.shown()))
        }
    }

    /// Whether this holds the keyboard, and whether that is shown.
    ///
    /// The two flags a sliding rail row hands from the form it parks to the form it draws (feature
    /// 030 FR-007), read and written as a pair because handing over only the first would show a ring
    /// a pointer press had hidden (FR-006).
    pub(super) fn held(&self) -> (bool, bool) {
        (self.focused, self.visible)
    }

    /// See [`Self::held`]. The indicator is put straight at what the handed-over flags show: a
    /// ring handed over is the one already drawn, so it does not fade in again (feature 1299, D1).
    pub(super) fn hold(&mut self, focused: bool, visible: bool) {
        self.focused = focused;
        self.visible = visible;
        self.fade.restart_at(f32::from(u8::from(self.shown())));
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
pub(super) fn ring_bounds(bounds: Rectangle) -> Rectangle {
    let inset = state::FOCUS_RING_WIDTH / 2.0;
    Rectangle {
        x: bounds.x + inset,
        y: bounds.y + inset,
        width: (bounds.width - state::FOCUS_RING_WIDTH).max(0.0),
        height: (bounds.height - state::FOCUS_RING_WIDTH).max(0.0),
    }
}

/// The state layer and ring over one rectangle, both at `strength` of their full opacity
/// (feature 1299, D1): `opacity` is the layer's at full.
fn draw_indicator(
    renderer: &mut iced::Renderer,
    indicator: Indicator,
    (opacity, strength): (f32, f32),
    bounds: Rectangle,
) {
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
            a: opacity * strength,
            ..style::color(indicator.layer)
        }),
    );
    // And FR-022's ring on top of it. A quad with a transparent fill is its border.
    renderer.fill_quad(
        renderer::Quad {
            bounds: ring_bounds(bounds),
            border: Border {
                color: Color {
                    a: strength,
                    ..style::color(indicator.outline)
                },
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

impl<'a, M: Clone + 'a> Widget<M, iced::Theme, iced::Renderer> for TakesTheKeyboard<'a, M> {
    forward_to_child!(content, M, iced::Theme, iced::Renderer;
        children, size, layout, mouse_interaction, overlay);

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
                state.focused = self.focused && self.enabled;
                state.visible = state.focused && self.shown;
                // The application is the one that just said this, so saying it back would be an
                // echo — and one that arrives a frame late, after the next `update` compares.
                state.reported = state.focused;
            }
        }
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Focus>()
    }

    fn state(&self) -> tree::State {
        // A wrapper created with the application's flag already true (a row that just appeared)
        // holds it from its first frame, rather than waiting for a change to be seen.
        if self.on_focus_change.is_none() || !self.enabled {
            return tree::State::new(Focus::default());
        }
        let shown = self.focused && self.shown;
        // Built already shown, the indicator is already there: it does not fade into existence.
        tree::State::new(Focus {
            focused: self.focused,
            supplied: self.focused,
            reported: self.focused,
            visible: shown,
            shown_reported: shown,
            pressed: false,
            fade: faded(if shown { 1.0 } else { 0.0 }),
        })
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
        let focus = tree.state.downcast_ref::<Focus>();
        // Without a fade the indicator is the shown focus itself, whatever the track holds.
        let strength = focus.strength(self.fade.is_some());
        if strength <= 0.0 {
            return;
        }
        let opacity = (
            focus.layer(self.enabled, self.child_layers && cursor.is_over(bounds)),
            strength,
        );
        match &self.regions {
            None => draw_indicator(renderer, indicator, opacity, bounds),
            Some(regions) => {
                let origin = bounds.position() - iced::Point::ORIGIN;
                let parts = regions(bounds.size());
                // No part measured: outline the whole rather than say nothing of the focus.
                if parts.is_empty() {
                    draw_indicator(renderer, indicator, opacity, bounds);
                }
                for region in parts {
                    draw_indicator(renderer, indicator, opacity, region + origin);
                }
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
                self.id.as_ref(),
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
        //
        // Unless something earlier in the tree has already taken it. A row or a column hands every
        // event to every child whether or not one captured it, and a click can leave this holding
        // focus while the application gives the keyboard elsewhere: a terminal-bar tab keeps it
        // while the terminal pane laid out above it writes each key to its process. That key is the
        // terminal's, not one this control answers, so it neither re-sends the press nor shows the
        // ring beside the terminal's own (FR-022a, BUG-016).
        if before && !shell.is_event_captured() {
            if let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event {
                if let Some(message) = self.message_for(key, *modifiers) {
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
        // keyboard, though: the pointer is what the user is using (FR-022a). A press on a control
        // told not to take the keyboard on a press moves nothing: the keyboard stays where it was,
        // on this control or elsewhere (feature 1122, U1, FR-005).
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
            let over = cursor.is_over(layout.bounds());
            if self.press_takes_focus || !over {
                let state = tree.state.downcast_mut::<Focus>();
                // A press over this control ripples, and the ripple is the only state layer while
                // it runs: any indicator still drawn here, shown or still fading out, goes at once
                // (feature 1299, R7).
                if over && state.fade.value() > 0.0 {
                    state.fade.restart_at(0.0);
                }
                state.focused = self.enabled && over;
                state.visible = false;
            }
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
        let shown = state.shown();
        if state.shown_reported != shown {
            state.shown_reported = shown;
            if let Some(on_shown_change) = &self.on_shown_change {
                shell.publish(on_shown_change(shown));
            }
        }
        // The indicator follows the shown focus: eased over `fade`, at once without one. A
        // traversal changed it outside any event; the frame that follows retargets it here.
        let target = f32::from(u8::from(shown));
        match self.fade {
            Some(over) => {
                state.fade.on_frame(event, target, over, shell);
            }
            None => state.fade.restart_at(target),
        }
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

    impl Focus {
        /// The focus layer's opacity at full strength, or `None` when the focus is not shown.
        pub(in crate::ui::material) fn indicator(&self, enabled: bool, over: bool) -> Option<f32> {
            self.shown().then(|| self.layer(enabled, over))
        }
    }

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
        // iced strokes a quad's border inside the quad, so the ring's outer edge is this rectangle
        // itself: half the width inside `bounds` on every side, never outside them.
        assert_eq!(ring.x - half, bounds.x);
        assert_eq!(ring.y - half, bounds.y);
        assert_eq!(ring.width + state::FOCUS_RING_WIDTH, bounds.width);
        assert_eq!(ring.height + state::FOCUS_RING_WIDTH, bounds.height);
        assert!(ring.x > bounds.x && ring.x + ring.width < bounds.x + bounds.width);
        assert!(ring.y > bounds.y && ring.y + ring.height < bounds.y + bounds.height);
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

    use super::super::test_support::{roles, Mounted};
    use iced::widget::{container, text};

    /// A wrapper that reports its focus, as the sidebar's rows do: the application supplies the
    /// flag, and `shown` says the focus is the keyboard's and so is drawn (feature 803, D6).
    fn row(supplied: bool, enabled: bool) -> Element<'static, String> {
        let r = roles();
        TakesTheKeyboard::new(container(text("a worktree row")).padding(8), enabled)
            .focused(supplied)
            .shown(supplied)
            .on_focus_change(|focused| format!("focus {focused}"))
            .indicator(Indicator {
                outline: r.secondary,
                layer: r.on_surface,
                radius: 8.0,
            })
            .into()
    }

    fn pixels(mounted: &mut Mounted<'_>) -> Vec<u8> {
        mounted.drawn(style::color(roles().surface)).0
    }

    /// The wrapper is already in the tree when the application's flag turns true: the next build
    /// draws the ring, with no key or pointer event in between.
    #[test]
    fn a_flag_turning_true_on_a_wrapper_already_in_the_tree_draws_the_ring() {
        let mut mounted = Mounted::new(row(false, false));
        let rest = pixels(&mut mounted);
        mounted.element = row(true, true);
        mounted.tree.diff(&mounted.element);
        mounted.relayout();
        assert_ne!(
            pixels(&mut mounted),
            rest,
            "no ring after the flag turned true"
        );
        mounted.element = row(false, false);
        mounted.tree.diff(&mounted.element);
        assert_eq!(
            pixels(&mut mounted),
            rest,
            "the ring stayed after the flag turned false"
        );
    }

    /// A wrapper created with the flag already true (a row that just appeared) draws it on its
    /// first frame: `state()` is seeded from the supplied flag.
    #[test]
    fn a_wrapper_created_with_the_flag_true_draws_the_ring_at_once() {
        let rest = pixels(&mut Mounted::new(row(false, false)));
        let mut fresh = Mounted::new(row(true, true));
        assert_ne!(pixels(&mut fresh), rest);
    }

    /// A disabled wrapper draws nothing and Tab traversal passes it by.
    #[test]
    fn a_disabled_wrapper_draws_nothing_and_is_skipped_by_tab() {
        let rest = pixels(&mut Mounted::new(row(false, false)));
        let mut disabled = Mounted::new(row(true, false));
        assert_eq!(
            pixels(&mut disabled),
            rest,
            "a disabled wrapper drew a ring"
        );
        disabled.focus_next();
        assert_eq!(
            pixels(&mut disabled),
            rest,
            "Tab landed on a disabled wrapper"
        );
    }

    /// Two controls, each publishing `"<name> shown <bool>"` when its shown focus changes.
    fn shown_pair() -> Mounted<'static> {
        let control = |name: &'static str| {
            TakesTheKeyboard::new(container(text(name)).padding(8), true)
                .on_shown_change(move |shown| format!("{name} shown {shown}"))
        };
        Mounted::new(iced::widget::column![control("first"), control("second")])
    }

    /// Feature 1121 T036 (U1): Tab focus publishes `true`; leaving it publishes `false`.
    #[test]
    fn tab_focus_publishes_shown_and_leaving_it_unshown() {
        let mut m = shown_pair();
        m.focus_next();
        assert_eq!(m.redraw(), ["first shown true"]);
        m.focus_next();
        let mut moved = m.redraw();
        moved.sort();
        assert_eq!(moved, ["first shown false", "second shown true"]);
        assert!(m.redraw().is_empty(), "a change is reported once");
    }

    /// U1, FR-002: a pointer press takes the keyboard without showing it, so it publishes
    /// nothing; a press on a keyboard-focused control hides its focus and publishes `false`.
    #[test]
    fn a_press_publishes_no_shown_focus() {
        let mut m = shown_pair();
        let at = m.container().center();
        assert!(m.press(at).is_empty(), "a press shows no focus");
        let mut m = shown_pair();
        m.focus_next();
        assert_eq!(m.redraw(), ["first shown true"]);
        assert_eq!(m.press(at), ["first shown false"]);
    }

    /// Feature 972, R5, FR-009 (T020): a chord publishes on exactly its modifiers; a key claim
    /// (the Menu key) publishes whatever is held; unfocused, nothing publishes.
    #[test]
    fn a_chord_publishes_on_its_modifiers_only_and_a_key_on_any() {
        use super::super::test_support::{key_with, named_key};
        use iced::keyboard::{key::Named, Modifiers};
        let cursor = mouse::Cursor::Unavailable;
        let control = || {
            Mounted::new(
                TakesTheKeyboard::new(container(text("a card")).padding(8), true)
                    .chord(Named::F10, Modifiers::SHIFT, "menu".to_string())
                    .key(Named::ContextMenu, "menu".to_string()),
            )
        };
        let mut m = control();
        assert!(m
            .send(key_with(Named::F10, Modifiers::SHIFT), cursor)
            .is_empty());
        m.focus_next();
        assert_eq!(
            m.send(key_with(Named::F10, Modifiers::SHIFT), cursor),
            ["menu"]
        );
        assert!(m.send(named_key(Named::F10), cursor).is_empty());
        assert!(m
            .send(
                key_with(Named::F10, Modifiers::SHIFT | Modifiers::CTRL),
                cursor
            )
            .is_empty());
        assert_eq!(m.send(named_key(Named::ContextMenu), cursor), ["menu"]);
    }

    /// A control publishing `"<name> focus <bool>"` and answering Enter with `"<name> enter"`.
    fn reporting(name: &'static str, press_takes_focus: bool) -> TakesTheKeyboard<'static, String> {
        TakesTheKeyboard::new(container(text(name)).padding(8), true)
            .key(iced::keyboard::key::Named::Enter, format!("{name} enter"))
            .on_focus_change(move |focused| format!("{name} focus {focused}"))
            .press_takes_focus(press_takes_focus)
    }

    /// Feature 1122 T005 (U1): a press on a control that does not take the keyboard on a press
    /// leaves it without the keyboard and publishes no focus change.
    #[test]
    fn a_press_on_a_control_that_does_not_take_focus_on_press_leaves_it_unfocused() {
        use super::super::test_support::named_key;
        let mut m = Mounted::new(reporting("link", false));
        let at = m.node.bounds().center();
        assert!(m.press(at).is_empty(), "a press published a focus change");
        assert!(
            m.send(
                named_key(iced::keyboard::key::Named::Enter),
                mouse::Cursor::Unavailable
            )
            .is_empty(),
            "Enter reached a control the press did not focus"
        );
    }

    /// Feature 1122 T005: a press elsewhere still drops the keyboard such a control holds by Tab.
    #[test]
    fn a_press_elsewhere_drops_the_keyboard_a_tab_gave_it() {
        let mut m = Mounted::new(iced::widget::column![
            reporting("link", false),
            container(text("elsewhere")).padding(8)
        ]);
        m.focus_next();
        assert_eq!(m.redraw(), ["link focus true"]);
        let elsewhere = m.node.children()[1].bounds().center();
        assert_eq!(m.press(elsewhere), ["link focus false"]);
    }

    /// Feature 1122 T005: by default a press takes the keyboard, as before.
    #[test]
    fn a_press_takes_the_keyboard_by_default() {
        let mut m = Mounted::new(
            TakesTheKeyboard::new(container(text("button")).padding(8), true)
                .on_focus_change(|focused| format!("focus {focused}")),
        );
        let at = m.node.bounds().center();
        assert_eq!(m.press(at), ["focus true"]);
    }

    /// Feature 1122 review A M1 F1 (FR-005): a press on a control that does not take the keyboard
    /// on a press leaves the keyboard a Tab gave it where it was, and its keys still answer.
    #[test]
    fn a_press_on_it_keeps_the_keyboard_a_tab_gave_it() {
        use super::super::test_support::named_key;
        let mut m = Mounted::new(reporting("link", false));
        m.focus_next();
        assert_eq!(m.redraw(), ["link focus true"]);
        let at = m.node.bounds().center();
        assert!(m.press(at).is_empty(), "the press moved the keyboard");
        assert_eq!(
            m.send(
                named_key(iced::keyboard::key::Named::Enter),
                mouse::Cursor::Unavailable
            ),
            ["link enter"]
        );
    }

    // --- Feature 1299: the focus indicator fades (D1) ------------------------------------------

    use super::super::test_support::FRAME_MS;
    use iced::window::RedrawRequest;

    const OVER: Duration = Duration::from_millis(micold_core::tokens::motion::duration::SHORT_3);

    /// A control that fades its indicator over [`OVER`] (or draws it at once, `fade: None`).
    fn fading(name: &'static str, fade: Option<Duration>) -> TakesTheKeyboard<'static, String> {
        let r = roles();
        TakesTheKeyboard::new(container(text(name)).padding(8), true)
            .indicator(Indicator {
                outline: r.secondary,
                layer: r.on_surface,
                radius: 8.0,
            })
            .fade(fade)
    }

    fn faded_pair() -> Mounted<'static> {
        Mounted::new(iced::widget::column![
            fading("first", Some(OVER)),
            fading("second", Some(OVER))
        ])
    }

    use super::super::test_support::{at, strengths, DONE, MIDWAY};

    /// FR-001, FR-002, SC-001, FR-009: Tab onto a faded control fades its indicator in, part-way
    /// at ~75 ms and full by 150 ms + a frame; a press elsewhere fades it out the same way; once
    /// at its end the control asks for no frame.
    #[test]
    fn a_faded_indicator_eases_in_and_out_and_then_asks_for_nothing() {
        let mut m = Mounted::new(iced::widget::column![
            fading("first", Some(OVER)),
            container(text("elsewhere")).padding(8)
        ]);
        m.focus_next();
        let up = m.frames(0, DONE, strengths);
        let mid = at(&up, MIDWAY)[0];
        assert!(0.0 < mid && mid < 1.0, "at ~75 ms the ring is at {mid}");
        assert_eq!(at(&up, DONE)[0], 1.0);
        let end = up
            .iter()
            .position(|(_, s, _)| s[0] == 1.0)
            .expect("it ends");
        assert!(up[..end]
            .iter()
            .all(|(.., r)| *r == RedrawRequest::NextFrame));
        assert!(up[end..].iter().all(|(.., r)| *r == RedrawRequest::Wait));
        let elsewhere = m.node.children()[1].bounds().center();
        m.press(elsewhere);
        let start = DONE + 100;
        let down = m.frames(start, start + DONE, strengths);
        let mid = at(&down, start + MIDWAY)[0];
        assert!(0.0 < mid && mid < 1.0, "at ~75 ms out the ring is at {mid}");
        assert_eq!(at(&down, start + DONE)[0], 0.0);
        assert_eq!(down.last().expect("ticks").2, RedrawRequest::Wait);
    }

    /// FR-004, FR-005: Tab from one faded control to the next fades one out while the other fades
    /// in, on the same ticks; a reversal at ~75 ms turns back from where it was, by no more than a
    /// tick's step, and ends within 150 ms + a frame of it.
    #[test]
    fn a_reversed_fade_turns_back_where_it_is_and_two_run_together() {
        let mut m = faded_pair();
        m.focus_next();
        let up = m.frames(0, MIDWAY, strengths);
        let steepest = up
            .windows(2)
            .map(|w| (w[1].1[0] - w[0].1[0]).abs())
            .fold(0.0, f32::max);
        let reached = at(&up, MIDWAY)[0];
        m.focus_next();
        let start = MIDWAY + FRAME_MS;
        let after = m.frames(start, start + DONE, strengths);
        let next = &after[0].1;
        assert!(
            (next[0] - reached).abs() <= steepest + 1e-4,
            "the first control jumped from {reached} to {}",
            next[0]
        );
        assert!(next[0] < reached, "the first control did not turn back");
        for pair in after.windows(2) {
            let (was, now) = (&pair[0].1, &pair[1].1);
            if now[0] != was[0] || now[1] != was[1] {
                assert!(now[0] <= was[0] && now[1] >= was[1], "{was:?} -> {now:?}");
            }
        }
        let midway = at(&after, start + MIDWAY);
        assert!(
            midway[0] > 0.0 && midway[1] > 0.0 && midway[1] < 1.0,
            "{midway:?}"
        );
        assert_eq!(*at(&after, start + DONE), [0.0, 1.0]);
    }

    /// D1, FR-011: a press on a faded control whose focus is shown removes its indicator at once:
    /// the press ripples, and the ripple is the only state layer.
    #[test]
    fn a_press_on_a_faded_control_drops_its_indicator_at_once() {
        let mut m = Mounted::new(fading("first", Some(OVER)));
        m.focus_next();
        assert_eq!(*at(&m.frames(0, DONE, strengths), DONE), [1.0]);
        m.press(m.node.bounds().center());
        let next = m.frames(DONE + FRAME_MS, DONE + FRAME_MS, strengths);
        assert_eq!(next[0].1, [0.0]);
        assert_eq!(next[0].2, RedrawRequest::Wait);
    }

    /// R7: a press on a faded control whose indicator is still fading out removes what is left of
    /// it at once, so no part-faded ring or layer is drawn under the ripple.
    #[test]
    fn a_press_on_a_control_fading_out_drops_its_indicator_at_once() {
        let mut m = faded_pair();
        m.focus_next();
        assert_eq!(*at(&m.frames(0, DONE, strengths), DONE), [1.0, 0.0]);
        let first = m.node.children()[0].bounds().center();
        let second = m.node.children()[1].bounds().center();
        m.press(second);
        let start = DONE + FRAME_MS;
        let out = m.frames(start, start + MIDWAY, strengths);
        let left = at(&out, start + MIDWAY)[0];
        assert!(0.0 < left && left < 1.0, "midway out the ring is at {left}");
        m.press(first);
        let next = m.frames(
            start + MIDWAY + FRAME_MS,
            start + MIDWAY + FRAME_MS,
            strengths,
        );
        assert_eq!(next[0].1, [0.0, 0.0]);
        assert_eq!(next[0].2, RedrawRequest::Wait);
    }

    /// D1: focus handed over by [`Focus::hold`] draws its indicator at once, not from none.
    #[test]
    fn a_held_focus_is_drawn_at_once() {
        let mut focus = Focus::default();
        focus.hold(true, true);
        assert_eq!(focus.strength(true), 1.0);
        focus.hold(false, false);
        assert_eq!(focus.strength(true), 0.0);
    }

    /// FR-010: a faded control at full strength draws exactly what an unfaded one does.
    #[test]
    fn a_faded_indicator_at_full_draws_todays_indicator() {
        let mut instant = Mounted::new(fading("first", None));
        instant.focus_next();
        instant.redraw();
        let mut faded = Mounted::new(fading("first", Some(OVER)));
        faded.focus_next();
        faded.frames(0, DONE, strengths);
        assert_eq!(pixels(&mut faded), pixels(&mut instant));
    }

    /// R8: a faded wrapper built already focused and shown starts at full and asks for no frame.
    #[test]
    fn a_faded_wrapper_built_focused_starts_at_full() {
        let r = roles();
        let mut m = Mounted::new(
            TakesTheKeyboard::new(container(text("a worktree row")).padding(8), true)
                .focused(true)
                .shown(true)
                .on_focus_change(|focused| format!("focus {focused}"))
                .indicator(Indicator {
                    outline: r.secondary,
                    layer: r.on_surface,
                    radius: 8.0,
                })
                .fade(Some(OVER)),
        );
        assert_eq!(strengths(&m.tree), [1.0]);
        let first = m.frames(0, 0, strengths);
        assert_eq!(first[0].1, [1.0]);
        assert_eq!(first[0].2, RedrawRequest::Wait);
    }
}
