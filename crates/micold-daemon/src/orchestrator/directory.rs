//! The link directory (FR-007, research R9): what the client API asks of the daemons, behind a
//! trait so a later scale-out (FR-046) replaces only the implementation. [`InProcess`] answers from
//! this orchestrator's own links: the list is `list::gather`, a session's content a one-shot
//! `ReadStructuredEvents` conversation on the owning daemon's link.

use std::sync::{Arc, Mutex};

use futures_util::future::{BoxFuture, FutureExt};
use micold_core::client_api::content::entries;
use micold_core::client_api::messages::{
    ContentEntry, Daemon, DaemonStatus, ErrorCode, Location, Project, Session, SessionKind,
    SessionState, Snapshot,
};
use micold_core::orchestrator::{DaemonRecord, Health};
use micold_core::protocol::messages::{
    ActivitySignal, ClientMsg, DaemonMsg, ListRows, ProjectSnapshot, SessionSummary,
    StructuredReadRefusal, WireLifecycle,
};
use micold_core::protocol::routing::{ListKind, MissingReason, WireDaemonId};

use super::list::{self, LIST_DEADLINE};
use super::Shared;

/// A request the directory could not answer, as the client sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError {
    /// The typed code.
    pub code: ErrorCode,
    /// For logs.
    pub message: String,
}

impl ApiError {
    /// `code` with `message`.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// What the client API asks of the daemons.
pub trait LinkDirectory: Send + Sync {
    /// Every daemon, project, location and session, in the stable order (epoch and seq 0).
    fn list(&self) -> BoxFuture<'_, Snapshot>;
    /// One structured session's content.
    fn read_session<'a>(
        &'a self,
        daemon: &'a str,
        project: &'a str,
        session: &'a str,
    ) -> BoxFuture<'a, Result<Vec<ContentEntry>, ApiError>>;
    /// Resolves when a daemon linked or left since the last call (the feed polls at once).
    fn changed(&self) -> BoxFuture<'_, ()>;
}

/// The directory of this orchestrator's own links.
pub struct InProcess {
    shared: Arc<Shared>,
    /// The last list, so a daemon that goes offline keeps its rows, marked unreachable (FR-014a).
    last: Mutex<Snapshot>,
}

impl InProcess {
    /// The directory over `shared`.
    pub fn new(shared: Arc<Shared>) -> Self {
        Self {
            shared,
            last: Mutex::default(),
        }
    }

    async fn read(
        &self,
        daemon: &str,
        project: &str,
        session: &str,
    ) -> Result<Vec<ContentEntry>, ApiError> {
        let not_found = || {
            ApiError::new(
                ErrorCode::NotFound,
                format!("no session {daemon}/{project}/{session}"),
            )
        };
        let id = WireDaemonId::new(daemon).map_err(|_| not_found())?;
        let health = self
            .shared
            .registry()
            .list()
            .into_iter()
            .find(|r| r.id == id)
            .map(|r| r.health)
            .ok_or_else(not_found)?;
        if health != Health::Online {
            return Err(missing(MissingReason::Offline));
        }
        let rows = tokio::time::timeout(
            LIST_DEADLINE,
            list::ask(&self.shared, &id, ListKind::Projects),
        )
        .await
        .unwrap_or(Err(MissingReason::Timeout))
        .map_err(missing)?;
        let ListRows::Projects(projects) = rows else {
            return Err(not_found());
        };
        let found = projects
            .into_iter()
            .find(|p| project_id(p) == project)
            .ok_or_else(not_found)?;
        let summary = found
            .sessions
            .iter()
            .find(|s| s.id.to_string() == session)
            .ok_or_else(not_found)?;
        if summary.kind.is_terminal() {
            return Err(terminal());
        }
        let request = ClientMsg::ReadStructuredEvents {
            project: found.path.clone(),
            session: summary.id,
        };
        let answer = list::converse(&self.shared, &id, Some(request), |msg| match msg {
            DaemonMsg::StructuredEvents { events, .. } => Some(Ok(events)),
            DaemonMsg::StructuredEventsRefused { reason, .. } => Some(Err(reason)),
            _ => None,
        });
        match tokio::time::timeout(LIST_DEADLINE, answer).await {
            Err(_) => Err(missing(MissingReason::Timeout)),
            Ok(Err(reason)) => Err(missing(reason)),
            Ok(Ok(Err(StructuredReadRefusal::TerminalSession))) => Err(terminal()),
            Ok(Ok(Err(StructuredReadRefusal::NotFound))) => Err(not_found()),
            Ok(Ok(Ok(events))) => Ok(entries(&events)),
        }
    }
}

impl LinkDirectory for InProcess {
    fn list(&self) -> BoxFuture<'_, Snapshot> {
        async move {
            let records = self.shared.registry().list();
            let merged = list::gather(&self.shared, ListKind::Projects).await;
            let answered = match merged.rows {
                ListRows::Projects(rows) => rows,
                _ => Vec::new(),
            };
            let mut last = self
                .last
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *last = snapshot_of(&records, &answered, &last);
            last.clone()
        }
        .boxed()
    }

    fn read_session<'a>(
        &'a self,
        daemon: &'a str,
        project: &'a str,
        session: &'a str,
    ) -> BoxFuture<'a, Result<Vec<ContentEntry>, ApiError>> {
        self.read(daemon, project, session).boxed()
    }

    fn changed(&self) -> BoxFuture<'_, ()> {
        self.shared.changed().boxed()
    }
}

