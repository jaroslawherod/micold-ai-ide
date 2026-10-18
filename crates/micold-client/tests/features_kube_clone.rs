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

mod sequencing {
    use super::*;
    use micold_client::features::kube_clone::{Output, Pending};
    use micold_core::naming::DerivedNames;
    use micold_core::worktree::CreateMode;

    fn names() -> DerivedNames {
        DerivedNames {
            branch: "feat/x".into(),
            dir_name: "feat-x".into(),
        }
    }

    fn begin(remotes: &[GitRemote]) -> Result<Pending, String> {
        Pending::begin(
            "app",
            DaemonId(2),
            "Dev",
            remotes,
            |_| false,
            names(),
            CreateMode::NewBranch,
            None,
        )
    }

    fn pending() -> Pending {
        begin(&[remote("origin", "git@github.com:acme/app.git")]).unwrap()
    }

    fn cloned(p: &Pending) -> DaemonMsg {
        DaemonMsg::RepoCloned {
            dest: p.plan.dest.clone(),
        }
    }

    #[test]
    fn the_sequence_is_clone_then_project_then_worktree() {
        let p = pending();
        let mut order = vec![match p.plan.request() {
            ClientMsg::RepoClone { .. } => "RepoClone",
            _ => "other",
        }];
        let out = p.on_reply(true, &cloned(&p)).unwrap();
        for o in &out {
            order.push(match o {
                Output::ProjectAdd { .. } => "ProjectAdd",
                Output::WorktreeCreate { .. } => "WorktreeCreate",
                Output::Failed(_) => "Failed",
            });
        }
        assert_eq!(order, ["RepoClone", "ProjectAdd", "WorktreeCreate"]);
        let Output::WorktreeCreate {
            project,
            branch,
            dir_name,
            ..
        } = &out[1]
        else {
            panic!()
        };
        assert_eq!(
            &out[0],
            &Output::ProjectAdd {
                path: project.clone()
            }
        );
        assert_eq!((branch.as_str(), dir_name.as_str()), ("feat/x", "feat-x"));
    }

    #[test]
    fn a_refusal_sends_nothing() {
        assert!(begin(&[]).is_err());
        let busy = Pending::begin(
            "app",
            DaemonId(2),
            "Dev",
            &[remote("origin", "git@github.com:acme/app.git")],
            |_| true,
            names(),
            CreateMode::NewBranch,
            None,
        );
        assert_eq!(
            busy.unwrap_err(),
            "Dev is already cloning git@github.com:acme/app.git."
        );
    }

    #[test]
    fn a_failed_clone_creates_nothing() {
        let p = pending();
        let reply = DaemonMsg::RepoCloneFailed {
            dest: p.plan.dest.clone(),
            failure: CloneFailureWire::AuthRequired,
        };
        let out = p.on_reply(true, &reply).unwrap();
        assert!(matches!(out.as_slice(), [Output::Failed(_)]), "{out:?}");
    }

    #[test]
    fn a_daemon_gone_after_the_clone_fails_the_create() {
        let p = pending();
        assert_eq!(
            p.on_reply(false, &cloned(&p)).unwrap(),
            vec![Output::Failed("Dev disconnected after cloning.".into())]
        );
    }

    #[test]
    fn a_disconnect_mid_clone_says_so() {
        assert_eq!(
            pending().on_disconnect(),
            "Dev disconnected while cloning git@github.com:acme/app.git."
        );
    }

    #[test]
    fn an_answer_about_another_clone_is_ignored() {
        let other = DaemonMsg::RepoCloned {
            dest: "/data/repos/else".into(),
        };
        assert_eq!(pending().on_reply(true, &other), None);
    }
}
