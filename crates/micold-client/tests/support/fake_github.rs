//! A fake GitHub REST API on loopback (#1542, FR-006, FR-011): plain HTTP on a random port, canned
//! replies per path, every request recorded. Included with `#[path]` by the tests that need it
//! (`scoped_token_fake_api.rs`, and the runtime-gated daemon test), so it holds only `std`, the
//! core's `ApiBase` and no `#[test]`. Every token a test sends is fake.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use micold_core::scoped_credential::pat::{ApiBase, EXPIRATION_HEADER};

/// One request the fake received.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recorded {
    /// The request target as sent: the path and any query.
    pub target: String,
    /// The `Authorization` header, if any.
    pub authorization: Option<String>,
}

#[derive(Clone)]
struct Canned {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

#[derive(Default)]
struct Shared {
    routes: HashMap<String, Canned>,
    seen: Vec<Recorded>,
}

/// The running fake. It serves until the test process ends.
pub struct FakeGithubApi {
    addr: String,
    shared: Arc<Mutex<Shared>>,
}

impl FakeGithubApi {
    /// Start on `127.0.0.1:0`. Every path answers `404` until a route says otherwise.
    pub fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let addr = listener.local_addr().expect("local addr").to_string();
        let shared = Arc::new(Mutex::new(Shared::default()));
        let served = shared.clone();
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let served = served.clone();
                thread::spawn(move || serve(stream, &served));
            }
        });
        Self { addr, shared }
    }

    /// The base a check is pointed at to reach this fake.
    pub fn base(&self) -> ApiBase {
        ApiBase::new(&format!("http://{}", self.addr))
    }

    /// The full URL of `path` on this fake.
    pub fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.addr)
    }

    /// Answer `path` (no query) with `status`, `headers` and `body`.
    pub fn route(&self, path: &str, status: u16, headers: &[(&str, &str)], body: &str) {
        let canned = Canned {
            status,
            headers: headers
                .iter()
                .map(|(n, v)| (n.to_string(), v.to_string()))
                .collect(),
            body: body.to_string(),
        };
        self.shared
            .lock()
            .unwrap()
            .routes
            .insert(path.to_string(), canned);
    }

    /// The repository probe of `owner/name`: `200` with the push permission and, when given, the
    /// token's expiry header (`2030-01-02 03:04:05 UTC` form).
    pub fn probe(&self, repo: &str, push: bool, expiry: Option<&str>) {
        let headers: Vec<(&str, &str)> =
            expiry.map(|e| (EXPIRATION_HEADER, e)).into_iter().collect();
        let body = format!(r#"{{"permissions":{{"push":{push}}}}}"#);
        self.route(&format!("/repos/{repo}"), 200, &headers, &body);
    }

    /// The private-repository listing: the `full_name`s the token reaches, with the expiry header
    /// when given.
    pub fn listing(&self, names: &[&str], expiry: Option<&str>) {
        let headers: Vec<(&str, &str)> =
            expiry.map(|e| (EXPIRATION_HEADER, e)).into_iter().collect();
        let items: Vec<String> = names
            .iter()
            .map(|n| format!(r#"{{"full_name":"{n}"}}"#))
            .collect();
        self.route(
            "/user/repos",
            200,
            &headers,
            &format!("[{}]", items.join(",")),
        );
    }

    /// Everything received so far, oldest first.
    pub fn requests(&self) -> Vec<Recorded> {
        self.shared.lock().unwrap().seen.clone()
    }
}

fn serve(mut stream: TcpStream, shared: &Mutex<Shared>) {
    let mut raw = Vec::new();
    let mut chunk = [0u8; 1024];
    while !raw.windows(4).any(|w| w == b"\r\n\r\n") {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(n) => raw.extend_from_slice(&chunk[..n]),
        }
    }
    let text = String::from_utf8_lossy(&raw);
    let mut lines = text.lines();
    let target = lines
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap_or("/")
        .to_string();
    let authorization = lines.find_map(|l| {
        let (name, value) = l.split_once(':')?;
        name.eq_ignore_ascii_case("authorization")
            .then(|| value.trim().to_string())
    });
    let path = target.split('?').next().unwrap_or("/").to_string();
    let canned = {
        let mut guard = shared.lock().unwrap();
        guard.seen.push(Recorded {
            target,
            authorization,
        });
        guard.routes.get(&path).cloned()
    }
    .unwrap_or(Canned {
        status: 404,
        headers: vec![],
        body: r#"{"message":"Not Found"}"#.to_string(),
    });
    let mut response = format!(
        "HTTP/1.1 {} X\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n",
        canned.status,
        canned.body.len()
    );
    for (name, value) in &canned.headers {
        response.push_str(&format!("{name}: {value}\r\n"));
    }
    response.push_str("\r\n");
    response.push_str(&canned.body);
    let _ = stream.write_all(response.as_bytes());
}
