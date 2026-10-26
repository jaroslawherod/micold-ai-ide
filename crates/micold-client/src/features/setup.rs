//! Each worktree's setup run as this client knows it (feature 1289, data-model §Client).
//!
//! Render-free: the daemon reports a location's run in `SetupChanged` (contract C2), and this
//! module keeps the latest summary per `(daemon, project, location)`, decides when a failed run
//! is worth the one error notification each client posts for it (FR-017, contract U5), words the
//! row's tag (contract U2), and picks which daemons a project's setup command is sent to (contract
//! C3). It also holds the one open setup output view (contract U4): the lines it was sent, whether
//! it follows the tail, and which location the shell watches for it (FR-016, FR-018, FR-021). The
//! shell routes the wire messages here and acts on what comes back; every decision is a pure
//! function the tests call.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use micold_core::daemons::DaemonId;
use micold_core::local_time::{LocalTime, UtcOffset};
use micold_core::session::SessionLocation;
use micold_core::settings::SetupCommandChange;
use micold_core::setup::output::KEEP_LINES;
use micold_core::setup::{RunId, SetupCommand, SetupRunSummary, SetupState};

use crate::features::structured_transcript::Follow;

/// The height of one line of the output view, in dp: the list is virtual, so every row is this
/// tall.
pub const ROW_HEIGHT: f32 = 20.0;

/// Which location's run a summary is: the daemon that runs it, the project path in that daemon's
/// catalog, and the worktree or the project folder.
pub type Key = (DaemonId, PathBuf, SessionLocation);

/// What this client knows of setup runs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// The latest summary of each location that has a run record. A failure notifies only when
    /// the summary before it was the same run, still running (FR-017): the daemon tells every
    /// connected client a run started, and replays its records on each attach, so a run first
    /// seen already failed ended before this client was there to hear it (M5 review A-F3).
    pub summaries: BTreeMap<Key, SetupRunSummary>,
    /// The setup output view, while it is open (contract U4).
    pub view: Option<OutputView>,
    /// The location whose output the shell last asked a daemon to send ([`sync_watch`]).
    pub watched: Option<Key>,
}

/// The open setup output view (contract U4): the kept lines of one location's latest run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputView {
    /// The location shown.
    pub key: Key,
    /// The run the lines belong to; `None` until the first batch arrives.
    pub run: Option<RunId>,
    /// The index of `lines[0]` in the run's output: how many earlier lines this view does not hold.
    pub first_line: u64,
    /// The lines held, oldest first, at most [`KEEP_LINES`] (FR-021).
    pub lines: Vec<String>,
    /// Whether new lines keep the end in view.
    pub follow: Follow,
    /// The list's scroll offset, as last reported, in pixels from the top.
    pub offset: u32,
    /// The list's viewport height, as last reported; 0 before the first report.
    pub viewport: u32,
    /// The first line of the watch sent last, until its backlog arrives: batches sent before the
    /// daemon took that watch are in the backlog, and are not taken (M6 review A-F2).
    pub pending: Option<u64>,
}

impl OutputView {
    fn new(key: Key) -> Self {
        Self {
            key,
            run: None,
            first_line: 0,
            lines: Vec::new(),
            follow: Follow::Following,
            offset: 0,
            viewport: 0,
            pending: Some(0),
        }
    }

    /// How many of the run's earliest lines this view does not hold, which its first row says
    /// (FR-021): the daemon no longer kept them, or the view let them go past [`KEEP_LINES`].
    pub fn dropped(&self) -> u64 {
        self.first_line
    }

    /// The index of the next line this view wants: where a watch resumes after a reconnect.
    pub fn next_line(&self) -> u64 {
        self.first_line + self.lines.len() as u64
    }

    /// The rows the list shows: the kept lines, under a first row that counts the dropped ones.
    pub fn rows(&self) -> usize {
        self.lines.len() + usize::from(self.dropped() > 0)
    }

    /// Take a batch of `run` whose first line has index `first_line`, from a daemon that no longer
    /// keeps the run's first `dropped` lines (contract C2). Returns the line to watch again from
    /// when the batch shows the daemon holds lines of a new run this view has not asked for.
    fn take(
        &mut self,
        run: RunId,
        first_line: u64,
        dropped: u64,
        lines: Vec<String>,
    ) -> Option<u64> {
        if let Some(from) = self.pending {
            // The backlog starts at `from`, or at the first line the daemon keeps past it.
            if first_line > from.max(dropped) {
                return None;
            }
            self.pending = None;
        }
        if self.run != Some(run) && first_line > dropped {
            // A watch resumed from a line of the run this view held, answered by a newer run: the
            // daemon still holds that run's lines from `dropped` (M6 review A-F1).
            self.pending = Some(dropped);
            return Some(dropped);
        }
        let end = self.next_line();
        if self.run == Some(run) && (self.first_line..=end).contains(&first_line) {
            // What overlaps the held lines (a watch resumed from earlier) is held already.
            let skip = usize::try_from(end - first_line).unwrap_or(usize::MAX);
            self.lines.extend(lines.into_iter().skip(skip));
        } else {
            // A new run, or a backlog, starts over; a gap means the lines between were dropped.
            self.run = Some(run);
            self.first_line = first_line;
            self.lines = lines;
        }
        let excess = self.lines.len().saturating_sub(KEEP_LINES);
        if excess > 0 {
            self.lines.drain(..excess);
            self.first_line += excess as u64;
        }
        None
    }
}

