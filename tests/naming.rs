//! T025 [US2] — worktree naming derivation (FR-005a/b, FR-006, FR-008, SC-003b).

use micold_ai_ide::naming::{
    derive, slugify, ConventionalType, DerivedNames, NamingError, WorktreeNaming,
};

fn naming(type_: Option<ConventionalType>, ticket: Option<&str>, name: &str) -> WorktreeNaming {
    WorktreeNaming {
        type_,
        ticket: ticket.map(str::to_string),
        name: name.to_string(),
    }
}

#[test]
fn derives_dir_and_branch_with_ticket() {
    let got = derive(&naming(
        Some(ConventionalType::Feat),
        Some("ABC-123"),
        "Login page",
    ))
    .unwrap();
    assert_eq!(
        got,
        DerivedNames {
            dir_name: "feat-abc-123-login-page".to_string(),
            branch: "feat/abc-123-login-page".to_string(),
        }
    );
}

#[test]
fn omits_ticket_segment_when_absent() {
    let got = derive(&naming(Some(ConventionalType::Chore), None, "cleanup")).unwrap();
    assert_eq!(got.dir_name, "chore-cleanup");
    assert_eq!(got.branch, "chore/cleanup");
}

#[test]
fn blank_ticket_is_treated_as_absent() {
    let got = derive(&naming(Some(ConventionalType::Fix), Some("   "), "bug")).unwrap();
    assert_eq!(got.dir_name, "fix-bug");
    assert_eq!(got.branch, "fix/bug");
}

#[test]
fn slugifies_illegal_and_separator_characters() {
    // FR-008: separators / illegal chars are sanitized, not rejected.
    let got = derive(&naming(
        Some(ConventionalType::Fix),
        Some("#42!"),
        "Race/cond",
    ))
    .unwrap();
    assert_eq!(got.dir_name, "fix-42-race-cond");
    assert_eq!(got.branch, "fix/42-race-cond");
}

#[test]
fn missing_type_is_rejected() {
    let err = derive(&naming(None, None, "thing")).unwrap_err();
    assert_eq!(err, NamingError::NoType);
}

#[test]
fn empty_name_after_slug_is_rejected() {
    let err = derive(&naming(Some(ConventionalType::Feat), None, "!!!")).unwrap_err();
    assert_eq!(err, NamingError::EmptyNameAfterSlug);
}

#[test]
fn derived_dir_has_no_slash_and_branch_has_one() {
    // SC-003b structural guarantees.
    let got = derive(&naming(
        Some(ConventionalType::Docs),
        Some("T-9"),
        "read me",
    ))
    .unwrap();
    assert!(!got.dir_name.contains('/'));
    assert_eq!(got.branch.matches('/').count(), 1);
}

#[test]
fn deterministic() {
    let a = derive(&naming(
        Some(ConventionalType::Perf),
        Some("X1"),
        "Speed Up",
    ))
    .unwrap();
    let b = derive(&naming(
        Some(ConventionalType::Perf),
        Some("X1"),
        "Speed Up",
    ))
    .unwrap();
    assert_eq!(a, b);
}

#[test]
fn slugify_collapses_and_trims() {
    assert_eq!(slugify("  Hello --- World!!  "), "hello-world");
    assert_eq!(slugify("___"), "");
    assert_eq!(slugify("ABC"), "abc");
}

#[test]
fn slugify_guards_windows_reserved_names() {
    assert_eq!(slugify("CON"), "con-wt");
    assert_eq!(slugify("nul"), "nul-wt");
}

#[test]
fn every_conventional_type_has_a_lowercase_token() {
    for t in ConventionalType::ALL {
        let s = t.as_str();
        assert!(!s.is_empty());
        assert!(s.chars().all(|c| c.is_ascii_lowercase()));
    }
}
