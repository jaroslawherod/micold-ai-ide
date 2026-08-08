//! `Select` — the library's own Material select (feature 022, Constitution Principle VIII).
//!
//! A filled field with a chevron, and a list of options anchored beneath it. The field is
//! [`FormField`](super::FormField), the list is [`material::picker`](super::picker), and where the
//! list sits and what it captures is [`cdk::picker`]. Nothing here is new: what this module owns is
//! the *trigger* — the value, the chevron, the state layer and the ripple — and the two pieces of
//! state that make the control answer for itself.
//!
//! # What this replaces, and why it is not a reversal
//!
//! Until feature 022 this wrapped the rendering stack's `pick_list`. That was the right answer when
//! it was written: the dropdown has to anchor to its trigger's own on-screen bounds inside
//! `Modal`'s content-sized dialog, where the hand-rolled panel before it had no `Length::Fill`
//! window to float against and revealed its list *inline*, pushing every later field down.
//! `pick_list` implements `Widget::overlay()` and had no such problem.
//!
//! What changed is that the codebase now has that mechanism of its own — [`cdk::picker`], built for
//! the search picker — which solves the same problem *and* is ours to style. So the list can be the
//! same list the search picker floats, row for row, which is what feature 022 exists to deliver.
//!
//! # Why this is a widget rather than a composition
//!
//! Every other control here is a builder that composes elements and hands them off. This one cannot
//! be, and the reason is [FR-013](../../../../specs/022-dedicated-select-component/spec.md).
//!
//! §7.7 wants the active indicator thickened while the list is **open**, and the whole point of
//! this feature is that the control knows that *by itself* — no caller supplies it, and
//! `Select::active` is gone because nothing could ever have answered it (that is accepted fidelity
//! gap #3). But `FormField` draws the indicator when the element is *built*, and the flag has to
//! live in widget-tree state, which nothing can read until `layout()`. So the trigger and its list
//! are assembled inside [`Widget::layout`], where `&mut self` and `&mut Tree` are both in hand,
//! and `tree.diff_children` keeps the assembled subtree's own state across frames.
//!
//! The single child is a [`cdk::picker::Picker`], so there is exactly one hand-written overlay in
//! this application and it is not this one (`one_overlay_implementation.rs`).
//!
//! # The one thing the list has to tell the trigger back
//!
//! Pressing a row must close the list *and* report the choice, in one step (contract §2). The
//! choice reports itself — the row carries the caller's own message — but the closing does not, and
//! the reason is worth recording because it is the only surprise in this design.
//!
//! Overlay events are dispatched before the widget tree's, and an overlay that captures one stops
//! it there. The rendering stack's button captures both halves of a press, so **a press on a row
//! never reaches this widget's `update` at all**. There is no message to intercept either: the row
//! publishes the caller's `M`, which this component cannot inspect and must not consume.
//!
//! So [`ListWatch`] sits between the panel and the base, inside the overlay, and reports one fact
//! back through a shared cell: whether the list *acted*. It is not a message channel and it carries
//! nothing about the choice — the choice is already on its way to the application. See [`Acted`].
//!
//! # Why every state change here says `invalidate_layout` **and** `invalidate_widgets`
//!
//! Neither is a frame request, and that is the point: `idle_requests_no_frames.rs` allows exactly
//! one `shell.request_redraw()` in the whole rendering layer, inside `cdk::motion`, so that a
//! settled interface asks for nothing. A select changing its own appearance has no `Progress` to
//! ask on its behalf — opening is not an animation, it is a state — so it must reach the screen
//! some other way.
//!
//! The two calls do different halves of that, and both are needed:
//!
//! - **`invalidate_layout`** re-runs `layout` inside the *current* event batch, so the trigger and
//!   the list are rebuilt from the new state before the same batch asks for the overlay. Without it
//!   a press would open a list that does not exist until the next event.
//! - **`invalidate_widgets`** tells the runtime the tree is stale, which is what makes it rebuild
//!   and repaint. A layout invalidation alone is not visible: the runtime consults it only while
//!   already drawing.
//!
//! Both are **edge-triggered** — they fire on a change of open or hover, not on every event — so the
//! render loop still settles, which is the property the frame gate exists to protect.
//!
//! [`cdk::picker`]: crate::ui::cdk::picker
//! [`cdk::picker::Picker`]: crate::ui::cdk::picker::Picker
//!
//! Contract: `specs/022-dedicated-select-component/contracts/select-component.md`.

