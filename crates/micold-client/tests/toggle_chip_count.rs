//! T044 [US3] — the shared chip's optional count (feature 029, FR-025/FR-025a; Constitution
//! Principle VIII).
//!
//! The count exists because the FR-006 migration can take rows out of a user's list on first
//! launch, and a number beside the reveal control is what accounts for them on screen. The design
//! constraint is that it must cost the other chips nothing: `ToggleChip` is the sidebar's filter
//! chip too, and a feature that shifted those by a pixel would be a visual regression in a control
//! this feature has no business touching.
//!
//! Asserted against [`chip_label`] rather than against a rendered `Element`: a chip is an iced
//! widget tree, and comparing pixels would be testing iced. The rule the feature actually adds is
//! the string, and that is what is here.

use micold_client::ui::chip_label;

#[test]
fn a_zero_count_renders_exactly_the_label() {
    // FR-025a, and the whole of the "costs the other chips nothing" guarantee: a chip that never
    // sets a count and a chip whose count is zero produce the same text, so they lay out the same.
    assert_eq!(
        chip_label("Show agent worktrees", 0),
        "Show agent worktrees"
    );
}

#[test]
fn a_count_is_appended_after_a_middle_dot() {
    assert_eq!(
        chip_label("Show agent worktrees", 3),
        "Show agent worktrees · 3"
    );
}

#[test]
fn the_label_itself_is_never_rewritten() {
    // FR-015a: 014's wording is what users learned to look for, so the count sits beside the label
    // rather than replacing or rephrasing it. Asserted as a prefix so a later change to the
    // separator cannot quietly take the label with it.
    for count in [0, 1, 9, 150] {
        let rendered = chip_label("Show agent worktrees", count);
        assert!(
            rendered.starts_with("Show agent worktrees"),
            "count {count} rewrote the label: {rendered}"
        );
    }
}

#[test]
fn a_count_of_one_is_not_special_cased() {
    // No pluralisation, because there is no noun — the number stands alone beside the label.
    assert_eq!(
        chip_label("Show agent worktrees", 1),
        "Show agent worktrees · 1"
    );
}

#[test]
fn an_uncounted_chip_of_any_label_is_returned_verbatim() {
    // The filter chips are the population this protects: they never call `.count`, so they take
    // the zero default, and every one of them must come out exactly as it went in.
    for label in ["feat", "fix", "chore", "Clear"] {
        assert_eq!(chip_label(label, 0), label);
    }
}
