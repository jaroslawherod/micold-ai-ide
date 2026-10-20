//! The scoped git credential of a sandboxed daemon (#1002): what git inside the sandbox may use,
//! and what it may never reach.
//!
//! This part (FR-004) keeps the host's git configuration from carrying a credential into the
//! sandbox: [`gitconfig::sanitize_gitconfig`] strips a shared `~/.gitconfig`, and
//! [`git_env::git_env`] resets every credential helper that is left.
//!
//! The other part (FR-008–FR-010, FR-012) is what hands the sandbox a credential scoped to one
//! repository: [`ScopedCredential`] (only [`ScopedCredential::accept`] makes one), the
//! [`CredentialSource`] seam, [`NoCredentialReason`] and the pure [`schedule::Schedule`]. The one
//! real source is a fine-grained PAT the user pastes ([`pat`], [`pat_store`]): it is checked, never
//! renewed, and withdrawn at its expiry.

#[cfg(any(test, feature = "kube-fake"))]
pub mod fake;
pub mod git_env;
pub mod gitconfig;
pub mod helper;
pub mod pat;
pub mod pat_store;
pub mod schedule;

pub use git_env::{git_env, git_env_with_helper};
pub use gitconfig::{parse_config_list, sanitize_gitconfig, sanitize_gitconfig_text};

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::github::GithubRepo;

/// Seconds since the Unix epoch, as the credential files and the protocol carry them.
pub type UnixTime = u64;

/// The longest a credential a source mints itself may live ([`ScopedCredential::accept`]).
pub const MINTED_LIFETIME_SECS: u64 = 3600;

/// The sanity cap on a pasted PAT's expiry (FR-009, data-model §2): 366 days, since GitHub's
/// longest fine-grained expiry is a year. FR-009 requires a known expiry, not a short one. The
/// daemon's grant check shares this constant.
pub const MAX_LIFETIME_SECS: u64 = 366 * 24 * 3600;

/// How far past a lifetime limit an expiry may lie: the source issued it by its own clock.
pub const CLOCK_SKEW_SECS: u64 = 300;

/// The forge the scoped credential is for, as git names it in `credential.<base>.*`.
pub const GITHUB_BASE: &str = "https://github.com";

/// How long before it expires a minted credential is renewed ([`schedule::RenewPolicy::Renew`]).
pub const RENEW_MARGIN_SECS: u64 = 600;

/// How long before its expiry a held credential is warned about (FR-008): 24 hours.
pub const EXPIRY_WARNING_SECS: u64 = 86_400;

/// The wording of "no token is stored for this repository" (data-model §6).
pub const SOURCE_SETUP_HINT: &str = "No GitHub token is stored for this repository. Paste a fine-grained token for it in Settings › GitHub access tokens to push from sandboxes.";

/// A token that never prints: its `Debug` is redacted and [`SecretToken::expose`] is the one way
/// to read it, used only where a request or a file is written.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SecretToken(String);

impl SecretToken {
    /// Wrap a token.
    pub fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }

    /// The secret itself. Every caller of this is a place to check when auditing the token's reach.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretToken(<redacted>)")
    }
}

/// What a credential may do beyond the always-present `contents: write` and `metadata: read`
/// (FR-005b, FR-011): a grant without those two is unrepresentable.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grant {
    /// Adds `pull_requests: write`.
    pub pull_requests: bool,
    /// Adds `issues: write`.
    pub issues: bool,
}

impl Grant {
    /// The permissions a credential may carry, sorted by name.
    pub fn permissions(&self) -> Vec<(&'static str, &'static str)> {
        let mut permissions = vec![("contents", "write"), ("metadata", "read")];
        if self.issues {
            permissions.push(("issues", "write"));
        }
        if self.pull_requests {
            permissions.push(("pull_requests", "write"));
        }
        permissions.sort_unstable();
        permissions
    }
}

fn level(access: &str) -> u8 {
    match access {
        "read" => 1,
        "write" => 2,
        "admin" => 3,
        _ => u8::MAX,
    }
}

