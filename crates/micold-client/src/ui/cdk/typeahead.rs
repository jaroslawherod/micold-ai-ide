//! The type-ahead's behaviour half (feature 021, FR-018).
//!
//! A text field whose result list floats beneath it, anchored to the field's own on-screen bounds
//! rather than to the window — which is what lets it work inside the add-worktree dialog, a
//! content-sized box where a window-level surface has nothing to anchor against. The list is
//! returned from `Widget::overlay()`, so the rendering stack positions it and no other field in
//! the form moves when the row count changes.
//!
//! **It decides nothing.** Key events are translated into [`micold_core::typeahead::Key`], the
//! answer comes back from [`intent_for`], and this module applies it. Where the list sits, what it
//! captures and when it closes are behaviour; what any of it looks like arrives already resolved
//! from `material::typeahead`.
//!
//! This is the one module in `cdk/` besides `overlay` that floats its own content, and
//! `tests/one_overlay_implementation.rs` holds it to saying why.
//!
//! Contract: `specs/021-branch-typeahead-search/contracts/typeahead-component.md` §3.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{mouse, overlay, renderer, Clipboard, Shell, Widget};
use iced::{keyboard, Element, Event, Length, Point, Rectangle, Size, Vector};
use micold_core::typeahead::{intent_for, Direction, Intent, Key};

/// A field with a floating list of results beneath it.
///
/// Composed rather than drawn: both halves arrive as already-built elements, so this module never
/// learns what either looks like.
pub struct Typeahead<'a, M, Theme = iced::Theme, Renderer = iced::Renderer> {
    field: Element<'a, M, Theme, Renderer>,
    menu: Element<'a, M, Theme, Renderer>,
    open: bool,
    highlight: Option<usize>,
    rows: usize,
    highlighted_enabled: bool,
    gap: f32,
    on_move: Option<Box<dyn Fn(Direction) -> M + 'a>>,
    on_pick: Option<M>,
    on_dismiss: Option<M>,
    on_focus: Option<M>,
}

impl<'a, M: Clone + 'a, Theme, Renderer> Typeahead<'a, M, Theme, Renderer> {
    /// A `field`, and the `menu` shown beneath it while `open`.
    ///
    /// `gap` is the distance between the two, in pixels. It arrives from the caller because how far
    /// apart they sit is a spacing decision, and spacing is appearance.
    pub fn new(
        field: impl Into<Element<'a, M, Theme, Renderer>>,
        menu: impl Into<Element<'a, M, Theme, Renderer>>,
        open: bool,
        gap: f32,
    ) -> Self {
        Self {
            field: field.into(),
            menu: menu.into(),
            open,
            highlight: None,
            rows: 0,
            highlighted_enabled: false,
            gap,
            on_move: None,
            on_pick: None,
            on_dismiss: None,
            on_focus: None,
        }
    }

    /// Where the keyboard is, how many rows it can reach, and whether the one it is on can be
    /// chosen — the three inputs the keyboard rule reads.
    pub fn keyboard(
        mut self,
        highlight: Option<usize>,
        rows: usize,
        highlighted_enabled: bool,
    ) -> Self {
        self.highlight = highlight;
        self.rows = rows;
        self.highlighted_enabled = highlighted_enabled;
        self
    }

    /// Emitted when the highlight should move.
    pub fn on_move(mut self, f: impl Fn(Direction) -> M + 'a) -> Self {
        self.on_move = Some(Box::new(f));
        self
    }

    /// Emitted when the highlighted row should be taken.
    pub fn on_pick(mut self, message: M) -> Self {
        self.on_pick = Some(message);
        self
    }

    /// Emitted when the list should close without taking anything.
    pub fn on_dismiss(mut self, message: M) -> Self {
        self.on_dismiss = Some(message);
        self
    }

    /// Emitted when the field is reached, so a closed list can open.
    pub fn on_focus(mut self, message: M) -> Self {
        self.on_focus = Some(message);
        self
    }
}

impl<M, Theme, Renderer> Widget<M, Theme, Renderer> for Typeahead<'_, M, Theme, Renderer>
where
    M: Clone,
    Renderer: renderer::Renderer,
{
    /// Two children, always — the menu keeps its widget state across a close and reopen, which is
    /// what stops a scrolled list from forgetting where it was.
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.field), Tree::new(&self.menu)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.field, &self.menu]);
    }

    fn size(&self) -> Size<Length> {
        self.field.as_widget().size()
    }

    /// Only the field is laid out. The menu never occupies space in the form — that is the whole
    /// point of anchoring it in an overlay (FR-001a).
    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.field
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        // Reaching the field is a press on it, not a focus event: the rendering stack's text input
        // keeps focus to itself and publishes nothing when it gains it, so a press inside the
        // field's own bounds is the signal available here. Only while closed — a press on an
        // already-open field is an ordinary click in the text.
        if !self.open && cursor.is_over(layout.bounds()) {
            if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
                if let Some(message) = self.on_focus.clone() {
                    shell.publish(message);
                }
            }
        }

        self.field.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.field.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.field.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.field
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    /// The list, floated beneath the field.
    ///
    /// Handing it to the rendering stack's own overlay system is what makes it independent of the
    /// parent layout's size constraints — the same mechanism every `pick_list` and tooltip relies
    /// on, and the only one that works inside a content-sized dialog.
    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        _viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, Theme, Renderer>> {
        if !self.open {
            return None;
        }

        let bounds = layout.bounds();
        let field = Rectangle {
            x: bounds.x + translation.x,
            y: bounds.y + translation.y,
            ..bounds
        };

        let (menu, state) = (&mut self.menu, &mut tree.children[1]);
        Some(overlay::Element::new(Box::new(Menu {
            content: menu,
            state,
            field,
            gap: self.gap,
            on_dismiss: self.on_dismiss.clone(),
            keys: Keys {
                highlight: self.highlight,
                rows: self.rows,
                highlighted_enabled: self.highlighted_enabled,
                on_move: self.on_move.as_deref(),
                on_pick: self.on_pick.clone(),
                on_dismiss: self.on_dismiss.clone(),
            },
        })))
    }
}

