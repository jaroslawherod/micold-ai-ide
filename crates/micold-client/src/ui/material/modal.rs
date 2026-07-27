//! `Modal` — the shared modal-overlay transition (Constitution Principle VIII).
//!
//! Every modal dialog (About, project selector, rename, add worktree, Settings) enters and
//! exits through this one component, so no overlay implements its own scrim/stack/`opaque`
//! wiring (FR-011). iced has no general per-widget opacity, so the transition is composed
//! from primitives the renderer *does* expose (research R3): an animated dimming **scrim**
//! (`fill_quad` alpha, which reveals `base` as it clears — FR-003) plus a dialog **fade + lift**
//! ([`fade`](super::fade) toward the surface color and [`scale`](super::scale) about center).
//! Driven by a single `progress` (1.0 = fully shown, 0.0 = hidden), so a consumer only builds
//! its dialog body and supplies a progress value.

use micold_core::tokens::Roles;
use crate::ui::style;
use iced::widget::{center, container, opaque, stack};
use iced::{Background, Color, Element};

/// The scrim's alpha at full progress (matches the prior static backdrop dimming so the modal
/// looks unchanged at rest — only the transition is new).
const SCRIM_ALPHA: f32 = 0.6;

/// Stacks a modal `dialog` over `base` with a fade + lift transition driven by `progress`
/// (1.0 = fully shown, 0.0 = fully hidden). Renders `base` as-is at `progress` <= 0.001, so a
/// closed/finished overlay leaves no trace. Composes `base` + an animated scrim (alpha =
/// `progress * SCRIM_ALPHA`, revealing `base` as it clears) + the centered dialog wrapped in
/// [`fade`](super::fade) + [`scale`](super::scale); input is captured via `opaque` while shown.
///
/// Builder form (Principle VIII): `Modal::new(base, dialog, roles).progress(p).into()`.
pub struct Modal<'a, M> {
    base: Element<'a, M>,
    dialog: Element<'a, M>,
    roles: Roles,
    progress: f32,
}

impl<'a, M: Clone + 'a> Modal<'a, M> {
    /// A modal `dialog` over `base`, themed by `roles`. Fully shown by default; set
    /// [`Self::progress`] to animate the enter/exit.
    pub fn new(
        base: impl Into<Element<'a, M>>,
        dialog: impl Into<Element<'a, M>>,
        roles: Roles,
    ) -> Self {
        Self {
            base: base.into(),
            dialog: dialog.into(),
            roles,
            progress: 1.0,
        }
    }

    /// Transition progress (0 = hidden → returns `base` as-is; 1 = fully shown).
    pub fn progress(mut self, progress: f32) -> Self {
        self.progress = progress;
        self
    }
}

impl<'a, M: Clone + 'a> From<Modal<'a, M>> for Element<'a, M> {
    fn from(m: Modal<'a, M>) -> Self {
        let Modal {
            base,
            dialog,
            roles,
            progress,
        } = m;
        let progress = progress.clamp(0.0, 1.0);
        // Hidden / finished: no scrim, no dialog, no input capture — just the app beneath.
        if progress <= 0.001 {
            return base;
        }

        // The dialog fades its contents toward its own surface color, then lifts (scales about
        // its center). Together with the scrim this reads as a Material dialog enter/exit.
        let dialog = super::scale(
            super::fade(dialog, progress, style::color(roles.surface)),
            progress,
        );

        // Full-window scrim: dims in over `base` and clears out to progressively reveal it
        // (FR-003). The centered dialog sits on top of it.
        let alpha = progress * SCRIM_ALPHA;
        let scrim = center(dialog).style(move |_theme: &iced::Theme| container::Style {
            background: Some(Background::Color(Color {
                a: alpha,
                ..Color::BLACK
            })),
            ..container::Style::default()
        });

        // `opaque` captures all input, so `base` stays non-interactive while the dialog shows.
        stack![base, opaque(scrim)].into()
    }
}
