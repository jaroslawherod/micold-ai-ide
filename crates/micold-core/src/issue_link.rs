//! The link from a worktree to its issue (feature 1122): which issue a worktree directory's name
//! names, and the label and browser address of the link its Overview card shows.
//!
//! Pure: no I/O and no logging. The address is built here and handed to the browser by the
//! caller; it is never logged, and [`IssueLink`]'s `Debug` prints the label only (FR-014). Every
//! rule is unit-tested here without the GUI (FR-016).

use crate::github::GithubRepo;
use crate::jira::JiraBaseUrl;

/// The ticket of a worktree directory's name, classified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueRef {
    /// An all-digit ticket: a GitHub issue number.
    Github(u64),
    /// A Jira key, written upper case.
    Jira(String),
}

/// What decides, besides the name, whether a card has an issue link (FR-001).
#[derive(Debug, Clone, Copy, Default)]
pub struct IssueSources<'a> {
    /// The project's GitHub repository, when it has one.
    pub github: Option<&'a GithubRepo>,
    /// Whether the GitHub issues switch (feature 1102) is on.
    pub github_on: bool,
    /// The Jira base URL, when it is set.
    pub jira_base: Option<&'a JiraBaseUrl>,
    /// Whether the Jira integration switch (feature 533) is on.
    pub jira_on: bool,
}

/// An issue link: the text the card shows and the address the browser is asked to open.
#[derive(Clone, PartialEq, Eq)]
pub struct IssueLink {
    /// "Issue #N", or a Jira ticket's bare key "KEY" (FR-002, BUG-1298 FR-024).
    pub label: String,
    /// The issue's page (FR-004). Never logged.
    pub address: String,
}

impl std::fmt::Debug for IssueLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IssueLink")
            .field("label", &self.label)
            .finish_non_exhaustive()
    }
}

/// The issue a worktree directory's name names: its ticket (`naming::ticket`) when it is all
/// digits (GitHub) or a Jira key; `None` otherwise.
pub fn issue_ref(dir_name: &str) -> Option<IssueRef> {
    let ticket = crate::naming::ticket(dir_name)?;
    if ticket.chars().all(|c| c.is_ascii_digit()) {
        return ticket.parse().ok().map(IssueRef::Github);
    }
    is_jira_key(&ticket).then(|| IssueRef::Jira(ticket.to_ascii_uppercase()))
}

/// The Jira key form, ignoring case: a letter, then letters, digits or `_`, then `-` and a
/// decimal number.
fn is_jira_key(ticket: &str) -> bool {
    let Some((key, number)) = ticket.split_once('-') else {
        return false;
    };
    key.starts_with(|c: char| c.is_ascii_alphabetic())
        && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !number.is_empty()
        && number.chars().all(|c| c.is_ascii_digit())
}

