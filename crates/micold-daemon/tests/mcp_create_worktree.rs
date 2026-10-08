//! Feature 034 (contracts/mcp-tools.md `create_worktree`; US2 scenarios 1–2, US3 scenario 6; A7,
//! A8, U145–U151, U219): an agent creates a worktree through `POST /mcp` exactly as the
//! create-worktree dialog would, from a session in a worktree or in the project root (Default;
//! constitution 1.7.0).
//!
//! The fixture is a real git repository with worktree `b` and a free local branch `taken`, and
//! sessions S1 (project root) and S3 (`b`, the usual caller), plus S10–S19 in `b` for the race.

#[path = "support/mcp.rs"]
mod mcp_support;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use mcp_support::*;
use micold_core::naming::{dir_name_from_branch, NamingError};
use micold_core::session::{AiCli, SessionId, TerminalMode};
use micold_daemon::state::DaemonState;
use serde_json::{json, Value};

/// SC-003: a change reaches every window within this bound.
const WINDOW_BOUND: Duration = Duration::from_secs(2);

struct Fixture {
    state: Arc<DaemonState>,
    addr: std::net::SocketAddr,
    project: tempfile::TempDir,
    _store: tempfile::TempDir,
}

impl Fixture {
    async fn new() -> Self {
        let project = tempfile::tempdir().unwrap();
        let store = tempfile::tempdir().unwrap();
        init_repo(project.path());
        add_worktree(project.path(), "b");
        git(project.path(), &["branch", "taken"]);
        let mut sessions = vec![
            session(sid(1), None, TerminalMode::AiCli, AiCli::ClaudeCode),
            session(sid(3), Some("b"), TerminalMode::AiCli, AiCli::ClaudeCode),
        ];
        sessions.extend(
            (10..20).map(|n| session(sid(n), Some("b"), TerminalMode::AiCli, AiCli::ClaudeCode)),
        );
        let state = state_over(
            vec![(project.path().to_path_buf(), true, sessions)],
            store.path(),
        );
        let addr = serve_tool_server(&state, store.path().join("mcp")).await;
        Self {
            state,
            addr,
            project,
            _store: store,
        }
    }

    fn repo(&self) -> &Path {
        self.project.path()
    }

    async fn call(&self, caller: SessionId, args: Value) -> Value {
        call_tool(
            self.addr,
            &credential(&self.state, caller),
            "create_worktree",
            args,
        )
        .await
    }

    async fn ok(&self, caller: SessionId, args: Value) -> Value {
        call_ok(
            self.addr,
            &credential(&self.state, caller),
            "create_worktree",
            args,
        )
        .await
    }

    async fn err(&self, caller: SessionId, args: Value) -> Value {
        call_err(
            self.addr,
            &credential(&self.state, caller),
            "create_worktree",
            args,
        )
        .await
    }

