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
    assert!(git_available(), "`git` is not on PATH");
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

#[path = "../support/git_http.rs"]
mod git_http;
use git_http::{git_http_backend, serve};

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

fn world(token: &'static str) -> World {
    let backend = git_http_backend()
        .expect("`git http-backend` is in `git --exec-path` (some distributions package it apart)");
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
    let port = serve("127.0.0.1", projects, backend, token);
    let creds = dir.path().join("creds");
    fs::create_dir_all(creds.join("o")).unwrap();
    let home = dir.path().join("home");
    let work = dir.path().join("work");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&work).unwrap();
    World {
        base: format!("http://127.0.0.1:{port}"),
        _dir: dir,
        home,
        creds,
        work,
        port,
    }
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
    assert!(git_available(), "`git` is not on PATH");
    let w = world("scoped-secret");
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
}

#[test]
fn nothing_is_offered_for_another_repository_or_host() {
    assert!(git_available(), "`git` is not on PATH");
    let w = world("scoped-secret");
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
    assert!(git_available(), "`git` is not on PATH");
    let w = world("scoped-secret");
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
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("could not read Username"),
        "git should fail for want of a credential: {stderr}"
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(20));
}
