//! `TokenChoice`: which `[[github.tokens]]` entry a repository uses (#1530, FR-008, data-model
//! *TokenChoice*). Pure: the caller reads the token files and the reach cache.
//!
//! Candidates are the tokens that are not expired and whose filter names the repository; a token
//! with no filter is a candidate only when its reach (what GitHub reports it can see) holds the
//! repository. Rank: exact `owner/repo` over `owner/*` over no filter; equally specific tokens (the
//! same pattern, or both unfiltered) go to the first in the file. The others that reach the
//! repository are reported as shadowed.

use crate::github::GithubRepo;
use crate::scoped_credential::token_choice::specificity;
use crate::scoped_credential::token_choice::Filter;

use super::RepoPattern;

/// One token as the choice sees it.
#[derive(Debug, Clone, Copy)]
pub struct Candidate<'a> {
    pub name: &'a str,
    /// The `repos` filter; empty = unfiltered.
    pub repos: &'a [RepoPattern],
    /// Past its `expires_at`.
    pub expired: bool,
    /// The repositories (`owner/repo`, lower case) GitHub says the token reaches, from the cache;
    /// `None` = not known (offline with no cache).
    pub reach: Option<&'a [String]>,
}

/// The outcome for one repository.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Chosen {
    /// The index of the token used.
    pub used: Option<usize>,
    /// The indexes of the other candidates, in rank order.
    pub shadowed: Vec<usize>,
}

/// The token `repo` uses among `tokens` (in file order).
pub fn choose(tokens: &[Candidate<'_>], repo: &GithubRepo) -> Chosen {
    let key = format!(
        "{}/{}",
        repo.owner().to_ascii_lowercase(),
        repo.name().to_ascii_lowercase()
    );
    let mut ranked: Vec<(usize, u8)> = tokens
        .iter()
        .enumerate()
        .filter(|(_, t)| !t.expired)
        .filter(|(_, t)| !t.repos.is_empty() || reaches(t, &key))
        .filter_map(|(i, t)| {
            let filter = Filter {
                name: t.name,
                repos: t.repos,
            };
            specificity(&filter, repo).map(|rank| (i, rank))
        })
        .collect();
    // Stable: equal specificity keeps file order.
    ranked.sort_by_key(|&(_, rank)| std::cmp::Reverse(rank));
    let mut order = ranked.into_iter().map(|(i, _)| i);
    Chosen {
        used: order.next(),
        shadowed: order.collect(),
    }
}

fn reaches(token: &Candidate<'_>, key: &str) -> bool {
    token
        .reach
        .is_some_and(|reach| reach.iter().any(|r| r.eq_ignore_ascii_case(key)))
}

/// A token file's content: the token on the first line, optionally an `expires_at=<unix>` line
/// (the stored-PAT shape, `token=<token>`, is read too). `None` when there is no token.
pub fn parse_token_file(text: &str) -> Option<(String, Option<u64>)> {
    let mut token = None;
    let mut expires_at = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("expires_at=") {
            expires_at = value.trim().parse().ok();
        } else if let Some(value) = line.strip_prefix("token=") {
            token = Some(value.trim().to_string());
        } else if token.is_none() && !line.starts_with("repos=") && !line.is_empty() {
            token = Some(line.to_string());
        }
    }
    token
        .filter(|t| !t.is_empty())
        .map(|token| (token, expires_at))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(text: &str) -> RepoPattern {
        RepoPattern::parse(text).unwrap()
    }

    fn repo(text: &str) -> GithubRepo {
        GithubRepo::from_remote_url(&format!("https://github.com/{text}")).unwrap()
    }

    fn reach(repos: &[&str]) -> Vec<String> {
        repos.iter().map(ToString::to_string).collect()
    }

    fn token<'a>(
        name: &'a str,
        repos: &'a [RepoPattern],
        reach: Option<&'a [String]>,
    ) -> Candidate<'a> {
        Candidate {
            name,
            repos,
            expired: false,
            reach,
        }
    }

    #[test]
    fn exact_beats_owner_beats_unfiltered() {
        let (exact, owner) = ([p("acme/app")], [p("acme/*")]);
        let seen = reach(&["acme/app"]);
        let tokens = [
            token("any", &[], Some(&seen)),
            token("owner", &owner, None),
            token("exact", &exact, None),
        ];
        let chosen = choose(&tokens, &repo("acme/app"));
        assert_eq!(chosen.used, Some(2));
        assert_eq!(chosen.shadowed, vec![1, 0]);
    }

    #[test]
    fn equally_specific_tokens_go_to_the_first_in_the_file() {
        let (one, two) = ([p("acme/*")], [p("acme/*")]);
        let tokens = [token("a", &one, None), token("b", &two, None)];
        let chosen = choose(&tokens, &repo("acme/app"));
        assert_eq!((chosen.used, chosen.shadowed), (Some(0), vec![1]));

        let seen = reach(&["acme/app"]);
        let tokens = [token("a", &[], Some(&seen)), token("b", &[], Some(&seen))];
        assert_eq!(choose(&tokens, &repo("acme/app")).used, Some(0));
    }

    #[test]
    fn an_expired_token_is_skipped() {
        let (one, two) = ([p("acme/app")], [p("acme/*")]);
        let mut first = token("first", &one, None);
        first.expired = true;
        let tokens = [first, token("second", &two, None)];
        let chosen = choose(&tokens, &repo("acme/app"));
        assert_eq!((chosen.used, chosen.shadowed), (Some(1), vec![]));
    }

    #[test]
    fn an_unfiltered_token_needs_the_repository_in_its_reach() {
        let seen = reach(&["acme/app"]);
        let tokens = [token("any", &[], Some(&seen))];
        assert_eq!(choose(&tokens, &repo("Acme/App")).used, Some(0));
        assert_eq!(choose(&tokens, &repo("acme/other")).used, None);
    }

    #[test]
    fn with_no_cached_reach_only_a_filtered_token_is_used_and_only_for_its_filter() {
        let filter = [p("acme/app")];
        let tokens = [token("any", &[], None), token("filtered", &filter, None)];
        assert_eq!(choose(&tokens, &repo("acme/app")).used, Some(1));
        assert_eq!(choose(&tokens, &repo("acme/other")).used, None);
    }

    #[test]
    fn a_token_file_is_the_token_then_an_optional_expiry() {
        assert_eq!(
            parse_token_file("ghp_x\nexpires_at=1700\n"),
            Some(("ghp_x".to_string(), Some(1700)))
        );
        assert_eq!(parse_token_file("ghp_x"), Some(("ghp_x".to_string(), None)));
        assert_eq!(
            parse_token_file("expires_at=5\nrepos=a/b\ntoken=ghp_y\n"),
            Some(("ghp_y".to_string(), Some(5)))
        );
        assert_eq!(parse_token_file("\n"), None);
    }
}
