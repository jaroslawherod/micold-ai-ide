//! Starting a run group (feature 483, contracts/run-group-wire.md W1, W2; US1 scenarios 1–4;
//! FR-004, FR-005, FR-006, FR-021; SC-002).
//!
//! A real git repository on `main` with a branch `base` one commit ahead, a stand-in `claude` that
//! records what is typed into it in `<bin>/input.<session id>`, no `copilot`, and a window
//! connected over an in-memory duplex.

// unix-only: the stand-in CLI is a `#!/bin/sh` script, as in `review_send.rs`.
#![cfg(unix)]

#[path = "support/runs.rs"]
mod runs_support;

use std::time::Duration;

use micold_core::naming::WorktreeNaming;
use micold_core::protocol::messages::{ClientMsg, ErrorKind};
use micold_core::runs::{GroupId, RunStatus, RunStep};
use micold_core::session::AiCli;
use runs_support::{
    branch_exists, create_msg, created, git, next_runs_push, request, settled, submitted, window,
    Sandbox, ENV, PROMPT, QUIET,
};

const CLAUDE: AiCli = AiCli::ClaudeCode;

/// Every W1 refusal creates no group, worktree or branch, writes no runs file and pushes nothing.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn w1_each_refusal_creates_nothing_and_pushes_nothing() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let project = s.project();
    let mut client = window(&s.state, &project).await;
    let elsewhere = tempfile::tempdir().unwrap();

    let blank = |name: &str, ticket: Option<&str>, typed: bool| WorktreeNaming {
        type_: typed.then_some(micold_core::naming::ConventionalType::Feat),
        ticket: ticket.map(str::to_owned),
        name: name.into(),
    };
    let cases: Vec<(&str, ClientMsg, ErrorKind, &str)> = vec![
        (
            "an unknown project",
            create_msg(1, elsewhere.path(), vec![CLAUDE, CLAUDE]),
            ErrorKind::InvalidInput,
            "",
        ),
        (
            "one run",
            create_msg(2, &project, vec![CLAUDE]),
            ErrorKind::InvalidInput,
            "2",
        ),
        (
            "nine runs",
            create_msg(3, &project, vec![CLAUDE; 9]),
            ErrorKind::InvalidInput,
            "8",
        ),
        (
            "no type",
            ClientMsg::RunGroupCreate {
                req: 4,
                project: project.clone(),
                naming: blank("login", None, false),
                prompt: PROMPT.into(),
                base_branch: "base".into(),
                providers: vec![CLAUDE, CLAUDE],
            },
            ErrorKind::InvalidInput,
            "Select a type",
        ),
        (
            "an unslugifiable name",
            ClientMsg::RunGroupCreate {
                req: 5,
                project: project.clone(),
                naming: blank("!!!", None, true),
                prompt: PROMPT.into(),
                base_branch: "base".into(),
                providers: vec![CLAUDE, CLAUDE],
            },
            ErrorKind::InvalidInput,
            "Enter a name",
        ),
        (
            "a blank prompt",
            ClientMsg::RunGroupCreate {
                req: 6,
                project: project.clone(),
                naming: runs_support::login_page(),
                prompt: " \n\t ".into(),
                base_branch: "base".into(),
                providers: vec![CLAUDE, CLAUDE],
            },
            ErrorKind::InvalidInput,
            "prompt",
        ),
        (
            "a missing base branch",
            ClientMsg::RunGroupCreate {
                req: 7,
                project: project.clone(),
                naming: runs_support::login_page(),
                prompt: PROMPT.into(),
                base_branch: "nope".into(),
                providers: vec![CLAUDE, CLAUDE],
            },
            ErrorKind::NotFound,
            "nope",
        ),
    ];
    let branches_before = git(&project, &["branch", "--list"]);
    for (what, msg, kind, names) in cases {
        let req = match &msg {
            ClientMsg::RunGroupCreate { req, .. } => *req,
            _ => unreachable!(),
        };
        let (answer, pushes) = request(&mut client, req, msg).await;
        let (got, message) = answer.expect_err(what);
        assert_eq!(got, kind, "{what} is {kind:?}: {message}");
        assert!(
            message.contains(names),
            "the message for {what} names {names:?}: {message}"
        );
        assert!(pushes.is_empty(), "{what} pushes nothing");
    }
    assert!(
        next_runs_push(&mut client, QUIET).await.is_none(),
        "no RunGroupsChanged follows a refusal"
    );
    assert!(!s.runs_path().exists(), "no runs file is written");
    assert_eq!(
        git(&project, &["branch", "--list"]),
        branches_before,
        "no branch is created"
    );
    assert!(
        !project.join(".claude/worktrees").exists()
            || std::fs::read_dir(project.join(".claude/worktrees"))
                .unwrap()
                .next()
                .is_none(),
        "no worktree folder is created"
    );
}

