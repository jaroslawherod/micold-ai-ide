//! The Jira Cloud adapter against a loopback fake (feature 533, contracts/jira-cloud-source.md
//! §2, §5; FR-009, FR-018, FR-020). Loopback `http` is accepted by `SiteOrigin` in debug builds,
//! which is how a test reaches a socket of its own.

#[path = "../src/shell/jira_http.rs"]
#[allow(dead_code)]
mod jira_http;

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use jira_http::JiraCloud;
use micold_core::github::IssueLoadError;
use micold_core::jira::{ApiToken, JiraConnection, JiraSource, ProjectKey};
use micold_core::jira_query::{QueryFailure, ScopedQuery, UserQuery};

const SENTINEL: &str = "sentinel-token-value-9f3a";
const PAGE: &str = include_str!("../../micold-core/tests/fixtures/jira/last_page.json");

/// A server that answers one request with `response` (or never, when `None`) and reports the raw
/// request it read.
fn serve(response: Option<String>) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut raw = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            let n = stream.read(&mut chunk).unwrap_or(0);
            raw.extend_from_slice(&chunk[..n]);
            let text = String::from_utf8_lossy(&raw).to_string();
            if let Some((head, body)) = text.split_once("\r\n\r\n") {
                let length = head
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|v| v.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                if body.len() >= length {
                    break;
                }
            }
            if n == 0 {
                break;
            }
        }
        tx.send(String::from_utf8_lossy(&raw).to_string()).unwrap();
        match response {
            Some(response) => {
                stream.write_all(response.as_bytes()).unwrap();
            }
            // Hold the connection open until the client gives up and closes it.
            None => while stream.read(&mut chunk).is_ok_and(|n| n > 0) {},
        }
    });
    (origin, rx)
}

/// An origin on a loopback port that was bound and then released, so nothing listens on it.
fn closed_origin() -> String {
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    format!("http://127.0.0.1:{port}")
}

