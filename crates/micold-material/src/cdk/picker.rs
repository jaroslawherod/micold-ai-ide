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

use std::time::Duration;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::operation::Focusable;
use iced::advanced::widget::{tree, Operation, Tree};
use iced::advanced::{mouse, overlay, renderer, Clipboard, Shell, Widget};
use iced::{keyboard, Element, Event, Length, Point, Rectangle, Size, Vector};
use micold_core::typeahead::{claims, intent_for, Direction, Intent, Key};

use super::motion::Progress;

/// At or below this the list counts as gone: it is not drawn, it takes no input, and `overlay()`
/// stops producing it. The same figure the material layer's own wrappers call hidden, so "invisible"
/// and "absent" land on the same frame rather than a frame apart.
const HIDDEN: f32 = 0.001;

/// How much of the list is still on screen.
///
/// Separate from the fade and the scale the material layer wraps the panel in, and deliberately so.
/// Those decide what the list *looks like* on its way out; this decides whether there is anything to
/// look at — which is `overlay()`'s question, and `overlay()` is this module's. Keeping them in one
/// place would mean either this file learning what a duration means, or the material layer deciding
/// whether an overlay exists.
#[derive(Debug, Clone, Copy)]
struct Visibility {
    progress: Progress,
}

/// A field with a floating list of results beneath it.
///
/// Composed rather than drawn: both halves arrive as already-built elements, so this module never
/// learns what either looks like.
pub struct Picker<'a, M, Theme = iced::Theme, Renderer = iced::Renderer> {
    field: Element<'a, M, Theme, Renderer>,
    menu: Element<'a, M, Theme, Renderer>,
    open: bool,
    highlight: Option<usize>,
    rows: usize,
    highlighted_enabled: bool,
    gap: f32,
    exit: Duration,
    on_move: Option<Box<dyn Fn(Direction) -> M + 'a>>,
    on_pick: Option<M>,
    on_dismiss: Option<M>,
    on_focus: Option<M>,
    in_flow: bool,
    panel_width: Option<f32>,
}

impl<'a, M: Clone + 'a, Theme, Renderer> Picker<'a, M, Theme, Renderer> {
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
            exit: Duration::ZERO,
            on_move: None,
            on_pick: None,
            on_dismiss: None,
            on_focus: None,
            in_flow: false,
            panel_width: None,
        }
    }

    /// Lay the menu out beneath the field, in the parent's flow, instead of floating it: the
    /// element grows to hold it, and a surface the picker sits on (a dialog's card) encloses it.
    ///
    /// Nothing is animated in this mode — the menu is there while `open` and absent otherwise —
    /// and the keyboard rule is applied by the picker's own `update`, where the events arrive.
    /// Dismissing by a press elsewhere is the caller's (a dialog's scrim does it).
    pub fn in_flow(mut self, in_flow: bool) -> Self {
        self.in_flow = in_flow;
        self
    }

    /// Float the menu as a panel of controls `width` wide rather than a list as wide as the field:
    /// its trailing edge on the field's (never past the window's leading edge), as a panel opened
    /// from a button at the end of a row sits. Tab and Shift+Tab then cycle through the field and
    /// the panel's controls, and claim the key, rather than closing it: the application's own
    /// traversal reaches a floated panel only after every control behind it. Escape and a press
    /// outside still close it, and Escape hands the keyboard back to the field.
    pub fn panel(mut self, width: f32) -> Self {
        self.panel_width = Some(width);
        self
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

    /// How long the list keeps existing after it closes, so it has something left to animate away.
    ///
    /// A bare `Duration` crossing into the behaviour layer, for the same reason `gap` is a bare
    /// number: how long a thing takes is appearance, and the material layer resolves it from the
    /// motion tokens before handing it over. Left unset the list still survives a frame — long
    /// enough to refuse the press that closed it, not long enough to see.
    pub fn exit(mut self, exit: Duration) -> Self {
        self.exit = exit;
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

impl<M: Clone, Theme, Renderer> Picker<'_, M, Theme, Renderer> {
    /// The field's layout, and the menu's when it is laid out in the flow beneath it.
    fn parts<'l>(&self, layout: Layout<'l>) -> (Layout<'l>, Option<Layout<'l>>) {
        if !(self.in_flow && self.open) {
            return (layout, None);
        }
        let mut children = layout.children();
        let field = children.next().unwrap_or(layout);
        (field, children.next())
    }

    /// The keyboard rule's inputs and outputs, as the menu's overlay and the in-flow menu both read
    /// them.
    fn keys(&self) -> Keys<'_, M> {
        Keys {
            highlight: self.highlight,
            rows: self.rows,
            highlighted_enabled: self.highlighted_enabled,
            on_move: self.on_move.as_deref(),
            on_pick: self.on_pick.clone(),
            on_dismiss: self.on_dismiss.clone(),
            tab_closes: self.panel_width.is_none(),
        }
    }
}

