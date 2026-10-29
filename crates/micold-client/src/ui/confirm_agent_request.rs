//! The dialog that puts an agent's destructive request to the user (feature 034, FR-014): who
//! asks, to do what, to which target — Allow performs it, Deny refuses it.

use crate::app::{Message, State};
use crate::features::agent_confirm::{self, Msg, Prompt};
use crate::ui::material::{self, Text, TypeRole};
use iced::widget::{column, container, Column};
use iced::{Element, Length};
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self};

/// The tallest a loss list grows before it scrolls.
const LIST_MAX_HEIGHT: f32 = 140.0;

/// The prompt as a modal surface, as the dialog body; `ui::view` wraps it in the shared
/// [`Modal`](crate::ui::material::Modal) transition.
pub fn modal<'a>(prompt: &Prompt, scheme: ColorScheme) -> Element<'a, Message> {
    let r = tokens::roles(scheme);
    let id = prompt.id;

    let body = "An AI session asked for this. Nothing changes unless you allow it; if nobody \
                answers within 60 seconds, the request fails.";

    let mut fields = column![
        Text::new(agent_confirm::headline(prompt), TypeRole::Headline, r),
        Text::new(body, TypeRole::Body, r).muted(),
    ];
    if let Some(note) = agent_confirm::self_targeted_note(prompt) {
        fields = fields.push(Text::new(note, TypeRole::Body, r));
    }

    for (heading, rows) in agent_confirm::loss_sections(prompt) {
        fields = fields.push(Text::new(heading, TypeRole::Body, r));
        if !rows.is_empty() {
            let list = Column::with_children(
                rows.into_iter()
                    .map(|row| Text::new(row, TypeRole::Caption, r).muted().into()),
            );
            fields = fields.push(
                container(material::Scrollable::new(list, r).height(Length::Shrink))
                    .max_height(LIST_MAX_HEIGHT),
            );
        }
    }

    material::dialog::confirm(
        r,
        fields,
        (
            "Allow",
            Message::AgentConfirm(Msg::Answered { id, allow: true }),
        ),
        (
            "Deny",
            Message::AgentConfirm(Msg::Answered { id, allow: false }),
        ),
        460.0,
    )
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
