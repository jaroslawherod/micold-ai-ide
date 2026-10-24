//! `MenuSelect` — a select whose options are caller-built rows that may hold chips (feature 1272,
//! Constitution Principle VIII).
//!
//! A text trigger (a fixed label part, an elastic title that ends in `…`, a chevron, a tooltip) and
//! a floating list of rows: options, options with chips and a note, headings and note lines. The
//! review's source control is its first caller: an option per source, with the Working tree
//! option's Committed and Uncommitted chips inside it.
//!
//! **Why not [`Select`](super::Select) or [`MenuOverlay`](super::MenuOverlay).** The select owns its
//! own openness and takes `T: ToString` options, so an option cannot hold chips and the list cannot
//! stay open on a chip press; a menu is icon-and-label actions anchored to a point, with no keyboard
//! highlight over a disabled entry (research R6). Openness and the highlight here are the caller's,
//! as with the [`Typeahead`](super::Typeahead).
//!
//! **Composed, not drawn.** Positioning, capture, dismissal and the keyboard rule are
//! [`cdk::picker`]'s, floating (never in flow), so `tests/one_overlay_implementation.rs` needs no
//! new exception; the list's surface, row states, ripple and transition are `material::picker`'s;
//! the trigger is the library's [`Button`](super::Button), which already answers `Enter` and `Space`
//! once it holds the keyboard.
//!
//! [`cdk::picker`]: crate::ui::cdk::picker
//!
//! Contract: `specs/1272-review-source-dropdown/contracts/source-control.md` §3.

use std::borrow::Cow;

use super::picker::{animated_menu, marker, row_surface, EXIT, GAP, MAX_ROWS_BEFORE_SCROLL};
use super::{ButtonVariant, Ellipsized, Glyph, Text, ToggleChip, TypeRole};
use crate::icons::Icon;
use crate::ui::cdk::picker::Picker as Behaviour;
use iced::widget::{button, column, container, row, Space};
use iced::{alignment, Element, Length};
use micold_core::tokens::{density, shape, spacing, state, Roles};
use micold_core::typeahead::Direction;

/// The list's height before it scrolls, unless the caller sets its own: the picker's eight menu
/// items.
pub const DEFAULT_LIST_HEIGHT: f32 = density::MENU_ITEM_BASE * MAX_ROWS_BEFORE_SCROLL;

/// The role an option's note is set in: supporting text under the option, as a picker row's second
/// line is.
const NOTE_ROLE: TypeRole = TypeRole::Caption;

/// One line of a [`MenuSelect`]'s list. Built with [`Self::option`], [`Self::option_with_chips`],
/// [`Self::heading`] or [`Self::note`].
///
/// An option is a keyboard stop, and each of its chips is one more, in order; a heading or a note
/// is none and takes no press. That order is what [`MenuSelect::keyboard`]'s highlight counts in.
pub struct MenuSelectRow<'a, M> {
    line: Line<'a, M>,
}

enum Line<'a, M> {
    Option {
        content: Element<'a, M>,
        chosen: bool,
        on_pick: M,
        chips: Vec<ToggleChip<M>>,
        note: Option<Cow<'a, str>>,
    },
    Heading(Cow<'a, str>),
    Note(Cow<'a, str>),
}

impl<'a, M> MenuSelectRow<'a, M> {
    /// An option showing `content`, marked when `chosen`, sending `on_pick` when pressed.
    pub fn option(content: impl Into<Element<'a, M>>, chosen: bool, on_pick: M) -> Self {
        Self::option_with_chips(content, chosen, on_pick, Vec::new(), None::<&str>)
    }

    /// An option with `chips` beside its content and an optional `note` under it. A chip press
    /// sends the chip's own message and nothing else: whether the list stays open is the caller's.
    pub fn option_with_chips(
        content: impl Into<Element<'a, M>>,
        chosen: bool,
        on_pick: M,
        chips: Vec<ToggleChip<M>>,
        note: Option<impl Into<Cow<'a, str>>>,
    ) -> Self {
        Self {
            line: Line::Option {
                content: content.into(),
                chosen,
                on_pick,
                chips,
                note: note.map(Into::into),
            },
        }
    }

    /// A heading above a group of options.
    pub fn heading(text: impl Into<Cow<'a, str>>) -> Self {
        Self {
            line: Line::Heading(text.into()),
        }
    }

    /// A muted line in place of options, or about them.
    pub fn note(text: impl Into<Cow<'a, str>>) -> Self {
        Self {
            line: Line::Note(text.into()),
        }
    }

    /// How many keyboard stops this row holds.
    fn stops(&self) -> usize {
        match &self.line {
            Line::Option { chips, .. } => 1 + chips.len(),
            Line::Heading(_) | Line::Note(_) => 0,
        }
    }
}

/// A trigger and a floating list of [`MenuSelectRow`]s, open while the caller says so.
///
/// Builder form (Principle VIII):
/// `MenuSelect::new(fixed, rows, open, on_toggle, on_dismiss, roles).title(t).keyboard(..).into()`.
pub struct MenuSelect<'a, M> {
    fixed: Cow<'a, str>,
    title: Option<&'a str>,
    rows: Vec<MenuSelectRow<'a, M>>,
    open: bool,
    on_toggle: M,
    on_dismiss: M,
    roles: Roles,
    tooltip: Option<String>,
    highlight: Option<usize>,
    stops: usize,
    highlighted_enabled: bool,
    on_move: Option<Box<dyn Fn(Direction) -> M + 'a>>,
    on_pick: Option<M>,
    max_list_height: f32,
    width: Length,
}

