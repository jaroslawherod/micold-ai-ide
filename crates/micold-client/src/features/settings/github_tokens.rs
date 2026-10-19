//! The Settings *GitHub access tokens* section (#1002, contracts/pat-source.md §3, FR-005,
//! FR-008): the fine-grained tokens the user pasted, one per repository, and the form that adds one.
//!
//! The tokens are not settings: they live in the device's `PatStore` and never in the settings
//! document, so Save and Cancel do not touch them. Add, Replace and Remove act at once. This is
//! the pure half: the form, the rows, the wording. Checking a token (a network call) and storing
//! it are the shell's: the reducer hands it a [`PatRequest`] and takes the answer back as
//! [`GithubTokens::checked`], then lists the repositories the credential wiring must
//! [`refresh`](GithubTokens::take_refresh). The token's text is held only in a [`TokenInput`], whose
//! `Debug` redacts, so no state, message or log line shows it.

use micold_core::github::GithubRepo;
use micold_core::local_time::rfc3339_utc;
use micold_core::scoped_credential::{pat, UnixTime, EXPIRY_WARNING_SECS};

use super::TokenInput;

/// One stored token as the list shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatRow {
    /// The repository it is for (lower case, as stored).
    pub repo: GithubRepo,
    /// When GitHub said it expires, at the last check.
    pub expires_at: UnixTime,
}

impl PatRow {
    /// The expiry as a UTC date, `2026-11-20`.
    pub fn expiry_date(&self) -> String {
        let secs = i64::try_from(self.expires_at).unwrap_or(i64::MAX);
        rfc3339_utc(secs).chars().take(10).collect()
    }

    /// The row's text: the repository and when its token expires. Never the token.
    pub fn label(&self) -> String {
        format!("{} - expires on {}", self.repo, self.expiry_date())
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
            "Git credential for {} {verb} on {}: paste a new token in Settings",
            self.repo,
            self.expiry_date()
        ))
    }
}

/// A token waiting to be checked: the shell checks it and answers with [`GithubTokens::checked`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatRequest {
    /// The repository it is for.
    pub repo: GithubRepo,
    /// The token, as pasted and trimmed.
    pub token: TokenInput,
}

/// The section's state: the rows and the add form.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GithubTokens {
    /// The stored tokens, sorted by repository.
    pub rows: Vec<PatRow>,
    /// **Repository**, as typed.
    pub repo: String,
    /// **Token**, as typed; cleared after a save.
    pub token: TokenInput,
    /// Why the last Add was refused, shown at the form.
    pub error: Option<String>,
    /// A check is running.
    pub checking: bool,
    request: Option<PatRequest>,
    refresh: Vec<GithubRepo>,
}

/// The repository a field value names: `owner/name` or a github.com URL; anything else is `None`.
pub fn parse_repo(text: &str) -> Option<GithubRepo> {
    let text = text.trim();
    if text.contains("://") {
        return GithubRepo::from_remote_url(text);
    }
    if text.contains(':') || text.contains('@') {
        return None;
    }
    GithubRepo::from_remote_url(&format!("https://github.com/{text}"))
}

impl GithubTokens {
    /// Replace the rows with what the store holds.
    pub fn load(&mut self, stored: impl IntoIterator<Item = (GithubRepo, UnixTime)>) {
        self.rows = stored
            .into_iter()
            .map(|(repo, expires_at)| PatRow {
                repo: repo.lowercased(),
                expires_at,
            })
            .collect();
        self.rows.sort_by(|a, b| a.repo.cmp(&b.repo));
    }

    /// The repository field was edited.
    pub fn repo_edited(&mut self, text: String) {
        self.repo = text;
        self.error = None;
    }

    /// The token field was edited.
    pub fn token_edited(&mut self, text: TokenInput) {
        self.token = text;
        self.error = None;
    }

