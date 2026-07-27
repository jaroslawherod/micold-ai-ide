//! `StageProgress` — a Material-style linear progress indicator paired with a current-stage
//! label (Constitution Principle VIII, feature 013).
//!
//! Composed from iced's own `progress_bar` widget (themed via the active [`Roles`]) rather than
//! forking a bespoke bar-drawing widget. The fill is a fixed, non-animated value rather than a
//! true completion percentage: whether the submodule stage will run at all is only known
//! *after* the branch/worktree already exists (research.md R2), so the bar makes no claim about
//! total duration or step count — it exists to stay visibly present for the operation's
//! duration, while the paired label names the concrete current step (`CreateStage::label`).

use crate::ui::material::style;
use iced::widget::{column, progress_bar, text};
use iced::{Element, Length};
use micold_core::tokens::{shape, spacing, type_scale, Roles};

/// Bar thickness, in logical pixels.
const HEIGHT: f32 = 4.0;
/// A fixed, non-zero, non-complete fill — reads as "in progress," not a real fraction.
const VALUE: f32 = 0.4;

/// A stage-progress indicator: a thin bar plus the current stage's plain-language label.
/// Builder form (Principle VIII): `StageProgress::new(label, roles).into()`.
pub struct StageProgress {
    label: String,
    roles: Roles,
}

impl StageProgress {
    /// A progress indicator showing `label` as the current stage, themed by `roles`.
    pub fn new(label: impl Into<String>, roles: Roles) -> Self {
        Self {
            label: label.into(),
            roles,
        }
    }
}

impl<'a, M: 'a> From<StageProgress> for Element<'a, M> {
    fn from(p: StageProgress) -> Self {
        let r = p.roles;
        let bar = progress_bar(0.0..=1.0, VALUE)
            .girth(Length::Fixed(HEIGHT))
            .style(move |_theme: &iced::Theme| progress_bar::Style {
                background: iced::Background::Color(style::color(r.surface_variant)),
                bar: iced::Background::Color(style::color(r.primary)),
                border: iced::border::rounded(shape::FULL),
            });

        column![
            bar,
            text(p.label).size(type_scale::LABEL).style(style::muted(r)),
        ]
        .spacing(spacing::XS)
        .into()
    }
}
