//! The install relay (feature 806, plan R8, contracts/provisioning.md *Install relay*): an HTTP
//! `CONNECT` proxy on the client's loopback, started for one install command and stopped when it
//! ends. The command carries `ssh -R` to it, so a target with no outbound network can still reach
//! the npm registry and nothing else.
//!
//! It answers `200` only for a `CONNECT host:port` in its allow list and carries the bytes
//! opaquely (TLS stays end to end). Anything else gets an error status and is closed; it never
//! accepts a request that is not `CONNECT`.

use std::io;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::{AbortHandle, JoinSet};

use super::HostPort;

/// The longest request head accepted, and how long a client may take to send it.
const HEAD_LIMIT: usize = 8 * 1024;
const HEAD_TIMEOUT: Duration = Duration::from_secs(10);
/// How long the relay waits to reach the destination.
const UPSTREAM_TIMEOUT: Duration = Duration::from_secs(15);

/// A running relay. Dropping it stops the listener and every connection it carries.
pub struct Relay {
    port: u16,
    task: AbortHandle,
}

impl Relay {
    /// Start a relay on a free loopback port, allowing only `allow`.
    pub async fn start(allow: Vec<HostPort>) -> io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let port = listener.local_addr()?.port();
        let task = tokio::spawn(async move {
            // Dropping the set when this task is aborted aborts every connection with it.
            let mut connections = JoinSet::new();
            loop {
                tokio::select! {
                    accepted = listener.accept() => {
                        let Ok((stream, _)) = accepted else {
                            // E.g. out of file descriptors: do not spin.
                            tokio::time::sleep(Duration::from_millis(50)).await;
                            continue;
                        };
                        let allow = allow.clone();
                        connections.spawn(async move {
                            let _ = serve(stream, &allow).await;
                        });
                    }
                    Some(_) = connections.join_next(), if !connections.is_empty() => {}
                }
            }
        })
        .abort_handle();
        Ok(Self { port, task })
    }

    /// The loopback port the relay listens on.
    pub fn port(&self) -> u16 {
        self.port
    }
}

impl Drop for Relay {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// The destination of a `CONNECT host:port HTTP/1.x` request line, `Err` with the status to answer
/// for anything else.
fn destination(head: &str) -> Result<HostPort, &'static str> {
    let line = head.lines().next().unwrap_or_default();
    let mut words = line.split_whitespace();
    if words.next() != Some("CONNECT") {
        return Err("405 Method Not Allowed");
    }
    let target = words.next().ok_or("400 Bad Request")?;
    let (host, port) = target.rsplit_once(':').ok_or("400 Bad Request")?;
    let port = port.parse().map_err(|_| "400 Bad Request")?;
    Ok(HostPort {
        host: host.to_ascii_lowercase(),
        port,
    })
}

async fn reply(stream: &mut TcpStream, status: &str) -> io::Result<()> {
    stream
        .write_all(format!("HTTP/1.1 {status}\r\nConnection: close\r\n\r\n").as_bytes())
        .await?;
    stream.shutdown().await
}

