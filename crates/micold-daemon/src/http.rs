//! Bounded HTTP/1.1 request handling for the daemon's loopback listeners.
//!
//! Two listeners speak HTTP: the hook receiver (`hooks.rs`, one bounded `POST` per connection) and
//! the tool server (`mcp::server`, feature 034). Both need the same small, hostile-input-safe subset:
//! read a head bounded to [`MAX_HEAD`], parse its request line and the two headers they care about,
//! read a body bounded by a limit each caller chooses, drain an over-bound body before refusing it,
//! and write one response then close. That subset lives here once so the two cannot drift.
//!
//! Nothing in this module logs: a body can carry transcript paths, prompts or credentials.

use std::io;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Maximum bytes read for the request head (request line + headers). A real request's head is a few
/// hundred bytes; this is generous but bounds a hostile/looping sender.
pub const MAX_HEAD: usize = 8 * 1024;

/// How much of an over-bound body to read and discard before answering `413`, so the peer's write
/// completes and it can actually read the status line instead of hitting `ECONNRESET` (BUG-010).
/// Draining is not buffering: bytes are discarded a chunk at a time, so this costs O(chunk) memory
/// no matter how much arrives. The cap stops a sender that would keep writing forever.
const MAX_DRAIN: usize = 8 * 1024 * 1024;

/// How long the drain above may wait. A byte cap alone is not enough: a peer that declares a large
/// `Content-Length` and then sends nothing (without closing its write half) would park the handler
/// forever. Over loopback a legitimate sender pushes megabytes in milliseconds, so a short deadline
/// costs a real request nothing and bounds a stalled one.
const DRAIN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

/// A parsed HTTP request head (request line + the headers we care about).
#[derive(Debug, PartialEq, Eq)]
pub struct Head {
    /// The request method, as sent (`POST`, `GET`, …).
    pub method: String,
    /// The request target, query string included.
    pub path: String,
    /// The declared `Content-Length`, `0` when absent or unparseable.
    pub content_length: usize,
    /// The token of an `Authorization: Bearer <token>` header, if one was sent in that form.
    pub bearer: Option<String>,
}

/// The outcome of reading a request head.
#[derive(Debug)]
pub enum HeadRead {
    /// A complete head, and the bytes read past it (the start of the body).
    Complete { head: Head, rest: Vec<u8> },
    /// No blank line within [`MAX_HEAD`] bytes: answer `431`.
    TooLarge,
    /// The request line is malformed: answer `400`.
    Malformed,
    /// The peer closed before a complete head: nothing to answer.
    Closed,
}

/// The outcome of reading a request body under a caller-chosen limit.
#[derive(Debug, PartialEq, Eq)]
pub enum Body {
    /// The whole body, exactly `Content-Length` bytes (fewer if the peer closed early).
    Complete(Vec<u8>),
    /// The declared or received body exceeds the limit. It has been drained; answer `413`.
    TooLarge,
}

/// Read a request head, bounded to [`MAX_HEAD`] bytes, up to the blank line that terminates it.
pub async fn read_head<S: AsyncRead + Unpin>(stream: &mut S) -> io::Result<HeadRead> {
    let mut buf = Vec::with_capacity(1024);
    let head_end = loop {
        if let Some(pos) = find_head_end(&buf) {
            break pos;
        }
        if buf.len() >= MAX_HEAD {
            return Ok(HeadRead::TooLarge);
        }
        let mut chunk = [0u8; 1024];
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            return Ok(HeadRead::Closed);
        }
        buf.extend_from_slice(&chunk[..n]);
    };
    match parse_head(&buf[..head_end]) {
        Some(head) => Ok(HeadRead::Complete {
            head,
            rest: buf[head_end..].to_vec(),
        }),
        None => Ok(HeadRead::Malformed),
    }
}

/// Read a body of `content_length` bytes, `already` holding the part the head read pulled in. A
/// declared length over `limit` is drained and refused before a byte is buffered; a peer that sends
/// more than `limit` despite its declaration is refused as soon as it crosses it.
pub async fn read_body<S: AsyncRead + Unpin>(
    stream: &mut S,
    already: Vec<u8>,
    content_length: usize,
    limit: usize,
) -> io::Result<Body> {
    if content_length > limit {
        // Drain first: closing mid-body costs the peer the response entirely (it sees ECONNRESET on
        // its remaining write), so it cannot report *why* the request failed.
        drain(stream, already.len()).await;
        return Ok(Body::TooLarge);
    }
    let mut body = already;
    while body.len() < content_length {
        let mut chunk = [0u8; 1024];
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
        if body.len() > limit {
            return Ok(Body::TooLarge);
        }
    }
    body.truncate(content_length);
    Ok(Body::Complete(body))
}

/// Read and discard up to [`MAX_DRAIN`] bytes of a body we are about to refuse, so the peer's write
/// can complete and it reads our status rather than an `ECONNRESET` (BUG-010). `already` counts the
/// body bytes the head read pulled in. Best-effort: a read error or a sender that exceeds the cap
/// just means we answer and close. Bytes are dropped as they arrive — never accumulated, never
/// logged. Bounded by both [`MAX_DRAIN`] bytes and [`DRAIN_TIMEOUT`]; whichever comes first.
pub async fn drain<S: AsyncRead + Unpin>(stream: &mut S, already: usize) {
    let mut discarded = already;
    let mut sink = [0u8; 8 * 1024];
    let _ = tokio::time::timeout(DRAIN_TIMEOUT, async {
        while discarded < MAX_DRAIN {
            match stream.read(&mut sink).await {
                Ok(0) | Err(_) => return,
                Ok(n) => discarded += n,
            }
        }
    })
    .await;
}

