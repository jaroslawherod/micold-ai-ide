//! `Keycap` — a key or chord drawn as an outlined key-cap, as `FieldButton`'s hint draws its chord
//! (#1274: the shortcut sheet's key column).
//!
//! Builder form: `Keycap::new("Ctrl+Shift+Enter", roles).into()`.

use iced::widget::container;
use iced::{Border, Element, Padding};

use micold_core::tokens::{shape, spacing, Roles};

use super::{field_button, style, Text, TypeRole};

/// The cap's inset around its words: the field button's hint's.
const INSET: Padding = Padding {
    top: 1.0,
    bottom: 1.0,
    left: spacing::XS,
    right: spacing::XS,
};

/// A key's label on an outlined cap.
pub struct Keycap {
    label: String,
    roles: Roles,
}

impl Keycap {
    /// A cap showing `label` (`j`, `Ctrl+Shift+Enter`), themed by `roles`.
    pub fn new(label: impl Into<String>, roles: Roles) -> Self {
        Self {
            label: label.into(),
            roles,
        }
    }
}

impl<'a, M: 'a> From<Keycap> for Element<'a, M> {
    fn from(cap: Keycap) -> Self {
        let r = cap.roles;
        container(Text::new(cap.label, TypeRole::Label, r).tint(field_button::content(r)))
            .padding(INSET)
            .style(move |_| container::Style {
                border: Border {
                    color: style::color(field_button::outline(r)),
                    width: 1.0,
                    radius: shape::EXTRA_SMALL.into(),
                },
                ..container::Style::default()
            })
            .into()
    }
}
