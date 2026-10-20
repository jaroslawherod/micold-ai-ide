//! The `report_review_comment` agent tool on the service (feature 946, contracts/agent-tool.md,
//! contracts/review-wire-delta.md W17; US1 scenarios 1, 3, 5, 7; FR-003, FR-007, FR-008, D5).
//!
//! A real git repository with worktrees `wt` and `other`, a catalog in a store directory (so "kept"
//! is checked by starting a second service over it), sessions S1 in `wt`, S3 in `other` and S4 in
//! the project root, and a stand-in `claude` that a send starts. Comments are written and sent
//! through the service's own review calls; the tool is called over the agent tool server as S1, S3
//! or S4; windows are registered clients, which receive every push.

// unix-only: the stand-in CLI is a `#!/bin/sh` script, as in `review_send.rs`.
#![cfg(unix)]

#[path = "support/mcp.rs"]
mod mcp_support;

#[path = "support/runs.rs"]
mod runs_support;

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use mcp_support::{
    add_worktree, call_err, call_ok, credential, fake_window, init_repo, log, log_lines_for,
    serve_tool_server, session, sid,
};
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{DaemonMsg, OperationResult, ReviewEditOp};
use micold_core::review::comment::{CommentState, Outcome, Reporter, ReviewComment, Scope};
use micold_core::review::Side;
use micold_core::session::{AiCli, SessionId, TerminalMode};
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::state::DaemonState;
use runs_support::{install_claude, service, Env, BOUND, ENV, QUIET};
use serde_json::{json, Value};
use tokio::sync::mpsc::UnboundedReceiver;

/// A window: every frame the service broadcasts.
type Window = UnboundedReceiver<Frame<DaemonMsg>>;

/// The reply every successful report carries. Distinctive, so a log line holding it is easy to find.
const REPLY: &str = "Renamed as asked, secret-reply-946";

/// S1 in `wt`, S3 in `other`, S4 in the project root, all `claude`, never started.
struct Sandbox {
    _env: Env,
    bin: tempfile::TempDir,
    _home: tempfile::TempDir,
    project: tempfile::TempDir,
    store: tempfile::TempDir,
    _bindings: tempfile::TempDir,
    state: Arc<DaemonState>,
    addr: SocketAddr,
}

impl Sandbox {
    async fn new() -> Self {
        log();
        let bin = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        let store = tempfile::tempdir().unwrap();
        let bindings = tempfile::tempdir().unwrap();
        install_claude(bin.path());
        init_repo(project.path());
        add_worktree(project.path(), "wt");
        add_worktree(project.path(), "other");
        let trust = json!({"projects": {project.path().to_str().unwrap(): {"hasTrustDialogAccepted": true}}});
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
        let claude = |id, dir| session(sid(id), dir, TerminalMode::AiCli, AiCli::ClaudeCode);
        let workspace = Workspace {
            projects: vec![Project::new(
                project.path().to_path_buf(),
                true,
                Availability::Available,
            )],
            active: Some(project.path().to_path_buf()),
            sessions: [(
                project.path().to_path_buf(),
                vec![
                    claude(1, Some("wt")),
                    claude(3, Some("other")),
                    claude(4, None),
                ],
            )]
            .into(),
            ..Default::default()
        };
        JsonFileStore::at(store.path().join("projects.json"))
            .save(&workspace)
            .unwrap();
        let state = service(store.path());
        let addr = serve_tool_server(&state, bindings.path().to_path_buf()).await;
        Self {
            _env: env,
            bin,
            _home: home,
            project,
            store,
            _bindings: bindings,
            state,
            addr,
        }
    }

    fn project(&self) -> PathBuf {
        self.project.path().to_path_buf()
    }

    /// Add a comment on lines `start..start + lines` of `path` in entry `dir`; returns it.
    fn add(&self, dir: &str, path: &str, start: u32, lines: u32, text: &str) -> ReviewComment {
        let quote = (0..lines).map(|n| format!("line {n}")).collect();
        self.state
            .review_edit(
                &self.project(),
                dir,
                ReviewEditOp::Add {
                    path: path.into(),
                    side: Side::New,
                    start,
                    end: start + lines - 1,
                    quote,
                    text: text.into(),
                    pr: None,
                },
            )
            .expect("a valid comment is accepted");
        self.comments(dir)
            .into_iter()
            .find(|c| c.text == text)
            .expect("the added comment is held")
    }

    /// Entry `dir`'s comments as the service holds them now.
    fn comments(&self, dir: &str) -> Vec<ReviewComment> {
        comments_of(&self.state, &self.project(), dir)
    }

    /// Comment `of` as the service holds it now, in whichever entry.
    fn comment(&self, of: &ReviewComment) -> ReviewComment {
        self.comments_everywhere()
            .into_iter()
            .find(|c| c.id == of.id)
            .expect("the comment is still held")
    }

