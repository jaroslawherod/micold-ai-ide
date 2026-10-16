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

use std::io::{BufRead, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const PAGE: &str = include_str!("../../micold-core/tests/fixtures/jira/search_page.json");
const LOGIN_PAGE: &str = include_str!("../../micold-core/tests/fixtures/jira/html_login_page.html");

const OK: u8 = 0;
const REFUSE: u8 = 1;
const SILENT: u8 = 2;
const HTML: u8 = 3;

fn respond(mut stream: TcpStream, mode: u8) {
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
    let (status, content_type, body) = match mode {
        SILENT => {
            thread::sleep(Duration::from_secs(60));
            return;
        }
        REFUSE => ("401 Unauthorized", "application/json", "{}"),
        HTML => ("200 OK", "text/html", LOGIN_PAGE),
        _ => ("200 OK", "application/json", PAGE),
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
    println!("fake Jira on http://127.0.0.1:{port} - modes: ok, refuse, silent, html");
    let mode = Arc::new(AtomicU8::new(OK));
    let switch = Arc::clone(&mode);
    thread::spawn(move || {
        for line in std::io::stdin().lock().lines().map_while(Result::ok) {
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
        thread::spawn(move || respond(stream, mode));
    }
}
