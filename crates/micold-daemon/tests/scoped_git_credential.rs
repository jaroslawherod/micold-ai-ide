//! Feature 1002, FR-004: git with the sandbox's environment consults no inherited credential
//! helper and sends no credential embedded in a host configuration. Extended by the in-container
//! helper's tests (T024).

// unix-only: the planted helper is a `#!/bin/sh` script.
#![cfg(unix)]

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use micold_core::scoped_credential::{git_env, sanitize_gitconfig_text, GITHUB_BASE};

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .stdout(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// `git credential fill` for `url`, with `home` as the user's home and the sandbox's `git_env`
/// (or none) on top. Returns its stdout.
fn fill(home: &Path, repo: &Path, with_env: bool) -> String {
    let mut git = Command::new("git");
    git.current_dir(repo)
        .args(["credential", "fill"])
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", home)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if with_env {
        git.envs(git_env(GITHUB_BASE));
    }
    let mut child = git.spawn().expect("git runs");
    {
        use std::io::Write;
        child
            .stdin
            .take()
            .expect("stdin")
            .write_all(b"protocol=https\nhost=github.com\npath=o/r\n\n")
            .expect("write the request");
    }
    let out = child.wait_with_output().expect("git finishes");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn a_planted_host_helper_never_runs_and_no_password_is_returned() {
    if !git_available() {
        eprintln!("skipped: `git` is not on PATH, so there is no helper to plant");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let home = dir.path().join("home");
    let repo = dir.path().join("repo");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&repo).unwrap();
    let store = dir.path().join("planted-store");
    fs::write(&store, "https://user:hostsecret@github.com\n").unwrap();
    let ran = dir.path().join("repo-helper-ran");
    let helper = dir.path().join("helper.sh");
    fs::write(
        &helper,
        format!(
            "#!/bin/sh\ntouch {}\necho username=u\necho password=repohelpersecret\n",
            ran.display()
        ),
    )
    .unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&helper, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let host_config = format!(
        "[credential]\n\thelper = store --file {}\n\
         [url \"https://tok:x-oauth@github.com/\"]\n\tinsteadOf = https://github.com/\n",
        store.display()
    );
    // The host's configuration, as the sandbox shares it: sanitized.
    let shared = sanitize_gitconfig_text(&host_config).expect("sanitized");
    fs::write(home.join(".gitconfig"), &shared).unwrap();
    assert!(
        !shared.contains("tok") && !shared.contains("store"),
        "{shared}"
    );
    // A repository-level, URL-specific helper: not the host's, but part of what git would consult.
    let init = Command::new("git")
        .current_dir(&repo)
        .args(["init", "-q"])
        .env("HOME", &home)
        .status()
        .unwrap();
    assert!(init.success());
    let config = Command::new("git")
        .current_dir(&repo)
        .args(["config", "credential.https://github.com.helper"])
        .arg(format!("!{}", helper.display()))
        .env("HOME", &home)
        .status()
        .unwrap();
    assert!(config.success());

    // Control: without the reset the repository helper answers, so the test can fail.
    let before = fill(&home, &repo, false);
    assert!(before.contains("repohelpersecret"), "{before}");
    fs::remove_file(&ran).unwrap();

    let out = fill(&home, &repo, true);
    assert!(!out.contains("password="), "{out}");
    assert!(!out.contains("secret"), "{out}");
    assert!(!ran.exists(), "the repository helper ran");
}

// ---- T024 (FR-007, FR-015): git over HTTPS with the in-container helper ------------------------

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;

fn git_http_backend() -> Option<String> {
    let out = Command::new("git").arg("--exec-path").output().ok()?;
    let path = format!(
        "{}/git-http-backend",
        String::from_utf8_lossy(&out.stdout).trim()
    );
    Path::new(&path).exists().then_some(path)
}

/// A loopback server speaking smart HTTP over `git http-backend`, one request per connection, with
/// basic auth `x-access-token:<token>`. Serves until the process ends.
fn serve(projects: std::path::PathBuf, backend: String, token: &'static str) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
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
            out.extend(std::iter::repeat('=').take(3 - chunk.len()));
        }
        out
    }
}

struct World {
    _dir: tempfile::TempDir,
    home: std::path::PathBuf,
    creds: std::path::PathBuf,
    work: std::path::PathBuf,
    base: String,
    port: u16,
}

impl World {
    /// A `git` as the sandbox runs it: cleared environment, the sandbox's `git_env` for the
    /// loopback base, the helper binary on `PATH`, the credential root and base overridden.
    fn git(&self, dir: &Path) -> Command {
        let helper = Path::new(env!("CARGO_BIN_EXE_micold-daemon"))
            .parent()
            .unwrap();
        let path = format!(
            "{}:{}",
            helper.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let mut git = Command::new("git");
        git.current_dir(dir)
            .env_clear()
            .env("PATH", path)
            .env("HOME", &self.home)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@example.com")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@example.com")
            .env("MICOLD_GIT_CREDENTIAL_ROOT", &self.creds)
            .env("MICOLD_GIT_CREDENTIAL_BASE", &self.base)
            .envs(git_env(&self.base))
            .stdin(Stdio::null());
        git
    }

    fn run(&self, dir: &Path, args: &[&str]) -> std::process::Output {
        self.git(dir).args(args).output().unwrap()
    }
}

