//! A repository's own remotes, for finding its GitHub repository (feature 034,
//! contracts/remote-list-rpc.md §1, research R5).

use std::path::Path;
use std::process::Command;

use micold_core::git::{parse_remote_list, FakeGit, Git, GitCli, GitRemote};

fn remote(name: &str, url: &str) -> GitRemote {
    GitRemote {
        name: name.into(),
        url: url.into(),
    }
}

/// Set in the child copy of this test binary that [`in_child`] starts.
const CHILD_VAR: &str = "MICOLD_TEST_CHILD";

/// Run `test` again in a child copy of this test binary, with `vars` set on that child only, and
/// return `true` in the child. The variables never touch this process, whose other tests run in
/// parallel threads and spawn processes of their own.
fn in_child(test: &str, vars: &[(&str, &std::ffi::OsStr)]) -> bool {
    if std::env::var_os(CHILD_VAR).is_some() {
        return true;
    }
    let mut child = std::process::Command::new(std::env::current_exe().expect("this test binary"));
    child
        .args(["--exact", test, "--test-threads=1", "--nocapture"])
        .env(CHILD_VAR, "1");
    for (name, value) in vars {
        child.env(name, value);
    }
    let out = child.output().expect("the child test binary runs");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success() && stdout.contains("1 passed"),
        "{test} failed in its child process: {}\n{stdout}\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    false
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?} failed: {out:?}");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn lines_parse_in_config_order() {
    let raw = "remote.upstream.url https://github.com/o/r.git\n\
               remote.origin.url git@gitlab.com:me/r.git\n";
    assert_eq!(
        parse_remote_list(raw),
        vec![
            remote("upstream", "https://github.com/o/r.git"),
            remote("origin", "git@gitlab.com:me/r.git"),
        ],
        "each `remote.<name>.url <url>` line is one remote, in the order git listed them"
    );
}

#[test]
fn a_dotted_remote_name_is_kept_whole() {
    assert_eq!(
        parse_remote_list("remote.a.b.url https://github.com/o/r\n"),
        vec![remote("a.b", "https://github.com/o/r")],
        "the name is everything between `remote.` and the last `.url`"
    );
}

#[test]
fn first_url_wins_and_empty_is_empty() {
    let raw = "remote.origin.url https://github.com/o/first\n\
               remote.origin.url https://github.com/o/second\n";
    assert_eq!(
        parse_remote_list(raw),
        vec![remote("origin", "https://github.com/o/first")],
        "a remote with several URLs is fetched from the first"
    );
    assert!(
        parse_remote_list("").is_empty(),
        "no output means no remotes"
    );
}

#[test]
fn fake_git_lists_remotes_in_insertion_order() {
    let repo = Path::new("/repo");
    let fake = FakeGit::new()
        .with_repo(repo)
        .with_remote(repo, "upstream", "https://github.com/o/r")
        .with_remote(repo, "origin", "git@gitlab.com:me/r.git");
    let raw = fake.remote_list(repo).expect("the fake lists remotes");
    assert_eq!(
        parse_remote_list(&raw),
        vec![
            remote("upstream", "https://github.com/o/r"),
            remote("origin", "git@gitlab.com:me/r.git"),
        ],
        "the fake answers in the order its remotes were added, as git answers in config order"
    );
}

/// U9 — `GitCli` lists a real repository's remotes.
#[test]
fn git_cli_lists_remotes() {
    let with = tempfile::tempdir().unwrap();
    git(with.path(), &["init", "-q"]);
    git(
        with.path(),
        &["remote", "add", "origin", "https://github.com/o/r.git"],
    );
    git(
        with.path(),
        &["remote", "add", "upstream", "https://gitlab.com/u/r.git"],
    );
    let raw = GitCli::new().remote_list(with.path()).expect("listed");
    assert_eq!(
        parse_remote_list(&raw),
        vec![
            remote("origin", "https://github.com/o/r.git"),
            remote("upstream", "https://gitlab.com/u/r.git"),
        ],
        "both remotes of a real repository, in config order"
    );
}

/// U9 — a real repository without remotes lists nothing, and that is not an error.
#[test]
fn git_cli_lists_no_remotes_without_error() {
    let without = tempfile::tempdir().unwrap();
    git(without.path(), &["init", "-q"]);
    assert_eq!(
        GitCli::new().remote_list(without.path()).expect(
            "git exits 1 when nothing matches, which means no remotes rather than a failure"
        ),
        "",
        "no remotes, no output"
    );
}

#[test]
fn global_insteadof_is_not_applied() {
    let home = tempfile::tempdir().unwrap();
    let global = home.path().join("gitconfig");
    std::fs::write(
        &global,
        "[url \"https://github.com/\"]\n\tinsteadOf = gh:\n",
    )
    .unwrap();
    // Every git the child runs, `GitCli`'s included, reads this file as the global config.
    if !in_child(
        "global_insteadof_is_not_applied",
        &[("GIT_CONFIG_GLOBAL", global.as_os_str())],
    ) {
        return;
    }

    let repo = tempfile::tempdir().unwrap();
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["remote", "add", "origin", "gh:o/r"]);
    assert_eq!(
        git(repo.path(), &["remote", "get-url", "origin"]).trim(),
        "https://github.com/o/r",
        "precondition: the global rewrite is live for this repository"
    );

    let raw = GitCli::new().remote_list(repo.path()).expect("listed");
    assert_eq!(
        parse_remote_list(&raw),
        vec![remote("origin", "gh:o/r")],
        "the repository's own URL is listed as written; a global rewrite would make the answer \
         depend on where the daemon runs (FR-026)"
    );
}
