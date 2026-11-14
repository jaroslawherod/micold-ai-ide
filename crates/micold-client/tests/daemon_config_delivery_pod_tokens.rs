//! Scoped GitHub tokens for a pod (#1760, FR-001, FR-002, FR-009): which stored tokens a pod
//! daemon receives, and how their files are read for the pod's startup Secret. Every token is fake.

use std::path::PathBuf;

use micold_client::daemon_config_delivery::select_tokens;
use micold_client::daemon_config_render::GithubTokenSetting;
use micold_core::daemon_config::RepoPattern;
use micold_core::github::GithubRepo;

fn token(name: &str, repos: &[&str]) -> GithubTokenSetting {
    GithubTokenSetting {
        name: name.into(),
        token_file: PathBuf::from(format!("/pats/{name}")),
        repos: repos
            .iter()
            .map(|r| RepoPattern::parse(r).expect("a pattern"))
            .collect(),
        host: None,
    }
}

fn repo(url: &str) -> GithubRepo {
    GithubRepo::from_remote_url(url).expect("a GitHub remote")
}

fn names(tokens: &[GithubTokenSetting]) -> Vec<&str> {
    tokens.iter().map(|t| t.name.as_str()).collect()
}

/// T006: a token is kept when a pattern names a project (`Exact` or `Owner`, either case), or
/// when it has no filter; Settings order is kept, and two tokens for one repository both stay.
#[test]
fn select_tokens_keeps_matching_and_unfiltered_tokens_in_settings_order() {
    let stored = vec![
        token("other", &["someone/else"]),
        token("owner", &["acme/*"]),
        token("any", &[]),
        token("exact", &["Acme/App"]),
        token("other-owner", &["elsewhere/*"]),
    ];
    let projects = [repo("https://github.com/ACME/app.git")];
    assert_eq!(
        names(&select_tokens(stored, &projects)),
        ["owner", "any", "exact"]
    );
}

/// T006, FR-009: a daemon with no saved project with a GitHub remote receives no token, not even
/// an unfiltered one.
#[test]
fn select_tokens_with_no_project_returns_nothing() {
    let stored = vec![token("any", &[]), token("owner", &["acme/*"])];
    assert!(select_tokens(stored, &[]).is_empty());
}