    /// **Add** was pressed. A repository that is neither `owner/name` nor a github.com URL, an
    /// empty token or one that is not a fine-grained token is refused here, at the form; otherwise
    /// a [`PatRequest`] is raised for the shell to check.
    pub fn add_requested(&mut self) {
        if self.checking {
            return;
        }
        let Some(repo) = parse_repo(&self.repo) else {
            self.error =
                Some("Enter the repository as owner/name or as a github.com URL.".to_string());
            return;
        };
        let token = pat::normalize(self.token.as_str());
        if token.expose().is_empty() {
            self.error = Some("Paste the fine-grained token for this repository.".to_string());
            return;
        }
        if let Err(reason) = pat::check_shape(&token) {
            self.error = Some(reason.message());
            return;
        }
        self.error = None;
        self.checking = true;
        self.request = Some(PatRequest {
            repo: repo.lowercased(),
            token: TokenInput::new(token.expose().to_string()),
        });
    }

    /// The request [`GithubTokens::add_requested`] raised, once.
    pub fn take_request(&mut self) -> Option<PatRequest> {
        self.request.take()
    }

    /// The check of `repo`'s token finished: with the expiry GitHub reported, the shell has stored
    /// it (the row is added or replaced, the field cleared, the wiring asked to refresh); with a
    /// reason, nothing was stored and the reason is shown at the form.
    pub fn checked(&mut self, repo: GithubRepo, result: Result<UnixTime, String>) {
        self.checking = false;
        match result {
            Ok(expires_at) => {
                let repo = repo.lowercased();
                self.rows.retain(|r| r.repo != repo);
                self.rows.push(PatRow {
                    repo: repo.clone(),
                    expires_at,
                });
                self.rows.sort_by(|a, b| a.repo.cmp(&b.repo));
                self.token = TokenInput::default();
                self.repo.clear();
                self.error = None;
                self.refresh.push(repo);
            }
            Err(reason) => self.error = Some(reason),
        }
    }

    /// **Replace** on a row: the form is aimed at that repository, ready for a new token.
    pub fn replace_requested(&mut self, repo: &GithubRepo) {
        self.repo = repo.to_string();
        self.token = TokenInput::default();
        self.error = None;
    }

    /// **Remove** on a row, after the shell deleted the stored token.
    pub fn removed(&mut self, repo: &GithubRepo) {
        let repo = repo.lowercased();
        self.rows.retain(|r| r.repo != repo);
        self.refresh.push(repo);
    }

    /// The repositories whose token was replaced or removed since the last call: the credential
    /// wiring checks them again for every daemon that uses them.
    pub fn take_refresh(&mut self) -> Vec<GithubRepo> {
        std::mem::take(&mut self.refresh)
    }
}

/// The help at the point of pasting (FR-005, FR-008, contract §3).
pub fn help(reach_check: bool) -> Vec<&'static str> {
    let mut lines = vec![
        "Create a fine-grained personal access token on GitHub for this one repository with the \
         permissions Contents: Read and write and Metadata: Read, and give it a short expiry.",
        "The sandbox can use everything the token can, and the client cannot narrow it.",
        "A token that leaks stays valid until it expires or you delete it on GitHub. Remove \
         deletes it here only; it does not delete it on GitHub.",
    ];
    if !reach_check {
        lines.push(
            "The client cannot verify which other repositories the token reaches: check on \
             GitHub that it reaches only this one.",
        );
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_repository_is_owner_slash_name_or_a_github_url() {
        for ok in [
            "o/r",
            " O/R ",
            "https://github.com/o/r",
            "https://github.com/o/r.git",
            "o/r.git",
        ] {
            assert_eq!(
                parse_repo(ok).map(|r| r.to_string().to_lowercase()),
                Some("o/r".into()),
                "{ok}"
            );
        }
        for bad in [
            "",
            "o",
            "o/",
            "/r",
            "o/r/x",
            "https://gitlab.com/o/r",
            "../r",
            "o r/x",
            "git@github.com:o/r",
        ] {
            assert_eq!(parse_repo(bad), None, "{bad}");
        }
    }
}