fn ok(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn connection(origin: &str) -> JiraConnection {
    JiraConnection::parse(origin, "ABC", "me@acme.com").expect("loopback http is accepted in debug")
}

fn token() -> ApiToken {
    ApiToken::parse(SENTINEL).unwrap()
}

/// The request carries `Authorization`, `Accept`, `Content-Type` and `User-Agent`, and no other
/// header of ours (FR-020).
fn assert_only_the_four_headers(head: &str) {
    use base64::Engine;
    let basic = base64::engine::general_purpose::STANDARD.encode(format!("me@acme.com:{SENTINEL}"));
    let headers: Vec<(String, String)> = head
        .lines()
        .skip(1)
        .map(|l| {
            let (k, v) = l.split_once(':').unwrap();
            (k.to_ascii_lowercase(), v.trim().to_string())
        })
        .collect();
    let get = |name: &str| {
        headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
    };
    assert_eq!(get("authorization"), Some(format!("Basic {basic}")));
    assert_eq!(get("accept").as_deref(), Some("application/json"));
    assert_eq!(get("content-type").as_deref(), Some("application/json"));
    assert_eq!(get("user-agent").as_deref(), Some("micold-ai-ide"));
    // Nothing else of ours: the rest is what the protocol itself needs.
    for (name, _) in &headers {
        assert!(
            [
                "authorization",
                "accept",
                "content-type",
                "user-agent",
                "host",
                "content-length",
                "accept-encoding"
            ]
            .contains(&name.as_str()),
            "unexpected header {name}"
        );
    }
}

#[test]
fn the_request_is_the_documented_post_with_four_headers_and_a_jql_only_body() {
    let (origin, rx) = serve(Some(ok(PAGE)));
    let page = JiraCloud::new()
        .list_open(&connection(&origin), &token(), Some("cursor-1"))
        .expect("a page");
    assert_eq!(page.issues.len(), 1);
    let raw = rx.recv().unwrap();
    let (head, body) = raw.split_once("\r\n\r\n").unwrap();
    assert!(
        head.starts_with("POST /rest/api/3/search/jql HTTP/1.1"),
        "{head}"
    );
    assert_only_the_four_headers(head);
    let json: serde_json::Value = serde_json::from_str(body).unwrap();
    let keys: std::collections::BTreeSet<_> = json.as_object().unwrap().keys().cloned().collect();
    let expected: std::collections::BTreeSet<_> = ["jql", "fields", "maxResults", "nextPageToken"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(keys, expected);
    assert_eq!(json["nextPageToken"], "cursor-1");
    assert!(!body.contains(SENTINEL));
}

#[test]
fn a_search_body_carries_only_the_search_jql_and_the_page_size() {
    let (origin, rx) = serve(Some(ok(PAGE)));
    let found = JiraCloud::new()
        .search_open(&connection(&origin), &token(), "login \"x\"")
        .expect("issues");
    assert_eq!(found.len(), 1);
    let raw = rx.recv().unwrap();
    let (_, body) = raw.split_once("\r\n\r\n").unwrap();
    let json: serde_json::Value = serde_json::from_str(body).unwrap();
    let keys: std::collections::BTreeSet<_> = json.as_object().unwrap().keys().cloned().collect();
    let expected: std::collections::BTreeSet<_> = ["jql", "fields", "maxResults"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(keys, expected);
    assert_eq!(
        json["jql"].as_str(),
        Some(
            "project = \"ABC\" AND statusCategory != Done AND \
             (summary ~ \"login \\\"x\\\"*\" OR labels = \"login \\\"x\\\"\") \
             ORDER BY updated DESC"
        )
    );
    assert!(!body.contains(SENTINEL));
}

#[test]
fn an_empty_search_sends_nothing() {
    let found = JiraCloud::new().search_open(&connection(&closed_origin()), &token(), "  ");
    assert_eq!(found, Ok(Vec::new()));
}

#[test]
fn a_redirect_is_not_followed() {
    let (origin, _rx) = serve(Some(format!(
        "HTTP/1.1 302 Found\r\nLocation: {}/elsewhere\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        closed_origin()
    )));
    let error = JiraCloud::new()
        .list_open(&connection(&origin), &token(), None)
        .unwrap_err();
    assert_eq!(error, IssueLoadError::NotJiraCloud);
}

#[test]
fn a_refused_credential_is_classified() {
    let (origin, _rx) = serve(Some(
        "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string(),
    ));
    let error = JiraCloud::new()
        .list_open(&connection(&origin), &token(), None)
        .unwrap_err();
    assert_eq!(error, IssueLoadError::CredentialsRefused);
}

#[test]
fn a_page_that_is_not_a_listing_is_not_jira_cloud() {
    let (origin, _rx) = serve(Some(ok("<html>log in</html>")));
    let error = JiraCloud::new()
        .list_open(&connection(&origin), &token(), None)
        .unwrap_err();
    assert_eq!(error, IssueLoadError::NotJiraCloud);
}

#[test]
fn a_stalled_server_times_out_at_the_budget() {
    let (origin, _rx) = serve(None);
    let started = std::time::Instant::now();
    let error = JiraCloud::with_timeout(Duration::from_millis(300))
        .list_open(&connection(&origin), &token(), None)
        .unwrap_err();
    assert_eq!(error, IssueLoadError::TimedOut);
    assert!(started.elapsed() < Duration::from_secs(3));
}

#[test]
fn the_token_is_in_no_error_string_or_debug() {
    // Nothing listens on a port that was just released: a refused connection.
    let conn = connection(&closed_origin());
    let error = JiraCloud::new()
        .list_open(&conn, &token(), None)
        .unwrap_err();
    assert!(matches!(error, IssueLoadError::Unreachable(_)), "{error:?}");
    assert!(!format!("{error:?}").contains(SENTINEL));
    assert!(!error
        .message(micold_core::github::NoticeSource::Jira {
            host: "h",
            key: "ABC"
        })
        .contains(SENTINEL));
    assert!(!format!("{:?}", JiraCloud::new()).contains(SENTINEL));
    assert!(!format!("{:?}", token()).contains(SENTINEL));
}

#[test]
fn the_production_request_budget_is_ten_seconds() {
    assert_eq!(jira_http::REQUEST_TIMEOUT, Duration::from_secs(10));
}

// --- feature 1101, T017: the status request ---------------------------------------------

/// A server answering the next connection with the next response, reporting each request's body.
fn serve_each(responses: Vec<String>) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for response in responses {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut raw = Vec::new();
            let mut chunk = [0u8; 4096];
            loop {
                let n = stream.read(&mut chunk).unwrap_or(0);
                raw.extend_from_slice(&chunk[..n]);
                let text = String::from_utf8_lossy(&raw).to_string();
                if let Some((head, body)) = text.split_once("\r\n\r\n") {
                    let length = head
                        .lines()
                        .find_map(|l| {
                            l.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if body.len() >= length {
                        break;
                    }
                }
                if n == 0 {
                    break;
                }
            }
            let text = String::from_utf8_lossy(&raw).to_string();
            let body = text
                .split_once("\r\n\r\n")
                .map_or("", |(_, b)| b)
                .to_string();
            tx.send(body).unwrap();
            stream.write_all(response.as_bytes()).unwrap();
        }
    });
    (origin, rx)
}

fn status_body(keys: &[(&str, &str)]) -> String {
    let issues: Vec<String> = keys
        .iter()
        .map(|(k, name)| {
            format!(
                r#"{{"key":"{k}","fields":{{"status":{{"name":"{name}","statusCategory":{{"key":"done"}}}}}}}}"#
            )
        })
        .collect();
    format!(r#"{{"issues":[{}]}}"#, issues.join(","))
}

fn bad_request() -> String {
    "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string()
}

fn keys_of(body: &str) -> Vec<String> {
    let json: serde_json::Value = serde_json::from_str(body).unwrap();
    let jql = json["jql"].as_str().unwrap();
    jql.split('(')
        .nth(1)
        .unwrap()
        .split(')')
        .next()
        .unwrap()
        .split(',')
        .map(|k| k.trim_matches('"').to_string())
        .collect()
}

#[test]
fn a_status_request_asks_for_the_keys_and_only_the_status_field() {
    let (origin, rx) = serve_each(vec![ok(&status_body(&[("ABC-1", "Done")]))]);
    let keys = vec!["ABC-1".to_string(), "abc-2".to_string()];
    let got = JiraCloud::new()
        .statuses(&connection(&origin), &token(), &keys)
        .expect("statuses");
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].0, "ABC-1");
    assert_eq!(got[0].1.as_ref().unwrap().name, "Done");
    let body: serde_json::Value = serde_json::from_str(&rx.recv().unwrap()).unwrap();
    assert_eq!(
        body["jql"],
        r#"project = "ABC" AND key in ("ABC-1","ABC-2") ORDER BY key ASC"#
    );
    assert_eq!(body["fields"], serde_json::json!(["status"]));
    assert_eq!(body["maxResults"], 2);
}

#[test]
fn a_400_bisects_the_chunk_and_a_key_alone_in_a_400_is_absent() {
    // ABC-2 is unknown to Jira: every request holding it answers 400.
    let (origin, rx) = serve_each(vec![
        bad_request(),                                             // [1,2,3,4]
        bad_request(),                                             // [1,2]
        ok(&status_body(&[("ABC-1", "Done")])),                    // [1]
        bad_request(),                                             // [2] alone
        ok(&status_body(&[("ABC-3", "Done"), ("ABC-4", "Done")])), // [3,4]
    ]);
    let keys: Vec<String> = (1..=4).map(|n| format!("ABC-{n}")).collect();
    let got = JiraCloud::new()
        .statuses(&connection(&origin), &token(), &keys)
        .expect("statuses");
    let names: Vec<&str> = got.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(names, ["ABC-1", "ABC-3", "ABC-4"]);
    let asked: Vec<Vec<String>> = (0..5).map(|_| keys_of(&rx.recv().unwrap())).collect();
    assert_eq!(asked[1], ["ABC-1", "ABC-2"]);
    assert_eq!(asked[3], ["ABC-2"]);
}

#[test]
fn any_other_error_aborts_the_read_and_names_neither_token_nor_site() {
    let (origin, _rx) = serve_each(vec![
        "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            .to_string(),
    ]);
    let keys: Vec<String> = (1..=3).map(|n| format!("ABC-{n}")).collect();
    let error = JiraCloud::new()
        .statuses(&connection(&origin), &token(), &keys)
        .unwrap_err();
    let text = format!("{error:?}");
    assert!(
        !text.contains(SENTINEL) && !text.contains(&origin),
        "{text}"
    );
    assert!(matches!(error, IssueLoadError::Other(_)), "{text}");
}

// Feature 1186: the advanced request (contracts/selector-source-and-storage.md §2).

fn scoped(raw: &str) -> ScopedQuery {
    ScopedQuery::new(
        &ProjectKey::parse("ABC").unwrap(),
        &UserQuery::parse(raw).unwrap(),
    )
}

/// The body of the one request `serve` read.
fn body_of(rx: &mpsc::Receiver<String>) -> serde_json::Value {
    let raw = rx.recv().unwrap();
    let (_, body) = raw.split_once("\r\n\r\n").unwrap();
    serde_json::from_str(body).unwrap()
}

#[test]
fn a_query_page_is_the_basic_post_with_the_scoped_jql() {
    let query = scoped("assignee = currentUser() ORDER BY priority DESC");
    let (origin, rx) = serve(Some(ok(PAGE)));
    JiraCloud::new()
        .query_page(&connection(&origin), &token(), &query, Some("cursor-1"))
        .expect("a page");
    let raw = rx.recv().unwrap();
    let (head, body) = raw.split_once("\r\n\r\n").unwrap();
    assert!(
        head.starts_with("POST /rest/api/3/search/jql HTTP/1.1"),
        "{head}"
    );
    assert_only_the_four_headers(head);
    assert!(!body.contains(SENTINEL));
    let json: serde_json::Value = serde_json::from_str(body).unwrap();
    let keys: std::collections::BTreeSet<_> = json.as_object().unwrap().keys().cloned().collect();
    let expected: std::collections::BTreeSet<_> = ["jql", "fields", "maxResults", "nextPageToken"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(keys, expected, "the basic body's fields and nothing more");
    assert_eq!(
        json["jql"].as_str(),
        Some("project = \"ABC\" AND (assignee = currentUser()) ORDER BY priority DESC"),
        "the scoped query is the JQL sent, as is (FR-005)"
    );
    assert_eq!(json["maxResults"], 100);
    assert_eq!(json["nextPageToken"], "cursor-1");

    let (origin, rx) = serve(Some(ok(PAGE)));
    JiraCloud::new()
        .list_open(&connection(&origin), &token(), None)
        .expect("a page");
    assert_eq!(
        json["fields"],
        body_of(&rx)["fields"],
        "a query reads the fields a basic listing reads"
    );
}

#[test]
fn a_first_query_page_sends_no_page_token() {
    let (origin, rx) = serve(Some(ok(PAGE)));
    JiraCloud::new()
        .query_page(&connection(&origin), &token(), &scoped("type = Bug"), None)
        .expect("a page");
    assert!(body_of(&rx).get("nextPageToken").is_none());
}

#[test]
fn a_query_page_parses_as_the_basic_listing_does() {
    let (origin, _rx) = serve(Some(ok(PAGE)));
    let page = JiraCloud::new()
        .query_page(&connection(&origin), &token(), &scoped("type = Bug"), None)
        .expect("a page");
    let (origin, _rx) = serve(Some(ok(PAGE)));
    let basic = JiraCloud::new()
        .list_open(&connection(&origin), &token(), None)
        .expect("a page");
    assert_eq!(page, basic);
}

#[test]
fn a_failed_query_is_the_load_failure_basic_mode_gives() {
    for (status, error) in [
        ("401 Unauthorized", IssueLoadError::CredentialsRefused),
        ("429 Too Many Requests", IssueLoadError::RateLimited),
    ] {
        let (origin, _rx) = serve(Some(format!(
            "HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        )));
        assert_eq!(
            JiraCloud::new().query_page(&connection(&origin), &token(), &scoped("x = 1"), None),
            Err(QueryFailure::Load(error)),
            "{status}"
        );
    }
    let (origin, _rx) = serve(Some(ok("<html>log in</html>")));
    assert_eq!(
        JiraCloud::new().query_page(&connection(&origin), &token(), &scoped("x = 1"), None),
        Err(QueryFailure::Load(IssueLoadError::NotJiraCloud))
    );
}

#[test]
fn a_stalled_query_times_out_at_the_budget() {
    let (origin, _rx) = serve(None);
    let started = std::time::Instant::now();
    let failure = JiraCloud::with_timeout(Duration::from_millis(300))
        .query_page(&connection(&origin), &token(), &scoped("x = 1"), None)
        .unwrap_err();
    assert_eq!(failure, QueryFailure::Load(IssueLoadError::TimedOut));
    assert!(started.elapsed() < Duration::from_secs(3));
}

// Feature 1186 M4: a query Jira refuses (contracts/jql-rules.md §4, FR-010).

fn bad_request_with(body: &str, content_type: &str) -> String {
    format!(
        "HTTP/1.1 400 Bad Request\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

#[test]
fn a_refused_query_carries_jiras_messages_in_jiras_order() {
    let body = r#"{"errorMessages":["Field 'asignee' does not exist or you do not have permission to view it.","The value 'Hgh' does not exist for the field 'priority'."],"errors":{}}"#;
    let (origin, _rx) = serve(Some(bad_request_with(body, "application/json")));
    assert_eq!(
        JiraCloud::new().query_page(
            &connection(&origin),
            &token(),
            &scoped("asignee = me"),
            None
        ),
        Err(QueryFailure::Rejected(vec![
            "Field 'asignee' does not exist or you do not have permission to view it.".into(),
            "The value 'Hgh' does not exist for the field 'priority'.".into(),
        ]))
    );
}

#[test]
fn a_refused_query_with_no_message_is_rejected_with_none() {
    for body in ["<html><body>Bad request</body></html>", ""] {
        let (origin, _rx) = serve(Some(bad_request_with(body, "text/html")));
        assert_eq!(
            JiraCloud::new().query_page(&connection(&origin), &token(), &scoped("x = 1"), None),
            Err(QueryFailure::Rejected(Vec::new())),
            "{body:?}"
        );
    }
}

#[test]
fn a_400_with_a_body_still_fails_a_basic_list_as_before() {
    let body = r#"{"errorMessages":["The value 'ABC' does not exist for the field 'project'."]}"#;
    let (origin, _rx) = serve(Some(bad_request_with(body, "application/json")));
    assert_eq!(
        JiraCloud::new().list_open(&connection(&origin), &token(), None),
        Err(micold_core::jira::classify(
            &micold_core::jira::Outcome::Status(400)
        ))
    );
}

#[test]
fn a_400_with_a_body_still_bisects_a_status_chunk() {
    let refusal = bad_request_with(
        r#"{"errorMessages":["Issue 'ABC-2' does not exist."]}"#,
        "application/json",
    );
    let (origin, _rx) = serve_each(vec![
        refusal.clone(),                        // [1,2]
        ok(&status_body(&[("ABC-1", "Done")])), // [1]
        refusal,                                // [2] alone
    ]);
    let keys: Vec<String> = (1..=2).map(|n| format!("ABC-{n}")).collect();
    let got = JiraCloud::new()
        .statuses(&connection(&origin), &token(), &keys)
        .expect("statuses");
    let names: Vec<&str> = got.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(names, ["ABC-1"]);
}
