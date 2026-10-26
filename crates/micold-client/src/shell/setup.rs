//! Setup runs in the shell (feature 1289): hands each project's setup command to the daemons that
//! hold it (contract C3), and routes `SetupChanged` into the reducer (contract C2), posting the
//! notification it asks for (contract U5). The decisions are `features::setup`'s.

use std::collections::BTreeMap;
use std::path::PathBuf;

use micold_client::features::setup::{self, Effect, Msg, Request};
use micold_client::ui::setup_output::SETUP_OUTPUT_SCROLL_ID;
use micold_core::daemons::{Binding, DaemonId};
use micold_core::protocol::messages::ClientMsg;
use micold_core::session::SessionLocation;
use micold_core::settings::SetupCommandChange;
use micold_core::setup::{SetupCommand, SetupRunSummary};

use crate::shell::daemon_sync::{send_op_bound, send_op_bound_req, PendingOp};
use crate::App;
use iced::Task;
use micold_client::app::Message;

/// Tell `daemon`, which just connected, the command of every project its catalog holds, or that
/// it has none (contract C3; M5 review A-F2). The commands are read from the store, so a save in
/// another window since this one started is what is sent. Projects registered on this connect took
/// theirs with their `ProjectAdd` ([`add_project`]).
pub fn on_connect(app: &mut App, daemon: DaemonId) {
    let held: Vec<PathBuf> = app
        .daemon_catalogs
        .get(&daemon)
        .map(|catalog| catalog.projects.iter().map(|p| p.path.clone()).collect())
        .unwrap_or_default();
    for (project, command) in setup::on_connect(in_force(app), held) {
        send_setup_set(app, Binding::Bound(daemon), project, command);
    }
}

/// Register `path` on the daemon `binding` routes to (`ProjectAdd`), and hand it the project's
/// stored setup command right behind it (contract C3): the daemon applies frames in order (C1), and
/// forgetting a project drops its command there, so every add carries it (M5 review A-F1). Nothing
/// follows an add that could not be sent.
pub fn add_project(app: &mut App, binding: Binding, path: PathBuf) {
    let added = path.clone();
    let sent = send_op_bound_req(app, binding, PendingOp::ProjectAdd, move |req| {
        ClientMsg::ProjectAdd { req, path: added }
    });
    if sent.is_none() {
        return;
    }
    if let Some(command) = setup::command_for(in_force(app), &path).cloned() {
        send_setup_set(app, binding, path, Some(command));
    }
}

fn send_setup_set(
    app: &mut App,
    binding: Binding,
    project: PathBuf,
    command: Option<SetupCommand>,
) {
    send_op_bound(app, binding, PendingOp::ProjectSetupSet, move |req| {
        ClientMsg::ProjectSetupSet {
            req,
            project,
            command,
        }
    });
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
        send_setup_set(app, Binding::Bound(daemon), project, command);
    }
}

/// Apply `msg` to the setup state, and perform what it asks for: the notification (FR-017), the
/// output view's scroll to its end, or its copy (contract U4).
pub fn update(app: &mut App, msg: micold_client::features::setup::Msg) -> Task<Message> {
    match app.core.update_setup(msg) {
        Effect::NotifyError(text) => app.core.notify_error(text),
        Effect::Watch(request) => send_watch(app, request),
        Effect::Run((daemon, project, location)) => {
            send_op_bound(
                app,
                Binding::Bound(daemon),
                PendingOp::SetupRun,
                move |req| ClientMsg::SetupRun {
                    req,
                    project,
                    location,
                },
            );
        }
        Effect::Stop((daemon, project, location)) => {
            send_op_bound(
                app,
                Binding::Bound(daemon),
                PendingOp::SetupStop,
                move |req| ClientMsg::SetupStop {
                    req,
                    project,
                    location,
                },
            );
        }
        Effect::ScrollToEnd => {
            return iced::widget::operation::snap_to_end(SETUP_OUTPUT_SCROLL_ID.clone());
        }
        Effect::Copy(text) => {
            return crate::shell::clipboard::interpret(
                micold_client::features::Outcome::ClipboardWrite(text),
            );
        }
        Effect::None => {}
    }
    Task::none()
}