use std::cell::Cell;
use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{tree, Operation, Tree};
use iced::advanced::{mouse, overlay, renderer, Clipboard, Shell, Widget};
use iced::widget::{container, row, Space};
use iced::{alignment, keyboard, Background, Element, Event, Length, Rectangle, Size, Vector};
use micold_core::tokens::{anatomy, shape, state, Roles};
use micold_core::typeahead::{intent_for, move_highlight, Intent};

use super::picker::{animated_menu, menu_element, Row, EXIT, GAP};
use super::style;
use super::text::TypeRole;
use crate::icons::Icon;
use crate::ui::cdk::picker::{key_for, Picker};

/// A Material select. Builder form (Principle VIII):
/// `Select::new(options, selected, on_selected, roles).placeholder("…").label("…").into()`.
///
/// The bounds are unchanged from the `pick_list`-backed version, so the existing consumer's call is
/// untouched and `ConventionalType` still satisfies them with no changes of its own (FR-030).
pub struct Select<'a, T, M> {
    options: &'a [T],
    selected: Option<T>,
    on_selected: Box<dyn Fn(T) -> M + 'a>,
    placeholder: String,
    roles: Roles,
    label: Option<String>,
    supporting: Option<String>,
    error: Option<String>,
    /// The trigger and its list, assembled in [`Widget::layout`] — see the module docs for why they
    /// cannot be built any earlier.
    content: Option<Element<'a, M>>,
}

impl<'a, T, M> Select<'a, T, M>
where
    T: Clone + ToString + PartialEq + 'a,
{
    /// A select over `options`, currently at `selected` (or unset), emitting `on_selected(t)`
    /// when `t` is picked, themed by `roles`.
    pub fn new(
        options: &'a [T],
        selected: Option<T>,
        on_selected: impl Fn(T) -> M + 'a,
        roles: Roles,
    ) -> Self {
        Self {
            options,
            selected,
            on_selected: Box::new(on_selected),
            placeholder: "Select…".to_string(),
            roles,
            label: None,
            supporting: None,
            error: None,
            content: None,
        }
    }

    /// The control's name, rendered inside its container above the value (FR-031a).
    ///
    /// §7.7 moves the free-standing label that sat above the select into the control's own
    /// container, which is what makes it look like every other field rather than like a button
    /// with a caption.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Explanatory text beneath the container.
    pub fn supporting(mut self, text: impl Into<String>) -> Self {
        self.supporting = Some(text.into());
        self
    }

    /// The validation failure, if any — replaces the supporting text and recolours the chrome.
    pub fn error(mut self, error: Option<impl Into<String>>) -> Self {
        self.error = error.map(Into::into);
        self
    }

    /// Text shown when nothing is selected (default: "Select…").
    ///
    /// Seen only once the label has floated clear: while the label rests it *is* the placeholder,
    /// and printing both would put two words on one line (see [`form_field::label_floats`]).
    ///
    /// [`form_field::label_floats`]: super::form_field::label_floats
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    // `active(bool)` was here. It existed only because `pick_list` could not report its open state
    // to anything but its own style closure, so the caller had to supply what the control knew and
    // in practice none did — the accepted gap in builder form. A component that holds the flag has
    // nobody to ask, so the method has nothing left to mean (contract §3).

    /// Which option is currently chosen, as an index into `options`.
    fn chosen(&self) -> Option<usize> {
        let selected = self.selected.as_ref()?;
        self.options.iter().position(|option| option == selected)
    }
}

/// What the open list did with the last event it saw, reported back to the select that owns it.
///
/// **Not a message channel.** The choice a row publishes is the caller's own and travels the
/// ordinary way; this carries one bit that no message could, because the row's press never reaches
/// the base widget at all (see the module docs). Three values rather than a `bool` because the
/// select has to tell "a row acted" from "the press landed on the panel and belongs to nobody" —
/// and a press it heard nothing about is a press somewhere else entirely, which dismisses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Acted {
    /// Nothing the select needs to know about.
    #[default]
    Nothing,
    /// The press landed inside the list without taking anything — the panel's own padding, say. The
    /// list stays up: a press on a menu is not a press away from it.
    Kept,
    /// A row was taken. The choice is already on its way; the list closes.
    Took,
}

