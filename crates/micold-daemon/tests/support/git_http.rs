//! A smart-HTTP git remote over `git http-backend` with basic auth `x-access-token:<token>`, used by
//! the loopback test of the helper (`mcp/scoped_git_credential.rs`). The runtime-gated container
//! test that used to share it (`sandbox_real_scoped_token.rs`) was retired with the grant/revoke
//! wire messages it drove (#1530).

#![allow(dead_code)]

use std::path::Path;
use std::process::{Command, Stdio};

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;

pub fn git_http_backend() -> Option<String> {
    let out = Command::new("git").arg("--exec-path").output().ok()?;
    let path = format!(
        "{}/git-http-backend",
        String::from_utf8_lossy(&out.stdout).trim()
    );
    Path::new(&path).exists().then_some(path)
}

/// A server on `bind` speaking smart HTTP over `git http-backend`, one request per connection, with
/// basic auth `x-access-token:<token>`. Serves until the process ends.
pub fn serve(
    bind: &str,
    projects: std::path::PathBuf,
    backend: String,
    token: &'static str,
) -> u16 {
    let listener = TcpListener::bind((bind, 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let (projects, backend) = (projects.clone(), backend.clone());
            std::thread::spawn(move || handle(stream, &projects, &backend, token));
        }
    });
    port
}

fn handle(mut stream: std::net::TcpStream, projects: &Path, backend: &str, token: &str) {
    use base64_lite::encode;
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut first = String::new();
    if reader.read_line(&mut first).is_err() {
        return;
    }
    let mut parts = first.split_whitespace();
    let (method, target) = (
        parts.next().unwrap_or("").to_string(),
        parts.next().unwrap_or("").to_string(),
    );
    let (mut length, mut ctype, mut auth, mut chunked) =
        (0usize, String::new(), String::new(), false);
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 || line.trim().is_empty() {
            break;
        }
        let lower = line.to_ascii_lowercase();
        if let Some(v) = lower.strip_prefix("content-length:") {
            length = v.trim().parse().unwrap_or(0);
        } else if lower.starts_with("content-type:") {
            ctype = line["content-type:".len()..].trim().to_string();
        } else if lower.starts_with("authorization:") {
            auth = line["authorization:".len()..].trim().to_string();
        } else if lower.starts_with("transfer-encoding:") && lower.contains("chunked") {
            chunked = true;
        }
    }
    let mut body = vec![0u8; length];
    if chunked {
        body.clear();
        loop {
            let mut size = String::new();
            reader.read_line(&mut size).unwrap();
            let n = usize::from_str_radix(size.trim(), 16).unwrap_or(0);
            if n == 0 {
                let mut end = String::new();
                let _ = reader.read_line(&mut end);
                break;
            }
            let mut chunk = vec![0u8; n];
            reader.read_exact(&mut chunk).unwrap();
            body.extend(chunk);
            let mut crlf = String::new();
            let _ = reader.read_line(&mut crlf);
        }
    } else {
        reader.read_exact(&mut body).unwrap();
    }
    let want = format!("Basic {}", encode(&format!("x-access-token:{token}")));
    if auth != want {
        let _ = stream.write_all(
            b"HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"git\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        );
        return;
    }
    let (path, query) = target.split_once('?').unwrap_or((&target, ""));
    let mut child = Command::new(backend)
        .env_clear()
        .env("GIT_PROJECT_ROOT", projects)
        .env("GIT_HTTP_EXPORT_ALL", "1")
        .env("REQUEST_METHOD", &method)
        .env("PATH_INFO", path)
        .env("QUERY_STRING", query)
        .env("CONTENT_TYPE", &ctype)
        .env("CONTENT_LENGTH", body.len().to_string())
        .env("REMOTE_USER", "x-access-token")
        .env("REMOTE_ADDR", "127.0.0.1")
        .env("GIT_HTTP_MAX_REQUEST_BUFFER", "100M")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&body).unwrap();
    let out = child.wait_with_output().unwrap().stdout;
    let split = out
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map_or((out.len(), 0), |i| (i, 4));
    let headers = String::from_utf8_lossy(&out[..split.0]).into_owned();
    let status = headers
        .lines()
        .find_map(|l| l.strip_prefix("Status:"))
        .map_or("200 OK".to_string(), |s| s.trim().to_string());
    let payload = &out[(split.0 + split.1).min(out.len())..];
    let mut response = format!("HTTP/1.1 {status}\r\n");
    for line in headers.lines().filter(|l| !l.starts_with("Status:")) {
        response.push_str(line);
        response.push_str("\r\n");
    }
    response.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n",
        payload.len()
    ));
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.write_all(payload);
}

mod base64_lite {
    pub fn encode(text: &str) -> String {
        const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        for chunk in text.as_bytes().chunks(3) {
            let n =
                chunk.iter().fold(0u32, |a, b| a << 8 | u32::from(*b)) << (8 * (3 - chunk.len()));
            for i in 0..=chunk.len() {
                out.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            }
            out.extend(std::iter::repeat_n('=', 3 - chunk.len()));
        }
        out
    }
}
