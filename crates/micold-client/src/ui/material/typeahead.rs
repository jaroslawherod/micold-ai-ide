//! `Typeahead` — a search field with an attached list of matched results (feature 021,
//! Constitution Principle VIII).
//!
//! What is left here after feature 022 is what makes this control a *search*: the field itself —
//! a text input with the search icon leading and the clear action trailing — and the assembly that
//! hands a query, its already-ranked rows and its keyboard position to the parts below.
//!
//! **The list is no longer this module's.** The row, its four channels of state, the panel they sit
//! on and the transition that brings it in all moved to [`material::picker`], because the select
//! needs exactly the same ones and a second copy would drift. That move is the whole of feature
//! 022's Principle VIII argument, and what remains here is only what a select does not have.
//!
//! Positioning, capture and dismissal are not this module's job either — [`cdk::picker`] does those
//! and is handed already-resolved values, the same arrangement `cdk::overlay` and `material::modal`
//! use.
//!
//! [`material::picker`]: super::picker
//! [`cdk::picker`]: crate::ui::cdk::picker
//!
//! Contracts: `specs/021-branch-typeahead-search/contracts/typeahead-component.md` §1, §4, and
//! `specs/022-dedicated-select-component/contracts/picker-base.md` §3.

use super::picker::{animated_menu, menu_element, Row, EXIT, GAP};
use crate::icons::Icon;
use crate::ui::cdk::picker::Picker as Behaviour;
use iced::Element;
use micold_core::tokens::Roles;
use micold_core::typeahead::Direction;

/// A search field with a floating list of matched results.
///
/// Builder form (Principle VIII):
/// `Typeahead::new(query, &rows, Message::Typed, roles).placeholder("…").on_pick(…).into()`.
pub struct Typeahead<'a, M> {
    query: &'a str,
    rows: Vec<Row>,
    on_input: Box<dyn Fn(String) -> M + 'a>,
    roles: Roles,
    placeholder: String,
    label: Option<String>,
    supporting: Option<String>,
    open: bool,
    highlighted: Option<usize>,
    selected: Option<usize>,
    empty_message: Option<String>,
    on_pick: Option<Box<dyn Fn(usize) -> M + 'a>>,
    on_move: Option<Box<dyn Fn(Direction) -> M + 'a>>,
    on_focus: Option<M>,
    on_dismiss: Option<M>,
}