/// The select's own state (data-model §2.2).
///
/// Both values fail `logical_state_ownership.rs`'s question — *would this still mean something with
/// the screen switched off?* — so both belong to the component. This is also the status quo:
/// `pick_list` held them privately too, which is exactly why `Select::active` could never be
/// answered.
#[derive(Debug, Clone)]
struct SelectState {
    open: bool,
    highlight: Option<usize>,
    /// Whether the pointer is over the trigger, for §5's state layer. Held rather than read at draw
    /// time because the layer is the trigger's *background*, decided when the trigger is built.
    hovered: bool,
    /// The channel described in the module docs. Shared with the [`ListWatch`] built into the
    /// list, which is rebuilt every layout — so the cell lives here, where it survives.
    acted: Rc<Cell<Acted>>,
}

impl Default for SelectState {
    fn default() -> Self {
        Self {
            open: false,
            highlight: None,
            hovered: false,
            acted: Rc::new(Cell::new(Acted::Nothing)),
        }
    }
}

impl SelectState {
    /// Read and clear whatever the list reported, applying it.
    ///
    /// Called from both `layout` and `update`, because either can be what runs next: a taken row
    /// captures its own press, so the base widget never sees the event and the report is collected
    /// on the following build instead.
    fn settle(&mut self) -> Acted {
        let acted = self.acted.replace(Acted::Nothing);
        if acted == Acted::Took {
            self.open = false;
            self.highlight = None;
        }
        acted
    }
}

/// Everything the assembly below reads out of the tree, taken in one go so the borrow ends before
/// anything is built.
struct View {
    open: bool,
    highlight: Option<usize>,
    hovered: bool,
    acted: Rc<Cell<Acted>>,
}

/// The trigger and its list, as one element.
///
/// A free function rather than a method so the returned element's lifetime is the *builder's*
/// rather than the borrow's — it holds owned messages and `&'a [T]`, never `&self`.
fn assemble<'a, T, M>(s: &Select<'a, T, M>, view: View) -> Element<'a, M>
where
    T: Clone + ToString + PartialEq + 'a,
    M: Clone + 'a,
{
    let r = s.roles;
    let populated = s.selected.is_some();
    // A resting label *is* the placeholder — it sits on the value's line — so the trigger must not
    // draw a second one under it. One rule, in one place: see `form_field::label_floats`.
    let resting = s.label.is_some() && !super::form_field::label_floats(populated, view.open);
    let value = match &s.selected {
        Some(chosen) => chosen.to_string(),
        None if resting => String::new(),
        None => s.placeholder.clone(),
    };

    // §5's state layer: the content colour over the container at the state's opacity. An open list
    // reads at the pressed opacity and a hovered one at hover — the same two the `pick_list` style
    // closure resolved, and for the same reason (§7.7 asks the select's open and hover states to be
    // carried by the layer). What is different is that the *component* answers now, rather than a
    // closure the rest of the application could not see into.
    let opacity = if view.open {
        state::PRESSED
    } else if view.hovered {
        state::HOVER
    } else {
        0.0
    };
    let layer = (opacity > 0.0)
        .then(|| Background::Color(style::state_fill(style::color(r.on_surface), opacity)));

    let line = row![
        super::Text::new(value, TypeRole::Body, r),
        Space::new().width(Length::Fill),
        super::glyph::icon(
            Icon::SelectChevron,
            anatomy::text_field::TRAILING_ICON,
            r.on_surface_variant,
        ),
    ]
    .align_y(alignment::Vertical::Center);

    // No padding of its own: `FormField` pads the container to §7.7's 16dp, and a trigger that
    // padded itself as well would sit inset from every text field beside it.
    let surface = container(line)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_: &iced::Theme| container::Style {
            background: layer,
            ..container::Style::default()
        });
    // Every pressable surface ripples (feature 019, FR-024c) and this is one. The wrapper watches
    // its own bounds for the press, so it needs no message — which is what lets the trigger publish
    // nothing at all and leaves the press for `Select::update` to read.
    let trigger: Element<'a, M> =
        super::Ripple::new(surface, r.on_surface, shape::EXTRA_SMALL).into();

    let mut field = super::FormField::new(trigger, r)
        // The indicator, answering for itself. Nothing supplies this and nothing can (FR-013).
        .active(view.open)
        // A selection counts as content; an unset select rests its label like an empty input.
        .populated(populated);
    if let Some(label) = s.label.clone() {
        field = field.label(label);
    }
    if let Some(supporting) = s.supporting.clone() {
        field = field.supporting(supporting);
    }
    let field: Element<'a, M> = field.error(s.error.clone()).into();

    // The rows are the search picker's rows, with no matched spans — an `EmphasisedLabel` with no
    // spans is a plain label, so the same renderer serves both lists and there is one definition of
    // what a row looks like (research R6, FR-007).
    let rows: Vec<Row> = s
        .options
        .iter()
        .map(|option| Row::new(option.to_string(), Vec::new()))
        .collect();
    // Taking a row reports the caller's own message directly: there is nothing for this component
    // to translate, and an index would only have to be turned back into an option.
    let pick = |index: usize| (s.on_selected)(s.options[index].clone());
    // No empty message: a select's options are fixed by the call site, so an empty list is a
    // call-site error rather than a search that found nothing (picker-base §3).
    let panel = menu_element(rows, view.highlight, s.chosen(), None, Some(&pick), r);
    // The list arrives and leaves rather than appearing and vanishing (FR-018, FR-019), by the same
    // call the search picker makes — so the two transitions are one definition and cannot drift.
    // Neither duration nor curve is named here; both are `material::picker`'s (FR-020, SC-007).
    //
    // `ListWatch` sits *outside* the transition rather than under it. A press has to be observable
    // for as long as the list is genuinely there, and a wrapper that is mid-fade is exactly the
    // thing that might stop delegating — whereas refusing input to a list on its way out is
    // `cdk::picker`'s job and it already does it (FR-022).
    let menu: Element<'a, M> = Element::new(ListWatch {
        content: animated_menu(panel, view.open, r),
        acted: view.acted,
    });

    // One child, and it is the shared base — so the select adds no second floating mechanism.
    // `.exit(EXIT)` tells that base how long to keep producing an overlay at all, so the animation
    // and the thing being animated cannot disagree about when the list is gone.
    Picker::new(field, menu, view.open, GAP).exit(EXIT).into()
}

