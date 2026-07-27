//! `Tag` — a reusable worktree tag/chip primitive (Constitution Principle VIII).
//!
//! A small pill carrying a short label on a solid fill, used for the color-coded type and
//! Jira/issue tags (and the missing/invalid status cue) beneath a worktree's name. Theme-aware
//! via the `(fill, on_fill)` color pair supplied by the caller (from `tokens::Roles`). Exposed
//! as a chainable builder terminating in `.into()` (Principle VIII builder-API rule), so it is
//! reused rather than each feature forking a bespoke chip.

use micold_core::tokens::{sidebar, spacing, Rgb};
use crate::ui::style;
use iced::widget::{container, text};
use iced::{Element, Padding};
use std::marker::PhantomData;

/// A pill-shaped tag chip. Construct with a label + `accent` color (rendered as a dimmed tonal
/// chip — accent text on a faint accent tint); optionally set the text size (defaults to the
/// sidebar tag size).
pub struct Tag<'a, M> {
    label: String,
    accent: Rgb,
    size: f32,
    _marker: PhantomData<&'a M>,
}

impl<'a, M: 'a> Tag<'a, M> {
    /// A tag showing `label` in the `accent` color.
    pub fn new(label: impl Into<String>, accent: Rgb) -> Self {
        Self {
            label: label.into(),
            accent,
            size: sidebar::TAG,
            _marker: PhantomData,
        }
    }

    /// Override the label text size.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
}

impl<'a, M: 'a> From<Tag<'a, M>> for Element<'a, M> {
    fn from(t: Tag<'a, M>) -> Self {
        container(text(t.label).size(t.size))
            .padding(Padding {
                top: 0.0,
                bottom: 0.0,
                left: spacing::XS,
                right: spacing::XS,
            })
            .style(style::chip(t.accent))
            .into()
    }
}
