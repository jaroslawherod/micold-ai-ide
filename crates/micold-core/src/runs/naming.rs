//! A group's run names (feature 483, research R5): the group name with each run's number, so the
//! runs' branches and folders are visibly related and distinct (FR-003).

use crate::naming::{DerivedNames, NamingError, WorktreeNaming};

/// The names of runs `1..=count`, in order: the group's naming with `-<number>` appended to the
/// name part, through [`crate::naming::derive`] so every rule of a single worktree's name holds.
pub fn derive_group(naming: &WorktreeNaming, count: u8) -> Result<Vec<DerivedNames>, NamingError> {
    // The group's own name first, so an invalid one is refused with the single worktree's error.
    crate::naming::derive(naming)?;
    (1..=count)
        .map(|number| {
            crate::naming::derive(&WorktreeNaming {
                name: format!("{} {number}", naming.name.trim()),
                ..naming.clone()
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::naming::{derive, ConventionalType};

    fn naming(name: &str, ticket: Option<&str>) -> WorktreeNaming {
        WorktreeNaming {
            type_: Some(ConventionalType::Feat),
            ticket: ticket.map(str::to_owned),
            name: name.into(),
        }
    }

    #[test]
    fn three_runs_get_numbered_branches_and_folders_in_order() {
        let names = derive_group(&naming("login page", None), 3).expect("a valid naming");
        let branches: Vec<_> = names.iter().map(|n| n.branch.as_str()).collect();
        let dirs: Vec<_> = names.iter().map(|n| n.dir_name.as_str()).collect();
        assert_eq!(
            branches,
            [
                "feat/login-page-1",
                "feat/login-page-2",
                "feat/login-page-3"
            ]
        );
        assert_eq!(
            dirs,
            [
                "feat-login-page-1",
                "feat-login-page-2",
                "feat-login-page-3"
            ]
        );
    }

    #[test]
    fn a_ticket_keeps_its_boundary() {
        let names = derive_group(&naming("login page", Some("ABC-12")), 2).expect("valid");
        assert_eq!(names[0].branch, "feat/abc-12_login-page-1");
        assert_eq!(names[1].dir_name, "feat-abc-12_login-page-2");
    }

    #[test]
    fn an_empty_or_unslugifiable_name_is_the_single_worktree_error() {
        for name in ["", "   ", "!!!"] {
            let input = naming(name, None);
            assert_eq!(
                derive_group(&input, 2),
                Err(derive(&input).expect_err("not a name")),
                "{name:?} is refused as a single worktree's name is"
            );
        }
        let no_type = WorktreeNaming {
            type_: None,
            ..naming("login", None)
        };
        assert_eq!(derive_group(&no_type, 2), Err(NamingError::NoType));
    }

    #[test]
    fn names_differing_only_in_case_derive_the_same_lowercase_names() {
        let upper = derive_group(&naming("Login Page", None), 2).expect("valid");
        let lower = derive_group(&naming("login page", None), 2).expect("valid");
        assert_eq!(upper, lower, "a case-only difference is the same name");
        assert!(upper.iter().all(
            |n| n.branch == n.branch.to_lowercase() && n.dir_name == n.dir_name.to_lowercase()
        ));
    }

    #[test]
    fn every_name_is_a_valid_ref_and_a_portable_folder() {
        const RESERVED: &[&str] = &["con", "prn", "aux", "nul", "com1", "lpt1"];
        for raw in [
            "con",
            "a.lock",
            "x..y",
            "über straße",
            "@{1}",
            "name~1^2:3",
            "nul",
        ] {
            let names = derive_group(&naming(raw, Some("T 1")), 8).expect("slugs to something");
            assert_eq!(names.len(), 8);
            for n in names {
                assert!(
                    crate::naming::is_valid_branch(&n.branch),
                    "{} is a valid ref",
                    n.branch
                );
                assert!(
                    n.dir_name.chars().all(|c| c.is_ascii_lowercase()
                        || c.is_ascii_digit()
                        || c == '-'
                        || c == '_'),
                    "{} holds only portable characters",
                    n.dir_name
                );
                assert!(!RESERVED.contains(&n.dir_name.as_str()));
                assert!(!n.dir_name.ends_with('.') && !n.dir_name.ends_with(' '));
            }
        }
    }
}
