//! The Dismiss group confirmation (feature 483, parallel-surfaces G5). Dismissing forgets the
//! grouping only; the dialog says what stays.

use crate::app::{Message, State};
use crate::features::runs::{Msg, DISMISS_CONFIRMATION};
use crate::ui::material::{self, Text, TypeRole};
use iced::widget::column;
use iced::Element;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, Roles};

/// The dialog body for the group named `name`.
pub fn modal<'a>(name: &str, r: Roles) -> Element<'a, Message> {
    let fields = column![
        Text::new(format!("Dismiss “{name}”?"), TypeRole::Headline, r),
        Text::new(DISMISS_CONFIRMATION, TypeRole::Body, r).muted(),
    ];
    material::dialog::confirm(
        r,
        fields,
        ("Dismiss group", Message::Runs(Msg::DismissConfirmed)),
        ("Cancel", Message::Runs(Msg::DismissCancelled)),
        460.0,
    )
}

/// The registered body, from the group the confirmation is about.
pub fn dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    _env_include_outcome: &'a EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    let target = state.runs.dismiss_target.as_ref()?;
    let name = state
        .runs
        .groups
        .iter()
        .find(|group| group.id == target.group)
        .map(|group| group.name.as_str())?;
    Some(modal(name, tokens::roles(scheme)))
}