/// What a source answered, before [`ScopedCredential::accept`] has judged it.
#[derive(Debug, Clone)]
pub struct SourceResponse {
    /// The token.
    pub token: SecretToken,
    /// The repositories the token reaches.
    pub repositories: Vec<GithubRepo>,
    /// The permissions the token carries, `(name, access)`.
    pub permissions: Vec<(String, String)>,
    /// When it expires; `None` is refused (FR-009).
    pub expires_at: Option<UnixTime>,
}

/// Why a project has no scoped credential (data-model §6, FR-012).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoCredentialReason {
    /// No token is stored for the repository.
    SourceNotSetUp,
    /// The stored token has expired.
    PatExpired,
    /// The source refused, or answered with more than was asked.
    SourceRefused(String),
    /// The source could not be reached.
    SourceUnreachable(String),
    /// The remote is not on github.com.
    NotGithub {
        /// The remote's host.
        host: String,
    },
    /// The project has no `origin`.
    NoRemote,
    /// The container's git cannot use scoped credentials.
    GitTooOld {
        /// `git --version`'s version.
        version: String,
    },
    /// A renewal failed and the held credential has expired (only a source that renews).
    RenewalFailed(Box<NoCredentialReason>),
}

impl NoCredentialReason {
    /// The sentence the status shows, remedy included.
    pub fn message(&self) -> String {
        match self {
            Self::SourceNotSetUp => SOURCE_SETUP_HINT.to_string(),
            Self::PatExpired => "The GitHub token for this repository expired. Paste a new one in \
                 Settings › GitHub access tokens."
                .to_string(),
            Self::SourceRefused(reason) => {
                format!("GitHub refused the token: {reason}. Paste a corrected one in Settings.")
            }
            Self::SourceUnreachable(reason) => {
                format!("GitHub could not be reached: {reason}. Retrying.")
            }
            Self::NotGithub { host } => {
                format!("The remote's host {host} is not supported for scoped credentials.")
            }
            Self::NoRemote => "The project has no GitHub remote (origin).".to_string(),
            Self::GitTooOld { version } => format!(
                "Git {version} in the container is older than 2.31, so it cannot use scoped \
                 credentials. Use an image with a newer git."
            ),
            Self::RenewalFailed(inner) => format!(
                "Renewing the credential failed: {}. Pushes fail until it succeeds.",
                inner.message().trim_end_matches('.')
            ),
        }
    }
}

/// One issued token, constructed only by [`ScopedCredential::accept`].
#[derive(Debug, Clone)]
pub struct ScopedCredential {
    repo: GithubRepo,
    token: SecretToken,
    grant: Grant,
    expires_at: UnixTime,
    issued_at: UnixTime,
}

impl ScopedCredential {
    /// Judge `response` to a request for `repo` with `grant` at `now`: it must reach exactly
    /// `[repo]`, carry no permission beyond `grant`, and expire after `now` and no later than
    /// `now + MINTED_LIFETIME_SECS` (FR-009). Anything else is refused.
    pub fn accept(
        repo: &GithubRepo,
        grant: Grant,
        response: SourceResponse,
        now: UnixTime,
    ) -> Result<Self, NoCredentialReason> {
        let wider = || NoCredentialReason::SourceRefused("wider than asked".to_string());
        let [reached] = response.repositories.as_slice() else {
            return Err(wider());
        };
        if !(reached.owner().eq_ignore_ascii_case(repo.owner())
            && reached.name().eq_ignore_ascii_case(repo.name()))
        {
            return Err(wider());
        }
        let allowed = grant.permissions();
        let within = response.permissions.iter().all(|(name, access)| {
            allowed
                .iter()
                .any(|(n, a)| n == name && level(access) <= level(a))
        });
        if !within {
            return Err(wider());
        }
        match response.expires_at {
            Some(expires_at)
                if expires_at > now
                    && expires_at <= now + MINTED_LIFETIME_SECS + CLOCK_SKEW_SECS =>
            {
                Ok(Self {
                    repo: repo.clone(),
                    token: response.token,
                    grant,
                    expires_at,
                    issued_at: now,
                })
            }
            _ => Err(wider()),
        }
    }

