//! Feature 1161 US4 (contracts/mcp-tools.md; FR-008, FR-009, FR-011, FR-012, FR-014; SC-003,
//! SC-004): `create_session` and `create_worktree` type the default prompt template, rendered
//! against the worktree, as the session's first input when the call gives no prompt of its own.
//!
//! A stand-in `claude` appends what is typed into it to `<bin>/claude.input`; a fake `IssueLookup`
//! answers the title and body, and the lookup bound is shortened instead of waited out.

// unix-only: the stand-in CLI is a `#!/bin/sh` script.
#![cfg(unix)]

#[path = "support/mcp.rs"]
mod mcp_support;
#[path = "support/stand_in_cli.rs"]
mod stand_in_cli;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use mcp_support::*;
use micold_core::github::{
    FakeIssueSource, GithubRepo, IssueLoadError, IssueLookup, IssueLookupError, IssueSnapshot,
};
use micold_core::issue_details::{IssueDetails, IssueDetailsSource};
use micold_core::prompt_template::shared_fixture;
use micold_core::session::{AiCli, TerminalMode};
use micold_daemon::state::DaemonState;
use serde_json::{json, Value};
use stand_in_cli::{install_cli, trust_project, Env};

/// The tests change process-wide variables (`PATH`, `HOME`, ...); one at a time.
static ENV: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

const READY_BOUND: Duration = Duration::from_secs(15);

struct Sandbox {
    _env: Env,
    bin: tempfile::TempDir,
    project: tempfile::TempDir,
    _home: tempfile::TempDir,
    store: tempfile::TempDir,
    state: Arc<DaemonState>,
    addr: std::net::SocketAddr,
}

impl Sandbox {
    async fn new() -> Self {
        let bin = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        let store = tempfile::tempdir().unwrap();
        install_cli(bin.path(), "claude", true);
        init_repo(project.path());
        for dir in ["caller", "b"] {
            add_worktree(project.path(), dir);
        }
        git(
            project.path(),
            &["remote", "add", "origin", "https://github.com/o/r.git"],
        );
        trust_project(home.path(), project.path());
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
        let caller = session(
            sid(3),
            Some("caller"),
            TerminalMode::AiCli,
            AiCli::ClaudeCode,
        );
        let state = state_over(
            vec![(project.path().to_path_buf(), true, vec![caller])],
            store.path(),
        );
        state.set_first_prompt_bound(READY_BOUND);
        let addr = serve_tool_server(&state, store.path().join("mcp")).await;
        Self {
            _env: env,
            bin,
            project,
            _home: home,
            store,
            state,
            addr,
        }
    }

    /// A worktree `dir`, which the state sees at its next catalog read.
    fn worktree(&self, dir: &str) {
        add_worktree(self.project.path(), dir);
        self.state.refresh_worktrees(self.project.path());
    }

    fn template(&self, text: &str) {
        self.state
            .set_default_prompt_template(text.to_string())
            .unwrap();
    }

    fn lookup(&self, lookup: Arc<dyn IssueLookup>) {
        self.state
            .set_issue_lookup(move |_| Some(Arc::clone(&lookup)));
    }

    async fn call(&self, tool: &str, args: Value) -> Value {
        call_ok(self.addr, &credential(&self.state, sid(3)), tool, args).await
    }

    async fn session_in(&self, worktree: &str, extra: Value) -> Value {
        let mut args = json!({"worktree": worktree, "ai_cli": "claude_code"});
        args.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        self.call("create_session", args).await
    }

    fn typed(&self) -> String {
        std::fs::read_to_string(self.bin.path().join("claude.input")).unwrap_or_default()
    }

    /// Wait (up to 10 s) until `claude` received `expected`; the caller asserts.
    async fn settle(&self, expected: &str) -> String {
        let deadline = Instant::now() + Duration::from_secs(10);
        while self.typed() != expected && Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        self.typed()
    }

    fn kill_all(&self) {
        for project in self.state.catalog_snapshot().projects {
            for s in project.sessions {
                if let Some(pty) = self.state.live_session(s.id) {
                    let _ = pty.kill();
                }
            }
        }
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        self.kill_all();
        let _ = (&self.project, &self.store);
    }
}

fn snapshot(title: &str, body: &str) -> IssueSnapshot {
    IssueSnapshot {
        number: 42,
        title: title.into(),
        labels: vec![],
        body_text: body.into(),
    }
}

fn answering(snapshot: IssueSnapshot) -> Arc<FakeIssueSource> {
    Arc::new(FakeIssueSource::new().with_lookup(Ok(snapshot)))
}

fn failing() -> Arc<FakeIssueSource> {
    Arc::new(
        FakeIssueSource::new().with_lookup(Err(IssueLookupError::Load(IssueLoadError::NoAccess))),
    )
}

/// A lookup that answers after `delay`, to outlast the shortened bound.
struct Slow(Duration);

impl IssueDetailsSource for Slow {
    fn read_details(
        &self,
        _: &GithubRepo,
        _: u32,
        _: Instant,
    ) -> Result<IssueDetails, IssueLoadError> {
        Err(IssueLoadError::NoAccess)
    }
}

