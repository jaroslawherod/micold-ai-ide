//! The confirmation a sandboxed file link asks for before this machine opens it (feature 031,
//! FR-018a). The path was translated out of the container, so the file is one the sandboxed
//! session could have written; opening it hands it to a host application, which is outside
//! everything the sandbox contains.

use crate::app::{Message, State};
use crate::features::session::Msg as SessionMsg;
use crate::ui::material::{self, Text, TypeRole};
use iced::widget::column;
use iced::Element;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::theme::ColorScheme;
use micold_core::tokens;

/// The confirmation for one translated host path, as the dialog body; `ui::view` wraps it in the
/// shared [`Modal`](crate::ui::material::Modal) transition.
pub fn modal<'a>(host_path: &str, scheme: ColorScheme) -> Element<'a, Message> {
    let r = tokens::roles(scheme);

    let warning = format!(
        "The sandboxed session linked to {host_path}. Files the sandbox wrote can contain scripts \
         or macros."
    );

    let fields = column![
        Text::new("Open a file from the sandbox?", TypeRole::Headline, r),
        Text::new(warning, TypeRole::Body, r).muted(),
    ];

    material::dialog::confirm(
        r,
        fields,
        ("Open", Message::Session(SessionMsg::LinkOpenConfirmed)),
        ("Cancel", Message::Session(SessionMsg::LinkOpenDeclined)),
        460.0,
    )
}

/// This dialog's body, built from the state that opened it — the uniform shape every registered
/// dialog has (feature 021, T035 — FR-008, FR-009).
pub fn dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    _env_include_outcome: &'a EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    state
        .session
        .pending_link_open
        .as_ref()
        .map(|pending| modal(&pending.link.display, scheme))
}
