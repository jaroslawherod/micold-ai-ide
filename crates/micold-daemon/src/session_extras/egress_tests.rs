//! `Egress::agent` against local listeners (#1536, I16-2): the proxy is used, `no_proxy` skips it.
//! No certificate generator is a dev-dependency, so a TLS handshake against a CA is not driven:
//! the CA file is tested for being tolerated (bad file) and not disturbing the proxy choice.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::Egress;

/// A plain HTTP listener on loopback answering 200 to every request; the count of requests seen.
pub(crate) fn listener() -> (SocketAddr, Arc<AtomicUsize>) {
    let socket = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = socket.local_addr().unwrap();
    let hits = Arc::new(AtomicUsize::new(0));
    let seen = hits.clone();
    std::thread::spawn(move || {
        for stream in socket.incoming().flatten() {
            let mut stream = stream;
            let mut buf = [0u8; 2048];
            let n = stream.read(&mut buf).unwrap_or(0);
            seen.fetch_add(1, Ordering::SeqCst);
            // A proxy is asked to tunnel (`CONNECT`): accept, and answer the request that follows.
            if buf[..n].starts_with(b"CONNECT") {
                let _ = stream.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n");
                let _ = stream.read(&mut buf);
            }
            let _ = stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok");
        }
    });
    (addr, hits)
}

fn get(egress: &Egress, url: &str, host: &str) -> u16 {
    let base = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(5)))
        .http_status_as_error(false);
    egress
        .agent(base, host)
        .get(url)
        .call()
        .map(|r| r.status().as_u16())
        .unwrap_or(0)
}

fn egress(proxy: &str, no_proxy: Option<&str>) -> Egress {
    Egress {
        https_proxy: Some(proxy.into()),
        no_proxy: no_proxy.map(str::to_string),
        ..Default::default()
    }
}

#[test]
fn a_request_through_an_egress_with_a_proxy_reaches_the_proxy() {
    let (proxy, proxied) = listener();
    let (origin, direct) = listener();
    // The stub proxy does not tunnel: only that it was asked (a `CONNECT`) is asserted.
    get(
        &egress(&format!("http://{proxy}"), None),
        &format!("http://{origin}/x"),
        "127.0.0.1",
    );
    assert_eq!(
        (
            proxied.load(Ordering::SeqCst),
            direct.load(Ordering::SeqCst)
        ),
        (1, 0),
        "the request did not go through the proxy"
    );
}

#[test]
fn a_host_in_no_proxy_skips_the_proxy() {
    for no_proxy in ["127.0.0.1", "other,127.0.0.1", "*"] {
        let (proxy, proxied) = listener();
        let (origin, direct) = listener();
        let status = get(
            &egress(&format!("http://{proxy}"), Some(no_proxy)),
            &format!("http://{origin}/x"),
            "127.0.0.1",
        );
        assert_eq!(status, 200, "{no_proxy}");
        assert_eq!(
            (
                proxied.load(Ordering::SeqCst),
                direct.load(Ordering::SeqCst)
            ),
            (0, 1),
            "{no_proxy}: the proxy was used"
        );
    }
}

#[test]
fn an_unlisted_host_still_uses_the_proxy_and_no_proxy_is_not_a_blanket() {
    let (proxy, proxied) = listener();
    let (origin, _) = listener();
    get(
        &egress(&format!("http://{proxy}"), Some("other.corp")),
        &format!("http://{origin}/x"),
        "127.0.0.1",
    );
    assert_eq!(proxied.load(Ordering::SeqCst), 1);
}

#[test]
fn a_lowercase_https_proxy_and_an_empty_value_are_read_from_the_configuration() {
    let dir = tempfile::tempdir().unwrap();
    let text = |value: &str| {
        format!("schema = 1\n[daemon]\nname = \"t\"\nkind = \"host\"\n[session.env]\nhttps_proxy = \"{value}\"\nno_proxy = \"a.corp\"\n")
    };
    let load = |value: &str| {
        let file = dir.path().join("daemon.toml");
        std::fs::write(&file, text(value)).unwrap();
        Egress::load(Some(&file))
    };
    let (proxy, proxied) = listener();
    let (origin, _) = listener();
    let lower = load(&format!("http://{proxy}"));
    assert_eq!(lower.no_proxy.as_deref(), Some("a.corp"));
    get(&lower, &format!("http://{origin}/x"), "127.0.0.1");
    assert_eq!(
        proxied.load(Ordering::SeqCst),
        1,
        "lowercase https_proxy ignored"
    );
    let empty = load("");
    assert_eq!(empty.https_proxy, None);
    get(&empty, &format!("http://{origin}/x"), "127.0.0.1");
    assert_eq!(proxied.load(Ordering::SeqCst), 1, "an empty proxy was used");
}

#[test]
fn an_unreadable_or_invalid_ca_file_leaves_the_agent_working() {
    let dir = tempfile::tempdir().unwrap();
    let junk = dir.path().join("ca.pem");
    std::fs::write(&junk, "not a certificate").unwrap();
    for ca in [junk, dir.path().join("missing.pem")] {
        let (origin, direct) = listener();
        let egress = Egress {
            ca_file: Some(ca),
            ..Default::default()
        };
        assert_eq!(
            get(&egress, &format!("http://{origin}/x"), "127.0.0.1"),
            200
        );
        assert_eq!(direct.load(Ordering::SeqCst), 1);
    }
}
