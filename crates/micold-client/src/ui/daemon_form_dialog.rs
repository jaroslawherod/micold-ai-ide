//! The add and edit daemon form, as a modal dialog with its own Save daemon and Cancel (feature
//! 1092, FR-001 to FR-003, FR-008, FR-011).
//!
//! The form is the Daemons page's own [`form`](crate::ui::settings::daemons::form), moved out of
//! the page. This lays it in a [`ScrollingDialog`]: the body scrolls when the window is short and
//! the actions stay visible. A running daemon's counted confirmation replaces the actions row, so
//! it is one dialog, not a second modal over the first.

use crate::app::{Message, State};
use crate::features::daemons::Msg as DaemonsMsg;
use crate::features::settings::missing_cli_notice;
use crate::ui::material::{dialog, Button, ScrollingDialog, Text, TypeRole};
use crate::ui::settings::daemons::{form, MachineFacts};
use iced::widget::{column, row};
use iced::Element;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, spacing};

/// The height of the confirmation region: the sentence over its buttons.
const CONFIRMATION_HEIGHT: f32 = 120.0;

/// This dialog's body, built from the state that opened it.
pub fn dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    _env_include_outcome: &'a EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    let list = &state.daemon_list;
    let draft = list.draft.as_ref()?;
    let r = tokens::roles(scheme);
    let facts = MachineFacts {
        draft: state.settings.settings_draft.as_ref().map(|d| &d.daemon),
        legacy_default: state.daemon_rows().legacy_default,
        missing_cli: missing_cli_notice(state.session.availability.home()),
    };
    let title = match (draft.editing, draft.name.trim()) {
        (None, _) => "Add daemon".to_string(),
        (Some(_), "") => "Edit daemon".to_string(),
        (Some(_), name) => name.to_string(),
    };
    let body = form(draft, list, facts, state.window.focused_field, r);
    let (actions, actions_height): (Element<'a, Message>, f32) = match list.form_confirmation() {
        Some(prompt) => (
            column![
                Text::new(prompt.text.clone(), TypeRole::Body, r).muted(),
                dialog::actions(row![
                    Button::filled(prompt.confirm_label, r)
                        .on_press(Message::Daemons(DaemonsMsg::RemovalConfirmed)),
                    Button::outlined(prompt.decline_label, r)
                        .on_press(Message::Daemons(DaemonsMsg::RemovalCancelled)),
                ]),
            ]
            .spacing(spacing::SM)
            .into(),
            CONFIRMATION_HEIGHT,
        ),
        None => (
            dialog::actions(row![
                Button::filled("Save daemon", r).on_press(Message::Daemons(DaemonsMsg::Saved)),
                Button::outlined("Cancel", r)
                    .on_press(Message::Daemons(DaemonsMsg::DraftDiscarded)),
            ])
            .into(),
            40.0,
        ),
    };
    let mut built = ScrollingDialog::new(title, body, actions, r).actions_height(actions_height);
    let (_, height) = state.window.window_size;
    if height > 0 {
        built = built.max_height(f32::from(height));
    }
    Some(built.into())
}
