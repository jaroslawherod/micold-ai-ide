//! Run groups are written before they are held or pushed, and every window gets the same list
//! (feature 483, contracts/run-group-wire.md W2, W5; FR-019, FR-020).
//!
//! The same sandbox as `run_group_create.rs`: a real git repository with a branch `base`, a
//! stand-in `claude`, and windows connected over in-memory duplexes. The runs file lives in
//! `<store>/runs`.

// unix-only: the stand-in CLI is a `#!/bin/sh` script, as in `review_send.rs`.
#![cfg(unix)]

#[path = "support/runs.rs"]
mod runs_support;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use micold_core::protocol::messages::{ClientMsg, DaemonMsg, ErrorKind};
use micold_core::session::AiCli;
use runs_support::{
    attach, branch_exists, connect, create_msg, created, next_runs_push, request, settled, window,
    Sandbox, BOUND, ENV, QUIET,
};

const CLAUDE: AiCli = AiCli::ClaudeCode;

/// The names in `<store>/runs`.
fn runs_dir_names(s: &Sandbox) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(s.store.path().join("runs"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

/// W5: the group is on disk by the time its first `RunGroupsChanged` arrives, every read of the
/// file while the runs progress is a whole JSON document, and once they settle the file is all
/// that is left in the runs directory (no temporary file).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn w5_the_runs_file_holds_the_group_before_the_first_push_and_is_written_whole() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let project = s.project();
    let path = s.runs_path();
    let mut client = window(&s.state, &project).await;

    // A reader that keeps opening the file while the runs are written and progress.
    let stop = Arc::new(AtomicBool::new(false));
    let reader = {
        let (stop, path) = (Arc::clone(&stop), path.clone());
        std::thread::spawn(move || {
            let mut reads = 0usize;
            while !stop.load(Ordering::Relaxed) {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    if let Err(err) = serde_json::from_str::<serde_json::Value>(&text) {
                        panic!("a partial runs file was observable ({err}): {text:?}");
                    }
                    reads += 1;
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            reads
        })
    };

    let (answer, pushes) = request(&mut client, 1, create_msg(1, &project, vec![CLAUDE; 2])).await;
    let id = created(&answer);
    let first = match pushes.into_iter().next() {
        Some(groups) => groups,
        None => next_runs_push(&mut client, BOUND)
            .await
            .expect("the accepted group is pushed"),
    };
    assert!(first.iter().any(|g| g.id == id), "the push holds the group");
    let on_disk = s.on_disk();
    assert!(
        on_disk.groups.iter().any(|g| g.id == id),
        "the runs file holds the group by its first push: {:?}",
        on_disk.groups
    );

    let group = settled(&mut client, id).await;
    stop.store(true, Ordering::Relaxed);
    let reads = reader.join().expect("every read saw a whole file");
    assert!(reads > 0, "the reader saw the file");
    assert_eq!(
        s.on_disk().groups,
        vec![group],
        "the file holds what was last pushed"
    );
    let file_name = path.file_name().unwrap().to_str().unwrap().to_owned();
    assert_eq!(
        runs_dir_names(&s),
        vec![file_name],
        "only the runs file is left; no temporary file"
    );
}

/// W5: when the runs file cannot be written the create is refused with `IoFailed`: no worktree,
/// no branch, no push, and a window that attaches afterwards gets no groups.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn w5_a_failing_write_refuses_the_create_and_changes_nothing() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let project = s.project();
    // A regular file where the runs directory belongs: neither root nor anyone else can write
    // under it, so the test holds whoever runs it.
    std::fs::write(s.store.path().join("runs"), "not a directory").unwrap();
    let mut client = window(&s.state, &project).await;

    let (answer, pushes) = request(&mut client, 1, create_msg(1, &project, vec![CLAUDE; 2])).await;
    match answer {
        Err((ErrorKind::IoFailed, message)) => {
            assert!(!message.is_empty(), "the refusal gives a reason")
        }
        other => panic!("the create is refused with IoFailed: {other:?}"),
    }
    assert!(pushes.is_empty(), "nothing pushed before the refusal");
    assert_eq!(
        next_runs_push(&mut client, QUIET).await,
        None,
        "nothing pushed after the refusal"
    );
    for n in 1..=2 {
        assert!(
            !s.worktree(&format!("feat-login-page-{n}")).exists(),
            "run {n} got no worktree"
        );
        assert!(
            !branch_exists(&project, &format!("feat/login-page-{n}")),
            "run {n} got no branch"
        );
    }
    assert!(s.inputs().is_empty(), "no session was started");

    let mut fresh = connect(&s.state).await;
    let frames = attach(&mut fresh, &project).await;
    let groups = frames.iter().find_map(|msg| match msg {
        DaemonMsg::RunGroupsChanged { groups, .. } => Some(groups.clone()),
        _ => None,
    });
    assert_eq!(groups, Some(vec![]), "the groups are as they were: none");
}

/// W2: two windows on the project get the same `RunGroupsChanged`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn w2_two_windows_receive_the_same_push() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let project = s.project();
    let mut a = window(&s.state, &project).await;
    let mut b = window(&s.state, &project).await;

    let (answer, pushes) = request(&mut a, 1, create_msg(1, &project, vec![CLAUDE; 2])).await;
    let id = created(&answer);
    let from_a = match pushes.into_iter().next() {
        Some(groups) => groups,
        None => next_runs_push(&mut a, BOUND)
            .await
            .expect("window a gets the push"),
    };
    let from_b = next_runs_push(&mut b, BOUND)
        .await
        .expect("window b gets the push");
    assert!(
        from_b.iter().any(|g| g.id == id),
        "b's push holds the group"
    );
    assert_eq!(from_a, from_b, "both windows get the same list");
}

