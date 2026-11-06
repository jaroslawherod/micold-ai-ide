//! #942's parts of a structured session's composer (feature 941, plan I-1): the mode `MenuSelect`
//! (feature 1703 FR-015, FR-017: one selector style), showing the session's permission mode (FR-015) and changing it (FR-017,
//! Bypass only after its confirmation); why a change was refused, at the end of the status row; and
//! *Waiting for you* in place of the busy label while a request of the session waits (FR-009).
//!
//! The selector sits in the composer's session options row, behind the settings icon (feature 1703
//! FR-008); it was in the input row before. Its openness and highlight are
//! [`permission_prompts::State::mode_menu`](crate::features::permission_prompts::State::mode_menu)'s.

use std::fmt;

use micold_core::session::{PermissionMode, SessionId};
use micold_core::tokens::Roles;

use crate::app::{Message, State};
use crate::features::permission_prompts::Msg as PromptsMsg;
use crate::ui::composer::Seams;
use crate::ui::material::{MenuSelect, MenuSelectRow, Text, TypeRole};

/// The mode control's label.
pub const MODE_LABEL: &str = "Permissions";

/// The mode control's width: the longest mode name, *Accept edits*, with its chevron.
pub const MODE_WIDTH: f32 = 180.0;

/// One mode as the control lists it, by the name the user reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModeChoice(pub PermissionMode);

impl fmt::Display for ModeChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.label())
    }
}

/// The modes the control offers, in [`PermissionMode::ALL`]'s order.
pub const MODE_CHOICES: [ModeChoice; 4] = [
    ModeChoice(PermissionMode::ALL[0]),
    ModeChoice(PermissionMode::ALL[1]),
    ModeChoice(PermissionMode::ALL[2]),
    ModeChoice(PermissionMode::ALL[3]),
];

/// What picking `choice` for `session` sends: the shell sends `SetPermissionMode`, or for Bypass
/// opens its confirmation first.
pub fn chosen(session: SessionId, choice: ModeChoice) -> Message {
    Message::PermissionPrompts(PromptsMsg::ModeRequested {
        session,
        mode: choice.0,
    })
}

/// `session`'s composer seams, its summary saying it runs in `mode`.
pub fn seams<'a>(
    state: &'a State,
    session: SessionId,
    mode: PermissionMode,
    roles: Roles,
) -> Seams<'a> {
    let prompts = &state.permission_prompts;
    let open = prompts.mode_menu(session);
    let rows = MODE_CHOICES
        .iter()
        .map(|choice| {
            MenuSelectRow::option(
                Text::new(choice.to_string(), TypeRole::Body, roles),
                choice.0 == mode,
                chosen(session, *choice),
            )
        })
        .collect();
    let current = MODE_CHOICES.iter().position(|c| c.0 == mode).unwrap_or(0);
    let highlighted = open.flatten();
    let select = MenuSelect::new(
        format!("{MODE_LABEL}:"),
        rows,
        open.is_some(),
        Message::PermissionPrompts(PromptsMsg::ModeMenuToggled { session, current }),
        Message::PermissionPrompts(PromptsMsg::ModeMenuClosed(session)),
        roles,
    )
    .title(Some(mode.label()))
    .tooltip(format!("{MODE_LABEL}: {}", mode.label()))
    .keyboard(highlighted, MODE_CHOICES.len(), highlighted.is_some())
    .on_move(move |direction| {
        Message::PermissionPrompts(PromptsMsg::ModeMenuMoved { session, direction })
    })
    .width(MODE_WIDTH);
    let select = match highlighted.and_then(|at| MODE_CHOICES.get(at)) {
        Some(choice) => select.on_pick(chosen(session, *choice)),
        None => select,
    };
    Seams {
        status: prompts
            .notice(session)
            .map(|why| Text::new(why, TypeRole::Caption, roles).into()),
        waiting: prompts.waiting(session),
        control: Some(select.into()),
        bypass: mode == PermissionMode::Bypass,
        ..Seams::default()
    }
}
