//! The window's bottom status bar: what it says, and the bar that says it (feature 1124).
//!
//! [`model`] turns the window's state into plain words — render-free, so every claim the bar makes
//! ("Daemon connected · local", "Getting image… 42%") is tested per state without a renderer
//! (research R5). [`view`] hands those words to the shared [`material::StatusBar`].
//!
//! The bar owns no state and sends no message: every item is informational (FR-014).

use iced::Element;
use micold_core::local_time::UtcOffset;
use micold_core::sandbox::lifecycle::SandboxState;
use micold_core::session::SessionLocation;
use micold_core::tokens::Roles;

use crate::app::{Message, State};
use crate::features::connection::ConnectionStatus;
use crate::features::plan_usage::View as UsageView;
use crate::features::sandbox::Sandbox;
use crate::ui::material::{self, GiveWay, ItemTone, StatusItem};

/// One item's words and look.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemModel {
    pub text: String,
    pub dot: Option<ItemTone>,
    pub tone: ItemTone,
    pub busy: bool,
    pub tooltip: Option<String>,
}

impl ItemModel {
    fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            dot: None,
            tone: ItemTone::Neutral,
            busy: false,
            tooltip: None,
        }
    }

    fn item(self, give_way: GiveWay) -> StatusItem {
        let mut item = StatusItem::new(self.text)
            .tone(self.tone)
            .busy(self.busy)
            .give_way(give_way);
        if let Some(dot) = self.dot {
            item = item.dot(dot);
        }
        if let Some(tooltip) = self.tooltip {
            item = item.tooltip(tooltip);
        }
        item
    }
}

/// Everything the bar says (data-model *StatusModel*).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusModel {
    /// The connection to the session service; always present (FR-004).
    pub connection: ItemModel,
    /// The sandbox; always present (FR-005).
    pub sandbox: ItemModel,
    /// The focused session's CLI, while the session view shows (FR-007).
    pub cli: Option<ItemModel>,
    /// The plan usage, with a current reading and the switch on (FR-011).
    pub usage: Option<UsageView>,
    /// The key hints, right-aligned (FR-008); an empty hint is not drawn.
    pub hints: [String; 2],
}

/// What the bar says for `state`, the connection and the sandbox at `now` in the zone `offset`.
pub fn model(
    state: &State,
    connection: &ConnectionStatus,
    sandbox: &Sandbox,
    now: i64,
    offset: UtcOffset,
) -> StatusModel {
    let _ = (now, offset);
    StatusModel {
        connection: connection_item(state, connection),
        sandbox: sandbox_item(sandbox),
        cli: None,
        usage: None,
        hints: [String::new(), String::new()],
    }
}

/// Where the active session's daemon runs (research R6): "local" for the host daemon or with no
/// project open, the daemon's name for another runtime, "no daemon" when unbound.
fn location(state: &State) -> String {
    if state.workspace.active.is_none() {
        return "local".to_string();
    }
    let key = match state.current_session_location() {
        Some(SessionLocation::Worktree(dir)) => dir,
        Some(SessionLocation::Default) | None => String::new(),
    };
    state.daemon_location_of(&key)
}

/// The connection item (data-model *Connection item*).
fn connection_item(state: &State, connection: &ConnectionStatus) -> ItemModel {
    let connected = || format!("Daemon connected · {}", location(state));
    let sentence = || super::connection_words(connection).map(|(_, body)| body);
    let (text, dot, tooltip) = match connection {
        ConnectionStatus::Connected => (connected(), ItemTone::Positive, None),
        ConnectionStatus::Disconnected => (
            "Daemon disconnected · reconnecting…".to_string(),
            ItemTone::Error,
            Some("The displayed content may be stale.".to_string()),
        ),
        ConnectionStatus::Displaced { .. } | ConnectionStatus::ProjectBusy { .. } => (
            "Read-only · another window".to_string(),
            ItemTone::Warning,
            sentence(),
        ),
        ConnectionStatus::VersionMismatch { .. } | ConnectionStatus::BuildMismatch { .. } => {
            (connected(), ItemTone::Warning, sentence())
        }
    };
    ItemModel {
        dot: Some(dot),
        tooltip,
        ..ItemModel::new(text)
    }
}