impl IssueLookup for Slow {
    fn read_issue(&self, _: &GithubRepo, _: u32) -> Result<IssueSnapshot, IssueLookupError> {
        std::thread::sleep(self.0);
        Ok(snapshot("Never seen", "Never seen"))
    }
}

const TITLED: &str = "Fix {issue_title} (#{issue})";

/// US4 s1, FR-008: the template, rendered against the worktree and its issue, is typed.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_session_types_the_rendered_template() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    s.worktree("fix-42_crash");
    s.template(TITLED);
    s.lookup(answering(snapshot("Crash on save", "")));
    let out = s.session_in("fix-42_crash", json!({})).await;
    assert_eq!(out["prompt_delivered"], json!(true), "{out}");
    assert_eq!(
        s.settle("Fix Crash on save (#42)\n").await,
        "Fix Crash on save (#42)\n"
    );
}

/// US4 s2, FR-008, SC-004, research R8: an explicit prompt wins and the template is neither parsed
/// nor looked up.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_explicit_prompt_wins_and_nothing_is_looked_up() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    s.worktree("fix-42_crash");
    s.template("{no_such_placeholder} {issue_title}");
    let fake = failing();
    s.lookup(fake.clone());
    let out = s.session_in("fix-42_crash", json!({"prompt": "hi"})).await;
    assert_eq!(out["prompt_delivered"], json!(true), "{out}");
    assert_eq!(s.settle("hi\n").await, "hi\n");
    assert!(fake.lookup_calls().is_empty(), "no issue was read");
}

/// US4 s3, FR-009: `create_worktree` with `ai_cli` and no prompt types the template, and the output
/// equals the shared fixture's for the same facts (FR-011).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_worktree_types_the_template_the_dialog_would() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    s.template(shared_fixture::TEMPLATE);
    s.lookup(answering(snapshot("Crash on save", "It crashes. {issue}")));
    let out = s
        .call(
            "create_worktree",
            json!({"branch": "fix/42_crash", "name": "fix-42_crash", "ai_cli": "claude_code"}),
        )
        .await;
    assert_eq!(out["prompt_delivered"], json!(true), "{out}");
    let expected = format!("{}\n", shared_fixture::OUTPUT);
    assert_eq!(s.settle(&expected).await, expected);
}

/// US4 s4, SC-003: an empty template changes nothing: no prompt is typed, none is reported.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_empty_template_types_nothing() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let out = s.session_in("b", json!({})).await;
    assert_eq!(out["prompt_delivered"], Value::Null, "{out}");
    assert_eq!(s.typed(), "");
}

/// US4 s5, FR-009: a template alone starts no session for `create_worktree`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_template_alone_starts_no_session() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    s.template(TITLED);
    let out = s.call("create_worktree", json!({"branch": "feat-y"})).await;
    assert!(out.get("session").is_none(), "{out}");
    assert_eq!(s.typed(), "");
}

/// US4 s6, FR-012: a failed lookup and one that outlasts the bound both render title and body
/// empty.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_failed_or_slow_lookup_renders_title_and_body_empty() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    s.worktree("fix-42_crash");
    s.template(TITLED);
    s.state.set_prompt_lookup_bound(Duration::from_millis(200));
    s.lookup(failing());
    s.session_in("fix-42_crash", json!({})).await;
    assert_eq!(s.settle("Fix  (#42)\n").await, "Fix  (#42)\n");

    let s2 = Sandbox::new().await;
    s2.worktree("fix-42_crash");
    s2.template(TITLED);
    s2.state.set_prompt_lookup_bound(Duration::from_millis(200));
    s2.lookup(Arc::new(Slow(Duration::from_secs(2))));
    let started = Instant::now();
    s2.session_in("fix-42_crash", json!({})).await;
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "the bound ended the wait"
    );
    assert_eq!(s2.settle("Fix  (#42)\n").await, "Fix  (#42)\n");
}

/// US4 s7, FR-014: a Jira key is `{issue}` and is never looked up.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_jira_key_renders_as_the_issue_with_no_lookup() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    s.worktree("fix-ABC-12_login");
    s.template(TITLED);
    let fake = failing();
    s.lookup(fake.clone());
    s.session_in("fix-ABC-12_login", json!({})).await;
    assert_eq!(s.settle("Fix  (#ABC-12)\n").await, "Fix  (#ABC-12)\n");
    assert!(fake.lookup_calls().is_empty(), "a Jira key reads nothing");
}

/// Research R8: a blank explicit prompt falls through to the template; a template that does not
/// parse leaves the call as it was.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_blank_prompt_falls_through_and_a_bad_template_is_no_prompt() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    s.template("on {branch}");
    s.session_in("b", json!({"prompt": "  "})).await;
    assert_eq!(s.settle("on b\n").await, "on b\n");

    let s2 = Sandbox::new().await;
    s2.template("{nope}");
    let out = s2.session_in("b", json!({})).await;
    assert_eq!(out["prompt_delivered"], Value::Null, "{out}");
    assert_eq!(s2.typed(), "");
}
