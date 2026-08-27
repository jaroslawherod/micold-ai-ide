//! The confirm-remove dialog for a session, rendered as a Material modal overlay (bugfix
//! BUG-003, FR-015c). Removing is permanent — unlike Close, there is no marker-based recovery
//! path back into the sidebar (the removed session's record is dropped outright).

use crate::app::{Message, State};
use crate::features::session::Msg as SessionMsg;
use crate::ui::material::{self, Button, SurfaceKind, Text, TypeRole};
use iced::widget::{column, row};
use iced::Element;
use iced::Length;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self};

/// The confirm-remove dialog for a session (shown by its sidebar `label`) as a modal surface, as the dialog body; `ui::view` wraps it
/// in the shared [`Modal`](crate::ui::material::Modal) transition.
pub fn modal<'a>(label: &str, scheme: ColorScheme) -> Element<'a, Message> {
    let r = tokens::roles(scheme);

    let warning = "This permanently deletes the session. Unlike Close, it cannot be recovered — \
                   the underlying `claude` conversation itself is untouched, but the app will \
                   never show or resume it again.";

    let fields = material::dialog::fields(column![
        Text::new(format!("Remove “{label}”?"), TypeRole::Headline, r),
        Text::new(warning, TypeRole::Body, r).muted(),
    ]);

    let actions = material::dialog::actions(row![
        Button::filled("Remove", r).on_press(Message::Session(SessionMsg::RemoveConfirmed)),
        Button::outlined("Cancel", r).on_press(Message::Session(SessionMsg::RemoveCancelled)),
    ]);

    let dialog = material::Surface::new(
        material::dialog::body(fields, actions),
        SurfaceKind::Dialog,
        r,
    )
    .width(Length::Fixed(460.0));

    dialog.into()
}

/// This dialog's body, built from the state that opened it — `None` when the surface is open but
/// the live state it renders is absent, so nothing is drawn rather than an empty dialog.
///
/// The uniform shape every registered dialog has, and the reason `ui::view` no longer needs a
/// match: the registration line in [`crate::overlay::registry`] names this beside the surface it
/// draws, so a dialog says where its own state lives instead of a central arm saying it for them
/// all (feature 021, T035 — FR-008, FR-009).
pub fn dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    _env_include_outcome: &'a EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    state
        .session
        .remove_target
        .and_then(|id| state.workspace.find_session(id))
        .map(|(_, session)| modal(session.label.display(), scheme))
}
