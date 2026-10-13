//! The confirmation before a daemon is removed, or before a running daemon's runtime settings are
//! applied (feature 491, T035, FR-012, FR-014).
//!
//! Modelled on [`crate::ui::confirm_placement`]: a question about an action already taken, with no
//! way past it but an answer. The sentence is built in the feature, from the counts, so a test can
//! read it; this only lays it out.

use crate::app::{Message, State};
use crate::features::daemons::{Msg as DaemonsMsg, RemovalPrompt};
use crate::ui::material::{self, Text, TypeRole};
use iced::widget::column;
use iced::Element;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::theme::ColorScheme;
use micold_core::tokens;

/// The dialog body for `prompt`.
pub fn modal<'a>(prompt: &'a RemovalPrompt, scheme: ColorScheme) -> Element<'a, Message> {
    let r = tokens::roles(scheme);
    let fields = column![
        Text::new(format!("Daemon {}", prompt.name), TypeRole::Headline, r),
        Text::new(prompt.text.clone(), TypeRole::Body, r).muted(),
    ];
    material::dialog::confirm(
        r,
        fields,
        ("Continue", Message::Daemons(DaemonsMsg::RemovalConfirmed)),
        ("Cancel", Message::Daemons(DaemonsMsg::RemovalCancelled)),
        460.0,
    )
}

/// This dialog's body, built from the state that opened it.
pub fn dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    _env_include_outcome: &'a EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    state
        .daemon_list
        .prompt()
        .map(|prompt| modal(prompt, scheme))
}
