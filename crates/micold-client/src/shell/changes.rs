//! The Changes view's shell half (feature 482, T021, research R2): runs the list read that
//! `features::changes` asks for off the update thread, as `shell/pr_status.rs` runs its reading.
//!
//! The reducer decides when a read starts and drops an answer that is no longer current; this
//! module only resolves the entry's directory, calls git in `spawn_blocking`, and answers
//! [`Msg::ListRead`] with the read's own `seq`.

use std::path::{Path, PathBuf};

use iced::Task;
use micold_client::app::Message;
use micold_client::features::changes::Effect;
use micold_client::features::changes::Msg;
use micold_core::git::Git;
use micold_core::review::base::{ReviewScope, Toggles};
use micold_core::review::changes::ChangeList;
use micold_core::session::SessionLocation;

use crate::App;

/// Apply a Changes view message, and run the read the reducer asks for.
pub fn update(app: &mut App, msg: Msg) -> Task<Message> {
    let effect = app.core.update_changes(msg);
    run(app, effect)
}

/// Run the read the root left pending while it opened the view (V1).
pub fn run_pending(app: &mut App) -> Task<Message> {
    match app.core.take_changes_effect() {
        Some(effect) => run(app, effect),
        None => Task::none(),
    }
}

/// Start `effect`'s read, answering `Msg::ListRead` when git is done.
fn run(app: &App, effect: Effect) -> Task<Message> {
    let Effect::ReadList {
        seq,
        entry,
        toggles,
    } = effect
    else {
        return Task::none();
    };
    let Some(dir) = entry_dir(app, &entry) else {
        return Task::done(Message::Changes(Msg::ListRead {
            seq,
            result: Err("No project is open".into()),
        }));
    };
    // No local git when the daemon's filesystem is not this one (feature 027): the paths the
    // sidebar lists are the daemon's, so a local read would describe other directories.
    let Some(git) = app.caps.shared_git() else {
        return Task::done(Message::Changes(Msg::ListRead {
            seq,
            result: Err("This computer cannot read the worktree's files: the session service runs elsewhere".into()),
        }));
    };
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || read_list(&*git, &dir, &entry, toggles))
                .await
                .unwrap_or_else(|joined| Err(joined.to_string()))
        },
        move |result| Message::Changes(Msg::ListRead { seq, result }),
    )
}

/// The directory of `entry`: a listed worktree's own path (an included one lives outside the
/// project), else where a session there would run.
fn entry_dir(app: &App, entry: &SessionLocation) -> Option<PathBuf> {
    if let SessionLocation::Worktree(dir_name) = entry {
        if let Some(worktree) = app
            .core
            .worktree
            .worktrees
            .iter()
            .find(|w| &w.dir_name == dir_name)
        {
            return Some(worktree.path.clone());
        }
    }
    app.core.location_dir(entry)
}

/// Resolve the base (worktrees only) and list the changed files (R1, R7).
fn read_list(
    git: &(dyn Git + Send + Sync),
    dir: &Path,
    entry: &SessionLocation,
    toggles: Toggles,
) -> Result<ChangeList, String> {
    let scope = match entry {
        SessionLocation::Default => ReviewScope::RootUncommitted,
        SessionLocation::Worktree(_) => ReviewScope::Worktree {
            base: git.review_base(dir),
        },
    };
    git.change_list(dir, scope, toggles)
        .map_err(|error| error.to_string())
}