impl<M: Clone, Theme, Renderer> Picker<'_, M, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    /// Down or Enter, unmodified, on a closed field that holds the keyboard opens it: `on_focus` is published
    /// and the key goes no further. A field reached by Tab never sees the press that `on_focus`
    /// otherwise comes from, so without this its list could not open from the keyboard, and Enter
    /// went on to whatever is behind the dialog (feature 1185 M5, quickstart Part B step 9). A key
    /// the field itself took (Enter running its `on_submit`) is left to it.
    fn open_from_keyboard(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        field_layout: Layout<'_>,
        renderer: &Renderer,
        shell: &mut Shell<'_, M>,
    ) {
        let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event else {
            return;
        };
        // Unmodified only: Ctrl+Enter or Shift+Down stay free for whatever binds them.
        let opens = modifiers.is_empty() && matches!(key_for(key), Key::Down | Key::Enter);
        if self.open || shell.is_event_captured() || !opens {
            return;
        }
        let Some(message) = self.on_focus.clone() else {
            return;
        };
        let mut census = Census::default();
        self.field.as_widget_mut().operate(
            &mut tree.children[0],
            field_layout,
            renderer,
            &mut census,
        );
        if census.focused.is_some() {
            shell.publish(message);
            shell.capture_event();
        }
    }
}