fn terminal() -> ApiError {
    ApiError::new(
        ErrorCode::TerminalNotSupported,
        "terminal output is not offered on this API",
    )
}

/// Why a daemon gave nothing, as the client sees it.
fn missing(reason: MissingReason) -> ApiError {
    let text = reason.to_string();
    let code = match reason {
        MissingReason::Offline => ErrorCode::DaemonUnreachable,
        MissingReason::Timeout => ErrorCode::Timeout,
        MissingReason::NeedsUpdate => ErrorCode::Refused,
        MissingReason::Error(e) if e.contains("ProjectBusy") => ErrorCode::ProjectBusy,
        MissingReason::Error(_) => ErrorCode::Refused,
    };
    ApiError::new(code, text)
}

/// A project's id on the wire: unique across daemons.
pub fn project_id(project: &ProjectSnapshot) -> String {
    format!("{}:{}", project.daemon, project.identity)
}

/// A location's id: the project's, then `#` and the worktree's directory (none for the root).
fn location_id(project: &str, worktree: Option<&str>) -> String {
    format!("{project}#{}", worktree.unwrap_or(""))
}

fn status(health: Health) -> DaemonStatus {
    match health {
        Health::Online => DaemonStatus::Online,
        Health::Offline => DaemonStatus::Offline,
        Health::NeedsUpdate => DaemonStatus::NeedsUpdate,
    }
}

/// What a session is doing, from its lifecycle and activity.
fn state(summary: &SessionSummary) -> SessionState {
    match (&summary.lifecycle, &summary.activity) {
        (WireLifecycle::Failed { .. }, _) => SessionState::Error,
        (WireLifecycle::Idle, _) | (_, ActivitySignal::Ended { .. }) => SessionState::Stopped,
        (_, ActivitySignal::Working) => SessionState::Working,
        (_, ActivitySignal::AwaitingInput) => SessionState::Waiting,
        _ => SessionState::Idle,
    }
}

/// The snapshot of `records` with the projects the online daemons `answered`. A daemon that gave
/// no rows keeps the ones it had in `previous`, its sessions marked unreachable (FR-014a).
pub fn snapshot_of(
    records: &[DaemonRecord],
    answered: &[ProjectSnapshot],
    previous: &Snapshot,
) -> Snapshot {
    let mut records: Vec<&DaemonRecord> = records.iter().collect();
    records.sort_by(|a, b| a.id.cmp(&b.id));
    let mut out = Snapshot::default();
    for record in records {
        let daemon_id = record.id.to_string();
        out.daemons.push(Daemon {
            id: daemon_id.clone(),
            name: record.name.clone(),
            status: status(record.health),
        });
        let mine: Vec<&ProjectSnapshot> =
            answered.iter().filter(|p| p.daemon == record.id).collect();
        if mine.is_empty() || record.health != Health::Online {
            keep_previous(&mut out, previous, &daemon_id);
            continue;
        }
        for project in mine {
            add_project(&mut out, project, &daemon_id);
        }
    }
    out
}

fn add_project(out: &mut Snapshot, project: &ProjectSnapshot, daemon_id: &str) {
    let id = project_id(project);
    out.projects.push(Project {
        id: id.clone(),
        daemon_id: daemon_id.to_owned(),
        name: project.display_name.clone(),
    });
    out.locations.push(Location {
        id: location_id(&id, None),
        project_id: id.clone(),
        name: project.display_name.clone(),
        branch: String::new(),
        is_default: true,
    });
    for worktree in &project.worktrees {
        out.locations.push(Location {
            id: location_id(&id, Some(&worktree.dir_name)),
            project_id: id.clone(),
            name: worktree.display_name.clone(),
            branch: worktree.branch.clone().unwrap_or_default(),
            is_default: false,
        });
    }
    for session in &project.sessions {
        out.sessions.push(Session {
            id: session.id.to_string(),
            location_id: location_id(&id, session.worktree_dir.as_deref()),
            name: session.title.display().to_owned(),
            kind: if session.kind.is_terminal() {
                SessionKind::Terminal
            } else {
                SessionKind::Structured
            },
            state: state(session),
            reachable: true,
        });
    }
}

/// The rows `daemon_id` had in `previous`, its sessions unreachable.
fn keep_previous(out: &mut Snapshot, previous: &Snapshot, daemon_id: &str) {
    let projects: Vec<_> = previous
        .projects
        .iter()
        .filter(|p| p.daemon_id == daemon_id)
        .cloned()
        .collect();
    let locations: Vec<_> = previous
        .locations
        .iter()
        .filter(|l| projects.iter().any(|p| p.id == l.project_id))
        .cloned()
        .collect();
    out.sessions.extend(
        previous
            .sessions
            .iter()
            .filter(|s| locations.iter().any(|l| l.id == s.location_id))
            .map(|s| Session {
                reachable: false,
                ..s.clone()
            }),
    );
    out.projects.extend(projects);
    out.locations.extend(locations);
}
