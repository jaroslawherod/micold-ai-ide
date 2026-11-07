//! The scoped-token checks of the #1002 quickstart Part C (steps 2, 3, 12; 8 and 10 went with the retired grant schedule, #1530) run through the
//! client's real HTTP path against a fake GitHub API on loopback (#1542, FR-005..FR-007, FR-011).
//! The check is pointed at the fake with a test-built [`ApiBase`]; nothing leaves the machine and
//! every token is fake.

#[path = "support/fake_github.rs"]
mod fake_github;

use fake_github::FakeGithubApi;
use micold_client::features::settings::github_tokens::help;
use micold_client::pat_source::{check_and_store, UreqTransport};
use micold_core::daemon_config::RepoPattern;
use micold_core::local_time::rfc3339_utc;
use micold_core::scoped_credential::pat::Transport;
use micold_core::scoped_credential::pat_store::PatStore;
use micold_core::scoped_credential::{SecretToken, UnixTime};

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