impl<'a, M: Clone + 'a> MenuSelect<'a, M> {
    /// A trigger reading `fixed_label` over `rows`, its list shown while `open`. A press on the
    /// trigger, or `Enter`/`Space` on it, sends `on_toggle`; a press outside the open list, `Esc` or
    /// `Tab` sends `on_dismiss`.
    pub fn new(
        fixed_label: impl Into<Cow<'a, str>>,
        rows: Vec<MenuSelectRow<'a, M>>,
        open: bool,
        on_toggle: M,
        on_dismiss: M,
        roles: Roles,
    ) -> Self {
        Self {
            fixed: fixed_label.into(),
            title: None,
            rows,
            open,
            on_toggle,
            on_dismiss,
            roles,
            tooltip: None,
            highlight: None,
            stops: 0,
            highlighted_enabled: false,
            on_move: None,
            on_pick: None,
            max_list_height: DEFAULT_LIST_HEIGHT,
            width: Length::Fill,
        }
    }

    /// The label's elastic part, after the fixed one: the only part that ends in `…` when the
    /// trigger is too narrow (research R5).
    pub fn title(mut self, title: Option<&'a str>) -> Self {
        self.title = title;
        self
    }

    /// What the trigger's tooltip says: the full label, and whatever else the caller adds.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Which stop the keyboard is on, how many stops the list has, and whether the highlighted one
    /// can be taken. Stops count options and their chips in row order (see [`MenuSelectRow`]).
    pub fn keyboard(
        mut self,
        highlight: Option<usize>,
        stops: usize,
        highlighted_enabled: bool,
    ) -> Self {
        self.highlight = highlight;
        self.stops = stops;
        self.highlighted_enabled = highlighted_enabled;
        self
    }

    /// Sent when an arrow key moves the highlight.
    pub fn on_move(mut self, f: impl Fn(Direction) -> M + 'a) -> Self {
        self.on_move = Some(Box::new(f));
        self
    }

    /// Sent when `Enter` takes the highlighted stop.
    pub fn on_pick(mut self, message: M) -> Self {
        self.on_pick = Some(message);
        self
    }

    /// The list's height before it scrolls inside itself. It is never taller than the room below
    /// (or above) the trigger either.
    pub fn max_list_height(mut self, height: f32) -> Self {
        self.max_list_height = height;
        self
    }

    /// The trigger's width, which the list takes too. `Fill` unless set.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }
}

/// The keyboard highlight on a chip: §5's focus ring, as a focused control draws it, since a chip
/// in the list never holds the keyboard itself.
fn ringed<'a, M: Clone + 'a>(chip: ToggleChip<M>, ringed: bool, r: Roles) -> Element<'a, M> {
    let chip: Element<'a, M> = chip.into();
    if !ringed {
        return chip;
    }
    container(chip)
        .style(move |_| container::Style {
            border: iced::Border {
                color: super::style::color(r.secondary),
                width: state::FOCUS_RING_WIDTH,
                radius: shape::FULL.into(),
            },
            ..container::Style::default()
        })
        .into()
}