    /// A credential from a token the user pasted, with the expiry GitHub reported for it: refused
    /// when that is not after `now` (`PatExpired`) or lies beyond [`MAX_LIFETIME_SECS`] plus the
    /// clock skew (FR-009).
    pub fn pasted(
        repo: &GithubRepo,
        token: SecretToken,
        expires_at: UnixTime,
        now: UnixTime,
    ) -> Result<Self, NoCredentialReason> {
        if expires_at <= now {
            return Err(NoCredentialReason::PatExpired);
        }
        if expires_at > now + MAX_LIFETIME_SECS + CLOCK_SKEW_SECS {
            return Err(NoCredentialReason::SourceRefused(
                "the token's expiry is more than a year away".to_string(),
            ));
        }
        Ok(Self {
            repo: repo.clone(),
            token,
            grant: Grant::default(),
            expires_at,
            issued_at: now,
        })
    }

    /// The one repository it reaches.
    pub fn repo(&self) -> &GithubRepo {
        &self.repo
    }

    /// The token.
    pub fn token(&self) -> &SecretToken {
        &self.token
    }

    /// What it was asked to be able to do.
    pub fn grant(&self) -> Grant {
        self.grant
    }

    /// When it stops working.
    pub fn expires_at(&self) -> UnixTime {
        self.expires_at
    }

    /// When it was issued.
    pub fn issued_at(&self) -> UnixTime {
        self.issued_at
    }
}

/// Where scoped credentials come from (research R3). Nothing but an implementation names a source.
pub trait CredentialSource: Send + Sync {
    /// Issue a credential for `repo` with `grant`.
    fn issue(
        &self,
        repo: &GithubRepo,
        grant: Grant,
    ) -> Result<ScopedCredential, NoCredentialReason>;

    /// Revoke an issued credential, best effort. The pasted-token source does nothing: the token is
    /// the user's and is never revoked on GitHub (FR-010).
    fn revoke(&self, credential: &ScopedCredential) -> Result<(), NoCredentialReason>;
}

/// What the client knows of one daemon project's credential (data-model §7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialStatus {
    /// Waiting for the first issue.
    Pending,
    /// A usable credential.
    Held {
        /// The repository it reaches.
        repo: GithubRepo,
        /// When it expires.
        expires_at: UnixTime,
    },
    /// None, and why.
    None(NoCredentialReason),
}

impl CredentialStatus {
    /// Whether this is a held credential with `within_secs` or less to live at `now` (FR-008).
    pub fn expires_within(&self, now: UnixTime, within_secs: u64) -> bool {
        matches!(self, Self::Held { expires_at, .. } if *expires_at <= now + within_secs)
    }
}

pub use crate::protocol::messages::CredentialTarget;

impl CredentialTarget {
    /// The GitHub repository the project pushes to: `NoRemote` without an `origin`, `NotGithub`
    /// for any other host.
    pub fn repo(&self) -> Result<GithubRepo, NoCredentialReason> {
        let url = self
            .push_url
            .as_deref()
            .ok_or(NoCredentialReason::NoRemote)?;
        GithubRepo::from_remote_url(url).ok_or_else(|| NoCredentialReason::NotGithub {
            host: remote_host(url),
        })
    }
}