    fn comments_everywhere(&self) -> Vec<ReviewComment> {
        self.state
            .review_pushes_on_attach(&self.project())
            .into_iter()
            .flat_map(|msg| match msg {
                DaemonMsg::ReviewChanged { comments, .. } => comments,
                _ => Vec::new(),
            })
            .collect()
    }

    /// Send entry `dir`'s Drafts; the send delivers them to the session it starts.
    async fn send(&self, dir: &str) {
        let result = self
            .state
            .review_send(&self.project(), dir, &[], None)
            .await;
        assert!(
            matches!(result, Ok(OperationResult::ReviewSent { .. })),
            "the send is delivered: {result:?}"
        );
    }

    /// Call `report_review_comment` as `caller`, expecting success.
    async fn report_ok(&self, caller: SessionId, args: Value) -> Value {
        let bearer = credential(&self.state, caller);
        call_ok(self.addr, &bearer, "report_review_comment", args).await
    }

    /// Call `report_review_comment` as `caller`, expecting a failure: its `{category, message}`.
    async fn report_err(&self, caller: SessionId, args: Value) -> Value {
        let bearer = credential(&self.state, caller);
        call_err(self.addr, &bearer, "report_review_comment", args).await
    }

    /// The stand-in waits `secs` before it draws anything.
    fn slow(&self, secs: f32) {
        std::fs::write(self.bin.path().join("slow"), secs.to_string()).unwrap();
    }

    /// Every session the stand-in ran for (`input.<id>` files).
    fn started(&self) -> Vec<SessionId> {
        std::fs::read_dir(self.bin.path())
            .unwrap()
            .filter_map(|entry| {
                let name = entry.ok()?.file_name().into_string().ok()?;
                let id = uuid::Uuid::parse_str(name.strip_prefix("input.")?).ok()?;
                Some(SessionId::from_uuid(id))
            })
            .collect()
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        for id in self.started() {
            if let Some(pty) = self.state.live_session(id) {
                let _ = pty.kill();
            }
        }
    }
}

/// Entry `dir` of `project`'s comments as `state` holds them.
fn comments_of(state: &DaemonState, project: &Path, dir: &str) -> Vec<ReviewComment> {
    state
        .review_pushes_on_attach(project)
        .into_iter()
        .find_map(|msg| match msg {
            DaemonMsg::ReviewChanged {
                worktree_dir,
                comments,
                ..
            } if worktree_dir == dir => Some(comments),
            _ => None,
        })
        .unwrap_or_default()
}

/// The arguments of a report on `comment`.
fn report(comment: &ReviewComment, outcome: &str, reply: &str) -> Value {
    json!({"comment": comment.reference().to_string(), "outcome": outcome, "reply": reply})
}

/// Wait up to `within` for a `ReviewChanged` of entry `dir` on `window` whose comments satisfy
/// `pred`; every frame seen meanwhile is returned in `seen`.
async fn window_gets(
    window: &mut Window,
    dir: &str,
    within: Duration,
    seen: &mut Vec<DaemonMsg>,
    pred: impl Fn(&[ReviewComment]) -> bool,
) -> bool {
    let wait = async {
        while let Some(frame) = window.recv().await {
            let Frame::Control(msg) = frame else {
                continue;
            };
            let hit = matches!(&msg, DaemonMsg::ReviewChanged { worktree_dir, comments, .. }
                if worktree_dir == dir && pred(comments));
            seen.push(msg);
            if hit {
                return true;
            }
        }
        false
    };
    tokio::time::timeout(within, wait).await.unwrap_or(false)
}

/// Whether `comments` hold `id` reported with `outcome`.
fn reported(comments: &[ReviewComment], of: &ReviewComment, outcome: Outcome) -> bool {
    comments.iter().any(|c| {
        c.id == of.id
            && matches!(&c.state, CommentState::Reported { report, .. } if report.outcome() == outcome)
    })
}

