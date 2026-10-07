//! The Changes view's shell half (feature 482, T021, T036, research R2): runs the list and diff
//! reads that `features::changes` asks for off the update thread, as `shell/pr_status.rs` runs its
//! reading, so a large diff never blocks input or redraws (FR-009).
//!
//! The reducer decides when a read starts and drops an answer that is no longer current; this
//! module only resolves the entry's directory, calls git in `spawn_blocking`, and answers
//! [`Msg::ListRead`] or [`Msg::DiffRead`] with the read's own `seq`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

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

/// Why no read can run here: the error each read answers with.
const NO_PROJECT: &str = "No project is open";
const NOT_LOCAL: &str =
    "This computer cannot read the worktree's files: the session service runs elsewhere";

/// Start `effect`'s read, answering `Msg::ListRead` or `Msg::DiffRead` when git is done.
fn run(app: &App, effect: Effect) -> Task<Message> {
    match effect {
        Effect::None => Task::none(),
        Effect::ReadList {
            seq,
            entry,
            toggles,
        } => {
            let answer = move |result| Message::Changes(Msg::ListRead { seq, result });
            let (dir, git) = match local(app, &entry) {
                Ok(found) => found,
                Err(why) => return Task::done(answer(Err(why.into()))),
            };
            off_thread(move || read_list(&*git, &dir, &entry, toggles), answer)
        }
        Effect::ReadDiff {
            seq,
            entry,
            scope,
            toggles,
            path,
            from,
            force_large,
        } => {
            let answer = move |result| Message::Changes(Msg::DiffRead { seq, result });
            let (dir, git) = match local(app, &entry) {
                Ok(found) => found,
                Err(why) => return Task::done(answer(Err(why.into()))),
            };
            off_thread(
                move || {
                    git.file_diff(&dir, &scope, toggles, &path, from.as_ref(), force_large)
                        .map_err(|error| error.to_string())
                },
                answer,
            )
        }
    }
}

/// The entry's directory and this computer's git, or why a read cannot run here.
fn local(
    app: &App,
    entry: &SessionLocation,
) -> Result<(PathBuf, Arc<dyn Git + Send + Sync>), &'static str> {
    let dir = entry_dir(app, entry).ok_or(NO_PROJECT)?;
    // No local git when the daemon's filesystem is not this one (feature 027): the paths the
    // sidebar lists are the daemon's, so a local read would describe other directories.
    let git = app.caps.shared_git().ok_or(NOT_LOCAL)?;
    Ok((dir, git))
}

/// Run `read` in `spawn_blocking` and answer with `answer`.
fn off_thread<T: Send + 'static>(
    read: impl FnOnce() -> Result<T, String> + Send + 'static,
    answer: impl Fn(Result<T, String>) -> Message + Send + 'static,
) -> Task<Message> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(read)
                .await
                .unwrap_or_else(|joined| Err(joined.to_string()))
        },
        answer,
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