fn world(token: &'static str) -> Option<World> {
    let backend = git_http_backend()?;
    let dir = tempfile::tempdir().unwrap();
    let projects = dir.path().join("projects");
    let bare = projects.join("o").join("r.git");
    fs::create_dir_all(&bare).unwrap();
    assert!(Command::new("git")
        .args(["init", "-q", "--bare", "-b", "main"])
        .arg(&bare)
        .status()
        .unwrap()
        .success());
    let port = serve(projects, backend, token);
    let creds = dir.path().join("creds");
    fs::create_dir_all(creds.join("o")).unwrap();
    let home = dir.path().join("home");
    let work = dir.path().join("work");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&work).unwrap();
    Some(World {
        base: format!("http://127.0.0.1:{port}"),
        _dir: dir,
        home,
        creds,
        work,
        port,
    })
}

fn credential_file(world: &World, expires_at: u64, token: &str) {
    fs::write(
        world.creds.join("o").join("r"),
        micold_core::scoped_credential::helper::render_credential_file(expires_at, token),
    )
    .unwrap();
}

fn far_future() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 3600
}

fn ok(out: &std::process::Output, what: &str) {
    assert!(
        out.status.success(),
        "{what}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn push_and_pull_succeed_with_the_scoped_credential_and_ssh_remotes_are_rewritten() {
    if !git_available() {
        eprintln!("skipped: `git` is not on PATH");
        return;
    }
    let Some(w) = world("scoped-secret") else {
        eprintln!("skipped: `git http-backend` is not installed");
        return;
    };
    credential_file(&w, far_future(), "scoped-secret");
    let url = format!("{}/o/r.git", w.base);
    let mine = w.work.join("mine");
    ok(&w.run(&w.work, &["clone", "-q", &url, "mine"]), "clone");
    fs::write(mine.join("a.txt"), "one\n").unwrap();
    ok(&w.run(&mine, &["add", "."]), "add");
    ok(&w.run(&mine, &["commit", "-q", "-m", "one"]), "commit");
    ok(
        &w.run(&mine, &["push", "-q", "origin", "HEAD:main"]),
        "push",
    );
    // A second clone pulls what the first pushed.
    let other = w.work.join("other");
    ok(
        &w.run(&w.work, &["clone", "-q", &url, "other"]),
        "second clone",
    );
    fs::write(mine.join("b.txt"), "two\n").unwrap();
    ok(&w.run(&mine, &["add", "."]), "add b");
    ok(&w.run(&mine, &["commit", "-q", "-m", "two"]), "commit b");
    ok(
        &w.run(&mine, &["push", "-q", "origin", "HEAD:main"]),
        "push b",
    );
    ok(&w.run(&other, &["pull", "-q", "origin", "main"]), "pull");
    assert!(other.join("b.txt").exists());

    // An SSH-form remote of the forge is rewritten to HTTPS and authenticates the same way. The
    // rewrite is of `git@github.com:`, so the loopback base stands in through the same rule
    // `git_env` writes for `ssh://git@github.com/`.
    ok(
        &w.run(
            &mine,
            &["remote", "set-url", "origin", "git@github.com:o/r.git"],
        ),
        "set-url",
    );
    let rewritten = w.run(&mine, &["ls-remote", "origin"]);
    ok(&rewritten, "ls-remote over the rewritten ssh remote");
    assert!(String::from_utf8_lossy(&rewritten.stdout).contains("refs/heads/main"));
    let _ = w.port;
}

#[test]
fn nothing_is_offered_for_another_repository_or_host() {
    if !git_available() {
        eprintln!("skipped: `git` is not on PATH");
        return;
    }
    let Some(w) = world("scoped-secret") else {
        eprintln!("skipped: `git http-backend` is not installed");
        return;
    };
    credential_file(&w, far_future(), "scoped-secret");
    let fill = |host: &str, path: &str| {
        let mut child = w
            .git(&w.work)
            .args(["credential", "fill"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(format!("protocol=http\nhost={host}\npath={path}\n\n").as_bytes())
            .unwrap();
        String::from_utf8_lossy(&child.wait_with_output().unwrap().stdout).into_owned()
    };
    let host = format!("127.0.0.1:{}", w.port);
    assert!(fill(&host, "o/r").contains("password=scoped-secret"));
    assert!(!fill(&host, "o/other").contains("password="));
    assert!(!fill("example.com", "o/r").contains("password="));
}

#[test]
fn an_expired_credential_makes_push_fail_at_once_without_a_prompt() {
    if !git_available() {
        eprintln!("skipped: `git` is not on PATH");
        return;
    }
    let Some(w) = world("scoped-secret") else {
        eprintln!("skipped: `git http-backend` is not installed");
        return;
    };
    credential_file(&w, far_future(), "scoped-secret");
    let url = format!("{}/o/r.git", w.base);
    let mine = w.work.join("mine");
    ok(&w.run(&w.work, &["clone", "-q", &url, "mine"]), "clone");
    fs::write(mine.join("a.txt"), "one\n").unwrap();
    ok(&w.run(&mine, &["add", "."]), "add");
    ok(&w.run(&mine, &["commit", "-q", "-m", "one"]), "commit");
    credential_file(&w, 1, "scoped-secret");
    let started = std::time::Instant::now();
    let out = w.run(&mine, &["push", "-q", "origin", "HEAD:main"]);
    assert!(!out.status.success(), "an expired credential pushed");
    assert!(started.elapsed() < std::time::Duration::from_secs(20));
}
