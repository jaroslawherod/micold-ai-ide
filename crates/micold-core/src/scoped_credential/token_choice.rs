//! Which of several GitHub tokens a repository uses (#1529, FR-007, FR-009, contracts/daemon-config
//! `[[github.tokens]]`): an exact `owner/repo` filter beats `owner/*`, which beats no filter; among
//! equally specific tokens the first in the list wins, and the list's order is the order Settings
//! shows. Nothing is refused for an overlap; [`uses`] says, per repository, which token wins and
//! which are shadowed. Pure: no I/O, no token.

use crate::daemon_config::RepoPattern;
use crate::github::GithubRepo;

/// A token as the choice sees it: its name and its filter (empty = unfiltered).
#[derive(Debug, Clone, Copy)]
pub struct Filter<'a> {
    pub name: &'a str,
    pub repos: &'a [RepoPattern],
}

const EXACT: u8 = 3;
const OWNER: u8 = 2;
const UNFILTERED: u8 = 1;

/// How specifically `filter` names `repo`: `None` when it does not reach it.
pub(crate) fn specificity(filter: &Filter<'_>, repo: &GithubRepo) -> Option<u8> {
    if filter.repos.is_empty() {
        return Some(UNFILTERED);
    }
    let (owner, name) = (
        repo.owner().to_ascii_lowercase(),
        repo.name().to_ascii_lowercase(),
    );
    filter
        .repos
        .iter()
        .filter_map(|p| match p {
            RepoPattern::Exact(o, r) if *o == owner && *r == name => Some(EXACT),
            RepoPattern::Owner(o) if *o == owner => Some(OWNER),
            _ => None,
        })
        .max()
}

/// The ranked candidates for `repo`: the best specificity first, and in list order within it.
fn ranked(filters: &[Filter<'_>], rank: impl Fn(&Filter<'_>) -> Option<u8>) -> Vec<usize> {
    let mut found: Vec<(usize, u8)> = filters
        .iter()
        .enumerate()
        .filter_map(|(i, f)| rank(f).map(|r| (i, r)))
        .collect();
    // Stable: equal specificity keeps list order.
    found.sort_by_key(|&(_, r)| std::cmp::Reverse(r));
    found.into_iter().map(|(i, _)| i).collect()
}

/// The index of the token `repo` uses, or `None` when no token's filter reaches it.
pub fn choose(filters: &[Filter<'_>], repo: &GithubRepo) -> Option<usize> {
    ranked(filters, |f| specificity(f, repo)).first().copied()
}

/// One row of the per-repository view: what a filter pattern names, the token that wins it and the
/// tokens it shadows (in list order).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoUse {
    /// `owner/repo`, `owner/*`, or `*` for every repository of the tokens with no filter.
    pub pattern: String,
    /// The winning token's name.
    pub used: String,
    /// The names of the other tokens that reach it.
    pub shadowed: Vec<String>,
}

/// Per repository the tokens' filters name (and `*` where two tokens have none): the token used
/// and those shadowed, in the order the patterns first appear. A pattern only one token reaches has
/// no shadowed token.
pub fn uses(filters: &[Filter<'_>]) -> Vec<RepoUse> {
    let mut patterns: Vec<Option<&RepoPattern>> = vec![];
    for p in filters.iter().flat_map(|f| f.repos) {
        if !patterns.contains(&Some(p)) {
            patterns.push(Some(p));
        }
    }
    if filters.iter().filter(|f| f.repos.is_empty()).count() > 1 {
        patterns.push(None);
    }
    patterns
        .into_iter()
        .filter_map(|pattern| {
            let order = ranked(filters, |f| match pattern {
                // A filter reaches an exact repository by that pattern, by its owner's, or by none.
                Some(RepoPattern::Exact(o, r)) => {
                    GithubRepo::from_remote_url(&format!("https://github.com/{o}/{r}"))
                        .and_then(|repo| specificity(f, &repo))
                }
                // An owner's whole set is reached by its own pattern or by no filter.
                Some(p @ RepoPattern::Owner(_)) => f
                    .repos
                    .contains(p)
                    .then_some(OWNER)
                    .or(f.repos.is_empty().then_some(UNFILTERED)),
                None => f.repos.is_empty().then_some(UNFILTERED),
            });
            let (first, rest) = order.split_first()?;
            Some(RepoUse {
                pattern: pattern.map_or_else(|| "*".to_string(), ToString::to_string),
                used: filters[*first].name.to_string(),
                shadowed: rest.iter().map(|i| filters[*i].name.to_string()).collect(),
            })
        })
        .collect()
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

    fn run(tokens: &[(&str, &[&str])], target: &str) -> Option<String> {
        let filters = owned(tokens);
        let view: Vec<Filter<'_>> = filters
            .iter()
            .map(|(name, repos)| Filter { name, repos })
            .collect();
        choose(&view, &repo(target)).map(|i| view[i].name.to_string())
    }

    fn owned<'a>(tokens: &[(&'a str, &[&str])]) -> Vec<(&'a str, Vec<RepoPattern>)> {
        tokens
            .iter()
            .map(|(n, r)| (*n, r.iter().map(|x| p(x)).collect()))
            .collect()
    }

    #[test]
    fn exact_beats_owner_beats_unfiltered_and_the_first_wins_a_tie() {
        let tokens: &[(&str, &[&str])] = &[
            ("any", &[]),
            ("org", &["Acme/*"]),
            ("app", &["acme/app"]),
            ("org2", &["acme/*"]),
        ];
        assert_eq!(run(tokens, "acme/app").as_deref(), Some("app"));
        assert_eq!(run(tokens, "ACME/web").as_deref(), Some("org"));
        assert_eq!(run(tokens, "other/x").as_deref(), Some("any"));
        let no_any = &tokens[1..];
        assert_eq!(run(no_any, "other/x"), None);
    }

    #[test]
    fn reordering_two_equally_specific_tokens_changes_the_winner() {
        let ab: &[(&str, &[&str])] = &[("a", &["acme/*"]), ("b", &["acme/*"])];
        let ba: &[(&str, &[&str])] = &[("b", &["acme/*"]), ("a", &["acme/*"])];
        assert_eq!(run(ab, "acme/x").as_deref(), Some("a"));
        assert_eq!(run(ba, "acme/x").as_deref(), Some("b"));
    }

    #[test]
    fn uses_names_the_winner_and_the_shadowed_tokens_per_repository() {
        let owned = owned(&[
            ("first", &["acme/*"]),
            ("second", &["acme/*", "acme/app"]),
            ("all", &[]),
            ("all2", &[]),
        ]);
        let view: Vec<Filter<'_>> = owned
            .iter()
            .map(|(name, repos)| Filter { name, repos })
            .collect();
        let row = |pattern: &str, used: &str, shadowed: &[&str]| RepoUse {
            pattern: pattern.into(),
            used: used.into(),
            shadowed: shadowed.iter().map(ToString::to_string).collect(),
        };
        assert_eq!(
            uses(&view),
            [
                row("acme/*", "first", &["second", "all", "all2"]),
                row("acme/app", "second", &["first", "all", "all2"]),
                row("*", "all", &["all2"]),
            ]
        );
    }
}