/// W2: a window gets `RunGroupsChanged` once, as the frame right after `Attached`, carrying the
/// groups already there.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn w2_an_attach_gets_the_groups_once_right_after_attached() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let project = s.project();

    let mut empty = connect(&s.state).await;
    let frames = attach(&mut empty, &project).await;
    let at = frames
        .iter()
        .position(|msg| matches!(msg, DaemonMsg::Attached { .. }))
        .expect("attached");
    match frames.get(at + 1) {
        Some(DaemonMsg::RunGroupsChanged { project: p, groups }) => {
            assert_eq!(p, &project);
            assert!(groups.is_empty(), "no groups yet");
        }
        other => panic!("RunGroupsChanged follows Attached: {other:?}"),
    }
    let count = frames
        .iter()
        .filter(|msg| matches!(msg, DaemonMsg::RunGroupsChanged { .. }))
        .count();
    assert_eq!(count, 1, "one RunGroupsChanged in the attach");
    assert_eq!(
        next_runs_push(&mut empty, QUIET).await,
        None,
        "and no other while nothing changes"
    );

    let (answer, _) = request(&mut empty, 1, create_msg(1, &project, vec![CLAUDE; 2])).await;
    let id = created(&answer);
    let group = settled(&mut empty, id).await;

    let mut later = connect(&s.state).await;
    let frames = attach(&mut later, &project).await;
    let at = frames
        .iter()
        .position(|msg| matches!(msg, DaemonMsg::Attached { .. }))
        .expect("attached");
    match frames.get(at + 1) {
        Some(DaemonMsg::RunGroupsChanged { groups, .. }) => {
            assert_eq!(
                groups,
                &vec![group],
                "the attach carries the existing group"
            )
        }
        other => panic!("RunGroupsChanged follows Attached: {other:?}"),
    }
}

/// W5: forgetting the project deletes its runs file.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn w5_forgetting_the_project_removes_its_runs_file() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let project = s.project();
    let mut client = window(&s.state, &project).await;
    let (answer, _) = request(&mut client, 1, create_msg(1, &project, vec![CLAUDE; 2])).await;
    settled(&mut client, created(&answer)).await;
    assert!(s.runs_path().is_file(), "the group was written");

    let (answer, _) = request(
        &mut client,
        2,
        ClientMsg::ProjectRemove {
            req: 2,
            path: project.clone(),
        },
    )
    .await;
    assert!(answer.is_ok(), "the project is forgotten: {answer:?}");
    assert!(!s.runs_path().exists(), "its runs file is gone");
}

// --- M2: restarts, deletes, Dismiss group ---

use micold_core::protocol::messages::OperationResult;
use micold_core::runs::store::RunsFile;
use micold_core::runs::{GroupId, RunGroup, RunStatus, RunStep};
use micold_core::worktree::CreateMode;

/// The groups a freshly attached window is told about.
async fn attached_groups(
    s: &Sandbox,
    state: &Arc<micold_daemon::state::DaemonState>,
) -> Vec<RunGroup> {
    let _ = s;
    let mut client = connect(state).await;
    let frames = attach(&mut client, &s.project()).await;
    frames
        .iter()
        .find_map(|msg| match msg {
            DaemonMsg::RunGroupsChanged { groups, .. } => Some(groups.clone()),
            _ => None,
        })
        .expect("the attach carries the groups")
}

