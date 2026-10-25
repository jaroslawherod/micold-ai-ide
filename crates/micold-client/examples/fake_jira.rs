//! A development-only fake Jira Cloud site for the manual pass (feature 533, quickstart §B).
//!
//! ```text
//! cargo run -p micold-client --example fake_jira -- [port]
//! ```
//!
//! Serves the captured search page at `POST /rest/api/3/search/jql` on `127.0.0.1:<port>`
//! (default 8099). Connect a project to `http://127.0.0.1:<port>` (a debug build accepts loopback
//! `http`) with any email and token. Type a mode on stdin to switch it: `ok` answers the page,
//! `refuse` answers 401, `silent` never answers, `html` answers a login page, `badjql` answers
//! `400` with Jira's message for a mistyped field (feature 1186, quickstart §B B5). Stop the
//! process to see the unreachable notice, start it again to retry. In `ok` mode each request's
//! `jql` is printed, so a step can check the scoped query an advanced run sent.
//!
//! A search whose JQL holds `key in (...)` (feature 1101, the linked tickets' statuses) is answered
//! with those keys' statuses, `To Do` unless a script changed them. `status ABC-1 done Done`
//! (key, a category of `new`, `indeterminate` or `done`, then the name) changes one, which the
//! next refresh shows. A key listed as `missing ABC-9` is not returned, as Jira does for a ticket
//! it cannot show.
//!
//! Feature 1185 (quickstart Part B): `GET /rest/api/3/project/{key}/components` answers the
//! components `Payments`, `Auth` and `Ä "quoted" \ name`, and a search whose JQL holds a
//! `component = "…"` clause is answered with that component's share of the page: every third
//! issue, starting at the component's place in that list; an unknown component has none.
//! `GET /rest/api/3/project/{key}/versions` answers `1.0` (archived), `2.3` (released) and `2.4`
//! (unreleased), and a `fixVersion = "…"` clause keeps every second issue, starting at the
//! version's place among the unarchived ones (`2.3`, `2.4`); `1.0` has none. Both clauses AND.

use std::collections::BTreeMap;
use std::io::{BufRead, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const PAGE: &str = include_str!("../../micold-core/tests/fixtures/jira/search_page.json");
const LOGIN_PAGE: &str = include_str!("../../micold-core/tests/fixtures/jira/html_login_page.html");

const OK: u8 = 0;
const REFUSE: u8 = 1;
const SILENT: u8 = 2;
const HTML: u8 = 3;
const BAD_JQL: u8 = 4;

/// What `badjql` answers: Jira's refusal of a query naming a field that does not exist.
const BAD_JQL_BODY: &str = r#"{"errorMessages":["Field 'asignee' does not exist or you do not have permission to view it."]}"#;

/// The statuses a script set: key to `(category key, name)`; and keys Jira "cannot show".
#[derive(Default)]
struct Tickets {
    statuses: BTreeMap<String, (String, String)>,
    missing: Vec<String>,
}

/// The answer to a `key in (...)` search, or `None` when the JQL is not one.
fn status_answer(body: &str, tickets: &Tickets) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(body).ok()?;
    let jql = json["jql"].as_str()?;
    let list = jql.split(" key in (").nth(1)?.split(')').next()?;
    let issues: Vec<serde_json::Value> = list
        .split(',')
        .map(|k| k.trim().trim_matches('"').to_string())
        .filter(|k| !k.is_empty() && !tickets.missing.contains(k))
        .map(|key| {
            let (category, name) = tickets
                .statuses
                .get(&key)
                .cloned()
                .unwrap_or_else(|| ("new".to_string(), "To Do".to_string()));
            serde_json::json!({
                "key": key,
                "fields": {"status": {"name": name, "statusCategory": {"key": category}}}
            })
        })
        .collect();
    Some(serde_json::json!({ "issues": issues, "isLast": true }).to_string())
}

/// The components the fake project has, in Jira's order (feature 1185).
const COMPONENTS: [&str; 3] = ["Payments", "Auth", "Ä \"quoted\" \\ name"];

/// The fix versions the fake project has, in Jira's order, as `(name, archived, released)`.
const VERSIONS: [(&str, bool, bool); 3] = [
    ("1.0", true, true),
    ("2.3", false, true),
    ("2.4", false, false),
];

/// The value of the JQL's `<field> = "…"` clause (`prefix` is `component = "`), unescaped, if it
/// has one.
fn clause_value(jql: &str, prefix: &str) -> Option<String> {
    let rest = jql.split_once(prefix)?.1;
    let mut value = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => value.push(chars.next()?),
            '"' => return Some(value),
            other => value.push(other),
        }
    }
    None
}

/// Whether issue `index` belongs to `value`'s share of `names` (every `names.len()`th issue,
/// starting at the value's place), or `true` when the JQL names no value; an unknown value has
/// none.
fn in_share(index: usize, value: Option<&str>, names: &[&str]) -> bool {
    value.is_none_or(|value| {
        names
            .iter()
            .position(|name| *name == value)
            .is_some_and(|place| index % names.len() == place)
    })
}

