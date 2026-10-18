//! The quick switcher's dialog (feature 968, contracts/ui.md U6): one search field over the ranked
//! list. Glue only — every decision (what is listed, in what order, what is highlighted, why the
//! list is empty) comes from [`quick_switch::list`]; this module lays it out.

use std::sync::LazyLock;

use iced::widget::column;
use iced::{Element, Length};
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::navigation::Dir;
use micold_core::quick_switch::EntryKey;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, Roles};
use micold_core::typeahead::Direction;

use crate::app::{Message, State};
use crate::features::quick_switch::{self, Empty, Kind, Msg};
use crate::ui::material::{self, SurfaceKind, TypeaheadRow};

/// The search field's widget id: the keyboard goes here when the switcher opens (research R8).
pub static QUICK_SWITCHER_FIELD_ID: LazyLock<iced::widget::Id> =
    LazyLock::new(|| iced::widget::Id::new("quick-switcher-field"));

/// The dialog's width, wide enough for a long name and its context line.
const WIDTH: f32 = 560.0;

/// What the dialog shows, as plain data: the showcase poses it without a [`State`].
pub struct View<'a> {
    pub query: &'a str,
    /// The rows in list order.
    pub rows: Vec<TypeaheadRow>,
    /// The entry each row stands for, by position.
    pub keys: Vec<EntryKey>,
    pub highlighted: Option<usize>,
    /// The row of the current location, session or project.
    pub current: Option<usize>,
    pub empty_message: &'static str,
}

/// The word a kind is shown under on a row's second line.
fn kind_label(kind: Kind) -> &'static str {
    match kind {
        Kind::Location => "Location",
        Kind::Session => "Session",
        Kind::Project => "Project",
    }
}

/// A row's second line: `"<Kind> · <context>"`, or the kind alone when there is no context.
pub fn context_line(kind: Kind, context: &str) -> String {
    if context.is_empty() {
        kind_label(kind).to_string()
    } else {
        format!("{} · {context}", kind_label(kind))
    }
}

/// The dialog's data for the open switcher (empty when closed).
pub fn view_of(state: &State) -> View<'_> {
    let list = quick_switch::list(state);
    let mut rows = Vec::new();
    let mut keys = Vec::new();
    let mut current = None;
    for (at, ranked) in list.ranked.iter().enumerate() {
        let entry = &list.entries[ranked.index];
        let row = TypeaheadRow::new(entry.candidate.name.clone(), ranked.spans.clone())
            .single_line()
            .details(context_line(entry.kind, &entry.context), Vec::new());
        rows.push(if entry.candidate.selectable {
            row
        } else {
            row.disabled()
        });
        keys.push(entry.candidate.key.clone());
        if entry.current {
            current = Some(at);
        }
    }
    View {
        query: state
            .quick_switch
            .open
            .as_ref()
            .map_or("", |open| open.query.as_str()),
        rows,
        keys,
        highlighted: list.highlighted,
        current,
        empty_message: match list.empty {
            Some(Empty::NothingToSwitchTo) => "Nothing to switch to",
            Some(Empty::NoMatches) | None => "No matches",
        },
    }
}

/// The dialog's surface around `view`, saying what happens in switcher messages.
pub fn surface<'a>(view: View<'a>, roles: Roles) -> Element<'a, Msg> {
    let View {
        query,
        rows,
        keys,
        highlighted,
        current,
        empty_message,
    } = view;
    let typeahead = material::Typeahead::new(query, rows, Msg::QueryChanged, roles)
        .id(QUICK_SWITCHER_FIELD_ID.clone())
        .placeholder("Jump to…")
        .open(true)
        .in_flow(true)
        .highlighted(highlighted)
        .selected(current)
        .empty_message(empty_message)
        .on_move(|direction| {
            Msg::Moved(match direction {
                Direction::Prev => Dir::Previous,
                Direction::Next => Dir::Next,
            })
        })
        .on_pick(move |at| keys.get(at).cloned().map_or(Msg::Dismissed, Msg::Chosen))
        .on_dismiss(Msg::Dismissed);
    material::Surface::new(
        material::dialog::fields(column![typeahead]),
        SurfaceKind::Dialog,
        roles,
    )
    .width(Length::Fixed(WIDTH))
    .into()
}

/// This dialog's body for the registry (the uniform shape of every registered dialog, see
/// [`crate::ui::about::dialog`]); `None` when the switcher is closed.
pub fn dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    _env_include_outcome: &'a EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    state.quick_switch.open.as_ref()?;
    Some(surface(view_of(state), tokens::roles(scheme)).map(Message::QuickSwitch))
}