/// Send what the open output view needs (contract C1, U4): a watch for the location it shows, an
/// unwatch for one it no longer shows. After every message, so every route that closes the view
/// stops the watch.
pub fn sync_watch(app: &mut App) {
    for request in setup::sync_watch(&mut app.core.setup) {
        send_watch(app, request);
    }
}

/// `daemon` connected: the open view of one of its locations is watched again from the line after
/// the last one held (FR-018).
pub fn rewatch(app: &mut App, daemon: DaemonId) {
    if let Some(request) = setup::rewatch(&mut app.core.setup, daemon) {
        send_watch(app, request);
    }
}

/// Send `request` to its daemon, when connected; a daemon away gets the watch on reconnect.
fn send_watch(app: &mut App, request: Request) {
    let (key, msg) = match request {
        Request::Watch { key, from_line } => {
            let (_, project, location) = key.clone();
            (
                key,
                ClientMsg::SetupOutputWatch {
                    project,
                    location,
                    from_line,
                },
            )
        }
        Request::Unwatch { key } => {
            let (_, project, location) = key.clone();
            (key, ClientMsg::SetupOutputUnwatch { project, location })
        }
    };
    if let Some(outbox) = app.outbox_for(key.0) {
        outbox.send(msg);
    }
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
        names: setup::names(&app.core, &project, &location),
        daemon,
        project,
        location,
        run,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::base_app;
    use micold_core::protocol::messages::{DaemonMsg, ErrorKind};

    const DAEMON: DaemonId = DaemonId(1);

    fn key() -> setup::Key {
        (
            DAEMON,
            PathBuf::from("/repo"),
            SessionLocation::Worktree("feat-a".into()),
        )
    }

    fn connected() -> (
        App,
        iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>,
    ) {
        let (tx, rx) = iced::futures::channel::mpsc::unbounded();
        let mut app = base_app();
        app.connect_test_daemon(micold_client::daemon::Outbox::new(tx));
        (app, rx)
    }

    /// M7 (T056): the entries send `SetupRun` and `SetupStop` for the location to its daemon.
    #[test]
    fn run_and_stop_requests_send_the_wire_messages() {
        let (mut app, mut rx) = connected();
        let _ = update(&mut app, Msg::RunRequested(key()));
        let _ = update(&mut app, Msg::StopRequested(key()));
        let sent: Vec<ClientMsg> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
        assert!(
            matches!(&sent[0], ClientMsg::SetupRun { project, location, .. }
                if project == &PathBuf::from("/repo")
                    && location == &SessionLocation::Worktree("feat-a".into())),
            "{sent:?}"
        );
        assert!(matches!(&sent[1], ClientMsg::SetupStop { .. }), "{sent:?}");
        assert_eq!(sent.len(), 2);
    }

    /// M7 (T056, FR-023a): the daemon's `Busy` answer reaches the user with the daemon's text.
    #[test]
    fn a_busy_answer_becomes_an_error_notification_with_the_daemons_text() {
        let (mut app, mut rx) = connected();
        let _ = update(&mut app, Msg::RunRequested(key()));
        let Some(ClientMsg::SetupRun { req, .. }) = rx.try_recv().ok() else {
            panic!("a SetupRun was sent");
        };
        assert!(app.core.notifications.queue.visible().is_none());
        let _ = crate::shell::daemon_sync::on_daemon_event(
            &mut app,
            DAEMON,
            DaemonMsg::OperationError {
                req,
                kind: ErrorKind::Busy,
                message: "Setup is already running in feat-a.".into(),
                detail: None,
            },
        );
        let shown = app
            .core
            .notifications
            .queue
            .visible()
            .expect("an error notification");
        assert!(
            format!("{shown:?}").contains("Setup is already running in feat-a."),
            "{shown:?}"
        );
    }
}
