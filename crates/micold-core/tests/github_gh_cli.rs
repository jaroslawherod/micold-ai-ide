//! `GhCli`, the production issue source, against a stub `gh` this test writes (feature 034,
//! contracts/github-issue-source.md §3, research R6).
//!
//! The stub records its arguments, the variables `GhCli` sets and its working directory next to
//! itself, then prints a fixture. It is a `sh` script on Unix and a `.cmd` on Windows; `cmd`
//! cannot record arguments one per line, so on Windows the argument check is by containment.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use micold_core::github::{
    list_args, search_args, GhCli, GithubRepo, Issue, IssueLoadError, IssueSource,
};

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
    /// Print a partial answer (search hits, and NOT_FOUND for the numbered lookup) and exit 1, as
    /// `gh` does for any GraphQL error.
    PartialSearch,
    /// Print a SAML-refused answer on stdout and the same refusal on stderr, and exit 1, as `gh`
    /// does for an organization that enforces SAML.
    SamlRefused,
    /// Print a repository NOT_FOUND answer, nothing on stderr, and exit 1.
    ListNotFound,
    /// Print an unknown failure to stderr, nothing on stdout, and exit 1.
    FailWithoutJson,
}

/// The files the stub prints, next to it.
fn copy_fixtures(dir: &Path) {
    std::fs::write(
        dir.join("saml.json"),
        r#"{"data":null,"errors":[{"type":"FORBIDDEN","message":"Resource protected by organization SAML enforcement."}]}"#,
    )
    .unwrap();
    for (from, to) in [
        ("list_page.json", "page.json"),
        ("not_logged_in.stderr", "login.stderr"),
        ("search_pr_number.json", "partial.json"),
        ("saml.stderr", "saml.stderr"),
        ("list_not_found.json", "not_found.json"),
        ("unknown.stderr", "unknown.stderr"),
    ] {
        std::fs::copy(fixture(from), dir.join(to)).unwrap();
    }
}

/// Write the stub into `dir` and return its path.
#[cfg(unix)]
fn stub(dir: &Path, then: Then) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    std::fs::copy(fixture("list_page.json"), dir.join("page.json")).unwrap();
    copy_fixtures(dir);
    let tail = match then {
        Then::PrintPage => "cat \"$dir/page.json\"",
        Then::Hang => "sleep 5",
        Then::NotLoggedIn => "cat \"$dir/login.stderr\" >&2; exit 4",
        Then::PartialSearch => "cat \"$dir/partial.json\"; exit 1",
        Then::SamlRefused => "cat \"$dir/saml.json\"; cat \"$dir/saml.stderr\" >&2; exit 1",
        Then::ListNotFound => "cat \"$dir/not_found.json\"; exit 1",
        Then::FailWithoutJson => "cat \"$dir/unknown.stderr\" >&2; exit 1",
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
    copy_fixtures(dir);
    let tail = match then {
        Then::PrintPage => "type \"%~dp0page.json\"",
        Then::Hang => "ping -n 6 127.0.0.1 >nul",
        Then::NotLoggedIn => "type \"%~dp0login.stderr\" 1>&2\r\nexit /b 4",
        Then::PartialSearch => "type \"%~dp0partial.json\"\r\nexit /b 1",
        Then::ListNotFound => "type \"%~dp0not_found.json\"\r\nexit /b 1",
        Then::SamlRefused => {
            "type \"%~dp0saml.json\"\r\ntype \"%~dp0saml.stderr\" 1>&2\r\nexit /b 1"
        }
        Then::FailWithoutJson => "type \"%~dp0unknown.stderr\" 1>&2\r\nexit /b 1",
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

/// U95 — a GraphQL answer with `data` is read whatever `gh`'s exit status: the numbered lookup's
/// NOT_FOUND exits 1, and the search hits beside it are the result. A failure with no JSON is
/// classified from stderr, as a list failure is.
#[test]
fn a_partial_response_is_parsed() {
    let dir = tempfile::tempdir().unwrap();
    let found = GhCli::new(stub(dir.path(), Then::PartialSearch))
        .search_open(&repo(), "#4313")
        .expect("the hits beside a NOT_FOUND lookup are the answer");
    assert_eq!(found.iter().map(Issue::number).collect::<Vec<_>>(), [4312]);
    let args = recorded(dir.path(), "args.txt");
    if cfg!(unix) {
        let got: Vec<&str> = args.lines().collect();
        assert_eq!(
            got,
            search_args(&repo(), "#4313"),
            "exactly `search_args` (FR-025)"
        );
    }

    let dir = tempfile::tempdir().unwrap();
    let failed = GhCli::new(stub(dir.path(), Then::FailWithoutJson)).search_open(&repo(), "x");
    assert!(
        matches!(failed, Err(IssueLoadError::Other(_))),
        "no JSON on stdout: classified from stderr, got {failed:?}"
    );

    // An answer the parser refuses is classified from stderr, which names what GraphQL's error
    // type does not: FORBIDDEN alone would read as `Other`, stderr says SAML (review A #2).
    let dir = tempfile::tempdir().unwrap();
    let listed = GhCli::new(stub(dir.path(), Then::SamlRefused)).list_open(&repo(), None);
    assert_eq!(listed.unwrap_err(), IssueLoadError::NoAccess);

    // An error the answer itself types exactly is kept, even when stderr says nothing (review A
    // round 2 #3).
    let dir = tempfile::tempdir().unwrap();
    let listed = GhCli::new(stub(dir.path(), Then::ListNotFound)).list_open(&repo(), None);
    assert_eq!(listed.unwrap_err(), IssueLoadError::NoAccess);
}
