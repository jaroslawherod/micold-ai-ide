//! The `Bind to daemon...` picker for a worktree that shows "no daemon" (feature 491, T036,
//! FR-014).
//!
//! One button per daemon of the registry, or, with none configured, the way to add one. Binding is
//! always this explicit action: a worktree never picks a daemon for itself.

use crate::app::{Message, State};
use crate::features::daemons::Msg as DaemonsMsg;
use crate::ui::material::{self, Button, SurfaceKind, Text, TypeRole};
use iced::widget::{column, row};
use iced::{Element, Length};
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::theme::ColorScheme;
use micold_core::tokens;

/// The picker's explanation when no daemon is configured.
pub const NONE_CONFIGURED: &str = "No daemons are set up. Add one in Settings, under Daemons, \
     then bind this worktree to it.";

/// This dialog's body, built from the state that opened it.
pub fn dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    _env_include_outcome: &'a EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    let target = state.daemon_list.bind_target.as_ref()?;
    let r = tokens::roles(scheme);
    let list = state.daemon_rows();
    let what = if target.worktree.is_empty() {
        "the Default location".to_string()
    } else {
        format!("worktree {}", target.worktree)
    };
    let mut fields = column![
        Text::new("Bind to daemon", TypeRole::Headline, r),
        Text::new(
            format!("Choose the daemon {what} runs on. Sessions and terminals start there."),
            TypeRole::Body,
            r
        )
        .muted(),
    ]
    .spacing(tokens::spacing::SM);
    if list.bind_choices.is_empty() {
        fields = fields.push(Text::new(NONE_CONFIGURED, TypeRole::Body, r).muted());
    }
    for (id, name) in list.bind_choices {
        fields = fields
            .push(Button::outlined(name, r).on_press(Message::Daemons(DaemonsMsg::BindChosen(id))));
    }
    let buttons =
        row![Button::outlined("Cancel", r).on_press(Message::Daemons(DaemonsMsg::BindCancelled)),];
    let actions = material::dialog::actions(buttons);
    Some(
        material::Surface::new(
            material::dialog::body(material::dialog::fields(fields), actions),
            SurfaceKind::Dialog,
            r,
        )
        .width(Length::Fixed(460.0))
        .into(),
    )
}