/// The host a remote URL names, for the message; `unknown` when it names none.
fn remote_host(url: &str) -> String {
    let url = url.trim();
    let authority = match url.split_once("://") {
        Some((_, rest)) => rest.split('/').next().unwrap_or_default(),
        None => match url.split_once(':') {
            Some((user_host, _)) if !user_host.contains('/') => user_host,
            _ => "",
        },
    };
    let host_port = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    let host = host_port.split(':').next().unwrap_or_default();
    if host.is_empty() {
        "unknown".to_string()
    } else {
        host.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: UnixTime = 10_000;

    fn repo() -> GithubRepo {
        GithubRepo::from_remote_url("https://github.com/o/r").unwrap()
    }

    fn response(expires_at: Option<UnixTime>) -> SourceResponse {
        SourceResponse {
            token: SecretToken::new("tok"),
            repositories: vec![repo()],
            permissions: vec![
                ("contents".into(), "write".into()),
                ("metadata".into(), "read".into()),
            ],
            expires_at,
        }
    }

    fn refused(r: Result<ScopedCredential, NoCredentialReason>) -> bool {
        matches!(r, Err(NoCredentialReason::SourceRefused(m)) if m == "wider than asked")
    }

    #[test]
    fn the_default_grant_is_contents_write_and_metadata_read() {
        assert_eq!(
            Grant::default().permissions(),
            [("contents", "write"), ("metadata", "read")]
        );
    }

    #[test]
    fn the_extra_permissions_are_added_only_when_set() {
        let both = Grant {
            pull_requests: true,
            issues: true,
        };
        assert_eq!(
            both.permissions(),
            [
                ("contents", "write"),
                ("issues", "write"),
                ("metadata", "read"),
                ("pull_requests", "write")
            ]
        );
        let prs = Grant {
            pull_requests: true,
            issues: false,
        };
        assert!(prs.permissions().contains(&("pull_requests", "write")));
        assert!(!prs.permissions().iter().any(|(n, _)| *n == "issues"));
    }

    #[test]
    fn a_secret_token_never_prints() {
        let shown = format!("{:?}", SecretToken::new("ghs_secret"));
        assert!(
            shown.contains("redacted") && !shown.contains("ghs_secret"),
            "{shown}"
        );
    }

    #[test]
    fn an_exact_response_is_accepted() {
        let c =
            ScopedCredential::accept(&repo(), Grant::default(), response(Some(NOW + 3600)), NOW)
                .unwrap();
        assert_eq!(
            (c.expires_at(), c.issued_at(), c.repo()),
            (NOW + 3600, NOW, &repo())
        );
        assert_eq!(c.token().expose(), "tok");
        assert!(!format!("{c:?}").contains("tok\""));
    }

    #[test]
    fn other_repositories_are_refused() {
        let other = GithubRepo::from_remote_url("https://github.com/o/other").unwrap();
        for repositories in [vec![other.clone()], vec![repo(), other], vec![]] {
            let r = SourceResponse {
                repositories,
                ..response(Some(NOW + 60))
            };
            assert!(refused(ScopedCredential::accept(
                &repo(),
                Grant::default(),
                r,
                NOW
            )));
        }
    }

    #[test]
    fn permissions_beyond_the_grant_are_refused() {
        for extra in [
            ("issues", "write"),
            ("contents", "admin"),
            ("metadata", "write"),
        ] {
            let mut r = response(Some(NOW + 60));
            r.permissions.push((extra.0.into(), extra.1.into()));
            assert!(
                refused(ScopedCredential::accept(&repo(), Grant::default(), r, NOW)),
                "{extra:?}"
            );
        }
        let mut r = response(Some(NOW + 60));
        r.permissions.push(("issues".into(), "write".into()));
        let grant = Grant {
            pull_requests: false,
            issues: true,
        };
        assert!(ScopedCredential::accept(&repo(), grant, r, NOW).is_ok());
    }

    #[test]
    fn an_expiry_within_the_clock_skew_is_accepted() {
        let r = response(Some(NOW + MINTED_LIFETIME_SECS + CLOCK_SKEW_SECS));
        assert!(ScopedCredential::accept(&repo(), Grant::default(), r, NOW).is_ok());
    }

    #[test]
    fn a_missing_late_or_past_expiry_is_refused() {
        for expires_at in [
            None,
            Some(NOW + MINTED_LIFETIME_SECS + CLOCK_SKEW_SECS + 1),
            Some(NOW),
        ] {
            assert!(refused(ScopedCredential::accept(
                &repo(),
                Grant::default(),
                response(expires_at),
                NOW
            )));
        }
        assert!(ScopedCredential::accept(
            &repo(),
            Grant::default(),
            response(Some(NOW + MINTED_LIFETIME_SECS)),
            NOW
        )
        .is_ok());
    }

    #[test]
    fn every_reason_has_its_message() {
        let cases = [
            (NoCredentialReason::SourceNotSetUp, SOURCE_SETUP_HINT.to_string()),
            (
                NoCredentialReason::SourceRefused("nope".into()),
                "GitHub refused the token: nope. Paste a corrected one in Settings.".into(),
            ),
            (
                NoCredentialReason::PatExpired,
                "The GitHub token for this repository expired. Paste a new one in Settings › GitHub access tokens.".into(),
            ),
            (
                NoCredentialReason::SourceUnreachable("offline".into()),
                "GitHub could not be reached: offline. Retrying.".into(),
            ),
            (
                NoCredentialReason::NotGithub { host: "gitlab.com".into() },
                "The remote's host gitlab.com is not supported for scoped credentials.".into(),
            ),
            (NoCredentialReason::NoRemote, "The project has no GitHub remote (origin).".into()),
            (
                NoCredentialReason::GitTooOld { version: "2.30.0".into() },
                "Git 2.30.0 in the container is older than 2.31, so it cannot use scoped credentials. Use an image with a newer git.".into(),
            ),
            (
                NoCredentialReason::RenewalFailed(Box::new(NoCredentialReason::NoRemote)),
                "Renewing the credential failed: The project has no GitHub remote (origin). Pushes fail until it succeeds.".into(),
            ),
        ];
        for (reason, text) in cases {
            assert_eq!(reason.message(), text);
        }
    }

    #[test]
    fn a_pasted_credential_needs_a_future_expiry_within_the_cap() {
        let paste = |expires_at| {
            ScopedCredential::pasted(&repo(), SecretToken::new("github_pat_x"), expires_at, NOW)
        };
        let c = paste(NOW + 300 * 86_400).unwrap();
        assert_eq!((c.expires_at(), c.issued_at()), (NOW + 300 * 86_400, NOW));
        assert_eq!(c.grant(), Grant::default());
        assert!(paste(NOW + MAX_LIFETIME_SECS + CLOCK_SKEW_SECS).is_ok());
        assert!(matches!(
            paste(NOW + MAX_LIFETIME_SECS + CLOCK_SKEW_SECS + 1),
            Err(NoCredentialReason::SourceRefused(_))
        ));
        for past in [NOW, NOW - 1, 0] {
            assert!(matches!(paste(past), Err(NoCredentialReason::PatExpired)));
        }
    }

    #[test]
    fn only_a_held_credential_close_to_expiry_warns() {
        let held = |expires_at| CredentialStatus::Held {
            repo: repo(),
            expires_at,
        };
        let day = EXPIRY_WARNING_SECS;
        assert!(held(NOW + day).expires_within(NOW, day));
        assert!(!held(NOW + day + 1).expires_within(NOW, day));
        for other in [
            CredentialStatus::Pending,
            CredentialStatus::None(NoCredentialReason::PatExpired),
        ] {
            assert!(!other.expires_within(NOW, day));
        }
    }

    #[test]
    fn a_target_resolves_to_its_repository_or_says_why_not() {
        let target = |url: Option<&str>| CredentialTarget {
            project: "/p".into(),
            push_url: url.map(str::to_string),
        };
        assert_eq!(target(Some("git@github.com:o/r.git")).repo(), Ok(repo()));
        assert_eq!(target(None).repo(), Err(NoCredentialReason::NoRemote));
        assert_eq!(
            target(Some("https://user:pw@gitlab.com:8443/o/r")).repo(),
            Err(NoCredentialReason::NotGithub {
                host: "gitlab.com".into()
            })
        );
        assert_eq!(
            target(Some("git@bitbucket.org:o/r.git")).repo(),
            Err(NoCredentialReason::NotGithub {
                host: "bitbucket.org".into()
            })
        );
        assert_eq!(
            target(Some("/srv/git/r.git")).repo(),
            Err(NoCredentialReason::NotGithub {
                host: "unknown".into()
            })
        );
    }
}
