//! Sending review comments to a running session (feature 482, contracts/review-wire.md W6–W9,
//! research R4, R5; US2 scenarios 3, 5, 6; FR-014, FR-017, FR-018, FR-021; US3 scenario 4).
//!
//! A real git repository with worktrees `wt` and `other`, sessions whose stand-in `claude` records
//! what is typed into it in `<bin>/input.<session id>`, and a window connected over an in-memory
//! duplex. The stand-in turns bracketed paste on, as the real CLIs do, unless
//! `<bin>/nopaste.<session id>` exists.

// unix-only: the stand-in CLI is a `#!/bin/sh` script, as in `mcp_create_session.rs`.
#![cfg(unix)]

#[path = "support/mcp.rs"]
mod mcp_support;

#[path = "support/runs.rs"]
mod runs_support;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use mcp_support::{add_worktree, init_repo, session, sid, state_over};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{
    ClientMsg, DaemonMsg, ErrorKind, OperationResult, ReviewEditOp,
};
use micold_core::review::comment::{CommentState, ReviewComment};
use micold_core::review::prompt::{self, EntryKind};
use micold_core::review::Side;
use micold_core::session::{AiCli, SessionId, SessionLocation, TerminalMode};
use micold_core::terminal::LaunchMode;
use micold_daemon::activity::{ActivityEvent, HookKind};
use micold_daemon::state::DaemonState;
use runs_support::{install_claude, submitted, window, Client, Env, BOUND, ENV, QUIET};

/// S1 and S2 in `wt`, S3 in `other`, all `claude`.
struct Sandbox {
    _env: Env,
    bin: tempfile::TempDir,
    _home: tempfile::TempDir,
    project: tempfile::TempDir,
    _store: tempfile::TempDir,
    state: Arc<DaemonState>,
}

impl Sandbox {
    async fn new() -> Self {
        let bin = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        let store = tempfile::tempdir().unwrap();
        install_claude(bin.path());
        init_repo(project.path());
        add_worktree(project.path(), "wt");
        add_worktree(project.path(), "other");
        let trust = serde_json::json!({"projects": {project.path().to_str().unwrap(): {"hasTrustDialogAccepted": true}}});
        std::fs::write(home.path().join(".claude.json"), trust.to_string()).unwrap();
        let mut path = vec![bin.path().to_path_buf()];
        path.extend(["/usr/bin", "/bin"].map(PathBuf::from));
        let env = Env::set(&[
            ("PATH", Some(std::env::join_paths(path).unwrap())),
            ("HOME", Some(home.path().into())),
            (
                "XDG_DATA_HOME",
                Some(home.path().join(".local/share").into()),
            ),
            ("CLAUDE_CONFIG_DIR", None),
            ("MICOLD_IMAGE_REFERENCE", None),
        ]);
        let state = state_over(
            vec![(
                project.path().to_path_buf(),
                true,
                vec![
                    session(sid(1), Some("wt"), TerminalMode::AiCli, AiCli::ClaudeCode),
                    session(sid(2), Some("wt"), TerminalMode::AiCli, AiCli::ClaudeCode),
                    session(
                        sid(3),
                        Some("other"),
                        TerminalMode::AiCli,
                        AiCli::ClaudeCode,
                    ),
                ],
            )],
            store.path(),
        );
        Self {
            _env: env,
            bin,
            _home: home,
            project,
            _store: store,
            state,
        }
    }

    fn project(&self) -> PathBuf {
        self.project.path().to_path_buf()
    }

