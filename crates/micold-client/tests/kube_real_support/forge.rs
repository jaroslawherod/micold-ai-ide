//! A fake GitHub the pod reaches on the host side of the docker network `kind` (#1760, research
//! R6): the REST listing a token's reach (`GET /user/repos`, also under `/api/v3`) and smart-HTTP
//! git served by `git http-backend`, both accepting only the run's secret. Git gets `Basic
//! x-access-token:<secret>`, compared in constant time; anything else is a 401 with
//! `WWW-Authenticate`. Every request's credential is recorded for the assertions.
#![cfg(feature = "kube-real-runtime")]

use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use base64::Engine;

/// The user name git sends with a token.
pub const USER: &str = "x-access-token";

/// One request the forge answered, with the credential it carried.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seen {
    pub method: String,
    pub path: String,
    /// `(user, secret)` of a `Basic` header, or `("Bearer", token)`; `None` when there was none.
    pub credential: Option<(String, String)>,
    /// Whether the forge accepted it.
    pub accepted: bool,
}

/// The fake forge: bare repositories under a temp directory, served on every interface.
pub struct Forge {
    port: u16,
    root: tempfile::TempDir,
    seen: Arc<Mutex<Vec<Seen>>>,
    stop: Arc<AtomicBool>,
}

struct Shared {
    secret: String,
    repos: Vec<String>,
    root: PathBuf,
    seen: Arc<Mutex<Vec<Seen>>>,
}

fn git(cwd: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(["-c", "init.defaultBranch=main"])
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(format!(
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        ))
    }
}

impl Forge {
    /// Serve `repos` (`owner/name`, each seeded with one commit on `main`) to whoever presents
    /// `secret`. The port is the system's pick; the pod reaches it at [`Forge::base`].
    pub fn start(secret: &str, repos: &[&str]) -> Result<Self, String> {
        let root = tempfile::tempdir().map_err(|e| e.to_string())?;
        for repo in repos {
            let bare = root.path().join(format!("{repo}.git"));
            std::fs::create_dir_all(&bare).map_err(|e| e.to_string())?;
            git(&bare, &["init", "-q", "--bare"])?;
            let seed = tempfile::tempdir().map_err(|e| e.to_string())?;
            git(seed.path(), &["init", "-q"])?;
            std::fs::write(seed.path().join("README"), "forge\n").map_err(|e| e.to_string())?;
            git(seed.path(), &["add", "."])?;
            git(seed.path(), &["commit", "-q", "-m", "seed"])?;
            git(
                seed.path(),
                &["push", "-q", bare.to_str().unwrap(), "HEAD:refs/heads/main"],
            )?;
        }
        let listener = TcpListener::bind(("0.0.0.0", 0)).map_err(|e| format!("forge bind: {e}"))?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let shared = Arc::new(Shared {
            secret: secret.to_string(),
            repos: repos.iter().map(|r| r.to_ascii_lowercase()).collect(),
            root: root.path().to_path_buf(),
            seen: seen.clone(),
        });
        let flag = stop.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                if flag.load(Ordering::SeqCst) {
                    return;
                }
                let Ok(stream) = stream else { continue };
                let shared = shared.clone();
                std::thread::spawn(move || {
                    let _ = serve(stream, &shared);
                });
            }
        });
        Ok(Self {
            port,
            root,
            seen,
            stop,
        })
    }

    /// The forge as a pod reaches it, which is also the `host` a delivered token names.
    pub fn base(&self) -> String {
        format!("http://{}:{}", super::HOST_IP, self.port)
    }

    /// `owner/name` as git reaches it from a pod.
    pub fn url(&self, repo: &str) -> String {
        format!("{}/{repo}.git", self.base())
    }

    /// The same repository on this machine, for a request of the test's own.
    pub fn local_url(&self, repo: &str) -> String {
        format!("http://127.0.0.1:{}/{repo}.git", self.port)
    }

    pub fn local_addr(&self) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], self.port))
    }

    fn bare(&self, repo: &str) -> PathBuf {
        self.root.path().join(format!("{repo}.git"))
    }

    /// The commit a branch of `repo` points at, `None` when it does not exist.
    pub fn branch(&self, repo: &str, branch: &str) -> Option<String> {
        git(
            &self.bare(repo),
            &[
                "rev-parse",
                "--verify",
                "-q",
                &format!("refs/heads/{branch}"),
            ],
        )
        .ok()
    }

    /// Add a commit on top of `main` as branch `branch` (the forge's side of a pull); its id.
    pub fn add_commit(&self, repo: &str, branch: &str) -> String {
        let bare = self.bare(repo);
        let sha = git(
            &bare,
            &[
                "commit-tree",
                "main^{tree}",
                "-p",
                "main",
                "-m",
                "forge side",
            ],
        )
        .expect("commit on the forge");
        git(
            &bare,
            &["update-ref", &format!("refs/heads/{branch}"), &sha],
        )
        .expect("branch on the forge");
        sha
    }

    /// Every request answered so far.
    pub fn requests(&self) -> Vec<Seen> {
        self.seen.lock().unwrap().clone()
    }
}

