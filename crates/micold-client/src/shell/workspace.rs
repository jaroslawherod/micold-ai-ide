//! The user's filesystem and their git working copies (feature 021, T055 — FR-019a).
//!
//! **An eighth module the T050–T054 split did not produce, and its absence was the finding.** Those
//! tasks moved out startup, persistence, the daemon protocol, the subscriptions, the environment
//! include and the OS theme — every system that had a *function* in `main.rs` named after it. The
//! filesystem never did: browsing for a folder, deciding whether it is a git repository, and
//! enumerating its worktrees lived only as arms of `update_inner`, so there was nothing to move
//! until the arms themselves moved. T055 is where it becomes visible.
//!
//! # Two systems or one
//!
//! Directory listing (`FolderBrowser`) and git (`Git`) are separate capabilities and are kept
//! separate as capabilities. They are one *shell module* because they are one conversation: the
//! folder picker exists to find a repository, `FolderChosen` asks git whether the chosen directory
//! is one, and the same arm then asks git what worktrees it holds. Splitting the module would put
//! two halves of a single decision in two files and satisfy nothing FR-019a is asking for — the
//! rule is about what a change to one external system can reach, and a change to "how we open a
//! project" reaches both.
//!
//! # What these arms deliberately do *not* do
//!
//! They do not persist. The daemon is the single writer of `projects.json` (see `shell/persist.rs`
//! for why), so opening a project sends `ProjectAdd` and lets the catalog come back; the local git
//! discovery here only seeds the worktree list so the UI is populated before the daemon's
//! post-attach refresh reconciles it.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use iced::Task;

use micold_client::app::Message;
use micold_core::fs_scan::FolderBrowser;
use micold_core::git::Git;
use micold_core::protocol::messages::ClientMsg;
use micold_core::selector::{Selector, SelectorStatus};
use micold_core::worktree::Worktree;

use crate::shell::daemon_sync::{send_op, switch_daemon_attachment, PendingOp};
use crate::App;

/// Where the folder picker opens: the user's home directory, or the filesystem root if the
/// platform will not name one.
pub(crate) fn start_dir() -> PathBuf {
    directories::UserDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(std::path::MAIN_SEPARATOR_STR))
}

/// List `dir`'s subdirectories off the update thread — a cold or network-mounted directory can
/// take long enough to drop frames if it is listed inline.
pub(crate) fn scan_task(
    browser: Arc<dyn FolderBrowser + Send + Sync>,
    dir: PathBuf,
) -> Task<Message> {
    Task::perform(async move { scan(&*browser, dir) }, |message| message)
}

fn scan(browser: &dyn FolderBrowser, dir: PathBuf) -> Message {
    match browser.list_subdirs(&dir) {
        Ok(entries) => Message::SelectorListingReady(entries),
        Err(error) => Message::SelectorListingFailed(error.to_string()),
    }
}

/// The included set is deliberately empty here (016 BUG-002): it is the daemon's, persisted per
/// project, and this call is only the local seed that shows *something* while the daemon's own
/// discovery is still in flight. The catalog push that follows replaces this list entirely, included
/// worktrees and all.
///
/// Empty when this client has no local git (feature 027, research R2 part 2). That is the right
/// answer rather than a degraded one: the seed exists to avoid a blank list for the few frames
/// before the daemon's push, and a seed derived from *host* paths while the daemon reports
/// *container* paths would not be a faster version of the truth — it would be a different list,
/// briefly shown, and then replaced.
pub(crate) fn discover_worktrees(git: Option<&dyn Git>, repo: &Path) -> Vec<Worktree> {
    match git {
        Some(git) => micold_core::worktree::discover(git, repo, &[]),
        None => Vec::new(),
    }
}

/// Open the folder picker at the user's home directory and start listing it.
pub(crate) fn on_project_selector_opened(app: &mut App) -> Task<Message> {
    let dir = start_dir();
    app.core.clear_for_dialog();
    app.core.selector = Some(Selector::open_at(dir.clone()));
    scan_task(app.caps.browser(), dir)
}

/// Navigating the picker lists the directory the reducer moved to — but only when the reducer
/// says it is waiting for one. Anything else (a cached listing, a refused navigation) already has
/// its answer and must not spawn a second scan.
pub(crate) fn on_selector_navigated(app: &mut App, message: Message) -> Task<Message> {
    app.core.update(message);
    match &app.core.selector {
        Some(selector) if selector.status == SelectorStatus::Loading => {
            scan_task(app.caps.browser(), selector.current_dir.clone())
        }
        _ => Task::none(),
    }
}

