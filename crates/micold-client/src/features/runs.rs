//! Run in parallel: the dialog that starts a group of prompted runs, and the project's group list
//! (feature 483, contracts/parallel-surfaces.md D and G).
//!
//! Render-free. The dialog is opened from the sidebar and sends one `RunGroupCreate`; the service
//! then pushes the whole group list (`RunGroupsChanged`), which [`Msg::GroupsChanged`] stores for
//! the sidebar to project. The shell turns [`Effect::Send`] into a correlated request.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::PathBuf;

use crate::app::Message;
use crate::features::changes::Load;
use crate::features::Outcome;
use crate::overlay::registry::Registered;
use crate::overlay::{DismissalRules, FloatingSurface, SurfaceId};
use micold_core::naming::{derive, ConventionalType, DerivedNames, NamingError, WorktreeNaming};
use micold_core::overlay::Layer;
use micold_core::protocol::messages::ActivitySignal;
use micold_core::protocol::messages::ClientMsg;
use micold_core::runs::naming::derive_group;
use micold_core::runs::{
    GroupId, RunGroup, RunStatus, RunSummary, DEFAULT_RUNS, MAX_RUNS, MIN_RUNS,
};
use micold_core::session::{AiCli, Session, SessionLocation};
use micold_core::worktree::{BlockReason, BranchCandidate, BranchOrigin};

/// What this feature remembers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// The Run in parallel dialog; present only while it is shown.
    pub dialog: Option<ParallelDialog>,
    /// Every run group of the active project, oldest first (the last `RunGroupsChanged`).
    pub groups: Vec<RunGroup>,
    /// Groups the user collapsed in the sidebar (G2); every other group is expanded.
    pub collapsed: BTreeSet<GroupId>,
    /// The group row's open right-click menu (G4).
    pub menu: Option<GroupMenu>,
    /// The group awaiting the Dismiss group confirmation (G5); present only while it is shown.
    pub dismiss_target: Option<DismissTarget>,
    /// The open Compare view (C1–C5); present only while it is shown.
    pub compare: Option<CompareView>,
    /// The next summary read's sequence number.
    pub next_seq: u64,
    /// The request the root left for the shell while it interpreted an outcome (opening the
    /// dialog), taken once right after the message that caused it.
    pub pending: Option<Effect>,
}

/// The open Compare view: which group, and each run's change counts as last read (data-model §
/// Client state).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompareView {
    /// The group shown.
    pub group: GroupId,
    /// The project that owns it.
    pub project: PathBuf,
    /// Each run's counts by run number; a run with no worktree has no entry.
    pub reads: BTreeMap<u8, Load<RunSummary>>,
}

/// One summary read the shell must run (T045).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryRead {
    /// The read's sequence number; the answer carries it back.
    pub seq: u64,
    /// The run read.
    pub run: u8,
    /// The run's worktree folder name.
    pub dir_name: String,
    /// The group's base branch the counts are against.
    pub base_branch: String,
}

impl State {
    /// Whether the group's row shows its runs.
    pub fn is_expanded(&self, id: GroupId) -> bool {
        !self.collapsed.contains(&id)
    }
}

/// A group row's right-click menu: which group, and where the press landed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupMenu {
    /// The group the menu acts on.
    pub group: GroupId,
    /// The press point in window pixels.
    pub anchor: (u16, u16),
}

/// The group a Dismiss group confirmation is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DismissTarget {
    /// The group.
    pub group: GroupId,
    /// The project that owns it.
    pub project: PathBuf,
}

/// The entries of a group row's menu, in order (G4).
pub const GROUP_MENU_ITEMS: [&str; 2] = ["Compare", "Dismiss group"];

/// What the Dismiss group confirmation says is left alone (G5, W4).
pub const DISMISS_CONFIRMATION: &str = "This forgets the grouping only. The worktrees, branches \
and sessions of its runs stay exactly as they are.";

