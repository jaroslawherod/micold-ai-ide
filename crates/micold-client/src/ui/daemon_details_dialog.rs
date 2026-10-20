//! A daemon's details, read-only, as a modal dialog with Close (feature 1092, FR-012).
//!
//! The body is the Daemons page's own [`details_block`](crate::ui::settings::daemons::details_block),
//! computed from live state on every view, so a new log line or error shows without reopening.

use crate::app::{Message, State};
use crate::features::daemons::Msg as DaemonsMsg;
use crate::ui::material::{dialog, Button, ScrollingDialog};
use crate::ui::settings::daemons::details_block;
use iced::widget::row;
use iced::Element;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::theme::ColorScheme;
use micold_core::tokens;

/// This dialog's body, built from the state that opened it.
pub fn dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    _env_include_outcome: &'a EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    let id = state.daemon_list.details?;
    let name = state.settings.daemons.get(id)?.name.as_str().to_string();
    let r = tokens::roles(scheme);
    let body = details_block(crate::features::daemons::details(&state.daemon_list, id), r);
    let actions = dialog::actions(row![
        Button::filled("Close", r).on_press(Message::Daemons(DaemonsMsg::DetailsClosed)),
    ]);
    let mut built = ScrollingDialog::new(name, body, actions, r).actions_height(40.0);
    let (_, height) = state.window.window_size;
    if height > 0 {
        built = built.max_height(f32::from(height));
    }
    Some(built.into())
}
