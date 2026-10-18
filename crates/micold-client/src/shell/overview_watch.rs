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
    let project = app.core.workspace.active.clone().unwrap_or_default();
    app.core
        .worktree
        .worktrees
        .iter()
        .filter(|worktree| worktree.status == WorktreeStatus::Valid)
        .map(|worktree| {
            let key = Key {
                project: project.clone(),
                dir: worktree.dir_name.clone(),
                root: worktree.path.clone(),
            };
            Subscription::run_with(key, |key| {
                let (dir, root, project) = (key.dir.clone(), key.root.clone(), key.project.clone());
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
                        project: project.clone(),
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
    project: PathBuf,
    dir: String,
    root: PathBuf,
}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.project.hash(state);
        self.dir.hash(state);
        self.root.hash(state);
    }
}

/// The directories to watch for a worktree at `root`: its specs and its spec-kit settings, each
/// recursively, and the root itself non-recursively so a `specs/` or `.specify/` made after the
/// Overview opened is noticed (the watch loop then watches it). Nothing else under the worktree says
/// anything about progress.
fn targets(root: &Path) -> Vec<(PathBuf, RecursiveMode)> {
    vec![
        (root.to_path_buf(), RecursiveMode::NonRecursive),
        (root.join("specs"), RecursiveMode::Recursive),
        (root.join(".specify"), RecursiveMode::Recursive),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R9: a worktree's watch targets are its `specs` and `.specify` recursively and its root.
    #[test]
    fn the_watch_targets_are_specs_and_specify_recursively() {
        let root = PathBuf::from("/repo/.claude/worktrees/wt");
        assert_eq!(
            targets(&root),
            vec![
                (root.clone(), RecursiveMode::NonRecursive),
                (root.join("specs"), RecursiveMode::Recursive),
                (root.join(".specify"), RecursiveMode::Recursive),
            ]
        );
    }

    /// FR-008 / SC-004: a `tasks.md` written under a watched worktree reaches the Overview as a
    /// `ProgressRead` carrying the new task ratio, through the real watcher and debouncer. Waits on
    /// that condition up to a bound; a sleep would either flake or pass on a dead watcher.
    #[test]
    fn a_tasks_file_change_reaches_progress_read() {
        use iced::futures::StreamExt;
        use std::time::Duration;

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let feature = root.join("specs/001-thing");
        std::fs::create_dir_all(&feature).unwrap();
        let read = |resolved: &Resolved, _batch: Vec<PathBuf>| {
            Some(Message::Overview(Msg::ProgressRead {
                project: PathBuf::from("/project"),
                dir: "wt".to_string(),
                progress: task_progress::read(&resolved.root, Some("1")),
            }))
        };
        let resolve_root = root.clone();
        let resolve = move || Resolved {
            targets: targets(&resolve_root),
            root: resolve_root,
            git_dirs: Vec::new(),
        };
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let seen = runtime.block_on(async {
            let mut stream = Box::pin(watch_loop(resolve, read, true));
            // The initial read proves the watch is up (it is sent after the targets are watched)
            // and that no task list means no ratio yet.
            let first = tokio::time::timeout(Duration::from_secs(10), stream.next())
                .await
                .expect("the initial read arrives")
                .expect("the stream is open");
            assert!(matches!(
                first,
                Message::Overview(Msg::ProgressRead { ref progress, .. }) if progress.tasks.is_none()
            ));
            std::fs::write(feature.join("tasks.md"), "- [x] T001 a\n- [ ] T002 b\n").unwrap();
            let wait = async {
                while let Some(message) = stream.next().await {
                    if let Message::Overview(Msg::ProgressRead { dir, progress, .. }) = message {
                        if progress.tasks.is_some() {
                            return Some(dir);
                        }
                    }
                }
                None
            };
            tokio::time::timeout(Duration::from_secs(10), wait).await
        });
        assert_eq!(seen.ok().flatten().as_deref(), Some("wt"));
    }
}