impl<M, Theme, Renderer> Widget<M, Theme, Renderer> for Picker<'_, M, Theme, Renderer>
where
    M: Clone,
    Renderer: renderer::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Visibility>()
    }

    /// A list built open is already there; one built closed has never been. Neither animates its
    /// *existence* into being on mount — only a change of `open` does.
    fn state(&self) -> tree::State {
        tree::State::new(Visibility {
            progress: Progress::new(if self.open { 1.0 } else { 0.0 }),
        })
    }

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
    /// point of anchoring it in an overlay (FR-001a) — unless the picker is [in flow](Self::in_flow).
    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let (field_tree, menu_tree) = tree.children.split_at_mut(1);
        let field = self
            .field
            .as_widget_mut()
            .layout(&mut field_tree[0], renderer, limits);
        if !(self.in_flow && self.open) {
            return field;
        }
        let (width, below) = (field.size().width, field.size().height + self.gap);
        let room = Size::new(width, (limits.max().height - below).max(0.0));
        let menu = self.menu.as_widget_mut().layout(
            &mut menu_tree[0],
            renderer,
            &layout::Limits::new(Size::ZERO, room).width(width),
        );
        let size = Size::new(width, below + menu.size().height);
        layout::Node::with_children(size, vec![field, menu.move_to(Point::new(0.0, below))])
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
        // How much of the list is left, advanced once per frame. Before the field sees the event,
        // because a frame tick is not the field's business and this must not depend on what the
        // field does with it.
        let target = if self.open { 1.0 } else { 0.0 };
        let progress = &mut tree.state.downcast_mut::<Visibility>().progress;
        let floated = progress.value() > HIDDEN;
        progress.on_frame(event, target, self.exit, shell);
        // The floated list comes or goes with this frame, after the runtime laid out the floated
        // layers for it. That layout is what the frame's `draw` uses, so without a fresh one the
        // list is drawn against a layout made without it, or another layer against the list's,
        // and a scrollable in it panics (#1453).
        if !self.in_flow && floated != (progress.value() > HIDDEN) {
            shell.invalidate_layout();
        }

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

        let (field_layout, menu_layout) = self.parts(layout);

        if menu_layout.is_some() {
            if let Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = event {
                if let Some((message, claim)) = self.keys().message_for(key) {
                    shell.publish(message);
                    if claim == Claim::Taken {
                        shell.capture_event();
                        return;
                    }
                }
            }
        }

        let (field_tree, menu_tree) = tree.children.split_at_mut(1);
        self.field.as_widget_mut().update(
            &mut field_tree[0],
            event,
            field_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
        if let Some(menu_layout) = menu_layout {
            self.menu.as_widget_mut().update(
                &mut menu_tree[0],
                event,
                menu_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }
        self.open_from_keyboard(tree, event, field_layout, renderer, shell);
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
        let (field_layout, menu_layout) = self.parts(layout);
        self.field.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            field_layout,
            cursor,
            viewport,
        );
        if let Some(menu_layout) = menu_layout {
            self.menu.as_widget().draw(
                &tree.children[1],
                renderer,
                theme,
                style,
                menu_layout,
                cursor,
                viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let (field_layout, menu_layout) = self.parts(layout);
        let field = self.field.as_widget().mouse_interaction(
            &tree.children[0],
            field_layout,
            cursor,
            viewport,
            renderer,
        );
        match menu_layout {
            Some(menu_layout) => field.max(self.menu.as_widget().mouse_interaction(
                &tree.children[1],
                menu_layout,
                cursor,
                viewport,
                renderer,
            )),
            _ => field,
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        let (field_layout, menu_layout) = self.parts(layout);
        let (field_tree, menu_tree) = tree.children.split_at_mut(1);
        self.field
            .as_widget_mut()
            .operate(&mut field_tree[0], field_layout, renderer, operation);
        if let Some(menu_layout) = menu_layout {
            self.menu
                .as_widget_mut()
                .operate(&mut menu_tree[0], menu_layout, renderer, operation);
        }
    }

    /// The list, floated beneath the field.
    ///
    /// Handing it to the rendering stack's own overlay system is what makes it independent of the
    /// parent layout's size constraints — the same mechanism every `pick_list` and tooltip relies
    /// on, and the only one that works inside a content-sized dialog.
    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        _renderer: &Renderer,
        _viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, Theme, Renderer>> {
        if self.in_flow {
            // The menu is in the tree, so only what the menu itself floats (a row's tooltip) is
            // produced here.
            let menu_layout = self.parts(layout).1?;
            return self.menu.as_widget_mut().overlay(
                &mut tree.children[1],
                menu_layout,
                _renderer,
                _viewport,
                translation,
            );
        }
        // Not `if !self.open`. A list that vanished the instant it closed would have nothing left
        // to fade, which is the defect this replaces — so the question is whether any of it is still
        // on screen, not whether the caller still wants it (contract picker-base §C1.5).
        let visibility = tree.state.downcast_ref::<Visibility>().progress.value();
        if visibility <= HIDDEN {
            return None;
        }
        // Below its own threshold on the way out the list is present but inert. `leaving` is what
        // carries that to the overlay, which is where the events actually arrive.
        let leaving = !self.open;

        let bounds = layout.bounds();
        let field = Rectangle {
            x: bounds.x + translation.x,
            y: bounds.y + translation.y,
            ..bounds
        };

        let keys = Keys {
            highlight: self.highlight,
            rows: self.rows,
            highlighted_enabled: self.highlighted_enabled,
            on_move: self.on_move.as_deref(),
            on_pick: self.on_pick.clone(),
            on_dismiss: self.on_dismiss.clone(),
            tab_closes: self.panel_width.is_none(),
        };
        let field_layout = self.parts(layout).0;
        let panel = self.panel_width.is_some();
        let (field_tree, menu_tree) = tree.children.split_at_mut(1);
        let trigger = panel.then(|| Trigger {
            element: &mut self.field,
            state: &mut field_tree[0],
            layout: field_layout,
        });
        Some(overlay::Element::new(Box::new(Menu {
            content: &mut self.menu,
            state: &mut menu_tree[0],
            trigger,
            field,
            panel_width: self.panel_width,
            gap: self.gap,
            leaving,
            on_dismiss: self.on_dismiss.clone(),
            keys,
        })))
    }
}

/// The rendering stack's key, as the render-free rule names it.
///
/// Public, and deliberately: a picker whose openness is its own answers these keys in its **own**
/// `update` rather than through [`Picker`]'s message hooks — the select does, because it has no
/// caller to send `on_move` or `on_dismiss` to. Both routes end in
/// [`intent_for`](micold_core::typeahead::intent_for), and this is the only step before it, so a
/// second copy of the mapping would be the one place the two controls could come to disagree about
/// what a key even *is* (FR-024, SC-008).
pub fn key_for(key: &keyboard::Key) -> Key {
    match key {
        keyboard::Key::Named(keyboard::key::Named::ArrowDown) => Key::Down,
        keyboard::Key::Named(keyboard::key::Named::ArrowUp) => Key::Up,
        keyboard::Key::Named(keyboard::key::Named::Enter) => Key::Enter,
        keyboard::Key::Named(keyboard::key::Named::Escape) => Key::Escape,
        keyboard::Key::Named(keyboard::key::Named::Tab) => Key::Tab,
        _ => Key::Other,
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
    /// Whether Tab closes the menu: a list's does, a panel of controls' moves between them.
    tab_closes: bool,
}

impl<M: Clone> Keys<'_, M> {
    /// What this key press should publish, and whether the list has claimed the key.
    ///
    /// The two answers come apart for exactly one key. Tab dismisses the list *and* belongs to the
    /// field, because what it is doing is moving focus out — swallowing it would leave the developer
    /// unable to tab past the picker at all. Every other key the rule names is the list's alone:
    /// capture is what stops Enter also submitting the dialog behind it.
    fn message_for(&self, key: &keyboard::Key) -> Option<(M, Claim)> {
        let key = key_for(key);
        if key == Key::Tab && !self.tab_closes {
            return None;
        }

        let message = match intent_for(key, self.highlight, self.rows, self.highlighted_enabled)? {
            Intent::Move(direction) => self.on_move.map(|f| f(direction)),
            Intent::Pick => self.on_pick.clone(),
            Intent::Dismiss => self.on_dismiss.clone(),
        }?;

        // The rule is `claims`, not a `matches!` here. It was written out in this file and again
        // in `material::select`, and the two had already come apart (feature 022, T030).
        let claim = if claims(key) {
            Claim::Taken
        } else {
            Claim::PassOn
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

/// A panel's field, which its keyboard cycle starts from and Escape returns to.
struct Trigger<'a, 'b, M, Theme, Renderer> {
    element: &'b mut Element<'a, M, Theme, Renderer>,
    state: &'b mut Tree,
    layout: Layout<'b>,
}

/// Where a panel's Tab, Shift+Tab or Escape puts the keyboard.
#[derive(Clone, Copy)]
enum Step {
    Forward,
    Back,
    Trigger,
}

/// Counts the focusables a walk meets and notes which one holds the keyboard.
#[derive(Default)]
struct Census {
    count: usize,
    focused: Option<usize>,
}

impl Operation for Census {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(
        &mut self,
        _id: Option<&iced::advanced::widget::Id>,
        _bounds: Rectangle,
        state: &mut dyn Focusable,
    ) {
        if state.is_focused() {
            self.focused = Some(self.count);
        }
        self.count += 1;
    }
}

/// Gives the keyboard to the `target`th focusable a walk meets and takes it from the others.
struct FocusAt {
    target: usize,
    seen: usize,
}

impl Operation for FocusAt {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(
        &mut self,
        _id: Option<&iced::advanced::widget::Id>,
        _bounds: Rectangle,
        state: &mut dyn Focusable,
    ) {
        if self.seen == self.target {
            state.focus();
        } else if state.is_focused() {
            state.unfocus();
        }
        self.seen += 1;
    }
}

/// The floating list itself: where it sits, what it captures, and when it closes.
struct Menu<'a, 'b, M, Theme, Renderer> {
    content: &'b mut Element<'a, M, Theme, Renderer>,
    state: &'b mut Tree,
    /// A panel's field ([`Picker::panel`]); `None` for a list.
    trigger: Option<Trigger<'a, 'b, M, Theme, Renderer>>,
    /// The field's on-screen rectangle — everything below is measured from it.
    field: Rectangle,
    /// A panel's own width, its trailing edge on the field's ([`Picker::panel`]).
    panel_width: Option<f32>,
    gap: f32,
    /// Whether the list is on its way out. A leaving list is drawn and animated but takes no input.
    leaving: bool,
    on_dismiss: Option<M>,
    keys: Keys<'b, M>,
}

impl<M, Theme, Renderer> Menu<'_, '_, M, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    /// Run `operation` over the panel's field, then its controls: the panel's keyboard cycle.
    fn walk(&mut self, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        if let Some(trigger) = self.trigger.as_mut() {
            trigger.element.as_widget_mut().operate(
                trigger.state,
                trigger.layout,
                renderer,
                operation,
            );
        }
        self.content
            .as_widget_mut()
            .operate(self.state, layout, renderer, operation);
    }

    /// Move the keyboard one `step` round the cycle. With nothing in it focused, Tab enters at the
    /// first control and Shift+Tab at the last.
    fn move_focus(&mut self, step: Step, layout: Layout<'_>, renderer: &Renderer) {
        let mut census = Census::default();
        self.walk(layout, renderer, &mut census);
        let Census { count, focused } = census;
        if count == 0 {
            return;
        }
        let target = match (step, focused) {
            (Step::Trigger, _) => 0,
            (Step::Forward, Some(at)) => (at + 1) % count,
            (Step::Forward, None) => 1 % count,
            (Step::Back, Some(at)) => (at + count - 1) % count,
            (Step::Back, None) => count - 1,
        };
        self.walk(layout, renderer, &mut FocusAt { target, seen: 0 });
    }
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

        let width = self.panel_width.unwrap_or(self.field.width);
        let x = match self.panel_width {
            Some(width) => (self.field.x + self.field.width - width).max(0.0),
            None => self.field.x,
        };
        let limits = layout::Limits::new(Size::ZERO, Size::new(width, room)).width(width);
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

        node.move_to(Point::new(x, y))
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
        // A leaving list is inert (FR-022). It is on screen only in the sense that it is still
        // fading, so a press landing where a row used to be must do nothing and Enter must take
        // nothing — otherwise closing the list opens a window in which it looks gone and still acts.
        //
        // Window events are the exception, and a load-bearing one: the frame tick arrives that way,
        // and content that stopped receiving it would freeze half-faded and never finish leaving.
        // The same carve-out `material::animation`'s wrappers make, for the same reason.
        if self.leaving && !matches!(event, Event::Window(_)) {
            return;
        }
        // Nor is a leaving list pointed at. Its content keeps its widget state across a close and
        // a reopen, so a row that went on seeing the cursor while it faded would keep its tooltip
        // open, or its wait running, and show the panel the moment the list came back (038
        // FR-015, FR-017). Without a cursor every row sees it leave on the first leaving frame.
        let cursor = if self.leaving {
            mouse::Cursor::Unavailable
        } else {
            cursor
        };

        // The keyboard first, and only the keys the rule claims. Capturing the event is what stops
        // Enter also submitting the dialog behind the list.
        if let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event {
            let pressed = key_for(key);
            // A panel keeps Tab to itself: the application's traversal would reach the panel's
            // controls only after every control behind it (feature 1185, visual pass M4).
            if self.trigger.is_some() && pressed == Key::Tab {
                let step = if modifiers.shift() {
                    Step::Back
                } else {
                    Step::Forward
                };
                self.move_focus(step, layout, renderer);
                shell.capture_event();
                return;
            }
            if let Some((message, claim)) = self.keys.message_for(key) {
                // Escape closes the panel, and the control holding the keyboard goes with it: the
                // field takes it back, so the next Tab goes on from there.
                if self.trigger.is_some() && pressed == Key::Escape {
                    self.move_focus(Step::Trigger, layout, renderer);
                }
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

        // A press that landed *on* the list belongs to the list, whatever the row under it decided
        // to do with it. Rows that do something claim it themselves; a row that deliberately does
        // nothing declines it, and an unclaimed press carries on to whatever sits behind — which
        // for a list floating past a dialog's edges is that dialog's scrim, and its cancellation
        // (016 BUG-002, FR-035, research R14).
        //
        // `row_element` makes each inert row opaque, which is the fix; this is the same rule stated
        // once for the whole surface, so a future row that forgets cannot reopen the hole.
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
            if cursor
                .position()
                .is_some_and(|p| layout.bounds().contains(p))
            {
                shell.capture_event();
            }
        }
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

    /// What the list's own content floats: a row's tooltip (feature 038, FR-023).
    ///
    /// The rows live in this overlay, so without this their panels would never be produced. The
    /// list's bounds are the viewport, and the layout is already in window coordinates, so there
    /// is nothing to translate by. A leaving list floats nothing: it takes no input, so nothing
    /// in it is rested on.
    fn overlay<'c>(
        &'c mut self,
        layout: Layout<'c>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'c, M, Theme, Renderer>> {
        if self.leaving {
            return None;
        }
        self.content.as_widget_mut().overlay(
            self.state,
            layout,
            renderer,
            &layout.bounds(),
            Vector::ZERO,
        )
    }
}

impl<'a, M, Theme, Renderer> From<Picker<'a, M, Theme, Renderer>>
    for Element<'a, M, Theme, Renderer>
where
    M: Clone + 'a,
    Theme: 'a,
    Renderer: renderer::Renderer + 'a,
{
    fn from(t: Picker<'a, M, Theme, Renderer>) -> Self {
        Element::new(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{key_with, renderer, WINDOW};
    use iced::advanced::clipboard;
    use iced::advanced::widget::operation::Focusable;
    use iced::keyboard::key::Named;
    use iced::keyboard::Modifiers;
    use iced::widget::{column, text_input};

    fn input(hint: &'static str) -> iced::widget::TextInput<'static, String> {
        text_input(hint, "").on_input(|s| s)
    }

    /// A trigger and an open panel of two fields, as the Jira picker's Filters button opens one
    /// (feature 1185, D15).
    fn panel() -> Element<'static, String> {
        panel_of(column![input("Reporter"), input("Label")].into())
    }

    /// The trigger and an open panel of `fields`.
    fn panel_of(fields: Element<'static, String>) -> Element<'static, String> {
        Picker::new(input("Filters"), fields, true, 4.0)
            .panel(360.0)
            .on_dismiss("dismissed".to_string())
            .into()
    }

    /// The panel with its two fields as the Filters panel holds them: closed lists of their own,
    /// each publishing `<name> opened` when reached (feature 1185 M5).
    fn panel_of_lists() -> Element<'static, String> {
        let list = |hint: &'static str| {
            Picker::new(input(hint), iced::widget::text("rows"), false, 4.0)
                .on_focus(format!("{hint} opened"))
                .on_dismiss(format!("{hint} dismissed"))
        };
        panel_of(column![list("Reporter"), list("Label")].into())
    }

    /// Which focusables hold the keyboard, in traversal order.
    #[derive(Default)]
    struct Held(Vec<bool>);

    impl Operation for Held {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }

        fn focusable(
            &mut self,
            _id: Option<&iced::advanced::widget::Id>,
            _bounds: Rectangle,
            state: &mut dyn Focusable,
        ) {
            self.0.push(state.is_focused());
        }
    }

    /// The trigger in the window, its panel floated, as the runtime holds them.
    struct Open {
        element: Element<'static, String>,
        tree: Tree,
        node: layout::Node,
        renderer: iced::Renderer,
    }

    impl Open {
        fn new() -> Self {
            Self::of(panel())
        }

        fn of(mut element: Element<'static, String>) -> Self {
            let renderer = renderer();
            let mut tree = Tree::new(element.as_widget());
            let node = element
                .as_widget_mut()
                .layout(
                    &mut tree,
                    &renderer,
                    &layout::Limits::new(Size::ZERO, WINDOW),
                )
                .move_to(Point::new(200.0, 40.0));
            Self {
                element,
                tree,
                node,
                renderer,
            }
        }

        /// The trigger holds the keyboard, as it does once Space on it opened the panel.
        fn focus_trigger(&mut self) {
            self.element.as_widget_mut().operate(
                &mut self.tree,
                Layout::new(&self.node),
                &self.renderer,
                &mut Focus,
            );
        }

        /// Which of trigger, Reporter, Label hold the keyboard.
        fn held(&mut self) -> Vec<bool> {
            let mut held = Held::default();
            self.element.as_widget_mut().operate(
                &mut self.tree,
                Layout::new(&self.node),
                &self.renderer,
                &mut held,
            );
            let mut overlay = self
                .element
                .as_widget_mut()
                .overlay(
                    &mut self.tree,
                    Layout::new(&self.node),
                    &self.renderer,
                    &Rectangle::with_size(WINDOW),
                    Vector::ZERO,
                )
                .expect("the panel is open");
            let node = overlay.as_overlay_mut().layout(&self.renderer, WINDOW);
            overlay
                .as_overlay_mut()
                .operate(Layout::new(&node), &self.renderer, &mut held);
            held.0
        }

        /// A key as the runtime sends it: the overlay first, the tree only if it was let past.
        /// What was published, and whether it was captured.
        fn press(&mut self, named: Named, modifiers: Modifiers) -> (Vec<String>, bool) {
            let event = key_with(named, modifiers);
            let viewport = Rectangle::with_size(WINDOW);
            let mut messages = Vec::new();
            let mut shell = Shell::new(&mut messages);
            if let Some(mut overlay) = self.element.as_widget_mut().overlay(
                &mut self.tree,
                Layout::new(&self.node),
                &self.renderer,
                &viewport,
                Vector::ZERO,
            ) {
                let node = overlay.as_overlay_mut().layout(&self.renderer, WINDOW);
                overlay.as_overlay_mut().update(
                    &event,
                    Layout::new(&node),
                    mouse::Cursor::Unavailable,
                    &self.renderer,
                    &mut clipboard::Null,
                    &mut shell,
                );
            }
            if !shell.is_event_captured() {
                self.element.as_widget_mut().update(
                    &mut self.tree,
                    &event,
                    Layout::new(&self.node),
                    mouse::Cursor::Unavailable,
                    &self.renderer,
                    &mut clipboard::Null,
                    &mut shell,
                    &viewport,
                );
            }
            let captured = shell.is_event_captured();
            (messages, captured)
        }
    }

    /// Focuses the first focusable it meets: the trigger.
    struct Focus;

    impl Operation for Focus {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }

        fn focusable(
            &mut self,
            _id: Option<&iced::advanced::widget::Id>,
            _bounds: Rectangle,
            state: &mut dyn Focusable,
        ) {
            state.focus();
        }
    }

    /// Visual pass M4 (feature 1185): Space opened the Filters panel and Tab went to the field
    /// under it, because the panel let Tab past to the application's traversal, which reaches the
    /// floated panel only after every control in the dialog. Tab and Shift+Tab now cycle through
    /// the trigger and the panel's fields, and claim the key so the application never moves it.
    #[test]
    fn tab_and_shift_tab_cycle_through_the_trigger_and_the_open_panel() {
        let mut open = Open::new();
        open.focus_trigger();
        assert_eq!(
            open.held(),
            [true, false, false],
            "the trigger starts with the keyboard"
        );

        let steps = [
            (
                Modifiers::empty(),
                [false, true, false],
                "Tab from the trigger reaches Reporter",
            ),
            (
                Modifiers::empty(),
                [false, false, true],
                "Tab from Reporter reaches Label",
            ),
            (
                Modifiers::empty(),
                [true, false, false],
                "Tab from Label wraps to the trigger",
            ),
            (
                Modifiers::SHIFT,
                [false, false, true],
                "Shift+Tab from the trigger wraps to Label",
            ),
            (
                Modifiers::SHIFT,
                [false, true, false],
                "Shift+Tab from Label reaches Reporter",
            ),
        ];
        for (modifiers, held, what) in steps {
            let (messages, captured) = open.press(Named::Tab, modifiers);
            assert!(
                captured,
                "{what}: the panel let Tab past to the dialog's traversal"
            );
            assert!(
                messages.is_empty(),
                "{what}: Tab closed the panel ({messages:?})"
            );
            assert_eq!(open.held(), held, "{what}");
        }
    }

    /// Escape closes the panel and gives the keyboard back to the trigger, so the next Tab goes on
    /// from the Filters button rather than from a field that is no longer there (quickstart step 9).
    #[test]
    fn escape_in_the_panel_closes_it_and_hands_the_keyboard_back_to_the_trigger() {
        let mut open = Open::new();
        open.focus_trigger();
        open.press(Named::Tab, Modifiers::empty());
        assert_eq!(open.held(), [false, true, false], "Tab reached Reporter");

        let (messages, captured) = open.press(Named::Escape, Modifiers::empty());
        assert_eq!(messages, ["dismissed"], "Escape closes the panel");
        assert!(
            captured,
            "Escape is the panel's: the dialog behind it must not also close"
        );
        assert_eq!(
            open.held(),
            [true, false, false],
            "the trigger has the keyboard again"
        );
    }

    /// Quickstart Part B step 9 (feature 1185 M5): a filter field reached by the panel's Tab cycle
    /// held the keyboard with its list closed, so Down did nothing — its choices never loaded — and
    /// Enter went past the dialog to the control behind it. Down or Enter on a closed field that
    /// holds the keyboard opens it, and the key goes no further.
    #[test]
    fn down_or_enter_in_a_tabbed_to_closed_field_opens_it_and_goes_no_further() {
        for key in [Named::ArrowDown, Named::Enter] {
            let mut open = Open::of(panel_of_lists());
            open.focus_trigger();
            open.press(Named::Tab, Modifiers::empty());
            assert_eq!(open.held(), [false, true, false], "Tab reached Reporter");
            let (messages, captured) = open.press(key, Modifiers::empty());
            assert_eq!(
                messages,
                ["Reporter opened"],
                "{key:?} opens Reporter's list"
            );
            assert!(captured, "{key:?} went past the panel to the dialog behind");
        }
    }

    /// The same keys with no field holding the keyboard open nothing: a field opens only where
    /// the keyboard is.
    #[test]
    fn down_or_enter_opens_no_field_that_lacks_the_keyboard() {
        for key in [Named::ArrowDown, Named::Enter] {
            let mut open = Open::of(panel_of_lists());
            open.focus_trigger();
            let (messages, _) = open.press(key, Modifiers::empty());
            assert!(
                messages.iter().all(|m| !m.ends_with("opened")),
                "{key:?} on the trigger opened a field: {messages:?}"
            );
        }
    }

    /// A modified Down or Enter (Ctrl+Enter, Shift+Down) is left to whatever binds it.
    #[test]
    fn a_modified_down_or_enter_opens_no_field() {
        for (key, modifiers) in [
            (Named::Enter, Modifiers::CTRL),
            (Named::ArrowDown, Modifiers::SHIFT),
        ] {
            let mut open = Open::of(panel_of_lists());
            open.focus_trigger();
            open.press(Named::Tab, Modifiers::empty());
            let (messages, _) = open.press(key, modifiers);
            assert!(
                messages.iter().all(|m| !m.ends_with("opened")),
                "{modifiers:?}+{key:?} opened Reporter: {messages:?}"
            );
        }
    }

    /// A list that starts to show while another floating layer is up is laid out before it is
    /// drawn (#1453). The runtime keeps the floated layers' layout from `update` for the `draw`
    /// after it; a list that came into being during that `update` (its first frame of fading in)
    /// was drawn against a layout made without it, and its scrollable found no child and panicked.
    #[test]
    fn a_list_that_starts_showing_beside_another_layer_is_laid_out_before_it_is_drawn() {
        use iced::widget::{container, scrollable, text};
        use iced_runtime::user_interface::{Cache, UserInterface};
        use std::time::{Duration, Instant};

        // The opening list first, so the layers group hands it the other layer's layout.
        let view = |opening: bool| -> Element<'static, String> {
            let source = Picker::new(
                input("Source"),
                container(scrollable(column![text("Working tree")])),
                opening,
                4.0,
            );
            let other = Picker::new(input("Other"), text("rows"), true, 4.0);
            column![source, other].into()
        };

        let mut renderer = renderer();
        let mut cache = Cache::default();
        let start = Instant::now();
        for frame in 0..30u64 {
            let mut ui = UserInterface::build(
                view(frame >= 2),
                WINDOW,
                std::mem::take(&mut cache),
                &mut renderer,
            );
            let tick = Event::Window(iced::window::Event::RedrawRequested(
                start + Duration::from_millis(16 * frame),
            ));
            let _ = ui.update(
                &[tick],
                mouse::Cursor::Unavailable,
                &mut renderer,
                &mut clipboard::Null,
                &mut Vec::new(),
            );
            ui.draw(
                &mut renderer,
                &iced::Theme::Light,
                &renderer::Style::default(),
                mouse::Cursor::Unavailable,
            );
            cache = ui.into_cache();
        }
    }
}