/// A settled two-run group, then the same service restarted over the same store.
async fn group_then_restart(s: &Sandbox) -> (RunGroup, Arc<micold_daemon::state::DaemonState>) {
    let project = s.project();
    let mut client = window(&s.state, &project).await;
    let (answer, _) = request(&mut client, 1, create_msg(1, &project, vec![CLAUDE; 2])).await;
    let group = settled(&mut client, created(&answer)).await;
    (group, runs_support::service(s.store.path()))
}

fn seed(s: &Sandbox, groups: Vec<RunGroup>) {
    s.files()
        .save_runs(&s.project(), &RunsFile { groups })
        .unwrap();
}

/// US2 s3, FR-008, FR-021: after a restart the groups, their runs and order are as they were, and
/// each run's session is a session the restarted service knows.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn after_a_restart_the_groups_runs_and_sessions_are_as_they_were() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let (group, restarted) = group_then_restart(&s).await;
    assert!(group.runs.iter().all(|r| r.session.is_some()));
    let groups = attached_groups(&s, &restarted).await;
    assert_eq!(groups, vec![group.clone()]);
    for run in &group.runs {
        let session = run.session.unwrap();
        assert!(
            restarted.session_cwd_and_cli(session).is_some(),
            "run {} still names a restored session",
            run.number
        );
    }
}

/// Make worktree `feat-login-page-3` the way a create that was killed after git finished leaves
/// it: folder, branch and the app-created provenance record.
async fn half_created(s: &Sandbox, client: &mut runs_support::Client) {
    let (answer, _) = request(
        client,
        50,
        ClientMsg::WorktreeCreate {
            req: 50,
            project: s.project(),
            branch: "feat/login-page-3".into(),
            dir_name: "feat-login-page-3".into(),
            mode: CreateMode::NewBranch,
        },
    )
    .await;
    assert!(answer.is_ok(), "the leftover worktree: {answer:?}");
}

fn extra_run(group: &RunGroup, number: u8, status: RunStatus) -> micold_core::runs::Run {
    let mut run = group.runs[0].clone();
    run.number = number;
    run.names.dir_name = format!("feat-login-page-{number}");
    run.names.branch = format!("feat/login-page-{number}");
    run.session = None;
    run.status = status;
    run
}

/// R10, FR-019, US2 s5: a `Creating` run whose worktree is app-created and hosts no session loads
/// as `Failed { Worktree, "interrupted" }` with its folder and branch removed; finished runs are
/// intact.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_interrupted_creating_run_is_failed_and_its_folder_and_branch_are_removed() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let project = s.project();
    let (mut group, _) = group_then_restart(&s).await;
    let mut client = window(&s.state, &project).await;
    half_created(&s, &mut client).await;
    let finished = group.runs.clone();
    group.runs.push(extra_run(&group, 3, RunStatus::Creating));
    seed(&s, vec![group.clone()]);

    let restarted = runs_support::service(s.store.path());
    let groups = attached_groups(&s, &restarted).await;
    let loaded = &groups[0];
    assert_eq!(
        loaded.runs[..2],
        finished[..],
        "the finished runs are intact"
    );
    assert_eq!(
        loaded.runs[2].status,
        RunStatus::Failed {
            step: RunStep::Worktree,
            reason: "interrupted".into()
        }
    );
    assert!(
        !s.worktree("feat-login-page-3").exists(),
        "its folder is gone"
    );
    assert!(
        !branch_exists(&project, "feat/login-page-3"),
        "its branch is gone"
    );
    assert_eq!(s.on_disk().groups, groups, "persisted as loaded");
}

/// R10: a `Creating` run whose worktree hosts a session is left alone, and the reason says so.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_interrupted_run_whose_worktree_hosts_a_session_is_left_and_says_so() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let project = s.project();
    let (mut group, _) = group_then_restart(&s).await;
    group.runs[0].status = RunStatus::Creating;
    group.runs[0].session = None;
    seed(&s, vec![group.clone()]);

    let restarted = runs_support::service(s.store.path());
    let groups = attached_groups(&s, &restarted).await;
    match &groups[0].runs[0].status {
        RunStatus::Failed {
            step: RunStep::Worktree,
            reason,
        } => {
            assert!(reason.starts_with("interrupted"), "{reason}");
            assert!(reason.contains("session"), "the reason says why: {reason}");
        }
        other => panic!("failed at its worktree: {other:?}"),
    }
    assert!(
        s.worktree("feat-login-page-1").exists(),
        "the folder is kept"
    );
    assert!(
        branch_exists(&project, "feat/login-page-1"),
        "the branch is kept"
    );
}