/// The sandbox item (data-model *Sandbox item*), checked in its stated order: a taken fallback wins
/// over the sandbox's state, as in the banner.
fn sandbox_item(sandbox: &Sandbox) -> ItemModel {
    let warning = |text: &str| ItemModel {
        tone: ItemTone::Warning,
        tooltip: sandbox.persistent_notice(),
        ..ItemModel::new(text)
    };
    let bringing_up = |text: String| ItemModel {
        busy: true,
        tooltip: super::sandbox_status::attempt_line(sandbox).map(|line| match line.detail {
            Some(detail) => format!("{}\n{detail}", line.label),
            None => line.label,
        }),
        ..ItemModel::new(text)
    };
    if sandbox.fallback.is_some() {
        return warning("No sandbox");
    }
    match &sandbox.state {
        SandboxState::Running(_) => ItemModel::new("Sandbox ready"),
        SandboxState::Disabled => ItemModel::new("Sandbox off"),
        SandboxState::Probing => bringing_up("Checking runtime…".to_string()),
        SandboxState::Acquiring(progress) => bringing_up(match progress.percent {
            Some(percent) => format!("Getting image… {percent}%"),
            None => "Getting image…".to_string(),
        }),
        SandboxState::Starting => bringing_up("Starting sandbox…".to_string()),
        SandboxState::Stale(_) => warning("Sandbox out of date"),
        SandboxState::Failed(_) => warning("Sandbox failed"),
    }
}

