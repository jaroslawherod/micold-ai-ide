//! `Modal` — the appearance of a modal dialog (Constitution Principle VIII).
//!
//! Every modal dialog (About, project selector, rename, add worktree, Settings) enters and exits
//! through this one component, so no dialog implements its own transition (FR-011). iced has
//! no general per-widget opacity, so the transition is composed from primitives the renderer *does*
//! expose (research R3): an animated dimming **scrim** (which reveals the window beneath as it
//! clears — FR-003) plus a dialog **fade + lift** ([`fade`](super::fade) toward the surface colour
//! and [`scale`](super::scale) about centre). Driven by a single `progress` (1.0 = fully shown,
//! 0.0 = hidden), so a consumer only builds its dialog body and supplies a progress value.
//!
//! What it no longer does is *place* itself. Centring, input blocking, dismissal and z-order all
//! moved to [`cdk::overlay`](crate::ui::cdk::overlay), the one primitive every floating surface now
//! shares (FR-008). This module decides the scrim's colour and the dialog's transition — appearance
//! — and hands the result over.

use crate::ui::cdk::overlay::{Anchor, Surface};
use iced::{Color, Element};
use micold_core::overlay::Layer;
use micold_core::tokens::Roles;

/// The scrim's alpha at full progress (matches the prior static backdrop dimming so the modal
/// looks unchanged at rest — only the transition is new).
const SCRIM_ALPHA: f32 = 0.6;

/// A modal dialog. Builder form (Principle VIII):
/// `Modal::new(dialog, roles).progress(p).on_dismiss(msg).into()`, yielding
/// `Option<Surface>` — `None` once the transition has finished, so a closed dialog leaves no trace.
pub struct Modal<'a, M> {
    dialog: Element<'a, M>,
    roles: Roles,
    progress: f32,
    on_dismiss: Option<M>,
}

impl<'a, M: Clone + 'a> Modal<'a, M> {
    /// A modal `dialog` themed by `roles`. Fully shown by default; set [`Self::progress`] to
    /// animate the enter/exit.
    pub fn new(dialog: impl Into<Element<'a, M>>, roles: Roles) -> Self {
        Self {
            dialog: dialog.into(),
            roles,
            progress: 1.0,
            on_dismiss: None,
        }
    }

    /// Transition progress (0 = hidden → yields no surface; 1 = fully shown).
    pub fn progress(mut self, progress: f32) -> Self {
        self.progress = progress;
        self
    }

    /// The message emitted when the user clicks the scrim. Omit it for a dialog that is only
    /// being animated out — a snapshot has nothing left to cancel.
    pub fn on_dismiss(mut self, message: M) -> Self {
        self.on_dismiss = Some(message);
        self
    }
}

impl<'a, M: Clone + 'a> From<Modal<'a, M>> for Option<Surface<'a, M>> {
    fn from(m: Modal<'a, M>) -> Self {
        let Modal {
            dialog,
            roles,
            progress,
            on_dismiss,
        } = m;
        let progress = progress.clamp(0.0, 1.0);
        // Hidden / finished: no scrim, no dialog, no input capture — just the app beneath.
        if progress <= 0.001 {
            return None;
        }

        // The dialog fades its contents toward its own surface colour, then lifts (scales about
        // its centre). Together with the scrim this reads as a Material dialog enter/exit.
        let dialog = super::scale(super::fade(dialog, progress, roles.surface), progress);

        let surface = Surface::new(Layer::Dialog, dialog, Anchor::Center).scrim(Color {
            a: progress * SCRIM_ALPHA,
            ..Color::BLACK
        });
        Some(match on_dismiss {
            Some(message) => surface.on_dismiss(message),
            None => surface,
        })
    }
}
