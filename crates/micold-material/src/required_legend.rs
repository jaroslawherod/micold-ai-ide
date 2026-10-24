//! `RequiredLegend` — the one line that explains the required-field marker (feature 937, C3).
//!
//! Material asks a form to state its convention once rather than leave the `*` to be guessed at.
//! A form pushes this as the first child of its fields column when it has a required field
//! (`RequiredMarks::legend`, FR-004); no form spells the words itself (FR-008, SC-006).

use std::marker::PhantomData;

use iced::Element;
use micold_core::required::{RequiredMarks, Requirement, LEGEND};
use micold_core::tokens::Roles;

use super::{Text, TypeRole};

/// `* required`, as muted `Caption` text. Builder form (Principle VIII):
/// `RequiredLegend::new(roles).into()`.
pub struct RequiredLegend<'a, M> {
    roles: Roles,
    marker: PhantomData<&'a M>,
}

impl<'a, M: 'a> RequiredLegend<'a, M> {
    /// The legend, themed by `roles`.
    pub fn new(roles: Roles) -> Self {
        Self {
            roles,
            marker: PhantomData,
        }
    }

    /// The legend for a form whose fields are `fields` (its `required_fields()`), or `None` when
    /// none of them is required: a form without a required field shows no legend (FR-004).
    pub fn for_fields<K: Ord + Copy>(
        fields: &[(K, Requirement)],
        roles: Roles,
    ) -> Option<Element<'a, M>> {
        RequiredMarks::<K>::legend(fields.iter().map(|(_, r)| *r)).then(|| Self::new(roles).into())
    }
}

impl<'a, M: 'a> From<RequiredLegend<'a, M>> for Element<'a, M> {
    fn from(l: RequiredLegend<'a, M>) -> Self {
        Text::new(LEGEND, TypeRole::Caption, l.roles).muted().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{roles, size_of};
    use iced::Size;

    const ROOM: Size = Size::new(400.0, 200.0);

    /// C3 / R6: the legend is the `LEGEND` words at `Caption`, muted, and nothing else — it lays
    /// out exactly as that text does, one `Caption` line tall.
    #[test]
    fn it_is_the_legend_text_at_caption() {
        let r = roles();
        let legend: Element<'_, ()> = RequiredLegend::new(r).into();
        let text: Element<'_, ()> = Text::new(LEGEND, TypeRole::Caption, r).muted().into();
        let (legend, text) = (size_of(legend, ROOM), size_of(text, ROOM));
        assert_eq!(legend, text);
        assert_eq!(legend.height, TypeRole::Caption.line_height_dp());
    }

    /// FR-004: any required field shows it; none shows nothing.
    #[test]
    fn it_shows_only_for_a_form_with_a_required_field() {
        let r = roles();
        let some = [(1u8, Requirement::text(true, ""))];
        let none = [(1u8, Requirement::text(false, ""))];
        assert!(RequiredLegend::<()>::for_fields(&some, r).is_some());
        assert!(RequiredLegend::<()>::for_fields(&none, r).is_none());
        assert!(RequiredLegend::<()>::for_fields::<u8>(&[], r).is_none());
    }

    /// A different wording is a different width: guards the test above against a legend that
    /// ignores `LEGEND`.
    #[test]
    fn its_width_follows_the_words() {
        let r = roles();
        let legend: Element<'_, ()> = RequiredLegend::new(r).into();
        let other: Element<'_, ()> = Text::new("x", TypeRole::Caption, r).muted().into();
        assert!(size_of(legend, ROOM).width > size_of(other, ROOM).width);
    }
}