/// What the dialog opens with, gathered by the root from the rest of the state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opening {
    /// The project the group is started in.
    pub project: PathBuf,
    /// The user's default AI CLI: what every new run starts on (FR-002).
    pub default_cli: AiCli,
    /// The CLIs available where sessions run: the only providers a run may pick (FR-002).
    pub offered: Vec<AiCli>,
    /// The project root's current branch when the client already knows it.
    pub base_branch: Option<String>,
}

/// The open dialog (D1–D4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParallelDialog {
    /// The project the group is started in.
    pub project: PathBuf,
    /// Type, optional ticket and name the runs' names are derived from.
    pub naming: WorktreeNaming,
    /// The prompt every run receives.
    pub prompt: String,
    /// The local branch every run starts from; empty until the branch listing names one.
    pub base_branch: String,
    /// The local branches the base-branch select offers.
    pub branches: Vec<String>,
    /// One provider per run, in run order.
    pub runs: Vec<AiCli>,
    /// The providers a run may pick.
    pub offered: Vec<AiCli>,
    /// What a new run starts on.
    pub default_cli: AiCli,
    /// The first failing rule, shown once the user has changed something or pressed confirm.
    pub error: Option<String>,
}

/// Why the dialog cannot be confirmed, in the order the rules are checked (D4, US1 s5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Invalid {
    /// The prompt is empty.
    EmptyPrompt,
    /// No type is chosen.
    NoType,
    /// The name is empty.
    EmptyName,
    /// The run count is outside `MIN_RUNS..=MAX_RUNS`.
    RunCount,
    /// A run's provider is not available where sessions run.
    ProviderNotOffered(AiCli),
    /// The derived branch or folder name is not valid.
    BadName(NamingError),
    /// No base branch is chosen.
    NoBaseBranch,
}

impl fmt::Display for Invalid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Invalid::EmptyPrompt => f.write_str("Enter a prompt"),
            Invalid::NoType => f.write_str("Select a type"),
            Invalid::EmptyName => f.write_str("Enter a name"),
            Invalid::RunCount => {
                write!(f, "Choose between {MIN_RUNS} and {MAX_RUNS} runs")
            }
            Invalid::ProviderNotOffered(cli) => {
                write!(
                    f,
                    "{} is not available where sessions run",
                    cli.provider().display_name()
                )
            }
            Invalid::BadName(error) => write!(f, "{error}"),
            Invalid::NoBaseBranch => f.write_str("Choose a base branch"),
        }
    }
}

impl ParallelDialog {
    fn new(opening: Opening) -> Self {
        ParallelDialog {
            project: opening.project,
            naming: WorktreeNaming {
                type_: None,
                ticket: None,
                name: String::new(),
            },
            prompt: String::new(),
            base_branch: opening.base_branch.unwrap_or_default(),
            branches: Vec::new(),
            runs: vec![opening.default_cli; DEFAULT_RUNS],
            offered: opening.offered,
            default_cli: opening.default_cli,
            error: None,
        }
    }

    /// The names each run will get, in order (D3, FR-003).
    pub fn derived_names(&self) -> Result<Vec<DerivedNames>, NamingError> {
        derive_group(&self.naming, self.runs.len() as u8)
    }

    /// The first rule the dialog breaks, or `Ok` when it can be confirmed.
    pub fn validate(&self) -> Result<(), Invalid> {
        if self.prompt.trim().is_empty() {
            return Err(Invalid::EmptyPrompt);
        }
        if self.naming.type_.is_none() {
            return Err(Invalid::NoType);
        }
        if self.naming.name.trim().is_empty() {
            return Err(Invalid::EmptyName);
        }
        if !(MIN_RUNS..=MAX_RUNS).contains(&self.runs.len()) {
            return Err(Invalid::RunCount);
        }
        if let Some(cli) = self.runs.iter().find(|cli| !self.offered.contains(cli)) {
            return Err(Invalid::ProviderNotOffered(*cli));
        }
        // The group's own name first, as `derive_group` does, then every run's.
        derive(&self.naming).map_err(Invalid::BadName)?;
        self.derived_names().map_err(Invalid::BadName)?;
        if self.base_branch.trim().is_empty() {
            return Err(Invalid::NoBaseBranch);
        }
        Ok(())
    }