/// The keyboard's inputs and outputs, carried into the overlay so the rule is applied where the
/// events actually arrive — overlays see them before the widget tree does, which is what lets the
/// list claim its four keys and leave every other one to the field.
struct Keys<'a, M> {
    highlight: Option<usize>,
    rows: usize,
    highlighted_enabled: bool,
    on_move: Option<&'a dyn Fn(Direction) -> M>,
    on_pick: Option<M>,
    on_dismiss: Option<M>,
}

impl<M: Clone> Keys<'_, M> {
    /// What this key press should publish, and whether the list has claimed the key.
    ///
    /// The two answers come apart for exactly one key. Tab dismisses the list *and* belongs to the
    /// field, because what it is doing is moving focus out — swallowing it would leave the developer
    /// unable to tab past the picker at all. Every other key the rule names is the list's alone:
    /// capture is what stops Enter also submitting the dialog behind it.
    fn message_for(&self, key: &keyboard::Key) -> Option<(M, Claim)> {
        let key = match key {
            keyboard::Key::Named(keyboard::key::Named::ArrowDown) => Key::Down,
            keyboard::Key::Named(keyboard::key::Named::ArrowUp) => Key::Up,
            keyboard::Key::Named(keyboard::key::Named::Enter) => Key::Enter,
            keyboard::Key::Named(keyboard::key::Named::Escape) => Key::Escape,
            keyboard::Key::Named(keyboard::key::Named::Tab) => Key::Tab,
            _ => Key::Other,
        };

        let message = match intent_for(key, self.highlight, self.rows, self.highlighted_enabled)? {
            Intent::Move(direction) => self.on_move.map(|f| f(direction)),
            Intent::Pick => self.on_pick.clone(),
            Intent::Dismiss => self.on_dismiss.clone(),
        }?;

        let claim = match key {
            Key::Tab => Claim::PassOn,
            _ => Claim::Taken,
        };
        Some((message, claim))
    }
}

/// Whether the list has taken a key or is only reacting to one on its way past.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Claim {
    /// The list handled it; nothing else should see it.
    Taken,
    /// The list reacted, but the key still belongs to whatever is behind it.
    PassOn,
}

/// The floating list itself: where it sits, what it captures, and when it closes.
struct Menu<'a, 'b, M, Theme, Renderer> {
    content: &'b mut Element<'a, M, Theme, Renderer>,
    state: &'b mut Tree,
    /// The field's on-screen rectangle — everything below is measured from it.
    field: Rectangle,
    gap: f32,
    on_dismiss: Option<M>,
    keys: Keys<'b, M>,
}

impl<M, Theme, Renderer> overlay::Overlay<M, Theme, Renderer> for Menu<'_, '_, M, Theme, Renderer>
where
    M: Clone,
    Renderer: renderer::Renderer,
{
    /// Directly beneath the field and exactly as wide, flipping above it when there is no room
    /// below — a list that opened off the bottom of the window would be worse than no list.
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let below = bounds.height - (self.field.y + self.field.height + self.gap);
        let above = self.field.y - self.gap;
        let room = below.max(above).max(0.0);

        let limits = layout::Limits::new(Size::ZERO, Size::new(self.field.width, room))
            .width(self.field.width);
        let node = self
            .content
            .as_widget_mut()
            .layout(self.state, renderer, &limits);

        let height = node.size().height;
        let y = if height <= below || below >= above {
            self.field.y + self.field.height + self.gap
        } else {
            (self.field.y - self.gap - height).max(0.0)
        };

        node.move_to(Point::new(self.field.x, y))
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.content.as_widget().draw(
            self.state,
            renderer,
            theme,
            style,
            layout,
            cursor,
            &layout.bounds(),
        );
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
    ) {
        // The keyboard first, and only the keys the rule claims. Capturing the event is what stops
        // Enter also submitting the dialog behind the list.
        if let Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = event {
            if let Some((message, claim)) = self.keys.message_for(key) {
                shell.publish(message);
                if claim == Claim::Taken {
                    shell.capture_event();
                    return;
                }
            }
        }

        // A press outside both the list and the field closes it. Inside the field it is an ordinary
        // click on a focused input, which must not dismiss what the developer is looking at.
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
            let outside = cursor
                .position()
                .map(|p| !layout.bounds().contains(p) && !self.field.contains(p))
                .unwrap_or(false);
            if outside {
                if let Some(message) = self.on_dismiss.clone() {
                    shell.publish(message);
                    shell.capture_event();
                    return;
                }
            }
        }

        self.content.as_widget_mut().update(
            self.state,
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            &layout.bounds(),
        );
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            self.state,
            layout,
            cursor,
            &layout.bounds(),
            renderer,
        )
    }

    fn operate(&mut self, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        self.content
            .as_widget_mut()
            .operate(self.state, layout, renderer, operation);
    }
}

impl<'a, M, Theme, Renderer> From<Typeahead<'a, M, Theme, Renderer>>
    for Element<'a, M, Theme, Renderer>
where
    M: Clone + 'a,
    Theme: 'a,
    Renderer: renderer::Renderer + 'a,
{
    fn from(t: Typeahead<'a, M, Theme, Renderer>) -> Self {
        Element::new(t)
    }
}