/// The link a card for the worktree directory `dir_name` shows, if any (FR-001, FR-002, FR-004).
pub fn issue_link(dir_name: &str, sources: &IssueSources<'_>) -> Option<IssueLink> {
    match issue_ref(dir_name)? {
        IssueRef::Github(number) => {
            let repo = sources.github.filter(|_| sources.github_on)?;
            Some(IssueLink {
                label: format!("Issue #{number}"),
                address: format!(
                    "https://github.com/{}/{}/issues/{number}",
                    repo.owner(),
                    repo.name()
                ),
            })
        }
        IssueRef::Jira(key) => {
            let base = sources.jira_base.filter(|_| sources.jira_on)?;
            Some(IssueLink {
                address: format!("{}/browse/{key}", base.as_str()),
                // The bare key (BUG-1298 FR-024): "MIC-87" reads as a ticket on its own.
                label: key.to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> GithubRepo {
        GithubRepo::from_remote_url("https://github.com/o/r").expect("a GitHub remote")
    }

    fn base(text: &str) -> JiraBaseUrl {
        JiraBaseUrl::parse(text).expect("a valid base URL")
    }

    #[test]
    fn an_all_digit_ticket_is_a_github_issue() {
        assert_eq!(
            issue_ref("feat-1122_overview-issue-link"),
            Some(IssueRef::Github(1122))
        );
    }

    #[test]
    fn a_lower_case_jira_key_is_written_upper_case() {
        assert_eq!(
            issue_ref("feat-mic-87_add-login"),
            Some(IssueRef::Jira("MIC-87".into()))
        );
    }

    #[test]
    fn an_upper_case_jira_key_is_kept() {
        assert_eq!(
            issue_ref("fix-ABC-123_login"),
            Some(IssueRef::Jira("ABC-123".into()))
        );
    }

    #[test]
    fn a_jira_key_may_carry_digits_after_its_first_letter() {
        assert_eq!(
            issue_ref("feat-ab2-9_x"),
            Some(IssueRef::Jira("AB2-9".into()))
        );
    }

    #[test]
    fn a_name_without_a_ticket_names_no_issue() {
        for name in ["feat_login", "plain", "feat-login"] {
            assert_eq!(issue_ref(name), None, "{name}");
        }
    }

    #[test]
    fn a_ticket_neither_digits_nor_a_jira_key_names_no_issue() {
        for name in [
            "feat-abc_x",
            "feat-87-mic_x",
            "feat-mic-_x",
            "feat-mic-8a_x",
        ] {
            assert_eq!(issue_ref(name), None, "{name}");
        }
    }

    #[test]
    fn an_all_digit_ticket_overflowing_u64_names_no_issue() {
        assert_eq!(issue_ref("feat-99999999999999999999_x"), None);
    }

    #[test]
    fn a_github_ticket_links_to_the_repositorys_issue_while_the_switch_is_on() {
        let repo = repo();
        let sources = IssueSources {
            github: Some(&repo),
            github_on: true,
            ..IssueSources::default()
        };
        assert_eq!(
            issue_link("feat-1122_overview-issue-link", &sources),
            Some(IssueLink {
                label: "Issue #1122".into(),
                address: "https://github.com/o/r/issues/1122".into(),
            })
        );
    }

    #[test]
    fn a_github_ticket_has_no_link_without_a_repository_or_with_the_switch_off() {
        let repo = repo();
        let jira = base("https://example.atlassian.net");
        for sources in [
            IssueSources {
                github: None,
                github_on: true,
                jira_base: Some(&jira),
                jira_on: true,
            },
            IssueSources {
                github: Some(&repo),
                github_on: false,
                jira_base: Some(&jira),
                jira_on: true,
            },
        ] {
            assert_eq!(issue_link("feat-1122_x", &sources), None, "{sources:?}");
        }
    }

    #[test]
    fn a_jira_key_links_to_its_browse_page_while_the_switch_is_on() {
        let jira = base("https://example.atlassian.net");
        let sources = IssueSources {
            jira_base: Some(&jira),
            jira_on: true,
            ..IssueSources::default()
        };
        assert_eq!(
            issue_link("feat-mic-87_add-login", &sources),
            Some(IssueLink {
                label: "MIC-87".into(),
                address: "https://example.atlassian.net/browse/MIC-87".into(),
            })
        );
    }

    #[test]
    fn a_jira_key_has_no_link_without_a_base_url_or_with_the_switch_off() {
        let repo = repo();
        let jira = base("https://example.atlassian.net");
        for sources in [
            IssueSources {
                github: Some(&repo),
                github_on: true,
                jira_base: None,
                jira_on: true,
            },
            IssueSources {
                github: Some(&repo),
                github_on: true,
                jira_base: Some(&jira),
                jira_on: false,
            },
        ] {
            assert_eq!(issue_link("feat-mic-87_x", &sources), None, "{sources:?}");
        }
    }

    #[test]
    fn a_link_never_borrows_the_other_trackers_source() {
        let repo = repo();
        let jira = base("https://example.atlassian.net");
        let github_only = IssueSources {
            github: Some(&repo),
            github_on: true,
            ..IssueSources::default()
        };
        let jira_only = IssueSources {
            jira_base: Some(&jira),
            jira_on: true,
            ..IssueSources::default()
        };
        assert_eq!(issue_link("feat-mic-87_x", &github_only), None);
        assert_eq!(issue_link("feat-1122_x", &jira_only), None);
    }

    #[test]
    fn a_links_debug_does_not_show_its_address() {
        let repo = repo();
        let sources = IssueSources {
            github: Some(&repo),
            github_on: true,
            ..IssueSources::default()
        };
        let link = issue_link("feat-1122_x", &sources).expect("a link");
        let debug = format!("{link:?}");
        assert!(debug.contains("Issue #1122"), "{debug}");
        assert!(!debug.contains(&link.address), "{debug}");
        assert!(!debug.contains("github.com"), "{debug}");
    }

    #[test]
    fn a_jira_address_has_exactly_one_slash_before_browse() {
        for (saved, address) in [
            (
                "https://example.atlassian.net/",
                "https://example.atlassian.net/browse/MIC-87",
            ),
            (
                "https://jira.example.com/jira",
                "https://jira.example.com/jira/browse/MIC-87",
            ),
        ] {
            let jira = base(saved);
            let sources = IssueSources {
                jira_base: Some(&jira),
                jira_on: true,
                ..IssueSources::default()
            };
            let link = issue_link("feat-mic-87_x", &sources).expect("a link");
            assert_eq!(link.address, address, "{saved}");
        }
    }
}
