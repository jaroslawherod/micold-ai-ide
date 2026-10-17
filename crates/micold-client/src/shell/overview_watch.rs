//! The Overview's progress watch (feature 813, research R9): while the Overview is open, watch
//! each valid worktree's `specs/` and `.specify/` and read its task progress once on opening and
//! again once a burst of changes is over, so a card follows the files within seconds (FR-008).
//!
//! The loop is the Changes view's (`shell/changes_watch.rs`): the notify watcher and the
//! [`Debouncer`]. Only the targets and the read differ. A closed Overview watches nothing.

use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use iced::Subscription;
use micold_client::app::Message;
use micold_client::features::overview::Msg;
use micold_core::task_progress;
use micold_core::worktree::WorktreeStatus;
use notify::RecursiveMode;

use crate::shell::changes_watch::{watch_loop, Resolved};
use crate::App;

/// One watch per valid worktree while the Overview is open.
pub fn watches(app: &App) -> Vec<Subscription<Message>> {
    if !app.core.overview.open {
        return Vec::new();
    }
    app.core
        .worktree
        .worktrees
        .iter()
        .filter(|worktree| worktree.status == WorktreeStatus::Valid)
        .map(|worktree| {
            let key = Key {
                dir: worktree.dir_name.clone(),
                root: worktree.path.clone(),
            };
            Subscription::run_with(key, |key| {
                let (dir, root) = (key.dir.clone(), key.root.clone());
                let ticket = micold_core::naming::ticket(&dir);
                let resolve = move || {
                    let root = root.canonicalize().unwrap_or(root);
                    let targets = targets(&root);
                    Resolved {
                        root,
                        git_dirs: Vec::new(),
                        targets,
                    }
                };
                let read = move |resolved: &Resolved, _batch: Vec<PathBuf>| {
                    Some(Message::Overview(Msg::ProgressRead {
                        dir: dir.clone(),
                        progress: task_progress::read(&resolved.root, ticket.as_deref()),
                    }))
                };
                watch_loop(resolve, read, true)
            })
        })
        .collect()
}

/// What identifies a watch: the worktree's directory name and path.
struct Key {
    dir: String,
    root: PathBuf,
}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.dir.hash(state);
        self.root.hash(state);
    }
}

/// The directories to watch for a worktree at `root`: its specs and its spec-kit settings, each
/// recursively. Nothing else under the worktree says anything about progress.
fn targets(root: &Path) -> Vec<(PathBuf, RecursiveMode)> {
    vec![
        (root.join("specs"), RecursiveMode::Recursive),
        (root.join(".specify"), RecursiveMode::Recursive),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R9: a worktree's watch targets are its `specs` and `.specify`, recursively.
    #[test]
    fn the_watch_targets_are_specs_and_specify_recursively() {
        let root = PathBuf::from("/repo/.claude/worktrees/wt");
        assert_eq!(
            targets(&root),
            vec![
                (root.join("specs"), RecursiveMode::Recursive),
                (root.join(".specify"), RecursiveMode::Recursive),
            ]
        );
    }
}
