//! Failure classification and notice texts for Jira (feature 533, contracts/jira-cloud-source.md
//! §6). Pure: a transport or status outcome in, an `IssueLoadError` and its notice out.

use micold_core::github::{GithubRepo, IssueLoadError};
use micold_core::jira::{classify, Outcome};

const HOST: &str = "acme.atlassian.net";
const KEY: &str = "ABC";
const SENTINEL: &str = "sentinel-token-9f3a";

fn notice(error: &IssueLoadError) -> String {
    error.message(micold_core::github::NoticeSource::Jira {
        host: HOST,
        key: KEY,
    })
}

#[test]
fn each_outcome_has_its_variant() {
    assert_eq!(
        classify(&Outcome::Unreachable("connection refused".into())),
        IssueLoadError::Unreachable("connection refused".into())
    );
    assert_eq!(classify(&Outcome::TimedOut), IssueLoadError::TimedOut);
    for status in [401, 403] {
        assert_eq!(
            classify(&Outcome::Status(status)),
            IssueLoadError::CredentialsRefused
        );
    }
    for status in [400, 404] {
        assert_eq!(classify(&Outcome::Status(status)), IssueLoadError::NoAccess);
    }
    assert_eq!(classify(&Outcome::Status(429)), IssueLoadError::RateLimited);
    assert_eq!(classify(&Outcome::Redirected), IssueLoadError::NotJiraCloud);
    assert_eq!(classify(&Outcome::BadBody), IssueLoadError::NotJiraCloud);
    assert_eq!(
        classify(&Outcome::Status(503)),
        IssueLoadError::Other("Jira returned an error (503).".into())
    );
}

#[test]
fn an_unreachable_reason_is_cut_to_eighty_characters() {
    let IssueLoadError::Unreachable(reason) = classify(&Outcome::Unreachable("é".repeat(200)))
    else {
        panic!("unreachable");
    };
    assert_eq!(reason.chars().count(), 80);
}

#[test]
fn notices_read_as_the_contract_says() {
    assert_eq!(
        notice(&IssueLoadError::Unreachable("dns failure".into())),
        "Couldn't reach acme.atlassian.net: dns failure."
    );
    assert_eq!(
        notice(&IssueLoadError::TimedOut),
        "acme.atlassian.net did not answer within 10 seconds."
    );
    assert_eq!(
        notice(&IssueLoadError::CredentialsRefused),
        "acme.atlassian.net refused the credentials. Check the email and API token in Settings > Jira."
    );
    assert_eq!(
        notice(&IssueLoadError::NoAccess),
        "acme.atlassian.net does not show project ABC to this account. Check the project key and your access in Settings > Jira."
    );
    assert_eq!(
        notice(&IssueLoadError::RateLimited),
        "acme.atlassian.net is limiting requests. Try again in a minute."
    );
    assert_eq!(
        notice(&IssueLoadError::NotJiraCloud),
        "acme.atlassian.net did not answer like Jira Cloud. Jira Data Center and Server are not supported."
    );
    assert_eq!(
        notice(&IssueLoadError::Other(
            "Jira returned an error (500).".into()
        )),
        "Jira returned an error (500)."
    );
}

#[test]
fn only_the_transport_reason_reaches_a_notice_and_it_is_cut_first() {
    // The classifier is handed transport text, never the request: the reason is the one text that
    // flows through, so the token appears exactly when the reason carried it within 80 characters.
    let carried = classify(&Outcome::Unreachable(format!("refused {SENTINEL}")));
    assert_eq!(
        notice(&carried),
        format!("Couldn't reach acme.atlassian.net: refused {SENTINEL}.")
    );
    let cut = classify(&Outcome::Unreachable(format!(
        "{}{SENTINEL}",
        "x".repeat(80)
    )));
    assert_eq!(
        notice(&cut),
        format!("Couldn't reach acme.atlassian.net: {}.", "x".repeat(80))
    );
    for outcome in [
        Outcome::TimedOut,
        Outcome::Status(401),
        Outcome::Status(404),
        Outcome::Status(429),
        Outcome::Redirected,
        Outcome::BadBody,
        Outcome::Status(500),
    ] {
        assert!(
            !notice(&classify(&outcome)).contains(SENTINEL),
            "{outcome:?}"
        );
    }
}

#[test]
fn github_notices_are_unchanged_and_a_repo_still_works_as_the_source() {
    let repo = GithubRepo::from_remote_url("https://github.com/o/r").expect("a repo");
    assert_eq!(
        IssueLoadError::NoAccess.message(&repo),
        "Couldn't read issues: your GitHub sign-in can't access o/r."
    );
    assert_eq!(
        IssueLoadError::Offline.message(&repo),
        "Couldn't reach GitHub. Check your connection, then retry."
    );
}

#[test]
fn a_variant_asked_in_the_other_providers_voice_gets_a_generic_text() {
    let repo = GithubRepo::from_remote_url("https://github.com/o/r").expect("a repo");
    for error in [
        IssueLoadError::Unreachable("x".into()),
        IssueLoadError::CredentialsRefused,
        IssueLoadError::NotJiraCloud,
    ] {
        assert_eq!(
            error.message(&repo),
            "Couldn't read issues. Check your connection and settings, then retry."
        );
    }
    for (error, text) in [
        (
            IssueLoadError::ToolMissing,
            "Couldn't read issues from acme.atlassian.net. Check Settings > Jira, then retry.",
        ),
        (
            IssueLoadError::NotSignedIn,
            "Couldn't read issues from acme.atlassian.net. Check Settings > Jira, then retry.",
        ),
        (
            IssueLoadError::Offline,
            "Couldn't reach acme.atlassian.net. Check your connection, then retry.",
        ),
    ] {
        assert_eq!(notice(&error), text);
    }
}
