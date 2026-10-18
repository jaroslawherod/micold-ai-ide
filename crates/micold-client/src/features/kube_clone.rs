//! Creating a worktree on a Kubernetes daemon (feature 686, FR-007, research K8): the pod holds no
//! copy of the project, so the create first asks the daemon to clone the project's remote into its
//! volume (`RepoClone`), then creates the worktree in that clone, which is a remote project with
//! 489's identity. Render-free: this decides what is sent and what each answer means; the shell
//! sends and the form shows.

use std::path::PathBuf;

use micold_core::daemons::DaemonId;
use micold_core::git::GitRemote;
use micold_core::kube::clone::{remote_for, CloneFailure, CloneTarget};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_core::ssh::remote_path::RemoteLocation;

/// A clone to ask a Kubernetes daemon for, ahead of a worktree create.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClonePlan {
    /// The remote's URL, as the project's config writes it.
    pub url: String,
    /// Where the clone lives in the pod (POSIX).
    pub dest: String,
    /// The clone as a project of the daemon: `micold-remote://<daemon-id><dest>`.
    pub project: PathBuf,
    /// What the form says while the clone runs.
    pub activity: String,
}

/// What an answer to the clone request means for the create.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// The clone is in place: register `project` and create the worktree in it.
    Create {
        /// The clone's identity.
        project: PathBuf,
    },
    /// The clone failed: show this and send nothing more.
    Failed(String),
}

/// Plan the clone of `project_name`'s remote onto `daemon`, or the refusal to show. Nothing is sent
/// for a refusal.
pub fn plan(
    project_name: &str,
    daemon: DaemonId,
    remotes: &[GitRemote],
) -> Result<ClonePlan, String> {
    let remote = remote_for(remotes).map_err(|why| why.message(project_name))?;
    let target = CloneTarget::for_url(&remote.url);
    let location = RemoteLocation {
        daemon,
        path: target.dest.clone(),
    };
    Ok(ClonePlan {
        url: remote.url.clone(),
        project: location.identity(),
        dest: target.dest,
        activity: format!("Cloning {project_name}"),
    })
}

impl ClonePlan {
    /// The request that makes the clone.
    pub fn request(&self) -> ClientMsg {
        ClientMsg::RepoClone {
            url: self.url.clone(),
            dest: self.dest.clone(),
        }
    }

    /// What `reply` means for this plan; `None` for a message about anything else. `daemon_name`
    /// words a failure.
    pub fn on_reply(&self, daemon_name: &str, reply: &DaemonMsg) -> Option<Step> {
        match reply {
            DaemonMsg::RepoCloned { dest } if *dest == self.dest => Some(Step::Create {
                project: self.project.clone(),
            }),
            DaemonMsg::RepoCloneFailed { dest, failure } if *dest == self.dest => {
                Some(Step::Failed(
                    CloneFailure::from_wire(failure.clone(), &self.url).message(daemon_name),
                ))
            }
            _ => None,
        }
    }
}
