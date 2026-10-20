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

#[path = "support/conn.rs"]
mod conn;

use conn::{connect, Client};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use mcp_support::{add_worktree, init_repo};
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{
    ClientMsg, DaemonMsg, ErrorKind, OperationResult, ReviewEditOp,
};
use micold_core::review::comment::{
    CommentId, CommentState, Outcome, PrAnchor, Reporter, ReviewComment, Scope,
};
use micold_core::review::store::ReviewFile;
use micold_core::review::Side;
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::review::Reviews;
use micold_daemon::state::DaemonState;

/// How long a test waits for a frame it expects.
const BOUND: Duration = Duration::from_secs(30);
/// How long a test waits to be sure a frame does not come.
const QUIET: Duration = Duration::from_millis(300);

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
        pr: None,
    }
}

fn anchor(number: u64) -> PrAnchor {
    PrAnchor {
        number,
        head_branch: format!("feat/{number}"),
        head_commit: "ab".repeat(20),
    }
}

/// An `Add` on pull request `pr`.
fn add_on(pr: PrAnchor, text: &str) -> ReviewEditOp {
    let mut op = add("src/a.rs", Side::New, 2, 2, &["two"], text);
    if let ReviewEditOp::Add { pr: slot, .. } = &mut op {
        *slot = Some(pr);
    }
    op
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
    assert_eq!(comment.state, CommentState::Draft);
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

// ---- Clear, discard, and removal with the worktree (M8, T091; W4, W11; US4 s3, s5) ----

/// A comment of entry `feat` in state `state`, as the file holds it.
fn stored(n: u128, state: CommentState) -> ReviewComment {
    ReviewComment {
        // Distinct leading digits, so each comment has its own reference (946 research R4).
        id: CommentId(uuid::Uuid::from_u128(n << 96)),
        state,
        ..serde_json::from_value(serde_json::json!({
            "id": "01234567-89ab-4def-8123-456789abcdef", "path": "src/a.rs", "side": "new",
            "start": 2, "end": 2, "quote": ["two"], "text": format!("comment {n}"),
            "state": { "pending": null }, "created": 1_790_000_000u64
        }))
        .unwrap()
    }
}

/// Seed entry `feat` (and the Default entry, untouched by everything here) with `comments`.
fn seed(f: &Fixture, comments: Vec<ReviewComment>) -> ReviewComment {
    let other = stored(99, CommentState::Draft);
    let mut file = ReviewFile::default();
    file.entries.insert("feat".into(), comments);
    file.entries.insert(String::new(), vec![other.clone()]);
    f.files().save_reviews(&f.project(), &file).unwrap();
    other
}

/// Send `msg` and wait for the answer to `req`; also returns the `ReviewChanged` pushes seen
/// meanwhile.
async fn request(client: &mut Client, req: u64, msg: ClientMsg) -> (DaemonMsg, Vec<Pushed>) {
    client.send(Frame::Control(msg)).await.unwrap();
    let mut seen = Vec::new();
    let answered = async {
        loop {
            if let Frame::Control(msg) = client.next().await.expect("stream open").unwrap() {
                if let Some(p) = pushed(&msg) {
                    seen.push(p);
                    continue;
                }
                match &msg {
                    DaemonMsg::OperationOk { req: r, .. }
                    | DaemonMsg::OperationError { req: r, .. }
                        if *r == req =>
                    {
                        return msg
                    }
                    _ => {}
                }
            }
        }
    };
    let answer = tokio::time::timeout(BOUND, answered)
        .await
        .expect("the service answers");
    // A push sent right after the answer.
    while let Ok(Some(Ok(Frame::Control(msg)))) = tokio::time::timeout(QUIET, client.next()).await {
        if let Some(p) = pushed(&msg) {
            seen.push(p);
        }
    }
    (answer, seen)
}

/// Feature 946 W15, FR-023: a `Reanchor` move applies while the comment still holds `from`;
/// `Ack` always; a repeat from another window applies nothing, writes nothing and pushes nothing.
#[tokio::test]
async fn w15_reanchor_applies_a_matching_move_once_and_never_changes_the_rest() {
    use micold_core::review::comment::Move;
    use micold_core::review::LineRange;

    let f = Fixture::new();
    let sent = stored(1, CommentState::Sent { at: 1_790_000_100 });
    seed(&f, vec![sent.clone()]);
    let state = f.service();
    let mut a = connect(&state).await;
    let mut b = connect(&state).await;
    attach(&mut a, &f.project()).await;
    attach(&mut b, &f.project()).await;

    let range = |n| LineRange::new(n, n).unwrap();
    let op = |from, to| ReviewEditOp::Reanchor {
        moves: vec![Move::new(sent.id, range(from), range(to)).unwrap()],
    };

    let (result, seen) = edit(&mut a, 1, &f.project(), "feat", op(2, 5)).await;
    result.expect("acknowledged");
    assert_eq!(seen.len(), 1, "the change is pushed once");
    let stored_now = f.on_disk().entries["feat"][0].clone();
    assert_eq!(stored_now.range, range(5), "written before the answer");
    let expected = ReviewComment {
        range: range(5),
        ..sent.clone()
    };
    assert_eq!(stored_now, expected, "quote, text and state are untouched");

    let after = f.on_disk();
    let (result, seen) = edit(&mut b, 2, &f.project(), "feat", op(9, 12)).await;
    result.expect("a stale move is skipped, not refused");
    assert_eq!(seen.len(), 1, "only a's change reached b");
    assert_eq!(f.on_disk(), after, "nothing applied, nothing written");

    let (result, seen) = edit(&mut b, 3, &f.project(), "feat", op(2, 5)).await;
    result.expect("a repeat is acknowledged too");
    assert!(seen.is_empty(), "nothing applied, nothing pushed");
    assert_eq!(f.on_disk(), after, "nothing applied, nothing written");
}

fn mark(id: CommentId, outcome: Outcome, note: &str) -> ReviewEditOp {
    ReviewEditOp::Mark {
        id,
        outcome,
        note: note.into(),
    }
}

/// Feature 946 W14, US1 s6, FR-006: marking a Sent comment by hand is refused with the kind that
/// says why and stores nothing; accepted, it is on disk before the answer and reaches every window.
#[tokio::test]
async fn w14_a_refused_mark_stores_and_pushes_nothing() {
    let f = Fixture::new();
    let sent = stored(1, CommentState::Sent { at: 1_790_000_100 });
    let draft = stored(2, CommentState::Draft);
    seed(&f, vec![sent.clone(), draft.clone()]);
    let before = f.on_disk();
    let state = f.service();
    let mut a = connect(&state).await;
    attach(&mut a, &f.project()).await;

    let over = "é".repeat(20_001);
    let cases = [
        (
            mark(stored(7, CommentState::Draft).id, Outcome::Applied, ""),
            ErrorKind::NotFound,
        ),
        (mark(draft.id, Outcome::Applied, ""), ErrorKind::Refused),
        (
            mark(sent.id, Outcome::Answered, "  \n "),
            ErrorKind::InvalidInput,
        ),
        (
            mark(sent.id, Outcome::Answered, &over),
            ErrorKind::InvalidInput,
        ),
    ];
    for (req, (op, expected)) in (1..).zip(cases) {
        let (result, seen) = edit(&mut a, req, &f.project(), "feat", op).await;
        let (kind, message) = result.expect_err("refused");
        assert_eq!(kind, expected, "{message}");
        assert!(seen.is_empty(), "nothing is pushed");
        assert_eq!(f.on_disk(), before, "nothing is stored");
    }
}

#[tokio::test]
async fn w14_a_comment_of_a_held_open_send_is_busy_and_a_reported_one_refused() {
    use micold_core::review::prompt::EntryKind;

    let f = Fixture::new();
    let sent = stored(1, CommentState::Sent { at: 1_790_000_100 });
    let draft = stored(2, CommentState::Draft);
    seed(&f, vec![sent.clone(), draft.clone()]);
    let catalog = Catalog::load(
        Box::new(f.files()),
        Box::new(JsonFileSettingsStore::at(
            f.store.path().join("settings.json"),
        )),
    );
    let mut reviews = Reviews::default();
    reviews
        .begin_send(
            &catalog,
            &f.project(),
            "feat",
            EntryKind::Worktree,
            (micold_core::review::comment::Scope::WorkingTree, None),
            &[],
        )
        .expect("the send opens and is held");
    let held = reviews
        .apply_edit(
            &catalog,
            &f.project(),
            "feat",
            mark(draft.id, Outcome::Applied, ""),
            1,
        )
        .expect_err("a comment of the open send");
    assert_eq!(held.kind, ErrorKind::Busy);

    let mut reviews = Reviews::default();
    reviews
        .apply_edit(
            &catalog,
            &f.project(),
            "feat",
            mark(sent.id, Outcome::Applied, ""),
            1,
        )
        .expect("a Sent comment is marked");
    let again = reviews
        .apply_edit(
            &catalog,
            &f.project(),
            "feat",
            mark(sent.id, Outcome::Answered, "no"),
            2,
        )
        .expect_err("already reported");
    assert_eq!(again.kind, ErrorKind::Refused);
}

#[tokio::test]
async fn w14_a_mark_is_on_disk_before_the_answer_and_reaches_every_window() {
    let f = Fixture::new();
    let sent = stored(1, CommentState::Sent { at: 1_790_000_100 });
    seed(&f, vec![sent.clone()]);
    let state = f.service();
    let mut a = connect(&state).await;
    attach(&mut a, &f.project()).await;
    let mut b = connect(&state).await;

    let (result, _) = edit(
        &mut a,
        1,
        &f.project(),
        "feat",
        mark(sent.id, Outcome::Answered, " \r\n kept it\r\n "),
    )
    .await;
    result.expect("a Sent comment can be marked");
    let on_disk = f.on_disk().entries["feat"][0].clone();
    let CommentState::Reported { sent_at, report } = &on_disk.state else {
        panic!("reported on disk: {:?}", on_disk.state);
    };
    assert_eq!(*sent_at, 1_790_000_100);
    assert_eq!(report.outcome(), Outcome::Answered);
    assert_eq!(report.reply(), "kept it");
    assert_eq!(report.by(), Reporter::User);
    assert_eq!(report.first_at(), report.last_at());
    let seen = next_pushed(&mut b).await;
    assert_eq!(
        (seen.worktree_dir.as_str(), seen.comments),
        ("feat", vec![on_disk]),
        "the other window is pushed the marked comment"
    );
}

#[tokio::test]
async fn w4_clear_sent_removes_sent_comments_and_discard_pending_the_pending_ones() {
    let f = Fixture::new();
    let sent = stored(1, CommentState::Sent { at: 1_790_000_100 });
    let pending = stored(2, CommentState::Draft);
    let other = seed(&f, vec![sent, pending.clone()]);
    let state = f.service();
    let mut a = connect(&state).await;
    attach(&mut a, &f.project()).await;
    let mut b = connect(&state).await;

    let (result, _) = edit(
        &mut a,
        1,
        &f.project(),
        "feat",
        ReviewEditOp::ClearSent {
            scope: Scope::WorkingTree,
        },
    )
    .await;
    result.expect("clear sent is accepted (W4)");
    assert_eq!(
        f.on_disk().entries.get("feat").map(Vec::as_slice),
        Some(&[pending.clone()][..]),
        "only the sent comment is gone, on disk before the answer (US4 s3, W5)"
    );
    let seen = next_pushed(&mut b).await;
    assert_eq!(
        (seen.worktree_dir.as_str(), seen.comments),
        ("feat", vec![pending]),
        "every window is pushed the pending comment alone"
    );

    let (result, _) = edit(
        &mut a,
        2,
        &f.project(),
        "feat",
        ReviewEditOp::DiscardPending {
            scope: Scope::WorkingTree,
        },
    )
    .await;
    result.expect("discard pending is accepted (W4)");
    assert!(
        f.on_disk().entries.get("feat").is_none_or(Vec::is_empty),
        "the pending comment is gone from disk (FR-019)"
    );
    let seen = next_pushed(&mut b).await;
    assert_eq!(
        (seen.worktree_dir.as_str(), seen.comments.len()),
        ("feat", 0)
    );
    assert_eq!(
        f.on_disk().entries.get("").map(Vec::as_slice),
        Some(&[other][..]),
        "another entry's comments are untouched (FR-021)"
    );
}

#[tokio::test]
async fn w11_deleting_the_worktree_removes_its_comments_from_memory_and_file_and_pushes_none() {
    let f = Fixture::new();
    let other = seed(
        &f,
        vec![
            stored(1, CommentState::Draft),
            stored(2, CommentState::Sent { at: 1_790_000_100 }),
        ],
    );
    let state = f.service();
    let mut a = connect(&state).await;
    let on_attach = attach(&mut a, &f.project()).await;
    assert!(on_attach.iter().any(|p| p.worktree_dir == "feat"));

    let (answer, seen) = request(
        &mut a,
        1,
        ClientMsg::WorktreeDelete {
            req: 1,
            project: f.project(),
            dir_name: "feat".into(),
            stop_sessions: true,
            delete_branch: true,
        },
    )
    .await;
    assert!(
        matches!(answer, DaemonMsg::OperationOk { .. }),
        "the worktree is deleted: {answer:?}"
    );
    assert!(
        seen.iter()
            .any(|p| p.worktree_dir == "feat" && p.comments.is_empty() && !p.sending),
        "an empty ReviewChanged is pushed for the deleted worktree (W11): {seen:?}"
    );
    assert!(
        !f.on_disk().entries.contains_key("feat"),
        "its comments are gone from the file (FR-020)"
    );
    assert_eq!(
        f.on_disk().entries.get("").map(Vec::as_slice),
        Some(&[other.clone()][..]),
        "the Default entry keeps its own"
    );

    // Gone from memory too: this service on attach, and a restarted one, push the Default
    // entry's comment and nothing for `feat`.
    let with_comments = |pushes: Vec<Pushed>| {
        pushes
            .into_iter()
            .filter(|p| !p.comments.is_empty())
            .map(|p| (p.worktree_dir, p.comments))
            .collect::<Vec<_>>()
    };
    let mut again = connect(&state).await;
    assert_eq!(
        with_comments(attach(&mut again, &f.project()).await),
        vec![(String::new(), vec![other.clone()])],
        "attach pushes the Default entry's comment and none for the deleted worktree (US4 s5)"
    );
    let restarted = f.service();
    let mut client = connect(&restarted).await;
    assert_eq!(
        with_comments(attach(&mut client, &f.project()).await),
        vec![(String::new(), vec![other])],
        "a restarted service has no comments for the deleted worktree either (US4 s5)"
    );
}

/// `forget_review_worktree` alone, with no worktree refresh after it to prune the entry: the
/// comments leave memory and the review file, and the empty push goes out (W11, FR-020).
#[tokio::test]
async fn w11_forgetting_a_worktree_drops_its_comments_from_memory_and_file_at_once() {
    let f = Fixture::new();
    let other = seed(&f, vec![stored(1, CommentState::Draft)]);
    let state = f.service();
    let mut a = connect(&state).await;
    let on_attach = attach(&mut a, &f.project()).await;
    assert!(
        on_attach.iter().any(|p| p.worktree_dir == "feat"),
        "`feat`'s comment is in memory before: {on_attach:?}"
    );

    state.forget_review_worktree(&f.project(), "feat");
    let seen = next_pushed(&mut a).await;
    assert_eq!(
        (seen.worktree_dir.as_str(), seen.comments.len()),
        ("feat", 0),
        "the empty ReviewChanged is pushed (W11)"
    );
    assert!(
        !f.on_disk().entries.contains_key("feat"),
        "its comments are gone from the file (FR-020)"
    );
    let left: Vec<(String, usize)> = state
        .review_pushes_on_attach(&f.project())
        .iter()
        .filter_map(pushed)
        .map(|p| (p.worktree_dir, p.comments.len()))
        .collect();
    assert_eq!(
        left,
        vec![(String::new(), 1)],
        "memory keeps only the Default entry's comment"
    );
    assert_eq!(
        f.on_disk().entries.get("").map(Vec::as_slice),
        Some(&[other][..]),
        "the Default entry keeps its own"
    );
}

#[tokio::test]
async fn a_worktree_removed_outside_the_app_loses_its_comments_on_the_next_refresh() {
    let f = Fixture::new();
    let other = seed(&f, vec![stored(1, CommentState::Draft)]);
    let state = f.service();
    let mut a = connect(&state).await;
    attach(&mut a, &f.project()).await;

    let status = std::process::Command::new("git")
        .current_dir(f.project())
        .args(["worktree", "remove", "--force", ".claude/worktrees/feat"])
        .status()
        .unwrap();
    assert!(status.success(), "git removed the worktree");
    let refreshed = Arc::clone(&state);
    let project = f.project();
    tokio::task::spawn_blocking(move || refreshed.refresh_worktrees(&project))
        .await
        .unwrap();

    let seen = next_pushed(&mut a).await;
    assert_eq!(
        (seen.worktree_dir.as_str(), seen.comments.len()),
        ("feat", 0),
        "the removed worktree's comments are pushed as none (Edge: removed worktree)"
    );
    assert!(
        !f.on_disk().entries.contains_key("feat"),
        "and gone from the file"
    );
    assert_eq!(
        f.on_disk().entries.get("").map(Vec::as_slice),
        Some(&[other][..]),
        "the Default entry is never pruned"
    );
}

#[tokio::test]
async fn an_unreadable_repository_prunes_nothing_on_refresh() {
    // Review A M8 F1: an unmounted drive or a renamed folder is not every worktree removed.
    let f = Fixture::new();
    seed(&f, vec![stored(1, CommentState::Draft)]);
    let state = f.service();
    let mut a = connect(&state).await;
    attach(&mut a, &f.project()).await;

    let away = f.project().with_extension("away");
    std::fs::rename(f.project(), &away).unwrap();
    let refreshed = Arc::clone(&state);
    let project = f.project();
    tokio::task::spawn_blocking(move || refreshed.refresh_worktrees(&project))
        .await
        .unwrap();
    std::fs::rename(&away, f.project()).unwrap();

    assert_eq!(
        f.on_disk().entries.get("feat").map(Vec::len),
        Some(1),
        "the worktree's comments stay while its repository cannot be read"
    );
}

#[tokio::test]
async fn removing_the_project_from_the_catalog_deletes_its_review_file() {
    let f = Fixture::new();
    seed(&f, vec![stored(1, CommentState::Draft)]);
    let file = f.files().reviews_path(&f.project());
    assert!(file.exists(), "seeded");
    let state = f.service();
    let mut a = connect(&state).await;
    attach(&mut a, &f.project()).await;

    let (answer, _) = request(
        &mut a,
        1,
        ClientMsg::ProjectRemove {
            req: 1,
            path: f.project(),
        },
    )
    .await;
    assert!(
        matches!(answer, DaemonMsg::OperationOk { .. }),
        "the project is removed: {answer:?}"
    );
    assert!(!file.exists(), "its reviews/ file is deleted (W11)");
}

// ---- Feature 1149: comments on a pull request's diff (W18, W19, W21) ----

/// `anchor(number)` as a comment of entry `feat`, in `state`.
fn stored_pr(n: u128, state: CommentState, number: u64) -> ReviewComment {
    ReviewComment {
        pr: Some(anchor(number)),
        ..stored(n, state)
    }
}

/// W18: each refusal is `InvalidInput`, nothing is stored or pushed; a valid anchor is stored
/// with the commit lower-cased.
#[tokio::test]
async fn w18_a_pull_request_add_is_checked_and_stored_with_its_anchor() {
    let f = Fixture::new();
    let state = f.service();
    let mut client = connect(&state).await;
    attach(&mut client, &f.project()).await;

    let with = |f: fn(&mut PrAnchor)| {
        let mut pr = anchor(7);
        f(&mut pr);
        add_on(pr, "t")
    };
    let refused: [(&str, &str, ReviewEditOp); 6] = [
        ("the Default entry", "", add_on(anchor(7), "t")),
        ("number 0", "feat", with(|p| p.number = 0)),
        ("an empty branch", "feat", with(|p| p.head_branch.clear())),
        (
            "a control character",
            "feat",
            with(|p| p.head_branch = "a\nb".into()),
        ),
        (
            "a short commit",
            "feat",
            with(|p| p.head_commit = "ab".repeat(10)),
        ),
        (
            "a non-hex commit",
            "feat",
            with(|p| p.head_commit = "zz".repeat(20)),
        ),
    ];
    for (n, (why, dir, op)) in refused.into_iter().enumerate() {
        let (result, seen) = edit(&mut client, n as u64 + 1, &f.project(), dir, op).await;
        assert_eq!(result.expect_err(why).0, ErrorKind::InvalidInput, "{why}");
        assert!(seen.is_empty(), "{why}: nothing is pushed");
    }
    assert_eq!(f.on_disk(), ReviewFile::default(), "nothing was stored");

    let mut upper = anchor(7);
    upper.head_commit = "AB".repeat(20);
    let (result, seen) = edit(&mut client, 9, &f.project(), "feat", add_on(upper, "kept")).await;
    result.expect("a valid anchor is accepted");
    let comment = seen.last().expect("pushed").comments[0].clone();
    assert_eq!(comment.pr, Some(anchor(7)), "stored lower-cased");
    assert_eq!(f.on_disk().entries["feat"], vec![comment], "and on disk");
}

/// W19, US1 s2, FR-005: scoped tidy touches only its scope and writes nothing when nothing
/// changed; a pull request scope on the Default entry is a no-op `Ack`.
#[tokio::test]
async fn w19_scoped_clear_and_discard_isolate_scopes_and_skip_no_op_writes() {
    let f = Fixture::new();
    let sent = CommentState::Sent { at: 1_790_000_100 };
    seed(
        &f,
        vec![
            stored_pr(1, sent.clone(), 7),
            stored_pr(2, CommentState::Draft, 7),
            stored_pr(3, sent.clone(), 8),
            stored(4, sent),
        ],
    );
    let state = f.service();
    let mut client = connect(&state).await;
    attach(&mut client, &f.project()).await;

    let (result, _) = edit(
        &mut client,
        1,
        &f.project(),
        "feat",
        ReviewEditOp::ClearSent {
            scope: Scope::PullRequest(7),
        },
    )
    .await;
    result.expect("accepted");
    let ids = |f: &Fixture| -> Vec<u128> {
        f.on_disk().entries["feat"]
            .iter()
            .map(|c| c.id.0.as_u128() >> 96)
            .collect()
    };
    assert_eq!(ids(&f), vec![2, 3, 4], "only PR 7's sent comment went");

    let before = f.on_disk();
    for (req, dir, op) in [
        (
            2,
            "feat",
            ReviewEditOp::ClearSent {
                scope: Scope::PullRequest(9),
            },
        ),
        (
            3,
            "feat",
            ReviewEditOp::DiscardPending {
                scope: Scope::WorkingTree,
            },
        ),
        (
            4,
            "",
            ReviewEditOp::DiscardPending {
                scope: Scope::PullRequest(7),
            },
        ),
    ] {
        let (result, seen) = edit(&mut client, req, &f.project(), dir, op).await;
        result.expect("a no-op is acknowledged");
        assert!(seen.is_empty(), "nothing changed, nothing pushed");
        assert_eq!(f.on_disk(), before, "nothing changed, nothing written");
    }

    let (result, _) = edit(
        &mut client,
        5,
        &f.project(),
        "feat",
        ReviewEditOp::DiscardPending {
            scope: Scope::PullRequest(7),
        },
    )
    .await;
    result.expect("accepted");
    assert_eq!(ids(&f), vec![3, 4], "PR 7's draft went, the rest stayed");
}

/// US1 s2: a PR comment is edited and deleted as a working-tree one, and goes with its worktree.
#[tokio::test]
async fn a_pull_request_comment_is_edited_deleted_and_forgotten_with_its_worktree() {
    let f = Fixture::new();
    seed(
        &f,
        vec![
            stored_pr(1, CommentState::Draft, 7),
            stored_pr(2, CommentState::Draft, 7),
        ],
    );
    let state = f.service();
    let mut client = connect(&state).await;
    attach(&mut client, &f.project()).await;
    let id = |n: u128| CommentId(uuid::Uuid::from_u128(n << 96));

    let (result, _) = edit(
        &mut client,
        1,
        &f.project(),
        "feat",
        ReviewEditOp::SetText {
            id: id(1),
            text: " new ".into(),
        },
    )
    .await;
    result.expect("edited");
    let (result, _) = edit(
        &mut client,
        2,
        &f.project(),
        "feat",
        ReviewEditOp::Delete { id: id(2) },
    )
    .await;
    result.expect("deleted");
    let kept = f.on_disk().entries["feat"].clone();
    assert_eq!((kept.len(), kept[0].text.as_str()), (1, "new"));
    assert_eq!(kept[0].pr, Some(anchor(7)), "the anchor stays");

    let (answer, _) = request(
        &mut client,
        3,
        ClientMsg::WorktreeDelete {
            req: 3,
            project: f.project(),
            dir_name: "feat".into(),
            stop_sessions: true,
            delete_branch: true,
        },
    )
    .await;
    assert!(
        matches!(answer, DaemonMsg::OperationOk { .. }),
        "{answer:?}"
    );
    assert!(
        f.on_disk().entries.get("feat").is_none_or(Vec::is_empty),
        "FR-005: the worktree's PR comments go with it"
    );
}

/// W21: `Reanchor` of a PR comment is compare-and-set on `from`.
#[tokio::test]
async fn w21_reanchor_moves_a_pull_request_comment_once() {
    use micold_core::review::comment::Move;
    use micold_core::review::LineRange;

    let f = Fixture::new();
    let c = stored_pr(1, CommentState::Draft, 7);
    seed(&f, vec![c.clone()]);
    let state = f.service();
    let mut client = connect(&state).await;
    attach(&mut client, &f.project()).await;
    let range = |n| LineRange::new(n, n).unwrap();
    let op = |from, to| ReviewEditOp::Reanchor {
        moves: vec![Move::new(c.id, range(from), range(to)).unwrap()],
    };

    let (result, seen) = edit(&mut client, 1, &f.project(), "feat", op(2, 6)).await;
    result.expect("acknowledged");
    assert_eq!(seen.len(), 1);
    assert_eq!(f.on_disk().entries["feat"][0].range, range(6));
    let after = f.on_disk();
    let (result, seen) = edit(&mut client, 2, &f.project(), "feat", op(2, 9)).await;
    result.expect("a stale `from` is skipped");
    assert!(seen.is_empty());
    assert_eq!(f.on_disk(), after);
}
