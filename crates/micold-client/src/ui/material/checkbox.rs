//! `Checkbox` — the library's wrapper around the rendering stack's checkbox (Principle VIII).
//!
//! Two call sites, both identical, both naming the style function directly. Wrapped for the same
//! reason as the rest: a call site that can reach the style layer can render a checkbox that does
//! not match the other one, and the only thing preventing it is that nobody has yet.
//!
//! Parity: the style resolves to exactly what the call sites use today (FR-005).
//!
//! # Feature 022: the keyboard the stack's checkbox does not have (BUG-003)
//!
//! FR-035 asks every input to answer focus, and this was recorded as impossible for the checkbox:
//! its style is a function of a `Status` with three variants — active, hovered, disabled — so there
//! was no focused state to attach a layer to.
//!
//! That was the *symptom*. The cause is larger and simpler: **the rendering stack's checkbox cannot
//! be focused at all.** Its widget state is the label's shaped paragraph, it implements no focus
//! traversal, and it answers no key. There was no focus to report because there was never any focus
//! — the control was reachable by pointer only, which is an accessibility gap as much as a visual
//! one.
//!
//! So this gives it one, in the smallest thing that can hold it: a wrapper widget that owns the
//! focus, takes it on a press, offers it to the focus traversal, toggles on Space (and only Space —
//! Enter belongs to the dialog, see [`TakesTheKeyboard::update`]), and reports
//! changes so a screen can supply the flag back. The stack's checkbox keeps drawing itself and
//! keeps owning the pointer; nothing about its appearance moved here.
//!
//! Deliberately **not** a reimplementation. `FilledField` owns the field's box because §7.7's
//! geometry could not be composed; nothing here is wrong with the checkbox's geometry, so what is
//! added is the one capability it lacks and no more.

use crate::ui::material::style;
use iced::advanced::widget::{operation, tree, Operation, Tree, Widget};
use iced::advanced::{layout, mouse, overlay, renderer, Clipboard, Layout, Shell};
use iced::widget::checkbox;
use iced::{keyboard, Element, Event, Length, Rectangle, Size, Vector};
use micold_core::tokens::Roles;

/// A labelled checkbox. Builder form (Principle VIII):
/// `Checkbox::new("Enabled", draft.enabled, roles).on_toggle(Message::Toggled).into()`.
///
/// Without an `on_toggle` it renders disabled.
pub struct Checkbox<'a, M> {
    label: String,
    checked: bool,
    roles: Roles,
    on_toggle: Option<Box<dyn Fn(bool) -> M + 'a>>,
    focused: bool,
    on_focus_change: Option<Box<dyn Fn(bool) -> M + 'a>>,
}

impl<'a, M: Clone + 'a> Checkbox<'a, M> {
    /// A checkbox reading `label`, currently `checked`, themed by `roles`.
    pub fn new(label: impl Into<String>, checked: bool, roles: Roles) -> Self {
        Self {
            label: label.into(),
            checked,
            roles,
            on_toggle: None,
            focused: false,
            on_focus_change: None,
        }
    }

    /// The message emitted when the box is toggled, given the new state.
    pub fn on_toggle(mut self, f: impl Fn(bool) -> M + 'a) -> Self {
        self.on_toggle = Some(Box::new(f));
        self
    }

    /// Whether the box holds the keyboard, which shades it with the focused state layer.
    ///
    /// Supplied rather than observed, exactly as [`FormField::active`](super::FormField::active)
    /// is, and for a sharper version of the same reason: the style is resolved when the widget is
    /// *built*, and the thing that knows about focus does not exist until afterwards. What a caller
    /// supplies here comes back from [`Self::on_focus_change`]; setting one without the other is
    /// BUG-003.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// The message emitted when the box takes or loses the keyboard (BUG-003).
    pub fn on_focus_change(mut self, f: impl Fn(bool) -> M + 'a) -> Self {
        self.on_focus_change = Some(Box::new(f));
        self
    }
}

impl<'a, M: Clone + 'a> From<Checkbox<'a, M>> for Element<'a, M> {
    fn from(c: Checkbox<'a, M>) -> Self {
        let mut widget = checkbox(c.checked)
            .label(c.label)
            .style(style::checkbox(c.roles, c.focused));
        // What Space will send, worked out now because the closure is about to be handed to the
        // inner widget. A checkbox has exactly one thing a key can do, so there is one message
        // rather than a second closure.
        let on_key = c.on_toggle.as_ref().map(|f| f(!c.checked));
        if let Some(on_toggle) = c.on_toggle {
            widget = widget.on_toggle(on_toggle);
        }

        TakesTheKeyboard {
            content: widget.into(),
            on_key,
            focused: c.focused,
            on_focus_change: c.on_focus_change,
        }
        .into()
    }
}