impl<'a, M: Clone + 'a> Typeahead<'a, M> {
    /// A field showing `query` over the already-matched, already-ranked `rows`, emitting
    /// `on_input` on every keystroke, themed by `roles`.
    ///
    /// The component does no matching, ranking or ordering of its own: it renders `rows` in the
    /// order given (contract §1.1).
    ///
    /// Rows arrive owned, as [`TreeView`](super::TreeView)'s items do: a caller builds them from
    /// whatever it matched this frame, so there is nowhere for them to live between frames.
    pub fn new(
        query: &'a str,
        rows: Vec<Row>,
        on_input: impl Fn(String) -> M + 'a,
        roles: Roles,
    ) -> Self {
        Self {
            query,
            rows,
            on_input: Box::new(on_input),
            roles,
            placeholder: "Search…".to_string(),
            label: None,
            supporting: None,
            open: false,
            highlighted: None,
            selected: None,
            empty_message: None,
            on_pick: None,
            on_move: None,
            on_focus: None,
            on_dismiss: None,
        }
    }

    /// Text shown when the field is empty.
    /// The control's name, rendered inside its container above the value (FR-031a).
    ///
    /// Forwarded to the search field's own `FormField`, so the branch picker's label sits where
    /// every other field's does. §7.7's migration table predates this control — it names the select
    /// the type-ahead replaced — and design-tokens §7.7 extends the same requirement here.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Explanatory text beneath the container.
    pub fn supporting(mut self, text: impl Into<String>) -> Self {
        self.supporting = Some(text.into());
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Whether the result list is showing. The caller owns this, so "open" can outlast an empty
    /// result set — which is what lets [`Self::empty_message`] ever be seen.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Which row the keyboard is on.
    pub fn highlighted(mut self, index: Option<usize>) -> Self {
        self.highlighted = index;
        self
    }

    /// Which row is the caller's current selection. Marked distinctly from the keyboard highlight,
    /// because both can sit on the same row at once (contract §4.7).
    pub fn selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }

    /// What the list says when the query matches nothing. Without it, an empty list shows nothing.
    pub fn empty_message(mut self, message: impl Into<String>) -> Self {
        self.empty_message = Some(message.into());
        self
    }

    /// Emitted with the index of a chosen row. A disabled row never emits it.
    pub fn on_pick(mut self, f: impl Fn(usize) -> M + 'a) -> Self {
        self.on_pick = Some(Box::new(f));
        self
    }

    /// Emitted when the keyboard moves through the results.
    pub fn on_move(mut self, f: impl Fn(Direction) -> M + 'a) -> Self {
        self.on_move = Some(Box::new(f));
        self
    }

    /// Emitted when the field takes focus, so the caller can open the list.
    pub fn on_focus(mut self, message: M) -> Self {
        self.on_focus = Some(message);
        self
    }

    /// Emitted when the list should close without a pick.
    pub fn on_dismiss(mut self, message: M) -> Self {
        self.on_dismiss = Some(message);
        self
    }
}
impl<'a, M: Clone + 'a> From<Typeahead<'a, M>> for Element<'a, M> {
    fn from(t: Typeahead<'a, M>) -> Self {
        let Typeahead {
            query,
            rows,
            on_input,
            roles: r,
            placeholder,
            label,
            supporting,
            open,
            highlighted,
            selected,
            empty_message,
            on_pick,
            on_move,
            on_focus,
            on_dismiss,
        } = t;

        let highlighted_enabled = highlighted
            .and_then(|i| rows.get(i))
            .is_some_and(|row| row.enabled);
        let row_count = rows.len();

        let panel = menu_element(
            rows,
            highlighted,
            selected,
            empty_message,
            on_pick.as_deref(),
            r,
        );

        // Clearing is emptying the query, so it goes through the caller's own input handler
        // rather than a message of its own — resolved here, before the handler moves into the
        // field (FR-016).
        let cleared = on_input(String::new());

        // The field is the library's own text field, with Material's two affordances in their
        // named slots: the search icon leading, the clear action trailing. Neither is assembled
        // here — `TextField` grew both, so the next searchable picker gets them by asking.
        //
        // The clear action appears only when there is something to clear, so an empty field
        // carries no action that would do nothing.
        // `.active(open)` is the half the select cannot manage. §7.7 wants the active indicator to
        // follow **open** rather than focus (FR-043a), and `pick_list` reports its open state to
        // its own style closure and to nobody else — so `Select::active` must be supplied and in
        // practice is not. This control's openness is already a caller-held value, so the
        // indicator follows it for real. The accepted gap stands for the select and closes here.
        let mut input = super::TextField::new(placeholder, query, r)
            .leading_icon(Icon::Search)
            .active(open)
            .on_input(on_input);
        // Nothing here has to say whether the label rests or floats: `TextField` derives that from
        // its own value, which for the type-ahead *is* the query — so a picker with an empty search
        // box rests its label exactly as an empty input does, and one being typed into floats it.
        if let Some(label) = label {
            input = input.label(label);
        }
        if let Some(supporting) = supporting {
            input = input.supporting(supporting);
        }
        if !query.is_empty() {
            input = input.trailing_action(Icon::Close, cleared);
        }
        let field: Element<'a, M> = input.into();

        // The list arrives and leaves rather than appearing and vanishing (FR-018, FR-019). The
        // wrapper animates what it looks like; `.exit(…)` tells the behaviour half how long to keep
        // producing an overlay at all, so the two cannot disagree about when the list is gone.
        let menu = animated_menu(panel, open, r);

        let mut behaviour = Behaviour::new(field, menu, open, GAP).exit(EXIT).keyboard(
            highlighted,
            row_count,
            highlighted_enabled,
        );

        // Enter takes the highlighted row, so the message is resolved here rather than in the
        // behaviour half — which knows an index but nothing about what sits at it.
        if let (Some(on_pick), Some(index)) = (&on_pick, highlighted) {
            if highlighted_enabled {
                behaviour = behaviour.on_pick(on_pick(index));
            }
        }
        if let Some(on_move) = on_move {
            behaviour = behaviour.on_move(on_move);
        }
        if let Some(on_dismiss) = on_dismiss {
            behaviour = behaviour.on_dismiss(on_dismiss);
        }
        if let Some(on_focus) = on_focus {
            behaviour = behaviour.on_focus(on_focus);
        }

        behaviour.into()
    }
}
