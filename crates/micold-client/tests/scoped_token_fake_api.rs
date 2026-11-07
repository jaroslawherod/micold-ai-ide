//! The scoped-token checks of the #1002 quickstart Part C (steps 2, 3, 8, 10, 12) run through the
//! client's real HTTP path against a fake GitHub API on loopback (#1542, FR-005..FR-007, FR-011).
//! The check is pointed at the fake with a test-built [`ApiBase`]; nothing leaves the machine and
//! every token is fake.

#[path = "support/fake_github.rs"]
mod fake_github;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use fake_github::FakeGithubApi;
use micold_client::features::settings::github_tokens::help;
use micold_client::pat_source::{check_and_store, PatSource, UreqTransport};
use micold_core::daemon_config::RepoPattern;
use micold_core::local_time::rfc3339_utc;
use micold_core::scoped_credential::pat::Transport;
use micold_core::scoped_credential::pat_store::PatStore;
use micold_core::scoped_credential::schedule::{RenewPolicy, Schedule};
use micold_core::scoped_credential::{
    CredentialStatus, CredentialTarget, Grant, NoCredentialReason, SecretToken, UnixTime,
    EXPIRY_WARNING_SECS,
};

const T0: UnixTime = 1_800_000_000;
const DAY: u64 = 86_400;
const PAT: &str = "github_pat_FAKE_scratch0001";
const REPO: &str = "you/scratch";

/// The expiry header's value for `at`, as GitHub writes it.
fn header_for(at: UnixTime) -> String {
    rfc3339_utc(at as i64)
        .replace('T', " ")
        .replace('Z', " UTC")
}

fn only_scratch() -> Vec<RepoPattern> {
    vec![RepoPattern::Exact("you".into(), "scratch".into())]
}

/// A fake that answers the listing and the probe of `you/scratch` for a token expiring at `expiry`.
fn fake_with_valid_token(expiry: UnixTime) -> FakeGithubApi {
    let fake = FakeGithubApi::start();
    fake.listing(&[], Some(&header_for(expiry)));
    fake.probe(REPO, true, Some(&header_for(expiry)));
    fake
}

fn paste(
    fake: &FakeGithubApi,
    store: &PatStore,
    token: &str,
    now: UnixTime,
) -> Result<micold_core::scoped_credential::pat::Checked, String> {
    check_and_store(
        &UreqTransport::default(),
        &fake.base(),
        Some(store),
        "work",
        only_scratch(),
        SecretToken::new(token),
        now,
    )
}

fn store() -> (PatStore, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    (PatStore::new(dir.path()), dir)
}

#[test]
fn the_fake_replies_with_its_canned_answer_and_records_the_request() {
    let fake = FakeGithubApi::start();
    fake.probe(REPO, true, Some("2030-01-02 03:04:05 UTC"));
    let url = fake.url(&format!("/repos/{REPO}"));
    let reply = UreqTransport::default()
        .get(&url, &SecretToken::new(PAT))
        .unwrap();
    assert_eq!(reply.status, 200);
    assert_eq!(
        reply.header("github-authentication-token-expiration"),
        Some("2030-01-02 03:04:05 UTC")
    );
    let seen = fake.requests();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].target, format!("/repos/{REPO}"));
    assert_eq!(
        seen[0].authorization.as_deref(),
        Some(format!("Bearer {PAT}").as_str())
    );
    let missing = UreqTransport::default()
        .get(&url.replace("scratch", "nowhere"), &SecretToken::new(PAT))
        .unwrap();
    assert_eq!(missing.status, 404);
}

#[test]
fn step_2_a_valid_token_is_stored_with_the_expiry_the_fake_reported() {
    let fake = fake_with_valid_token(T0 + 30 * DAY);
    let (store, _dir) = store();
    let checked = paste(&fake, &store, PAT, T0).unwrap();
    assert_eq!(checked.expires_at, T0 + 30 * DAY);
    let stored = store.get("work").expect("stored");
    assert_eq!(stored.token.expose(), PAT);
    assert_eq!(stored.expires_at, T0 + 30 * DAY);
}

