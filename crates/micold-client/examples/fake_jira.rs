//! A development-only fake Jira Cloud site for the manual pass (feature 533, quickstart §B).
//!
//! ```text
//! cargo run -p micold-client --example fake_jira -- [port]
//! ```
//!
//! Serves the captured search page at `POST /rest/api/3/search/jql` on `127.0.0.1:<port>`
//! (default 8099). Connect a project to `http://127.0.0.1:<port>` (a debug build accepts loopback
//! `http`) with any email and token. Type a mode on stdin to switch it: `ok` answers the page,
//! `refuse` answers 401, `silent` never answers, `html` answers a login page. Stop the process to
//! see the unreachable notice, start it again to retry.
//!
//! A search whose JQL holds `key in (...)` (feature 1101, the linked tickets' statuses) is answered
//! with those keys' statuses, `To Do` unless a script changed them. `status ABC-1 done Done`
//! (key, a category of `new`, `indeterminate` or `done`, then the name) changes one, which the
//! next refresh shows. A key listed as `missing ABC-9` is not returned, as Jira does for a ticket
//! it cannot show.

use std::io::{BufRead, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU8, Ordering};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const PAGE: &str = include_str!("../../micold-core/tests/fixtures/jira/search_page.json");
const LOGIN_PAGE: &str = include_str!("../../micold-core/tests/fixtures/jira/html_login_page.html");

const OK: u8 = 0;
const REFUSE: u8 = 1;
const SILENT: u8 = 2;
const HTML: u8 = 3;

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
    let status_page = status_answer(request_body, &tickets.lock().expect("tickets"));
    let (status, content_type, body) = match mode {
        SILENT => {
            thread::sleep(Duration::from_secs(60));
            return;
        }
        REFUSE => ("401 Unauthorized", "application/json", "{}"),
        HTML => ("200 OK", "text/html", LOGIN_PAGE),
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
    println!("fake Jira on http://127.0.0.1:{port} - modes: ok, refuse, silent, html; status KEY CATEGORY NAME; missing KEY");
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
