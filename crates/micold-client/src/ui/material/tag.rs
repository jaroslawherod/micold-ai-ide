//! `Tag` — a reusable worktree tag/chip primitive (Constitution Principle VIII).
//!
//! A small pill carrying a short label on a solid fill, used for the color-coded type and
//! Jira/issue tags (and the missing/invalid status cue) beneath a worktree's name. Theme-aware
//! via the `(fill, on_fill)` color pair supplied by the caller (from `tokens::Roles`). Exposed
//! as a chainable builder terminating in `.into()` (Principle VIII builder-API rule), so it is
//! reused rather than each feature forking a bespoke chip.

use crate::ui::material::style;
use crate::ui::material::TypeRole;
use iced::widget::{container, text};
use iced::{Element, Padding};
use micold_core::tokens::{spacing, Rgb};
use std::marker::PhantomData;

/// A pill-shaped tag chip. Construct with a label + `accent` color (rendered as a dimmed tonal
/// chip — accent text on a faint accent tint); optionally set the type role (defaults to the
/// sidebar tag role).
pub struct Tag<'a, M> {
    label: String,
    accent: Rgb,
    on_accent: Option<Rgb>,
    role: TypeRole,
    _marker: PhantomData<&'a M>,
}

impl<'a, M: 'a> Tag<'a, M> {
    /// A tag showing `label` in the `accent` color.
    pub fn new(label: impl Into<String>, accent: Rgb) -> Self {
        Self {
            label: label.into(),
            accent,
            on_accent: None,
            role: TypeRole::SidebarTag,
            _marker: PhantomData,
        }
    }

    /// Draw the accent as an opaque fill with `on_accent` as the label, instead of the default
    /// tint.
    ///
    /// For a tag whose background is not the caller's to know — one that can land on a plain
    /// surface or on a filled container depending on state. The tint reads as an accent only over
    /// the former; see [`style::chip_solid`].
    pub fn solid(mut self, on_accent: Rgb) -> Self {
        self.on_accent = Some(on_accent);
        self
    }

    /// Override the label's type role.
    ///
    /// A role rather than a size, so a tag outside the sidebar cannot end up at a size that is not
    /// in the scale — which is what a bare `f32` here allowed.
    pub fn role(mut self, role: TypeRole) -> Self {
        self.role = role;
        self
    }
}

impl<'a, M: 'a> From<Tag<'a, M>> for Element<'a, M> {
    fn from(t: Tag<'a, M>) -> Self {
        // Taken apart rather than built with `Text`, because a tag carries an accent colour instead
        // of a `Roles` set and `Text` needs one. Still a role, so the weight and line height come
        // from the scale rather than from the renderer's defaults.
        let chip = container(
            text(t.label)
                .size(t.role.size())
                .font(t.role.font())
                .line_height(t.role.line_height()),
        )
        .padding(Padding {
            top: 0.0,
            bottom: 0.0,
            left: spacing::XS,
            right: spacing::XS,
        });
        // Styled in each arm rather than boxing one closure: the two style functions have different
        // types, and a `Box<dyn Fn>` here buys nothing the match does not.
        match t.on_accent {
            Some(on_accent) => chip.style(style::chip_solid(t.accent, on_accent)).into(),
            None => chip.style(style::chip(t.accent)).into(),
        }
    }
}