/// The bar for `model`: connection, sandbox, CLI and usage on the left, key hints on the right,
/// each giving way at narrow widths in the order FR-012 fixes (data-model *GiveWay*).
pub fn view<'a>(model: StatusModel, roles: Roles) -> Element<'a, Message> {
    let mut bar = material::StatusBar::new(roles)
        .item(model.connection.item(GiveWay::Shorten(2)))
        .item(model.sandbox.item(GiveWay::Shorten(1)));
    if let Some(cli) = model.cli {
        bar = bar.item(cli.item(GiveWay::Shorten(0)));
    }
    if let Some(usage) = model.usage {
        bar = bar.item_element(
            material::UsageIndicator::new(usage.label, &roles)
                .warning(usage.warning)
                .details(usage.details),
            GiveWay::Drop(1),
        );
    }
    for hint in model.hints.into_iter().filter(|h| !h.is_empty()) {
        bar = bar.hint(StatusItem::new(hint).give_way(GiveWay::Drop(0)));
    }
    bar.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use micold_core::daemons::{ContainerSettings, DaemonId, DaemonRegistry, DaemonRuntime};
    use micold_core::project::{Availability, Project};
    use micold_core::sandbox::lifecycle::{Failure, Stage};
    use micold_core::sandbox::placement::ConsentedFallback;
    use micold_core::sandbox::runtime::{ContainerId, Progress, RuntimeError, RuntimeKind};
    use micold_core::sandbox::SandboxProfile;
    use micold_core::session::{AiCli, Session};
    use micold_core::worktree::{Worktree, WorktreeStatus};
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    const PROJECT: &str = "/repo";

    fn at(state: &State, connection: &ConnectionStatus, sandbox: &Sandbox) -> StatusModel {
        model(state, connection, sandbox, 0, UtcOffset::UTC)
    }

    fn connection_of(state: &State, connection: ConnectionStatus) -> ItemModel {
        at(state, &connection, &Sandbox::default()).connection
    }

    fn sandbox_of(sandbox: Sandbox) -> ItemModel {
        at(&State::default(), &ConnectionStatus::Connected, &sandbox).sandbox
    }

    /// A project with worktree `feat-a`, a host daemon "Host" and a container daemon "Box", and an
    /// active session at `location`.
    fn project(location: SessionLocation) -> (State, DaemonId, DaemonId) {
        let mut state = State::default();
        let path = PathBuf::from(PROJECT);
        state.workspace.projects.push(Project {
            path: path.clone(),
            display_name: "repo".to_string(),
            is_git_repo: true,
            availability: Availability::Available,
        });
        state.workspace.active = Some(path.clone());
        state.worktree.worktrees = vec![Worktree {
            dir_name: "feat-a".to_string(),
            path: PathBuf::from(format!("{PROJECT}/.claude/worktrees/feat-a")),
            branch: Some("feat/a".to_string()),
            status: WorktreeStatus::Valid,
            included: false,
        }];
        let mut registry = DaemonRegistry::new(Vec::new(), 1);
        let host = registry.add("Host", DaemonRuntime::Host, true).unwrap();
        let boxed = registry
            .add(
                "Box",
                DaemonRuntime::Container(ContainerSettings {
                    profile: SandboxProfile::default(),
                    container_name: "box".into(),
                    port: 7001,
                    migration: None,
                }),
                true,
            )
            .unwrap();
        state.settings.daemons = registry;
        let session = Session::start_new(location, AiCli::ClaudeCode);
        state.session.active = Some(session.id);
        state.workspace.sessions.insert(path, vec![session]);
        (state, host, boxed)
    }

    fn bind(state: &mut State, pairs: &[(&str, DaemonId)]) {
        let map: BTreeMap<String, DaemonId> =
            pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        state.workspace.bindings.insert(PathBuf::from(PROJECT), map);
    }

    fn connected_text(state: &State) -> String {
        connection_of(state, ConnectionStatus::Connected).text
    }

    #[test]
    fn connected_with_no_project_open_names_local() {
        let item = connection_of(&State::default(), ConnectionStatus::Connected);
        assert_eq!(item.text, "Daemon connected · local");
        assert_eq!(item.dot, Some(ItemTone::Positive));
        assert_eq!(item.tooltip, None);
    }

    #[test]
    fn a_host_daemon_is_local_and_another_runtime_is_named() {
        let (mut state, host, boxed) = project(SessionLocation::Worktree("feat-a".to_string()));
        bind(&mut state, &[("feat-a", host)]);
        assert_eq!(connected_text(&state), "Daemon connected · local");
        bind(&mut state, &[("feat-a", boxed)]);
        assert_eq!(connected_text(&state), "Daemon connected · Box");
    }

    /// The Default location's key is `""`, and so is a session whose location is unknown.
    #[test]
    fn the_default_location_and_no_location_use_the_default_key() {
        let (mut state, _, boxed) = project(SessionLocation::Default);
        bind(&mut state, &[("", boxed)]);
        assert_eq!(connected_text(&state), "Daemon connected · Box");
        state.session.active = None;
        assert_eq!(connected_text(&state), "Daemon connected · Box");
    }

    #[test]
    fn an_unbound_or_missing_daemon_says_no_daemon() {
        let (mut state, _, _) = project(SessionLocation::Worktree("feat-a".to_string()));
        state.settings.legacy_default_daemon = None;
        bind(&mut state, &[("feat-a", DaemonId(999))]);
        assert_eq!(connected_text(&state), "Daemon connected · no daemon");
    }

    #[test]
    fn disconnected_is_an_error_dot_with_the_stale_warning() {
        let item = connection_of(&State::default(), ConnectionStatus::Disconnected);
        assert_eq!(item.text, "Daemon disconnected · reconnecting…");
        assert_eq!(item.dot, Some(ItemTone::Error));
        assert_eq!(
            item.tooltip.as_deref(),
            Some("The displayed content may be stale.")
        );
    }

    #[test]
    fn read_only_states_are_a_warning_with_the_banner_sentence() {
        for status in [
            ConnectionStatus::Displaced { by: "w2".into() },
            ConnectionStatus::ProjectBusy {
                holder: "w2".into(),
            },
        ] {
            let item = connection_of(&State::default(), status.clone());
            assert_eq!(item.text, "Read-only · another window");
            assert_eq!(item.dot, Some(ItemTone::Warning));
            let sentence = super::super::connection_words(&status).unwrap().1;
            assert_eq!(item.tooltip, Some(sentence));
        }
    }

    #[test]
    fn mismatches_stay_connected_with_a_warning_dot_and_the_sentence() {
        for status in [
            ConnectionStatus::VersionMismatch {
                client: 2,
                daemon: 1,
                daemon_build: "b1".into(),
            },
            ConnectionStatus::BuildMismatch {
                client_build: "b2".into(),
                daemon_build: "b1".into(),
            },
        ] {
            let item = connection_of(&State::default(), status.clone());
            assert_eq!(item.text, "Daemon connected · local");
            assert_eq!(item.dot, Some(ItemTone::Warning));
            let sentence = super::super::connection_words(&status).unwrap().1;
            assert_eq!(item.tooltip, Some(sentence));
        }
    }

    fn failure() -> Failure {
        Failure {
            stage: Stage::Probing,
            error: RuntimeError::NotInstalled {
                kind: RuntimeKind::Docker,
            },
        }
    }

    fn sandbox(state: SandboxState) -> Sandbox {
        Sandbox {
            state,
            ..Sandbox::default()
        }
    }

    fn progress(percent: Option<u8>) -> Progress {
        Progress {
            stage: "Downloading".into(),
            detail: Some("layer 3".into()),
            percent,
        }
    }

    #[test]
    fn steady_sandbox_states_are_neutral_and_quiet() {
        for (state, text) in [
            (
                SandboxState::Running(ContainerId("c".into())),
                "Sandbox ready",
            ),
            (SandboxState::Disabled, "Sandbox off"),
        ] {
            let item = sandbox_of(sandbox(state));
            assert_eq!(item, ItemModel::new(text));
        }
    }

    #[test]
    fn bring_up_stages_are_busy_with_the_attempt_line_as_tooltip() {
        for (state, text) in [
            (SandboxState::Probing, "Checking runtime…"),
            (
                SandboxState::Acquiring(progress(Some(42))),
                "Getting image… 42%",
            ),
            (SandboxState::Acquiring(progress(None)), "Getting image…"),
            (SandboxState::Starting, "Starting sandbox…"),
        ] {
            let s = sandbox(state);
            let line = super::super::sandbox_status::attempt_line(&s).unwrap();
            let expected = match line.detail {
                Some(d) => format!("{}\n{d}", line.label),
                None => line.label,
            };
            let item = sandbox_of(s);
            assert_eq!(item.text, text);
            assert_eq!(item.tone, ItemTone::Neutral);
            assert!(item.busy, "{text} is busy");
            assert_eq!(item.tooltip, Some(expected));
        }
    }

    /// FR-006: the runtime's line and the retry reason both reach the tooltip.
    #[test]
    fn a_retry_reason_reaches_the_tooltip_on_its_own_line() {
        let mut s = sandbox(SandboxState::Acquiring(progress(Some(42))));
        s.previous_attempt = Some(failure());
        let tooltip = sandbox_of(s).tooltip.unwrap();
        let (label, detail) = tooltip.split_once('\n').expect("two lines");
        assert_eq!(label, "Getting the sandbox image");
        assert!(detail.contains("Downloading 42%"), "{detail}");
        assert!(detail.contains("Trying again:"), "{detail}");
    }

    #[test]
    fn standing_conditions_are_warning_words_with_the_notice() {
        let stale = sandbox(SandboxState::Stale(ContainerId("c".into())));
        let failed = sandbox(SandboxState::Failed(failure()));
        for (s, text) in [(stale, "Sandbox out of date"), (failed, "Sandbox failed")] {
            let notice = s.persistent_notice();
            let item = sandbox_of(s);
            assert_eq!(item.text, text);
            assert_eq!(item.tone, ItemTone::Warning);
            assert!(!item.busy);
            assert!(notice.is_some());
            assert_eq!(item.tooltip, notice);
        }
    }

    /// A taken fallback wins over whatever the sandbox's state is.
    #[test]
    fn a_taken_fallback_wins_over_the_state() {
        let mut s = sandbox(SandboxState::Running(ContainerId("c".into())));
        s.fallback = Some(ConsentedFallback {
            because: "no runtime".into(),
        });
        let notice = s.persistent_notice();
        let item = sandbox_of(s);
        assert_eq!(item.text, "No sandbox");
        assert_eq!(item.tone, ItemTone::Warning);
        assert_eq!(item.tooltip, notice);
    }

    /// Until US4/US5, the bar carries only the connection and the sandbox.
    #[test]
    fn cli_usage_and_hints_are_empty_for_now() {
        let m = at(
            &State::default(),
            &ConnectionStatus::Connected,
            &Sandbox::default(),
        );
        assert_eq!(m.cli, None);
        assert_eq!(m.usage, None);
        assert!(m.hints.iter().all(String::is_empty));
    }
}