/// A project the catalog lists but that is not a git repository is refused as `InvalidInput`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn w1_a_project_that_is_not_a_repository_is_refused() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    std::fs::remove_dir_all(s.project().join(".git")).unwrap();
    let mut client = runs_support::connect(&s.state).await;
    let (answer, pushes) = request(
        &mut client,
        1,
        create_msg(1, &s.project(), vec![CLAUDE, CLAUDE]),
    )
    .await;
    let (kind, message) = answer.expect_err("not a repository");
    assert_eq!(kind, ErrorKind::InvalidInput, "{message}");
    assert!(pushes.is_empty());
    assert!(!s.runs_path().exists());
}

/// US1 s1, s2: three worktrees on numbered branches from the base branch's tip, one session each
/// in its own worktree, each prompted exactly once and nothing typed into another run.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn us1_s1_s2_three_runs_each_get_a_worktree_a_session_and_the_prompt_once() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let project = s.project();
    let base_tip = git(&project, &["rev-parse", "base"]);
    let mut client = window(&s.state, &project).await;

    let (answer, pushes) = request(&mut client, 1, create_msg(1, &project, vec![CLAUDE; 3])).await;
    let id = created(&answer);
    let first = match pushes.first() {
        Some(groups) => groups.clone(),
        None => next_runs_push(&mut client, runs_support::BOUND)
            .await
            .expect("the accepted group is pushed"),
    };
    let group = first
        .iter()
        .find(|g| g.id == id)
        .expect("the group is pushed");
    assert_eq!(group.base_branch, "base");
    assert_eq!(group.base_commit, base_tip, "the base tip at create time");
    assert!(
        group.runs.iter().all(|r| r.status == RunStatus::Creating),
        "the first push has every run Creating: {:?}",
        group.runs
    );

    let group = settled(&mut client, id).await;
    assert_eq!(group.runs.len(), 3);
    let mut sessions = Vec::new();
    for (i, run) in group.runs.iter().enumerate() {
        let n = i + 1;
        assert_eq!(usize::from(run.number), n);
        assert_eq!(run.status, RunStatus::Prompted, "run {n}");
        assert_eq!(run.names.branch, format!("feat/login-page-{n}"));
        assert_eq!(run.names.dir_name, format!("feat-login-page-{n}"));
        assert!(
            s.worktree(&run.names.dir_name).is_dir(),
            "run {n}'s worktree"
        );
        assert_eq!(
            git(&project, &["rev-parse", &run.names.branch]),
            base_tip,
            "run {n}'s branch starts at the base branch's tip"
        );
        let session = run.session.expect("the run records its session");
        let (cwd, cli) = s
            .state
            .session_cwd_and_cli(session)
            .expect("the session is kept");
        assert_eq!(cli, run.provider, "run {n}'s session is of its provider");
        assert_eq!(
            cwd.canonicalize().unwrap(),
            s.worktree(&run.names.dir_name).canonicalize().unwrap(),
            "run {n}'s session runs in its own worktree"
        );
        assert_eq!(
            s.typed_holds(session, PROMPT).await,
            submitted(PROMPT),
            "run {n} received the prompt once, as its first input"
        );
        sessions.push(session);
    }
    tokio::time::sleep(QUIET).await;
    let inputs = s.inputs();
    assert_eq!(
        inputs.len(),
        3,
        "exactly the three runs' sessions read input"
    );
    for (session, typed) in inputs {
        assert!(sessions.contains(&session));
        assert_eq!(
            typed,
            submitted(PROMPT),
            "nothing more was typed into a run"
        );
    }
}

