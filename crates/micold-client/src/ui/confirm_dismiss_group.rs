//! The Dismiss group confirmation (feature 483, parallel-surfaces G5). Dismissing forgets the
//! grouping only; the dialog says what stays.

use crate::app::{Message, State};
use crate::features::runs::{Msg, DISMISS_CONFIRMATION};
use crate::ui::material::{self, Button, SurfaceKind, Text, TypeRole};
use iced::widget::{column, row};
use iced::{Element, Length};
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self};

/// The dialog body for the group named `name`.
pub fn modal<'a>(name: &str, scheme: ColorScheme) -> Element<'a, Message> {
    let r = tokens::roles(scheme);
    let fields = material::dialog::fields(column![
        Text::new(format!("Dismiss “{name}”?"), TypeRole::Headline, r),
        Text::new(DISMISS_CONFIRMATION, TypeRole::Body, r).muted(),
    ]);
    let actions = material::dialog::actions(row![
        Button::filled("Dismiss group", r).on_press(Message::Runs(Msg::DismissConfirmed)),
        Button::outlined("Cancel", r).on_press(Message::Runs(Msg::DismissCancelled)),
    ]);
    material::Surface::new(
        material::dialog::body(fields, actions),
        SurfaceKind::Dialog,
        r,
    )
    .width(Length::Fixed(460.0))
    .into()
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
    Some(modal(name, scheme))
}