/// The focus the rendering stack's checkbox does not have.
///
/// Layout-transparent: it is its child's size, at its child's position, and adds no node. Every
/// method here delegates, and the three that do not are the whole of what it adds — holding the
/// focus, answering the keys, and saying when that changed.
struct TakesTheKeyboard<'a, M> {
    content: Element<'a, M>,
    on_key: Option<M>,
    /// What the application says about this box's focus — see [`Focus::supplied`].
    focused: bool,
    on_focus_change: Option<Box<dyn Fn(bool) -> M + 'a>>,
}

impl<M> TakesTheKeyboard<'_, M> {
    /// Whether the box has anything to toggle. `on_key` is `Some` exactly when `on_toggle` was, and
    /// a checkbox without one renders disabled — so this is the disabled test, read off the one
    /// field that still remembers.
    fn is_enabled(&self) -> bool {
        self.on_key.is_some()
    }
}

/// Whether this holds the keyboard.
///
/// Implements the stack's own focus trait, so a focus traversal moves through the checkbox like any
/// text input — which is what makes this a keyboard fix and not only a paint one. Nothing in the
/// application runs such a traversal today; the control is nonetheless *in* it rather than absent
/// from it, which is the difference between a gap and a decision.
#[derive(Default)]
struct Focus {
    focused: bool,
    /// The application's answer, as of the last frame this saw it (BUG-004).
    ///
    /// Focus is observed here and *held* by the application, which is two copies of one fact. This
    /// is what lets the second one win when they disagree: a screen that takes the keyboard back —
    /// `State::focus_terminal()` clears `focused_field` with no press landing anywhere near this
    /// box — changes the supplied flag, and the box gives the keyboard up rather than drawing
    /// itself at rest while still answering Space.
    ///
    /// A **change** in the supplied flag, not a disagreement with it. A disagreement is also what
    /// an unreported focus looks like — the traversal in [`TakesTheKeyboard::operate`] can take
    /// focus without publishing anything — and undoing that would make this control unreachable by
    /// the very traversal it was joined to.
    supplied: bool,
    /// What was last *published* to the application, so a change can be told from a repetition.
    ///
    /// Not the same question as `supplied`, and that is why it is a third bool rather than a reuse
    /// of the second. `supplied` watches the application; this watches the box. A focus traversal
    /// moves through [`TakesTheKeyboard::operate`], which is not an event and carries no shell —
    /// so the change it makes has no `update` around it to be noticed in, and the before/after
    /// bracket that catches a press cannot see it at all. Comparing against what was last said
    /// catches it on the frame that follows, which is when there is finally a shell to say it to.
    reported: bool,
}

impl operation::Focusable for Focus {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn focus(&mut self) {
        self.focused = true;
    }

    fn unfocus(&mut self) {
        self.focused = false;
    }
}

impl<'a, M: Clone + 'a> Widget<M, iced::Theme, iced::Renderer> for TakesTheKeyboard<'a, M> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        // A rebuild is where the application's answer can be seen changing, and the only place: a
        // frame carries no event of its own, so `update` alone would miss a flag that went true and
        // back between two keystrokes (BUG-004). Nothing has to be deferred here the way the
        // field's does — the focus this reconciles is a bool in this very tree, not something
        // living inside a child that has to be reached through an operation.
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
        // Nothing of its own is drawn. The focused layer is composited into the box's fill by
        // `style::checkbox`, because `checkbox::Style` has a single opaque background and no room
        // for a quad — so the appearance stays in the styling layer with every other appearance,
        // and this stays a widget that only holds a fact.
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style_,
            layout,
            cursor,
            viewport,
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

        // Space, and **only** Space. That is the key a checkbox answers everywhere it exists — the
        // platform convention and WAI-ARIA's — and Enter is deliberately left alone, because Enter
        // belongs to the dialog. Today it reaches `TextField::on_submit`, which is what saves the
        // settings form and confirms both renames; a dialog-level default action is the obvious
        // next thing to add. Either way, toggling is what the box does and committing the form is
        // not its business, so it must not be the thing that answers first.
        //
        // Before the child sees it, and captured: the stack's checkbox answers no key at all, so
        // there is nothing to wait for and nothing else here wants a Space.
        if before {
            if let Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = event {
                if matches!(key, keyboard::Key::Named(keyboard::key::Named::Space)) {
                    if let Some(message) = self.on_key.clone() {
                        shell.publish(message);
                        shell.capture_event();
                    }
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
        // stack's text input follows, so a dialog holding both behaves as one thing rather than as
        // two controls with different ideas about what a click means.
        //
        // Except when there is nothing to toggle: a checkbox with no `on_toggle` renders disabled,
        // and a disabled control that swallows the keyboard is a dead stop in the tab order and a
        // focus ring on something that cannot be operated.
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
            let now = self.is_enabled() && cursor.is_over(layout.bounds());
            let state = tree.state.downcast_mut::<Focus>();
            state.focused = now;
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
        // Offered to the traversal only while it can be operated, for the same reason a press does
        // not focus a disabled one: a tab stop that does nothing is worse than no tab stop.
        if self.is_enabled() {
            operation.focusable(
                None,
                layout.bounds(),
                tree.state.downcast_mut::<Focus>() as &mut dyn operation::Focusable,
            );
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
