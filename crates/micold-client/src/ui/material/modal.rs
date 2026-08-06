//! `Modal` — the appearance of a modal dialog (Constitution Principle VIII).
//!
//! Every modal dialog (About, project selector, rename, add worktree, Settings) enters and exits
//! through this one component, so no dialog implements its own transition (FR-011). iced has
//! no general per-widget opacity, so the transition is composed from primitives the renderer *does*
//! expose (research R3): an animated dimming **scrim** (which reveals the window beneath as it
//! clears — FR-003) plus a dialog **fade + lift** ([`fade`](super::fade) toward the surface colour
//! and [`scale`](super::scale) about centre).
//!
//! A consumer builds its dialog body and says whether it is open. The three tracks that carry it in
//! and out are owned by the widgets that draw them (FR-011): they share a target, a duration and a
//! frame clock, so they move in lockstep without anything coordinating them.
//!
//! What it does not do is *place* itself. Centring, input blocking, dismissal and z-order all
//! moved to [`cdk::overlay`](crate::ui::cdk::overlay), the one primitive every floating surface now
//! shares (FR-008). This module decides the scrim's colour and the dialog's transition — appearance
//! — and hands the result over.

use std::time::Duration;

use crate::ui::cdk::overlay::{Anchor, Surface};
use iced::{Color, Element};
use micold_core::overlay::Layer;
use micold_core::tokens::motion::duration;
use micold_core::tokens::Roles;

/// The scrim's alpha at full progress — Material's 32% (contract §4).
///
/// Down from feature 003's 0.6, which was chosen to match an older static backdrop. The dialog now
/// separates from what is behind it by elevation 3's tone and shadow rather than by drowning the
/// background, so a lighter scrim keeps the context beneath legible instead of hiding it.
const SCRIM_ALPHA: f32 = 0.32;

/// Dialog entrance — Material Design 3 "medium" duration; clearly perceptible (the ~90ms this
/// replaced was not).
const ENTER: Duration = Duration::from_millis(duration::MEDIUM_2);
/// Dialog exit — ~0.8× the entrance (Material convention: exits are quicker).
const EXIT: Duration = Duration::from_millis(duration::SHORT_4);

/// A modal dialog. Builder form (Principle VIII):
/// `Modal::new(dialog, roles).shown(open).on_dismiss(msg).on_hidden(msg).into()`.
pub struct Modal<'a, M> {
    dialog: Element<'a, M>,
    roles: Roles,
    shown: bool,
    key: u64,
    on_dismiss: Option<M>,
    on_hidden: Option<M>,
}

impl<'a, M: Clone + 'a> Modal<'a, M> {
    /// A modal `dialog` themed by `roles`, open — a dialog is built because it is being shown.
    /// It animates in from hidden; [`Self::shown`] `false` animates it back out again.
    pub fn new(dialog: impl Into<Element<'a, M>>, roles: Roles) -> Self {
        Self {
            dialog: dialog.into(),
            roles,
            shown: true,
            key: 0,
            on_dismiss: None,
            on_hidden: None,
        }
    }

    /// Whether the dialog is open. Going from `true` to `false` plays the exit.
    pub fn shown(mut self, shown: bool) -> Self {
        self.shown = shown;
        self
    }

    /// Which dialog this is, so that opening a different one over the top of the current one
    /// enters from the beginning rather than inheriting a transition that had already finished.
    pub fn restart_on(mut self, key: u64) -> Self {
        self.key = key;
        self
    }

    /// The message emitted when the user clicks the scrim. Omit it for a dialog that is only
    /// being animated out — a snapshot has nothing left to cancel.
    pub fn on_dismiss(mut self, message: M) -> Self {
        self.on_dismiss = Some(message);
        self
    }

    /// The message emitted once the exit has finished, so whoever is holding the dialog's
    /// render data can let go of it. Without it a closed dialog would keep its surface forever.
    pub fn on_hidden(mut self, message: M) -> Self {
        self.on_hidden = Some(message);
        self
    }
}

impl<'a, M: Clone + 'a> From<Modal<'a, M>> for Surface<'a, M> {
    fn from(m: Modal<'a, M>) -> Self {
        let Modal {
            dialog,
            roles,
            shown,
            key,
            on_dismiss,
            on_hidden,
        } = m;

        // The dialog fades its contents toward its own surface colour, then lifts (scales about
        // its centre). Together with the scrim this reads as a Material dialog enter/exit.
        let dialog = super::scale(
            super::fade(dialog, shown, ENTER, super::SurfaceKind::Dialog.tone(roles))
                // Both the tone and the corners come from the dialog's own surface kind. The
                // tone matters as much as the shape: a dialog is elevation 3, so veiling it with
                // `roles.surface` painted a rectangle several tones too dark over it for the whole
                // of every entrance and exit.
                .rounded(super::SurfaceKind::Dialog.shape())
                .exiting_over(EXIT)
                .animate_in()
                .restart_on(key),
            shown,
            ENTER,
        )
        .exiting_over(EXIT)
        .animate_in()
        .restart_on(key);

        // The scrim is the track that reports the exit as finished: it outlives the dialog body
        // visually, and it is the layer whose disappearance means the window is usable again.
        let mut scrim = super::scrim(
            Color {
                a: SCRIM_ALPHA,
                ..Color::BLACK
            },
            shown,
            ENTER,
        )
        .exiting_over(EXIT)
        .animate_in()
        .restart_on(key);
        if let Some(message) = on_hidden {
            scrim = scrim.on_hidden(message);
        }

        let surface = Surface::new(Layer::Dialog, dialog, Anchor::Center).scrim(scrim);
        match on_dismiss {
            Some(message) => surface.on_dismiss(message),
            None => surface,
        }
    }
}
