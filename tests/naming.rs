//! T025 [US2] — worktree naming derivation (FR-005a/b, FR-006, FR-008, SC-003b).

use micold_ai_ide::naming::{
    derive, display_name, parse_tags, slugify, ConventionalType, DerivedNames, NamingError, Tag,
    WorktreeNaming,
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

// --- Feature 008: friendly-name + tag derivation (contracts/naming-tags.md) ---

#[test]
fn parse_tags_type_and_issue() {
    assert_eq!(
        parse_tags("feat-abc-123-login-page"),
        vec![
            Tag::Type(ConventionalType::Feat),
            Tag::Issue("ABC-123".to_string()),
        ]
    );
}

#[test]
fn parse_tags_type_only_when_no_issue() {
    assert_eq!(
        parse_tags("fix-crash-on-open"),
        vec![Tag::Type(ConventionalType::Fix)]
    );
    assert_eq!(
        parse_tags("chore-bump-deps"),
        vec![Tag::Type(ConventionalType::Chore)]
    );
}

#[test]
fn parse_tags_ignores_trailing_number_in_name() {
    // A number at the END of a multi-word name is NOT a ticket — the ticket immediately follows
    // the type. So `feat-add-retry-3` is name "add retry 3", not issue "RETRY-3".
    assert_eq!(
        parse_tags("feat-add-retry-3"),
        vec![Tag::Type(ConventionalType::Feat)]
    );
    assert_eq!(display_name("feat-add-retry-3"), "Add retry 3");
}

#[test]
fn parse_tags_untyped_when_no_known_type() {
    assert!(parse_tags("my-experiment").is_empty());
    assert!(parse_tags("main").is_empty());
}

#[test]
fn parse_tags_returns_at_most_one_type_and_issue() {
    let tags = parse_tags("feat-abc-123-def-456-thing");
    assert_eq!(tags.iter().filter(|t| matches!(t, Tag::Type(_))).count(), 1);
    assert_eq!(
        tags.iter().filter(|t| matches!(t, Tag::Issue(_))).count(),
        1
    );
    // First issue pair wins.
    assert_eq!(tags[1], Tag::Issue("ABC-123".to_string()));
}

#[test]
fn parse_tags_never_yields_status() {
    for name in ["feat-x", "main", "fix-abc-1-y"] {
        assert!(parse_tags(name)
            .iter()
            .all(|t| !matches!(t, Tag::Status(_))));
    }
}

#[test]
fn display_name_strips_type_and_issue() {
    assert_eq!(display_name("feat-abc-123-login-page"), "Login page");
    assert_eq!(display_name("fix-crash-on-open"), "Crash on open");
    assert_eq!(display_name("chore-bump-deps"), "Bump deps");
}

#[test]
fn display_name_untyped_names() {
    assert_eq!(display_name("my-experiment"), "My experiment");
    assert_eq!(display_name("main"), "Main");
}

#[test]
fn display_name_falls_back_when_only_type_and_issue() {
    // Nothing descriptive remains → readable form of the whole dir name.
    assert_eq!(display_name("feat-abc-123"), "Feat abc 123");
}

#[test]
fn display_name_never_empty() {
    for name in ["feat-abc-123", "main", "x", "feat-x"] {
        assert!(!display_name(name).is_empty());
    }
}

// =======================================================================================
// Feature 016 — the branch → directory inverse mapping (FR-014).
// =======================================================================================

use micold_ai_ide::naming::dir_name_from_branch;

#[test]
fn dir_name_from_branch_inverts_the_derive_mapping() {
    // The round trip that matters: a branch `derive()` would have produced maps back to the
    // directory `derive()` would have produced alongside it.
    for (type_, ticket, name) in [
        ("feat", Some("abc-123"), "login page"),
        ("fix", None, "crash on start"),
        ("chore", Some("x1"), "bump deps"),
    ] {
        let derived = derive(&WorktreeNaming {
            type_: ConventionalType::from_token(type_),
            ticket: ticket.map(str::to_string),
            name: name.to_string(),
        })
        .unwrap();
        assert_eq!(
            dir_name_from_branch(&derived.branch),
            derived.dir_name,
            "branch {} should map back to its own directory",
            derived.branch
        );
    }
}

#[test]
fn dir_name_from_branch_flattens_every_segment() {
    assert_eq!(
        dir_name_from_branch("feat/abc-123-login"),
        "feat-abc-123-login"
    );
    assert_eq!(dir_name_from_branch("main"), "main");
    assert_eq!(
        dir_name_from_branch("feature/JIRA-9/Fix Thing"),
        "feature-jira-9-fix-thing"
    );
}

#[test]
fn dir_name_from_branch_slugifies_each_segment() {
    // Uppercase folded, punctuation collapsed to single dashes, no leading/trailing dash.
    assert_eq!(dir_name_from_branch("Feat/Login_Page!"), "feat-login-page");
    assert_eq!(dir_name_from_branch("a//b"), "a-b");
    assert_eq!(
        dir_name_from_branch("/leading/trailing/"),
        "leading-trailing"
    );
}

#[test]
fn dir_name_from_branch_keeps_the_windows_reserved_name_guard() {
    // Inherited from `slugify` — a directory literally named `con` is unusable on Windows
    // (Constitution Principle VI).
    assert_eq!(dir_name_from_branch("con"), "con-wt");
    assert_eq!(dir_name_from_branch("feat/con"), "feat-con-wt");
}

#[test]
fn dir_name_from_branch_yields_empty_when_nothing_usable_remains() {
    // Callers treat empty as "cannot derive a directory" rather than creating `.claude/worktrees/`.
    assert_eq!(dir_name_from_branch("///"), "");
    assert_eq!(dir_name_from_branch("!!!"), "");
}

#[test]
fn dir_name_from_branch_output_is_always_a_valid_directory_segment() {
    for branch in [
        "feat/x",
        "Feat/ABC-1/Some Name",
        "release/v1.2.3",
        "a/b/c/d",
    ] {
        let dir = dir_name_from_branch(branch);
        assert!(!dir.is_empty());
        assert!(!dir.starts_with('-') && !dir.ends_with('-'), "{dir}");
        assert!(
            dir.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "{dir} must be [a-z0-9-] only"
        );
    }
}
