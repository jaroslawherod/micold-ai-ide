//! #942's parts of a structured session's composer (feature 941, plan I-1): the mode `Select`
//! beside its text area, showing the session's permission mode (FR-015) and changing it (FR-017,
//! Bypass only after its confirmation); why a change was refused, at the end of the status row; and
//! *Waiting for you* in place of the busy label while a request of the session waits (FR-009).
//!
//! The `Select` sits in the input row rather than the status row: a filled field is taller than the
//! status row's words and would add its height to every composer, while the input row is as tall
//! as the text area already (M9, ledger decision 21).

use std::fmt;

use iced::widget::container;
use iced::Length;
use micold_core::session::{PermissionMode, SessionId};
use micold_core::tokens::Roles;

use crate::app::{Message, State};
use crate::features::permission_prompts::Msg as PromptsMsg;
use crate::ui::composer::Seams;
use crate::ui::material::{Select, Text, TypeRole};

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
    let select = Select::new(
        &MODE_CHOICES,
        Some(ModeChoice(mode)),
        move |choice| chosen(session, choice),
        roles,
    )
    .label(MODE_LABEL);
    Seams {
        status: prompts
            .notice(session)
            .map(|why| Text::new(why, TypeRole::Caption, roles).into()),
        waiting: prompts.waiting(session),
        control: Some(container(select).width(Length::Fixed(MODE_WIDTH)).into()),
        ..Seams::default()
    }
}
