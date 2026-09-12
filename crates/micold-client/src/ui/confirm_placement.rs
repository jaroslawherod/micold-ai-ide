//! The confirm-move dialog for a placement change (BUG-003, FR-032/FR-033).
//!
//! Moving where sessions run is the only setting on this form that ends processes. Everything
//! else takes effect on the next session; this one stops the service the user's current sessions
//! are inside, which is why FR-032 asks before applying it rather than after — and why declining
//! leaves the whole save unapplied (FR-032b) instead of writing the file and asking about the
//! restart separately.

use crate::app::{Message, State};
use crate::features::settings::{placement_move_consequence, Msg as SettingsMsg};
use crate::ui::material::{self, Button, SurfaceKind, Text, TypeRole};
use iced::widget::{column, row};
use iced::Element;
use iced::Length;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::sandbox::placement::PlacementKind;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self};

/// The dialog body for a move from `from` to `to`.
///
/// Both ends are named, in the words the select above uses for them. "The placement will change"
/// is not a question anyone can answer; "sessions will move from this computer into a container"
/// is.
pub fn modal<'a>(
    from: PlacementKind,
    to: PlacementKind,
    live_sessions: usize,
    scheme: ColorScheme,
) -> Element<'a, Message> {
    let r = tokens::roles(scheme);

    // Built in the feature, not here: the count and its plural are the one part of this dialog
    // that can be *wrong*, and a `String` is assertable where an `Element` is not (FR-033).
    let consequence = placement_move_consequence(from, to, live_sessions);

    let fields = material::dialog::fields(column![
        Text::new(
            format!("Move sessions {}?", to.label().to_lowercase()),
            TypeRole::Headline,
            r
        ),
        Text::new(consequence, TypeRole::Body, r).muted(),
    ]);

    let actions = material::dialog::actions(row![
        Button::filled("Move sessions", r)
            .on_press(Message::Settings(SettingsMsg::PlacementChangeConfirmed)),
        Button::outlined("Cancel", r)
            .on_press(Message::Settings(SettingsMsg::PlacementChangeCancelled)),
    ]);

    material::Surface::new(
        material::dialog::body(fields, actions),
        SurfaceKind::Dialog,
        r,
    )
    .width(Length::Fixed(460.0))
    .into()
}

/// This dialog's body, built from the state that opened it (feature 021, T035 — FR-008, FR-009).
pub fn dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    _env_include_outcome: &'a EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    state.settings.pending_placement.map(|change| {
        // Every session the application knows about, across every project: the service that hosts
        // them is one process, and the move stops it — so "the ones in this project" would be an
        // undercount of what the user is agreeing to lose.
        let live = state.workspace.sessions.values().map(Vec::len).sum();
        modal(change.from, change.to, live, scheme)
    })
}
