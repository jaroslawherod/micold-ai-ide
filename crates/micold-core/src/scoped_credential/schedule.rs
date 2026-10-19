//! The renewal state machine of one sandboxed daemon's credentials (#1002, FR-008–FR-010, FR-012,
//! data-model §8, research R8).
//!
//! State is kept per repository, so two projects on one repository share one credential and two
//! repositories get two. [`Schedule::due`] and [`Schedule::next_wake`] are pure; the methods that
//! call a [`CredentialSource`] take the time as an argument, so a test drives the whole machine
//! with a clock it owns.

use std::collections::BTreeMap;
use std::path::PathBuf;

use super::{
    CredentialSource, CredentialStatus, CredentialTarget, Grant, NoCredentialReason,
    ScopedCredential, UnixTime, RENEW_MARGIN_SECS,
};
use crate::github::GithubRepo;

/// The wait before retry `n` (0-based) of a failed issue: 1, 2, 4, 8, then 10 minutes.
fn backoff_secs(attempt: u32) -> u64 {
    const MINUTES: [u64; 5] = [1, 2, 4, 8, 10];
    MINUTES[(attempt as usize).min(MINUTES.len() - 1)] * 60
}

/// What one repository is up to.
#[derive(Debug)]
struct RepoState {
    held: Option<ScopedCredential>,
    /// The latest failure since the last success, with the failures in a row.
    failure: Option<(NoCredentialReason, u32)>,
    /// When the next issue is due.
    due_at: UnixTime,
}

/// What the client does for one daemon's projects.
#[derive(Debug)]
pub struct Schedule {
    grant: Grant,
    /// Each project's repository, or why it has none.
    targets: BTreeMap<PathBuf, Result<GithubRepo, NoCredentialReason>>,
    repos: BTreeMap<GithubRepo, RepoState>,
}

/// The outcome of one issue [`Schedule::run`] attempted.
#[derive(Debug)]
pub struct Renewal {
    /// The repository.
    pub repo: GithubRepo,
    /// The credential to send the daemon, or why there is none.
    pub result: Result<ScopedCredential, NoCredentialReason>,
}

impl Schedule {
    /// An empty schedule issuing credentials with `grant`.
    pub fn new(grant: Grant) -> Self {
        Self {
            grant,
            targets: BTreeMap::new(),
            repos: BTreeMap::new(),
        }
    }

    /// The daemon reported `targets` (the whole list). New repositories are due at `now`; a
    /// repository no project uses any more is revoked at `source` and returned, for the daemon to
    /// delete its file. With `git_too_old` nothing is issued and every target reads `GitTooOld`.
    pub fn set_targets(
        &mut self,
        targets: &[CredentialTarget],
        git_too_old: Option<&str>,
        source: &dyn CredentialSource,
        now: UnixTime,
    ) -> Vec<GithubRepo> {
        self.targets = targets
            .iter()
            .map(|t| {
                let resolved = match git_too_old {
                    Some(version) => Err(NoCredentialReason::GitTooOld {
                        version: version.to_string(),
                    }),
                    None => t.repo(),
                };
                (t.project.clone(), resolved)
            })
            .collect();
        let wanted: Vec<GithubRepo> = self
            .targets
            .values()
            .filter_map(|t| t.as_ref().ok().cloned())
            .collect();
        for repo in &wanted {
            self.repos.entry(repo.clone()).or_insert(RepoState {
                held: None,
                failure: None,
                due_at: now,
            });
        }
        let gone: Vec<GithubRepo> = self
            .repos
            .keys()
            .filter(|r| !wanted.contains(r))
            .cloned()
            .collect();
        for repo in &gone {
            if let Some(RepoState {
                held: Some(credential),
                ..
            }) = self.repos.remove(repo)
            {
                let _ = source.revoke(&credential);
            }
        }
        gone
    }

    /// The repositories whose issue is due at `now`.
    pub fn due(&self, now: UnixTime) -> Vec<GithubRepo> {
        self.repos
            .iter()
            .filter(|(_, s)| s.due_at <= now)
            .map(|(r, _)| r.clone())
            .collect()
    }

    /// The next time something needs doing: an issue falling due, or a held credential that has
    /// failed to renew expiring (its status changes then).
    pub fn next_wake(&self, now: UnixTime) -> Option<UnixTime> {
        self.repos
            .values()
            .flat_map(|s| {
                let expiry = s
                    .held
                    .as_ref()
                    .filter(|c| s.failure.is_some() && c.expires_at() > now)
                    .map(ScopedCredential::expires_at);
                [Some(s.due_at), expiry]
            })
            .flatten()
            .min()
    }

