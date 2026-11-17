//! The orchestrator's ops listener (#1981, FR-006): `GET /healthz` answers whenever the process
//! does, `GET /readyz` once the registry is loaded and the link listener is bound. Bound only when
//! `--ops-addr` is given. Hand-written HTTP/1.1, one request per connection, plain text,
//! `Connection: close` (contracts/ops-http.md); no `/metrics` (#1742).

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::{TcpListener, TcpStream};

/// What `/readyz` waits for.
#[derive(Debug, Default)]
pub struct Readiness {
    registry_loaded: AtomicBool,
    link_bound: AtomicBool,
}

impl Readiness {
    /// The registry has been loaded from the store.
    pub fn set_registry_loaded(&self) {
        self.registry_loaded.store(true, Ordering::Release);
    }

    /// The link listener is bound.
    pub fn set_link_bound(&self) {
        self.link_bound.store(true, Ordering::Release);
    }

    /// Both are done.
    pub fn is_ready(&self) -> bool {
        self.registry_loaded.load(Ordering::Acquire) && self.link_bound.load(Ordering::Acquire)
    }
}

/// The longest request head read; a longer one is answered 400.
const MAX_HEAD: usize = 8 * 1024;
/// How long a peer may take to send its request head.
const READ_TIMEOUT: Duration = Duration::from_secs(5);

/// Serve `listener` until the future is dropped; each connection runs on its own task.
pub async fn serve_ops(listener: TcpListener, readiness: Arc<Readiness>) -> io::Result<()> {
    serve_ops_with(listener, readiness, READ_TIMEOUT).await
}

/// [`serve_ops`] with `read_timeout` for the request head (tests shorten it).
async fn serve_ops_with(
    listener: TcpListener,
    readiness: Arc<Readiness>,
    read_timeout: Duration,
) -> io::Result<()> {
    loop {
        let (stream, _) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                tracing::warn!(error = %e, "ops accept failed");
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }
        };
        let readiness = Arc::clone(&readiness);
        tokio::spawn(async move {
            let _ = answer(stream, &readiness, read_timeout).await;
        });
    }
}

async fn answer(
    mut stream: TcpStream,
    readiness: &Readiness,
    read_timeout: Duration,
) -> io::Result<()> {
    let (status, body) = match tokio::time::timeout(read_timeout, read_head(&mut stream)).await {
        Err(_) => ("408 Request Timeout", "request timeout"),
        Ok(Ok(Some(line))) => respond(&line, readiness),
        Ok(Ok(None)) => ("400 Bad Request", "bad request"),
        Ok(Err(e)) => return Err(e),
    };
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).await?;
    stream.shutdown().await
}

/// The request line, once the whole head has arrived; `None` for a head too long or not UTF-8.
async fn read_head(stream: &mut TcpStream) -> io::Result<Option<String>> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 1024];
    while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            break;
        }
        if buf.len() + n > MAX_HEAD {
            return Ok(None);
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    let Ok(text) = std::str::from_utf8(&buf) else {
        return Ok(None);
    };
    Ok(text.lines().next().map(str::to_owned))
}

/// The status and body for `request_line`.
fn respond(request_line: &str, readiness: &Readiness) -> (&'static str, &'static str) {
    let mut parts = request_line.split_whitespace();
    let (Some(method), Some(path)) = (parts.next(), parts.next()) else {
        return ("400 Bad Request", "bad request");
    };
    let path = path.split('?').next().unwrap_or(path);
    match (method, path) {
        ("GET", "/healthz") => ("200 OK", "ok"),
        ("GET", "/readyz") if readiness.is_ready() => ("200 OK", "ready"),
        ("GET", "/readyz") => ("503 Service Unavailable", "not ready"),
        (_, "/healthz" | "/readyz") => ("405 Method Not Allowed", "method not allowed"),
        _ => ("404 Not Found", "not found"),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Send `request` raw to `addr`; the whole response as text.
    pub(crate) async fn raw(addr: std::net::SocketAddr, request: &str) -> String {
        let mut stream = TcpStream::connect(addr).await.unwrap();
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut out = String::new();
        stream.read_to_string(&mut out).await.unwrap();
        out
    }

    pub(crate) async fn get(addr: std::net::SocketAddr, path: &str) -> String {
        raw(addr, &format!("GET {path} HTTP/1.1\r\nHost: x\r\n\r\n")).await
    }

    async fn started_with(read_timeout: Duration) -> (std::net::SocketAddr, Arc<Readiness>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let readiness = Arc::new(Readiness::default());
        tokio::spawn(serve_ops_with(
            listener,
            Arc::clone(&readiness),
            read_timeout,
        ));
        (addr, readiness)
    }

    async fn started() -> (std::net::SocketAddr, Arc<Readiness>) {
        started_with(READ_TIMEOUT).await
    }

    /// A head past the limit is 400; exactly one byte over, so the server has read everything
    /// sent and closes cleanly.
    #[tokio::test]
    async fn an_oversized_head_is_refused() {
        let (addr, _readiness) = started().await;
        let long = format!("GET /{}", "a".repeat(MAX_HEAD - 4));
        assert_eq!(long.len(), MAX_HEAD + 1);
        let response = raw(addr, &long).await;
        assert!(response.starts_with("HTTP/1.1 400 "), "{response}");
    }

    /// A peer that never finishes its head is answered 408 and closed.
    #[tokio::test]
    async fn a_slow_head_times_out_with_408() {
        let (addr, _readiness) = started_with(Duration::from_millis(100)).await;
        let response = raw(addr, "GET /healthz HTTP/1.1\r\n").await;
        assert!(response.starts_with("HTTP/1.1 408 "), "{response}");
        assert!(response.contains("Connection: close"), "{response}");
    }

    #[tokio::test]
    async fn healthz_answers_at_once() {
        let (addr, _readiness) = started().await;
        let response = get(addr, "/healthz").await;
        assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
        assert!(response.contains("\r\nConnection: close\r\n"), "{response}");
        assert!(response.ends_with("\r\n\r\nok"), "{response}");
    }

    #[tokio::test]
    async fn readyz_waits_for_both_flags() {
        let (addr, readiness) = started().await;
        let not_ready = get(addr, "/readyz").await;
        assert!(not_ready.starts_with("HTTP/1.1 503 "), "{not_ready}");
        assert!(not_ready.ends_with("not ready"), "{not_ready}");
        readiness.set_registry_loaded();
        assert!(get(addr, "/readyz").await.starts_with("HTTP/1.1 503 "));
        readiness.set_link_bound();
        let ready = get(addr, "/readyz").await;
        assert!(ready.starts_with("HTTP/1.1 200 OK\r\n"), "{ready}");
        assert!(ready.ends_with("\r\n\r\nready"), "{ready}");
    }

    #[tokio::test]
    async fn readyz_needs_the_registry_too() {
        let (addr, readiness) = started().await;
        readiness.set_link_bound();
        assert!(get(addr, "/readyz?x=1").await.starts_with("HTTP/1.1 503 "));
    }

    #[tokio::test]
    async fn other_paths_and_methods_and_garbage() {
        let (addr, _readiness) = started().await;
        assert!(get(addr, "/metrics").await.starts_with("HTTP/1.1 404 "));
        let post = raw(addr, "POST /healthz HTTP/1.1\r\n\r\n").await;
        assert!(post.starts_with("HTTP/1.1 405 "), "{post}");
        assert!(post.contains("Connection: close"), "{post}");
        assert!(raw(addr, "\r\n\r\n").await.starts_with("HTTP/1.1 400 "));
    }
}
