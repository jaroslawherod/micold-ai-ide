//! The label-to-type mapping (feature 034, contracts/issue-naming-and-typing.md §2): the default
//! table and the first-entry-wins lookup that turns an issue's labels into a worktree type.

use micold_core::issue_types::{
    self, type_for_labels, validate_mapping, LabelTypeEntry, MappingError, MappingErrorKind,
};
use micold_core::naming::ConventionalType;

fn entry(label: &str, type_: ConventionalType) -> LabelTypeEntry {
    LabelTypeEntry {
        label: label.to_string(),
        type_,
    }
}

fn labels(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| n.to_string()).collect()
}

#[test]
fn default_mapping() {
    assert_eq!(
        issue_types::default_mapping(),
        vec![
            entry("bug", ConventionalType::Fix),
            entry("enhancement", ConventionalType::Feat),
            entry("documentation", ConventionalType::Docs),
        ],
        "the default maps GitHub's stock type-bearing labels, in this order (FR-021)"
    );
}

#[test]
fn mapping_order_wins() {
    let mapping = issue_types::default_mapping();
    assert_eq!(
        type_for_labels(&mapping, &labels(&["bug"])),
        Some(ConventionalType::Fix),
        "a `bug` label selects `fix` (AS1)"
    );
    assert_eq!(
        type_for_labels(&mapping, &labels(&["enhancement", "bug"])),
        Some(ConventionalType::Fix),
        "the mapping entry listed first wins, whatever order the issue lists its labels (AS2)"
    );
    assert_eq!(
        type_for_labels(&mapping, &labels(&["bug", "enhancement"])),
        Some(ConventionalType::Fix),
        "the mapping's order decides, not the issue's (AS2)"
    );
    let reversed: Vec<_> = mapping.into_iter().rev().collect();
    assert_eq!(
        type_for_labels(&reversed, &labels(&["bug", "enhancement"])),
        Some(ConventionalType::Feat),
        "reordering the mapping changes the winner (FR-017)"
    );
}

#[test]
fn case_and_no_match() {
    let mapping = vec![entry(" Bug ", ConventionalType::Fix)];
    assert_eq!(
        type_for_labels(&mapping, &labels(&["bug"])),
        Some(ConventionalType::Fix),
        "an entry `Bug` matches a label `bug`, trimmed and ignoring case (AS5)"
    );
    assert_eq!(
        type_for_labels(&issue_types::default_mapping(), &labels(&["BUG "])),
        Some(ConventionalType::Fix),
        "the issue's label is trimmed and case-folded too"
    );
    assert_eq!(
        type_for_labels(
            &issue_types::default_mapping(),
            &labels(&["question", "wontfix"])
        ),
        None,
        "no mapped label gives no type (AS3)"
    );
    assert_eq!(
        type_for_labels(&issue_types::default_mapping(), &[]),
        None,
        "an issue without labels gets no type"
    );
    assert_eq!(
        type_for_labels(&[], &labels(&["bug"])),
        None,
        "an empty mapping maps nothing"
    );
    let two_to_one = vec![
        entry("bug", ConventionalType::Fix),
        entry("regression", ConventionalType::Fix),
    ];
    assert_eq!(
        type_for_labels(&two_to_one, &labels(&["regression"])),
        Some(ConventionalType::Fix),
        "several labels may map to one type (spec Edge Cases)"
    );
}

#[test]
fn validation() {
    assert_eq!(
        validate_mapping(&issue_types::default_mapping()),
        Ok(()),
        "the default mapping is valid"
    );
    assert_eq!(validate_mapping(&[]), Ok(()), "an empty mapping is valid");
    assert_eq!(
        validate_mapping(&[
            entry("bug", ConventionalType::Fix),
            entry("   ", ConventionalType::Feat),
        ]),
        Err(MappingError {
            index: 1,
            kind: MappingErrorKind::Blank
        }),
        "a label that is blank after trimming is refused at its index (FR-019)"
    );
    assert_eq!(
        validate_mapping(&[
            entry("bug", ConventionalType::Fix),
            entry("docs", ConventionalType::Docs),
            entry("Bug ", ConventionalType::Chore),
        ]),
        Err(MappingError {
            index: 2,
            kind: MappingErrorKind::Duplicate { of: 0 }
        }),
        "a label repeating an earlier one ignoring case and spaces names the earlier index (FR-019)"
    );
    assert_eq!(
        validate_mapping(&[
            entry("bug", ConventionalType::Fix),
            entry("", ConventionalType::Feat),
            entry("BUG", ConventionalType::Fix),
        ]),
        Err(MappingError {
            index: 1,
            kind: MappingErrorKind::Blank
        }),
        "the first offending entry in order is the one reported"
    );
    assert_eq!(
        validate_mapping(&[
            entry("bug", ConventionalType::Fix),
            entry("regression", ConventionalType::Fix),
        ]),
        Ok(()),
        "the same type twice is valid (spec Edge Cases)"
    );
}

// --- 533: a Jira issue's type name joins its labels (contracts/issue-naming.md §2) -------------------

#[test]
fn a_type_name_is_matched_like_a_label() {
    let mapping = issue_types::default_mapping();
    assert_eq!(
        issue_types::type_for_issue(&mapping, Some("Bug"), &[]),
        Some(ConventionalType::Fix),
        "a Jira `Bug` issue type maps to `fix` by default"
    );
    assert_eq!(
        issue_types::type_for_issue(&mapping, Some(" BUG "), &[]),
        Some(ConventionalType::Fix),
        "trimmed and case-insensitive"
    );
    assert_eq!(
        issue_types::type_for_issue(&mapping, Some("Story"), &labels(&["enhancement"])),
        Some(ConventionalType::Feat),
        "a label still counts when the type name maps to nothing"
    );
    assert_eq!(
        issue_types::type_for_issue(&mapping, Some("Task"), &[]),
        None
    );
}

#[test]
fn the_first_mapping_entry_wins_whether_the_match_is_the_type_name_or_a_label() {
    let mapping = issue_types::default_mapping();
    assert_eq!(
        issue_types::type_for_issue(&mapping, Some("Task"), &labels(&["documentation", "bug"])),
        Some(ConventionalType::Fix),
        "`bug` is listed before `documentation`"
    );
    assert_eq!(
        issue_types::type_for_issue(&mapping, Some("documentation"), &labels(&["bug"])),
        Some(ConventionalType::Fix),
        "the mapping's order decides, not type-name-first"
    );
}

#[test]
fn without_a_type_name_it_is_type_for_labels() {
    let mapping = issue_types::default_mapping();
    for set in [&["bug"][..], &["enhancement", "bug"], &["question"], &[]] {
        assert_eq!(
            issue_types::type_for_issue(&mapping, None, &labels(set)),
            type_for_labels(&mapping, &labels(set))
        );
    }
}