    /// Issue every credential due at `now` through `source`, recording each outcome.
    pub fn run(&mut self, source: &dyn CredentialSource, now: UnixTime) -> Vec<Renewal> {
        self.due(now)
            .into_iter()
            .map(|repo| {
                let result = source.issue(&repo, self.grant);
                self.record(&repo, &result, now);
                Renewal { repo, result }
            })
            .collect()
    }

    fn record(
        &mut self,
        repo: &GithubRepo,
        result: &Result<ScopedCredential, NoCredentialReason>,
        now: UnixTime,
    ) {
        let Some(state) = self.repos.get_mut(repo) else {
            return;
        };
        match result {
            Ok(credential) => {
                let (issued, expires) = (credential.issued_at(), credential.expires_at());
                // A credential shorter than the margin is renewed halfway, never in a hot loop.
                state.due_at = if expires > issued + RENEW_MARGIN_SECS {
                    expires - RENEW_MARGIN_SECS
                } else {
                    issued + (expires - issued) / 2 + 1
                };
                state.held = Some(credential.clone());
                state.failure = None;
            }
            Err(reason) => {
                let attempts = state.failure.as_ref().map_or(0, |(_, n)| *n);
                state.due_at = now + backoff_secs(attempts);
                state.failure = Some((reason.clone(), attempts + 1));
            }
        }
    }

    /// The status of `project` at `now`.
    pub fn status(&self, project: &std::path::Path, now: UnixTime) -> CredentialStatus {
        let repo = match self.targets.get(project) {
            None => return CredentialStatus::None(NoCredentialReason::NoRemote),
            Some(Err(reason)) => return CredentialStatus::None(reason.clone()),
            Some(Ok(repo)) => repo,
        };
        let Some(state) = self.repos.get(repo) else {
            return CredentialStatus::Pending;
        };
        match (&state.held, &state.failure) {
            (Some(c), _) if c.expires_at() > now => CredentialStatus::Held {
                repo: repo.clone(),
                expires_at: c.expires_at(),
            },
            (Some(_), Some((reason, _))) => {
                CredentialStatus::None(NoCredentialReason::RenewalFailed(Box::new(reason.clone())))
            }
            (Some(_), None) => CredentialStatus::Pending,
            (None, Some((reason, _))) => CredentialStatus::None(reason.clone()),
            (None, None) => CredentialStatus::Pending,
        }
    }