/// What the shell sends a daemon so the open view gets its output (contract C1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// `SetupOutputWatch` of `key` from line `from_line`.
    Watch { key: Key, from_line: u64 },
    /// `SetupOutputUnwatch` of `key`.
    Unwatch { key: Key },
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
    /// A `SetupOutput` batch from `daemon` (contract C2): `lines` of `run`, the first at index
    /// `first_line`; the daemon no longer keeps the run's first `dropped` lines.
    Output {
        daemon: DaemonId,
        project: PathBuf,
        location: SessionLocation,
        run: RunId,
        first_line: u64,
        lines: Vec<String>,
        dropped: u64,
    },
    /// **Setup output** was chosen for a location: open its view (contract U3, U4).
    ViewOpened(Key),
    /// The view was closed.
    ViewClosed,
    /// Follow the tail, or stop following it.
    FollowToggled,
    /// The view's list scrolled to `offset`, `viewport` pixels of it visible, of `content` pixels
    /// as the list measured them.
    Scrolled {
        offset: u32,
        viewport: u32,
        content: u32,
    },
    /// **Copy output**.
    CopyRequested,
}

/// What the shell does after a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Nothing.
    None,
    /// Post this error notification (contract U5).
    NotifyError(String),
    /// Scroll the view's list to its end.
    ScrollToEnd,
    /// Put this text on the clipboard.
    Copy(String),
    /// Send this watch: the daemon holds lines the view has not asked for (M6 review A-F1).
    Watch(Request),
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
                // The record went away with its worktree, and with it what the view showed.
                state.summaries.remove(&key);
                if state.view.as_ref().is_some_and(|view| view.key == key) {
                    state.view = None;
                }
                return Effect::None;
            };
            let id = summary.run.id();
            let notice = failure_notice(&names, summary.run.state());
            let was_running = state.summaries.insert(key, summary).is_some_and(|before| {
                before.run.id() == id && matches!(before.run.state(), SetupState::Running)
            });
            match notice {
                Some(text) if was_running => Effect::NotifyError(text),
                _ => Effect::None,
            }
        }
        Msg::Output {
            daemon,
            project,
            location,
            run,
            first_line,
            lines,
            dropped,
        } => {
            let key = (daemon, project, location);
            let Some(view) = state.view.as_mut().filter(|view| view.key == key) else {
                return Effect::None;
            };
            match view.take(run, first_line, dropped, lines) {
                Some(from_line) => Effect::Watch(Request::Watch { key, from_line }),
                None => following(view),
            }
        }
        Msg::ViewOpened(key) => {
            // The location shown already keeps its lines and its watch.
            if state.view.as_ref().is_none_or(|view| view.key != key) {
                state.view = Some(OutputView::new(key));
            }
            Effect::None
        }
        Msg::ViewClosed => {
            state.view = None;
            Effect::None
        }
        Msg::FollowToggled => {
            let Some(view) = state.view.as_mut() else {
                return Effect::None;
            };
            view.follow = match view.follow {
                Follow::Following => Follow::Detached {
                    offset: view.offset,
                },
                Follow::Detached { .. } => Follow::Following,
            };
            following(view)
        }
        Msg::Scrolled {
            offset,
            viewport,
            content,
        } => {
            let Some(view) = state.view.as_mut() else {
                return Effect::None;
            };
            // A new viewport height is a relayout (the window resized, the list first shown), not
            // the user scrolling: a following view stays at the end (M6 review V-F1).
            let relayout = view.viewport != viewport;
            view.offset = offset;
            view.viewport = viewport;
            if relayout && view.follow == Follow::Following {
                return Effect::ScrollToEnd;
            }
            // Judged against the content the list measured, so a report from a frame laid out
            // before the latest batch is at its own end (M6 review A-F3).
            (view.follow, _) = view.follow.scrolled(offset, viewport, content);
            Effect::None
        }
        Msg::CopyRequested => state
            .view
            .as_ref()
            .map_or(Effect::None, |view| Effect::Copy(view.lines.join("\n"))),
    }
}