fn is_sent(comment: &ReviewComment) -> bool {
    matches!(comment.state, CommentState::Sent { .. })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn s1_a_report_is_stored_pushed_to_every_window_and_kept_after_a_restart() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let project = s.project();
    let first = s.add("wt", "src/a.rs", 12, 3, "Rename a.");
    s.add("wt", "src/b.rs", 4, 1, "And b.");
    s.send("wt").await;
    assert!(is_sent(&s.comment(&first)), "the send delivered it");
    let mut one = fake_window(&s.state);
    let mut two = fake_window(&s.state);

    let result = s.report_ok(sid(1), report(&first, "applied", REPLY)).await;
    let answered = tokio::time::Instant::now();
    assert_eq!(
        result,
        json!({
            "comment": first.reference().to_string(),
            "status": "applied",
            "file": "src/a.rs",
            "lines": "12-14",
            "previous_status": "sent",
        }),
        "the contract's success result"
    );

    let mut seen = Vec::new();
    assert!(
        window_gets(&mut two, "wt", Duration::from_secs(2), &mut seen, |c| {
            reported(c, &first, Outcome::Applied)
        })
        .await,
        "the second window receives the push within 2 s of the result (FR-007, SC-005): {seen:?}"
    );
    assert!(answered.elapsed() <= Duration::from_secs(2));
    assert!(
        window_gets(&mut one, "wt", BOUND, &mut seen, |c| {
            reported(c, &first, Outcome::Applied)
        })
        .await,
        "every window receives it (US1 s1)"
    );
    let now = s.comment(&first);
    let CommentState::Reported { report, .. } = &now.state else {
        panic!("reported: {now:?}");
    };
    assert_eq!(report.reply(), REPLY);
    assert_eq!(report.outcome(), Outcome::Applied);
    assert_eq!(report.by(), Reporter::Agent { session: sid(1).0 });
    assert_eq!(report.first_at(), report.last_at(), "a first report");

    // No confirmation was asked (FR-003): nothing of the kind reached either window.
    tokio::time::sleep(QUIET).await;
    while let Ok(Frame::Control(msg)) = one.try_recv().or_else(|_| two.try_recv()) {
        seen.push(msg);
    }
    assert!(
        !seen
            .iter()
            .any(|msg| matches!(msg, DaemonMsg::ConfirmationRequested { .. })),
        "a report never waits for the user: {seen:?}"
    );

    // The audit line names the reference, never the reply (034 FR-018). The log is shared by
    // every test of this binary, so only this comment's lines count.
    let reference = first.reference().to_string();
    let lines: Vec<String> = log_lines_for(sid(1))
        .into_iter()
        .filter(|line| line.contains("report_review_comment") && line.contains(&reference))
        .collect();
    assert_eq!(lines.len(), 1, "one audit line: {lines:?}");
    assert!(
        lines[0].contains(&first.reference().to_string()),
        "{}",
        lines[0]
    );
    assert!(lines[0].contains("ok"), "{}", lines[0]);
    let logged = String::from_utf8_lossy(&log().lock().unwrap()).into_owned();
    assert!(
        !logged.contains("secret-reply-946"),
        "the reply is never logged"
    );

    // A restarted service has it (US1 s7, FR-008).
    let restarted = service(s.store.path());
    let after = comments_of(&restarted, &project, "wt");
    assert!(
        reported(&after, &first, Outcome::Applied),
        "the report is back after a restart: {after:?}"
    );
    let back = after.iter().find(|c| c.id == first.id).unwrap();
    assert_eq!(
        back.state, now.state,
        "status, reply, times and reporter all kept"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn refusals_follow_the_contracts_order_and_change_nothing() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let sent = s.add("wt", "src/a.rs", 1, 1, "Sent one.");
    s.send("wt").await;
    let draft = s.add("wt", "src/a.rs", 5, 1, "Still a draft.");
    let before = s.comment(&sent);

    // s3: answered needs a reply.
    let error = s
        .report_err(sid(1), report(&sent, "answered", "  \n "))
        .await;
    assert_eq!(error["category"], "invalid_input", "{error}");
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("needs an explanation"),
        "{error}"
    );
    assert_eq!(s.comment(&sent), before, "the comment stays Sent (s3)");

    // 5a: a session of another worktree.
    let error = s.report_err(sid(3), report(&sent, "applied", "")).await;
    assert_eq!(error["category"], "refused_by_policy", "{error}");
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("another worktree"),
        "{error}"
    );
    assert_eq!(s.comment(&sent), before, "5a changes nothing");

    // 7: a Draft was never sent.
    let error = s.report_err(sid(1), report(&draft, "applied", "")).await;
    assert_eq!(error["category"], "conflict", "{error}");
    assert!(
        error["message"].as_str().unwrap().contains("still a draft"),
        "{error}"
    );
    assert_eq!(s.comment(&draft).state, CommentState::Draft);

    // s5: a later report replaces the first; first_at is kept.
    s.report_ok(sid(1), report(&sent, "applied", "")).await;
    let CommentState::Reported { report: first, .. } = s.comment(&sent).state else {
        panic!("reported");
    };
    let again = s
        .report_ok(sid(1), report(&sent, "answered", "Not needed after all."))
        .await;
    assert_eq!(again["previous_status"], "applied", "{again}");
    assert_eq!(again["status"], "answered", "{again}");
    assert_eq!(again["lines"], "1", "one line is named alone: {again}");
    let CommentState::Reported { report: second, .. } = s.comment(&sent).state else {
        panic!("reported");
    };
    assert_eq!(second.outcome(), Outcome::Answered);
    assert_eq!(second.reply(), "Not needed after all.");
    assert_eq!(second.first_at(), first.first_at(), "first_at kept (s5)");
    assert!(second.last_at() >= first.last_at());

    // 5b: a cleared comment is gone.
    s.state
        .review_edit(
            &s.project(),
            "wt",
            ReviewEditOp::ClearSent {
                scope: Scope::WorkingTree,
            },
        )
        .unwrap();
    let error = s.report_err(sid(1), report(&sent, "applied", "")).await;
    assert_eq!(error["category"], "not_found", "{error}");
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("no such comment"),
        "{error}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn d5_a_default_session_reports_on_the_default_entrys_comment() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let root = s.add("", "src/root.rs", 2, 2, "Explain more.");
    let wt = s.add("wt", "src/a.rs", 1, 1, "Worktree one.");
    s.send("").await;
    s.send("wt").await;

    let result = s
        .report_ok(
            sid(4),
            report(&root, "answered", "Already explained above."),
        )
        .await;
    assert_eq!(result["status"], "answered", "{result}");
    assert_eq!(result["file"], "src/root.rs", "{result}");
    assert!(reported(&s.comments(""), &root, Outcome::Answered));

    let error = s.report_err(sid(4), report(&wt, "applied", "")).await;
    assert_eq!(
        error["category"], "refused_by_policy",
        "a Default session reports only on the Default entry: {error}"
    );
    let error = s.report_err(sid(1), report(&root, "applied", "")).await;
    assert_eq!(
        error["category"], "refused_by_policy",
        "a worktree session cannot report on the Default entry: {error}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn s6_a_comment_inside_an_open_send_is_a_conflict() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let comment = s.add("wt", "src/a.rs", 1, 1, "Held open.");
    s.slow(2.0);
    let mut window = fake_window(&s.state);
    let sending = {
        let state = Arc::clone(&s.state);
        let project = s.project();
        tokio::spawn(async move { state.review_send(&project, "wt", &[], None).await })
    };
    let opened = async {
        while let Some(frame) = window.recv().await {
            if let Frame::Control(DaemonMsg::ReviewChanged { sending: true, .. }) = frame {
                return;
            }
        }
    };
    tokio::time::timeout(BOUND, opened)
        .await
        .expect("the send opens");

    let error = s.report_err(sid(1), report(&comment, "applied", "")).await;
    assert_eq!(error["category"], "conflict", "{error}");
    assert!(
        error["message"].as_str().unwrap().contains("being sent"),
        "{error}"
    );

    let result = sending.await.unwrap();
    assert!(result.is_ok(), "{result:?}");
    assert!(is_sent(&s.comment(&comment)), "the report changed nothing");
}

