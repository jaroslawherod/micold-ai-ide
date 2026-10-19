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
