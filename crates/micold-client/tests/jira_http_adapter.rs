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
use micold_core::jira::{ApiToken, JiraConnection, JiraSource};

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
            None => thread::sleep(Duration::from_secs(3)),
        }
    });
    (origin, rx)
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
        micold_core::jira::search_jql(&connection(&origin).project_key, "login \"x\"").as_deref()
    );
    assert!(!body.contains(SENTINEL));
}

#[test]
fn an_empty_search_sends_nothing() {
    let found = JiraCloud::new().search_open(&connection("http://127.0.0.1:9"), &token(), "  ");
    assert_eq!(found, Ok(Vec::new()));
}

#[test]
fn a_redirect_is_not_followed() {
    let (origin, _rx) = serve(Some(
        "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/elsewhere\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string(),
    ));
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
    // Nothing listens on port 1: a refused connection.
    let conn = connection("http://127.0.0.1:1");
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
