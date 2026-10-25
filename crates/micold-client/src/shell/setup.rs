//! Setup runs in the shell (feature 1289): hands each project's setup command to the daemons that
//! hold it (contract C3), and routes `SetupChanged` into the reducer (contract C2), posting the
//! notification it asks for (contract U5). The decisions are `features::setup`'s.

use std::collections::BTreeMap;
use std::path::PathBuf;

use micold_client::features::setup::{self, Effect, Msg, Names};
use micold_core::daemons::{Binding, DaemonId};
use micold_core::protocol::messages::ClientMsg;
use micold_core::session::SessionLocation;
use micold_core::settings::SetupCommandChange;
use micold_core::setup::{SetupCommand, SetupRunSummary};

use crate::shell::daemon_sync::{send_op_bound, PendingOp};
use crate::App;
use iced::Task;
use micold_client::app::Message;

/// Tell `daemon`, which just connected, the command of every project it holds: those its catalog
/// lists and those `registered` was just sent in `ProjectAdd` (contract C3). The commands are read
/// from the store, so a save in another window since this one started is what is sent.
pub fn on_connect(app: &mut App, daemon: DaemonId, registered: Vec<PathBuf>) {
    in_force(app);
    let held: Vec<PathBuf> = app
        .daemon_catalogs
        .get(&daemon)
        .map(|catalog| catalog.projects.iter().map(|p| p.path.clone()).collect())
        .unwrap_or_default();
    send_commands(app, daemon, held.into_iter().chain(registered));
}

/// Each project's setup command as stored, re-read from the store when there is one (another
/// window may have saved since), else as this window holds it.
pub fn in_force(app: &mut App) -> &BTreeMap<String, SetupCommand> {
    if let Some(store) = app.caps.settings() {
        app.core.settings.setup_commands = store.load().settings.setup_commands;
    }
    &app.core.settings.setup_commands
}

/// The open project's path and display name, as Settings › Project shows it.
pub fn open_project(app: &App) -> Option<(PathBuf, String)> {
    app.core
        .workspace
        .active_project()
        .map(|p| (p.path.clone(), p.display_name.clone()))
}

/// Tell `daemon` the command of each project in `paths` that has one: projects just registered
/// there, in the order their `ProjectAdd` went (the daemon applies frames in order, contract C1).
pub fn send_commands(app: &mut App, daemon: DaemonId, paths: impl IntoIterator<Item = PathBuf>) {
    for (project, command) in setup::on_connect(&app.core.settings.setup_commands, paths) {
        send_op_bound(
            app,
            Binding::Bound(daemon),
            PendingOp::ProjectSetupSet,
            move |req| ClientMsg::ProjectSetupSet {
                req,
                project,
                command: Some(command),
            },
        );
    }
}

/// A save changed one project's command: tell each connected daemon whose catalog holds it, with
/// its own catalog's path (contract C3).
pub fn on_saved(app: &mut App, change: &SetupCommandChange) {
    let catalogs = app
        .daemon_catalogs
        .iter()
        .filter(|(id, _)| app.outbox_for(**id).is_some())
        .map(|(id, catalog)| {
            (
                *id,
                catalog.projects.iter().map(|p| p.path.clone()).collect(),
            )
        })
        .collect();
    for (daemon, project, command) in setup::on_change(change, &catalogs) {
        send_op_bound(
            app,
            Binding::Bound(daemon),
            PendingOp::ProjectSetupSet,
            move |req| ClientMsg::ProjectSetupSet {
                req,
                project,
                command,
            },
        );
    }
}

/// Apply `msg` to the setup state, and post the notification it asks for (FR-017).
pub fn update(app: &mut App, msg: micold_client::features::setup::Msg) -> Task<Message> {
    match setup::update(&mut app.core.setup, msg) {
        Effect::NotifyError(text) => app.core.notify_error(text),
        Effect::None => {}
    }
    Task::none()
}

/// A `SetupChanged` from `daemon` (contract C2), as the reducer's message: the summary, with the
/// names its notification would show.
pub fn changed(
    app: &App,
    daemon: DaemonId,
    project: PathBuf,
    location: SessionLocation,
    run: Option<SetupRunSummary>,
) -> Msg {
    Msg::Changed {
        names: names(app, &project, &location),
        daemon,
        project,
        location,
        run,
    }
}

/// What a notification calls `project` and `location`: the names the sidebar shows.
fn names(app: &App, project: &std::path::Path, location: &SessionLocation) -> Names {
    let workspace = &app.core.workspace;
    let project_name = workspace
        .projects
        .iter()
        .find(|p| p.path == project)
        .map(|p| p.display_name.clone())
        .unwrap_or_else(|| {
            project
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| project.display().to_string())
        });
    let location = match location {
        SessionLocation::Default => "Default".to_owned(),
        SessionLocation::Worktree(dir) if workspace.active.as_deref() == Some(project) => {
            app.core.worktree_display_name(dir)
        }
        SessionLocation::Worktree(dir) => micold_core::naming::display_name(dir),
    };
    Names {
        project: project_name,
        location,
    }
}
