//! Review comments on the service (feature 482, contracts/review-wire.md W1–W3, W5, W12; US2
//! scenarios 1, 2, 7; US4 scenario 4): `ReviewEdit` checks its input, stores the change in
//! `reviews/<project id>.json` before answering, and pushes the entry's comments as
//! `ReviewChanged` to every window, after each edit and on attach. A restarted service has them
//! back.
//!
//! Drives `server::serve_connection` over in-memory duplexes against a real git repository with
//! worktree `feat`, and a catalog and settings in a temporary directory, so "kept" is checked by
//! starting a second service over that directory.

#[path = "support/mcp.rs"]
mod mcp_support;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use mcp_support::{add_worktree, init_repo};
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{
    ClientMsg, DaemonMsg, ErrorKind, OperationResult, ReviewEditOp,
};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_core::review::comment::{CommentId, CommentState, ReviewComment};
use micold_core::review::store::ReviewFile;
use micold_core::review::Side;
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use tokio_util::codec::Framed;

/// How long a test waits for a frame it expects.
const BOUND: Duration = Duration::from_secs(30);
/// How long a test waits to be sure a frame does not come.
const QUIET: Duration = Duration::from_millis(300);

type Client = Framed<tokio::io::DuplexStream, ClientCodec>;

/// A repository with worktree `feat`, and a store directory whose catalog lists it.
struct Fixture {
    repo: tempfile::TempDir,
    store: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let repo = tempfile::tempdir().unwrap();
        init_repo(repo.path());
        add_worktree(repo.path(), "feat");
        let store = tempfile::tempdir().unwrap();
        let workspace = Workspace {
            projects: vec![Project::new(
                repo.path().to_path_buf(),
                true,
                Availability::Available,
            )],
            active: Some(repo.path().to_path_buf()),
            ..Default::default()
        };
        JsonFileStore::at(store.path().join("projects.json"))
            .save(&workspace)
            .unwrap();
        Self { repo, store }
    }

    fn project(&self) -> PathBuf {
        self.repo.path().to_path_buf()
    }

    fn files(&self) -> JsonFileStore {
        JsonFileStore::at(self.store.path().join("projects.json"))
    }

    /// A service over the store directory. Called twice, the second is the first restarted.
    fn service(&self) -> Arc<DaemonState> {
        Arc::new(DaemonState::new(Catalog::load(
            Box::new(self.files()),
            Box::new(JsonFileSettingsStore::at(
                self.store.path().join("settings.json"),
            )),
        )))
    }

    /// What the review file holds now.
    fn on_disk(&self) -> ReviewFile {
        self.files().load_reviews(&self.project())
    }
}

/// Connect and complete the handshake.
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
        Frame::Control(DaemonMsg::Welcome { .. }) => client,
        other => panic!("expected Welcome, got {other:?}"),
    }
}

/// One `ReviewChanged` push.
#[derive(Debug, Clone, PartialEq)]
struct Pushed {
    project: PathBuf,
    worktree_dir: String,
    comments: Vec<ReviewComment>,
    sending: bool,
}

fn pushed(msg: &DaemonMsg) -> Option<Pushed> {
    match msg {
        DaemonMsg::ReviewChanged {
            project,
            worktree_dir,
            comments,
            sending,
        } => Some(Pushed {
            project: project.clone(),
            worktree_dir: worktree_dir.clone(),
            comments: comments.clone(),
            sending: *sending,
        }),
        _ => None,
    }
}

/// Attach to `project` and return the `ReviewChanged` pushes that arrive with it.
async fn attach(client: &mut Client, project: &Path) -> Vec<Pushed> {
    client
        .send(Frame::Control(ClientMsg::Attach {
            project: project.to_path_buf(),
            force: true,
        }))
        .await
        .unwrap();
    let mut seen = Vec::new();
    let attached = async {
        loop {
            if let Frame::Control(msg) = client.next().await.expect("stream open").unwrap() {
                if let Some(p) = pushed(&msg) {
                    seen.push(p);
                }
                if matches!(msg, DaemonMsg::CatalogChanged { .. }) {
                    return;
                }
            }
        }
    };
    tokio::time::timeout(BOUND, attached)
        .await
        .expect("attach completes with its catalog");
    // Anything still in flight right after.
    while let Ok(Some(Ok(Frame::Control(msg)))) = tokio::time::timeout(QUIET, client.next()).await {
        if let Some(p) = pushed(&msg) {
            seen.push(p);
        }
    }
    seen
}

