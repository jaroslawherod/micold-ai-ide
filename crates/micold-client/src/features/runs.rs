//! Run in parallel: the dialog that starts a group of prompted runs, and the project's group list
//! (feature 483, contracts/parallel-surfaces.md D and G).
//!
//! Render-free. The dialog is opened from the sidebar and sends one `RunGroupCreate`; the service
//! then pushes the whole group list (`RunGroupsChanged`), which [`Msg::GroupsChanged`] stores for
//! the sidebar to project. The shell turns [`Effect::Send`] into a correlated request.

use std::collections::BTreeSet;
use std::fmt;
use std::path::PathBuf;

use crate::app::Message;
use crate::features::Outcome;
use crate::overlay::registry::Registered;
use crate::overlay::{DismissalRules, FloatingSurface, SurfaceId};
use micold_core::naming::{derive, ConventionalType, DerivedNames, NamingError, WorktreeNaming};
use micold_core::overlay::Layer;
use micold_core::protocol::messages::ClientMsg;
use micold_core::runs::naming::derive_group;
use micold_core::runs::{GroupId, RunGroup, DEFAULT_RUNS, MAX_RUNS, MIN_RUNS};
use micold_core::session::AiCli;
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
    /// The request the root left for the shell while it interpreted an outcome (opening the
    /// dialog), taken once right after the message that caused it.
    pub pending: Option<Effect>,
}

impl State {
    /// Whether the group's row shows its runs.
    pub fn is_expanded(&self, id: GroupId) -> bool {
        !self.collapsed.contains(&id)
    }
}

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
}

/// What the shell must do after a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Nothing.
    None,
    /// Send this request; its `req` is assigned by the shell.
    Send(ClientMsg),
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