/// FR-011 (characterization: reports are keyed by `CommentRef`, so this may pass on arrival): a
/// report on a pull request's comment shows Applied or Answered with the reply, as for a
/// working-tree comment.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fr011_a_report_on_a_pull_request_comment_is_accepted_as_for_a_working_tree_one() {
    use micold_core::protocol::messages::PrSend;
    use micold_core::review::comment::PrAnchor;
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let project = s.project();
    let commit = "c".repeat(40);
    for text in ["Rename a.", "Why b?"] {
        s.state
            .review_edit(
                &project,
                "wt",
                ReviewEditOp::Add {
                    path: "src/a.rs".into(),
                    side: Side::New,
                    start: 1,
                    end: 1,
                    quote: vec!["x".into()],
                    text: text.into(),
                    pr: Some(PrAnchor {
                        number: 4,
                        head_branch: "feat/x".into(),
                        head_commit: commit.clone(),
                    }),
                },
            )
            .expect("a pull request comment is accepted");
    }
    let pr = PrSend {
        number: 4,
        title: "T".into(),
        head_branch: "feat/x".into(),
        state: micold_core::pull_request::BranchPrState::Open,
        current_head: commit,
    };
    let sent = s.state.review_send(&project, "wt", &[], Some(&pr)).await;
    assert!(
        matches!(sent, Ok(OperationResult::ReviewSent { .. })),
        "{sent:?}"
    );
    let held = s.comments("wt");
    let (first, second) = (held[0].clone(), held[1].clone());
    assert!(is_sent(&first) && is_sent(&second));

    let result = s.report_ok(sid(1), report(&first, "applied", REPLY)).await;
    assert_eq!(result["status"], "applied", "{result}");
    s.report_ok(sid(1), report(&second, "answered", "Because."))
        .await;

    let after = s.comments("wt");
    assert!(reported(&after, &first, Outcome::Applied));
    assert!(reported(&after, &second, Outcome::Answered));
    let CommentState::Reported { report, .. } = &s.comment(&second).state else {
        panic!("reported");
    };
    assert_eq!(report.reply(), "Because.");
    assert!(
        after.iter().all(|c| c.pr.is_some()),
        "still pull request comments"
    );
}
