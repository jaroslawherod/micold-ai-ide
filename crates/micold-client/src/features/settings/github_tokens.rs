//! The Settings *GitHub tokens* section (#1002, #1529, contracts/pat-source.md §3, FR-005, FR-007,
//! FR-008, FR-009): the fine-grained tokens the user pasted, each with a name and an optional
//! repository filter, and the form that adds one. One token may reach any number of repositories.
//!
//! The tokens are not settings: they live in the device's `PatStore` and never in the settings
//! document, so Save and Cancel do not touch them. Add, Replace, Remove and Move act at once. This
//! is the pure half: the form, the rows in file order, the reach and the per-repository choice, the
//! wording. Checking a token (a network call) and storing it are the shell's: the reducer hands it
//! a [`PatRequest`] and takes the answer back as [`GithubTokens::checked`], then says whether the
//! credential wiring must [`refresh`](GithubTokens::take_refresh). The token's text is held only in
//! a [`TokenInput`], whose `Debug` redacts, so no state, message or log line shows it.

use micold_core::daemon_config::RepoPattern;
use micold_core::local_time::utc_date;
use micold_core::scoped_credential::pat_store::{valid_name, MAX_NAME_LEN};
use micold_core::scoped_credential::token_choice::{uses, Filter, RepoUse};
use micold_core::scoped_credential::{pat, UnixTime, EXPIRY_WARNING_SECS};
use micold_core::typeahead::Direction;

use super::TokenInput;

/// One stored token as the list shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatRow {
    /// Its name (lower case, as stored).
    pub name: String,
    /// The repositories it is narrowed to; empty = every repository the token reaches.
    pub repos: Vec<RepoPattern>,
    /// When GitHub said it expires, at the last check.
    pub expires_at: UnixTime,
    /// The private repositories GitHub listed for it when it was last pasted in this session;
    /// `None` when it was not checked since Settings opened.
    pub reach: Option<Vec<String>>,
}

impl PatRow {
    /// The expiry as a UTC date, `2026-11-20`.
    pub fn expiry_date(&self) -> String {
        utc_date(self.expires_at)
    }

    /// The row's text: the name and when its token expires. Never the token.
    pub fn label(&self) -> String {
        format!("{} - expires on {}", self.name, self.expiry_date())
    }

    /// Which repositories the token reaches: its filter, or every repository it can access.
    pub fn reach_text(&self) -> String {
        if self.repos.is_empty() {
            return match &self.reach {
                Some(listed) if !listed.is_empty() => format!(
                    "Reaches every repository the token can access, including {}",
                    listed.join(", ")
                ),
                _ => "Reaches every repository the token can access".to_string(),
            };
        }
        let list: Vec<String> = self.repos.iter().map(ToString::to_string).collect();
        format!("Reaches {}", list.join(", "))
    }

    /// The warning shown from [`EXPIRY_WARNING_SECS`] before the expiry (FR-008), `None` before.
    pub fn warning(&self, now: UnixTime) -> Option<String> {
        if self.expires_at > now + EXPIRY_WARNING_SECS {
            return None;
        }
        let verb = if self.expires_at > now {
            "expires"
        } else {
            "expired"
        };
        Some(format!(
            "GitHub token {} {verb} on {}: paste a new token in Settings",
            self.name,
            self.expiry_date()
        ))
    }
}

/// What the shell learned about a token it checked and stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checked {
    /// The expiry GitHub reported.
    pub expires_at: UnixTime,
    /// The private repositories GitHub listed for it.
    pub reach: Vec<String>,
}

/// A token waiting to be checked: the shell checks it and answers with [`GithubTokens::checked`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatRequest {
    /// The name it is stored under.
    pub name: String,
    /// The repositories it is narrowed to; empty = none.
    pub repos: Vec<RepoPattern>,
    /// The token, as pasted and trimmed.
    pub token: TokenInput,
}

/// The section's state: the rows and the add form.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GithubTokens {
    /// The stored tokens, in file order: the first of two equally specific tokens wins.
    pub rows: Vec<PatRow>,
    /// **Name**, as typed.
    pub name: String,
    /// **Repositories**, as typed: `owner/repo` or `owner/*`, separated by commas.
    pub repos: String,
    /// **Token**, as typed; cleared after a save.
    pub token: TokenInput,
    /// Why the last Add was refused, shown at the form.
    pub error: Option<String>,
    /// A check is running.
    pub checking: bool,
    request: Option<PatRequest>,
    refresh: bool,
}

