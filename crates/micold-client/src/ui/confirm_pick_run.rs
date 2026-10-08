//! The still-working confirmation of **Pick this one** (feature 483, parallel-surfaces C7).

use crate::app::{Message, State};
use crate::features::runs::{Msg, PICK_WORKING_CONFIRMATION};
use crate::ui::material::{self, Button, SurfaceKind, Text, TypeRole};
use iced::widget::{column, row};
use iced::{Element, Length};
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, Roles};

/// The dialog body for run `run`.
pub fn modal<'a>(run: u8, r: Roles) -> Element<'a, Message> {
    let fields = material::dialog::fields(column![
        Text::new(format!("Pick run {run}?"), TypeRole::Headline, r),
        Text::new(PICK_WORKING_CONFIRMATION, TypeRole::Body, r).muted(),
    ]);
    let actions = material::dialog::actions(row![
        Button::filled("Pick this one", r).on_press(Message::Runs(Msg::PickConfirmed)),
        Button::outlined("Cancel", r).on_press(Message::Runs(Msg::PickCancelled)),
    ]);
    material::Surface::new(
        material::dialog::body(fields, actions),
        SurfaceKind::Dialog,
        r,
    )
    .width(Length::Fixed(460.0))
    .into()
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
