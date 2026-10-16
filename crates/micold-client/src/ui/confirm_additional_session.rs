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

/// The dialog body: `place` names the worktree (or the project root) and `existing` its session.
pub fn modal<'a>(place: &str, existing: &str, scheme: ColorScheme) -> Element<'a, Message> {
    let r = tokens::roles(scheme);
    let body = format!(
        "{place} already has the session \u{201c}{existing}\u{201d}. The new session is a helper: \
         it runs beside it and the existing one stays the main session."
    );
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

/// This dialog's body from the state that opened it; `None` when the location holds no session
/// any more, so nothing is drawn rather than a dialog naming nothing.
pub fn dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    _env_include_outcome: &'a EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    let location = state.session.additional_target.as_ref()?;
    let main = micold_core::occupancy::main_session(state.active_sessions(), location)?;
    let (_, session) = state.workspace.find_session(main)?;
    let place = match location {
        SessionLocation::Default => "The project root".to_string(),
        SessionLocation::Worktree(dir) => {
            format!(
                "The worktree \u{201c}{}\u{201d}",
                state.worktree_display_name(dir)
            )
        }
    };
    Some(modal(&place, session.label.display(), scheme))
}