/// Keep the end in view when `view` follows it.
fn following(view: &OutputView) -> Effect {
    match view.follow {
        Follow::Following => Effect::ScrollToEnd,
        Follow::Detached { .. } => Effect::None,
    }
}

/// What the shell sends so the daemons send the output the view shows, and no other (contract
/// C1, U4): an unwatch for the location it watched, and a watch from line 0 for the one the view
/// shows, when they differ. Marks the view's location watched. Run after every message, so a view
/// closed by any route (Escape, the scrim, another dialog) stops its watch.
pub fn sync_watch(state: &mut State) -> Vec<Request> {
    let wanted = state.view.as_ref().map(|view| &view.key);
    if wanted == state.watched.as_ref() {
        return Vec::new();
    }
    let wanted = wanted.cloned();
    let mut requests = Vec::new();
    if let Some(key) = state.watched.take() {
        requests.push(Request::Unwatch { key });
    }
    if let Some(key) = wanted {
        requests.push(Request::Watch {
            key: key.clone(),
            from_line: 0,
        });
        state.watched = Some(key);
    }
    requests
}

/// The watch `daemon` is sent again when it reconnects (FR-018): the open view's location, from
/// the line after the last one held, when the view shows a location on that daemon. Batches
/// before its backlog are not taken.
pub fn rewatch(state: &mut State, daemon: DaemonId) -> Option<Request> {
    let view = state.view.as_mut().filter(|view| view.key.0 == daemon)?;
    let from_line = view.next_line();
    view.pending = Some(from_line);
    Some(Request::Watch {
        key: view.key.clone(),
        from_line,
    })
}

/// Whether a location's menu offers **Setup output** (contract U3): while it has a run record.
pub fn output_offered(summary: Option<&SetupRunSummary>) -> bool {
    summary.is_some()
}

/// Whether the delete confirmation says setup will be stopped first (contract U6): while the
/// worktree's run is running.
pub fn delete_warns_setup(summary: Option<&SetupRunSummary>) -> bool {
    summary.is_some_and(|summary| matches!(summary.run.state(), SetupState::Running))
}

impl State {
    /// The run `daemon` reports for `location` of `project`, if it has a record.
    pub fn summary(
        &self,
        daemon: DaemonId,
        project: &Path,
        location: &SessionLocation,
    ) -> Option<&SetupRunSummary> {
        self.summaries
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

/// What a daemon that just connected is told (contract C3): one `(path, command)` per project its
/// catalog holds, `None` for one without a command, so a command cleared while the daemon was
/// away is cleared there too (M5 review A-F2). A path given twice is sent once.
pub fn on_connect(
    commands: &BTreeMap<String, SetupCommand>,
    held: impl IntoIterator<Item = PathBuf>,
) -> Vec<(PathBuf, Option<SetupCommand>)> {
    held.into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|path| {
            let command = command_for(commands, &path).cloned();
            (path, command)
        })
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

/// Which run record `location` of the active project has, by the daemon its binding names: the
/// key the row's tag, its menu and the delete confirmation read (contract U2, U3, U6). `None` with
/// no project open or no daemon bound.
pub fn key_of(state: &crate::app::State, location: &SessionLocation) -> Option<Key> {
    let project = state.workspace.active.clone()?;
    let binding_key = match location {
        SessionLocation::Worktree(dir) => dir.as_str(),
        SessionLocation::Default => "",
    };
    let micold_core::daemons::Binding::Bound(daemon) = state.daemon_binding(binding_key) else {
        return None;
    };
    Some((daemon, project, location.clone()))
}

/// The run record of `location` in the active project, if it has one.
pub fn summary_of<'a>(
    state: &'a crate::app::State,
    location: &SessionLocation,
) -> Option<&'a SetupRunSummary> {
    let (daemon, project, location) = key_of(state, location)?;
    state.setup.summary(daemon, &project, &location)
}

/// What a notification or the output view calls `project` and `location`: the names the sidebar
/// shows.
pub fn names(state: &crate::app::State, project: &Path, location: &SessionLocation) -> Names {
    let workspace = &state.workspace;
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
            state.worktree_display_name(dir)
        }
        SessionLocation::Worktree(dir) => micold_core::naming::display_name(dir),
    };
    Names {
        project: project_name,
        location,
    }
}

/// The setup output view, as a floating surface (contract U4): open while the view is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetupOutputDialog;

crate::floating_surface!(SetupOutputDialog, id: crate::overlay::SurfaceId::new("setup_output"), layer: Dialog, cancelled_by: crate::app::Message::Setup(Msg::ViewClosed));

impl crate::overlay::registry::Registered for SetupOutputDialog {
    fn open_in(state: &crate::app::State) -> Option<Self> {
        state.setup.view.as_ref().map(|_| SetupOutputDialog)
    }
}