    fn revalidate(&mut self) {
        self.error = self.validate().err().map(|invalid| invalid.to_string());
    }
}

/// A change to this feature's state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// **Run in parallel** was chosen: open the dialog.
    Opened(Opening),
    /// The prompt was edited.
    PromptChanged(String),
    /// A type was chosen.
    TypeChanged(ConventionalType),
    /// The ticket was edited.
    TicketChanged(String),
    /// The name was edited.
    NameChanged(String),
    /// A base branch was chosen.
    BaseBranchChanged(String),
    /// Run `index`'s provider was chosen.
    ProviderChanged(usize, AiCli),
    /// **Add run**.
    RunAdded,
    /// Run `index`'s remove button.
    RunRemoved(usize),
    /// The branch listing the dialog asked for arrived.
    BranchesListed(Vec<BranchCandidate>),
    /// **Start runs**.
    Confirmed,
    /// The dialog was dismissed.
    Dismissed,
    /// The project's whole group list arrived (`RunGroupsChanged`).
    GroupsChanged(Vec<RunGroup>),
    /// A group row's chevron.
    GroupToggled(GroupId),
    /// Open (or close, when already open on it) the group row's menu at the press point.
    MenuToggled(GroupId, (u16, u16)),
    /// The group row's menu was dismissed.
    MenuDismissed,
    /// **Compare** was picked: open the group's Compare view (C1).
    CompareOpened {
        /// The group.
        group: GroupId,
        /// The project that owns it.
        project: PathBuf,
    },
    /// Compare's close action.
    CompareClosed,
    /// Summary read `seq` of run `run` ended.
    SummaryRead {
        /// The read's sequence number.
        seq: u64,
        /// The run read.
        run: u8,
        /// Its counts, or why git could not give them.
        result: Result<RunSummary, String>,
    },
    /// Files changed in run `run`'s worktree (the watch): read its counts again (C5).
    RunChanged {
        /// The run whose worktree changed.
        run: u8,
    },
    /// **Open diff** on run `run`'s row (C4).
    DiffOpened {
        /// The run.
        run: u8,
    },
    /// **Dismiss group** was picked: ask first.
    DismissAsked {
        /// The group.
        group: GroupId,
        /// The project that owns it.
        project: PathBuf,
    },
    /// The dismiss confirmation was confirmed.
    DismissConfirmed,
    /// The dismiss confirmation was cancelled.
    DismissCancelled,
}

/// What the shell must do after a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Nothing.
    None,
    /// Send this request; its `req` is assigned by the shell.
    Send(ClientMsg),
    /// Read these runs' change counts off the UI thread; each answers [`Msg::SummaryRead`] with its
    /// own `seq` (the read is the shell's, never `update`'s).
    ReadSummaries(Vec<SummaryRead>),
}