    /// Start `id` and wait until its stand-in is reading input (bracketed paste already asked
    /// for, unless `nopaste`).
    async fn start(&self, id: SessionId, nopaste: bool) {
        if nopaste {
            std::fs::write(self.bin.path().join(format!("nopaste.{}", id.0)), "").unwrap();
        }
        self.state.begin_start(id);
        let started = micold_daemon::ops::start_session(
            &self.state,
            id,
            LaunchMode::Fresh,
            // The helper is used to run several sessions of one entry side by side on purpose.
            micold_core::occupancy::Additional::Yes,
        )
        .await
        .unwrap();
        assert!(started, "session {} starts", id.0);
        let input = self.input_path(id);
        let deadline = Instant::now() + BOUND;
        while !input.exists() {
            assert!(Instant::now() < deadline, "{} never read input", id.0);
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        // The service's terminal has read the stand-in's mode switch by now.
        tokio::time::sleep(QUIET).await;
    }

    fn input_path(&self, id: SessionId) -> PathBuf {
        self.bin.path().join(format!("input.{}", id.0))
    }

    /// What was typed into `id` so far.
    fn typed(&self, id: SessionId) -> String {
        std::fs::read_to_string(self.input_path(id)).unwrap_or_default()
    }

    /// What was typed into every session that read input so far, sessions the service started
    /// itself included.
    fn inputs(&self) -> Vec<(SessionId, String)> {
        std::fs::read_dir(self.bin.path())
            .unwrap()
            .filter_map(|entry| {
                let name = entry.ok()?.file_name().into_string().ok()?;
                let id = uuid::Uuid::parse_str(name.strip_prefix("input.")?).ok()?;
                let id = SessionId::from_uuid(id);
                Some((id, self.typed(id)))
            })
            .collect()
    }

    /// The stand-in waits `secs` before it draws anything.
    fn slow(&self, secs: f32) {
        std::fs::write(self.bin.path().join("slow"), secs.to_string()).unwrap();
    }

    /// Wait until what was typed into `id` holds `needle`.
    async fn typed_holds(&self, id: SessionId, needle: &str) -> String {
        let deadline = Instant::now() + BOUND;
        loop {
            let typed = self.typed(id);
            if typed.contains(needle) {
                return typed;
            }
            assert!(
                Instant::now() < deadline,
                "{} never received {needle:?}; it has {typed:?}",
                id.0
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let mut ids: Vec<SessionId> = (1..=3).map(sid).collect();
        ids.extend(self.inputs().into_iter().map(|(id, _)| id));
        for id in ids {
            if let Some(pty) = self.state.live_session(id) {
                let _ = pty.kill();
            }
        }
    }
}

/// One `ReviewChanged` push: the entry, its comments and whether a send is open.
type Pushed = (String, Vec<ReviewComment>, bool);

/// Send `msg` (carrying `req`) and wait for its answer, collecting the pushes seen meanwhile.
async fn request(
    client: &mut Client,
    req: u64,
    msg: ClientMsg,
) -> (Result<OperationResult, (ErrorKind, String)>, Vec<Pushed>) {
    client.send(Frame::Control(msg)).await.unwrap();
    answer(client, req).await
}

/// Wait for the answer to `req`, collecting the pushes seen meanwhile.
async fn answer(
    client: &mut Client,
    req: u64,
) -> (Result<OperationResult, (ErrorKind, String)>, Vec<Pushed>) {
    let mut seen = Vec::new();
    let answered = async {
        loop {
            let Frame::Control(msg) = client.next().await.expect("stream open").unwrap() else {
                continue;
            };
            match msg {
                DaemonMsg::ReviewChanged {
                    worktree_dir,
                    comments,
                    sending,
                    ..
                } => seen.push((worktree_dir, comments, sending)),
                DaemonMsg::OperationOk { req: r, result } if r == req => return Ok(result),
                DaemonMsg::OperationError {
                    req: r,
                    kind,
                    message,
                    ..
                } if r == req => return Err((kind, message)),
                _ => {}
            }
        }
    };
    let result = tokio::time::timeout(BOUND, answered)
        .await
        .expect("the service answers");
    (result, seen)
}

#[allow(clippy::too_many_arguments)] // a test helper naming every field of one comment
/// The working-tree directory of entry `dir`.
fn entry_dir(project: &Path, dir: &str) -> PathBuf {
    let location = if dir.is_empty() {
        SessionLocation::Default
    } else {
        SessionLocation::Worktree(dir.into())
    };
    location.cwd(project)
}

/// Make `path` of entry `dir` hold `quote` from line `start` (feature 946 W16: a Draft is judged
/// against the file on disk), keeping the other lines it already has.
fn place_quote(project: &Path, dir: &str, path: &str, start: u32, quote: &[&str]) {
    let file = entry_dir(project, dir).join(path);
    let text = std::fs::read_to_string(&file).unwrap_or_default();
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let first = start as usize - 1;
    if lines.len() < first + quote.len() {
        lines.resize(first + quote.len(), String::new());
    }
    for (offset, line) in quote.iter().enumerate() {
        lines[first + offset] = (*line).to_owned();
    }
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, lines.join("\n") + "\n").unwrap();
}

#[allow(clippy::too_many_arguments)] // a test helper: one argument per field of the comment
async fn add(
    client: &mut Client,
    req: u64,
    project: &Path,
    dir: &str,
    path: &str,
    start: u32,
    quote: &[&str],
    text: &str,
) -> Vec<ReviewComment> {
    let end = start + quote.len() as u32 - 1;
    place_quote(project, dir, path, start, quote);
    let (result, seen) = request(
        client,
        req,
        ClientMsg::ReviewEdit {
            req,
            project: project.to_path_buf(),
            worktree_dir: dir.into(),
            edit: ReviewEditOp::Add {
                path: path.into(),
                side: Side::New,
                start,
                end,
                quote: quote.iter().map(|line| (*line).to_owned()).collect(),
                text: text.into(),
                pr: None,
            },
        },
    )
    .await;
    result.expect("a valid comment is accepted");
    seen.last().expect("the edit is pushed").1.clone()
}

async fn send(
    client: &mut Client,
    req: u64,
    project: &Path,
    dir: &str,
) -> (Result<OperationResult, (ErrorKind, String)>, Vec<Pushed>) {
    request(client, req, send_msg(req, project, dir)).await
}

fn send_msg(req: u64, project: &Path, dir: &str) -> ClientMsg {
    ClientMsg::ReviewSend {
        req,
        project: project.to_path_buf(),
        worktree_dir: dir.into(),
        outdated: Vec::new(),
        pr: None,
    }
}

/// The session a send started: `ReviewSent { started: true }` naming the session that now runs.
fn started_any_session(result: &Result<OperationResult, (ErrorKind, String)>) -> SessionId {
    match result {
        Ok(OperationResult::ReviewSent {
            session,
            started: true,
        }) => *session,
        other => panic!("a session was started for the send: {other:?}"),
    }
}

/// [`started_any_session`], asserting it is a new session, not one of the sandbox's records
/// (which the attach prunes: they hold no conversation).
fn started_session(result: &Result<OperationResult, (ErrorKind, String)>) -> SessionId {
    let session = started_any_session(result);
    assert!(
        !(1..=3).map(sid).any(|known| known == session),
        "a new session, not an existing record: {}",
        session.0
    );
    session
}

/// The last push of a send holds exactly the comments `added` (same ids, same order), each sent
/// when `sent`, else pending: an empty push passes no `all(...)` check here.
fn assert_pushed_exactly(
    pushed: &[ReviewComment],
    added: &[ReviewComment],
    sent: bool,
    rule: &str,
) {
    let ids = |comments: &[ReviewComment]| comments.iter().map(|c| c.id).collect::<Vec<_>>();
    assert_eq!(
        ids(pushed),
        ids(added),
        "the push holds exactly the entry's comments ({rule}): {pushed:?}"
    );
    for comment in pushed {
        assert_eq!(
            matches!(comment.state, CommentState::Sent { .. }),
            sent,
            "each comment is {} ({rule}): {comment:?}",
            if sent { "sent" } else { "pending" }
        );
    }
}

fn pending(comments: &[ReviewComment]) -> Vec<ReviewComment> {
    comments
        .iter()
        .filter(|c| c.state == CommentState::Draft)
        .cloned()
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn w6_w8_one_prompt_reaches_the_running_session_and_the_comments_become_sent() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let project = s.project();
    s.start(sid(1), false).await;
    let mut client = window(&s.state, &project).await;

    add(
        &mut client,
        1,
        &project,
        "wt",
        "src/a.rs",
        3,
        &["fn a() {}"],
        "Rename a.",
    )
    .await;
    let comments = add(
        &mut client,
        2,
        &project,
        "wt",
        "docs/b.md",
        1,
        &["# B", "", "text"],
        "Say more.",
    )
    .await;
    let expected = prompt::build(EntryKind::Worktree, &pending(&comments), &[]);

    let (result, seen) = send(&mut client, 3, &project, "wt").await;
    assert_eq!(
        result,
        Ok(OperationResult::ReviewSent {
            session: sid(1),
            started: false
        }),
        "the running session received it (W8)"
    );
    let typed = s.typed_holds(sid(1), &submitted(&expected)).await;
    assert_eq!(
        typed,
        submitted(&expected),
        "exactly one prompt, the one prompt::build gives, typed as one submission (US2 s3)"
    );
    let sending: Vec<bool> = seen.iter().map(|(_, _, sending)| *sending).collect();
    assert_eq!(
        sending,
        vec![true, false],
        "sending shows while the send is open (W6, W8)"
    );
    let last = &seen.last().unwrap().1;
    assert_pushed_exactly(
        last,
        &comments,
        true,
        "every comment of the send is sent, W8",
    );

    // Feature 946 (T012): Sent comments beside two new Drafts.
    add(
        &mut client,
        4,
        &project,
        "wt",
        "src/a.rs",
        9,
        &["x"],
        "Only this one.",
    )
    .await;
    let after = add(
        &mut client,
        5,
        &project,
        "wt",
        "src/c.rs",
        2,
        &["y"],
        "And this one.",
    )
    .await;
    let (result, seen) = send(&mut client, 6, &project, "wt").await;
    assert!(result.is_ok(), "{result:?}");
    let drafts = pending(&after);
    assert_eq!(drafts.len(), 2, "two Drafts beside the two Sent comments");
    let second = prompt::build(EntryKind::Worktree, &drafts, &[]);
    let typed = s.typed_holds(sid(1), &submitted(&second)).await;
    assert_eq!(
        typed,
        format!("{}{}", submitted(&expected), submitted(&second)),
        "the second send is one submission of the Drafts' batch alone (US2 s1, FR-009)"
    );
    assert_eq!(
        second.matches("### Comment ").count(),
        2,
        "one block per Draft"
    );
    for draft in &drafts {
        assert!(
            second.contains(&format!("### Comment {}: ", draft.reference())),
            "each Draft's block is headed by its reference: {second}"
        );
    }
    for sent in &comments {
        assert!(
            !second.contains(&sent.reference().to_string()),
            "nothing about a Sent comment: {second}"
        );
    }
    assert!(
        !second.contains("Rename a."),
        "no sent comment is sent again"
    );
    let last = &seen.last().unwrap().1;
    assert!(
        last.iter()
            .all(|c| matches!(c.state, CommentState::Sent { .. })),
        "the two Drafts became Sent (FR-015): {last:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn w6_a_send_with_nothing_pending_is_invalid_input() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let project = s.project();
    s.start(sid(1), false).await;
    let mut client = window(&s.state, &project).await;
    let (result, seen) = send(&mut client, 1, &project, "wt").await;
    assert_eq!(
        result.map_err(|(kind, _)| kind),
        Err(ErrorKind::InvalidInput),
        "no pending comment: nothing to send (US2 s5)"
    );
    assert!(seen.is_empty(), "nothing opened, nothing pushed: {seen:?}");
    tokio::time::sleep(QUIET).await;
    assert_eq!(s.typed(sid(1)), "", "nothing typed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn w9_a_terminal_without_bracketed_paste_gets_nothing_and_the_comments_stay_pending() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let project = s.project();
    s.start(sid(1), true).await;
    let mut client = window(&s.state, &project).await;
    add(
        &mut client,
        1,
        &project,
        "wt",
        "a",
        1,
        &["x"],
        "Line one\nline two",
    )
    .await;
    let added = add(&mut client, 2, &project, "wt", "b", 1, &["y"], "Second.").await;

    let (result, seen) = send(&mut client, 3, &project, "wt").await;
    let (kind, message) = result.expect_err("the send is refused");
    assert_eq!(kind, ErrorKind::Refused, "{message}");
    assert!(message.contains("paste"), "the message says why: {message}");
    let (_, comments, sending) = seen.last().expect("the end of the send is pushed").clone();
    assert!(!sending, "the send is closed");
    assert_pushed_exactly(
        &comments,
        &added,
        false,
        "both stay Draft, FR-015 (946 US2 s6)",
    );
    tokio::time::sleep(QUIET).await;
    assert_eq!(s.typed(sid(1)), "", "nothing was typed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fr021_a_prompt_never_reaches_another_entrys_running_session() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let project = s.project();
    s.start(sid(3), false).await;
    let mut client = window(&s.state, &project).await;
    add(
        &mut client,
        1,
        &project,
        "wt",
        "a",
        1,
        &["x"],
        "For wt only.",
    )
    .await;

    let (result, _) = send(&mut client, 2, &project, "wt").await;
    assert!(
        !matches!(
            result,
            Ok(OperationResult::ReviewSent { session, .. }) if session == sid(3)
        ),
        "`other`'s session is never the target: {result:?}"
    );
    // With no session of `wt` running, the send starts one in `wt` and types the prompt there.
    let new = started_session(&result);
    let (cwd, _) = s
        .state
        .session_cwd_and_cli(new)
        .expect("the session is kept");
    assert_eq!(
        cwd.file_name().and_then(|name| name.to_str()),
        Some("wt"),
        "the started session runs in `wt`, the entry sent from (FR-021): {}",
        cwd.display()
    );
    s.typed_holds(new, "For wt only.").await;
    tokio::time::sleep(QUIET).await;
    assert!(
        !s.typed(sid(3)).contains("For wt only."),
        "nothing reached `other`'s session"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn r5_the_most_recently_active_running_session_of_the_entry_receives_it() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let project = s.project();
    s.start(sid(1), false).await;
    s.start(sid(2), false).await;
    let mut client = window(&s.state, &project).await;

    add(&mut client, 1, &project, "wt", "a", 1, &["x"], "First.").await;
    let (result, _) = send(&mut client, 2, &project, "wt").await;
    assert_eq!(
        result,
        Ok(OperationResult::ReviewSent {
            session: sid(2),
            started: false
        }),
        "S2 started last"
    );
    s.typed_holds(sid(2), "First.").await;

    client
        .send(Frame::Control(ClientMsg::SessionInput {
            process: None,
            session: sid(1),
            serial: 0,
            // A line end: the stand-in `cat` reads a canonical-mode terminal line by line.
            bytes: b"x\r".to_vec(),
        }))
        .await
        .unwrap();
    s.typed_holds(sid(1), "x").await;
    add(&mut client, 3, &project, "wt", "a", 2, &["y"], "Second.").await;
    let (result, _) = send(&mut client, 4, &project, "wt").await;
    assert_eq!(
        result,
        Ok(OperationResult::ReviewSent {
            session: sid(1),
            started: false
        }),
        "the user typed into S1 last"
    );
    s.typed_holds(sid(1), "Second.").await;

    assert!(
        s.state
            .note_activity(sid(2), ActivityEvent::Hook(HookKind::UserPromptSubmit)),
        "S2's activity changes"
    );
    add(&mut client, 5, &project, "wt", "a", 3, &["z"], "Third.").await;
    let (result, _) = send(&mut client, 6, &project, "wt").await;
    assert_eq!(
        result,
        Ok(OperationResult::ReviewSent {
            session: sid(2),
            started: false
        }),
        "S2's activity changed last"
    );
    s.typed_holds(sid(2), "Third.").await;
    assert!(
        !s.typed(sid(1)).contains("Third."),
        "only one session receives a prompt"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn us3_s1_with_no_session_running_one_starts_and_its_first_input_is_the_prompt() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let project = s.project();
    let mut client = window(&s.state, &project).await;
    let comments = add(
        &mut client,
        1,
        &project,
        "wt",
        "src/a.rs",
        3,
        &["fn a() {}"],
        "Rename a.",
    )
    .await;
    let expected = prompt::build(EntryKind::Worktree, &pending(&comments), &[]);

    let (result, seen) = send(&mut client, 2, &project, "wt").await;
    let new = started_session(&result);
    let sessions = s.state.sessions_for(&project);
    assert_eq!(
        sessions
            .iter()
            .map(|session| session.id)
            .collect::<Vec<_>>(),
        [new],
        "the entry's own session is started and none was added (FR-001)"
    );
    let (cwd, cli) = s
        .state
        .session_cwd_and_cli(new)
        .expect("the session is kept");
    assert_eq!(cli, s.state.default_ai_cli(), "the default AI CLI (FR-016)");
    assert_eq!(
        cwd.file_name().and_then(|name| name.to_str()),
        Some("wt"),
        "it runs in the entry's worktree: {}",
        cwd.display()
    );
    assert_eq!(
        s.typed_holds(new, &submitted(&expected)).await,
        submitted(&expected),
        "its first input is the prompt, typed once"
    );
    let (_, last, sending) = seen.last().expect("the end of the send is pushed").clone();
    assert!(!sending, "the send is closed");
    assert_pushed_exactly(&last, &comments, true, "the comments are sent, US3 s1");
    assert!(
        s.state.live_session(new).is_some_and(|pty| pty.is_alive()),
        "the session the send started keeps running"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn us3_s1_the_default_entry_starts_its_session_in_the_project_root() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let project = s.project();
    let mut client = window(&s.state, &project).await;
    let comments = add(&mut client, 1, &project, "", "a", 1, &["x"], "At the root.").await;
    let expected = prompt::build(EntryKind::Default, &pending(&comments), &[]);

    let (result, _) = send(&mut client, 2, &project, "").await;
    let new = started_session(&result);
    let (cwd, _) = s.state.session_cwd_and_cli(new).unwrap();
    assert_eq!(
        cwd, project,
        "the Default entry's session runs in the project root"
    );
    assert_eq!(
        s.typed_holds(new, &submitted(&expected)).await,
        submitted(&expected)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn r10_an_ended_session_is_started_again_and_no_second_one_is_created() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let project = s.project();
    s.start(sid(1), false).await;
    // Attached while S1 is live, so the attach's pruning of conversation-less sessions spares it.
    let mut client = window(&s.state, &project).await;
    assert!(
        micold_daemon::ops::stop_session(&s.state, sid(1))
            .await
            .unwrap(),
        "S1 ends"
    );
    add(
        &mut client,
        1,
        &project,
        "wt",
        "a",
        1,
        &["x"],
        "Again for S1.",
    )
    .await;

    let (result, _) = send(&mut client, 2, &project, "wt").await;
    let new = started_any_session(&result);
    assert_eq!(new, sid(1), "the ended session is started again (R10)");
    s.typed_holds(new, "Again for S1.").await;
    assert_eq!(
        s.state.sessions_for(&project).len(),
        1,
        "no second session was created"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn us3_s2_an_ai_cli_that_cannot_start_leaves_the_comments_pending() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let project = s.project();
    let mut client = window(&s.state, &project).await;
    let added = add(
        &mut client,
        1,
        &project,
        "wt",
        "a",
        1,
        &["x"],
        "Never typed.",
    )
    .await;
    std::fs::remove_file(s.bin.path().join("claude")).unwrap();

    let (result, seen) = send(&mut client, 2, &project, "wt").await;
    let (kind, message) = result.expect_err("the send fails");
    assert_eq!(kind, ErrorKind::Refused, "{message}");
    assert!(!message.is_empty(), "the error says why");
    let (_, comments, sending) = seen.last().expect("the end of the send is pushed").clone();
    assert!(!sending, "`sending` is cleared (W9)");
    assert_pushed_exactly(
        &comments,
        &added,
        false,
        "every comment stays pending, US3 s2, FR-017",
    );
    assert!(s.inputs().is_empty(), "nothing was typed anywhere");
}

/// The trust-question branch (`FirstPromptUndelivered::AsksTrust`): the CLI would first ask whether
/// to trust the folder, and typing the prompt would answer that question (review B M6 F3).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn us3_s2_a_cli_that_would_ask_to_trust_the_folder_gets_nothing_typed() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let project = s.project();
    std::fs::remove_file(s._home.path().join(".claude.json")).unwrap();
    let mut client = window(&s.state, &project).await;
    let added = add(&mut client, 1, &project, "wt", "a", 1, &["x"], "Not typed.").await;

    let (result, seen) = send(&mut client, 2, &project, "wt").await;
    let (kind, message) = result.expect_err("the send fails");
    assert_eq!(kind, ErrorKind::Refused, "{message}");
    assert!(
        message.contains("trust"),
        "the error names the trust question: {message}"
    );
    let (_, comments, sending) = seen.last().expect("the end of the send is pushed").clone();
    assert!(!sending, "`sending` is cleared (W9)");
    assert_pushed_exactly(
        &comments,
        &added,
        false,
        "every comment stays pending, FR-017",
    );
    // The session starts (its stand-in creates an empty input file); the prompt is never typed.
    assert!(
        s.inputs().iter().all(|(_, typed)| typed.is_empty()),
        "nothing was typed anywhere"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn w9_a_session_not_ready_within_the_bound_gets_nothing_and_the_comments_stay_pending() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let project = s.project();
    s.state.set_first_prompt_bound(Duration::from_secs(1));
    s.slow(3.0);
    let mut client = window(&s.state, &project).await;
    let added = add(&mut client, 1, &project, "wt", "a", 1, &["x"], "Too early.").await;

    let (result, seen) = send(&mut client, 2, &project, "wt").await;
    let (kind, message) = result.expect_err("the send fails");
    assert_eq!(kind, ErrorKind::Refused, "{message}");
    assert!(message.contains("ready"), "the message says why: {message}");
    let (_, comments, sending) = seen.last().expect("the end of the send is pushed").clone();
    assert!(!sending, "`sending` is cleared (W9)");
    assert_pushed_exactly(
        &comments,
        &added,
        false,
        "every comment stays pending, FR-017",
    );
    tokio::time::sleep(Duration::from_secs(3) + QUIET).await;
    assert!(
        s.inputs()
            .iter()
            .all(|(_, typed)| !typed.contains("Too early.")),
        "a session ready after the bound is typed nothing: {:?}",
        s.inputs()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fr018_a_second_window_sending_meanwhile_is_busy_and_the_prompt_is_typed_once() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let project = s.project();
    s.slow(1.0);
    let mut first = window(&s.state, &project).await;
    let mut second = window(&s.state, &project).await;
    let comments = add(&mut first, 1, &project, "wt", "a", 1, &["x"], "Once only.").await;
    let expected = prompt::build(EntryKind::Worktree, &pending(&comments), &[]);

    first
        .send(Frame::Control(send_msg(2, &project, "wt")))
        .await
        .unwrap();
    let opened = async {
        loop {
            if let Frame::Control(DaemonMsg::ReviewChanged { sending: true, .. }) =
                second.next().await.expect("stream open").unwrap()
            {
                return;
            }
        }
    };
    tokio::time::timeout(BOUND, opened)
        .await
        .expect("the second window sees the send open");
    let (busy, _) = send(&mut second, 1, &project, "wt").await;
    assert_eq!(
        busy.map_err(|(kind, _)| kind),
        Err(ErrorKind::Busy),
        "a send in progress makes the entry's send unavailable (FR-018, US3 s3)"
    );

    let (result, _) = answer(&mut first, 2).await;
    let new = started_session(&result);
    assert_eq!(
        s.typed_holds(new, &submitted(&expected)).await,
        submitted(&expected),
        "the prompt is typed once"
    );
    tokio::time::sleep(QUIET).await;
    let typed: Vec<_> = s
        .inputs()
        .into_iter()
        .filter(|(_, typed)| typed.contains("Once only."))
        .collect();
    assert_eq!(typed.len(), 1, "one session received it: {typed:?}");
}

/// Send `dir`'s Drafts with `outdated` as the client's verdicts.
async fn send_with(
    client: &mut Client,
    req: u64,
    project: &Path,
    dir: &str,
    outdated: Vec<micold_core::review::comment::CommentId>,
) -> (Result<OperationResult, (ErrorKind, String)>, Vec<Pushed>) {
    let msg = ClientMsg::ReviewSend {
        req,
        project: project.to_path_buf(),
        worktree_dir: dir.into(),
        outdated,
        pr: None,
    };
    request(client, req, msg).await
}

/// Feature 946 W16, US4 s1, FR-023: a Draft whose quoted lines moved on disk is sent with its new
/// lines and stored at them, though no window reopened the file.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn w16_a_draft_whose_code_moved_is_sent_and_stored_at_its_new_lines() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let project = s.project();
    s.start(sid(1), false).await;
    let mut client = window(&s.state, &project).await;
    let quote = ["fn a() {}", "fn b() {}"];
    let comments = add(
        &mut client,
        1,
        &project,
        "wt",
        "src/a.rs",
        3,
        &quote,
        "Rename.",
    )
    .await;
    let file = entry_dir(&project, "wt").join("src/a.rs");
    let before = std::fs::read_to_string(&file).unwrap();
    std::fs::write(&file, format!("{}{before}", "new\n".repeat(5))).unwrap();

    let (result, seen) = send(&mut client, 2, &project, "wt").await;
    assert!(result.is_ok(), "{result:?}");
    let mut moved = pending(&comments);
    moved[0].range = micold_core::review::LineRange::new(8, 9).unwrap();
    let expected = prompt::build(EntryKind::Worktree, &moved, &[]);
    assert!(
        expected.contains("lines 8"),
        "the heading names the new lines: {expected}"
    );
    let typed = s.typed_holds(sid(1), &submitted(&expected)).await;
    assert_eq!(typed, submitted(&expected), "the batch names the new lines");
    let last = &seen.last().unwrap().1;
    assert_eq!(last[0].range, moved[0].range, "stored at the new range");
    assert!(matches!(last[0].state, CommentState::Sent { .. }));
}

/// Feature 946 W16, FR-021/FR-022: code that is gone gets the outdated note whatever the client
/// said; a removed-side Draft keeps the client's verdict, and the client's word about a
/// current-side comment is not used.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn w16_the_daemon_judges_current_side_drafts_and_the_client_removed_side_ones() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let project = s.project();
    s.start(sid(1), false).await;
    let mut client = window(&s.state, &project).await;

    let gone = add(
        &mut client,
        1,
        &project,
        "wt",
        "src/a.rs",
        3,
        &["fn a() {}"],
        "Gone.",
    )
    .await;
    let kept = add(
        &mut client,
        2,
        &project,
        "wt",
        "src/b.rs",
        1,
        &["keep"],
        "Kept.",
    )
    .await;
    let removed = {
        let (result, seen) = request(
            &mut client,
            3,
            ClientMsg::ReviewEdit {
                req: 3,
                project: project.clone(),
                worktree_dir: "wt".into(),
                edit: ReviewEditOp::Add {
                    path: "src/c.rs".into(),
                    side: Side::Old,
                    start: 4,
                    end: 4,
                    quote: vec!["old line".into()],
                    text: "Removed side.".into(),
                    pr: None,
                },
            },
        )
        .await;
        result.expect("added");
        seen.last().unwrap().1.clone()
    };
    std::fs::write(
        entry_dir(&project, "wt").join("src/a.rs"),
        "something else\n",
    )
    .unwrap();
    let id_of = |all: &[ReviewComment], text: &str| all.iter().find(|c| c.text == text).unwrap().id;
    let (gone_id, kept_id, removed_id) = (
        id_of(&gone, "Gone."),
        id_of(&kept, "Kept."),
        id_of(&removed, "Removed side."),
    );

    // The client lists the removed-side comment and, wrongly, the intact current-side one.
    let (result, _) = send_with(&mut client, 4, &project, "wt", vec![removed_id, kept_id]).await;
    assert!(result.is_ok(), "{result:?}");
    let expected = prompt::build(
        EntryKind::Worktree,
        &pending(&removed),
        &[gone_id, removed_id],
    );
    let typed = s.typed_holds(sid(1), &submitted(&expected)).await;
    assert_eq!(
        typed,
        submitted(&expected),
        "outdated: the gone current-side Draft and the client's removed-side one only"
    );
}

// --- Feature 1149: sending a pull request's review (W20) ---

fn git_in(dir: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

fn pr_send(
    number: u64,
    head_branch: &str,
    current_head: &str,
) -> micold_core::protocol::messages::PrSend {
    micold_core::protocol::messages::PrSend {
        number,
        title: "Add retry".into(),
        head_branch: head_branch.into(),
        state: micold_core::pull_request::BranchPrState::Open,
        current_head: current_head.into(),
    }
}

/// Add a comment on pull request `number` (head branch `branch`, written at `commit`); a pull
/// request's comments are not judged against the working tree, so no file is placed.
#[allow(clippy::too_many_arguments)] // a test helper: one argument per field of the comment
async fn add_pr(
    client: &mut Client,
    req: u64,
    project: &Path,
    dir: &str,
    number: u64,
    branch: &str,
    commit: &str,
    path: &str,
    text: &str,
) -> Vec<ReviewComment> {
    let (result, seen) = request(
        client,
        req,
        ClientMsg::ReviewEdit {
            req,
            project: project.to_path_buf(),
            worktree_dir: dir.into(),
            edit: ReviewEditOp::Add {
                path: path.into(),
                side: Side::New,
                start: 1,
                end: 1,
                quote: vec!["quoted".into()],
                text: text.into(),
                pr: Some(micold_core::review::comment::PrAnchor {
                    number,
                    head_branch: branch.into(),
                    head_commit: commit.into(),
                }),
            },
        },
    )
    .await;
    result.expect("a pull request comment is accepted");
    seen.last().expect("the edit is pushed").1.clone()
}

async fn send_pr(
    client: &mut Client,
    req: u64,
    project: &Path,
    dir: &str,
    pr: Option<micold_core::protocol::messages::PrSend>,
) -> (Result<OperationResult, (ErrorKind, String)>, Vec<Pushed>) {
    let msg = ClientMsg::ReviewSend {
        req,
        project: project.to_path_buf(),
        worktree_dir: dir.into(),
        outdated: Vec::new(),
        pr,
    };
    request(client, req, msg).await
}

fn context(
    pr: &micold_core::protocol::messages::PrSend,
    checked_out: Option<&str>,
    unreadable: Vec<String>,
) -> prompt::PrContext {
    prompt::PrContext {
        number: pr.number,
        title: pr.title.clone(),
        head_branch: pr.head_branch.clone(),
        state: pr.state,
        current_head: pr.current_head.clone(),
        checked_out: checked_out.map(str::to_owned),
        unreadable,
    }
}

/// The W20 mix: ten comments on three files of pull request 12, one of pull request 13 and two of
/// the working tree, with a running session.
struct W20Mix {
    s: Sandbox,
    project: PathBuf,
    branch: String,
    head: String,
    client: Client,
}

async fn w20_mix() -> W20Mix {
    let s = Sandbox::new().await;
    let project = s.project();
    let wt = entry_dir(&project, "wt");
    let branch = git_in(&wt, &["symbolic-ref", "--short", "HEAD"]);
    let head = git_in(&wt, &["rev-parse", "HEAD"]);
    s.start(sid(1), false).await;
    let mut client = window(&s.state, &project).await;
    for n in 0..10u64 {
        let path = ["src/a.rs", "src/b.rs", "docs/c.md"][(n % 3) as usize];
        let text = format!("pr12 note {n}");
        add_pr(
            &mut client,
            n + 1,
            &project,
            "wt",
            12,
            &branch,
            &head,
            path,
            &text,
        )
        .await;
    }
    add_pr(
        &mut client,
        20,
        &project,
        "wt",
        13,
        "other/branch",
        &head,
        "src/z.rs",
        "pr13 note",
    )
    .await;
    add(
        &mut client,
        21,
        &project,
        "wt",
        "src/w.rs",
        1,
        &["tree"],
        "tree note",
    )
    .await;
    add(
        &mut client,
        22,
        &project,
        "wt",
        "src/v.rs",
        1,
        &["tree"],
        "tree note 2",
    )
    .await;
    W20Mix {
        s,
        project,
        branch,
        head,
        client,
    }
}

/// W20, SC-002, FR-006: only the first pull request's ten drafts are delivered, in one batch typed
/// once, and become sent; another pull request's and the working tree's drafts stay Draft.
///
/// The batch is checked by its key literal lines, not by calling `build_pr`; the exact bytes are
/// pinned by `p1_pr_example_gives_the_exact_bytes` in `micold-core`'s `review/prompt.rs`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn w20_a_pull_request_send_delivers_only_its_ten_drafts_and_they_become_sent() {
    let _guard = ENV.lock().await;
    let mut m = w20_mix().await;
    let pr = Some(pr_send(12, &m.branch, &m.head));
    let (result, seen) = send_pr(&mut m.client, 24, &m.project, "wt", pr).await;
    assert_eq!(
        result,
        Ok(OperationResult::ReviewSent {
            session: sid(1),
            started: false
        })
    );
    let (_, last, sending) = seen.last().expect("the end of the send is pushed").clone();
    assert!(!sending);
    let sent: Vec<&ReviewComment> = last
        .iter()
        .filter(|c| matches!(c.state, CommentState::Sent { .. }))
        .collect();
    assert_eq!(sent.len(), 10, "exactly the pull request's ten");
    assert!(sent
        .iter()
        .all(|c| c.pr.as_ref().is_some_and(|pr| pr.number == 12)));
    assert_eq!(
        pending(&last).len(),
        3,
        "pr 13's and the working tree's two stay Draft"
    );

    let preamble = format!(
        "Please work through these review comments on pull request #12 \"Add retry\" (head branch {}, head commit {}).",
        m.branch, m.head
    );
    // The batch arrives in chunks: wait for every note, not just the preamble.
    m.s.typed_holds(sid(1), &preamble).await;
    let mut typed = String::new();
    for n in 0..10 {
        typed = m.s.typed_holds(sid(1), &format!("pr12 note {n}")).await;
    }
    assert_eq!(typed.matches(&preamble).count(), 1, "one batch, typed once");
    assert_eq!(typed.matches("### Comment C-").count(), 10);
    for n in 0..10 {
        assert!(typed.contains(&format!("pr12 note {n}")), "note {n}");
    }
    assert!(!typed.contains("pr13 note") && !typed.contains("tree note"));
    assert!(!typed.contains("checked out") && !typed.contains("written against"));
}

/// W20: a pull request with no draft is refused and opens and types nothing.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn w20_a_send_for_a_pull_request_with_no_draft_types_nothing() {
    let _guard = ENV.lock().await;
    let mut m = w20_mix().await;
    let pr = Some(pr_send(99, &m.branch, &m.head));
    let (result, seen) = send_pr(&mut m.client, 23, &m.project, "wt", pr).await;
    assert_eq!(
        result.expect_err("nothing to send").0,
        ErrorKind::InvalidInput
    );
    assert!(
        seen.is_empty(),
        "a pull request with no draft opens nothing"
    );
    assert_eq!(m.s.typed(sid(1)), "");
}

/// W20, FR-006: `pr: None` sends the working tree's two drafts alone.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn w20_a_working_tree_send_holds_the_working_trees_drafts_only() {
    let _guard = ENV.lock().await;
    let mut m = w20_mix().await;
    let (result, seen) = send_pr(&mut m.client, 25, &m.project, "wt", None).await;
    result.expect("sent");
    let (_, last, _) = seen.last().expect("pushed").clone();
    assert_eq!(
        pending(&last).len(),
        11,
        "pr 12's ten and pr 13's stay Draft"
    );
    let tree: Vec<ReviewComment> = last.iter().filter(|c| c.pr.is_none()).cloned().collect();
    let expected = prompt::build(EntryKind::Worktree, &tree, &[]);
    assert!(expected.contains("tree note 2") && !expected.contains("pr12 note"));
    let typed = m.s.typed_holds(sid(1), &submitted(&expected)).await;
    assert!(!typed.contains("pr12 note") && !typed.contains("pr13 note"));
}

/// W20 refusals: W18's rules for the pull request, the Default entry, and no draft.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn w20_a_bad_pull_request_or_the_default_entry_or_no_draft_is_invalid_input() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let project = s.project();
    let head = "a".repeat(40);
    let mut client = window(&s.state, &project).await;
    add_pr(
        &mut client,
        1,
        &project,
        "wt",
        5,
        "feat/x",
        &head,
        "a.rs",
        "note",
    )
    .await;
    let bad = [
        ("wt", pr_send(0, "feat/x", &head)),
        ("wt", pr_send(5, "", &head)),
        ("wt", pr_send(5, "feat/\nx", &head)),
        ("wt", pr_send(5, "feat/x", "not-a-commit")),
        ("wt", pr_send(5, "feat/x", &"a".repeat(39))),
        ("", pr_send(5, "feat/x", &head)),
        ("wt", pr_send(6, "feat/x", &head)),
        ("other", pr_send(5, "feat/x", &head)),
    ];
    for (n, (dir, pr)) in bad.into_iter().enumerate() {
        let (result, seen) =
            send_pr(&mut client, 10 + n as u64, &project, dir, Some(pr.clone())).await;
        assert_eq!(
            result.map_err(|(kind, _)| kind),
            Err(ErrorKind::InvalidInput),
            "{dir:?} {pr:?}"
        );
        assert!(seen.is_empty(), "nothing opened: {seen:?}");
    }
}

/// W20, FR-010, W9: a delivery failure leaves the pull request's comments Draft.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn w20_a_delivery_failure_leaves_the_pull_requests_comments_draft() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let project = s.project();
    let head = git_in(&entry_dir(&project, "wt"), &["rev-parse", "HEAD"]);
    let mut client = window(&s.state, &project).await;
    let added = add_pr(
        &mut client,
        1,
        &project,
        "wt",
        5,
        "feat/x",
        &head,
        "a.rs",
        "Never typed.",
    )
    .await;
    std::fs::remove_file(s.bin.path().join("claude")).unwrap();

    let (result, seen) = send_pr(
        &mut client,
        2,
        &project,
        "wt",
        Some(pr_send(5, "feat/x", &head)),
    )
    .await;
    assert_eq!(result.expect_err("the send fails").0, ErrorKind::Refused);
    let (_, comments, sending) = seen.last().expect("pushed").clone();
    assert!(!sending);
    assert_pushed_exactly(&comments, &added, false, "FR-010: they stay Draft");
    assert!(s.inputs().is_empty());
}

/// W20, FR-010: two sends of one entry, one is `Busy`; and a head move while the send is open
/// leaves each comment's recorded commit unchanged.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn w20_two_concurrent_sends_one_is_busy_and_recorded_commits_do_not_move() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let project = s.project();
    s.slow(1.0);
    let recorded = "1".repeat(40);
    let moved = "2".repeat(40);
    let mut first = window(&s.state, &project).await;
    let mut second = window(&s.state, &project).await;
    add_pr(
        &mut first,
        1,
        &project,
        "wt",
        5,
        "feat/x",
        &recorded,
        "a.rs",
        "Once only.",
    )
    .await;
    first
        .send(Frame::Control(ClientMsg::ReviewSend {
            req: 2,
            project: project.clone(),
            worktree_dir: "wt".into(),
            outdated: Vec::new(),
            pr: Some(pr_send(5, "feat/x", &moved)),
        }))
        .await
        .unwrap();
    let opened = async {
        loop {
            if let Frame::Control(DaemonMsg::ReviewChanged { sending: true, .. }) =
                second.next().await.expect("stream open").unwrap()
            {
                return;
            }
        }
    };
    tokio::time::timeout(BOUND, opened)
        .await
        .expect("the send opens");
    let (busy, _) = send_pr(
        &mut second,
        1,
        &project,
        "wt",
        Some(pr_send(5, "feat/x", &moved)),
    )
    .await;
    assert_eq!(busy.map_err(|(kind, _)| kind), Err(ErrorKind::Busy));

    let (result, seen) = answer(&mut first, 2).await;
    let new = started_session(&result);
    let (_, last, _) = seen.last().expect("pushed").clone();
    assert_eq!(
        last[0].pr.as_ref().unwrap().head_commit,
        recorded,
        "the recorded commit stays"
    );
    let typed = s.typed_holds(new, "Once only.").await;
    assert!(typed.contains(&format!(
        "written against commit {recorded}; the pull request's head is now {moved}"
    )));
}

/// US3 s2, FR-009, real git: another branch checked out gives the switch line, a commit the
/// worktree does not hold gives the "not available" line, and the batch is delivered anyway.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn us3_another_branch_and_a_missing_commit_add_notes_and_are_still_delivered() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let project = s.project();
    let wt = entry_dir(&project, "wt");
    let head = git_in(&wt, &["rev-parse", "HEAD"]);
    git_in(&wt, &["checkout", "-q", "-b", "elsewhere"]);
    let missing = "a".repeat(40);
    s.start(sid(1), false).await;
    let mut client = window(&s.state, &project).await;
    add_pr(
        &mut client,
        1,
        &project,
        "wt",
        5,
        "feat/x",
        &missing,
        "a.rs",
        "Gone.",
    )
    .await;

    let pr = pr_send(5, "feat/x", &head);
    let (result, seen) = send_pr(&mut client, 2, &project, "wt", Some(pr.clone())).await;
    assert_eq!(
        result,
        Ok(OperationResult::ReviewSent {
            session: sid(1),
            started: false
        }),
        "delivery is not refused"
    );
    let (_, last, _) = seen.last().expect("pushed").clone();
    let expected = prompt::build_pr(
        &context(&pr, Some("elsewhere"), vec![missing.clone()]),
        &last,
        &[],
    );
    assert!(expected.contains("Switch to feat/x before changing anything."));
    assert!(expected.contains(&format!(
        "Commit {missing} is not available in this worktree"
    )));
    assert_eq!(
        s.typed_holds(sid(1), &submitted(&expected)).await,
        submitted(&expected)
    );
}

/// US3 s3 (482 FR-016): with no session running, one starts and receives the pull request's batch
/// as its first input; a detached head counts as another branch.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn us3_no_running_session_starts_one_that_receives_the_pull_request_batch() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let project = s.project();
    let wt = entry_dir(&project, "wt");
    let head = git_in(&wt, &["rev-parse", "HEAD"]);
    git_in(&wt, &["checkout", "-q", "--detach"]);
    let mut client = window(&s.state, &project).await;
    add_pr(
        &mut client,
        1,
        &project,
        "wt",
        5,
        "feat/x",
        &head,
        "a.rs",
        "Start one.",
    )
    .await;

    let pr = pr_send(5, "feat/x", &head);
    let (result, seen) = send_pr(&mut client, 2, &project, "wt", Some(pr.clone())).await;
    let new = started_session(&result);
    let (_, last, _) = seen.last().expect("pushed").clone();
    let expected = prompt::build_pr(&context(&pr, None, vec![]), &last, &[]);
    assert_eq!(
        s.typed_holds(new, &submitted(&expected)).await,
        submitted(&expected),
        "its first input is the batch"
    );
}

/// Feature 1271 T021 (RM5, FR-007): a delivered send leaves the entry's reviewed marks as they
/// were, in memory and in every push after it.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rm5_a_delivered_send_leaves_the_reviewed_marks() {
    use micold_core::review::comment::Scope;
    use micold_core::review::marks::{ChangeId, ReviewMark};
    use micold_core::review::RelPath;

    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let project = s.project();
    s.start(sid(1), false).await;
    let mut client = window(&s.state, &project).await;

    let change = ChangeId::parse(&"cd".repeat(20)).unwrap();
    let (result, _) = request(
        &mut client,
        1,
        ClientMsg::ReviewEdit {
            req: 1,
            project: project.clone(),
            worktree_dir: "wt".into(),
            edit: ReviewEditOp::SetReviewed {
                scope: Scope::WorkingTree,
                path: "src/a.rs".into(),
                change: Some(change),
            },
        },
    )
    .await;
    result.expect("marked");
    add(
        &mut client,
        2,
        &project,
        "wt",
        "src/a.rs",
        3,
        &["fn a() {}"],
        "Rename a.",
    )
    .await;
    let (result, _) = send(&mut client, 3, &project, "wt").await;
    assert!(result.is_ok(), "{result:?}");

    let marks: Vec<(String, Vec<ReviewMark>)> = s
        .state
        .review_pushes_on_attach(&project)
        .into_iter()
        .filter_map(|msg| match msg {
            DaemonMsg::ReviewChanged {
                worktree_dir,
                marks,
                ..
            } => Some((worktree_dir, marks)),
            _ => None,
        })
        .collect();
    assert_eq!(
        marks,
        vec![(
            "wt".to_owned(),
            vec![ReviewMark {
                scope: Scope::WorkingTree,
                path: RelPath::from_native("src/a.rs").unwrap(),
                change,
            }]
        )],
        "the send left the mark (RM5)"
    );
}
