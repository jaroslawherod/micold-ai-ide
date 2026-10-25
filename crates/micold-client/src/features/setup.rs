//! Each worktree's setup run as this client knows it (feature 1289, data-model §Client).
//!
//! Render-free: the daemon reports a location's run in `SetupChanged` (contract C2), and this
//! module keeps the latest summary per `(daemon, project, location)`, decides when a failed run
//! is worth the one error notification each client posts for it (FR-017, contract U5), words the
//! row's tag (contract U2), and picks which daemons a project's setup command is sent to (contract
//! C3). The shell routes the wire messages here and acts on what comes back; every decision is a
//! pure function the tests call.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use micold_core::daemons::DaemonId;
use micold_core::local_time::{LocalTime, UtcOffset};
use micold_core::session::SessionLocation;
use micold_core::settings::SetupCommandChange;
use micold_core::setup::{RunId, SetupCommand, SetupRunSummary, SetupState};

/// Which location's run a summary is: the daemon that runs it, the project path in that daemon's
/// catalog, and the worktree or the project folder.
pub type Key = (DaemonId, PathBuf, SessionLocation);

/// What this client knows of setup runs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// The latest summary of each location that has a run record.
    pub runs: BTreeMap<Key, SetupRunSummary>,
    /// The failed runs this client has already posted a notification for (FR-017): one per run,
    /// whether it learned of the failure live or in the catch-up after an attach.
    pub notified: BTreeSet<(DaemonId, PathBuf, SessionLocation, RunId)>,
}

/// The names a notification shows for a location, as the app shows them elsewhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Names {
    /// The project's display name.
    pub project: String,
    /// The worktree's display name, or the Default entry's.
    pub location: String,
}

/// What reaches the reducer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// A `SetupChanged` from `daemon` (contract C2): the location's run, or `None` when its record
    /// went away (the worktree was deleted).
    Changed {
        daemon: DaemonId,
        project: PathBuf,
        location: SessionLocation,
        run: Option<SetupRunSummary>,
        /// What the notification calls the project and the location.
        names: Names,
    },
}

/// What the shell does after a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Nothing.
    None,
    /// Post this error notification (contract U5).
    NotifyError(String),
}

/// Apply `msg` to `state`.
pub fn update(state: &mut State, msg: Msg) -> Effect {
    match msg {
        Msg::Changed {
            daemon,
            project,
            location,
            run,
            names,
        } => {
            let key = (daemon, project, location);
            let Some(summary) = run else {
                // The record went away with its worktree: so do the runs it notified for, so a
                // worktree recreated under the same name starts from nothing.
                state
                    .notified
                    .retain(|(d, p, l, _)| (d, p, l) != (&key.0, &key.1, &key.2));
                state.runs.remove(&key);
                return Effect::None;
            };
            let id = summary.run.id();
            let notice = failure_notice(&names, summary.run.state());
            state.runs.insert(key.clone(), summary);
            match notice {
                Some(text) if state.notified.insert((key.0, key.1, key.2, id)) => {
                    Effect::NotifyError(text)
                }
                _ => Effect::None,
            }
        }
    }
}

impl State {
    /// The run `daemon` reports for `location` of `project`, if it has a record.
    pub fn summary(
        &self,
        daemon: DaemonId,
        project: &Path,
        location: &SessionLocation,
    ) -> Option<&SetupRunSummary> {
        self.runs
            .get(&(daemon, project.to_path_buf(), location.clone()))
    }
}

/// The notification for a run that ended in `state`, or `None` unless it failed (contract U5): a
/// run that was stopped or interrupted ended because someone or something asked it to.
pub fn failure_notice(names: &Names, state: &SetupState) -> Option<String> {
    if !matches!(state, SetupState::Failed(_)) {
        return None;
    }
    let reason = state.reason_line()?;
    let reason = reason.strip_suffix('.').unwrap_or(&reason);
    Some(format!(
        "Setup failed in {} / {}: {reason}.",
        names.project, names.location
    ))
}

/// How a row tag is tinted (contract U2): the words carry the state, the tint only repeats it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Running, succeeded or stopped.
    Neutral,
    /// Failed.
    Error,
    /// Interrupted.
    Warning,
}

/// A row's setup tag: its text, its tooltip and its tint (contract U2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowTag {
    pub label: &'static str,
    pub tooltip: String,
    pub tone: Tone,
}

/// The tag a row shows for `summary`, with times in `offset` (the local zone at that instant).
pub fn row_tag(summary: &SetupRunSummary, offset: impl Fn(u64) -> UtcOffset) -> RowTag {
    let run = &summary.run;
    let state = run.state();
    let at = |secs: u64| {
        LocalTime::at(i64::try_from(secs).unwrap_or(i64::MAX), offset(secs)).date_hh_mm()
    };
    let ended = run.ended_at().unwrap_or(run.started_at());
    let (tooltip, tone) = match state {
        SetupState::Running => (
            format!("Setup running since {}", at(run.started_at())),
            Tone::Neutral,
        ),
        SetupState::Succeeded => (format!("Setup finished at {}", at(ended)), Tone::Neutral),
        SetupState::Failed(_) => {
            let reason = state.reason_line().unwrap_or_default();
            let reason = reason.strip_suffix('.').unwrap_or(&reason).to_owned();
            (
                format!("Setup failed at {}: {reason}", at(ended)),
                Tone::Error,
            )
        }
        SetupState::Stopped => (format!("Setup stopped at {}", at(ended)), Tone::Neutral),
        SetupState::Interrupted => (
            "Setup interrupted: the daemon stopped".to_owned(),
            Tone::Warning,
        ),
    };
    RowTag {
        label: state.label(),
        tooltip,
        tone,
    }
}

/// The command `commands` holds for `project`, keyed as the settings file keys it (contract C3).
pub fn command_for<'a>(
    commands: &'a BTreeMap<String, SetupCommand>,
    project: &Path,
) -> Option<&'a SetupCommand> {
    commands.get(&micold_core::jira::connection_key(project))
}

/// What a daemon that just connected is told (contract C3): one `(path, command)` per project it
/// holds that has a command. `held` is every project path registered on it, as its catalog holds
/// them and as the client just sent them in `ProjectAdd`; a path given twice is sent once.
pub fn on_connect(
    commands: &BTreeMap<String, SetupCommand>,
    held: impl IntoIterator<Item = PathBuf>,
) -> Vec<(PathBuf, SetupCommand)> {
    held.into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter_map(|path| command_for(commands, &path).map(|c| (path, c.clone())))
        .collect()
}

/// Who is told of `change` (contract C3): each connected daemon whose catalog holds the project,
/// with the path that daemon's catalog holds it under. `catalogs` is each connected daemon's
/// project paths; a daemon that does not hold the project is told nothing.
pub fn on_change(
    change: &SetupCommandChange,
    catalogs: &BTreeMap<DaemonId, Vec<PathBuf>>,
) -> Vec<(DaemonId, PathBuf, Option<SetupCommand>)> {
    catalogs
        .iter()
        .flat_map(|(daemon, paths)| {
            paths
                .iter()
                .filter(|path| micold_core::jira::connection_key(path) == change.project)
                .map(|path| (*daemon, path.clone(), change.command.clone()))
        })
        .collect()
}
