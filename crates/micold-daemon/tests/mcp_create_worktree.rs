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
            json!({}),
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

/// FR-011: the refusal names the branch and says to retry with `branch` and `mode`
/// `existing_local` or `track_remote`.
fn assert_retry_hint(error: &Value, branch: &str) {
    assert_eq!(error["category"], "conflict", "{error}");
    let message = error["message"].as_str().unwrap();
    for needle in [
        branch,
        "existing_local",
        "track_remote",
        "retry with branch",
    ] {
        assert!(message.contains(needle), "missing {needle:?}: {message}");
    }
}

/// The keys of a literal-branch result before feature 532 (FR-009).
const LITERAL_RESULT_KEYS: [&str; 6] = [
    "ref",
    "display_name",
    "branch",
    "path",
    "status",
    "session_count",
];

/// A non-empty folder under the managed root, which the pre-flight reports as taken.
fn occupy(f: &Fixture, dir: &str) {
    let dir = f.repo().join(".claude").join("worktrees").join(dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("keep"), "x").unwrap();
}

fn git_out(repo: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .expect("git runs");
    String::from_utf8_lossy(&out.stdout).into_owned()
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
            "session_count": 0,
        })
    );
    assert!(f.repo().join(".claude/worktrees/feat-x/.git").exists());
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
            "session_count": 0,
        })
    );
    assert!(f.repo().join(".claude/worktrees/feat-x/.git").exists());
    assert_eq!(f.on_disk(), ["b", "feat-x"]);
    assert!(f.listed().await.contains(&"feat-x".to_string()));
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
    assert!(f
        .listed()
        .await
        .contains(&"fix-123_login-crash".to_string()));
}

/// FR-003, SC-001, SC-002 (feature 532): a ticket in any accepted form names the worktree as the
/// dialog does, and the sidebar's issue tag (what `TagFilter::HasIssue` keys on) is present
/// exactly when the ticket is not blank.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_ticket_in_any_form_names_the_worktree_as_the_dialog_does() {
    use micold_core::naming::{derive, parse_tags, ConventionalType, Tag, WorktreeNaming};

    let f = Fixture::new().await;
    let mut n = 0;
    // Each ticket form with the issue tag the sidebar shows for it (literals, not `derive`).
    for (ticket, label) in [
        ("123", "#123"),
        ("#123", "#123"),
        ("ABC-123", "ABC-123"),
        ("", ""),
    ] {
        for type_ in [ConventionalType::Feat, ConventionalType::Fix] {
            for name in ["add thing", "Login crash"] {
                n += 1;
                // A unique suffix keeps each combination its own branch.
                let name = format!("{name} {n}");
                let expected = derive(&WorktreeNaming {
                    type_: Some(type_),
                    ticket: Some(ticket.to_string()),
                    name: name.clone(),
                })
                .unwrap();
                let row = f
                    .ok(
                        sid(3),
                        json!({"type": type_.as_str(), "ticket": ticket, "name": name}),
                    )
                    .await;
                assert_eq!(row["branch"], expected.branch, "{ticket:?} {row}");
                assert_eq!(row["directory"], expected.dir_name, "{ticket:?} {row}");
                let issue_tags: Vec<Tag> = parse_tags(row["ref"].as_str().unwrap())
                    .into_iter()
                    .filter(|tag| matches!(tag, Tag::Issue(_)))
                    .collect();
                let want: Vec<Tag> = if label.is_empty() {
                    vec![]
                } else {
                    vec![Tag::Issue(label.into())]
                };
                assert_eq!(issue_tags, want, "{type_:?} {name:?} {ticket:?}: {row}");
            }
        }
    }
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
        assert_retry_hint(&error, &format!("fix/{}", name.replace(' ', "-")));
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
    // FR-009: exactly the pre-feature keys; no hint, warning or extra field.
    let mut keys: Vec<&str> = row
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    let mut expected = LITERAL_RESULT_KEYS.to_vec();
    expected.sort_unstable();
    assert_eq!(keys, expected, "{row}");
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

