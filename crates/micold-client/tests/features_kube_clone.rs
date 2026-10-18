//! Creating a worktree on a Kubernetes daemon in isolation (feature 686, M7: FR-007, A2): the
//! feature constructs only its own types and the core's, over a fake daemon.

use micold_client::features::kube_clone::{plan, Step};
use micold_core::daemons::DaemonId;
use micold_core::git::GitRemote;
use micold_core::protocol::messages::{ClientMsg, CloneFailureWire, DaemonMsg};
use micold_core::ssh::remote_path::display_path;

fn remote(name: &str, url: &str) -> GitRemote {
    GitRemote {
        name: name.into(),
        url: url.into(),
    }
}

/// The fake daemon: answers a clone request, records the order of what it was sent.
fn fake_daemon(sent: &mut Vec<&'static str>, msg: &ClientMsg) -> Option<DaemonMsg> {
    match msg {
        ClientMsg::RepoClone { dest, .. } => {
            sent.push("RepoClone");
            Some(DaemonMsg::RepoCloned { dest: dest.clone() })
        }
        _ => None,
    }
}

#[test]
fn a_project_with_no_remote_is_refused_with_the_reason_and_sends_nothing() {
    let why = plan("app", DaemonId(2), &[]).unwrap_err();
    assert_eq!(
        why,
        "app has no git remote; a Kubernetes daemon clones from it"
    );
    let why = plan("app", DaemonId(2), &[remote("a", "1"), remote("b", "2")]).unwrap_err();
    assert_eq!(why, "app has several remotes and none is origin: a, b");
}

#[test]
fn a_project_with_a_remote_clones_first_then_creates_in_the_clone_as_a_remote_project() {
    let plan = plan(
        "app",
        DaemonId(2),
        &[remote("origin", "git@github.com:acme/app.git")],
    )
    .unwrap();
    assert_eq!(plan.activity, "Cloning app");
    let mut sent = Vec::new();
    let request = plan.request();
    let reply = fake_daemon(&mut sent, &request).unwrap();
    let Some(Step::Create { project }) = plan.on_reply("Dev", &reply) else {
        panic!("the clone did not lead to a create");
    };
    assert_eq!(sent, vec!["RepoClone"]);
    let text = project.to_string_lossy().into_owned();
    assert!(
        text.starts_with("micold-remote://") && text.contains("/data/repos/app-"),
        "{text}"
    );
    assert!(display_path(&project).starts_with("/data/repos/app-"));
}

#[test]
fn a_failed_clone_ends_the_create_with_the_contract_text() {
    let plan = plan("app", DaemonId(2), &[remote("origin", "https://h/app.git")]).unwrap();
    let reply = DaemonMsg::RepoCloneFailed {
        dest: plan.dest.clone(),
        failure: CloneFailureWire::AuthRequired,
    };
    assert_eq!(
        plan.on_reply("Dev", &reply),
        Some(Step::Failed(
            "cloning https://h/app.git needs a credential: allow Git credentials or an SSH key for Dev"
                .into()
        ))
    );
    let other = DaemonMsg::RepoCloned {
        dest: "/data/repos/else".into(),
    };
    assert_eq!(plan.on_reply("Dev", &other), None);
}
