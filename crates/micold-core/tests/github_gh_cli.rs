//! `GhCli`, the production issue source, against a stub `gh` this test writes (feature 034,
//! contracts/github-issue-source.md §3, research R6).
//!
//! The stub records its arguments, the variables `GhCli` sets and its working directory next to
//! itself, then prints a fixture. It is a `sh` script on Unix and a `.cmd` on Windows; `cmd`
//! cannot record arguments one per line, so on Windows the argument check is by containment.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use micold_core::github::{list_args, GhCli, GithubRepo, IssueLoadError, IssueSource};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/gh")
        .join(name)
}

fn repo() -> GithubRepo {
    GithubRepo::from_remote_url("https://github.com/o/r").unwrap()
}

/// What the stub does after recording.
enum Then {
    /// Print `list_page.json` and exit 0.
    PrintPage,
    /// Outlive any short bound.
    Hang,
    /// Print the not-logged-in text to stderr and exit 4, as `gh` does.
    NotLoggedIn,
}

/// Write the stub into `dir` and return its path.
#[cfg(unix)]
fn stub(dir: &Path, then: Then) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    std::fs::copy(fixture("list_page.json"), dir.join("page.json")).unwrap();
    std::fs::copy(fixture("not_logged_in.stderr"), dir.join("login.stderr")).unwrap();
    let tail = match then {
        Then::PrintPage => "cat \"$dir/page.json\"",
        Then::Hang => "sleep 5",
        Then::NotLoggedIn => "cat \"$dir/login.stderr\" >&2; exit 4",
    };
    let script = format!(
        "#!/bin/sh\n\
         dir=$(dirname \"$0\")\n\
         for a in \"$@\"; do printf '%s\\n' \"$a\"; done > \"$dir/args.txt\"\n\
         {{\n\
           echo \"GH_PROMPT_DISABLED=$GH_PROMPT_DISABLED\"\n\
           echo \"GH_NO_UPDATE_NOTIFIER=$GH_NO_UPDATE_NOTIFIER\"\n\
           echo \"NO_COLOR=$NO_COLOR\"\n\
           echo \"CLICOLOR=$CLICOLOR\"\n\
           echo \"GH_PAGER=${{GH_PAGER-unset}}\"\n\
           echo \"GH_DEBUG=${{GH_DEBUG-unset}}\"\n\
         }} > \"$dir/env.txt\"\n\
         pwd -P > \"$dir/cwd.txt\"\n\
         {tail}\n"
    );
    let path = dir.join("gh");
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// Write the stub into `dir` and return its path.
#[cfg(windows)]
fn stub(dir: &Path, then: Then) -> PathBuf {
    std::fs::copy(fixture("list_page.json"), dir.join("page.json")).unwrap();
    std::fs::copy(fixture("not_logged_in.stderr"), dir.join("login.stderr")).unwrap();
    let tail = match then {
        Then::PrintPage => "type \"%~dp0page.json\"",
        Then::Hang => "ping -n 6 127.0.0.1 >nul",
        Then::NotLoggedIn => "type \"%~dp0login.stderr\" 1>&2\r\nexit /b 4",
    };
    let script = format!(
        "@echo off\r\n\
         echo %*> \"%~dp0args.txt\"\r\n\
         (echo GH_PROMPT_DISABLED=%GH_PROMPT_DISABLED%& echo GH_NO_UPDATE_NOTIFIER=%GH_NO_UPDATE_NOTIFIER%& echo NO_COLOR=%NO_COLOR%& echo CLICOLOR=%CLICOLOR%)> \"%~dp0env.txt\"\r\n\
         cd > \"%~dp0cwd.txt\"\r\n\
         {tail}\r\n"
    );
    let path = dir.join("gh.cmd");
    std::fs::write(&path, script).unwrap();
    path
}

fn recorded(dir: &Path, name: &str) -> String {
    std::fs::read_to_string(dir.join(name))
        .unwrap_or_else(|e| panic!("the stub recorded {name}: {e}"))
}

fn home() -> PathBuf {
    let home = directories::BaseDirs::new()
        .expect("a home directory")
        .home_dir()
        .to_path_buf();
    std::fs::canonicalize(&home).unwrap_or(home)
}

#[test]
fn gh_cli_runs_gh_as_specified() {
    let dir = tempfile::tempdir().unwrap();
    let gh = stub(dir.path(), Then::PrintPage);
    // A user who debugs `gh` from their shell profile; the app must not inherit that.
    std::env::set_var("GH_DEBUG", "api");

    let page = GhCli::new(gh)
        .list_open(&repo(), Some("CUR"))
        .expect("the stub's page parses");
    assert_eq!(
        page.issues.len(),
        3,
        "the page gh printed is the page returned"
    );
    assert_eq!(page.total_open, 142);

    let expected = list_args(&repo(), Some("CUR"));
    let args = recorded(dir.path(), "args.txt");
    if cfg!(unix) {
        let got: Vec<&str> = args.lines().collect();
        assert_eq!(got, expected, "exactly `list_args`, nothing more (FR-025)");
    } else {
        for arg in expected.iter().filter(|a| !a.starts_with("query=")) {
            assert!(args.contains(arg.as_str()), "{arg} was passed: {args}");
        }
    }

    let env = recorded(dir.path(), "env.txt");
    for line in [
        "GH_PROMPT_DISABLED=1",
        "GH_NO_UPDATE_NOTIFIER=1",
        "NO_COLOR=1",
        "CLICOLOR=0",
    ] {
        assert!(
            env.lines().any(|l| l.trim() == line),
            "{line} is set so gh never prompts, notifies or colours: {env}"
        );
    }
    if cfg!(unix) {
        assert!(
            env.lines().any(|l| l == "GH_DEBUG=unset"),
            "GH_DEBUG is removed, so debug traces never reach the stderr `classify` reads: {env}"
        );
        assert!(
            env.lines().any(|l| l == "GH_PAGER="),
            "GH_PAGER is set, and empty, so gh never pages: {env}"
        );
    }

    let cwd = PathBuf::from(recorded(dir.path(), "cwd.txt").trim());
    assert_eq!(
        std::fs::canonicalize(&cwd).unwrap_or(cwd),
        home(),
        "gh runs in the user's home, so it never reads a repository's local config"
    );
}

#[test]
fn a_hung_gh_is_timed_out() {
    let dir = tempfile::tempdir().unwrap();
    let bound = Duration::from_millis(500);
    let started = Instant::now();
    let result = GhCli::new(stub(dir.path(), Then::Hang))
        .with_timeout(bound)
        .list_open(&repo(), None);
    assert_eq!(result.unwrap_err(), IssueLoadError::TimedOut);
    assert!(
        started.elapsed() < bound + Duration::from_secs(2),
        "the bound is enforced, not waited out"
    );
}

#[test]
fn exit_4_is_not_signed_in() {
    let dir = tempfile::tempdir().unwrap();
    let result = GhCli::new(stub(dir.path(), Then::NotLoggedIn)).list_open(&repo(), None);
    assert_eq!(result.unwrap_err(), IssueLoadError::NotSignedIn);
}