/// Apply a message.
pub fn update(state: &mut State, msg: Msg) -> Effect {
    match msg {
        Msg::Opened(opening) => {
            let project = opening.project.clone();
            state.dialog = Some(ParallelDialog::new(opening));
            return Effect::Send(ClientMsg::BranchList { req: 0, project });
        }
        Msg::Dismissed => state.dialog = None,
        Msg::Confirmed => return confirmed(state),
        Msg::GroupsChanged(groups) => {
            state
                .collapsed
                .retain(|id| groups.iter().any(|group| group.id == *id));
            state.groups = groups;
            let gone = |id: GroupId| !state.groups.iter().any(|group| group.id == id);
            if state.menu.is_some_and(|menu| gone(menu.group)) {
                state.menu = None;
            }
            if state.dismiss_target.as_ref().is_some_and(|t| gone(t.group)) {
                state.dismiss_target = None;
            }
            return compare_groups_changed(state);
        }
        Msg::CompareOpened { group, project } => {
            state.menu = None;
            return compare_opened(state, group, project);
        }
        Msg::CompareClosed => state.compare = None,
        Msg::SummaryRead { seq, run, result } => return summary_read(state, seq, run, result),
        Msg::RunChanged { run } => return request_read(state, run),
        // The root turns it into an outcome (`open_diff`); nothing to remember here.
        Msg::DiffOpened { .. } => {}
        Msg::MenuToggled(group, anchor) => {
            state.menu = match state.menu {
                Some(open) if open.group == group => None,
                _ => Some(GroupMenu { group, anchor }),
            };
        }
        Msg::MenuDismissed => state.menu = None,
        Msg::DismissAsked { group, project } => {
            state.menu = None;
            state.dismiss_target = Some(DismissTarget { group, project });
        }
        Msg::DismissCancelled => state.dismiss_target = None,
        Msg::DismissConfirmed => {
            if let Some(target) = state.dismiss_target.take() {
                return Effect::Send(ClientMsg::RunGroupDismiss {
                    req: 0,
                    project: target.project,
                    group: target.group,
                });
            }
        }
        Msg::GroupToggled(id) => {
            if !state.collapsed.remove(&id) {
                state.collapsed.insert(id);
            }
        }
        Msg::BranchesListed(candidates) => {
            if let Some(dialog) = &mut state.dialog {
                branches_listed(dialog, candidates);
            }
        }
        edit => {
            if let Some(dialog) = &mut state.dialog {
                edited(dialog, edit);
            }
        }
    }
    Effect::None
}

/// **Run in parallel** was chosen: open the dialog over the active project and leave its branch
/// listing request for the shell.
pub fn requested(state: &mut crate::app::State) -> Vec<Outcome> {
    let project = state.workspace.active.clone().unwrap_or_default();
    let opening = Opening {
        offered: state.session.offered_providers(Some(project.as_path())),
        default_cli: state.session.default_ai_cli,
        base_branch: None,
        project,
    };
    state.clear_for_dialog();
    state.runs.pending = Some(update(&mut state.runs, Msg::Opened(opening)));
    vec![Outcome::SurfaceOpened(ParallelRunDialog.id())]
}

/// Apply a message and leave the request it makes in [`State::pending`] for the shell.
pub fn apply(state: &mut State, msg: Msg) {
    let effect = update(state, msg);
    if effect != Effect::None {
        state.pending = Some(effect);
    }
}

/// Open Compare on `group`: one read per run that has a worktree (C5, US3 s1).
fn compare_opened(state: &mut State, group: GroupId, project: PathBuf) -> Effect {
    if !state.groups.iter().any(|g| g.id == group) {
        return Effect::None;
    }
    state.compare = Some(CompareView {
        group,
        project,
        reads: BTreeMap::new(),
    });
    request_missing(state)
}

/// The group list changed while Compare is open: close it when its group is gone, drop the counts
/// of runs that left, and read runs that gained a worktree (US3 s3: status follows the catalog).
fn compare_groups_changed(state: &mut State) -> Effect {
    let Some(view) = &mut state.compare else {
        return Effect::None;
    };
    let Some(group) = state.groups.iter().find(|g| g.id == view.group) else {
        state.compare = None;
        return Effect::None;
    };
    view.reads
        .retain(|number, _| group.runs.iter().any(|run| run.number == *number));
    request_missing(state)
}

