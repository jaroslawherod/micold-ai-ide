//! The still-working confirmation of **Pick this one** (feature 483, parallel-surfaces C7).

use crate::app::{Message, State};
use crate::features::runs::{Msg, PICK_WORKING_CONFIRMATION};
use crate::ui::material::{self, Text, TypeRole};
use iced::widget::column;
use iced::Element;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, Roles};

/// The dialog body for run `run`.
pub fn modal<'a>(run: u8, r: Roles) -> Element<'a, Message> {
    let fields = column![
        Text::new(format!("Pick run {run}?"), TypeRole::Headline, r),
        Text::new(PICK_WORKING_CONFIRMATION, TypeRole::Body, r).muted(),
    ];
    material::dialog::confirm(
        r,
        fields,
        ("Pick this one", Message::Runs(Msg::PickConfirmed)),
        ("Cancel", Message::Runs(Msg::PickCancelled)),
        460.0,
    )
}

/// The registered body, from the pick the confirmation is about.
pub fn dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    _env_include_outcome: &'a EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    let target = state.runs.pick_target.as_ref()?;
    Some(modal(target.run, tokens::roles(scheme)))
}