#[test]
fn step_2_a_classic_token_is_refused_before_any_request_and_nothing_is_stored() {
    let fake = fake_with_valid_token(T0 + 30 * DAY);
    let (store, _dir) = store();
    let refusal = paste(&fake, &store, "ghp_FAKE_classic", T0).unwrap_err();
    assert!(refusal.contains("fine-grained"), "{refusal}");
    assert!(store.get("work").is_none());
    assert!(fake.requests().is_empty());
}

#[test]
fn step_3_a_token_for_another_repository_is_refused_and_nothing_is_stored() {
    let fake = FakeGithubApi::start();
    fake.listing(&[], None);
    let (store, _dir) = store();
    let refusal = paste(&fake, &store, PAT, T0).unwrap_err();
    assert!(refusal.contains("does not reach you/scratch"), "{refusal}");
    assert!(store.get("work").is_none());
}

#[test]
fn step_3_a_token_with_no_expiry_is_refused_and_nothing_is_stored() {
    let fake = FakeGithubApi::start();
    fake.listing(&[], None);
    fake.probe(REPO, true, None);
    let (store, _dir) = store();
    let refusal = paste(&fake, &store, PAT, T0).unwrap_err();
    assert!(refusal.contains("no expiry"), "{refusal}");
    assert!(store.get("work").is_none());
}

#[test]
fn step_3_a_token_without_push_is_refused_and_nothing_is_stored() {
    let fake = FakeGithubApi::start();
    fake.listing(&[], None);
    fake.probe(REPO, false, Some(&header_for(T0 + DAY)));
    let (store, _dir) = store();
    let refusal = paste(&fake, &store, PAT, T0).unwrap_err();
    assert!(refusal.contains("cannot push"), "{refusal}");
    assert!(store.get("work").is_none());
}

/// Since #1529 (FR-009) a token that also reaches a second private repository is accepted: its reach
/// is listed for Settings to show, and nothing is refused for the count. The #1542 scenario that
/// said "refused" predates that change.
#[test]
fn step_3_a_token_reaching_a_second_private_repository_is_stored_and_its_reach_listed() {
    let fake = fake_with_valid_token(T0 + 30 * DAY);
    fake.listing(&["you/scratch", "you/other"], None);
    let (store, _dir) = store();
    let checked = paste(&fake, &store, PAT, T0).unwrap();
    assert_eq!(checked.reach, ["you/scratch", "you/other"]);
    assert!(store.get("work").is_some());
}

/// A schedule over the stored token for `you/scratch`, the token checked against `fake`.
struct Rig {
    source: PatSource,
    schedule: Schedule,
    store: PatStore,
    clock: Arc<AtomicU64>,
    fake: FakeGithubApi,
    _dir: tempfile::TempDir,
}

impl Rig {
    fn new(fake: FakeGithubApi) -> Self {
        let (store, dir) = store();
        let clock = Arc::new(AtomicU64::new(T0));
        let now = clock.clone();
        let source = PatSource::new(
            Some(store.clone()),
            Arc::new(UreqTransport::default()),
            Arc::new(move || now.load(Ordering::SeqCst)),
        )
        .with_api_base(fake.base());
        let mut schedule = Schedule::new(Grant::default(), RenewPolicy::AtExpiry);
        let target = CredentialTarget {
            project: "/p".into(),
            push_url: Some(format!("https://github.com/{REPO}")),
        };
        schedule.set_targets(&[target], None, &source, T0);
        Self {
            source,
            schedule,
            store,
            clock,
            fake,
            _dir: dir,
        }
    }

    fn now(&self) -> UnixTime {
        self.clock.load(Ordering::SeqCst)
    }

    fn put(&self, expires_at: UnixTime) {
        let checked = paste(&self.fake, &self.store, PAT, self.now());
        assert_eq!(checked.map(|c| c.expires_at), Ok(expires_at));
    }

    fn status(&self) -> CredentialStatus {
        self.schedule.status("/p".as_ref(), self.now())
    }
}

