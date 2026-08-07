//! FR-006 / SC-003 — a failed submodule fetch must say *which* submodule failed and *why*
//! (`010-submodule-worktree-support`, found by T023's manual validation).
//!
//! The requirement is that the user can identify the failing submodule and its cause "directly from
//! what's shown, without inspecting logs". T021 wired the client to append `OperationError`'s
//! `detail` to the message, which was the missing half at the time — but the detail it appends is
//! whatever the error carries, and [`GitCli::submodule_update_init_recursive`] carried a fixed
//! string. git's own words went to the progress callback and nowhere else, so every submodule
//! failure reached the user as the same sentence regardless of cause.
//!
//! These tests run the **real** `git`, because the thing under test is what git says and whether it
//! survives into the error — a fake by construction cannot fail that.

use std::path::Path;
use std::process::Command;

use micold_core::git::{Git, GitCli};

fn run(dir: &Path, args: &[&str]) -> std::process::Output {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

fn init_repo(dir: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    run(dir, &["init", "-q"]);
    run(dir, &["config", "user.email", "t@t.test"]);
    run(dir, &["config", "user.name", "T"]);
    run(dir, &["config", "protocol.file.allow", "always"]);
    std::fs::write(dir.join("README.md"), "x").unwrap();
    run(dir, &["add", "."]);
    run(dir, &["commit", "-qm", "init"]);
}

/// A superproject whose submodule `vendor/broken` points at a path that does not exist — the
/// unreachable-remote case, reachable without a network.
fn superproject_with_a_broken_submodule(root: &Path) -> std::path::PathBuf {
    let nested = root.join("nested");
    let super_ = root.join("super");
    init_repo(&nested);
    init_repo(&super_);
    run(
        &super_,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            "-q",
            nested.to_str().unwrap(),
            "vendor/broken",
        ],
    );
    run(&super_, &["commit", "-qm", "add submodule"]);
    let missing = root.join("does-not-exist");
    run(
        &super_,
        &[
            "config",
            "--file",
            ".gitmodules",
            "submodule.vendor/broken.url",
            missing.to_str().unwrap(),
        ],
    );
    run(
        &super_,
        &[
            "config",
            "-f",
            ".git/config",
            "submodule.vendor/broken.url",
            missing.to_str().unwrap(),
        ],
    );
    run(&super_, &["add", ".gitmodules"]);
    run(&super_, &["commit", "-qm", "break the url"]);
    super_
}

#[test]
fn a_failed_submodule_fetch_names_the_submodule_and_the_reason() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = superproject_with_a_broken_submodule(tmp.path());
    // Run it where production does: a freshly added worktree, whose submodule directory is empty
    // and therefore has to be cloned. In the superproject itself the submodule is already
    // populated, so the update is a no-op and nothing fails — which is exactly why this bug was
    // only ever seen on worktree creation.
    let worktree = tmp.path().join("wt");
    run(
        &repo,
        &[
            "worktree",
            "add",
            "-b",
            "probe",
            worktree.to_str().unwrap(),
            "HEAD",
        ],
    );

    let err = GitCli::new()
        .submodule_update_init_recursive(&worktree, &mut |_| {})
        .expect_err("a submodule pointing at a missing repository must fail");
    let text = err.to_string();

    // The two things SC-003 says the user must be able to identify without opening a log.
    assert!(
        text.contains("vendor/broken"),
        "the error must name the failing submodule; got: {text}"
    );
    assert!(
        text.contains("does not exist"),
        "the error must carry git's own reason; got: {text}"
    );
}

#[test]
fn a_successful_fetch_still_reports_success() {
    // The failure path must not be bought by breaking the ordinary one.
    let tmp = tempfile::tempdir().unwrap();
    let nested = tmp.path().join("nested");
    let super_ = tmp.path().join("super");
    init_repo(&nested);
    init_repo(&super_);
    run(
        &super_,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            "-q",
            nested.to_str().unwrap(),
            "vendor/ok",
        ],
    );
    run(&super_, &["commit", "-qm", "add submodule"]);

    let mut lines = Vec::new();
    GitCli::new()
        .submodule_update_init_recursive(&super_, &mut |l| lines.push(l))
        .expect("an already-satisfiable submodule updates cleanly");
    assert!(super_.join("vendor/ok/README.md").is_file());
}
