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
    BranchContainment, ClientMsg, DaemonMsg, ErrorKind, MergedBranchQuery, OperationResult,
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

    let reply = check(
        &mut client,
        f.project.path(),
        vec![query("ahead", &f.merged)],
    )
    .await;

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
        vec![
            query("no-such-branch", &f.merged),
            query("at", NEVER_FETCHED),
        ],
    )
    .await;

    assert_eq!(
        answers(reply),
        vec![BranchContainment::Unknown, BranchContainment::Unknown],
        "a missing branch, then a head the repository does not hold"
    );
}

/// U61 (review A). A branch is only ever a name under `refs/heads/`: a name carrying a revision
/// suffix is answered `Unknown`. Each name below is one git would resolve to `merged`, the
/// commit before branch `ahead`'s tip — so `Contained` here would hide `ahead`'s newer commit.
#[tokio::test]
async fn a_branch_name_with_a_revision_suffix_is_unknown() {
    let f = fixture();
    let mut client = connect(&f.state).await;

    let reply = check(
        &mut client,
        f.project.path(),
        vec![
            query("ahead~1", &f.merged),
            query("ahead^", &f.merged),
            query("ahead^{commit}~1", &f.merged),
            query("ahead@{0}~1", &f.merged),
        ],
    )
    .await;

    assert_eq!(answers(reply), vec![BranchContainment::Unknown; 4]);
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

/// U63. The order is all that ties an answer to its branch (data-model §6): one answer per query,
/// in the order asked, whatever the answers are.
#[tokio::test]
async fn the_answers_are_one_per_query_in_query_order() {
    let f = fixture();
    let mut client = connect(&f.state).await;

    let reply = check(
        &mut client,
        f.project.path(),
        vec![
            query("ahead", &f.merged),
            query("at", &f.merged),
            query("no-such-branch", &f.merged),
            query("behind", &f.merged),
        ],
    )
    .await;

    assert_eq!(
        answers(reply),
        vec![
            BranchContainment::Beyond,
            BranchContainment::Contained,
            BranchContainment::Unknown,
            BranchContainment::Contained,
        ]
    );
}

/// U64. One reading asks about at most 50 branches at once (contracts/reading-and-wire.md §3):
/// 50 queries are answered, and a longer list is refused as a malformed request, not cut short.
#[tokio::test]
async fn fifty_queries_are_answered_and_fifty_one_are_refused() {
    let f = fixture();
    let mut client = connect(&f.state).await;
    let queries = |count: usize| vec![query("at", &f.merged); count];

    let fifty = check(&mut client, f.project.path(), queries(50)).await;
    assert_eq!(
        answers(fifty),
        vec![BranchContainment::Contained; 50],
        "the largest list a reading sends is answered in full"
    );

    match check(&mut client, f.project.path(), queries(51)).await {
        DaemonMsg::OperationError { kind, .. } => assert_eq!(
            kind,
            ErrorKind::InvalidInput,
            "a list over the limit is a malformed request"
        ),
        other => panic!("51 queries must be refused, got {other:?}"),
    }
}

/// U65. The question is about a project's repository: a folder the catalog holds that is not one
/// is refused as the worktree requests are, and a path the catalog does not hold is not found.
#[tokio::test]
async fn a_project_that_is_not_a_repository_is_refused() {
    let folder = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let state = Arc::new(DaemonState::new(catalog_with_project(
        folder.path(),
        store.path(),
        false,
    )));
    let mut client = connect(&state).await;

    match check(
        &mut client,
        folder.path(),
        vec![query("main", NEVER_FETCHED)],
    )
    .await
    {
        DaemonMsg::OperationError { kind, .. } => assert_eq!(kind, ErrorKind::Refused),
        other => panic!("a folder that is not a repository must be refused, got {other:?}"),
    }

    let unknown = folder.path().join("not-a-project");
    match check(&mut client, &unknown, vec![query("main", NEVER_FETCHED)]).await {
        DaemonMsg::OperationError { kind, .. } => assert_eq!(kind, ErrorKind::NotFound),
        other => panic!("a path that is not a project must be not found, got {other:?}"),
    }
}

/// Every file under `dir`, the repository's own `.git` included, with its content.
fn files_under(dir: &Path) -> BTreeMap<std::path::PathBuf, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in std::fs::read_dir(&next).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let content = std::fs::read(&path).unwrap();
                files.insert(path, content);
            }
        }
    }
    files
}

/// U66. The question only reads (FR-016, FR-018a): after every kind of answer the repository holds
/// the same files with the same content — no ref moved, nothing fetched, nothing left behind.
#[tokio::test]
async fn the_check_leaves_the_repository_as_it_found_it() {
    let f = fixture();
    let mut client = connect(&f.state).await;
    let before = files_under(f.project.path());

    let reply = check(
        &mut client,
        f.project.path(),
        vec![
            query("at", &f.merged),
            query("ahead", &f.merged),
            query("no-such-branch", &f.merged),
            query("at", NEVER_FETCHED),
            query("at", "HEAD"),
        ],
    )
    .await;

    assert_eq!(answers(reply).len(), 5, "the check ran");
    let after = files_under(f.project.path());
    assert_eq!(
        before.keys().collect::<Vec<_>>(),
        after.keys().collect::<Vec<_>>(),
        "the check added or removed a file"
    );
    assert!(before == after, "the check changed a file's content");
}