/// SC-002, US2 s1-s3: each invalid input is refused with its reason and leaves nothing behind.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_invalid_input_is_refused_with_its_reason_and_leaves_nothing() {
    let f = Fixture::new().await;
    let before = f.snapshot().await;
    let bad_type = f.err(sid(3), json!({"type": "feature", "name": "x"})).await;
    assert_eq!(bad_type["category"], "invalid_input", "{bad_type}");
    let message = bad_type["message"].as_str().unwrap();
    assert!(
        message.contains("feature") && message.contains("fix"),
        "{message}"
    );
    let empty = f.err(sid(3), json!({"type": "fix", "name": "???"})).await;
    assert_eq!(
        empty["message"],
        NamingError::EmptyNameAfterSlug.to_string()
    );
    let no_type = f.err(sid(3), json!({"name": "thing"})).await;
    assert_eq!(no_type["message"], NamingError::NoType.to_string());
    f.assert_unchanged(&before).await;
    let branches = git_out(f.repo(), &["branch", "--list", "fix/*", "feature/*"]);
    assert!(branches.trim().is_empty(), "no branch left: {branches}");
}

/// US2 s4, FR-011: the same inputs twice; the second names the branch and says how to reuse it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_same_inputs_twice_refuse_with_the_retry_hint() {
    let f = Fixture::new().await;
    let args = json!({"type": "fix", "ticket": "7", "name": "again"});
    f.ok(sid(3), args.clone()).await;
    let before = f.snapshot().await;
    let second = f.err(sid(3), args).await;
    assert_retry_hint(&second, "fix/7_again");
    f.assert_unchanged(&before).await;
}

/// Plan Design 5: with only the directory taken the directory text wins, with the branch also
/// taken the branch message wins.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn directory_taken_gives_the_directory_text_unless_the_branch_exists() {
    let f = Fixture::new().await;
    occupy(&f, "fix-only-dir");
    let error = f
        .err(sid(3), json!({"type": "fix", "name": "only dir"}))
        .await;
    assert_eq!(error["category"], "conflict", "{error}");
    let message = error["message"].as_str().unwrap();
    assert!(message.contains("fix-only-dir"), "{message}");
    assert!(!message.contains("track_remote"), "{message}");

    git(f.repo(), &["branch", "fix/both"]);
    occupy(&f, "fix-both");
    let error = f.err(sid(3), json!({"type": "fix", "name": "both"})).await;
    assert_retry_hint(&error, "fix/both");
}