    /// The daemon stopped or was deleted: revoke every held credential at `source` (best effort,
    /// FR-010) and drop everything. Returns the repositories, for the daemon's files.
    pub fn revoke_all(&mut self, source: &dyn CredentialSource) -> Vec<GithubRepo> {
        self.targets.clear();
        std::mem::take(&mut self.repos)
            .into_iter()
            .map(|(repo, state)| {
                if let Some(credential) = state.held {
                    let _ = source.revoke(&credential);
                }
                repo
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scoped_credential::fake::FakeSource;

    const T0: UnixTime = 100_000;
    const HOUR: u64 = 3600;
    const MIN: u64 = 60;

    fn target(project: &str, url: Option<&str>) -> CredentialTarget {
        CredentialTarget {
            project: project.into(),
            push_url: url.map(str::to_string),
        }
    }

    fn repo(name: &str) -> GithubRepo {
        GithubRepo::from_remote_url(&format!("https://github.com/o/{name}")).unwrap()
    }

    fn one() -> Vec<CredentialTarget> {
        vec![target("/p", Some("https://github.com/o/a"))]
    }

    fn started(targets: &[CredentialTarget]) -> (Schedule, FakeSource) {
        let (mut s, f) = (Schedule::new(Grant::default()), FakeSource::new());
        f.set_now(T0);
        s.set_targets(targets, None, &f, T0);
        (s, f)
    }

    #[test]
    fn new_repositories_are_due_now() {
        let (s, _) = started(&[
            target("/p", Some("https://github.com/o/a")),
            target("/q", Some("git@github.com:o/b.git")),
            target("/r", None),
        ]);
        assert_eq!(s.due(T0), vec![repo("a"), repo("b")]);
        assert_eq!(s.next_wake(T0), Some(T0));
    }

    #[test]
    fn a_vanished_repository_is_revoked_and_dropped() {
        let (mut s, f) = started(&one());
        s.run(&f, T0);
        let gone = s.set_targets(&[], None, &f, T0 + 5);
        assert_eq!((gone, f.revoked()), (vec![repo("a")], vec![repo("a")]));
        assert_eq!((s.due(T0 + 5), s.next_wake(T0 + 5)), (vec![], None));
    }

    #[test]
    fn a_changed_remote_revokes_the_old_repository_unless_another_project_uses_it() {
        let (mut s, f) = started(&[
            target("/p", Some("https://github.com/o/a")),
            target("/q", Some("https://github.com/o/a")),
        ]);
        s.run(&f, T0);
        let moved = [
            target("/p", Some("https://github.com/o/b")),
            target("/q", Some("https://github.com/o/a")),
        ];
        assert!(s.set_targets(&moved, None, &f, T0 + 1).is_empty());
        assert_eq!((s.due(T0 + 1), f.revoked()), (vec![repo("b")], vec![]));
        let moved = [
            target("/p", Some("https://github.com/o/b")),
            target("/q", Some("https://github.com/o/b")),
        ];
        assert_eq!(s.set_targets(&moved, None, &f, T0 + 2), vec![repo("a")]);
        assert_eq!(f.revoked(), vec![repo("a")]);
    }

    #[test]
    fn a_good_issue_is_renewed_ten_minutes_before_it_expires() {
        let (mut s, f) = started(&one());
        let out = s.run(&f, T0);
        assert!(out[0].result.is_ok());
        assert_eq!(s.due(T0 + HOUR - 10 * MIN - 1), vec![]);
        assert_eq!(s.due(T0 + HOUR - 10 * MIN), vec![repo("a")]);
        assert_eq!(s.next_wake(T0 + 1), Some(T0 + HOUR - 10 * MIN));
        assert_eq!(
            s.status("/p".as_ref(), T0 + 1),
            CredentialStatus::Held {
                repo: repo("a"),
                expires_at: T0 + HOUR
            }
        );
    }

    #[test]
    fn two_consecutive_renewals_leave_a_valid_credential_throughout() {
        let (mut s, f) = started(&one());
        s.run(&f, T0);
        let mut now = T0;
        for round in 1..=2u64 {
            now = s.next_wake(now).unwrap();
            f.set_now(now);
            assert_eq!(
                s.status("/p".as_ref(), now),
                CredentialStatus::Held {
                    repo: repo("a"),
                    expires_at: T0 + (round - 1) * (HOUR - 10 * MIN) + HOUR,
                }
            );
            let out = s.run(&f, now);
            assert_eq!(out.len(), 1, "round {round}");
            assert!(out[0].result.is_ok());
        }
        assert_eq!(f.issued().len(), 3);
        assert!(matches!(
            s.status("/p".as_ref(), now),
            CredentialStatus::Held { .. }
        ));
    }

    #[test]
    fn failures_with_nothing_held_retry_at_1_2_4_8_10_10_minutes() {
        let (mut s, f) = started(&one());
        let mut now = T0;
        let mut waits = vec![];
        for _ in 0..6 {
            f.fail_next(NoCredentialReason::SourceUnreachable("offline".into()));
            f.set_now(now);
            s.run(&f, now);
            let wake = s.next_wake(now).unwrap();
            waits.push((wake - now) / MIN);
            assert_eq!(s.due(wake - 1), vec![]);
            now = wake;
        }
        assert_eq!(waits, [1, 2, 4, 8, 10, 10]);
        assert_eq!(
            s.status("/p".as_ref(), now),
            CredentialStatus::None(NoCredentialReason::SourceUnreachable("offline".into()))
        );
    }

    #[test]
    fn a_success_after_failures_resets_the_backoff() {
        let (mut s, f) = started(&one());
        f.fail_next(NoCredentialReason::SourceUnreachable("offline".into()));
        s.run(&f, T0);
        s.run(&f, T0 + MIN);
        assert!(matches!(
            s.status("/p".as_ref(), T0 + MIN),
            CredentialStatus::Held { .. }
        ));
        f.fail_next(NoCredentialReason::SourceUnreachable("again".into()));
        let due = s.next_wake(T0 + MIN).unwrap();
        s.run(&f, due);
        assert_eq!(s.next_wake(due), Some(due + MIN));
    }

    #[test]
    fn a_failed_renewal_keeps_the_credential_until_it_expires_then_says_so() {
        let (mut s, f) = started(&one());
        s.run(&f, T0);
        let due = T0 + HOUR - 10 * MIN;
        f.fail_next(NoCredentialReason::SourceRefused("revoked".into()));
        f.set_now(due);
        s.run(&f, due);
        assert!(matches!(
            s.status("/p".as_ref(), due + 1),
            CredentialStatus::Held { .. }
        ));
        assert_eq!(s.next_wake(due), Some(due + MIN));
        let expired = T0 + HOUR;
        assert_eq!(
            s.status("/p".as_ref(), expired),
            CredentialStatus::None(NoCredentialReason::RenewalFailed(Box::new(
                NoCredentialReason::SourceRefused("revoked".into())
            )))
        );
        // The next success restores it.
        f.set_now(expired);
        s.run(&f, expired);
        assert!(matches!(
            s.status("/p".as_ref(), expired),
            CredentialStatus::Held { .. }
        ));
    }

    #[test]
    fn the_expiry_of_a_credential_that_failed_to_renew_is_a_wake_up() {
        let (mut s, f) = started(&one());
        s.run(&f, T0);
        let due = T0 + HOUR - 10 * MIN;
        f.fail_next(NoCredentialReason::SourceUnreachable("offline".into()));
        s.run(&f, due);
        // Retry in a minute comes first; once that is past, the expiry is still ahead.
        assert_eq!(s.next_wake(due), Some(due + MIN));
        s.run(&f, due + MIN);
        assert!(s.next_wake(due + MIN).is_some());
    }

    #[test]
    fn stop_revokes_everything_and_drops_it() {
        let (mut s, f) = started(&[
            target("/p", Some("https://github.com/o/a")),
            target("/q", Some("https://github.com/o/b")),
        ]);
        s.run(&f, T0);
        let mut files = s.revoke_all(&f);
        files.sort();
        assert_eq!(files, vec![repo("a"), repo("b")]);
        assert_eq!(f.revoked().len(), 2);
        assert_eq!((s.due(T0 + 99 * HOUR), s.next_wake(T0)), (vec![], None));
    }

    #[test]
    fn a_revoke_that_fails_still_drops_the_credential() {
        let (mut s, f) = started(&one());
        s.run(&f, T0);
        f.fail_revokes();
        assert_eq!(s.revoke_all(&f), vec![repo("a")]);
        assert_eq!(f.revoked().len(), 1);
        assert_eq!(s.due(T0), vec![]);
    }

    #[test]
    fn projects_on_one_repository_share_one_credential_and_two_repositories_get_two() {
        let (mut s, f) = started(&[
            target("/p", Some("https://github.com/o/a")),
            target("/q", Some("git@github.com:o/a.git")),
            target("/r", Some("https://github.com/o/b")),
        ]);
        let out = s.run(&f, T0);
        assert_eq!(out.len(), 2);
        assert_eq!(f.issued().len(), 2);
        let held = |p: &str| match s.status(p.as_ref(), T0) {
            CredentialStatus::Held { repo, .. } => repo,
            other => panic!("{other:?}"),
        };
        assert_eq!(
            (held("/p"), held("/q"), held("/r")),
            (repo("a"), repo("a"), repo("b"))
        );
    }

    #[test]
    fn a_project_without_a_github_remote_issues_nothing_and_says_why() {
        let (mut s, f) = started(&[
            target("/n", None),
            target("/g", Some("https://gitlab.com/o/r")),
        ]);
        assert!(s.run(&f, T0).is_empty() && f.issued().is_empty());
        assert_eq!(
            s.status("/n".as_ref(), T0),
            CredentialStatus::None(NoCredentialReason::NoRemote)
        );
        assert_eq!(
            s.status("/g".as_ref(), T0),
            CredentialStatus::None(NoCredentialReason::NotGithub {
                host: "gitlab.com".into()
            })
        );
    }

    #[test]
    fn a_git_that_is_too_old_issues_nothing() {
        let (mut s, f) = (Schedule::new(Grant::default()), FakeSource::new());
        s.set_targets(&one(), Some("2.30.0"), &f, T0);
        assert!(s.run(&f, T0).is_empty() && f.issued().is_empty());
        assert_eq!(
            s.status("/p".as_ref(), T0),
            CredentialStatus::None(NoCredentialReason::GitTooOld {
                version: "2.30.0".into()
            })
        );
    }

    #[test]
    fn a_target_before_its_first_issue_is_pending() {
        let (s, _) = started(&one());
        assert_eq!(s.status("/p".as_ref(), T0), CredentialStatus::Pending);
    }

    #[test]
    fn the_grant_reaches_the_source() {
        let (mut s, f) = (
            Schedule::new(Grant {
                pull_requests: true,
                issues: false,
            }),
            FakeSource::new(),
        );
        f.set_now(T0);
        s.set_targets(&one(), None, &f, T0);
        s.run(&f, T0);
        assert_eq!(
            f.issued()[0].1,
            Grant {
                pull_requests: true,
                issues: false
            }
        );
    }

    #[test]
    fn due_and_next_wake_do_not_change_the_schedule() {
        let (s, _) = started(&one());
        let before = format!("{s:?}");
        let _ = (s.due(T0 + 7), s.next_wake(T0 + 7));
        assert_eq!(format!("{s:?}"), before);
    }
}
