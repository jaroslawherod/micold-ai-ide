//! `SectionList` — the navigation rail a full-surface view is divided by (Principle VIII, feature
//! 027 FR-026/FR-026a).
//!
//! A column of named destinations, one of them current. Pressing one emits its message; the
//! caller changes what it shows and passes a new selection back. The component holds no state of
//! its own, which is what lets the same rail serve a settings surface today and anything else
//! later — the selection lives where the thing being selected lives.
//!
//! # Why this is in the library
//!
//! FR-026a asks for it explicitly, and the reason is worth stating: a rail built privately inside
//! the settings view is invisible to every gate this crate holds components to. It would not be
//! checked for the builder shape, would not appear in the showcase, would not be held to the type
//! scale or the state layers, and the second view that wanted one would grow its own. The
//! `NavigationDrawer` beside it is the same argument already won once.
//!
//! # What it is not
//!
//! Not a `NavigationDrawer`. That one animates a panel out of the way and leaves a rail behind —
//! it answers "is the panel on screen?". This answers "which destination is current?", and the two
//! compose: the settings surface puts a `SectionList` *inside* a drawer, so the rail can collapse
//! at a narrow window without either component knowing about the other's question.
//!
//! Not a `Select` either, though both pick one of several. A select hides the alternatives behind
//! a trigger and is a *field* — it edits a value. This shows every destination at once and is
//! *navigation* — it changes what the surface displays and edits nothing.

use std::marker::PhantomData;

use iced::widget::{column, container, row, Space};
use iced::{Alignment, Element, Length};

use super::{Button, ButtonVariant, Tag, Text, TypeRole};
use micold_core::tokens::{spacing, Rgb, Roles};

/// The rail's width. Wide enough for the longest section name this application has at the label
/// role, and fixed rather than shrink-to-fit so that switching sections never moves the content
/// beside it — a rail that resized with its own selection would shift the whole form sideways.
const RAIL_WIDTH: f32 = 208.0;

/// One destination in a [`SectionList`].
///
/// A record, not a component: it carries no appearance of its own and never becomes an element on
/// its own terms — the list decides how a destination is drawn, which is what keeps a selected row
/// in one place rather than one per call site. Public fields for the same reason [`MenuItem`]'s
/// are: a caller builds these in a `map`, and a constructor per optional field would be noise.
///
/// [`MenuItem`]: super::MenuItem
pub struct Section<M> {
    /// The destination's name, as shown.
    pub label: String,
    /// Emitted when this destination is pressed — including when it is already current, so that
    /// pressing the current row is inert rather than special.
    pub message: M,
    /// A short trailing marker, shown beside the label. For a destination whose *content* has
    /// something to say from outside it — "Sharing", on a section holding an opt-in that is on.
    pub badge: Option<String>,
}

impl<M> Section<M> {
    /// A destination with no badge.
    pub fn new(label: impl Into<String>, message: M) -> Self {
        Self {
            label: label.into(),
            message,
            badge: None,
        }
    }
}

/// A rail of named destinations with one of them current. Builder form (Principle VIII):
/// `SectionList::new(sections, roles).selected(i).into()`.
pub struct SectionList<'a, M> {
    sections: Vec<Section<M>>,
    selected: usize,
    badge_accent: Option<Rgb>,
    roles: Roles,
    _marker: PhantomData<&'a M>,
}

impl<'a, M: Clone + 'a> SectionList<'a, M> {
    /// A rail showing `sections`, with the first current.
    pub fn new(sections: Vec<Section<M>>, roles: Roles) -> Self {
        Self {
            sections,
            selected: 0,
            badge_accent: None,
            roles,
            _marker: PhantomData,
        }
    }

    /// Which destination is current, by index.
    ///
    /// An index out of range marks none of them rather than panicking or clamping to an end: the
    /// selection is the caller's state, and a caller mid-edit — a section removed, a list rebuilt
    /// — is better served by a rail that shows nothing current for a frame than by one that
    /// silently claims the wrong destination is.
    pub fn selected(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    /// The colour a badge is drawn in. Defaults to the roles' primary accent; state it when the
    /// badge is a *warning* rather than a marker.
    pub fn badge_accent(mut self, accent: Rgb) -> Self {
        self.badge_accent = Some(accent);
        self
    }
}