impl<'a, T, M> Widget<M, iced::Theme, iced::Renderer> for Select<'a, T, M>
where
    T: Clone + ToString + PartialEq + 'a,
    M: Clone + 'a,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<SelectState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(SelectState::default())
    }

    /// None yet. The child cannot exist before `layout`, because what it looks like depends on
    /// state this cannot reach — see the module docs. `layout` diffs it in, which is what keeps the
    /// list's own state (a scroll position, a ripple mid-animation) across frames.
    fn children(&self) -> Vec<Tree> {
        Vec::new()
    }

    /// The wrapper's shape, not the child's: `FormField` fills its width and is sized by its own
    /// content, and this has to answer before there is a child to ask.
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let view = {
            let state = tree.state.downcast_mut::<SelectState>();
            state.settle();
            View {
                open: state.open,
                highlight: state.highlight,
                hovered: state.hovered,
                acted: state.acted.clone(),
            }
        };

        let mut content = assemble(&*self, view);
        tree.diff_children(std::slice::from_ref(&content));
        let node = content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        self.content = Some(content);
        node
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
        // The trigger's own box — `FormField`'s first band. Not the whole element: the supporting
        // text sits beneath the container, and pressing a hint is not pressing the control.
        let trigger = layout
            .children()
            .next()
            .map(|band| band.bounds())
            .unwrap_or_else(|| layout.bounds());

        let acted = {
            let state = tree.state.downcast_mut::<SelectState>();
            let acted = state.settle();
            if acted == Acted::Took {
                shell.invalidate_layout();
                shell.invalidate_widgets();
            }
            acted
        };

        match event {
            Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                if self.keyboard(tree, key, shell) {
                    return;
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let over = cursor.is_over(trigger);
                let chosen = self.chosen();
                let state = tree.state.downcast_mut::<SelectState>();
                if over {
                    // A press on the trigger toggles, whichever way round it is (contract §2).
                    state.open = !state.open;
                    // Opening seeds the keyboard on the current choice, so the list opens with the
                    // value marked *and reachable* — what `pick_list` gave for free and what must
                    // not leave with it (feature 013's FR-003).
                    state.highlight = if state.open { chosen } else { None };
                    shell.invalidate_layout();
                    shell.invalidate_widgets();
                } else if state.open && acted == Acted::Nothing && cursor.position().is_some() {
                    // Away from both the trigger and the list. `Acted::Kept` is how the list says
                    // the press was its own; without a report the press was somewhere else.
                    state.open = false;
                    state.highlight = None;
                    shell.invalidate_layout();
                    shell.invalidate_widgets();
                }
                // Deliberately not captured: the press still belongs to the ripple underneath.
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let over = cursor.is_over(trigger);
                let state = tree.state.downcast_mut::<SelectState>();
                if state.hovered != over {
                    state.hovered = over;
                    // The state layer is the trigger's background, so a change of hover is a change
                    // of what has to be built — which is a layout, not just a repaint.
                    shell.invalidate_layout();
                    shell.invalidate_widgets();
                }
            }
            _ => {}
        }

        if let (Some(content), Some(state)) = (self.content.as_mut(), tree.children.first_mut()) {
            content.as_widget_mut().update(
                state, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        if let (Some(content), Some(state)) = (self.content.as_ref(), tree.children.first()) {
            content
                .as_widget()
                .draw(state, renderer, theme, style, layout, cursor, viewport);
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        match (self.content.as_ref(), tree.children.first()) {
            (Some(content), Some(state)) => content
                .as_widget()
                .mouse_interaction(state, layout, cursor, viewport, renderer),
            _ => mouse::Interaction::None,
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        if let (Some(content), Some(state)) = (self.content.as_mut(), tree.children.first_mut()) {
            content
                .as_widget_mut()
                .operate(state, layout, renderer, operation);
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, iced::Theme, iced::Renderer>> {
        let content = self.content.as_mut()?;
        let state = tree.children.first_mut()?;
        content
            .as_widget_mut()
            .overlay(state, layout, renderer, viewport, translation)
    }
}

impl<'a, T, M> Select<'a, T, M>
where
    T: Clone + ToString + PartialEq + 'a,
    M: Clone + 'a,
{
    /// Answer `key` while the list is open, returning whether the key was claimed.
    ///
    /// The rule itself is [`intent_for`] — the same render-free function the base applies for the
    /// search picker, so "what does Down mean here" is decided in one place for both controls
    /// (FR-024, SC-008). What differs is only where the answer goes: the search picker's openness
    /// is its caller's, so the base publishes a message; this one's is its own, so it acts.
    fn keyboard(&self, tree: &mut Tree, key: &keyboard::Key, shell: &mut Shell<'_, M>) -> bool {
        let rows = self.options.len();
        let state = tree.state.downcast_mut::<SelectState>();
        if !state.open {
            return false;
        }
        // Every option a select offers can be chosen: `disabled_options` is deliberately not built
        // (data-model §2.1), so the highlighted row is always takeable.
        let Some(intent) = intent_for(key_for(key), state.highlight, rows, true) else {
            return false;
        };

        shell.invalidate_layout();
        shell.invalidate_widgets();
        match intent {
            Intent::Move(direction) => {
                state.highlight = move_highlight(state.highlight, direction, rows);
                true
            }
            Intent::Pick => {
                let chosen = state
                    .highlight
                    .and_then(|index| self.options.get(index))
                    .cloned();
                state.open = false;
                state.highlight = None;
                if let Some(option) = chosen {
                    shell.publish((self.on_selected)(option));
                }
                true
            }
            Intent::Dismiss => {
                state.open = false;
                state.highlight = None;
                // Tab dismisses and **passes on**, so focus still moves — the one key where the two
                // answers come apart, and `intent_for`'s own docs say why. Swallowing it would
                // leave a developer unable to tab past the select at all.
                !matches!(key, keyboard::Key::Named(keyboard::key::Named::Tab))
            }
        }
    }
}

impl<'a, T, M> From<Select<'a, T, M>> for Element<'a, M>
where
    T: Clone + ToString + PartialEq + 'a,
    M: Clone + 'a,
{
    fn from(s: Select<'a, T, M>) -> Self {
        Element::new(s)
    }
}

/// The list, plus the one bit it reports back to the select that owns it.
///
/// Layout-transparent — it returns its child's node unchanged, exactly as `fade` and `scale` do —
/// so it adds nothing to the list's geometry and `picker_parity.rs` compares like with like.
///
/// A private struct, so the library inventory does not count it as a component: it is a piece of
/// this control's plumbing rather than something a call site could ever build.
struct ListWatch<'a, M> {
    content: Element<'a, M>,
    acted: Rc<Cell<Acted>>,
}

impl<M> Widget<M, iced::Theme, iced::Renderer> for ListWatch<'_, M> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
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
        // A local shell, so "did the list act?" is answerable at all. The messages are the caller's
        // and pass straight through — `merge` carries the redraw request, the layout invalidation
        // and the capture with them, so nothing is lost by the detour.
        let mut published: Vec<M> = Vec::new();
        let mut local = Shell::new(&mut published);
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            &mut local,
            viewport,
        );
        let took = !local.is_empty();
        shell.merge(local, |message| message);

        if took {
            self.acted.set(Acted::Took);
        } else if matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        ) && cursor.is_over(layout.bounds())
        {
            self.acted.set(Acted::Kept);
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
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

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
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
