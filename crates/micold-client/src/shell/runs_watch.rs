//! Compare's file watch (feature 483, T046, research R11): while the Compare view is open, watch
//! each run's worktree and say [`Msg::RunChanged`] once a burst of relevant changes is over, so
//! that run's counts refresh within 2 seconds with no user action (FR-010).
//!
//! The decisions are the Changes view's watch (`shell/changes_watch.rs`): which paths count is
//! `watch::relevant_paths`, when a burst is over is `watch::Debouncer`, what git ignores is
//! `Git::ignored`. `shell/subscriptions.rs` asks for these only while Compare is open.

use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;

use iced::Subscription;
use micold_client::app::Message;
use micold_client::features::runs::Msg;
use micold_core::git::Git;
use micold_core::session::SessionLocation;

use crate::App;

/// One watch per run of the open Compare view that has a worktree readable here.
pub fn watches(app: &App) -> Vec<Subscription<Message>> {
    let state = &app.core.runs;
    let Some(view) = &state.compare else {
        return Vec::new();
    };
    let Some(group) = state.groups.iter().find(|g| g.id == view.group) else {
        return Vec::new();
    };
    let Some(git) = app.caps.shared_git() else {
        return Vec::new();
    };
    group
        .runs
        .iter()
        .filter(|run| run.status.has_worktree())
        .filter_map(|run| {
            let entry = SessionLocation::Worktree(run.names.dir_name.clone());
            let root = crate::shell::changes::entry_dir(app, &entry)?;
            let key = Key {
                root,
                run: run.number,
                git: git.clone(),
            };
            Some(Subscription::run_with(key, |key| {
                crate::shell::changes_watch::run(
                    key.root.clone(),
                    key.git.clone(),
                    Message::Runs(Msg::RunChanged { run: key.run }),
                )
            }))
        })
        .collect()
}

/// What identifies a watch: the run's directory and number. The git handle rides along and does
/// not count.
struct Key {
    root: PathBuf,
    run: u8,
    git: Arc<dyn Git + Send + Sync>,
}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.root.hash(state);
        self.run.hash(state);
    }
}
