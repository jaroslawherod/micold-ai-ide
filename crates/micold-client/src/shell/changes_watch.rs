//! The Changes view's file watch (feature 482, T086, research R9): while the view is open, watch
//! the entry's directory and its git metadata and say [`Msg::Changed`] once a burst of relevant
//! changes is over, so the list and the open diff refresh with no user action (FR-010).
//!
//! The subscription holds no decision of its own: which paths count is
//! [`watch::relevant_paths`], when a burst is over is [`watch::Debouncer`], what git ignores is
//! [`Git::ignored`], and what a change means is the reducer's. `shell/subscriptions.rs` asks for it
//! only while the view is open, so a closed view watches nothing.

use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use iced::futures::channel::mpsc;
use iced::futures::{SinkExt, Stream};
use iced::Subscription;
use micold_client::app::Message;
use micold_client::features::changes::Msg;
use micold_core::git::Git;
use micold_core::review::watch::{self, Debouncer};
use micold_core::session::SessionLocation;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::App;

/// The watch of `entry`, or `None` when its files cannot be read here (no project, or the session
/// service runs on another computer, as for the reads in `shell/changes.rs`).
pub fn watch(app: &App, entry: &SessionLocation) -> Option<Subscription<Message>> {
    let root = crate::shell::changes::entry_dir(app, entry)?;
    let git = app.caps.shared_git()?;
    Some(Subscription::run_with(Key { root, git }, |key| {
        run(key.root.clone(), key.git.clone())
    }))
}

/// What identifies a watch: the entry's directory. Another entry (or project) is another watch;
/// the same one is kept across updates. The git handle rides along and does not count.
struct Key {
    root: PathBuf,
    git: Arc<dyn Git + Send + Sync>,
}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.root.hash(state);
    }
}

/// The directories to watch and how: the entry recursively, and each git dir outside it (a
/// worktree's own git dir and the common one live in the main checkout) by itself plus its `refs/`
/// (R9). A git dir inside the entry (the Default entry's `.git`) is already under its watch.
fn targets(root: &Path, git_dirs: &[PathBuf]) -> Vec<(PathBuf, RecursiveMode)> {
    let mut targets = vec![(root.to_path_buf(), RecursiveMode::Recursive)];
    for dir in git_dirs.iter().filter(|dir| !dir.starts_with(root)) {
        targets.push((dir.clone(), RecursiveMode::NonRecursive));
        targets.push((dir.join("refs"), RecursiveMode::Recursive));
    }
    targets
}

/// Watch `root` until the subscription is dropped (the view closed or moved to another entry).
fn run(root: PathBuf, git: Arc<dyn Git + Send + Sync>) -> impl Stream<Item = Message> {
    iced::stream::channel(1, |mut output: mpsc::Sender<Message>| async move {
        // Canonical paths, as the watcher reports them (`/private/var` on macOS).
        let resolve = {
            let git = git.clone();
            move || {
                let root = root.canonicalize().unwrap_or(root);
                let dirs: Vec<PathBuf> = git
                    .git_dirs(&root)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|dir| dir.canonicalize().unwrap_or(dir))
                    .collect();
                (root, dirs)
            }
        };
        let Ok((root, git_dirs)) = tokio::task::spawn_blocking(resolve).await else {
            return;
        };
        let (events, mut changed) = tokio::sync::mpsc::unbounded_channel::<Vec<PathBuf>>();
        let watcher = RecommendedWatcher::new(
            move |event: notify::Result<Event>| {
                if let Ok(event) = event {
                    let _ = events.send(event.paths);
                }
            },
            Config::default(),
        );
        let Ok(mut watcher) = watcher else {
            return;
        };
        for (dir, mode) in targets(&root, &git_dirs) {
            // A git dir with no `refs/` of its own (a worktree's) has nothing there to watch.
            let _ = watcher.watch(&dir, mode);
        }
        let mut debouncer = Debouncer::default();
        loop {
            let next = match debouncer.deadline() {
                Some(deadline) => {
                    let deadline = tokio::time::Instant::from_std(deadline);
                    tokio::time::timeout_at(deadline, changed.recv()).await
                }
                None => Ok(changed.recv().await),
            };
            match next {
                Ok(Some(paths)) => {
                    debouncer.push(watch::relevant_paths(&root, &git_dirs, paths), Instant::now())
                }
                // The watcher is gone: nothing more will come.
                Ok(None) => return,
                Err(_quiet) => {}
            }
            let Some(batch) = debouncer.ready(Instant::now()) else {
                continue;
            };
            let check = {
                let (git, root, git_dirs) = (git.clone(), root.clone(), git_dirs.clone());
                move || counts(&*git, &root, &git_dirs, batch)
            };
            // A failed check counts the change, as in `counts`.
            let counts = tokio::task::spawn_blocking(check).await.unwrap_or(true);
            if counts && output.send(Message::Changes(Msg::Changed)).await.is_err() {
                return;
            }
        }
    })
}

/// Whether `batch` holds a change the view shows: a git state file, or a worktree file git does
/// not ignore. A failed ignore check counts the change (a needless read beats a missed one).
fn counts(
    git: &(dyn Git + Send + Sync),
    root: &Path,
    git_dirs: &[PathBuf],
    batch: Vec<PathBuf>,
) -> bool {
    let (state, files): (Vec<PathBuf>, Vec<PathBuf>) = batch
        .into_iter()
        .partition(|path| git_dirs.iter().any(|dir| path.starts_with(dir)));
    if !state.is_empty() {
        return true;
    }
    match git.ignored(root, &files) {
        Ok(ignored) => files.iter().any(|file| !ignored.contains(file)),
        Err(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_git_dir_inside_the_entry_is_covered_by_the_entry_s_watch() {
        let root = PathBuf::from("/repo");
        let targets = targets(&root, &[root.join(".git")]);
        assert_eq!(targets, vec![(root, RecursiveMode::Recursive)]);
    }

    #[test]
    fn a_worktree_watches_its_own_git_dir_and_the_common_refs() {
        let root = PathBuf::from("/repo/.claude/worktrees/wt");
        let own = PathBuf::from("/repo/.git/worktrees/wt");
        let common = PathBuf::from("/repo/.git");
        let targets = targets(&root, &[own.clone(), common.clone()]);
        assert_eq!(
            targets,
            vec![
                (root, RecursiveMode::Recursive),
                (own.clone(), RecursiveMode::NonRecursive),
                (own.join("refs"), RecursiveMode::Recursive),
                (common.clone(), RecursiveMode::NonRecursive),
                (common.join("refs"), RecursiveMode::Recursive),
            ]
        );
    }
}