/// Start a read for every run with a worktree and no counts yet.
fn request_missing(state: &mut State) -> Effect {
    let Some(view) = &state.compare else {
        return Effect::None;
    };
    let wanted: Vec<u8> = state
        .groups
        .iter()
        .find(|g| g.id == view.group)
        .into_iter()
        .flat_map(|g| &g.runs)
        .filter(|run| run.status.has_worktree() && !view.reads.contains_key(&run.number))
        .map(|run| run.number)
        .collect();
    let reads: Vec<SummaryRead> = wanted
        .into_iter()
        .filter_map(|run| start_read(state, run))
        .collect();
    if reads.is_empty() {
        Effect::None
    } else {
        Effect::ReadSummaries(reads)
    }
}

/// Read `run` again: now, or once the read under way ends (refreshes coalesce).
fn request_read(state: &mut State, run: u8) -> Effect {
    match start_read(state, run) {
        Some(read) => Effect::ReadSummaries(vec![read]),
        None => Effect::None,
    }
}

fn start_read(state: &mut State, number: u8) -> Option<SummaryRead> {
    let view = state.compare.as_mut()?;
    let group = state.groups.iter().find(|g| g.id == view.group)?;
    let run = group.runs.iter().find(|r| r.number == number)?;
    if !run.status.has_worktree() {
        return None;
    }
    if let Some(Load::Loading { again, .. }) = view.reads.get_mut(&number) {
        *again = true;
        return None;
    }
    let seq = state.next_seq;
    state.next_seq += 1;
    let last = match view.reads.remove(&number) {
        Some(Load::Ready(summary)) => Some(summary),
        _ => None,
    };
    view.reads.insert(
        number,
        Load::Loading {
            seq,
            again: false,
            last,
        },
    );
    Some(SummaryRead {
        seq,
        run: number,
        dir_name: run.names.dir_name.clone(),
        base_branch: group.base_branch.clone(),
    })
}

/// Read `seq` ended: show it unless a newer read replaced it, then run a queued read.
fn summary_read(
    state: &mut State,
    seq: u64,
    run: u8,
    result: Result<RunSummary, String>,
) -> Effect {
    let Some(view) = &mut state.compare else {
        return Effect::None;
    };
    let again = match view.reads.get(&run) {
        Some(Load::Loading { seq: s, again, .. }) if *s == seq => *again,
        _ => return Effect::None,
    };
    view.reads.insert(
        run,
        match result {
            Ok(summary) => Load::Ready(summary),
            Err(message) => Load::Failed(message),
        },
    );
    if again {
        request_read(state, run)
    } else {
        Effect::None
    }
}

/// What a run's change counts say in its Compare row (C2, C3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunCounts {
    /// The run has no worktree to count.
    None,
    /// The first read is under way.
    Loading,
    /// The counts.
    Ready(RunSummary),
    /// Git could not be read, with its message.
    Failed(String),
}

/// One row of the Compare view (C2–C5), ready to draw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompareRow {
    /// The run's number.
    pub number: u8,
    /// Its provider.
    pub provider: AiCli,
    /// The status text (C2).
    pub status: &'static str,
    /// Why the run failed or its prompt was not delivered, shown in place of the counts (C3).
    pub reason: Option<String>,
    /// The change counts.
    pub counts: RunCounts,
    /// Whether the row offers **Open diff**: the run has a worktree (C3, C4).
    pub can_open_diff: bool,
}

/// The status text of a run (C2): Creating and Starting from the run, Working and Waiting for
/// input from its session, the rest from the run's status. A prompted run whose session gives no
/// signal yet says Prompted rather than guessing.
pub fn status_text(status: &RunStatus, activity: Option<&ActivitySignal>) -> &'static str {
    match status {
        RunStatus::Creating => "Creating",
        RunStatus::Starting => "Starting",
        RunStatus::Prompted => match activity {
            Some(ActivitySignal::Working) => "Working",
            Some(ActivitySignal::AwaitingInput) => "Waiting for input",
            Some(ActivitySignal::Ended { .. }) => "Ended",
            Some(ActivitySignal::Unknown) | None => "Prompted",
        },
        RunStatus::PromptNotDelivered { .. } => "Prompt not delivered",
        RunStatus::Failed { .. } => "Failed",
        RunStatus::Picked => "Picked",
    }
}