/// Read off and discard the rest of a declared body of `content_length` bytes, `already` of which
/// the head read pulled in, before refusing a request on its head alone (`401`, `404`, `405`). The
/// peer's write then completes and it reads the status rather than an `ECONNRESET`. Never reads past
/// the declared length; bounded by [`MAX_DRAIN`] bytes and [`DRAIN_TIMEOUT`] like [`drain`].
pub async fn discard_body<S: AsyncRead + Unpin>(
    stream: &mut S,
    already: usize,
    content_length: usize,
) {
    let mut remaining = content_length.min(MAX_DRAIN).saturating_sub(already);
    let mut sink = [0u8; 8 * 1024];
    let _ = tokio::time::timeout(DRAIN_TIMEOUT, async {
        while remaining > 0 {
            let want = remaining.min(sink.len());
            match stream.read(&mut sink[..want]).await {
                Ok(0) | Err(_) => return,
                Ok(n) => remaining -= n,
            }
        }
    })
    .await;
}

/// Write a minimal HTTP/1.1 response with an empty body and close-after semantics.
pub async fn respond<S: AsyncWrite + Unpin>(
    stream: &mut S,
    status: u16,
    reason: &str,
) -> io::Result<()> {
    let head =
        format!("HTTP/1.1 {status} {reason}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    stream.write_all(head.as_bytes()).await?;
    stream.flush().await
}

/// Write an HTTP/1.1 response carrying `body` as `application/json`, with close-after semantics.
pub async fn respond_json<S: AsyncWrite + Unpin>(
    stream: &mut S,
    status: u16,
    reason: &str,
    body: &[u8],
) -> io::Result<()> {
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.flush().await
}

/// The index just past the `\r\n\r\n` that ends the request head, if present.
pub fn find_head_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n").map(|p| p + 4)
}

/// Parse the request line + the `Content-Length` and `Authorization: Bearer` headers from a request
/// head. Header names are matched case-insensitively (HTTP allows any casing). Returns `None` if the
/// request line is malformed.
pub fn parse_head(head: &[u8]) -> Option<Head> {
    let text = std::str::from_utf8(head).ok()?;
    let mut lines = text.split("\r\n");
    let request_line = lines.next()?;
    let mut parts = request_line.split(' ');
    let method = parts.next()?.to_string();
    let path = parts.next()?.to_string();
    // A third field (HTTP version) must exist for a well-formed request line.
    parts.next()?;
    if method.is_empty() || path.is_empty() {
        return None;
    }

    let mut content_length = 0usize;
    let mut bearer = None;
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim();
        match name.as_str() {
            "content-length" => content_length = value.parse().unwrap_or(0),
            "authorization" => {
                bearer = value
                    .strip_prefix("Bearer ")
                    .or_else(|| value.strip_prefix("bearer "))
                    .map(|t| t.trim().to_string());
            }
            _ => {}
        }
    }
    Some(Head {
        method,
        path,
        content_length,
        bearer,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A caller-chosen body limit small enough to test at its exact boundary.
    const LIMIT: usize = 16;

    #[test]
    fn parses_a_well_formed_hook_request_head() {
        let head = b"POST /hook/abc HTTP/1.1\r\nHost: x\r\nContent-Length: 42\r\nAuthorization: Bearer secret-token\r\n\r\n";
        let end = find_head_end(head).unwrap();
        let parsed = parse_head(&head[..end]).unwrap();
        assert_eq!(parsed.method, "POST");
        assert_eq!(parsed.path, "/hook/abc");
        assert_eq!(parsed.content_length, 42);
        assert_eq!(parsed.bearer.as_deref(), Some("secret-token"));
    }

    #[test]
    fn header_names_are_case_insensitive() {
        let head =
            b"POST /hook/x HTTP/1.1\r\ncontent-length: 7\r\nAUTHORIZATION: bearer tok\r\n\r\n";
        let parsed = parse_head(head).unwrap();
        assert_eq!(parsed.content_length, 7);
        assert_eq!(parsed.bearer.as_deref(), Some("tok"));
    }

    #[test]
    fn a_malformed_request_line_is_rejected() {
        assert!(parse_head(b"GARBAGE\r\n\r\n").is_none());
        assert!(parse_head(b"POST\r\n\r\n").is_none());
    }

    #[test]
    fn find_head_end_needs_the_blank_line() {
        assert_eq!(find_head_end(b"POST / HTTP/1.1\r\n"), None);
        assert_eq!(find_head_end(b"POST / HTTP/1.1\r\n\r\n"), Some(19));
    }

    #[tokio::test]
    async fn a_body_exactly_at_the_limit_is_read_in_full() {
        let body = vec![b'x'; LIMIT];
        // The head read already pulled in the first 4 bytes; the rest arrives on the stream.
        let mut stream: &[u8] = &body[4..];
        let read = read_body(&mut stream, body[..4].to_vec(), LIMIT, LIMIT)
            .await
            .unwrap();
        assert_eq!(read, Body::Complete(body));
    }

    #[tokio::test]
    async fn a_body_one_byte_over_the_limit_is_refused_and_drained() {
        let body = vec![b'x'; LIMIT + 1];
        let mut stream: &[u8] = &body;
        let read = read_body(&mut stream, Vec::new(), LIMIT + 1, LIMIT)
            .await
            .unwrap();
        assert_eq!(read, Body::TooLarge);
        assert!(stream.is_empty(), "the refused body must be drained");
    }

    #[tokio::test]
    async fn a_refused_requests_declared_body_is_read_off_and_nothing_past_it() {
        // 3 body bytes arrived with the head; 5 more are declared; 2 bytes follow the body.
        let mut stream: &[u8] = b"45678NEXT";
        discard_body(&mut stream, 3, 8).await;
        assert_eq!(stream, b"NEXT");
    }
}