/// US1 s3, FR-005, FR-006: run 2's branch is taken, so run 2 fails at its worktree with nothing
/// of its own left behind, and runs 1 and 3 are prompted.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn us1_s3_a_run_whose_worktree_cannot_be_created_fails_alone() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let project = s.project();
    git(&project, &["branch", "feat/login-page-2", "main"]);
    let taken_tip = git(&project, &["rev-parse", "feat/login-page-2"]);
    let mut client = window(&s.state, &project).await;

    let (answer, _) = request(&mut client, 1, create_msg(1, &project, vec![CLAUDE; 3])).await;
    let group = settled(&mut client, created(&answer)).await;
    let statuses: Vec<_> = group.runs.iter().map(|r| r.status.clone()).collect();
    match &statuses[1] {
        RunStatus::Failed {
            step: RunStep::Worktree,
            reason,
        } => assert!(!reason.is_empty(), "the reason is given"),
        other => panic!("run 2 failed at its worktree: {other:?}"),
    }
    assert_eq!(group.runs[1].session, None, "run 2 has no session");
    assert!(
        !s.worktree("feat-login-page-2").exists(),
        "run 2 left no folder"
    );
    assert_eq!(
        git(&project, &["rev-parse", "feat/login-page-2"]),
        taken_tip,
        "the existing branch is untouched"
    );
    for n in [0, 2] {
        assert_eq!(
            statuses[n],
            RunStatus::Prompted,
            "run {} is prompted",
            n + 1
        );
        let session = group.runs[n].session.expect("a session");
        s.typed_holds(session, PROMPT).await;
    }
}

/// FR-005: a run whose provider is not available fails at its session, alone.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fr005_a_run_whose_provider_is_unavailable_fails_at_its_session_alone() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let project = s.project();
    let mut client = window(&s.state, &project).await;

    let (answer, _) = request(
        &mut client,
        1,
        create_msg(1, &project, vec![CLAUDE, AiCli::Copilot, CLAUDE]),
    )
    .await;
    let group = settled(&mut client, created(&answer)).await;
    match &group.runs[1].status {
        RunStatus::Failed {
            step: RunStep::Session,
            reason,
        } => assert!(!reason.is_empty()),
        other => panic!("run 2 failed at its session: {other:?}"),
    }
    assert!(
        s.worktree("feat-login-page-2").is_dir(),
        "run 2 keeps its worktree"
    );
    assert!(branch_exists(&project, "feat/login-page-2"));
    for n in [0, 2] {
        assert_eq!(group.runs[n].status, RunStatus::Prompted, "run {}", n + 1);
    }
}

/// US1 s4: a CLI that is not ready within the bound gets nothing typed; the runs keep their
/// worktree and session and are `PromptNotDelivered` with the reason.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn us1_s4_a_cli_never_ready_leaves_the_prompt_undelivered_and_keeps_the_run() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let project = s.project();
    s.state.set_first_prompt_bound(Duration::from_secs(1));
    s.slow(3.0);
    let mut client = window(&s.state, &project).await;

    let (answer, _) = request(&mut client, 1, create_msg(1, &project, vec![CLAUDE; 2])).await;
    let group = settled(&mut client, created(&answer)).await;
    for run in &group.runs {
        match &run.status {
            RunStatus::PromptNotDelivered { reason } => {
                assert!(reason.contains("ready"), "the reason says why: {reason}")
            }
            other => panic!("run {} is not delivered: {other:?}", run.number),
        }
        assert!(
            run.session.is_some(),
            "run {} keeps its session",
            run.number
        );
        assert!(s.worktree(&run.names.dir_name).is_dir());
    }
    tokio::time::sleep(Duration::from_secs(3) + QUIET).await;
    assert!(
        s.inputs().iter().all(|(_, typed)| !typed.contains(PROMPT)),
        "a CLI ready after the bound is typed nothing: {:?}",
        s.inputs()
    );
}