    /// The directories under the managed root.
    fn on_disk(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(self.repo().join(".claude/worktrees"))
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// The worktree refs `list_worktrees` reports.
    async fn listed(&self) -> Vec<String> {
        let out = call_ok(
            self.addr,
            &credential(&self.state, sid(3)),
            "list_worktrees",
            json!({"include_hidden": true}),
        )
        .await;
        out["worktrees"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["ref"].as_str().unwrap().to_string())
            .collect()
    }

    /// Nothing on disk or in the listing changed from `before`.
    async fn assert_unchanged(&self, before: &(Vec<String>, Vec<String>)) {
        assert_eq!(
            &(self.on_disk(), self.listed().await),
            before,
            "a failed create changes nothing (FR-013)"
        );
    }

    async fn snapshot(&self) -> (Vec<String>, Vec<String>) {
        (self.on_disk(), self.listed().await)
    }
}

fn native(path: PathBuf) -> String {
    path.display().to_string()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_worktree_makes_the_dialogs_worktree_and_every_window_sees_it() {
    let f = Fixture::new().await;
    let mut window = fake_window(&f.state);
    let asked = Instant::now();
    let row = f.ok(sid(3), json!({"branch": "feat-x"})).await;
    let repo = f.repo().to_path_buf();
    let seen = window_sees(&mut window, WINDOW_BOUND, |catalog| {
        catalog
            .projects
            .iter()
            .any(|p| p.path == repo && p.worktrees.iter().any(|w| w.dir_name == "feat-x"))
    })
    .await;
    assert!(
        seen && asked.elapsed() < WINDOW_BOUND + Duration::from_secs(1),
        "a window sees the new worktree without a refresh (SC-003)"
    );
    assert_eq!(
        row,
        json!({
            "ref": "feat-x",
            "display_name": row["display_name"],
            "branch": "feat-x",
            "path": native(f.repo().join(".claude").join("worktrees").join("feat-x")),
            "status": "clean",
            "app_created": true,
            "assistant_owned": false,
            "session_count": 0,
        })
    );
    assert!(f.repo().join(".claude/worktrees/feat-x/.git").exists());
    let (records, _) = f.state.provenance(f.repo());
    assert!(
        records.contains("feat-x"),
        "recorded as created by the app, as the dialog's create is (FR-009)"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn without_a_name_the_directory_is_derived_from_the_branch() {
    let f = Fixture::new().await;
    let row = f.ok(sid(3), json!({"branch": "feat/login-form"})).await;
    assert_eq!(row["ref"], dir_name_from_branch("feat/login-form"));
    assert_eq!(row["branch"], "feat/login-form");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_existing_or_checked_out_branch_is_a_conflict_and_nothing_changes() {
    let f = Fixture::new().await;
    let before = f.snapshot().await;

    let existing = f.err(sid(3), json!({"branch": "taken"})).await;
    assert_eq!(existing["category"], "conflict", "{existing}");
    let message = existing["message"].as_str().unwrap();
    assert!(
        message.contains("already exists") && message.contains("existing_local"),
        "names the pre-flight's situation and the mode that fits it: {message}"
    );
    f.assert_unchanged(&before).await;

    let held = f
        .err(sid(3), json!({"branch": "b", "name": "b-again"}))
        .await;
    assert_eq!(held["category"], "conflict", "{held}");
    assert!(
        held["message"]
            .as_str()
            .unwrap()
            .contains("already checked out in the worktree 'b'"),
        "the dialog's own sentence for a held branch: {held}"
    );
    f.assert_unchanged(&before).await;

    let root = f
        .err(sid(3), json!({"branch": "main", "name": "main-again"}))
        .await;
    assert_eq!(root["category"], "conflict", "{root}");
    f.assert_unchanged(&before).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_name_the_naming_rules_reject_is_invalid_input_and_creates_nothing() {
    let f = Fixture::new().await;
    let before = f.snapshot().await;

    let empty = f
        .err(sid(3), json!({"branch": "feat-y", "name": "!!!"}))
        .await;
    assert_eq!(empty["category"], "invalid_input");
    assert!(
        empty["message"]
            .as_str()
            .unwrap()
            .contains(&NamingError::EmptyNameAfterSlug.to_string()),
        "the dialog's message: {empty}"
    );

    let spaced = f
        .err(sid(3), json!({"branch": "feat-y", "name": "Feat Y"}))
        .await;
    assert_eq!(spaced["category"], "invalid_input");
    assert!(
        spaced["message"].as_str().unwrap().contains("feat-y"),
        "says which name the rules would accept: {spaced}"
    );
    f.assert_unchanged(&before).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_branch_name_git_rejects_is_invalid_input_with_gits_message() {
    let f = Fixture::new().await;
    let before = f.snapshot().await;
    let error = f
        .err(sid(3), json!({"branch": "bad..branch", "name": "bad"}))
        .await;
    assert_eq!(error["category"], "invalid_input", "{error}");
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("not a valid branch name"),
        "git's own words: {error}"
    );
    f.assert_unchanged(&before).await;
}

/// `git check-ref-format --branch` expands `@{-N}` to the branch checked out before, so it would
/// pass; the agent names a branch, never a reflog shorthand (EC-3, U229).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_previous_branch_shorthand_is_invalid_input() {
    let f = Fixture::new().await;
    // A checkout history, so `@{-1}` names a real branch (`prev`) for git to expand to.
    git(f.project.path(), &["checkout", "-q", "-b", "prev"]);
    git(f.project.path(), &["checkout", "-q", "-"]);
    let before = f.snapshot().await;
    let error = f.err(sid(3), json!({"branch": "@{-1}"})).await;
    assert_eq!(error["category"], "invalid_input", "{error}");
    f.assert_unchanged(&before).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn existing_local_checks_out_a_free_branch() {
    let f = Fixture::new().await;
    let row = f
        .ok(sid(3), json!({"branch": "taken", "mode": "existing_local"}))
        .await;
    assert_eq!(row["ref"], "taken");
    assert_eq!(row["branch"], "taken");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ten_concurrent_creates_of_one_branch_leave_one_worktree() {
    let f = Arc::new(Fixture::new().await);
    let calls: Vec<_> = (10..20)
        .map(|n| {
            let f = Arc::clone(&f);
            tokio::spawn(async move { f.call(sid(n), json!({"branch": "race"})).await })
        })
        .collect();
    let mut ok = 0;
    let mut conflicts = 0;
    for call in calls {
        let result = call.await.unwrap();
        if result["isError"] == json!(false) {
            ok += 1;
        } else if result["structuredContent"]["error"]["category"] == "conflict" {
            conflicts += 1;
        } else {
            panic!("neither a success nor a conflict: {result}");
        }
    }
    assert_eq!((ok, conflicts), (1, 9), "SC-005");
    assert_eq!(f.on_disk(), ["b", "race"], "exactly one worktree on disk");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn back_to_back_creates_from_one_caller_all_succeed() {
    let f = Fixture::new().await;
    for n in 0..5 {
        f.ok(sid(3), json!({"branch": format!("burst-{n}")})).await;
    }
    assert_eq!(
        f.on_disk(),
        ["b", "burst-0", "burst-1", "burst-2", "burst-3", "burst-4"],
        "no cap beyond the user's own (EC-15, D3)"
    );
}

/// U219 (constitution 1.7.0, FR-015a; the isolation & lifecycle gate): a session in the project
/// root (Default) creates a worktree through the tool server and gets exactly what a session in
/// a worktree gets: the dialog's worktree, recorded as created by the app, in every window.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_default_session_creates_a_worktree_as_any_session_does() {
    let f = Fixture::new().await;
    let mut window = fake_window(&f.state);
    let row = f.ok(sid(1), json!({"branch": "feat-x"})).await;
    assert_eq!(
        row,
        json!({
            "ref": "feat-x",
            "display_name": row["display_name"],
            "branch": "feat-x",
            "path": native(f.repo().join(".claude").join("worktrees").join("feat-x")),
            "status": "clean",
            "app_created": true,
            "assistant_owned": false,
            "session_count": 0,
        })
    );
    assert!(f.repo().join(".claude/worktrees/feat-x/.git").exists());
    assert_eq!(f.on_disk(), ["b", "feat-x"]);
    assert!(f.listed().await.contains(&"feat-x".to_string()));
    let (records, _) = f.state.provenance(f.repo());
    assert!(
        records.contains("feat-x"),
        "recorded as created by the app, as the dialog's create is (FR-009)"
    );
    let repo = f.repo().to_path_buf();
    assert!(
        window_sees(&mut window, WINDOW_BOUND, |catalog| {
            catalog
                .projects
                .iter()
                .any(|p| p.path == repo && p.worktrees.iter().any(|w| w.dir_name == "feat-x"))
        })
        .await,
        "every window sees the new worktree (SC-003)"
    );

    // The calling session is still where it was: creating a worktree moves nobody into it.
    let who = call_ok(f.addr, &credential(&f.state, sid(1)), "whoami", json!({})).await;
    assert_eq!(who["worktree"], "default", "{who}");
}

/// The exception is create-only: a Default session that just created a worktree is still refused
/// renaming it, or any other, and the refusal no longer says it may not create.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_default_session_is_still_refused_rename_after_creating() {
    let f = Fixture::new().await;
    f.ok(sid(1), json!({"branch": "feat-x"})).await;
    let before = f.snapshot().await;
    for worktree in ["feat-x", "b"] {
        let error = call_err(
            f.addr,
            &credential(&f.state, sid(1)),
            "rename_worktree",
            json!({"worktree": worktree, "display_name": "Renamed"}),
        )
        .await;
        assert_eq!(
            error["category"], "refused_by_policy",
            "{worktree}: {error}"
        );
        let message = error["message"].as_str().unwrap();
        assert!(message.contains("Principle III"), "{worktree}: {error}");
        assert!(
            !message.contains("create"),
            "{worktree}: creating is allowed, so the refusal must not name it: {error}"
        );
    }
    f.assert_unchanged(&before).await;
}

/// A create the dialog would refuse is refused for a Default session the same way, by the same
/// check: the exception opens the policy gate and nothing behind it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_default_session_gets_the_dialogs_refusals_too() {
    let f = Fixture::new().await;
    let before = f.snapshot().await;
    let from_default = f.err(sid(1), json!({"branch": "b"})).await;
    let from_worktree = f.err(sid(3), json!({"branch": "b"})).await;
    assert_eq!(from_default, from_worktree);
    assert_ne!(
        from_default["category"], "refused_by_policy",
        "{from_default}"
    );
    f.assert_unchanged(&before).await;
}

// ---- Feature 550: the form's inputs (type, ticket, name) ----

/// US1 s1, s3, FR-003, FR-012, FR-013, SC-003: type, ticket and name give the form's branch,
/// directory and sidebar tags, and the result reports them.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn type_ticket_and_name_create_the_forms_worktree() {
    use micold_core::naming::{parse_tags, ConventionalType, Tag};

    let f = Fixture::new().await;
    let row = f
        .ok(
            sid(3),
            json!({"type": "fix", "ticket": "#123", "name": "login crash"}),
        )
        .await;
    assert_eq!(row["ref"], "fix-123_login-crash");
    assert_eq!(row["branch"], "fix/123_login-crash");
    assert_eq!(row["directory"], "fix-123_login-crash");
    assert_eq!(row["type"], "fix");
    assert_eq!(row["ticket"], "123");
    assert_eq!(
        row["path"],
        native(
            f.repo()
                .join(".claude")
                .join("worktrees")
                .join("fix-123_login-crash")
        )
    );
    assert_eq!(
        parse_tags(row["ref"].as_str().unwrap()),
        vec![Tag::Type(ConventionalType::Fix), Tag::Issue("#123".into())],
        "the sidebar derives the form's tags from the directory"
    );
    assert!(f
        .repo()
        .join(".claude/worktrees/fix-123_login-crash/.git")
        .exists());
    let (records, _) = f.state.provenance(f.repo());
    assert!(records.contains("fix-123_login-crash"));
    assert!(f
        .listed()
        .await
        .contains(&"fix-123_login-crash".to_string()));
}

/// US1 s2: no ticket means no ticket segment, no issue tag and no `ticket` in the result.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn without_a_ticket_there_is_no_ticket_segment_or_tag() {
    use micold_core::naming::{parse_tags, ConventionalType, Tag};

    let f = Fixture::new().await;
    let row = f
        .ok(sid(3), json!({"type": "feat", "name": "Dark mode"}))
        .await;
    let empty = f
        .ok(
            sid(3),
            json!({"type": "feat", "ticket": "", "name": "Dark mode two"}),
        )
        .await;
    assert_eq!(empty["branch"], "feat/dark-mode-two");
    assert!(empty.get("ticket").is_none(), "{empty}");
    assert_eq!(row["branch"], "feat/dark-mode");
    assert_eq!(row["directory"], "feat-dark-mode");
    assert_eq!(row["type"], "feat");
    assert!(row.get("ticket").is_none(), "{row}");
    assert_eq!(
        parse_tags("feat-dark-mode"),
        vec![Tag::Type(ConventionalType::Feat)]
    );
}

/// FR-004: the form's own refusals, and nothing is created.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_forms_naming_refusals_apply() {
    let f = Fixture::new().await;
    let before = f.snapshot().await;
    let no_type = f.err(sid(3), json!({"name": "x"})).await;
    assert_eq!(no_type["category"], "invalid_input", "{no_type}");
    assert_eq!(no_type["message"], NamingError::NoType.to_string());
    let no_name = f.err(sid(3), json!({"type": "fix", "name": "!!!"})).await;
    assert_eq!(no_name["category"], "invalid_input", "{no_name}");
    assert_eq!(
        no_name["message"],
        NamingError::EmptyNameAfterSlug.to_string()
    );
    let no_name = f.err(sid(3), json!({"type": "fix"})).await;
    assert_eq!(
        no_name["message"],
        NamingError::EmptyNameAfterSlug.to_string()
    );
    f.assert_unchanged(&before).await;
}

/// A derived branch that exists locally, only on a remote, or in a worktree is a conflict that
/// creates nothing (the wording is pinned in M2).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_derived_branch_that_already_exists_is_a_conflict() {
    let f = Fixture::new().await;
    git(f.repo(), &["branch", "fix/local-one"]);
    git(f.repo(), &["branch", "fix/held-one"]);
    let elsewhere = tempfile::tempdir().unwrap();
    let held = elsewhere.path().join("held").display().to_string();
    git(f.repo(), &["worktree", "add", "-q", &held, "fix/held-one"]);
    let remote = tempfile::tempdir().unwrap();
    git(remote.path(), &["init", "-q", "--bare", "."]);
    git(
        f.repo(),
        &[
            "remote",
            "add",
            "origin",
            &remote.path().display().to_string(),
        ],
    );
    git(f.repo(), &["branch", "fix/remote-one"]);
    git(f.repo(), &["push", "-q", "origin", "fix/remote-one"]);
    git(f.repo(), &["branch", "-q", "-D", "fix/remote-one"]);
    git(f.repo(), &["fetch", "-q", "origin"]);

    let before = f.snapshot().await;
    for name in ["local one", "remote one", "held one"] {
        let error = f.err(sid(3), json!({"type": "fix", "name": name})).await;
        assert_eq!(error["category"], "conflict", "{name}: {error}");
        assert!(
            error["message"].as_str().unwrap().contains("fix/"),
            "{name}: {error}"
        );
        f.assert_unchanged(&before).await;
    }
    let branches = std::process::Command::new("git")
        .current_dir(f.repo())
        .args(["branch", "--list", "fix/remote-one"])
        .output()
        .unwrap();
    assert!(branches.stdout.is_empty(), "no local branch was created");
}

/// US4 s1: literal call shapes behave as before, with no derived fields.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn literal_calls_are_unchanged() {
    let f = Fixture::new().await;
    let row = f
        .ok(sid(3), json!({"branch": "feat-lit", "name": "lit"}))
        .await;
    assert_eq!(row["ref"], "lit");
    assert_eq!(row["branch"], "feat-lit");
    assert!(row.get("type").is_none() && row.get("ticket").is_none());
    let mixed = f
        .err(sid(3), json!({"branch": "feat-no", "type": "fix"}))
        .await;
    assert_eq!(mixed["category"], "invalid_input", "{mixed}");
    assert!(!f.on_disk().contains(&"feat-no".to_string()));
}

/// The Default session may create a derived worktree, as it may a literal one.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_default_session_may_create_a_derived_worktree() {
    let f = Fixture::new().await;
    let row = f
        .ok(sid(1), json!({"type": "docs", "name": "guide update"}))
        .await;
    assert_eq!(row["branch"], "docs/guide-update");
}

/// A literal `github_issue` is refused until the lookup ships, and nothing is created.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn github_issue_is_refused_and_creates_nothing() {
    let f = Fixture::new().await;
    let before = f.snapshot().await;
    let error = f.err(sid(3), json!({"github_issue": 12})).await;
    assert_eq!(error["category"], "invalid_input", "{error}");
    f.assert_unchanged(&before).await;
}