/// What the open-project gate says when it refuses (FR-001a). One wording, whichever side of the
/// connection answered the question.
pub(crate) const NOT_A_REPOSITORY: &str = "Only git repositories can be opened as projects.";

/// Open the chosen folder as a project — but only if it is a git repository (FR-001a).
///
/// Who answers that question depends on where the daemon is (feature 027, research R2 part 2). With
/// a local git the answer is immediate and the project opens in this same update. Without one the
/// gate becomes a round trip, and [`on_repo_root_answer`] is the other half.
pub(crate) fn on_folder_chosen(app: &mut App, path: PathBuf) -> Task<Message> {
    // Close the picker BEFORE the git gate. Notifications render inside `base`, which
    // every modal wraps behind its scrim, so a refusal reported while the selector was
    // still open would be dimmed out of view.
    app.core.selector = None;
    let Some(git) = app.caps.git() else {
        // No local git: ask the side that has one. Nothing is opened, refused, or persisted until
        // the answer lands — the gate has not been passed, only deferred.
        let asked = path.clone();
        send_op(app, PendingOp::RepoRootQuery(path), move |req| {
            ClientMsg::RepoRootQuery { req, path: asked }
        });
        return Task::none();
    };
    if !git.is_repo_root(&path) {
        app.core
            .update(Message::ProjectOpenRefused(NOT_A_REPOSITORY.to_string()));
        return Task::none();
    }
    open_verified_project(app, path)
}

/// The daemon's verdict on a folder this client could not judge for itself (feature 027, R2).
///
/// `path` is echoed back from the request precisely so this can be checked: the user may have
/// cancelled, opened something else, or switched project while the question was in flight, and an
/// answer about a folder nobody is waiting on any more must not open it.
pub(crate) fn on_repo_root_answer(
    app: &mut App,
    asked: PathBuf,
    answered: PathBuf,
    is_repo_root: bool,
) -> Task<Message> {
    if asked != answered {
        return Task::none();
    }
    if !is_repo_root {
        app.core
            .update(Message::ProjectOpenRefused(NOT_A_REPOSITORY.to_string()));
        return Task::none();
    }
    open_verified_project(app, answered)
}

/// Everything that happens once the folder is known to be a repository, whoever established it.
fn open_verified_project(app: &mut App, path: PathBuf) -> Task<Message> {
    // Switch without tearing down the outgoing project's sessions (feature 008, BS-1).
    // `open_or_activate` moves `active` to the new project, so capture the outgoing
    // foreground FIRST (I1), then finish the switch bookkeeping for the new project. The
    // in-memory `open_or_activate` gives instant UI; local git discovery seeds the worktree
    // list until the daemon's post-attach refresh reconciles it (T055).
    let previous = app.core.workspace.active.clone();
    app.core.record_foreground();
    app.core
        .workspace
        .open_or_activate(path.clone(), app.caps.scanner());
    let arrival = app.core.restore_after_activation(&path);
    micold_client::app::drain(arrival, |o| micold_client::app::interpret(&mut app.core, o));
    let outcomes = app
        .core
        .set_worktrees(discover_worktrees(app.caps.git(), &path));
    micold_client::app::drain(outcomes, |o| {
        micold_client::app::interpret(&mut app.core, o)
    });
    app.core.worktree_error = None;
    crate::log_foreground_choice(app, &path);
    // The daemon is the single writer: tell it to learn this project (persist + discover),
    // and switch this client's attachment to it. No local `persist()`, no local
    // transcript-reconcile — sessions come from the daemon catalog via reconcile_catalog.
    let add_path = path.clone();
    send_op(app, PendingOp::ProjectAdd, move |req| {
        ClientMsg::ProjectAdd {
            req,
            path: add_path,
        }
    });
    switch_daemon_attachment(app, previous, &path);
    Task::none()
}

