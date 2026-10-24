//! The confirmation of a switch between a Claude session's structured view and its terminal while
//! work is in progress (#943 FR-021, US4 scenario 5): the service names the work the switch will
//! end, and nothing changes until the user confirms. Built like `ui/confirm_delete.rs`; Escape and
//! a scrim click cancel.

use crate::app::{Message, State};
use crate::features::composer::Msg as ComposerMsg;
use crate::features::session::{switch_label, switch_question};
use crate::ui::material::{self, Text, TypeRole};
use iced::widget::column;
use iced::Element;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::theme::ColorScheme;
use micold_core::tokens;

/// The confirmation while the window's switch offer waits for it.
pub fn dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    _env_include_outcome: &'a EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    let offer = state.composers.switch_offer.as_ref()?;
    let message = offer.confirm.as_deref()?;
    let r = tokens::roles(scheme);
    let fields = column![
        Text::new(switch_question(offer.to), TypeRole::Headline, r),
        Text::new(message.to_owned(), TypeRole::Body, r).muted(),
    ];
    Some(material::dialog::confirm(
        r,
        fields,
        (
            switch_label(offer.to),
            Message::Composer(ComposerMsg::SwitchConfirmed(offer.session)),
        ),
        (
            "Cancel",
            Message::Composer(ComposerMsg::SwitchDismissed(offer.session)),
        ),
        460.0,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::session::OPEN_IN_TERMINAL;
    use micold_core::session::{SessionId, SessionKind};

    fn offered(confirm: Option<&str>) -> (State, SessionId) {
        let mut state = State::default();
        let session = SessionId::new();
        state.offer_switch(
            session,
            SessionKind::Terminal,
            Some("mcp".to_owned()),
            "why".to_owned(),
        );
        if let Some(message) = confirm {
            let request = state.switch_confirmed().expect("offered");
            state.switch_needs_confirmation(request, message.to_owned());
        }
        (state, session)
    }

    /// T063 (FR-021): the confirmation is drawn only once the service asked for it, and names the
    /// work that will end in the service's words.
    #[test]
    fn the_confirmation_draws_while_the_service_asks_and_names_the_work() {
        use crate::ui::material::test_support::Mounted;
        let outcome = EnvIncludeOutcome::Disabled;
        assert!(dialog(&State::default(), ColorScheme::Light, &outcome).is_none());
        let (offer, _) = offered(None);
        assert!(
            dialog(&offer, ColorScheme::Light, &outcome).is_none(),
            "the offer itself is a card in the view, not this dialog"
        );
        let work = "A turn is running: switching ends it.";
        let (asking, _) = offered(Some(work));
        let drawn = dialog(&asking, ColorScheme::Dark, &outcome).expect("asks");
        let painted = Mounted::new(drawn.map(|_| String::new())).painted_text();
        for text in ["Open in terminal?", work, OPEN_IN_TERMINAL, "Cancel"] {
            assert!(
                painted.iter().any(|p| p.content == text),
                "draws `{text}`: {painted:?}"
            );
        }
    }
}