/// The open Compare view's group and its rows in run order; `sessions` are the project's sessions,
/// where each run's activity comes from.
pub fn compare_rows<'a>(
    state: &'a State,
    sessions: &[Session],
) -> Option<(&'a RunGroup, Vec<CompareRow>)> {
    let view = state.compare.as_ref()?;
    let group = state.groups.iter().find(|g| g.id == view.group)?;
    let rows = group
        .runs
        .iter()
        .map(|run| {
            let activity = run
                .session
                .and_then(|id| sessions.iter().find(|s| s.id == id))
                .map(|s| &s.activity);
            let reason = match &run.status {
                RunStatus::Failed { reason, .. } | RunStatus::PromptNotDelivered { reason } => {
                    Some(reason.clone())
                }
                _ => None,
            };
            let counts = match view.reads.get(&run.number) {
                _ if !run.status.has_worktree() => RunCounts::None,
                None | Some(Load::Idle) => RunCounts::Loading,
                Some(Load::Loading { last: Some(s), .. }) | Some(Load::Ready(s)) => {
                    RunCounts::Ready(*s)
                }
                Some(Load::Loading { last: None, .. }) => RunCounts::Loading,
                Some(Load::Failed(message)) => RunCounts::Failed(message.clone()),
            };
            CompareRow {
                number: run.number,
                provider: run.provider,
                status: status_text(&run.status, activity),
                reason,
                counts,
                can_open_diff: run.status.has_worktree(),
            }
        })
        .collect();
    Some((group, rows))
}

/// **Open diff** on `run`: the run's Changes view opens (C4); a run with no worktree opens nothing.
pub fn open_diff(state: &State, run: u8) -> Vec<Outcome> {
    let found = state
        .compare
        .as_ref()
        .and_then(|view| state.groups.iter().find(|g| g.id == view.group))
        .and_then(|group| group.runs.iter().find(|r| r.number == run))
        .filter(|run| run.status.has_worktree());
    match found {
        Some(run) => vec![Outcome::ChangesRequested(SessionLocation::Worktree(
            run.names.dir_name.clone(),
        ))],
        None => Vec::new(),
    }
}

fn edited(dialog: &mut ParallelDialog, msg: Msg) {
    match msg {
        Msg::PromptChanged(text) => dialog.prompt = text,
        Msg::TypeChanged(type_) => dialog.naming.type_ = Some(type_),
        Msg::TicketChanged(text) => {
            dialog.naming.ticket = (!text.trim().is_empty()).then_some(text);
        }
        Msg::NameChanged(text) => dialog.naming.name = text,
        Msg::BaseBranchChanged(branch) => dialog.base_branch = branch,
        Msg::ProviderChanged(index, cli) => {
            if let Some(slot) = dialog.runs.get_mut(index) {
                *slot = cli;
            }
        }
        Msg::RunAdded => {
            if dialog.runs.len() < MAX_RUNS {
                dialog.runs.push(dialog.default_cli);
            }
        }
        Msg::RunRemoved(index) => {
            if dialog.runs.len() > MIN_RUNS && index < dialog.runs.len() {
                dialog.runs.remove(index);
            }
        }
        _ => return,
    }
    dialog.revalidate();
}

fn branches_listed(dialog: &mut ParallelDialog, candidates: Vec<BranchCandidate>) {
    let current = candidates
        .iter()
        .find(|c| c.blocked_by == Some(BlockReason::CheckedOutInProjectRoot))
        .map(|c| c.name.clone());
    dialog.branches = candidates
        .into_iter()
        .filter(|c| c.origin == BranchOrigin::Local)
        .map(|c| c.name)
        .collect();
    if dialog.base_branch.is_empty() {
        dialog.base_branch = current
            .or_else(|| dialog.branches.first().cloned())
            .unwrap_or_default();
    }
}

