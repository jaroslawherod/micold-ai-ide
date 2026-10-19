//! A scriptable in-memory [`CredentialSource`] for sequence tests (#1002), as `kube::fake` is for
//! clusters. Compiled for core tests and, through the `kube-fake` feature, for client tests.

use std::collections::VecDeque;
use std::sync::Mutex;

use super::{
    CredentialSource, Grant, NoCredentialReason, ScopedCredential, SecretToken, SourceResponse,
    UnixTime, MINTED_LIFETIME_SECS,
};
use crate::github::GithubRepo;

#[derive(Default)]
struct Inner {
    now: UnixTime,
    lifetime: UnixTime,
    next: u64,
    failures: VecDeque<NoCredentialReason>,
    revoke_fails: bool,
    issued: Vec<(GithubRepo, Grant)>,
    revoked: Vec<GithubRepo>,
}

/// Issues `fake-token-<n>` credentials that live [`MINTED_LIFETIME_SECS`] from the source's clock,
/// which a test moves with [`FakeSource::set_now`]; failures are scripted one at a time.
pub struct FakeSource {
    inner: Mutex<Inner>,
}

impl Default for FakeSource {
    fn default() -> Self {
        Self {
            inner: Mutex::new(Inner {
                lifetime: MINTED_LIFETIME_SECS,
                ..Inner::default()
            }),
        }
    }
}

impl FakeSource {
    /// A source whose clock reads 0.
    pub fn new() -> Self {
        Self::default()
    }

    /// Move the clock the credentials are issued against.
    pub fn set_now(&self, now: UnixTime) {
        self.inner.lock().unwrap().now = now;
    }

    /// Make the next `issue` fail with `reason` (queued: each call consumes one).
    pub fn fail_next(&self, reason: NoCredentialReason) {
        self.inner.lock().unwrap().failures.push_back(reason);
    }

    /// Make every `revoke` fail (offline).
    pub fn fail_revokes(&self) {
        self.inner.lock().unwrap().revoke_fails = true;
    }

    /// Every request made, in order.
    pub fn issued(&self) -> Vec<(GithubRepo, Grant)> {
        self.inner.lock().unwrap().issued.clone()
    }

    /// Every credential revoked, in order (failed attempts included).
    pub fn revoked(&self) -> Vec<GithubRepo> {
        self.inner.lock().unwrap().revoked.clone()
    }
}

impl CredentialSource for FakeSource {
    fn issue(
        &self,
        repo: &GithubRepo,
        grant: Grant,
    ) -> Result<ScopedCredential, NoCredentialReason> {
        let mut inner = self.inner.lock().unwrap();
        inner.issued.push((repo.clone(), grant));
        if let Some(reason) = inner.failures.pop_front() {
            return Err(reason);
        }
        inner.next += 1;
        let response = SourceResponse {
            token: SecretToken::new(format!("fake-token-{}", inner.next)),
            repositories: vec![repo.clone()],
            permissions: grant
                .permissions()
                .into_iter()
                .map(|(n, a)| (n.to_string(), a.to_string()))
                .collect(),
            expires_at: Some(inner.now + inner.lifetime),
        };
        ScopedCredential::accept(repo, grant, response, inner.now)
    }

    fn revoke(&self, credential: &ScopedCredential) -> Result<(), NoCredentialReason> {
        let mut inner = self.inner.lock().unwrap();
        inner.revoked.push(credential.repo().clone());
        if inner.revoke_fails {
            Err(NoCredentialReason::SourceUnreachable("offline".into()))
        } else {
            Ok(())
        }
    }
}
