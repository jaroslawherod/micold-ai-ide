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

use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use mcp_support::{add_worktree, init_repo, session, sid, state_over};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{
    ClientMsg, DaemonMsg, ErrorKind, OperationResult, ReviewEditOp,
};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_core::review::comment::{CommentState, ReviewComment};
use micold_core::review::prompt::{self, EntryKind};
use micold_core::review::Side;
use micold_core::session::{AiCli, SessionId, TerminalMode};
use micold_core::terminal::LaunchMode;
use micold_daemon::activity::{ActivityEvent, HookKind};
use micold_daemon::state::DaemonState;
use tokio_util::codec::Framed;

/// The tests change process-wide variables (`PATH`, `HOME`); one at a time.
static ENV: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// How long a test waits for something it expects.
const BOUND: Duration = Duration::from_secs(30);
/// How long a test waits to be sure something does not come.
const QUIET: Duration = Duration::from_millis(500);

type Client = Framed<tokio::io::DuplexStream, ClientCodec>;

/// Every variable a test changes, restored on drop.
struct Env {
    saved: Vec<(&'static str, Option<OsString>)>,
}

impl Env {
    fn set(vars: &[(&'static str, Option<OsString>)]) -> Self {
        let saved = vars
            .iter()
            .map(|(name, value)| {
                let previous = std::env::var_os(name);
                match value {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
                (*name, previous)
            })
            .collect();
        Self { saved }
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        for (name, previous) in self.saved.drain(..) {
            match previous {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}

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
        let started = micold_daemon::ops::start_session(&self.state, id, LaunchMode::Fresh)
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

/// The stand-in `claude`: waits `<bin>/slow` seconds when that file exists, asks for bracketed
/// paste (unless told not to), draws a prompt, and appends everything typed into it to
/// `<bin>/input.<its --session-id>`.
fn install_claude(bin: &Path) {
    let dir = bin.display();
    let path = bin.join("claude");
    std::fs::write(
        &path,
        format!(
            "#!/bin/sh\n\
             sid=''; prev=''\n\
             for a in \"$@\"; do [ \"$prev\" = --session-id ] && sid=\"$a\"; prev=\"$a\"; done\n\
             [ -e '{dir}'/slow ] && sleep \"$(cat '{dir}'/slow)\"\n\
             [ -e '{dir}'/nopaste.\"$sid\" ] || printf '\\033[?2004h'\n\
             printf 'ready> '\n\
             exec cat >> '{dir}'/input.\"$sid\"\n"
        ),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// Connect, complete the handshake and attach to `project`.
async fn window(state: &Arc<DaemonState>, project: &Path) -> Client {
    let (server_io, client_io) = tokio::io::duplex(1024 * 1024);
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
    client
        .send(Frame::Control(ClientMsg::Attach {
            project: project.to_path_buf(),
            force: true,
        }))
        .await
        .unwrap();
    let attached = async {
        loop {
            if let Frame::Control(DaemonMsg::CatalogChanged { .. }) =
                client.next().await.expect("stream open").unwrap()
            {
                return;
            }
        }
    };
    tokio::time::timeout(BOUND, attached)
        .await
        .expect("attach completes with its catalog");
    client
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
    }
}

/// The session a send started: `ReviewSent { started: true }` naming a session that is not one of
/// the sandbox's records.
fn started_session(result: &Result<OperationResult, (ErrorKind, String)>) -> SessionId {
    match result {
        Ok(OperationResult::ReviewSent {
            session,
            started: true,
        }) => {
            assert!(
                !(1..=3).map(sid).any(|known| known == *session),
                "a new session, not an existing record: {}",
                session.0
            );
            *session
        }
        other => panic!("a session was started for the send: {other:?}"),
    }
}

/// The bytes a submission of `prompt` types into a terminal with bracketed paste on, as the
/// stand-in's cooked terminal writes them (its Enter arrives as a line feed).
fn submitted(prompt: &str) -> String {
    format!("\u{1b}[200~{prompt}\u{1b}[201~\n")
}

fn pending(comments: &[ReviewComment]) -> Vec<ReviewComment> {
    comments
        .iter()
        .filter(|c| c.state == CommentState::Pending)
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
    let typed = s.typed_holds(sid(1), "Say more.").await;
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
    assert!(
        last.iter()
            .all(|c| matches!(c.state, CommentState::Sent { .. })),
        "every comment of the send is sent: {last:?}"
    );

    let after = add(
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
    let (result, _) = send(&mut client, 5, &project, "wt").await;
    assert!(result.is_ok(), "{result:?}");
    let second = prompt::build(EntryKind::Worktree, &pending(&after), &[]);
    let typed = s.typed_holds(sid(1), "Only this one.").await;
    assert_eq!(
        typed,
        format!("{}{}", submitted(&expected), submitted(&second)),
        "the second send carries only the new pending comment (US2 s6)"
    );
    assert!(
        !second.contains("Rename a."),
        "no sent comment is sent again"
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

    let (result, seen) = send(&mut client, 2, &project, "wt").await;
    let (kind, message) = result.expect_err("the send is refused");
    assert_eq!(kind, ErrorKind::Refused, "{message}");
    assert!(message.contains("paste"), "the message says why: {message}");
    let (_, comments, sending) = seen.last().expect("the end of the send is pushed").clone();
    assert!(!sending, "the send is closed");
    assert!(
        comments.iter().all(|c| c.state == CommentState::Pending),
        "nothing is marked sent (FR-017)"
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
        s.typed_holds(new, "Rename a.").await,
        submitted(&expected),
        "its first input is the prompt, typed once"
    );
    let (_, last, sending) = seen.last().expect("the end of the send is pushed").clone();
    assert!(!sending, "the send is closed");
    assert!(
        last.iter()
            .all(|c| matches!(c.state, CommentState::Sent { .. })),
        "the comments are sent (US3 s1): {last:?}"
    );
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
        s.typed_holds(new, "At the root.").await,
        submitted(&expected)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn us3_s4_an_ended_session_is_not_resumed_and_a_new_one_starts() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let project = s.project();
    s.start(sid(1), false).await;
    assert!(
        micold_daemon::ops::stop_session(&s.state, sid(1))
            .await
            .unwrap(),
        "S1 ends"
    );
    let mut client = window(&s.state, &project).await;
    add(
        &mut client,
        1,
        &project,
        "wt",
        "a",
        1,
        &["x"],
        "Not for S1.",
    )
    .await;

    let (result, _) = send(&mut client, 2, &project, "wt").await;
    let new = started_session(&result);
    s.typed_holds(new, "Not for S1.").await;
    assert!(
        s.state
            .live_session(sid(1))
            .is_none_or(|pty| !pty.is_alive()),
        "the ended session is not resumed (US3 s4)"
    );
    assert!(
        !s.typed(sid(1)).contains("Not for S1."),
        "nothing reached S1"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn us3_s2_an_ai_cli_that_cannot_start_leaves_the_comments_pending() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let project = s.project();
    let mut client = window(&s.state, &project).await;
    add(
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
    assert!(
        comments.iter().all(|c| c.state == CommentState::Pending),
        "every comment stays pending (US3 s2, FR-017)"
    );
    assert!(s.inputs().is_empty(), "nothing was typed anywhere");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn w9_a_session_not_ready_within_the_bound_gets_nothing_and_the_comments_stay_pending() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let project = s.project();
    s.state.set_first_prompt_bound(Duration::from_secs(1));
    s.slow(3.0);
    let mut client = window(&s.state, &project).await;
    add(&mut client, 1, &project, "wt", "a", 1, &["x"], "Too early.").await;

    let (result, seen) = send(&mut client, 2, &project, "wt").await;
    let (kind, message) = result.expect_err("the send fails");
    assert_eq!(kind, ErrorKind::Refused, "{message}");
    assert!(message.contains("ready"), "the message says why: {message}");
    let (_, comments, sending) = seen.last().expect("the end of the send is pushed").clone();
    assert!(!sending, "`sending` is cleared (W9)");
    assert!(
        comments.iter().all(|c| c.state == CommentState::Pending),
        "every comment stays pending (FR-017)"
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
        s.typed_holds(new, "Once only.").await,
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