/// Send `edit` and wait for its answer; also returns the `ReviewChanged` pushes seen meanwhile.
async fn edit(
    client: &mut Client,
    req: u64,
    project: &Path,
    worktree_dir: &str,
    edit: ReviewEditOp,
) -> (Result<(), (ErrorKind, String)>, Vec<Pushed>) {
    client
        .send(Frame::Control(ClientMsg::ReviewEdit {
            req,
            project: project.to_path_buf(),
            worktree_dir: worktree_dir.into(),
            edit,
        }))
        .await
        .unwrap();
    let mut seen = Vec::new();
    let answered = async {
        loop {
            if let Frame::Control(msg) = client.next().await.expect("stream open").unwrap() {
                if let Some(p) = pushed(&msg) {
                    seen.push(p);
                    continue;
                }
                match msg {
                    DaemonMsg::OperationOk {
                        req: r,
                        result: OperationResult::Ack,
                    } if r == req => return Ok(()),
                    DaemonMsg::OperationError {
                        req: r,
                        kind,
                        message,
                        ..
                    } if r == req => return Err((kind, message)),
                    _ => {}
                }
            }
        }
    };
    let result = tokio::time::timeout(BOUND, answered)
        .await
        .expect("the service answers the edit");
    (result, seen)
}

/// The next `ReviewChanged` this client is pushed.
async fn next_pushed(client: &mut Client) -> Pushed {
    let next = async {
        loop {
            if let Frame::Control(msg) = client.next().await.expect("stream open").unwrap() {
                if let Some(p) = pushed(&msg) {
                    return p;
                }
            }
        }
    };
    tokio::time::timeout(BOUND, next)
        .await
        .expect("a ReviewChanged is pushed")
}

fn add(path: &str, side: Side, start: u32, end: u32, quote: &[&str], text: &str) -> ReviewEditOp {
    ReviewEditOp::Add {
        path: path.into(),
        side,
        start,
        end,
        quote: quote.iter().map(|line| (*line).to_owned()).collect(),
        text: text.into(),
    }
}

/// Add one valid comment to `feat` and return it as pushed.
async fn add_one(client: &mut Client, req: u64, project: &Path, text: &str) -> ReviewComment {
    let (result, seen) = edit(
        client,
        req,
        project,
        "feat",
        add("src/a.rs", Side::New, 2, 3, &["two", "three"], text),
    )
    .await;
    result.expect("a valid comment is accepted");
    let last = seen
        .last()
        .expect("the editing window is pushed the change too");
    last.comments
        .iter()
        .find(|c| c.text == text.trim())
        .cloned()
        .expect("the new comment is in the push")
}

#[tokio::test]
async fn w1_an_invalid_add_is_invalid_input_and_nothing_is_stored() {
    let f = Fixture::new();
    let state = f.service();
    let mut client = connect(&state).await;
    attach(&mut client, &f.project()).await;

    let refused = [
        (
            "an absolute path",
            add("/etc/passwd", Side::New, 1, 1, &["x"], "t"),
        ),
        (
            "a `\\` in the path",
            add("src\\a.rs", Side::New, 1, 1, &["x"], "t"),
        ),
        ("line 0", add("a.rs", Side::New, 0, 1, &["x", "y"], "t")),
        (
            "start after end",
            add("a.rs", Side::New, 3, 2, &["x", "y"], "t"),
        ),
        (
            "a short quote",
            add("a.rs", Side::Old, 1, 3, &["x", "y"], "t"),
        ),
        ("empty text", add("a.rs", Side::New, 1, 1, &["x"], " \n ")),
    ];
    for (n, (why, op)) in refused.into_iter().enumerate() {
        let (result, seen) = edit(&mut client, n as u64 + 1, &f.project(), "feat", op).await;
        let (kind, _) = result.expect_err(why);
        assert_eq!(kind, ErrorKind::InvalidInput, "{why} is InvalidInput (W1)");
        assert!(seen.is_empty(), "{why}: a refused edit pushes nothing");
    }
    assert_eq!(f.on_disk(), ReviewFile::default(), "nothing was stored");
}

#[tokio::test]
async fn w2_an_entry_that_is_not_in_the_catalog_is_not_found_and_nothing_is_stored() {
    let f = Fixture::new();
    let state = f.service();
    let mut client = connect(&state).await;
    attach(&mut client, &f.project()).await;

    let op = || add("a.rs", Side::New, 1, 1, &["x"], "t");
    let (result, _) = edit(&mut client, 1, &f.project(), "nope", op()).await;
    assert_eq!(
        result.expect_err("an unknown worktree").0,
        ErrorKind::NotFound,
        "a worktree the project does not have is NotFound (W2)"
    );
    let elsewhere = tempfile::tempdir().unwrap();
    let (result, _) = edit(&mut client, 2, elsewhere.path(), "", op()).await;
    assert_eq!(
        result.expect_err("an unknown project").0,
        ErrorKind::NotFound,
        "a project the catalog does not list is NotFound"
    );
    assert_eq!(f.on_disk(), ReviewFile::default(), "nothing was stored");
}

