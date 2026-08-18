//! T025 [US2] — worktree naming derivation (FR-005a/b, FR-006, FR-008, SC-003b).

use micold_core::naming::{
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
            dir_name: "feat-abc-123_login-page".to_string(),
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
    assert_eq!(got.dir_name, "fix-42_race-cond");
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
        parse_tags("feat-abc-123_login-page"),
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
fn a_number_in_a_name_is_never_a_ticket() {
    // BUG-003. No `_`, so there is no ticket to find — however much the tail looks like one.
    // `feat-reporting-2` is the case that was misread as issue `REPORTING-2`, which emptied the
    // descriptive remainder and made the whole label fall back to "Feat reporting 2".
    for (dir_name, expected) in [
        ("feat-add-retry-3", "Add retry 3"),
        ("feat-reporting-2", "Reporting 2"),
        ("feat-auth-2", "Auth 2"),
        ("feat-abc-123", "Abc 123"),
    ] {
        assert_eq!(
            parse_tags(dir_name),
            vec![Tag::Type(ConventionalType::Feat)],
            "{dir_name} has no ticket boundary, so it has no ticket"
        );
        assert_eq!(display_name(dir_name), expected);
    }
}

#[test]
fn parse_tags_untyped_when_no_known_type() {
    assert!(parse_tags("my-experiment").is_empty());
    assert!(parse_tags("main").is_empty());
}

#[test]
fn parse_tags_returns_at_most_one_type_and_issue() {
    let tags = parse_tags("feat-abc-123_def-456-thing");
    assert_eq!(tags.iter().filter(|t| matches!(t, Tag::Type(_))).count(), 1);
    assert_eq!(
        tags.iter().filter(|t| matches!(t, Tag::Issue(_))).count(),
        1
    );
    // Everything between the type and the boundary is the ticket; everything after it is the
    // name, digits and all.
    assert_eq!(tags[1], Tag::Issue("ABC-123".to_string()));
    assert_eq!(display_name("feat-abc-123_def-456-thing"), "Def 456 thing");
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
    assert_eq!(display_name("feat-abc-123_login-page"), "Login page");
    assert_eq!(display_name("fix-crash-on-open"), "Crash on open");
    assert_eq!(display_name("chore-bump-deps"), "Bump deps");
}

#[test]
fn display_name_untyped_names() {
    assert_eq!(display_name("my-experiment"), "My experiment");
    assert_eq!(display_name("main"), "Main");
}

#[test]
fn display_name_falls_back_when_nothing_descriptive_remains() {
    // A boundary with nothing after it: readable form of the whole dir name, since the
    // alternative is an empty label.
    assert_eq!(display_name("feat-abc-123_"), "Feat abc 123");
    assert_eq!(display_name("feat"), "Feat");
}

#[test]
fn display_name_never_empty() {
    for name in ["feat-abc-123", "feat-abc-123_", "main", "x", "feat-x", "_"] {
        assert!(!display_name(name).is_empty());
    }
}

// =======================================================================================
// Feature 016 — the branch → directory inverse mapping (FR-014).
// =======================================================================================

use micold_core::naming::dir_name_from_branch;

#[test]
fn dir_name_from_branch_inverts_the_derive_mapping_when_there_is_no_ticket() {
    for (type_, name) in [("fix", "crash on start"), ("chore", "bump deps")] {
        let derived = derive(&WorktreeNaming {
            type_: ConventionalType::from_token(type_),
            ticket: None,
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

/// A branch does not carry the ticket boundary — only the directory does — so a worktree created
/// from a *selected* branch lands in a ticketless directory even when the branch was originally
/// derived from a ticket (BUG-003).
///
/// This asymmetry is deliberate. The alternative is to guess where the boundary goes, which is
/// exactly the guess that produced BUG-003, and it would be made against a branch that may not
/// have come from this app at all. A ticketless reading is wrong about the chip; a guessed one is
/// wrong about the name.
#[test]
fn dir_name_from_branch_cannot_recover_a_ticket_the_branch_never_carried() {
    let derived = derive(&WorktreeNaming {
        type_: ConventionalType::from_token("feat"),
        ticket: Some("abc-123".to_string()),
        name: "login page".to_string(),
    })
    .unwrap();
    assert_eq!(derived.dir_name, "feat-abc-123_login-page");
    assert_eq!(derived.branch, "feat/abc-123-login-page");

    let from_branch = dir_name_from_branch(&derived.branch);
    assert_eq!(from_branch, "feat-abc-123-login-page");
    assert!(
        !from_branch.contains('_'),
        "the inverse must never invent a boundary"
    );
    assert_eq!(
        parse_tags(&from_branch),
        vec![Tag::Type(ConventionalType::Feat)],
        "no boundary, no ticket — not a guessed one"
    );
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

// =======================================================================================
// BUG-003 — the ticket boundary. A directory name says whether it has a ticket instead of
// being asked to look like one.
// =======================================================================================

/// The property the whole scheme rests on: `slugify` maps every non-alphanumeric character to
/// `-`, so a derived name can never contain `_`. That is what frees the character to mean
/// exactly one thing.
#[test]
fn slugify_can_never_produce_the_ticket_boundary() {
    for raw in ["a_b", "Login_Page!", "___", "ABC_123_x"] {
        assert!(
            !slugify(raw).contains('_'),
            "slugify({raw:?}) = {:?}",
            slugify(raw)
        );
    }
}

/// A GitHub/GitLab reference is a bare number, which the old pair rule could not recognise at
/// all — `#123` slugified to `123`, failed the "starts with a letter" test, and was dropped
/// silently while its digits leaked into the display name.
#[test]
fn a_numeric_ticket_is_kept_and_rendered_as_an_issue_number() {
    let got = derive(&naming(
        Some(ConventionalType::Feat),
        Some("#123"),
        "Login page",
    ))
    .unwrap();
    assert_eq!(got.dir_name, "feat-123_login-page");
    assert_eq!(got.branch, "feat/123-login-page");
    assert_eq!(
        parse_tags(&got.dir_name),
        vec![
            Tag::Type(ConventionalType::Feat),
            Tag::Issue("#123".to_string())
        ]
    );
    assert_eq!(display_name(&got.dir_name), "Login page");
}

/// Anything that is not all digits is a tracker key, upper-cased the way its tracker writes it.
#[test]
fn a_key_shaped_ticket_is_upper_cased() {
    for (ticket, expected) in [("abc-123", "ABC-123"), ("gh-42", "GH-42"), ("x1", "X1")] {
        let dir_name = format!("feat-{ticket}_login");
        assert_eq!(
            parse_tags(&dir_name),
            vec![
                Tag::Type(ConventionalType::Feat),
                Tag::Issue(expected.to_string())
            ],
            "{dir_name}"
        );
    }
}

/// The boundary is what marks the ticket, not the type — a directory name that never got a
/// recognised type still says where its ticket ends.
#[test]
fn an_untyped_name_can_still_carry_a_ticket() {
    assert_eq!(
        parse_tags("abc-123_login-page"),
        vec![Tag::Issue("ABC-123".to_string())]
    );
    assert_eq!(display_name("abc-123_login-page"), "Login page");
}

/// A boundary with nothing before it is not a ticket. Hand-made directories are the only way to
/// reach this, and an empty chip is worse than no chip.
#[test]
fn a_boundary_with_nothing_before_it_yields_no_ticket() {
    assert_eq!(
        parse_tags("feat_login-page"),
        vec![Tag::Type(ConventionalType::Feat)]
    );
    assert_eq!(display_name("feat_login-page"), "Login page");
    assert!(parse_tags("_login").is_empty());
    assert_eq!(display_name("_login"), "Login");
}

/// Only the FIRST boundary divides; a second one is just a separator inside the name, because a
/// derived name has at most one and a hand-made one must still render.
#[test]
fn a_second_boundary_is_part_of_the_name() {
    assert_eq!(
        parse_tags("feat-abc-1_a_b"),
        vec![
            Tag::Type(ConventionalType::Feat),
            Tag::Issue("ABC-1".to_string())
        ]
    );
    assert_eq!(display_name("feat-abc-1_a_b"), "A b");
}
