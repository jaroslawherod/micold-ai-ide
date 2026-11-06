//! The pasted-token source over HTTPS (#1002, contracts/pat-source.md; #1542): [`PatSource`], its
//! [`UreqTransport`] and the Settings check [`check_and_store`]. In the library, not the binary's
//! `shell`, so an integration test can drive them against a loopback fake GitHub API.
//!
//! The API base is a value the caller passes ([`ApiBase`]); every shipped constructor uses the
//! default, `https://api.github.com`, and nothing here reads an environment variable, a setting or
//! a file for it (FR-003, FR-004).

use std::sync::Arc;
use std::time::Duration;

use micold_core::daemon_config::RepoPattern;
use micold_core::github::GithubRepo;
use micold_core::scoped_credential::pat::{self, ApiBase, Checked, Reply, Transport};
use micold_core::scoped_credential::pat_store::{PatStore, StoredPat};
use micold_core::scoped_credential::token_choice::{choose, Filter};
use micold_core::scoped_credential::{
    CredentialSource, Grant, NoCredentialReason, ScopedCredential, SecretToken, UnixTime,
};

/// The whole request's budget: connect, send and read.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// The clock the source checks against.
pub type Clock = Arc<dyn Fn() -> UnixTime + Send + Sync>;

/// GitHub's REST API over HTTPS (`ureq` 3), as `jira_http` does Jira: 10 second budget, no redirect,
/// a non-2xx status is an `Ok` reply (contract §2).
#[derive(Debug, Clone)]
pub struct UreqTransport {
    agent: ureq::Agent,
}

impl Default for UreqTransport {
    fn default() -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(REQUEST_TIMEOUT))
            .max_redirects(0)
            .http_status_as_error(false)
            .build()
            .into();
        Self { agent }
    }
}

impl Transport for UreqTransport {
    fn get(&self, url: &str, token: &SecretToken) -> Result<Reply, String> {
        let failure = |e: ureq::Error| match e {
            ureq::Error::Timeout(_) => "timed out".to_string(),
            other => pat::short_error(&other.to_string(), token),
        };
        let mut response = self
            .agent
            .get(url)
            .header("Authorization", format!("Bearer {}", token.expose()))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "micold-ai-ide")
            .call()
            .map_err(failure)?;
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .filter_map(|(name, value)| {
                Some((name.as_str().to_string(), value.to_str().ok()?.to_string()))
            })
            .collect();
        let body = response.body_mut().read_to_string().map_err(failure)?;
        Ok(Reply {
            status,
            headers,
            body,
        })
    }
}

/// The pasted tokens as a [`CredentialSource`] (contract §1): `issue` is the stored token of the
/// repository, checked, with the expiry GitHub reports written back; `revoke` does nothing, never a
/// request, because the token is the user's.
pub struct PatSource {
    store: Option<PatStore>,
    transport: Arc<dyn Transport>,
    now: Clock,
    base: ApiBase,
}

impl PatSource {
    /// The source over `store` (none: every repository reads *not set up*).
    pub fn new(store: Option<PatStore>, transport: Arc<dyn Transport>, now: Clock) -> Self {
        Self {
            store,
            transport,
            now,
            base: ApiBase::default(),
        }
    }

    /// The same source checking against `base` instead of `https://api.github.com`. For a test that
    /// points the check at a loopback fake; no shipped caller uses it (FR-003, FR-004).
    pub fn with_api_base(mut self, base: ApiBase) -> Self {
        self.base = base;
        self
    }

    /// The device's store, GitHub over HTTPS and the system clock.
    pub fn on_device() -> Self {
        Self::new(
            PatStore::on_device(),
            Arc::new(UreqTransport::default()),
            Arc::new(micold_core::unix_time::unix_secs),
        )
    }
}

impl CredentialSource for PatSource {
    fn issue(
        &self,
        repo: &GithubRepo,
        _grant: Grant,
    ) -> Result<ScopedCredential, NoCredentialReason> {
        let store = self.store.as_ref();
        let (name, stored) = store
            .and_then(|s| token_for(s, repo))
            .ok_or(NoCredentialReason::SourceNotSetUp)?;
        let now = (self.now)();
        if stored.expires_at <= now {
            return Err(NoCredentialReason::PatExpired);
        }
        let expires_at = pat::check(&*self.transport, &self.base, repo, &stored.token, now)?;
        let credential = ScopedCredential::pasted(repo, stored.token.clone(), expires_at, now)?;
        if expires_at != stored.expires_at {
            let refreshed = StoredPat {
                expires_at,
                ..stored
            };
            if let Some(store) = store {
                let _ = store.put(&name, &refreshed);
            }
        }
        Ok(credential)
    }

    fn revoke(&self, _credential: &ScopedCredential) -> Result<(), NoCredentialReason> {
        Ok(())
    }
}

/// The stored token `repo` uses, with its name: an exact filter beats `owner/*` beats no filter,
/// and the first in the store's order wins a tie (#1529, FR-007).
fn token_for(store: &PatStore, repo: &GithubRepo) -> Option<(String, StoredPat)> {
    let tokens = store.list();
    let filters: Vec<Filter<'_>> = tokens
        .iter()
        .map(|(name, pat)| Filter {
            name,
            repos: &pat.repos,
        })
        .collect();
    let chosen = choose(&filters, repo)?;
    tokens.into_iter().nth(chosen)
}

/// Check a pasted token named `name` and, only when GitHub accepts it, store it narrowed to `repos`
/// (Settings, Add or Replace). Returns the expiry and the reach GitHub reports, or the reason for
/// the form; nothing is stored on a refusal.
pub fn check_and_store(
    transport: &dyn Transport,
    base: &ApiBase,
    store: Option<&PatStore>,
    name: &str,
    repos: Vec<RepoPattern>,
    token: SecretToken,
    now: UnixTime,
) -> Result<Checked, String> {
    let store = store.ok_or("No data directory to keep the token in.")?;
    // Push access is probed on the first repository the filter names exactly.
    let probe = repos.iter().find_map(|p| match p {
        RepoPattern::Exact(o, r) => {
            GithubRepo::from_remote_url(&format!("https://github.com/{o}/{r}"))
        }
        RepoPattern::Owner(_) => None,
    });
    let checked =
        pat::check_added(transport, base, &token, probe.as_ref(), now).map_err(|r| r.message())?;
    // The same acceptance the source applies, so a token stored here is one `issue` will hold.
    if let Some(repo) = &probe {
        ScopedCredential::pasted(repo, token.clone(), checked.expires_at, now)
            .map_err(|r| r.message())?;
    }
    store
        .put(
            name,
            &StoredPat {
                token,
                expires_at: checked.expires_at,
                repos,
            },
        )
        .map_err(|e| format!("Could not store the token: {e}"))?;
    Ok(checked)
}
