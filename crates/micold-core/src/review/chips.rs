//! Suggestion chips under the comment box (#1275, US3; FR-023, FR-024): canned comment text the
//! reviewer can drop in with one press, shown only when `Settings::suggestion_chips` is on.

/// One chip: the label it shows and the text it puts in the box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SuggestionChip {
    pub label: &'static str,
    pub text: &'static str,
}

/// The chips, in the order they show.
pub const CHIPS: [SuggestionChip; 3] = [
    SuggestionChip {
        label: "Rename",
        text: "Rename this to ",
    },
    SuggestionChip {
        label: "Add test",
        text: "Add a test that covers this.",
    },
    SuggestionChip {
        label: "Why?",
        text: "Why is this needed?",
    },
];

/// The box's text after pressing `chip` (FR-024): an empty (or blank) box takes the chip's text;
/// otherwise the chip's text goes on a new line after what is there.
pub fn insert(text: &str, chip: &SuggestionChip) -> String {
    if text.trim().is_empty() {
        chip.text.to_owned()
    } else if text.ends_with('\n') {
        format!("{text}{}", chip.text)
    } else {
        format!("{text}\n{}", chip.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHIP: SuggestionChip = CHIPS[1];

    /// K1: the spec's three chips, in order.
    #[test]
    fn k1_the_chips_are_rename_add_test_and_why() {
        assert_eq!(
            CHIPS,
            [
                SuggestionChip {
                    label: "Rename",
                    text: "Rename this to "
                },
                SuggestionChip {
                    label: "Add test",
                    text: "Add a test that covers this."
                },
                SuggestionChip {
                    label: "Why?",
                    text: "Why is this needed?"
                },
            ]
        );
    }

    /// K2: an empty or blank box takes the chip's text.
    #[test]
    fn k2_an_empty_or_blank_box_takes_the_chips_text() {
        assert_eq!(insert("", &CHIP), CHIP.text);
        assert_eq!(insert("  \n", &CHIP), CHIP.text);
    }

    /// K3: text already typed stays, and the chip's text goes on a new line.
    #[test]
    fn k3_typed_text_is_kept_and_the_chip_goes_on_a_new_line() {
        assert_eq!(
            insert("Looks off", &CHIP),
            format!("Looks off\n{}", CHIP.text)
        );
    }

    /// K4: no blank line when the text already ends with a newline.
    #[test]
    fn k4_a_trailing_newline_adds_no_blank_line() {
        assert_eq!(
            insert("Looks off\n", &CHIP),
            format!("Looks off\n{}", CHIP.text)
        );
    }
}