/// R10: an interrupted `Starting` run is `Failed { Session }` and keeps its worktree and branch.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_interrupted_starting_run_fails_at_its_session_and_keeps_its_worktree() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let project = s.project();
    let (mut group, _) = group_then_restart(&s).await;
    group.runs[1].status = RunStatus::Starting;
    group.runs[1].session = None;
    seed(&s, vec![group]);

    let restarted = runs_support::service(s.store.path());
    let groups = attached_groups(&s, &restarted).await;
    assert_eq!(
        groups[0].runs[1].status,
        RunStatus::Failed {
            step: RunStep::Session,
            reason: "interrupted".into()
        }
    );
    assert!(s.worktree("feat-login-page-2").exists());
    assert!(branch_exists(&project, "feat/login-page-2"));
}

/// W4, FR-020: dismissing removes only the grouping, pushes, and answers `Ack`; an unknown group is
/// `NotFound`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn w4_dismiss_removes_only_the_group_and_an_unknown_group_is_not_found() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let project = s.project();
    let mut client = window(&s.state, &project).await;
    let (answer, _) = request(&mut client, 1, create_msg(1, &project, vec![CLAUDE; 2])).await;
    let group = settled(&mut client, created(&answer)).await;
    let (second, _) = request(&mut client, 2, create_msg(2, &project, vec![CLAUDE; 2])).await;
    let other = settled(&mut client, created(&second)).await;

    let dismiss = |req, group| ClientMsg::RunGroupDismiss {
        req,
        project: project.clone(),
        group,
    };
    let (answer, pushes) = request(&mut client, 3, dismiss(3, group.id)).await;
    assert!(matches!(answer, Ok(OperationResult::Ack)), "{answer:?}");
    let last = match pushes.last() {
        Some(groups) => groups.clone(),
        None => next_runs_push(&mut client, BOUND).await.expect("a push"),
    };
    assert!(last.iter().all(|g| g.id != group.id), "the group is gone");
    assert!(!s.on_disk().groups.iter().any(|g| g.id == group.id));
    assert!(
        s.on_disk().groups.iter().any(|g| g.id == other.id),
        "the other group stays"
    );
    for run in &group.runs {
        assert!(s.worktree(&run.names.dir_name).exists());
        assert!(branch_exists(&project, &run.names.branch));
        assert!(s.typed(run.session.unwrap()).contains("secret-sauce-483"));
    }
    let (answer, _) = request(&mut client, 4, dismiss(4, GroupId::new())).await;
    assert!(
        matches!(answer, Err((ErrorKind::NotFound, _))),
        "{answer:?}"
    );
}

async fn delete_run(client: &mut runs_support::Client, req: u64, s: &Sandbox, dir: &str) {
    let (answer, _) = request(
        client,
        req,
        ClientMsg::WorktreeDelete {
            req,
            project: s.project(),
            dir_name: dir.into(),
            stop_sessions: true,
            delete_branch: true,
        },
    )
    .await;
    assert!(answer.is_ok(), "deleted {dir}: {answer:?}");
}

/// W5, US2 s4: deleting run 2 of 3 leaves `#1, #3` with their numbers; deleting the rest removes the
/// group.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn w5_deleting_a_runs_worktree_removes_it_from_its_group_and_an_emptied_group_goes() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let project = s.project();
    let mut client = window(&s.state, &project).await;
    let (answer, _) = request(&mut client, 1, create_msg(1, &project, vec![CLAUDE; 3])).await;
    let group = settled(&mut client, created(&answer)).await;

    delete_run(&mut client, 2, &s, "feat-login-page-2").await;
    let groups = attached_groups(&s, &s.state).await;
    let numbers: Vec<u8> = groups[0].runs.iter().map(|r| r.number).collect();
    assert_eq!(numbers, vec![1, 3]);
    assert_eq!(s.on_disk().groups, groups, "persisted");

    delete_run(&mut client, 3, &s, "feat-login-page-1").await;
    delete_run(&mut client, 4, &s, "feat-login-page-3").await;
    assert!(attached_groups(&s, &s.state).await.is_empty());
    assert!(s.on_disk().groups.is_empty());
    let _ = group;
}

/// Data-model § RunGroup: deleting the winner's worktree keeps `winner` set.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn deleting_the_winners_worktree_keeps_the_winner_set() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let project = s.project();
    let (mut group, _) = group_then_restart(&s).await;
    group.runs[1].status = RunStatus::Picked;
    group.winner = Some(2);
    seed(&s, vec![group]);
    let restarted = runs_support::service(s.store.path());
    let mut client = window(&restarted, &project).await;

    delete_run(&mut client, 2, &s, "feat-login-page-2").await;
    let groups = attached_groups(&s, &restarted).await;
    assert_eq!(groups[0].winner, Some(2));
    assert_eq!(groups[0].runs.len(), 1);
}