async fn serve(mut client: TcpStream, allow: &[HostPort]) -> io::Result<()> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 1024];
    let end = loop {
        if let Some(at) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break at + 4;
        }
        if buf.len() > HEAD_LIMIT {
            return reply(&mut client, "431 Request Header Fields Too Large").await;
        }
        let n = tokio::time::timeout(HEAD_TIMEOUT, client.read(&mut chunk))
            .await
            .map_err(|_| io::Error::from(io::ErrorKind::TimedOut))??;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&chunk[..n]);
    };
    let head = String::from_utf8_lossy(&buf[..end]).into_owned();
    let to = match destination(&head) {
        Ok(to) => to,
        Err(status) => return reply(&mut client, status).await,
    };
    if !allow
        .iter()
        .any(|a| a.host.eq_ignore_ascii_case(&to.host) && a.port == to.port)
    {
        return reply(&mut client, "403 Forbidden").await;
    }
    let connected = tokio::time::timeout(
        UPSTREAM_TIMEOUT,
        TcpStream::connect((to.host.as_str(), to.port)),
    )
    .await;
    let Ok(Ok(mut upstream)) = connected else {
        return reply(&mut client, "502 Bad Gateway").await;
    };
    client
        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        .await?;
    // Bytes the client sent after the head (a pipelined TLS hello) belong to the upstream.
    upstream.write_all(&buf[end..]).await?;
    tokio::io::copy_bidirectional(&mut client, &mut upstream).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn upstream_echo() -> (u16, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let task = tokio::spawn(async move {
            loop {
                let (mut s, _) = listener.accept().await.unwrap();
                tokio::spawn(async move {
                    let mut buf = [0u8; 64];
                    while let Ok(n) = s.read(&mut buf).await {
                        if n == 0 || s.write_all(&buf[..n]).await.is_err() {
                            break;
                        }
                    }
                });
            }
        });
        (port, task)
    }

    async fn ask(relay: &Relay, request: &str) -> (String, TcpStream) {
        let mut s = TcpStream::connect(("127.0.0.1", relay.port()))
            .await
            .unwrap();
        s.write_all(request.as_bytes()).await.unwrap();
        let mut head = Vec::new();
        let mut byte = [0u8; 1];
        while !head.ends_with(b"\r\n\r\n") {
            if s.read(&mut byte).await.unwrap() == 0 {
                break;
            }
            head.push(byte[0]);
        }
        (String::from_utf8_lossy(&head).into_owned(), s)
    }

    fn allow(port: u16) -> Vec<HostPort> {
        vec![HostPort {
            host: "127.0.0.1".into(),
            port,
        }]
    }

    #[tokio::test]
    async fn an_allowed_connect_is_answered_200_and_relays_bytes_both_ways() {
        let (up, _server) = upstream_echo().await;
        let relay = Relay::start(allow(up)).await.unwrap();
        let (head, mut s) = ask(&relay, &format!("CONNECT 127.0.0.1:{up} HTTP/1.1\r\n\r\n")).await;
        assert!(head.starts_with("HTTP/1.1 200"), "{head}");
        s.write_all(b"hello").await.unwrap();
        let mut got = [0u8; 5];
        s.read_exact(&mut got).await.unwrap();
        assert_eq!(&got, b"hello");
    }

    #[tokio::test]
    async fn a_connect_to_any_other_destination_is_403() {
        let (up, _server) = upstream_echo().await;
        let relay = Relay::start(allow(up)).await.unwrap();
        for target in ["other.example:443", &format!("127.0.0.1:{}", up + 1)] {
            let (head, _) = ask(&relay, &format!("CONNECT {target} HTTP/1.1\r\n\r\n")).await;
            assert!(head.starts_with("HTTP/1.1 403"), "{target}: {head}");
        }
    }

    #[tokio::test]
    async fn a_request_that_is_not_connect_is_refused_and_closed() {
        let (up, _server) = upstream_echo().await;
        let relay = Relay::start(allow(up)).await.unwrap();
        let (head, mut s) = ask(
            &relay,
            &format!("GET http://127.0.0.1:{up}/ HTTP/1.1\r\n\r\n"),
        )
        .await;
        assert!(head.starts_with("HTTP/1.1 405"), "{head}");
        let mut rest = Vec::new();
        s.read_to_end(&mut rest).await.unwrap();
        assert!(rest.is_empty(), "closed after the refusal");
    }

    #[tokio::test]
    async fn dropping_the_relay_stops_listening() {
        let relay = Relay::start(Vec::new()).await.unwrap();
        let port = relay.port();
        drop(relay);
        for _ in 0..50 {
            if TcpStream::connect(("127.0.0.1", port)).await.is_err() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("the relay still accepts after it was dropped");
    }
}