fn confirmed(state: &mut State) -> Effect {
    let Some(dialog) = &mut state.dialog else {
        return Effect::None;
    };
    if let Err(invalid) = dialog.validate() {
        dialog.error = Some(invalid.to_string());
        return Effect::None;
    }
    let dialog = state.dialog.take().expect("checked above");
    Effect::Send(ClientMsg::RunGroupCreate {
        req: 0,
        project: dialog.project,
        naming: dialog.naming,
        prompt: dialog.prompt,
        base_branch: dialog.base_branch,
        providers: dialog.runs,
    })
}

/// The Run in parallel dialog, as a floating surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParallelRunDialog;

impl FloatingSurface for ParallelRunDialog {
    fn id(&self) -> SurfaceId {
        SurfaceId::new("run_in_parallel")
    }

    fn layer(&self) -> Layer {
        Layer::Dialog
    }

    fn dismissal(&self) -> DismissalRules {
        DismissalRules::for_layer(Layer::Dialog).cancelled_by(Message::Runs(Msg::Dismissed))
    }
}

impl Registered for ParallelRunDialog {
    fn open_in(state: &crate::app::State) -> Option<Self> {
        state.runs.dialog.as_ref().map(|_| ParallelRunDialog)
    }
}

/// The group row's right-click menu, as a floating surface (G4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupContextMenu;

impl GroupContextMenu {
    /// This surface's identity.
    pub const ID: SurfaceId = SurfaceId::new("run_group_menu");
}

impl FloatingSurface for GroupContextMenu {
    fn id(&self) -> SurfaceId {
        Self::ID
    }

    fn layer(&self) -> Layer {
        Layer::ContextMenu
    }

    fn dismissal(&self) -> DismissalRules {
        DismissalRules::for_layer(Layer::ContextMenu)
            .cancelled_by(Message::Runs(Msg::MenuDismissed))
    }
}

impl Registered for GroupContextMenu {
    fn open_in(state: &crate::app::State) -> Option<Self> {
        state.runs.menu.map(|_| GroupContextMenu)
    }
}

/// The Dismiss group confirmation, as a floating surface (G5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfirmDismissGroupDialog;

impl FloatingSurface for ConfirmDismissGroupDialog {
    fn id(&self) -> SurfaceId {
        SurfaceId::new("confirm_dismiss_run_group")
    }

    fn layer(&self) -> Layer {
        Layer::Dialog
    }

    fn dismissal(&self) -> DismissalRules {
        DismissalRules::for_layer(Layer::Dialog).cancelled_by(Message::Runs(Msg::DismissCancelled))
    }
}

impl Registered for ConfirmDismissGroupDialog {
    fn open_in(state: &crate::app::State) -> Option<Self> {
        state
            .runs
            .dismiss_target
            .as_ref()
            .map(|_| ConfirmDismissGroupDialog)
    }
}

/// Route a message through the root: the two that open a surface say so, and the dialog closes
/// the other surfaces first.
pub fn routed(state: &mut crate::app::State, msg: Msg) -> Vec<Outcome> {
    match msg {
        Msg::MenuToggled(..) => {
            apply(&mut state.runs, msg);
            crate::features::surface_opened(state.runs.menu.is_some(), GroupContextMenu::ID)
        }
        Msg::CompareOpened { .. } => {
            // Compare takes the Changes view's place; the root closes the Changes view
            // (`State::update`), since that is another feature's state.
            apply(&mut state.runs, msg);
            Vec::new()
        }
        Msg::DiffOpened { run } => open_diff(&state.runs, run),
        Msg::DismissAsked { .. } => {
            state.clear_for_dialog();
            apply(&mut state.runs, msg);
            Vec::new()
        }
        other => {
            apply(&mut state.runs, other);
            Vec::new()
        }
    }
}
