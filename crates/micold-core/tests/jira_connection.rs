//! Jira connection parsing (feature 533, FR-006, contracts/jira-connection.md §2): every part is
//! refused with its own text, and every refused field is reported at once.

use micold_core::jira::{
    AccountEmail, JiraConnection, JiraField, ProjectKey, SiteOrigin, EMAIL_REFUSED, KEY_REFUSED,
    SITE_REFUSED,
};

#[test]
fn a_site_is_normalised_to_its_origin() {
    for (input, origin) in [
        ("https://acme.atlassian.net", "https://acme.atlassian.net"),
        ("https://acme.atlassian.net/", "https://acme.atlassian.net"),
        (
            "  https://Acme.atlassian.net/jira/software ",
            "https://acme.atlassian.net",
        ),
        (
            "https://jira.example.com:8443/x",
            "https://jira.example.com:8443",
        ),
    ] {
        let parsed = SiteOrigin::parse(input).unwrap_or_else(|_| panic!("{input} accepted"));
        assert_eq!(parsed.to_string(), origin, "{input}");
    }
}

#[test]
fn a_site_that_is_not_an_https_origin_is_refused() {
    for input in [
        "",
        "acme.atlassian.net",
        "http://acme.atlassian.net",
        "ftp://acme.atlassian.net",
        "https://user:pw@acme.atlassian.net",
        "https://",
        "https:///path",
        "https://acme.atlassian.net?x=1",
        "https://acme.atlassian.net/#frag",
        "https://acme atlassian.net",
        "https://acme.atlassian.net:abc",
        "https://acme.atlassian.net:0",
        "https://acme.atlassian.net:65536",
        "https://acme.atlassian.net:+80",
        "https://acme..atlassian.net",
        "https://acme.atlassian.net.",
        "https://-acme.atlassian.net",
    ] {
        let problem = SiteOrigin::parse(input).expect_err(input);
        assert_eq!(problem.field, JiraField::Site, "{input}");
        assert_eq!(problem.reason, SITE_REFUSED, "{input}");
    }
}

#[cfg(debug_assertions)]
#[test]
fn a_development_build_accepts_a_loopback_http_origin() {
    assert_eq!(
        SiteOrigin::parse("http://127.0.0.1:8080/")
            .unwrap()
            .as_str(),
        "http://127.0.0.1:8080"
    );
    assert!(SiteOrigin::parse("http://example.com").is_err());
}

#[test]
fn a_project_key_is_trimmed_and_upper_cased() {
    assert_eq!(ProjectKey::parse(" abc ").unwrap().as_str(), "ABC");
    assert_eq!(ProjectKey::parse("A1_B2").unwrap().as_str(), "A1_B2");
    assert_eq!(
        ProjectKey::parse("ABCDEFGHIJ").unwrap().as_str(),
        "ABCDEFGHIJ"
    );
}

#[test]
fn a_project_key_outside_the_rule_is_refused() {
    for input in ["", "A", "1AB", "_AB", "ABCDEFGHIJK", "AB-C", "AB C", "ÄB"] {
        let problem = ProjectKey::parse(input).expect_err(input);
        assert_eq!(problem.field, JiraField::ProjectKey, "{input}");
        assert_eq!(problem.reason, KEY_REFUSED, "{input}");
    }
}

#[test]
fn an_email_needs_one_at_a_local_part_and_a_dotted_domain() {
    assert_eq!(
        AccountEmail::parse(" me@acme.com ").unwrap().as_str(),
        "me@acme.com"
    );
    for input in [
        "",
        "me",
        "me@",
        "@acme.com",
        "me@acme",
        "me@@acme.com",
        "a@b@acme.com",
        "me @acme.com",
        "me@.com",
        "me@acme.",
    ] {
        let problem = AccountEmail::parse(input).expect_err(input);
        assert_eq!(problem.field, JiraField::Email, "{input}");
        assert_eq!(problem.reason, EMAIL_REFUSED, "{input}");
    }
}

#[test]
fn a_connection_reports_every_refused_field_at_once() {
    let ok = JiraConnection::parse("https://acme.atlassian.net/", "abc", "me@acme.com").unwrap();
    assert_eq!(ok.site.as_str(), "https://acme.atlassian.net");
    assert_eq!(ok.project_key.as_str(), "ABC");

    let problems = JiraConnection::parse("nope", "1", "x").unwrap_err();
    let fields: Vec<_> = problems.iter().map(|p| p.field).collect();
    assert_eq!(
        fields,
        vec![JiraField::Site, JiraField::ProjectKey, JiraField::Email]
    );
}

#[test]
fn a_stored_connection_is_re_validated_on_read() {
    let good: JiraConnection = serde_json::from_str(
        r#"{"site":"https://acme.atlassian.net","project_key":"ABC","email":"me@acme.com"}"#,
    )
    .unwrap();
    assert_eq!(good.project_key.as_str(), "ABC");
    assert!(serde_json::from_str::<JiraConnection>(
        r#"{"site":"http://acme.atlassian.net","project_key":"ABC","email":"me@acme.com"}"#
    )
    .is_err());
}