/// One option: the chosen marker, the content, its chips, and the note under them, on one
/// pressable row surface. `first` is the option's own stop index.
#[allow(clippy::too_many_arguments)]
fn option_element<'a, M: Clone + 'a>(
    content: Element<'a, M>,
    chosen: bool,
    on_pick: M,
    chips: Vec<ToggleChip<M>>,
    note: Option<Cow<'a, str>>,
    first: usize,
    highlight: Option<usize>,
    r: Roles,
) -> Element<'a, M> {
    let mut line = row![container(content).width(Length::Fill)]
        .spacing(spacing::XS)
        .align_y(alignment::Vertical::Center);
    for (i, chip) in chips.into_iter().enumerate() {
        line = line.push(ringed(chip, highlight == Some(first + 1 + i), r));
    }
    let mut body = column![line].spacing(spacing::XS).width(Length::Fill);
    if let Some(note) = note {
        body = body.push(Text::new(note, NOTE_ROLE, r).muted());
    }
    // The marker centred on the first line, not on the row, as a two-line picker row places it.
    let mark = container(marker(chosen, r))
        .height(Length::Fixed(density::MENU_ITEM_BASE - 2.0 * spacing::XS))
        .align_y(alignment::Vertical::Center);
    // The menu item's height is the least the row is, as its touch target (see the picker's
    // two-line row); a strut holds it, and taller content sets the height.
    let strut = Space::new()
        .width(Length::Shrink)
        .height(Length::Fixed(density::MENU_ITEM_BASE - 2.0 * spacing::XS));
    let content = row![strut, mark, body]
        .spacing(spacing::SM)
        .align_y(alignment::Vertical::Top);
    let surface = button(content)
        .width(Length::Fill)
        .height(Length::Shrink)
        .padding([spacing::XS, spacing::SM]);
    row_surface(surface, highlight == Some(first), chosen, Some(on_pick), r)
}

/// A heading or a note line: text on the list, with nothing to press.
fn line_element<'a, M: 'a>(text: Text<'a, M>) -> Element<'a, M> {
    container(text)
        .width(Length::Fill)
        .padding([spacing::XS, spacing::SM])
        .into()
}

impl<'a, M: Clone + 'a> From<MenuSelect<'a, M>> for Element<'a, M> {
    fn from(s: MenuSelect<'a, M>) -> Self {
        let r = s.roles;

        let mut list = column![].width(Length::Fill);
        let mut stop = 0usize;
        for item in s.rows {
            let stops = item.stops();
            list = list.push(match item.line {
                Line::Option {
                    content,
                    chosen,
                    on_pick,
                    chips,
                    note,
                } => option_element(content, chosen, on_pick, chips, note, stop, s.highlight, r),
                Line::Heading(text) => line_element(Text::new(text, TypeRole::Section, r)),
                Line::Note(text) => line_element(Text::new(text, NOTE_ROLE, r).muted()),
            });
            stop += stops;
        }
        // The cap is a layout constraint, as the picker's is: the overlay already refuses to grow
        // past the room on screen, and this stops a long list from taking all of it.
        let capped = container(super::Scrollable::new(list, r).height(Length::Shrink))
            .max_height(s.max_list_height);
        let panel = super::menu_panel(capped, Length::Fill, r, true, spacing::XS);
        let menu = animated_menu(panel, s.open, r);

        // The trigger: the fixed part whole, the title in what is left, the chevron at the end.
        let elastic: Element<'a, M> = match s.title {
            Some(title) => Ellipsized::at_role_spaced(title, TypeRole::Body, r.on_surface).into(),
            None => Space::new().width(Length::Fill).into(),
        };
        let label = row![
            Text::new(s.fixed, TypeRole::Body, r),
            elastic,
            Glyph::new(Icon::SelectChevron, TypeRole::Action, r).tint(r.on_surface_variant),
        ]
        .spacing(spacing::XS)
        .align_y(alignment::Vertical::Center)
        .width(Length::Fill);
        let trigger = super::Button::with_content(label, ButtonVariant::Outlined, r)
            .width(s.width)
            .on_press(s.on_toggle);

        let mut behaviour = Behaviour::new(trigger, menu, s.open, GAP)
            .exit(EXIT)
            .keyboard(s.highlight, s.stops, s.highlighted_enabled)
            .on_dismiss(s.on_dismiss);
        if let Some(on_move) = s.on_move {
            behaviour = behaviour.on_move(on_move);
        }
        if let (Some(on_pick), true) = (s.on_pick, s.highlighted_enabled) {
            behaviour = behaviour.on_pick(on_pick);
        }
        let element: Element<'a, M> = behaviour.into();
        match s.tooltip {
            // Around the whole control, not the trigger alone: the picker floats only its list, so
            // a tooltip inside it would never be shown.
            Some(tip) => super::Tooltip::new(element, tip, r).into(),
            None => element,
        }
    }
}