/// Two concurrent identical requests: one wins and no partial worktree is left.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_concurrent_identical_requests_leave_one_worktree() {
    let f = Arc::new(Fixture::new().await);
    let calls: Vec<_> = [10, 11]
        .into_iter()
        .map(|n| {
            let f = Arc::clone(&f);
            tokio::spawn(async move {
                f.call(sid(n), json!({"type": "feat", "name": "race"}))
                    .await
            })
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
    assert_eq!((ok, conflicts), (1, 1));
    assert_eq!(f.on_disk(), ["b", "feat-race"]);
    assert_eq!(
        git_out(f.repo(), &["branch", "--list", "feat/race"])
            .lines()
            .count(),
        1
    );
}

// ---- Feature 550 US3: a GitHub issue ----

mod github_issue {
    use super::*;
    use micold_core::github::{
        FakeIssueSource, IssueLoadError, IssueLookup, IssueLookupError, IssueSnapshot,
    };
    use micold_core::issue_types::LabelTypeEntry;
    use micold_core::naming::ConventionalType;
    use micold_core::settings::{JsonFileSettingsStore, Settings, SettingsStore};

    fn snapshot(number: u64, title: &str, labels: &[&str]) -> IssueSnapshot {
        IssueSnapshot {
            number,
            title: title.to_string(),
            labels: labels.iter().map(|l| l.to_string()).collect(),
        }
    }

    /// `origin` on github.com, and `fake` answering the lookups.
    fn on_github(f: &Fixture, fake: FakeIssueSource) -> Arc<FakeIssueSource> {
        git(
            f.repo(),
            &["remote", "add", "origin", "https://github.com/o/r.git"],
        );
        let fake = Arc::new(fake);
        let held = Arc::clone(&fake);
        f.state
            .set_issue_lookup(move |_| Some(Arc::clone(&held) as Arc<dyn IssueLookup>));
        fake
    }

    /// US3 s1, FR-005: an open `bug` issue gives the branch the form's pick gives.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn an_open_bug_issue_creates_the_forms_worktree() {
        let f = Fixture::new().await;
        let fake = on_github(
            &f,
            FakeIssueSource::new().with_lookup(Ok(snapshot(123, "Login crash", &["bug"]))),
        );
        let row = f.ok(sid(3), json!({"github_issue": 123})).await;
        assert_eq!(row["branch"], "fix/123_login-crash");
        assert_eq!(row["directory"], "fix-123_login-crash");
        assert_eq!(row["type"], "fix");
        assert_eq!(row["ticket"], "123");
        assert_eq!(fake.lookup_calls(), vec![("o/r".to_string(), 123)]);
    }

    /// 1102 FR-010: with the switch off `github_issue` is refused before any GitHub request, and
    /// calls without it are unchanged; switched back on, it works again.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_switch_off_refuses_github_issue_before_any_request() {
        let f = Fixture::new().await;
        let fake = on_github(
            &f,
            FakeIssueSource::new().with_lookup(Ok(snapshot(123, "Login crash", &["bug"]))),
        );
        f.state.set_github_issues_enabled(false).unwrap();
        let before = f.snapshot().await;
        let error = f.err(sid(3), json!({"github_issue": 123})).await;
        assert_eq!(error["category"], "invalid_input", "{error}");
        assert_eq!(
            error["message"],
            "GitHub issues are turned off in Settings (GitHub). Give a branch, or type, ticket \
             and name instead."
        );
        assert!(fake.lookup_calls().is_empty(), "no GitHub request");
        assert!(
            fake.details_calls().is_empty(),
            "no GitHub details request: {:?}",
            fake.details_calls()
        );
        f.assert_unchanged(&before).await;

        let row = f.ok(sid(3), json!({"branch": "feat/x"})).await;
        assert_eq!(row["branch"], "feat/x");
        let row = f
            .ok(sid(3), json!({"type": "fix", "ticket": "9", "name": "y"}))
            .await;
        assert_eq!(row["branch"], "fix/9_y");

        f.state.set_github_issues_enabled(true).unwrap();
        let row = f.ok(sid(3), json!({"github_issue": 123})).await;
        assert_eq!(row["branch"], "fix/123_login-crash");
        assert_eq!(fake.lookup_calls(), vec![("o/r".to_string(), 123)]);
    }

    /// US3 s2: no mapped label and no `type` is the form's "Select a type"; an explicit type works.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn unmapped_labels_need_a_type() {
        let f = Fixture::new().await;
        on_github(
            &f,
            FakeIssueSource::new()
                .with_lookup(Ok(snapshot(7, "Question", &["question"])))
                .with_lookup(Ok(snapshot(7, "Question", &["question"]))),
        );
        let before = f.snapshot().await;
        let error = f.err(sid(3), json!({"github_issue": 7})).await;
        assert_eq!(error["category"], "invalid_input", "{error}");
        assert_eq!(error["message"], NamingError::NoType.to_string());
        f.assert_unchanged(&before).await;
        let row = f
            .ok(sid(3), json!({"github_issue": 7, "type": "docs"}))
            .await;
        assert_eq!(row["branch"], "docs/7_question");
    }

    /// US3 s3, FR-007: each way the issue is unusable is refused with the form's reason.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn an_unusable_issue_is_refused_with_the_forms_reason_and_creates_nothing() {
        let cases: Vec<(IssueLookupError, &str, &str)> = vec![
            (
                IssueLookupError::NotOpenIssue { closed: true },
                "invalid_input",
                "issue #5 in o/r is closed",
            ),
            (
                IssueLookupError::NotOpenIssue { closed: false },
                "invalid_input",
                "#5 is not an open issue in o/r: it may be a pull request or not exist",
            ),
            (
                IssueLookupError::Load(IssueLoadError::NotSignedIn),
                "service_error",
                "you're not signed in to GitHub",
            ),
            (
                IssueLookupError::Load(IssueLoadError::NoAccess),
                "service_error",
                "your GitHub sign-in can't access o/r",
            ),
            (
                IssueLookupError::Load(IssueLoadError::RateLimited),
                "service_error",
                "GitHub's rate limit was reached",
            ),
            (
                IssueLookupError::Load(IssueLoadError::TimedOut),
                "service_error",
                "GitHub didn't answer within 10 seconds.",
            ),
            (
                IssueLookupError::Load(IssueLoadError::Offline),
                "service_error",
                "Couldn't reach GitHub",
            ),
        ];
        for (failure, category, text) in cases {
            let f = Fixture::new().await;
            on_github(&f, FakeIssueSource::new().with_lookup(Err(failure.clone())));
            let before = f.snapshot().await;
            let error = f
                .err(sid(3), json!({"github_issue": 5, "type": "fix"}))
                .await;
            assert_eq!(error["category"], category, "{failure:?}: {error}");
            let message = error["message"].as_str().unwrap();
            assert!(message.contains(text), "{failure:?}: {message}");
            f.assert_unchanged(&before).await;
        }
    }

    /// FR-007: no `gh` on the machine is the form's "isn't installed" text.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn no_gh_is_refused_as_the_form_does() {
        let f = Fixture::new().await;
        git(
            f.repo(),
            &["remote", "add", "origin", "https://github.com/o/r.git"],
        );
        f.state.set_issue_lookup(|_| None);
        let before = f.snapshot().await;
        let error = f.err(sid(3), json!({"github_issue": 5})).await;
        assert_eq!(error["category"], "service_error", "{error}");
        assert!(error["message"]
            .as_str()
            .unwrap()
            .contains("isn't installed"));
        f.assert_unchanged(&before).await;
    }

    /// FR-007: a project with no GitHub remote is refused with the form's text, before any lookup.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn no_github_remote_is_refused_without_a_lookup() {
        let f = Fixture::new().await;
        let fake = Arc::new(FakeIssueSource::new());
        let held = Arc::clone(&fake);
        f.state
            .set_issue_lookup(move |_| Some(Arc::clone(&held) as Arc<dyn IssueLookup>));
        git(
            f.repo(),
            &["remote", "add", "origin", "https://example.com/o/r.git"],
        );
        let before = f.snapshot().await;
        let error = f.err(sid(3), json!({"github_issue": 5})).await;
        assert_eq!(error["category"], "invalid_input", "{error}");
        assert_eq!(error["message"], "This repository has no GitHub remote.");
        assert!(fake.lookup_calls().is_empty());
        f.assert_unchanged(&before).await;
    }

    /// US3 s4, FR-006: explicit values replace the issue's.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn explicit_values_replace_the_issues() {
        let f = Fixture::new().await;
        on_github(
            &f,
            FakeIssueSource::new()
                .with_lookup(Ok(snapshot(123, "Login crash", &["bug"])))
                .with_lookup(Ok(snapshot(124, "Login crash", &["bug"]))),
        );
        let row = f
            .ok(
                sid(3),
                json!({"github_issue": 123, "type": "chore", "ticket": "ABC-1", "name": "other thing"}),
            )
            .await;
        assert_eq!(row["branch"], "chore/abc-1_other-thing");
        let row = f
            .ok(sid(3), json!({"github_issue": 124, "name": "renamed"}))
            .await;
        assert_eq!(row["branch"], "fix/124_renamed");
    }

    /// FR-005: a title over 50 characters is cut as the form cuts it.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_long_title_is_cut_as_the_form_cuts_it() {
        let title = "Login crash when the user opens the project settings dialog twice";
        let f = Fixture::new().await;
        on_github(
            &f,
            FakeIssueSource::new().with_lookup(Ok(snapshot(9, title, &["bug"]))),
        );
        let row = f.ok(sid(3), json!({"github_issue": 9})).await;
        assert_eq!(
            row["branch"],
            "fix/9_login-crash-when-the-user-opens-the-project"
        );
        assert_eq!(
            row["directory"],
            "fix-9_login-crash-when-the-user-opens-the-project"
        );
    }

    /// FR-005: the mapping is read at call time, so one saved after the daemon started applies.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_mapping_saved_after_start_applies_to_the_next_call() {
        let f = Fixture::new().await;
        on_github(
            &f,
            FakeIssueSource::new().with_lookup(Ok(snapshot(3, "Thing", &["Bug"]))),
        );
        JsonFileSettingsStore::at(f._store.path().join("settings.json"))
            .save(&Settings {
                issue_label_types: vec![LabelTypeEntry {
                    label: "bug".into(),
                    type_: ConventionalType::Chore,
                }],
                ..Settings::default()
            })
            .unwrap();
        let row = f.ok(sid(3), json!({"github_issue": 3})).await;
        assert_eq!(row["branch"], "chore/3_thing");
    }

    /// SC-005, FR-008: a call without `github_issue`, and one refused before it is needed, makes
    /// no lookup.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn calls_without_github_issue_make_no_lookup() {
        let f = Fixture::new().await;
        let fake = on_github(&f, FakeIssueSource::new());
        f.ok(sid(3), json!({"type": "fix", "name": "plain"})).await;
        f.ok(sid(3), json!({"branch": "feat-lit"})).await;
        f.err(sid(3), json!({"type": "fix", "name": "???"})).await;
        let mixed = f
            .err(sid(3), json!({"branch": "feat-y", "github_issue": 4}))
            .await;
        assert_eq!(mixed["category"], "invalid_input", "{mixed}");
        assert!(fake.lookup_calls().is_empty(), "{:?}", fake.lookup_calls());
    }

    // ---- Feature 837: the issue's relationships in the result (tool-result.md) ----

    use micold_core::github::GithubRepo;
    use micold_core::issue_details::{parse_details, IssueDetails, Kind};
    use std::time::{Duration, Instant};

    const EVERY_KIND: &[u8] =
        include_bytes!("../../micold-core/tests/fixtures/issue_details/every_kind.json");

    fn repo() -> GithubRepo {
        GithubRepo::from_remote_url("https://github.com/o/r.git").expect("a GitHub remote")
    }

    /// Issue #12 of the recorded answer with every kind, read as an issue of `o/r`, so references
    /// into the recording's own `acme/widgets` carry their repository.
    fn every_kind() -> IssueDetails {
        parse_details(EVERY_KIND, &repo()).expect("the recorded answer parses")
    }

    /// `fake` answering #12's lookup as an open `bug` issue, and its details with `details`.
    fn issue_12(details: Result<IssueDetails, IssueLoadError>) -> FakeIssueSource {
        FakeIssueSource::new()
            .with_lookup(Ok(snapshot(12, "Parser crash", &["bug"])))
            .with_details(details)
    }

    fn states(list: &Value) -> Vec<String> {
        list["items"]
            .as_array()
            .unwrap_or_else(|| panic!("no items in {list}"))
            .iter()
            .map(|item| item["state"].as_str().unwrap_or_default().to_string())
            .collect()
    }

    /// US1 s1–s4, FR-002, FR-003: the result carries every kind the issue has, as the model
    /// reports it, beside the worktree's own keys.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_github_issue_call_reports_every_relationship() {
        let f = Fixture::new().await;
        let fake = on_github(&f, issue_12(Ok(every_kind())));
        let row = f.ok(sid(3), json!({"github_issue": 12})).await;
        assert_eq!(row["branch"], "fix/12_parser-crash");
        let issue = &row["issue"];
        assert_eq!(*issue, every_kind().to_json(), "{issue}");
        // US1 s1: one open and one closed blocker.
        assert_eq!(issue["blocked"], true);
        assert_eq!(issue["blocked_by"]["open"], 1);
        assert_eq!(states(&issue["blocked_by"]), ["open", "closed"]);
        // US1 s2: blocking, parent and sub-issues with their counts.
        assert_eq!(issue["blocking"]["total"], 1);
        assert_eq!(issue["parent"]["title"], "Q4 epic");
        assert_eq!(issue["sub_issues"]["closed"], 1);
        assert_eq!(issue["sub_issues"]["total"], 2);
        // US1 s3: labels, assignees, milestone, projects with field values.
        assert_eq!(issue["labels"]["items"], json!(["bug", "parser"]));
        assert_eq!(issue["assignees"]["items"], json!(["octocat"]));
        assert_eq!(issue["milestone"], "v1.4");
        assert_eq!(issue["projects"][0]["fields"]["Status"], "In progress");
        // US1 s4: closing and linked pull requests with their states.
        assert_eq!(
            issue["closing_pull_requests"]["items"][0]["ref"],
            "acme/widgets#20"
        );
        assert_eq!(states(&issue["closing_pull_requests"]), ["draft"]);
        assert_eq!(
            issue["linked_pull_requests"]["items"][0]["ref"],
            "acme/widgets#21"
        );
        assert_eq!(states(&issue["linked_pull_requests"]), ["merged"]);
        assert_eq!(fake.details_calls(), vec![("o/r".to_string(), 12)]);
    }

    /// US1 s5: an issue with no relationships reports only its number.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn an_issue_with_nothing_reports_only_its_number() {
        let empty = parse_details(
            include_bytes!("../../micold-core/tests/fixtures/issue_details/empty.json"),
            &repo(),
        )
        .expect("the recorded answer parses");
        let f = Fixture::new().await;
        on_github(&f, issue_12(Ok(empty)));
        let row = f.ok(sid(3), json!({"github_issue": 12})).await;
        assert_eq!(row["issue"], json!({"number": 12}));
    }

    /// US1 s6, FR-010, SC-006: calls without `github_issue` read no details and carry no `issue`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn calls_without_github_issue_read_no_details() {
        let f = Fixture::new().await;
        let fake = on_github(&f, FakeIssueSource::new().with_details(Ok(every_kind())));
        let literal = f.ok(sid(3), json!({"branch": "feat-lit"})).await;
        assert!(literal.get("issue").is_none(), "{literal}");
        let derived = f
            .ok(
                sid(3),
                json!({"type": "fix", "ticket": "ABC-1", "name": "plain"}),
            )
            .await;
        assert_eq!(derived["branch"], "fix/abc-1_plain");
        assert!(derived.get("issue").is_none(), "{derived}");
        assert!(
            fake.details_calls().is_empty(),
            "{:?}",
            fake.details_calls()
        );
    }

    /// US1 s6, FR-006: explicit values with `github_issue` name as today and still carry `issue`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn explicit_values_with_github_issue_still_carry_the_issue() {
        let f = Fixture::new().await;
        on_github(&f, issue_12(Ok(every_kind())));
        let row = f
            .ok(
                sid(3),
                json!({"github_issue": 12, "type": "chore", "ticket": "ABC-1", "name": "other thing"}),
            )
            .await;
        assert_eq!(row["branch"], "chore/abc-1_other-thing");
        assert_eq!(row["issue"], every_kind().to_json());
    }

    /// FR-010: a refused `github_issue` call returns its error and no `issue`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_refused_github_issue_call_carries_no_issue() {
        let f = Fixture::new().await;
        on_github(
            &f,
            FakeIssueSource::new()
                .with_lookup(Err(IssueLookupError::NotOpenIssue { closed: true }))
                .with_details(Ok(every_kind())),
        );
        let before = f.snapshot().await;
        let error = f.err(sid(3), json!({"github_issue": 12})).await;
        assert_eq!(error["category"], "invalid_input", "{error}");
        assert!(error.get("issue").is_none(), "{error}");
        f.assert_unchanged(&before).await;

        let f = Fixture::new().await;
        on_github(&f, issue_12(Ok(every_kind())));
        occupy(&f, "fix-12_parser-crash");
        let error = f.err(sid(3), json!({"github_issue": 12})).await;
        assert!(error.get("issue").is_none(), "{error}");
    }

    /// US3 s1, FR-012: a kind that cannot be read is listed under `not_readable`; the rest stay.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_kind_that_cannot_be_read_is_listed_and_the_rest_reported() {
        let mut details = every_kind();
        details.projects = Kind::NotReadable("no read:project scope".into());
        let f = Fixture::new().await;
        on_github(&f, issue_12(Ok(details)));
        let row = f.ok(sid(3), json!({"github_issue": 12})).await;
        let issue = &row["issue"];
        assert_eq!(issue["not_readable"]["projects"], "no read:project scope");
        assert!(issue.get("projects").is_none(), "{issue}");
        for kind in [
            "blocked_by",
            "blocking",
            "parent",
            "sub_issues",
            "labels",
            "assignees",
            "milestone",
            "closing_pull_requests",
            "linked_pull_requests",
        ] {
            assert!(issue.get(kind).is_some(), "{kind} missing: {issue}");
        }
    }

    /// US3 s2, FR-012: a failed read is one `details_error` and the worktree is still created.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_failed_read_is_a_details_error_and_the_worktree_is_created() {
        let f = Fixture::new().await;
        on_github(&f, issue_12(Err(IssueLoadError::RateLimited)));
        let row = f.ok(sid(3), json!({"github_issue": 12})).await;
        assert_eq!(
            row["issue"],
            json!({"number": 12, "details_error": IssueLoadError::RateLimited.message(&repo())})
        );
        assert!(
            row["path"].as_str().is_some_and(|p| Path::new(p).is_dir()),
            "{row}"
        );
    }

    /// US3 s2, FR-011, SC-005: a read slower than the bound returns within it with the production
    /// bound's timeout sentence, and the worktree is created.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_slow_read_is_cut_at_the_bound() {
        let f = Fixture::new().await;
        on_github(
            &f,
            issue_12(Ok(every_kind())).with_details_delay(Duration::from_secs(5)),
        );
        f.state.set_details_bound(Duration::from_millis(200));
        let started = Instant::now();
        let row = f.ok(sid(3), json!({"github_issue": 12})).await;
        let took = started.elapsed();
        assert!(took < Duration::from_millis(2200), "took {took:?}");
        assert_eq!(
            row["issue"]["details_error"],
            IssueLoadError::TimedOut.message(&repo())
        );
        assert!(
            row["path"].as_str().is_some_and(|p| Path::new(p).is_dir()),
            "{row}"
        );
    }

    /// US3 s4, FR-003: an issue blocked by an open issue is created, reported blocked, not refused.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_blocked_issue_is_created_and_reported_blocked() {
        let f = Fixture::new().await;
        on_github(&f, issue_12(Ok(every_kind())));
        let before = f.snapshot().await;
        let row = f.ok(sid(3), json!({"github_issue": 12})).await;
        assert_eq!(row["issue"]["blocked"], true);
        assert_ne!(f.snapshot().await, before, "the worktree is created");
    }
}
