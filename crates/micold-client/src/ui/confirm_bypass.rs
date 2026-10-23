//! The Bypass confirmation (#942 FR-014, FR-017): before a structured session is created in
//! Bypass, or switched to it, the dialog names what Bypass does, and nothing changes until the
//! user confirms. Built like `ui/confirm_delete.rs`; Escape and a scrim click cancel.

use crate::app::{Message, State};
use crate::features::permission_prompts::Msg as PromptsMsg;
use crate::features::session::Msg as SessionMsg;
use crate::ui::material::{self, Text, TypeRole};
use iced::widget::column;
use iced::Element;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::theme::ColorScheme;
use micold_core::tokens;

/// The dialog's headline.
pub const TITLE: &str = "Run in Bypass?";

/// What the dialog says Bypass does (FR-014: the confirmation names its effect).
pub const EFFECT: &str = "Claude will run every tool without asking: it can run commands, change \
     and delete files and reach the network without a prompt. Choose Bypass only for work you \
     would let it do unattended.";

/// The dialog body, its buttons sending `confirm` and `cancel`.
pub fn modal<'a>(confirm: Message, cancel: Message, scheme: ColorScheme) -> Element<'a, Message> {
    let r = tokens::roles(scheme);
    let fields = column![
        Text::new(TITLE, TypeRole::Headline, r),
        Text::new(EFFECT, TypeRole::Body, r).muted(),
    ];
    material::dialog::confirm(
        r,
        fields,
        ("Use Bypass", confirm),
        ("Cancel", cancel),
        460.0,
    )
}

/// The confirmation of a structured start in Bypass (FR-014), while one waits for it.
pub fn start_dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    _env_include_outcome: &'a EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    state.session.bypass_start.as_ref().map(|_| {
        modal(
            Message::Session(SessionMsg::BypassStartConfirmed),
            Message::Session(SessionMsg::BypassStartCancelled),
            scheme,
        )
    })
}

/// The confirmation of a session's mode change to Bypass (FR-017), while one waits for it.
pub fn mode_dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    _env_include_outcome: &'a EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    state.permission_prompts.mode_confirmation().map(|_| {
        modal(
            Message::PermissionPrompts(PromptsMsg::ModeConfirmed),
            Message::PermissionPrompts(PromptsMsg::ModeCancelled),
            scheme,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::session::BypassStart;
    use micold_core::session::{AiCli, PermissionMode, SessionId, SessionLocation};

    /// T064 (FR-014): the dialog names Bypass's effect in the words the spec gives it.
    #[test]
    fn the_dialog_names_the_effect() {
        assert!(EFFECT.starts_with("Claude will run every tool without asking"));
    }

    /// T064 (FR-014, FR-017): each confirmation is drawn exactly while its state waits for it.
    #[test]
    fn each_confirmation_draws_while_it_waits() {
        let outcome = EnvIncludeOutcome::Disabled;
        let mut state = State::default();
        assert!(start_dialog(&state, ColorScheme::Light, &outcome).is_none());
        assert!(mode_dialog(&state, ColorScheme::Light, &outcome).is_none());

        state.session.bypass_start = Some(BypassStart {
            location: SessionLocation::Default,
            provider: AiCli::ClaudeCode,
            menu: None,
        });
        assert!(start_dialog(&state, ColorScheme::Light, &outcome).is_some());
        assert!(mode_dialog(&state, ColorScheme::Light, &outcome).is_none());

        state.session.bypass_start = None;
        let session = SessionId::new();
        assert_eq!(
            state
                .permission_prompts
                .request_mode(session, PermissionMode::Bypass),
            None
        );
        assert!(start_dialog(&state, ColorScheme::Light, &outcome).is_none());
        assert!(mode_dialog(&state, ColorScheme::Light, &outcome).is_some());
    }
}
