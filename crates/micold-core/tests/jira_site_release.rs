//! A release build never accepts a plain-`http` site, not even a loopback one (feature 533,
//! FR-006, research R10). The development build's loopback allowance exists only for the manual
//! pass's fake site, so this runs in a release build only
//! (`cargo test --release -p micold-core --test jira_site_release`, a CI step of its own).

use micold_core::jira::SiteOrigin;

#[cfg_attr(debug_assertions, ignore = "release-only: run with --release")]
#[test]
fn a_release_build_refuses_a_loopback_http_site() {
    for input in [
        "http://127.0.0.1:8080",
        "http://localhost:8080",
        "http://127.0.0.1",
    ] {
        assert!(SiteOrigin::parse(input).is_err(), "{input} refused");
    }
    assert!(SiteOrigin::parse("https://acme.atlassian.net").is_ok());
}