/// W2 (482 W12): the prompt text appears in no log line.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn w2_the_prompt_is_never_logged() {
    let _guard = ENV.lock().await;
    let log = runs_support_log::log();
    let s = Sandbox::new();
    let project = s.project();
    git(&project, &["branch", "feat/login-page-2", "main"]);
    let mut client = window(&s.state, &project).await;
    let (answer, _) = request(
        &mut client,
        1,
        create_msg(1, &project, vec![CLAUDE, CLAUDE, AiCli::Copilot]),
    )
    .await;
    let group = settled(&mut client, created(&answer)).await;
    s.typed_holds(group.runs[0].session.unwrap(), PROMPT).await;
    let text = String::from_utf8_lossy(&log.lock().unwrap()).into_owned();
    assert!(!text.is_empty(), "the service logged something");
    assert!(
        !text.contains("secret-sauce-483"),
        "the prompt is in no log line"
    );
}

/// Pick and dismiss are not served in this milestone (T035 and T058 retire this).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pick_and_dismiss_are_refused_for_now() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let project = s.project();
    let mut client = window(&s.state, &project).await;
    let group = GroupId::new();
    for msg in [
        ClientMsg::RunGroupPick {
            req: 1,
            project: project.clone(),
            group,
            run: 1,
        },
        ClientMsg::RunGroupDismiss {
            req: 1,
            project: project.clone(),
            group,
        },
    ] {
        let (answer, _) = request(&mut client, 1, msg).await;
        let (kind, _) = answer.expect_err("not served yet");
        assert_eq!(kind, ErrorKind::Refused);
    }
}

#[path = "support/mcp.rs"]
#[allow(unused_imports)]
mod runs_support_log;

/// Review A M1a: every run starts at the base commit recorded at create time, even when a tag
/// shares the base branch's name (git resolves a bare `base` to the tag first).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn every_run_starts_at_the_recorded_base_commit_even_under_a_same_named_tag() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let project = s.project();
    git(&project, &["tag", "base", "main"]);
    let base_tip = git(&project, &["rev-parse", "refs/heads/base"]);
    let mut client = window(&s.state, &project).await;

    let (answer, _) = request(&mut client, 1, create_msg(1, &project, vec![CLAUDE; 2])).await;
    let group = settled(&mut client, created(&answer)).await;
    assert_eq!(group.base_commit, base_tip);
    for run in &group.runs {
        assert_eq!(run.status, RunStatus::Prompted, "run {}", run.number);
        assert_eq!(
            git(
                &project,
                &["rev-parse", &format!("refs/heads/{}", run.names.branch)]
            ),
            base_tip,
            "run {} starts at the base branch's tip, not the tag",
            run.number
        );
    }
}

/// Review A M1a: a run whose CLI cannot be started fails at its session; it is not
/// `PromptNotDelivered`, which would make a run whose agent never ran pickable.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_run_whose_session_never_starts_fails_at_its_session() {
    use std::os::unix::fs::PermissionsExt;
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let project = s.project();
    // On PATH, so the provider is offered, but not executable, so its process cannot start.
    std::fs::set_permissions(
        s.bin.path().join("claude"),
        std::fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    let mut client = window(&s.state, &project).await;

    let (answer, _) = request(&mut client, 1, create_msg(1, &project, vec![CLAUDE; 2])).await;
    let group = settled(&mut client, created(&answer)).await;
    for run in &group.runs {
        match &run.status {
            RunStatus::Failed {
                step: RunStep::Session,
                reason,
            } => assert!(!reason.is_empty(), "run {} gives a reason", run.number),
            other => panic!("run {} failed at its session: {other:?}", run.number),
        }
    }
}
