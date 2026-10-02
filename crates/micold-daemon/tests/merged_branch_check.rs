//! `ClientMsg::MergedBranchCheck` (feature 040, contracts/reading-and-wire.md §3): for each merged
//! pull request's branch the daemon answers whether the local branch holds commits beyond the pull
//! request's last commit, from the repository as it is now and without changing it.
//!
//! Drives `server::serve_connection` over an in-memory duplex against a temporary repository, as
//! `remote_list.rs` does, so the whole route — `spawn_blocking` git, the refusals — is under test.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{
    BranchContainment, ClientMsg, DaemonMsg, MergedBranchQuery, OperationResult,
};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use tokio_util::codec::Framed;

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?} failed: {out:?}");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// A catalog holding one project at `project_dir`, a git repository or not.
fn catalog_with_project(project_dir: &Path, store_dir: &Path, is_git_repo: bool) -> Catalog {
    let workspace = Workspace {
        projects: vec![Project::new(
            project_dir.to_path_buf(),
            is_git_repo,
            Availability::Available,
        )],
        active: Some(project_dir.to_path_buf()),
        sessions: BTreeMap::new(),
        worktree_names: BTreeMap::new(),
        ..Default::default()
    };
    let projects_path = store_dir.join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&workspace)
        .unwrap();
    Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(store_dir.join("settings.json"))),
    )
}

/// A service over one project: a repository whose history is `first` ← `merged` ← `later`, with
/// branch `behind` at `first`, `at` at `merged` (the pull request's last commit) and `ahead` at
/// `later`.
struct Fixture {
    project: tempfile::TempDir,
    _store: tempfile::TempDir,
    state: Arc<DaemonState>,
    merged: String,
    later: String,
}

fn fixture() -> Fixture {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let path = project.path();
    git(path, &["init", "-q"]);
    git(path, &["config", "user.email", "t@t.test"]);
    git(path, &["config", "user.name", "t"]);
    let mut commits = Vec::new();
    for (message, branch) in [("first", "behind"), ("merged", "at"), ("later", "ahead")] {
        git(path, &["commit", "-q", "--allow-empty", "-m", message]);
        git(path, &["branch", branch]);
        commits.push(git(path, &["rev-parse", "HEAD"]));
    }
    let state = Arc::new(DaemonState::new(catalog_with_project(
        path,
        store.path(),
        true,
    )));
    let later = commits.pop().unwrap();
    let merged = commits.pop().unwrap();
    Fixture {
        project,
        _store: store,
        state,
        merged,
        later,
    }
}

type Client = Framed<tokio::io::DuplexStream, ClientCodec>;

async fn connect(state: &Arc<DaemonState>) -> Client {
    let (server_io, client_io) = tokio::io::duplex(256 * 1024);
    tokio::spawn(micold_daemon::server::serve_connection(
        Arc::clone(state),
        server_io,
    ));
    let mut client = Framed::new(client_io, ClientCodec::new());
    client
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: "test".into(),
            client_instance: micold_core::protocol::messages::ClientInstance::current(),
            client_package_version: PACKAGE_VERSION.into(),
            auth_token: None,
            client_fingerprint: BUILD_FINGERPRINT.into(),
            require_fingerprint_match: false,
        }))
        .await
        .unwrap();
    match client.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Welcome { .. }) => {}
        other => panic!("expected Welcome, got {other:?}"),
    }
    client
}

/// Send `msg` and return the reply correlated to `req`, skipping anything else.
async fn ask(client: &mut Client, req: u64, msg: ClientMsg) -> DaemonMsg {
    client.send(Frame::Control(msg)).await.unwrap();
    let wait = async {
        loop {
            match client.next().await.expect("stream open").unwrap() {
                Frame::Control(
                    m @ (DaemonMsg::OperationOk { req: r, .. }
                    | DaemonMsg::OperationError { req: r, .. }),
                ) if r == req => return m,
                _ => continue,
            }
        }
    };
    tokio::time::timeout(std::time::Duration::from_secs(30), wait)
        .await
        .expect("the reply arrives")
}