impl Drop for Forge {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wake the accept loop so it sees the flag.
        let _ = TcpStream::connect(self.local_addr());
    }
}

struct Request {
    method: String,
    target: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Request {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

fn read_request(stream: &mut TcpStream) -> Option<Request> {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    while !buf.ends_with(b"\r\n\r\n") {
        if stream.read(&mut byte).ok()? == 0 || buf.len() > 64 * 1024 {
            return None;
        }
        buf.push(byte[0]);
    }
    let head = String::from_utf8_lossy(&buf).to_string();
    let mut lines = head.split("\r\n");
    let mut first = lines.next()?.split_whitespace();
    let method = first.next()?.to_string();
    let target = first.next()?.to_string();
    let headers: Vec<(String, String)> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
        .collect();
    let mut request = Request {
        method,
        target,
        headers,
        body: Vec::new(),
    };
    if request
        .header("transfer-encoding")
        .is_some_and(|v| v.eq_ignore_ascii_case("chunked"))
    {
        request.body = read_chunked(stream)?;
    } else if let Some(len) = request.header("content-length") {
        let mut body = vec![0u8; len.parse().ok()?];
        stream.read_exact(&mut body).ok()?;
        request.body = body;
    }
    Some(request)
}

fn read_line(stream: &mut TcpStream) -> Option<String> {
    let mut line = Vec::new();
    let mut byte = [0u8; 1];
    while !line.ends_with(b"\r\n") {
        if stream.read(&mut byte).ok()? == 0 {
            return None;
        }
        line.push(byte[0]);
    }
    Some(String::from_utf8_lossy(&line).trim().to_string())
}

fn read_chunked(stream: &mut TcpStream) -> Option<Vec<u8>> {
    let mut body = Vec::new();
    loop {
        let size = usize::from_str_radix(read_line(stream)?.split(';').next()?, 16).ok()?;
        if size == 0 {
            while !read_line(stream)?.is_empty() {}
            return Some(body);
        }
        let mut chunk = vec![0u8; size];
        stream.read_exact(&mut chunk).ok()?;
        body.extend(chunk);
        read_line(stream)?;
    }
}

/// Equal, in time that does not depend on where they first differ.
fn same(a: &[u8], b: &[u8]) -> bool {
    let mut diff = a.len() ^ b.len();
    for i in 0..a.len().max(b.len()) {
        diff |= usize::from(a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0));
    }
    diff == 0
}

/// The credential of an `Authorization` header.
fn credential(header: Option<&str>) -> Option<(String, String)> {
    let header = header?;
    if let Some(token) = header.strip_prefix("Bearer ") {
        return Some(("Bearer".into(), token.trim().to_string()));
    }
    let encoded = header.strip_prefix("Basic ")?;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .ok()?;
    let text = String::from_utf8(decoded).ok()?;
    let (user, secret) = text.split_once(':')?;
    Some((user.to_string(), secret.to_string()))
}

fn reply(stream: &mut TcpStream, status: &str, headers: &[String], body: &[u8]) {
    let mut head = format!("HTTP/1.1 {status}\r\n");
    for h in headers {
        head.push_str(h);
        head.push_str("\r\n");
    }
    head.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    ));
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.shutdown(Shutdown::Both);
}