/// The repository filter a field value names: `owner/repo` or `owner/*` patterns separated by
/// commas, spaces or lines; empty text is no filter. A pattern that is neither is an error naming it.
pub fn parse_repos(text: &str) -> Result<Vec<RepoPattern>, String> {
    let mut out: Vec<RepoPattern> = vec![];
    for word in text.split(|c: char| c == ',' || c.is_whitespace()) {
        if word.is_empty() {
            continue;
        }
        let pattern = RepoPattern::parse(word)
            .ok_or_else(|| format!("\"{word}\" is not owner/repo or owner/*."))?;
        if !out.contains(&pattern) {
            out.push(pattern);
        }
    }
    Ok(out)
}

impl GithubTokens {
    /// Replace the rows with what the store holds, in its order.
    pub fn load(&mut self, stored: impl IntoIterator<Item = (String, Vec<RepoPattern>, UnixTime)>) {
        self.rows = stored
            .into_iter()
            .map(|(name, repos, expires_at)| PatRow {
                name,
                repos,
                expires_at,
                reach: None,
            })
            .collect();
    }

    /// The tokens' filters, for the choice.
    fn filters(&self) -> Vec<Filter<'_>> {
        self.rows
            .iter()
            .map(|r| Filter {
                name: &r.name,
                repos: &r.repos,
            })
            .collect()
    }

    /// Per repository the filters name: the token used and those shadowed (FR-009).
    pub fn uses(&self) -> Vec<RepoUse> {
        uses(&self.filters())
    }

    /// The name field was edited.
    pub fn name_edited(&mut self, text: String) {
        self.name = text;
        self.error = None;
    }

    /// The repositories field was edited.
    pub fn repos_edited(&mut self, text: String) {
        self.repos = text;
        self.error = None;
    }

    /// The token field was edited.
    pub fn token_edited(&mut self, text: TokenInput) {
        self.token = text;
        self.error = None;
    }

    /// **Add** was pressed. A name that cannot be a file name, a filter that is not a list of
    /// `owner/repo` or `owner/*`, an empty token or one that is not a fine-grained token is refused
    /// here, at the form; otherwise a [`PatRequest`] is raised for the shell to check.
    pub fn add_requested(&mut self) {
        if self.checking {
            return;
        }
        let name = self.name.trim().to_ascii_lowercase();
        if !valid_name(&name) {
            self.error = Some(format!(
                "Name the token with letters, digits, - _ or . (at most {MAX_NAME_LEN})."
            ));
            return;
        }
        let repos = match parse_repos(&self.repos) {
            Ok(repos) => repos,
            Err(reason) => {
                self.error = Some(reason);
                return;
            }
        };
        let token = pat::normalize(self.token.as_str());
        if token.expose().is_empty() {
            self.error = Some("Paste the fine-grained token.".to_string());
            return;
        }
        if let Err(reason) = pat::check_shape(&token) {
            self.error = Some(reason.message());
            return;
        }
        self.error = None;
        self.checking = true;
        self.request = Some(PatRequest {
            name,
            repos,
            token: TokenInput::new(token.expose().to_string()),
        });
    }

    /// The request [`GithubTokens::add_requested`] raised, once.
    pub fn take_request(&mut self) -> Option<PatRequest> {
        self.request.take()
    }

    /// The check of the token `name` finished: with what GitHub reported, the shell has stored it
    /// (the row is added, or replaced in its place; the form cleared; the wiring asked to
    /// refresh); with a reason, nothing was stored and the reason is shown at the form.
    pub fn checked(
        &mut self,
        name: String,
        repos: Vec<RepoPattern>,
        result: Result<Checked, String>,
    ) {
        self.checking = false;
        match result {
            Ok(checked) => {
                let row = PatRow {
                    name: name.clone(),
                    repos,
                    expires_at: checked.expires_at,
                    reach: Some(checked.reach),
                };
                match self.rows.iter_mut().find(|r| r.name == name) {
                    Some(existing) => *existing = row,
                    None => self.rows.push(row),
                }
                self.token = TokenInput::default();
                self.name.clear();
                self.repos.clear();
                self.error = None;
                self.refresh = true;
            }
            Err(reason) => self.error = Some(reason),
        }
    }

    /// **Replace** on a row: the form is aimed at that token, ready for a new one.
    pub fn replace_requested(&mut self, name: &str) {
        let Some(row) = self.rows.iter().find(|r| r.name == name) else {
            return;
        };
        self.name = row.name.clone();
        self.repos = row
            .repos
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        self.token = TokenInput::default();
        self.error = None;
    }

    /// **Remove** on a row, after the shell deleted the stored token.
    pub fn removed(&mut self, name: &str) {
        self.rows.retain(|r| r.name != name);
        self.refresh = true;
    }

    /// A row's **Move up** or **Move down**: the order decides the winner among equally specific
    /// tokens. The shell keeps the new order ([`GithubTokens::names`]).
    pub fn moved(&mut self, index: usize, direction: Direction) {
        let to = match direction {
            Direction::Prev => index.checked_sub(1),
            Direction::Next => Some(index + 1).filter(|i| *i < self.rows.len()),
        };
        if let Some(to) = to.filter(|_| index < self.rows.len()) {
            self.rows.swap(index, to);
            self.refresh = true;
        }
    }

    /// The tokens' names in the order shown.
    pub fn names(&self) -> Vec<String> {
        self.rows.iter().map(|r| r.name.clone()).collect()
    }

    /// Whether a token was added, replaced, removed or moved since the last call: the credential
    /// wiring checks every repository again for every daemon.
    pub fn take_refresh(&mut self) -> bool {
        std::mem::take(&mut self.refresh)
    }
}

