//! Required fields: which fields a form refuses empty, and when to say so (feature 937).
//!
//! A form's render-free feature module says, per field, whether the field is required now and
//! whether it is empty ([`Requirement`]). [`RequiredMarks`] remembers what the user did on the open
//! draft (fields left, a submit attempted) and turns the two into a [`FieldRequirement`], which the
//! material components draw: the [`MARKER`] after the label, and for [`FieldRequirement::Missing`]
//! the error state with [`MESSAGE`]. [`LEGEND`] explains the marker once per form.
//!
//! Contract: `specs/937-required-field-markers/contracts/required-field.md`.

use std::collections::BTreeSet;

/// The explanation a form with a required field shows once, above its first field (FR-004).
pub const LEGEND: &str = "* required";

/// The error a required field shows when it is left or submitted empty (FR-005).
pub const MESSAGE: &str = "This field is required";

/// The character appended to a required field's label (FR-001, FR-003).
pub const MARKER: &str = "*";

/// How a field is drawn with respect to being required.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FieldRequirement {
    /// Not required: no marker, no required error.
    #[default]
    Optional,
    /// Required: the marker, no error.
    Required,
    /// Required, empty, and the user has left it or attempted a submit: the marker and the
    /// required error (unless the field's own error is set, which wins — FR-010).
    Missing,
}

/// What a form says about one field now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Requirement {
    /// The form refuses to proceed with this field empty, given its other fields (FR-002).
    pub required: bool,
    /// The field holds no value, by the trim rule of the validator it mirrors.
    pub empty: bool,
}

impl Requirement {
    /// A text field: empty when blank after trimming.
    #[must_use]
    pub fn text(required: bool, value: &str) -> Self {
        Self {
            required,
            empty: value.trim().is_empty(),
        }
    }

    /// A choice (select, picker): empty when nothing is chosen.
    #[must_use]
    pub fn choice<T>(required: bool, value: Option<&T>) -> Self {
        Self {
            required,
            empty: value.is_none(),
        }
    }
}

/// What the user has done on one open draft: the fields they left, and whether they attempted a
/// submit. Default is a fresh form, on which no required error shows (FR-006).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequiredMarks<K: Ord + Copy> {
    left: BTreeSet<K>,
    attempted: bool,
}

impl<K: Ord + Copy> Default for RequiredMarks<K> {
    fn default() -> Self {
        Self {
            left: BTreeSet::new(),
            attempted: false,
        }
    }
}

impl<K: Ord + Copy> RequiredMarks<K> {
    /// Records that the user left field `k` (blur).
    pub fn leave(&mut self, k: K) {
        self.left.insert(k);
    }

    /// Records a submit attempt on the draft.
    pub fn attempt(&mut self) {
        self.attempted = true;
    }

    /// How field `k` is drawn, given what the form says about it now. Pure over the marks and the
    /// requirement, so filling a field clears its error in the same update (FR-007).
    #[must_use]
    pub fn state(&self, k: K, requirement: Requirement) -> FieldRequirement {
        if !requirement.required {
            FieldRequirement::Optional
        } else if requirement.empty && (self.attempted || self.left.contains(&k)) {
            FieldRequirement::Missing
        } else {
            FieldRequirement::Required
        }
    }

    /// How field `k` is drawn, looked up in a form's `required_fields()` list: a field the list
    /// does not name is [`FieldRequirement::Optional`].
    #[must_use]
    pub fn state_in(&self, fields: &[(K, Requirement)], k: K) -> FieldRequirement {
        fields
            .iter()
            .find(|(f, _)| *f == k)
            .map_or(FieldRequirement::Optional, |(_, r)| self.state(k, *r))
    }