#[test]
fn step_8_a_removed_token_is_withdrawn_and_granted_again_when_pasted_without_a_restart() {
    let rig_fake = fake_with_valid_token(T0 + 30 * DAY);
    let mut rig = Rig::new(rig_fake);
    rig.put(T0 + 30 * DAY);
    let granted = rig.schedule.run(&rig.source, T0);
    assert!(granted[0].result.is_ok());
    assert!(matches!(rig.status(), CredentialStatus::Held { .. }));

    rig.store.remove("work").unwrap();
    rig.schedule.refresh_all(T0);
    let withdrawn = rig.schedule.run(&rig.source, T0);
    assert_eq!(
        withdrawn[0].result.as_ref().err(),
        Some(&NoCredentialReason::SourceNotSetUp)
    );
    assert!(withdrawn[0].withdraw);
    assert_eq!(
        rig.status(),
        CredentialStatus::None(NoCredentialReason::SourceNotSetUp)
    );

    rig.put(T0 + 30 * DAY);
    rig.schedule.refresh_all(T0);
    let again = rig.schedule.run(&rig.source, T0);
    assert!(again[0].result.is_ok());
    assert!(matches!(rig.status(), CredentialStatus::Held { .. }));
}

#[test]
fn step_10_a_token_within_a_day_of_its_expiry_warns_and_a_past_expiry_is_withdrawn() {
    let soon = T0 + 12 * 3600;
    let mut rig = Rig::new(fake_with_valid_token(soon));
    rig.put(soon);
    rig.schedule.run(&rig.source, T0);
    let status = rig.status();
    assert!(status.expires_within(T0, EXPIRY_WARNING_SECS), "{status:?}");

    rig.clock.store(soon, Ordering::SeqCst);
    rig.schedule.refresh_all(soon);
    let expired = rig.schedule.run(&rig.source, soon);
    assert_eq!(
        expired[0].result.as_ref().err(),
        Some(&NoCredentialReason::PatExpired)
    );
    assert!(expired[0].withdraw, "the daemon is told to delete the file");
    assert_eq!(
        rig.status(),
        CredentialStatus::None(NoCredentialReason::PatExpired)
    );
}

#[test]
fn a_token_goes_only_to_the_base_in_force_and_an_unreachable_base_stores_nothing() {
    let fake = fake_with_valid_token(T0 + 30 * DAY);
    let (store, _dir) = store();
    paste(&fake, &store, PAT, T0).unwrap();
    let seen = fake.requests();
    assert!(!seen.is_empty());
    assert!(seen
        .iter()
        .all(|r| r.authorization.as_deref() == Some(format!("Bearer {PAT}").as_str())));

    // A base nothing listens on: the check cannot reach it and never falls back to api.github.com.
    // A port that was bound and then released: nothing listens on it now.
    let dead = {
        let released = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        format!("http://{}", released.local_addr().unwrap())
    };
    let (empty, _d) = self::store();
    let refusal = check_and_store(
        &UreqTransport::default(),
        &micold_core::scoped_credential::pat::ApiBase::new(&dead),
        Some(&empty),
        "work",
        only_scratch(),
        SecretToken::new(PAT),
        T0,
    )
    .unwrap_err();
    assert!(
        refusal.starts_with("GitHub could not be reached:"),
        "expected the source-unreachable reason, got: {refusal}"
    );
    assert!(empty.get("work").is_none());
}

#[test]
fn step_12_the_paste_form_help_names_the_permissions_the_sandbox_and_the_risks() {
    let text = help().join(" ");
    for needle in [
        "Contents: Read and write",
        "Metadata: Read",
        // The sentence that says a permission the user adds (Pull requests: Read and write) is held
        // by the sandbox as well.
        "The sandbox can use everything the token can, and the client cannot narrow it.",
        "until it expires or you delete it on GitHub",
        "Remove deletes it here only; it does not delete it on GitHub.",
    ] {
        assert!(text.contains(needle), "{needle}");
    }
}