/// How a row at `index` is drawn given the current selection.
///
/// A free function so the rule can be asserted without a renderer: exactly one index is filled,
/// and an out-of-range selection fills none.
fn variant_at(index: usize, selected: usize) -> ButtonVariant {
    if index == selected {
        ButtonVariant::Filled
    } else {
        ButtonVariant::Text
    }
}

impl<'a, M: Clone + 'a> From<SectionList<'a, M>> for Element<'a, M> {
    fn from(list: SectionList<'a, M>) -> Self {
        let roles = list.roles;
        let accent = list.badge_accent.unwrap_or(roles.primary);
        let selected = list.selected;

        let rows = list.sections.into_iter().enumerate().map(|(i, section)| {
            let variant = variant_at(i, selected);
            // The label is drawn at the button's own content colour, so it is the *variant* that
            // decides whether the current row reads as filled — not a tint chosen here. Building
            // the row's text by hand would put a second answer to "what colour is a button's
            // label" in the library, which is the drift `Button::leading`'s history records.
            let mut content = row![Text::new(section.label, TypeRole::Action, roles)
                .tint(variant.content(roles))
                .width(Length::Fill)]
            .spacing(spacing::SM)
            .align_y(Alignment::Center);
            if let Some(badge) = section.badge {
                content = content.push(Tag::<M>::new(badge, accent).role(TypeRole::Caption));
            }

            Button::with_content(content, variant, roles)
                .width(Length::Fill)
                .on_press(section.message)
                .into()
        });

        container(
            column(rows)
                .spacing(spacing::XS)
                .push(Space::new().height(Length::Fill)),
        )
        .width(Length::Fixed(RAIL_WIDTH))
        .height(Length::Fill)
        .padding(spacing::SM)
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use micold_core::theme::ColorScheme;
    use micold_core::tokens::roles;

    fn sections() -> Vec<Section<()>> {
        vec![
            Section::new("Appearance", ()),
            Section::new("Terminal", ()),
            Section::new("Session service", ()),
        ]
    }

    /// The property a rail exists to have. Two filled rows say two destinations are current, which
    /// is not a state the surface behind it can be in.
    #[test]
    fn exactly_one_row_is_marked_current() {
        for selected in 0..3 {
            let filled = (0..3)
                .filter(|i| variant_at(*i, selected) == ButtonVariant::Filled)
                .count();
            assert_eq!(
                filled, 1,
                "with section {selected} current, {filled} rows were drawn as current"
            );
        }
    }

    /// A selection past the end marks nothing rather than the last row. The rail is a view of the
    /// caller's state; inventing a current destination it did not ask for would have the surface
    /// and its rail disagreeing about what is on screen.
    #[test]
    fn a_selection_out_of_range_marks_nothing() {
        let filled = (0..3)
            .filter(|i| variant_at(*i, 99) == ButtonVariant::Filled)
            .count();
        assert_eq!(filled, 0);
    }

    /// The rail's width does not depend on which row is current, so choosing a section never moves
    /// the form beside it.
    #[test]
    fn the_rail_is_the_same_width_whichever_section_is_current() {
        let r = roles(ColorScheme::Dark);
        for selected in [0usize, 1, 2, 99] {
            let element: Element<'_, ()> =
                SectionList::new(sections(), r).selected(selected).into();
            assert_eq!(
                element.as_widget().size().width,
                Length::Fixed(RAIL_WIDTH),
                "the rail changed width with section {selected} current"
            );
        }
    }

    /// A badge is content, not a second layout: adding one must not change the rail's footprint,
    /// or turning a credential opt-in on would shift the form.
    #[test]
    fn a_badge_does_not_change_the_rails_width() {
        let r = roles(ColorScheme::Dark);
        let mut badged = sections();
        badged[2].badge = Some("Sharing".into());
        let element: Element<'_, ()> = SectionList::new(badged, r).into();
        assert_eq!(element.as_widget().size().width, Length::Fixed(RAIL_WIDTH));
    }

    /// An empty rail is representable. It is not a state this application reaches, but a component
    /// that panics on one is a component that cannot be composed with a computed list.
    #[test]
    fn an_empty_rail_is_representable() {
        let r = roles(ColorScheme::Dark);
        let element: Element<'_, ()> = SectionList::new(Vec::<Section<()>>::new(), r).into();
        assert_eq!(element.as_widget().size().width, Length::Fixed(RAIL_WIDTH));
    }
}