#[tokio::test]
async fn w3_set_text_and_delete_reach_only_their_own_entrys_comments() {
    let f = Fixture::new();
    let state = f.service();
    let mut client = connect(&state).await;
    attach(&mut client, &f.project()).await;
    let comment = add_one(&mut client, 1, &f.project(), "keep me").await;

    let cases = [
        (
            "SetText through the Default entry",
            "",
            ReviewEditOp::SetText {
                id: comment.id,
                text: "hijack".into(),
            },
        ),
        (
            "Delete through the Default entry",
            "",
            ReviewEditOp::Delete { id: comment.id },
        ),
        (
            "SetText of an unknown id",
            "feat",
            ReviewEditOp::SetText {
                id: CommentId::new(),
                text: "x".into(),
            },
        ),
        (
            "Delete of an unknown id",
            "feat",
            ReviewEditOp::Delete {
                id: CommentId::new(),
            },
        ),
    ];
    for (n, (why, dir, op)) in cases.into_iter().enumerate() {
        let (result, _) = edit(&mut client, n as u64 + 2, &f.project(), dir, op).await;
        assert_eq!(
            result.expect_err(why).0,
            ErrorKind::NotFound,
            "{why} (W3, FR-021)"
        );
    }
    assert_eq!(
        f.on_disk().entries.get("feat").map(Vec::as_slice),
        Some(&[comment][..]),
        "the comment is untouched"
    );
}

#[tokio::test]
async fn w5_each_edit_is_on_disk_before_its_answer_and_reaches_every_window() {
    let f = Fixture::new();
    let state = f.service();
    let mut a = connect(&state).await;
    attach(&mut a, &f.project()).await;
    let mut b = connect(&state).await;

    let comment = add_one(&mut a, 1, &f.project(), "  Why clone here?  ").await;
    assert_eq!(comment.text, "Why clone here?", "stored trimmed (W1)");
    assert_eq!(comment.state, CommentState::Pending);
    assert_eq!(
        (comment.side, comment.range.start(), comment.range.end()),
        (Side::New, 2, 3)
    );
    assert_eq!(
        f.on_disk().entries.get("feat").map(Vec::as_slice),
        Some(&[comment.clone()][..]),
        "the comment was on disk when the edit was answered (W5)"
    );
    let seen = next_pushed(&mut b).await;
    assert_eq!(
        (
            seen.project.as_path(),
            seen.worktree_dir.as_str(),
            seen.sending
        ),
        (f.project().as_path(), "feat", false),
        "the other window is told which entry changed"
    );
    assert_eq!(
        seen.comments,
        vec![comment.clone()],
        "and holds the new comment"
    );

    let (result, _) = edit(
        &mut a,
        2,
        &f.project(),
        "feat",
        ReviewEditOp::SetText {
            id: comment.id,
            text: "Clone is fine".into(),
        },
    )
    .await;
    result.expect("a pending comment can be edited (US2 s7)");
    assert_eq!(
        f.on_disk().entries["feat"][0].text,
        "Clone is fine",
        "the edit is on disk"
    );
    assert_eq!(next_pushed(&mut b).await.comments[0].text, "Clone is fine");

    let (result, _) = edit(
        &mut a,
        3,
        &f.project(),
        "feat",
        ReviewEditOp::Delete { id: comment.id },
    )
    .await;
    result.expect("a pending comment can be deleted (US2 s7)");
    assert!(
        f.on_disk().entries.get("feat").is_none_or(Vec::is_empty),
        "the deletion is on disk"
    );
    let gone = next_pushed(&mut b).await;
    assert_eq!(
        (gone.worktree_dir.as_str(), gone.comments.len()),
        ("feat", 0),
        "an entry whose last comment went is pushed as an empty list"
    );
}

#[tokio::test]
async fn us4_s4_comments_and_their_states_are_back_after_a_restart_and_pushed_on_attach() {
    let f = Fixture::new();
    // A sent comment from an earlier run, as the file holds it.
    let sent = ReviewComment {
        state: CommentState::Sent { at: 1_790_000_100 },
        ..serde_json::from_value(serde_json::json!({
            "id": "01234567-89ab-4def-8123-456789abcdef", "path": "src/lib.rs", "side": "old",
            "start": 4, "end": 4, "quote": ["gone"], "text": "Why remove this?",
            "state": { "pending": null }, "created": 1_790_000_000u64
        }))
        .unwrap()
    };
    let mut seed = ReviewFile::default();
    seed.entries.insert(String::new(), vec![sent.clone()]);
    f.files().save_reviews(&f.project(), &seed).unwrap();

    let first = f.service();
    let mut client = connect(&first).await;
    let on_attach = attach(&mut client, &f.project()).await;
    assert!(
        on_attach
            .iter()
            .any(|p| p.worktree_dir.is_empty() && p.comments == vec![sent.clone()]),
        "attach pushes the stored comments of each entry: {on_attach:?}"
    );
    let pending = add_one(&mut client, 1, &f.project(), "Pending one").await;
    drop(client);
    drop(first);

    let restarted = f.service();
    let mut client = connect(&restarted).await;
    let mut on_attach = attach(&mut client, &f.project()).await;
    on_attach.sort_by(|x, y| x.worktree_dir.cmp(&y.worktree_dir));
    let entries: Vec<(String, Vec<ReviewComment>)> = on_attach
        .into_iter()
        .map(|p| (p.worktree_dir, p.comments))
        .collect();
    assert_eq!(
        entries,
        vec![(String::new(), vec![sent]), ("feat".into(), vec![pending])],
        "the same comments with the same states come back after a restart (US4 s4)"
    );
}