fn serve(mut stream: TcpStream, shared: &Shared) -> Option<()> {
    let request = read_request(&mut stream)?;
    let (path, query) = request
        .target
        .split_once('?')
        .unwrap_or((request.target.as_str(), ""));
    let given = credential(request.header("authorization"));
    // Git speaks Basic `x-access-token:<secret>`; the REST listing takes the token as a Bearer.
    let accepted = given.as_ref().is_some_and(|(user, secret)| {
        (user == USER || user == "Bearer") && same(secret.as_bytes(), shared.secret.as_bytes())
    });
    shared.seen.lock().unwrap().push(Seen {
        method: request.method.clone(),
        path: path.to_string(),
        credential: given,
        accepted,
    });
    if !accepted {
        reply(
            &mut stream,
            "401 Unauthorized",
            &["WWW-Authenticate: Basic realm=\"forge\"".to_string()],
            b"",
        );
        return Some(());
    }
    let api = path.strip_prefix("/api/v3").unwrap_or(path);
    if api == "/user/repos" {
        let listing: Vec<serde_json::Value> = shared
            .repos
            .iter()
            .map(|r| serde_json::json!({ "full_name": r }))
            .collect();
        let body = serde_json::to_vec(&listing).ok()?;
        reply(
            &mut stream,
            "200 OK",
            &["Content-Type: application/json".to_string()],
            &body,
        );
        return Some(());
    }
    let known = shared
        .repos
        .iter()
        .any(|r| path.to_ascii_lowercase().starts_with(&format!("/{r}.git/")));
    if !known {
        reply(&mut stream, "404 Not Found", &[], b"");
        return Some(());
    }
    let (status, headers, body) = cgi(shared, &request, path, query)?;
    reply(&mut stream, &status, &headers, &body);
    Some(())
}

/// Run `git http-backend` for the request: its CGI answer as a status line, headers and body.
fn cgi(
    shared: &Shared,
    request: &Request,
    path: &str,
    query: &str,
) -> Option<(String, Vec<String>, Vec<u8>)> {
    let mut cmd = Command::new("git");
    cmd.arg("http-backend")
        .env("GIT_PROJECT_ROOT", &shared.root)
        .env("GIT_HTTP_EXPORT_ALL", "1")
        .env("REQUEST_METHOD", &request.method)
        .env("PATH_INFO", path)
        .env("QUERY_STRING", query)
        .env("CONTENT_LENGTH", request.body.len().to_string())
        .env("REMOTE_USER", USER)
        .env("REMOTE_ADDR", "127.0.0.1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(kind) = request.header("content-type") {
        cmd.env("CONTENT_TYPE", kind);
    }
    if let Some(encoding) = request.header("content-encoding") {
        cmd.env("HTTP_CONTENT_ENCODING", encoding);
    }
    let mut child = cmd.spawn().ok()?;
    let mut stdin = child.stdin.take()?;
    let body = request.body.clone();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&body);
    });
    let out = child.wait_with_output().ok()?;
    let _ = writer.join();
    let split = out.stdout.windows(4).position(|w| w == b"\r\n\r\n")?;
    let head = String::from_utf8_lossy(&out.stdout[..split]).to_string();
    let mut status = "200 OK".to_string();
    let mut headers = Vec::new();
    for line in head.split("\r\n") {
        match line.split_once(':') {
            Some(("Status", value)) => status = value.trim().to_string(),
            Some(_) => headers.push(line.to_string()),
            None => {}
        }
    }
    Some((status, headers, out.stdout[split + 4..].to_vec()))
}
