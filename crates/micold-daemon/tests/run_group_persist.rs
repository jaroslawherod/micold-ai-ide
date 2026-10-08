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
