//! The spec and commit facts of the worktrees' location titles (feature 1504, plan D5, contract
//! L7): each listed worktree's feature spec heading and its branch's own latest commit subject,
//! read on a blocking thread when the worktree list or the active project changes and when the
//! Overview opens, never on a timer and never on the UI thread (FR-006, SC-005).

use std::path::PathBuf;
use std::sync::Arc;

use iced::Task;
use micold_client::app::Message;
use micold_client::features::overview::{Msg, TitleFacts};
use micold_core::git::Git;
use micold_core::location_title::{read_spec_title, title_ticket, TitleTicket};
use micold_core::worktree::WorktreeStatus;

use crate::App;

/// Read the facts of the listed worktrees `dirs` (every listed one): a new round is recorded for
/// them ([`Msg::TitleFactsRequested`]), then each valid one is read on a blocking thread, its spec
/// heading and its own-commit subject independently, within `shell::changes`'s read deadline. A
/// late, failed or missing `git` loses only the commit. Nothing at all without local git: the
/// worktrees are then the daemon's, not this machine's. No title text is logged (FR-011, L9).
pub fn read_facts(app: &mut App, dirs: &[String]) -> Task<Message> {
    let (Some(git), Some(project)) = (app.caps.shared_git(), app.core.workspace.active.clone())
    else {
        return Task::none();
    };
    app.core.update(Message::Overview(Msg::TitleFactsRequested {
        project: project.clone(),
        dirs: dirs.to_vec(),
    }));
    let round = app.core.overview.facts_round;
    let reads: Vec<Task<Message>> = app
        .core
        .worktree
        .worktrees
        .iter()
        .filter(|worktree| worktree.status == WorktreeStatus::Valid)
        .filter(|worktree| dirs.contains(&worktree.dir_name))
        .map(|worktree| {
            let spec_ticket = match title_ticket(&worktree.dir_name, worktree.branch.as_deref()) {
                Some(TitleTicket::Github(number)) => Some(number),
                _ => None,
            };
            let read = FactsRead {
                path: worktree.path.clone(),
                spec_ticket,
                has_branch: worktree.branch.is_some(),
            };
            let (project, dir) = (project.clone(), worktree.dir_name.clone());
            Task::perform(read.run(Arc::clone(&git)), move |facts| {
                Message::Overview(Msg::TitleFactsRead {
                    project,
                    dir,
                    round,
                    facts,
                })
            })
        })
        .collect();
    Task::batch(reads)
}

/// One worktree's facts read: where it is and what its title ticket and branch say.
struct FactsRead {
    path: PathBuf,
    spec_ticket: Option<u64>,
    has_branch: bool,
}

impl FactsRead {
    /// The spec heading and the own-commit subject, each on its own blocking thread and each
    /// bounded by the read deadline, so neither can hold back or lose the other.
    async fn run(self, git: Arc<dyn Git + Send + Sync>) -> TitleFacts {
        let path = self.path.clone();
        let spec_ticket = self.spec_ticket;
        let spec = bounded(move || read_spec_title(&path, spec_ticket));
        let path = self.path;
        let has_branch = self.has_branch;
        let commit = bounded(move || has_branch.then(|| git.own_commit_subject(&path)).flatten());
        let (spec, commit) = iced::futures::join!(spec, commit);
        TitleFacts { spec, commit }
    }
}

/// `read` on a blocking thread; none once [`crate::shell::changes::READ_DEADLINE`] passes or when
/// the thread fails.
async fn bounded(read: impl FnOnce() -> Option<String> + Send + 'static) -> Option<String> {
    tokio::time::timeout(
        crate::shell::changes::READ_DEADLINE,
        tokio::task::spawn_blocking(read),
    )
    .await
    .ok()
    .and_then(Result::ok)
    .flatten()
}

/// A fingerprint of the active project's worktree list for the facts (L7): by directory name and
/// branch like the issue titles' fingerprint, and also by status and path, since the facts are
/// read only for a valid worktree and from where it is: a worktree that turns valid or moves is a
/// changed list here, though not for the issue titles.
pub fn facts_fingerprint(app: &App) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    crate::shell::issues::worktrees_fingerprint(app).hash(&mut hasher);
    for worktree in &app.core.worktree.worktrees {
        worktree.status.hash(&mut hasher);
        worktree.path.hash(&mut hasher);
    }
    hasher.finish()
}

/// The directory names of the active project's listed worktrees.
pub fn listed_dirs(app: &App) -> Vec<String> {
    app.core
        .worktree
        .worktrees
        .iter()
        .map(|worktree| worktree.dir_name.clone())
        .collect()
}