/// One line of the per-repository view: `acme/*: uses work; shadowed: other` (FR-009).
pub fn use_text(line: &RepoUse) -> String {
    let who = if line.pattern == "*" {
        "tokens with no filter".to_string()
    } else {
        line.pattern.clone()
    };
    if line.shadowed.is_empty() {
        format!("{who}: uses {}", line.used)
    } else {
        format!(
            "{who}: uses {}; shadowed: {}",
            line.used,
            line.shadowed.join(", ")
        )
    }
}

/// The help at the point of pasting (FR-005, FR-007, FR-008, contract §3).
pub fn help() -> Vec<&'static str> {
    vec![
        "Create a fine-grained personal access token on GitHub with the permissions Contents: Read \
         and write and Metadata: Read, and give it a short expiry. One token can reach as many \
         repositories as you choose on GitHub.",
        "Leave Repositories empty to use the token for every repository it reaches, or limit it to \
         owner/repo or owner/* entries separated by commas.",
        "A repository uses the most specific token: owner/repo, then owner/*, then no filter. Of \
         two equally specific tokens the first in the list wins; move one up to prefer it.",
        "The sandbox can use everything the token can, and the client cannot narrow it.",
        "A token that leaks stays valid until it expires or you delete it on GitHub. Remove \
         deletes it here only; it does not delete it on GitHub.",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_filter_is_a_list_of_owner_repo_or_owner_star() {
        assert_eq!(parse_repos(""), Ok(vec![]));
        assert_eq!(
            parse_repos("Acme/App, acme/*  acme/app"),
            Ok(vec![
                RepoPattern::parse("acme/app").unwrap(),
                RepoPattern::parse("acme/*").unwrap()
            ])
        );
        for bad in ["o", "o/", "*/r", "o/r/x", "o/r*", "https://github.com/o/r"] {
            assert!(parse_repos(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_warning_starts_exactly_a_day_before_the_expiry() {
        let row = PatRow {
            name: "work".into(),
            repos: vec![],
            expires_at: 1_800_000_000,
            reach: None,
        };
        assert_eq!(row.warning(row.expires_at - EXPIRY_WARNING_SECS - 1), None);
        let warned = row.warning(row.expires_at - EXPIRY_WARNING_SECS).unwrap();
        assert!(warned.contains(" expires on "), "{warned}");
        let last = row.warning(row.expires_at - 1).unwrap();
        assert!(last.contains(" expires on "), "{last}");
        let expired = row.warning(row.expires_at).unwrap();
        assert!(expired.contains(" expired on "), "{expired}");
    }
}
