//! The confirm dialog for starting another session in an occupied worktree (feature 776, FR-016).
//! The one-session rule is soft: the user may ask for a helper, after being told what is there.

use crate::app::{Message, State};
use crate::features::session::Msg as SessionMsg;
use crate::ui::material::{self, Text, TypeRole};
use iced::widget::column;
use iced::Element;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::session::SessionLocation;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self};

/// The dialog's text: `place` names the worktree (or the project root) and `existing` its session,
/// when this window's catalog knows it. A refusal can open the dialog before the catalog has heard
/// of the occupant (feature 802, FR-001), and then the text says so without naming a session.
pub fn body_text(place: &str, existing: Option<&str>) -> String {
    match existing {
        Some(name) => format!(
            "{place} already has the session \u{201c}{name}\u{201d}. The new session is a helper: \
             it runs beside it and the existing one stays the main session."
        ),
        None => format!(
            "{place} already has a session. The new session is a helper: it runs beside it and \
             the existing one stays the main session."
        ),
    }
}

/// The dialog body: `place` names the worktree (or the project root) and `existing` its session.
pub fn modal<'a>(place: &str, existing: Option<&str>, scheme: ColorScheme) -> Element<'a, Message> {
    let r = tokens::roles(scheme);
    let body = body_text(place, existing);
    let fields = column![
        Text::new("Start another session?", TypeRole::Headline, r),
        Text::new(body, TypeRole::Body, r).muted(),
    ];
    material::dialog::confirm(
        r,
        fields,
        ("Start", Message::Session(SessionMsg::AdditionalConfirmed)),
        ("Cancel", Message::Session(SessionMsg::AdditionalCancelled)),
        460.0,
    )
}

/// The dialog for a start another live session blocks (feature 776, FR-013, FR-016): `text` names
/// the live session; confirming runs both.
fn blocked_modal<'a>(text: &str, scheme: ColorScheme) -> Element<'a, Message> {
    let r = tokens::roles(scheme);
    let fields = column![
        Text::new("Start beside a running session?", TypeRole::Headline, r),
        Text::new(text.to_string(), TypeRole::Body, r).muted(),
    ];
    material::dialog::confirm(
        r,
        fields,
        ("Start", Message::Session(SessionMsg::AdditionalConfirmed)),
        ("Cancel", Message::Session(SessionMsg::AdditionalCancelled)),
        460.0,
    )
}

/// This dialog's body from the state that opened it; `None` when nothing opened it.
pub fn dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    _env_include_outcome: &'a EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    if let Some(id) = state.session.blocked_start {
        let text = state.session_start_block_text(id)?;
        return Some(blocked_modal(&text, scheme));
    }
    let (place, existing) = target_text(state)?;
    Some(modal(&place, existing.as_deref(), scheme))
}

/// The place and the main session's label the dialog for `additional_target` names; `None` when
/// no target is open. The label is `None` while this window's catalog does not know the occupant.
/// Split out of [`dialog`] so a test can read what the dialog says without rendering it.
pub fn target_text(state: &State) -> Option<(String, Option<String>)> {
    let location = state.session.additional_target.as_ref()?;
    let existing = micold_core::occupancy::main_session(state.active_sessions(), location)
        .and_then(|main| state.workspace.find_session(main))
        .map(|(_, session)| session.label.display().to_owned());
    let place = match location {
        SessionLocation::Default => "The project root".to_string(),
        SessionLocation::Worktree(dir) => {
            format!(
                "The worktree \u{201c}{}\u{201d}",
                state.worktree_display_name(dir)
            )
        }
    };
    Some((place, existing))
}