/// The page narrowed to the component's and the fix version's shares, when the request's JQL
/// names either (an archived version has no share).
fn filtered_answer(body: &str) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(body).ok()?;
    let jql = json["jql"].as_str()?;
    let component = clause_value(jql, "component = \"");
    let version = clause_value(jql, "fixVersion = \"");
    if component.is_none() && version.is_none() {
        return None;
    }
    let live: Vec<&str> = VERSIONS
        .iter()
        .filter(|(_, archived, _)| !archived)
        .map(|(name, _, _)| *name)
        .collect();
    let mut page: serde_json::Value = serde_json::from_str(PAGE).ok()?;
    let kept: Vec<serde_json::Value> = page["issues"]
        .as_array()?
        .iter()
        .enumerate()
        .filter(|(i, _)| {
            in_share(*i, component.as_deref(), &COMPONENTS)
                && in_share(*i, version.as_deref(), &live)
        })
        .map(|(_, issue)| issue.clone())
        .collect();
    page["issues"] = serde_json::Value::from(kept);
    page["isLast"] = serde_json::Value::from(true);
    Some(page.to_string())
}

fn respond(mut stream: TcpStream, mode: u8, tickets: &Mutex<Tickets>) {
    let mut seen = Vec::new();
    let mut chunk = [0u8; 4096];
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    // The request is read only far enough to know it is whole: headers, then Content-Length bytes.
    loop {
        let n = stream.read(&mut chunk).unwrap_or(0);
        seen.extend_from_slice(&chunk[..n]);
        let text = String::from_utf8_lossy(&seen).to_string();
        if let Some((head, body)) = text.split_once("\r\n\r\n") {
            let length = head
                .lines()
                .find_map(|l| {
                    l.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .and_then(|v| v.trim().parse::<usize>().ok())
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
    let request = String::from_utf8_lossy(&seen).to_string();
    let request_body = request.split_once("\r\n\r\n").map_or("", |(_, b)| b);
    let status_page = status_answer(request_body, &tickets.lock().expect("tickets"))
        .or_else(|| filtered_answer(request_body));
    let components = request.starts_with("GET ") && request.contains("/components ");
    let versions = request.starts_with("GET ") && request.contains("/versions ");
    let version_list = serde_json::Value::from(
        VERSIONS
            .iter()
            .map(|(name, archived, released)| {
                serde_json::json!({ "name": name, "archived": archived, "released": released })
            })
            .collect::<Vec<_>>(),
    )
    .to_string();
    let component_list = serde_json::Value::from(
        COMPONENTS
            .iter()
            .map(|name| serde_json::json!({ "name": name }))
            .collect::<Vec<_>>(),
    )
    .to_string();
    if mode == OK {
        if let Some(jql) = serde_json::from_str::<serde_json::Value>(request_body)
            .ok()
            .and_then(|json| json["jql"].as_str().map(str::to_string))
        {
            println!("jql: {jql}");
        }
    }
    let (status, content_type, body) = match mode {
        SILENT => {
            thread::sleep(Duration::from_secs(60));
            return;
        }
        REFUSE => ("401 Unauthorized", "application/json", "{}"),
        HTML => ("200 OK", "text/html", LOGIN_PAGE),
        BAD_JQL => ("400 Bad Request", "application/json", BAD_JQL_BODY),
        _ if components => ("200 OK", "application/json", component_list.as_str()),
        _ if versions => ("200 OK", "application/json", version_list.as_str()),
        _ => match &status_page {
            Some(page) => ("200 OK", "application/json", page.as_str()),
            None => ("200 OK", "application/json", PAGE),
        },
    };
    let _ = write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
}

fn main() {
    let port = std::env::args()
        .nth(1)
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8099);
    let listener = TcpListener::bind(("127.0.0.1", port)).expect("bind the fake Jira port");
    println!("fake Jira on http://127.0.0.1:{port} - modes: ok, refuse, silent, html, badjql; status KEY CATEGORY NAME; missing KEY");
    let mode = Arc::new(AtomicU8::new(OK));
    let switch = Arc::clone(&mode);
    let tickets = Arc::new(Mutex::new(Tickets::default()));
    let script = Arc::clone(&tickets);
    thread::spawn(move || {
        for line in std::io::stdin().lock().lines().map_while(Result::ok) {
            let words: Vec<&str> = line.split_whitespace().collect();
            match words.as_slice() {
                ["status", key, category, name @ ..] if !name.is_empty() => {
                    script.lock().expect("tickets").statuses.insert(
                        key.to_ascii_uppercase(),
                        (category.to_string(), name.join(" ")),
                    );
                    println!("{key}: {}", name.join(" "));
                    continue;
                }
                ["missing", key] => {
                    script
                        .lock()
                        .expect("tickets")
                        .missing
                        .push(key.to_ascii_uppercase());
                    println!("{key}: not returned");
                    continue;
                }
                _ => {}
            }
            let next = match line.trim() {
                "ok" => OK,
                "refuse" => REFUSE,
                "silent" => SILENT,
                "html" => HTML,
                "badjql" => BAD_JQL,
                other => {
                    println!("unknown mode {other:?}");
                    continue;
                }
            };
            switch.store(next, Ordering::SeqCst);
            println!("mode: {}", line.trim());
        }
    });
    for stream in listener.incoming().flatten() {
        let mode = mode.load(Ordering::SeqCst);
        let tickets = Arc::clone(&tickets);
        thread::spawn(move || respond(stream, mode, &tickets));
    }
}