/// Re-enter a project the workspace already knows.
pub(crate) fn on_known_project_reopened(app: &mut App, path: PathBuf) -> Task<Message> {
    app.core.workspace.refresh_availability(app.caps.scanner());
    // Non-destructive switch: keep the outgoing project's sessions running in the
    // background and restore the target project's foreground (feature 008, BS-1/BS-3).
    let previous = app.core.workspace.active.clone();
    if let Some(arrival) = app.core.switch_active(&path) {
        micold_client::app::drain(arrival, |o| micold_client::app::interpret(&mut app.core, o));
        let outcomes = app
            .core
            .set_worktrees(discover_worktrees(app.caps.git(), &path));
        micold_client::app::drain(outcomes, |o| {
            micold_client::app::interpret(&mut app.core, o)
        });
        crate::log_foreground_choice(app, &path);
        // Already a known project (no ProjectAdd); just move the daemon attachment.
        switch_daemon_attachment(app, previous, &path);
    }
    Task::none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::base_app;
    use micold_core::fs_scan::FakeFolderScanner;
    use micold_core::project::FolderEntry;
    use micold_core::protocol::messages::{DaemonMsg, OperationResult};

    /// A client with no local git and a daemon to ask (feature 027, research R2 part 2) — the
    /// Windows and remote-placement shape, reproduced on any platform because the capability is
    /// removed by value rather than by `cfg`.
    fn client_without_local_git() -> (
        App,
        iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>,
    ) {
        let (tx, rx) = iced::futures::channel::mpsc::unbounded();
        let mut app = base_app();
        app.daemon = Some(micold_client::daemon::Outbox::new(tx));
        app.caps = std::mem::replace(
            &mut app.caps,
            crate::shell::capabilities::Capabilities::real(),
        )
        .without_local_git();
        (app, rx)
    }

    /// The whole point of R2 part 2, as one assertion: a client that cannot see the daemon's
    /// filesystem does not guess. It asks.
    ///
    /// The failure this prevents is not a wrong error message. On Windows the host sees
    /// `C:\\Users\\u\\p` and the container sees `/mnt/host/c/Users/u/p`; a client answering the
    /// open-project gate from its own filesystem would admit or refuse a folder on the strength of
    /// a path the daemon will never act on.
    #[test]
    fn without_local_git_the_gate_is_asked_over_the_wire() {
        let (mut app, mut rx) = client_without_local_git();
        let chosen = PathBuf::from("/repo/thing");
        let _ = on_folder_chosen(&mut app, chosen.clone());

        let sent = rx.try_recv().expect("a question was asked");
        match sent {
            ClientMsg::RepoRootQuery { path, .. } => assert_eq!(path, chosen),
            other => panic!("expected the gate to be asked over the wire, got {other:?}"),
        }
        assert!(
            app.core.workspace.active.is_none(),
            "nothing may be opened until the answer lands — the gate is deferred, not passed"
        );
        assert!(
            app.core.notify.visible().is_none(),
            "nor may anything be refused: no verdict has been given yet"
        );
    }

    /// And the answer completes the interaction the user started.
    #[test]
    fn a_yes_from_the_daemon_opens_the_project() {
        let (mut app, mut rx) = client_without_local_git();
        let chosen = PathBuf::from("/repo/thing");
        let _ = on_folder_chosen(&mut app, chosen.clone());
        let req = match rx.try_recv().expect("asked") {
            ClientMsg::RepoRootQuery { req, .. } => req,
            other => panic!("{other:?}"),
        };

        let _ = crate::shell::daemon_sync::on_daemon_event(
            &mut app,
            DaemonMsg::OperationOk {
                req,
                result: OperationResult::RepoRoot {
                    path: chosen.clone(),
                    is_repo_root: true,
                },
            },
        );
        assert_eq!(app.core.workspace.active.as_deref(), Some(chosen.as_path()));
    }

    /// A no is refused in the same words the local gate uses. One rule, two answerers.
    #[test]
    fn a_no_from_the_daemon_is_refused_in_the_same_words() {
        let (mut app, mut rx) = client_without_local_git();
        let chosen = PathBuf::from("/repo/not-one");
        let _ = on_folder_chosen(&mut app, chosen.clone());
        let req = match rx.try_recv().expect("asked") {
            ClientMsg::RepoRootQuery { req, .. } => req,
            other => panic!("{other:?}"),
        };

        let _ = crate::shell::daemon_sync::on_daemon_event(
            &mut app,
            DaemonMsg::OperationOk {
                req,
                result: OperationResult::RepoRoot {
                    path: chosen,
                    is_repo_root: false,
                },
            },
        );
        assert!(app.core.workspace.active.is_none());
        assert_eq!(
            app.core.notify.visible().map(|n| n.message.as_str()),
            Some(NOT_A_REPOSITORY),
            "the wording must not fork by who answered"
        );
    }

    /// An answer about a folder nobody is waiting on any more must not open it.
    ///
    /// This is what the echoed path is for. A gate answered asynchronously is a gate that can be
    /// answered after the user has moved on, and "the reply arrived, so act on it" is how a
    /// cancelled dialog opens a project.
    #[test]
    fn an_answer_about_a_different_folder_is_ignored() {
        let (mut app, mut rx) = client_without_local_git();
        let chosen = PathBuf::from("/repo/asked-about");
        let _ = on_folder_chosen(&mut app, chosen);
        let req = match rx.try_recv().expect("asked") {
            ClientMsg::RepoRootQuery { req, .. } => req,
            other => panic!("{other:?}"),
        };

        let _ = crate::shell::daemon_sync::on_daemon_event(
            &mut app,
            DaemonMsg::OperationOk {
                req,
                result: OperationResult::RepoRoot {
                    path: PathBuf::from("/repo/something-else"),
                    is_repo_root: true,
                },
            },
        );
        assert!(
            app.core.workspace.active.is_none(),
            "an answer about another folder must not open one"
        );
    }

    /// The worktree seed is empty rather than wrong when there is no local git.
    ///
    /// A seed built from host paths while the daemon reports container paths is not a faster
    /// version of the truth — it is a different list, shown briefly and then replaced.
    #[test]
    fn no_local_git_means_no_local_worktree_seed() {
        let (app, _rx) = client_without_local_git();
        assert!(app.caps.git().is_none());
        assert!(discover_worktrees(app.caps.git(), Path::new("/repo/thing")).is_empty());
    }

    /// `base_app` carries the real git capability, and a temp dir is reliably not a repo root.
    fn choose_a_non_repository() -> App {
        let mut app = base_app();
        app.core.selector = Some(Selector::open_at(PathBuf::from("/tmp")));
        let dir = std::env::temp_dir().join("micold-t055-not-a-repo");
        let _ = std::fs::create_dir_all(&dir);
        let _ = on_folder_chosen(&mut app, dir);
        app
    }

    /// A directory that is not a git repository is refused.
    ///
    /// The refusal is the whole of FR-001a, and the failure it prevents is not a bad error
    /// message: a non-repository opened as a project produces a workspace entry whose worktree
    /// discovery, branch listing and worktree creation all fail separately and later.
    #[test]
    fn a_folder_that_is_not_a_repository_is_refused() {
        assert!(
            choose_a_non_repository().core.workspace.active.is_none(),
            "a non-repository must not become the active project"
        );
    }

    /// …and the picker closes *before* the refusal is reported.
    ///
    /// Its own test, because it is its own bug and it fails silently: notifications render inside
    /// `base`, which every modal wraps behind its scrim, so a refusal raised while the selector is
    /// still open is dimmed out of view. The project is still correctly refused — the user simply
    /// sees nothing happen and clicks again.
    #[test]
    fn the_picker_closes_before_the_refusal_is_reported() {
        assert!(
            choose_a_non_repository().core.selector.is_none(),
            "the picker must close before the refusal is reported, or the notification renders \
             behind the modal's scrim and the user sees nothing happen"
        );
    }

    /// The listing runs against the browser capability rather than the real filesystem, which is
    /// the point of the capability — and it asks about the directory it was given, not an
    /// ambient one.
    #[test]
    fn the_scan_asks_the_browser_about_the_directory_it_was_given() {
        let browser = FakeFolderScanner::new().with_subdirs(
            "/work",
            vec![
                FolderEntry {
                    name: "a".to_string(),
                    path: PathBuf::from("/work/a"),
                    is_git_repo: true,
                },
                FolderEntry {
                    name: "b".to_string(),
                    path: PathBuf::from("/work/b"),
                    is_git_repo: false,
                },
            ],
        );

        match scan(&browser, PathBuf::from("/work")) {
            Message::SelectorListingReady(entries) => assert_eq!(entries.len(), 2),
            other => panic!("expected a listing, got {other:?}"),
        }
        // A different directory is a different answer — an implementation that ignored `dir` and
        // returned one cached listing would pass the assertion above on its own.
        match scan(&browser, PathBuf::from("/elsewhere")) {
            Message::SelectorListingReady(entries) => assert!(entries.is_empty()),
            other => panic!("expected an empty listing, got {other:?}"),
        }
    }

    /// A directory the browser cannot read becomes a message the picker can show, not a panic and
    /// not silence. An unreadable directory is ordinary — a permission-denied mount, a stale
    /// automount — and the picker has to stay usable.
    #[test]
    fn a_directory_that_cannot_be_listed_becomes_a_reported_failure() {
        let browser = FakeFolderScanner::new().with_unreadable("/nope");

        match scan(&browser, PathBuf::from("/nope")) {
            Message::SelectorListingFailed(_) => {}
            other => panic!("expected a reported failure, got {other:?}"),
        }
    }
}