fn query(branch: &str, head: &str) -> MergedBranchQuery {
    MergedBranchQuery {
        branch: branch.into(),
        head: head.into(),
    }
}

/// Ask the merged-branch question for `project` and return the reply.
async fn check(client: &mut Client, project: &Path, checks: Vec<MergedBranchQuery>) -> DaemonMsg {
    const REQ: u64 = 7;
    ask(
        client,
        REQ,
        ClientMsg::MergedBranchCheck {
            req: REQ,
            project: project.to_path_buf(),
            checks,
        },
    )
    .await
}

/// The answers of a successful reply.
fn answers(reply: DaemonMsg) -> Vec<BranchContainment> {
    match reply {
        DaemonMsg::OperationOk {
            result: OperationResult::MergedBranchCheck { answers },
            ..
        } => answers,
        other => panic!("expected a MergedBranchCheck result, got {other:?}"),
    }
}

/// U59. A branch that stands at its merged pull request's last commit, or behind it, holds
/// nothing the merge did not take (FR-015).
#[tokio::test]
async fn a_branch_at_or_behind_the_head_is_contained() {
    let f = fixture();
    let mut client = connect(&f.state).await;

    let reply = check(
        &mut client,
        f.project.path(),
        vec![query("at", &f.merged), query("behind", &f.merged)],
    )
    .await;

    assert_eq!(
        answers(reply),
        vec![BranchContainment::Contained, BranchContainment::Contained],
        "a tip equal to the head, and a tip that is an ancestor of it"
    );
}

/// U60. Commits made on the branch after the merge are work the merge did not take: no removal
/// may be suggested for it (FR-017, story 3 scenario 4).
#[tokio::test]
async fn a_branch_with_a_commit_after_the_head_is_beyond() {
    let f = fixture();
    let mut client = connect(&f.state).await;

    let reply = check(&mut client, f.project.path(), vec![query("ahead", &f.merged)]).await;

    assert_eq!(answers(reply), vec![BranchContainment::Beyond]);
}

/// A well-formed commit id the repository does not hold: a pull request head never fetched.
const NEVER_FETCHED: &str = "0123456789abcdef0123456789abcdef01234567";

/// U61. What the repository cannot show is never read as "nothing newer" (FR-017): a branch it
/// does not have, and a pull request whose last commit never reached this machine.
#[tokio::test]
async fn a_missing_branch_and_a_head_that_is_not_a_local_object_are_unknown() {
    let f = fixture();
    let mut client = connect(&f.state).await;

    let reply = check(
        &mut client,
        f.project.path(),
        vec![query("no-such-branch", &f.merged), query("at", NEVER_FETCHED)],
    )
    .await;

    assert_eq!(
        answers(reply),
        vec![BranchContainment::Unknown, BranchContainment::Unknown],
        "a missing branch, then a head the repository does not hold"
    );
}

/// U62. A head is used only when it is a full commit id, 40 or 64 hexadecimal characters; anything
/// else is answered `Unknown` and never reaches git. Each head below is one git itself would
/// resolve to `later`, which holds branch `at` — so `Contained` here would mean git was asked.
#[tokio::test]
async fn a_head_that_is_not_a_full_commit_id_is_unknown_without_running_git() {
    let f = fixture();
    let mut client = connect(&f.state).await;
    let abbreviated = &f.later[..f.later.len() - 1];

    let reply = check(
        &mut client,
        f.project.path(),
        vec![
            query("at", &f.later),
            query("at", "HEAD"),
            query("at", "ahead"),
            query("at", abbreviated),
            query("at", ""),
        ],
    )
    .await;

    assert_eq!(
        answers(reply),
        vec![
            BranchContainment::Contained,
            BranchContainment::Unknown,
            BranchContainment::Unknown,
            BranchContainment::Unknown,
            BranchContainment::Unknown,
        ],
        "the full id is answered from the repository; a ref name, a branch name, an abbreviated \
         id and an empty head are not passed to git"
    );
}
