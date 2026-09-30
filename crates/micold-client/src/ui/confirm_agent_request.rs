//! The dialog that puts an agent's destructive request to the user (feature 034, FR-014): who
//! asks, to do what, to which target — Allow performs it, Deny refuses it.

use crate::app::{Message, State};
use crate::features::agent_confirm::{self, Msg, Prompt};
use crate::ui::material::{self, Button, SurfaceKind, Text, TypeRole};
use iced::widget::{column, row};
use iced::Element;
use iced::Length;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self};

/// The prompt as a modal surface, as the dialog body; `ui::view` wraps it in the shared
/// [`Modal`](crate::ui::material::Modal) transition.
pub fn modal<'a>(prompt: &Prompt, scheme: ColorScheme) -> Element<'a, Message> {
    let r = tokens::roles(scheme);
    let id = prompt.id;

    let body = "An AI session asked for this. Nothing changes unless you allow it; if nobody \
                answers within 60 seconds, the request fails.";

    let fields = material::dialog::fields(column![
        Text::new(agent_confirm::headline(prompt), TypeRole::Headline, r),
        Text::new(body, TypeRole::Body, r).muted(),
    ]);

    let actions = material::dialog::actions(row![
        Button::filled("Allow", r)
            .on_press(Message::AgentConfirm(Msg::Answered { id, allow: true })),
        Button::outlined("Deny", r)
            .on_press(Message::AgentConfirm(Msg::Answered { id, allow: false })),
    ]);

    let dialog = material::Surface::new(
        material::dialog::body(fields, actions),
        SurfaceKind::Dialog,
        r,
    )
    .width(Length::Fixed(460.0));

    dialog.into()
}

/// This dialog's body, built from the oldest pending prompt — `None` when there is none.
pub fn dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    _env_include_outcome: &'a EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    state
        .agent_confirm
        .shown()
        .map(|prompt| modal(prompt, scheme))
}