    /// Whether a form whose fields are `requirements` shows the [`LEGEND`]: any field required.
    #[must_use]
    pub fn legend(requirements: impl IntoIterator<Item = Requirement>) -> bool {
        requirements.into_iter().any(|r| r.required)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_in_reads_a_listed_field_and_leaves_an_unlisted_one_optional() {
        let fields = [(1u8, Requirement::text(true, ""))];
        let mut marks = RequiredMarks::default();
        assert_eq!(marks.state_in(&fields, 1), FieldRequirement::Required);
        assert_eq!(marks.state_in(&fields, 2), FieldRequirement::Optional);
        marks.attempt();
        assert_eq!(marks.state_in(&fields, 1), FieldRequirement::Missing);
        assert_eq!(marks.state_in(&fields, 2), FieldRequirement::Optional);
    }

    const NAME: u8 = 1;
    const TYPE: u8 = 2;

    fn required_empty() -> Requirement {
        Requirement {
            required: true,
            empty: true,
        }
    }

    #[test]
    fn text_is_empty_when_blank_after_trim() {
        assert!(
            Requirement::text(true, "  ").empty,
            "whitespace only counts as empty, as the validators trim"
        );
        assert!(
            !Requirement::text(true, " a ").empty,
            "a value with content around spaces is not empty"
        );
    }

    #[test]
    fn choice_is_empty_when_nothing_is_chosen() {
        assert!(
            Requirement::choice(true, None::<&u8>).empty,
            "no choice is empty"
        );
        assert!(
            !Requirement::choice(true, Some(&3u8)).empty,
            "a choice is not empty"
        );
    }

    #[test]
    fn fresh_form_never_shows_the_error() {
        let marks = RequiredMarks::<u8>::default();
        assert_eq!(
            marks.state(NAME, required_empty()),
            FieldRequirement::Required,
            "a fresh form marks a required empty field but shows no error (FR-006)"
        );
    }

    #[test]
    fn leaving_a_field_shows_the_error_on_that_field_only() {
        let mut marks = RequiredMarks::<u8>::default();
        marks.leave(NAME);
        assert_eq!(
            marks.state(NAME, required_empty()),
            FieldRequirement::Missing
        );
        assert_eq!(
            marks.state(TYPE, required_empty()),
            FieldRequirement::Required,
            "a field the user did not leave shows no error"
        );
    }

    #[test]
    fn a_submit_attempt_shows_the_error_on_every_required_empty_field() {
        let mut marks = RequiredMarks::<u8>::default();
        marks.attempt();
        assert_eq!(
            marks.state(NAME, required_empty()),
            FieldRequirement::Missing
        );
        assert_eq!(
            marks.state(TYPE, required_empty()),
            FieldRequirement::Missing
        );
    }

    #[test]
    fn a_filled_field_clears_the_error_and_keeps_the_marks() {
        let mut marks = RequiredMarks::<u8>::default();
        marks.attempt();
        marks.leave(NAME);
        let before = marks.clone();
        let filled = Requirement::text(true, "feature");
        assert_eq!(
            marks.state(NAME, filled),
            FieldRequirement::Required,
            "a value clears the error in the same update (FR-007)"
        );
        assert_eq!(marks, before, "reading the state changes no marks");
        assert_eq!(
            marks.state(NAME, required_empty()),
            FieldRequirement::Missing,
            "emptying it again shows the error again"
        );
    }

    #[test]
    fn a_field_that_is_not_required_is_optional_whatever_the_marks() {
        let mut marks = RequiredMarks::<u8>::default();
        marks.attempt();
        marks.leave(NAME);
        let optional = Requirement {
            required: false,
            empty: true,
        };
        assert_eq!(
            marks.state(NAME, optional),
            FieldRequirement::Optional,
            "a field that stops being required drops marker and error"
        );
    }

    #[test]
    fn legend_shows_exactly_when_a_field_is_required() {
        let optional = Requirement {
            required: false,
            empty: false,
        };
        assert!(!RequiredMarks::<u8>::legend([]), "no fields, no legend");
        assert!(
            !RequiredMarks::<u8>::legend([optional, optional]),
            "no required field, no legend"
        );
        assert!(
            RequiredMarks::<u8>::legend([optional, Requirement::text(true, "x")]),
            "one required field, even filled, shows the legend"
        );
    }

    #[test]
    fn strings_are_the_material_wording() {
        assert_eq!(LEGEND, "* required");
        assert_eq!(MESSAGE, "This field is required");
        assert_eq!(MARKER, "*");
    }
}
