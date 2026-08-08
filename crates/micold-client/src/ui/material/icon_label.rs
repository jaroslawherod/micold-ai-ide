//! `IconLabel` — a glyph and a short label as one unit (Principle VIII).
//!
//! Two feature modules built this by hand, identically: the "git" badge beside a folder name in
//! the project selector and beside a project name in the shell's known list. Both spelled out
//! `row![Glyph::new(..), Text::new(..)].spacing(XS).align_y(Center)`, which is a component with
//! its parts left loose — and a loose part is one a call site can get wrong. `shell.rs` proved
//! that: five buttons wore the same shape behind a local `labeled()` helper and sized their glyph
//! at the *label's* role, 14dp, where §7.3 puts a button's leading icon at 18.
//!
//! **The glyph takes the label's role here, and that is the point of the type.** A labelled icon
//! is one piece of text with a picture in front of it, so the picture matches the words. A
//! button's leading icon is not that: §7.3 sizes it at 18 whatever the label's role, and
//! [`Button::leading`](crate::ui::material::Button::leading) owns that figure. Conflating the two
//! is the defect T113 and the shell fix each corrected once. Keeping them as two named things —
//! this type, and that slot — is what stops it being corrected a third time.
//!
//! `composite_call_sites.rs` holds the line: a feature module may no longer put a `Glyph` and a
//! `Text` side by side in a row of its own.

use crate::icons::Icon;
use crate::ui::material::{Glyph, Text, TypeRole};
use iced::widget::row;
use iced::{Alignment, Element};
use micold_core::tokens::{spacing, Rgb, Roles};
use std::borrow::Cow;

/// A glyph followed by a label, both at the same type role.
pub struct IconLabel<'a, M> {
    icon: Icon,
    label: Cow<'a, str>,
    role: TypeRole,
    roles: Roles,
    tint: Option<Rgb>,
    muted: bool,
    _marker: std::marker::PhantomData<M>,
}

impl<'a, M: 'a> IconLabel<'a, M> {
    /// A labelled icon at `role`. The glyph and the label both take it.
    pub fn new(icon: Icon, label: impl Into<Cow<'a, str>>, role: TypeRole, roles: Roles) -> Self {
        Self {
            icon,
            label: label.into(),
            role,
            roles,
            tint: None,
            muted: false,
            _marker: std::marker::PhantomData,
        }
    }

    /// Tint the glyph. Which semantic tint an icon carries is the caller's decision, resolved by
    /// [`icon_role`](crate::icons::icon_role) — the same reason [`Glyph`] asks for it.
    pub fn tint(mut self, tint: Rgb) -> Self {
        self.tint = Some(tint);
        self
    }

    /// Render the label in the muted tone, for a badge that annotates rather than states.
    pub fn muted(mut self) -> Self {
        self.muted = true;
        self
    }
}

impl<'a, M: 'a> From<IconLabel<'a, M>> for Element<'a, M> {
    fn from(l: IconLabel<'a, M>) -> Self {
        let mut glyph = Glyph::new(l.icon, l.role, l.roles);
        if let Some(tint) = l.tint {
            glyph = glyph.tint(tint);
        }
        let mut text = Text::new(l.label, l.role, l.roles);
        if l.muted {
            text = text.muted();
        }
        row![glyph, text]
            .spacing(spacing::XS)
            .align_y(Alignment::Center)
            .into()
    }
}
