//! The add-worktree form: its state, its sub-states, and the operations over them.
//!
//! This is the one feature whose intermediate state no other feature reads — it is opened,
//! filled in across several steps (type, ticket, name, branch source, typeahead, conflict
//! resolution), then submitted or cancelled as a unit. That independent lifecycle is what
//! qualifies it as feature 021's sole nested-unit candidate (research.md §5); until Tier 3 it is
//! an ordinary feature module holding data and operations only.
//!
//! Render-free, like every module here: `tests/features_are_render_free.rs` holds that line.
//!
//! # The state this feature remembers (feature 028, contract S1)
//!
//! Two fields in [`State`], reached as `state.worktree_form`: `form`, the [`WorktreeForm`] itself
//! when one is open (`None` when it is not — its presence *is* the form being shown), and
//! `worktree_error`, the last failure to report back to the user.
//!
//! `form` sheds the `worktree_form` the qualifier now carries. `worktree_error` does not, and the
//! distinction is the point: `worktree_form` names the *form*, while `worktree_error` names what
//! failed — a worktree operation, which may have been started from somewhere other than this form
//! (T031).
//!
//! This is the feature whose intermediate state no other feature reads, per the header above. The
//! struct is where that claim becomes checkable: everything the form accumulates between opening
//! and submitting is inside `form`, and nothing outside this module names any of it.

use crate::app::Message;
use crate::features::window::FieldId;
use crate::overlay::registry::Registered;
use crate::overlay::{DismissalRules, FloatingSurface, SurfaceId};
use micold_core::env_include::EnvIncludeSnapshot;
use micold_core::format::group_digits;
use micold_core::git::GitRemote;
use micold_core::github::{
    choose_remote, describe_listed, merge_searched, next_description_cursor, DescriptionPage,
    GithubRepo, Issue, IssueListing, IssueLoadError, NoticeSource, RemoteChoice,
};
use micold_core::issue_details::IssueDetails;
use micold_core::issue_types::LabelTypeEntry;
use micold_core::jira::{connection_key, IssueProvider, JiraConnection};
use micold_core::naming::{
    derive, dir_name_from_branch, naming_for_issue, ConventionalType, DerivedNames, NamingError,
    WorktreeNaming,
};
use micold_core::overlay::Layer;
use micold_core::protocol::messages::SessionDefinition;
use micold_core::required::{RequiredMarks, Requirement};
use micold_core::session::AiCli;
use micold_core::typeahead::{clamp_highlight, move_highlight, rank, Direction, Match, Query};
use micold_core::worktree::{BranchCandidate, BranchSituation, CreateMode, CreateStage, Worktree};
use std::path::PathBuf;

/// What this feature remembers (feature 028, contract S1).
///
/// The fields keep the names they had as flat members of `app::State`, and the reducers below
/// spell the root's type `crate::app::State` now that `State` here means this struct.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// The add-worktree form, present only while its overlay is shown (FR-005).
    ///
    /// Named `form` rather than keeping the flat `worktree_form`: the qualifier already says
    /// which form it is, and `state.worktree_form.worktree_form` says it twice. Its sibling keeps
    /// its full name because `worktree_error` is not the form's error — it also carries a refused
    /// project open (FR-001a), which happens with no form on screen at all.
    pub form: Option<WorktreeForm>,
    /// A message shown when opening a non-git directory was refused (FR-001a), or a worktree
    /// create failed (FR-017). Transient.
    pub worktree_error: Option<String>,
    /// A create whose form was cancelled while it was still running (feature 013, BUG-001).
    ///
    /// Cancel closes the overlay and does not stop the create (FR-010b), so its outcome still
    /// arrives — with no form to land on. This is what says the outcome is owed a notification,
    /// and what the form knew about the create that the notification has to repeat.
    pub cancelled_create: Option<CancelledCreate>,
    /// The last issue-load seq handed out (feature 034, FR-007a). Lives here, not on the form, so
    /// it outlives a closed form and a late result can never match a new form's request.
    pub issue_request_seq: u64,
}

impl State {
    /// The issue number of row `index` of the shown results, if there is such a row.
    pub fn issue_number_at(&self, index: usize) -> Option<u64> {
        self.form.as_ref()?.issue_number_at(index)
    }

    /// The seq the open form is waiting for, when a load is in flight.
    pub fn awaited_issue_load(&self) -> Option<u64> {
        match self.form.as_ref()?.issues {
            IssueList::Loading { seq } => Some(seq),
            _ => None,
        }
    }

    /// The search state of the open form's loaded list, if it has one.
    fn issue_search(&self) -> Option<&SearchState> {
        match &self.form.as_ref()?.issues {
            IssueList::Loaded { search, .. } => Some(search),
            _ => None,
        }
    }

    /// The seq a keystroke is waiting out the debounce for (research R9).
    pub fn pending_issue_search(&self) -> Option<u64> {
        match self.issue_search()? {
            SearchState::Pending { seq } => Some(*seq),
            _ => None,
        }
    }

    /// The seq of the search beyond the loaded issues now running (FR-005a).
    pub fn awaited_issue_search(&self) -> Option<u64> {
        match self.issue_search()? {
            SearchState::Searching { seq } => Some(*seq),
            _ => None,
        }
    }

    /// What a running search asks: the repository, the text and the `gh` the list was read with.
    pub fn issue_search_request(&self) -> Option<(GithubRepo, String, PathBuf)> {
        let form = self.form.as_ref()?;
        if form.jira_site().is_some() {
            return None;
        }
        match &form.issues {
            IssueList::Loaded {
                gh,
                search: SearchState::Searching { .. },
                ..
            } => Some((
                form.github_repo()?.clone(),
                form.issue_query.trim().to_string(),
                gh.clone(),
            )),
            _ => None,
        }
    }

    /// What a running Jira search asks: the typed text. `None` unless Jira is the provider and a
    /// search beyond the loaded issues is running (533 FR-012).
    pub fn jira_search_text(&self) -> Option<String> {
        let form = self.form.as_ref()?;
        form.jira_site()?;
        match &form.issues {
            IssueList::Loaded {
                search: SearchState::Searching { .. },
                ..
            } => Some(form.issue_query.trim().to_string()),
            _ => None,
        }
    }

    /// The read of the picked issue's details the open form awaits (837 form-details §1).
    ///
    /// `Some` only while `Loading`, the list is loaded and the repository known, so the shell can
    /// tell a message that moved the pick (the request changed) from one the reducer dropped. A
    /// number above `u32::MAX` has no request: `read_details` takes a `u32`, as `IssueLookup` does.
    pub fn issue_details_request(&self) -> Option<DetailsRequest> {
        let form = self.form.as_ref()?;
        let IssueDetailsState::Loading { seq, number } = form.issue_details else {
            return None;
        };
        let IssueList::Loaded { gh, .. } = &form.issues else {
            return None;
        };
        Some(DetailsRequest {
            seq,
            number: u32::try_from(number).ok()?,
            repo: form.github_repo()?.clone(),
            gh: gh.clone(),
        })
    }

    /// The page of descriptions the open form awaits, and what asks for it (038 FR-024).
    ///
    /// `Some` only while a loaded list's pass awaits a page, so the shell can tell a message that
    /// moved the pass on (the request changed) from one the reducer dropped (it did not).
    pub fn issue_description_request(&self) -> Option<DescriptionRequest> {
        let form = self.form.as_ref()?;
        match &form.issues {
            IssueList::Loaded {
                gh,
                descriptions: DescriptionPass::Loading { seq, cursor, .. },
                ..
            } => Some(DescriptionRequest {
                seq: *seq,
                repo: form.github_repo()?.clone(),
                cursor: cursor.clone(),
                gh: gh.clone(),
            }),
            _ => None,
        }
    }
}

/// The read of the picked GitHub issue's details: `number` of `repo`, with the `gh` the list was
/// read with, answered under `seq` (837 contracts/form-details.md §1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetailsRequest {
    pub seq: u64,
    pub number: u32,
    pub repo: GithubRepo,
    pub gh: PathBuf,
}

/// Where the picked issue's details are (837 data-model §6).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum IssueDetailsState {
    /// No GitHub issue is picked.
    #[default]
    Idle,
    /// The read is awaited; only the answer carrying this `seq` and `number` applies.
    Loading { seq: u64, number: u64 },
    /// The details of the picked issue.
    Loaded {
        number: u64,
        details: Box<IssueDetails>,
    },
    /// The read failed; `message` is the plain sentence naming the reason.
    Failed { number: u64, message: String },
}

/// One request of the description pass: the page after `cursor` of `repo`'s open issues, read with
/// the `gh` the list was read with, answered under `seq` (038 contracts/issue-fields.md §6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescriptionRequest {
    /// The seq of the load this pass follows.
    pub seq: u64,
    pub repo: GithubRepo,
    /// Where the page starts; `None` for the first.
    pub cursor: Option<String>,
    pub gh: PathBuf,
}

/// Where the second pass over a loaded list is: the one that reads the descriptions the list
/// request leaves out, a page at a time (038 FR-024, contracts/issue-fields.md §6).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum DescriptionPass {
    /// A page is awaited; only the answer carrying this `seq` and `cursor` applies.
    Loading {
        /// The seq of the load this pass follows.
        seq: u64,
        /// Where the awaited page starts; `None` for the first.
        cursor: Option<String>,
        /// Pages already read.
        pages: usize,
    },
    /// No page is awaited: the pass finished, failed, or there was nothing to describe.
    #[default]
    Done,
}

/// What a form cancelled mid-create knew about its create (feature 013, FR-010b).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CancelledCreate {
    /// The mode the create runs in, which words its stage (feature 016, FR-024).
    pub mode: CreateMode,
    /// The last stage the daemon reported, if any, for a failure to name (FR-009).
    pub stage: Option<CreateStage>,
}

impl CancelledCreate {
    /// The failure notification's text: the stage it failed at, then the failure's own message.
    fn failure(&self, message: &str) -> String {
        match self.stage {
            Some(stage) => format!(
                "Creating the worktree failed at \"{}\": {message}",
                stage.label(&self.mode)
            ),
            None => format!("Creating the worktree failed: {message}"),
        }
    }
}

/// Transient creation status for the add-worktree form (feature 010, research R4). Not
/// persisted — reset to `Editing` whenever the form is (re)opened.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WorktreeFormStatus {
    /// The user is filling in the form; no create is in flight.
    #[default]
    Editing,
    /// `WorktreeCreateStarted` was dispatched; the async create (including any submodule
    /// fetch) is running. The form shows a "Creating worktree…" state and disables submission.
    Creating,
}

/// Which half of the add-worktree form is active (feature 016, FR-010).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BranchSource {
    /// Type + ticket + name inputs — a brand-new branch. Today's form.
    #[default]
    New,
    /// Pick from the branches that already exist (User Story 2).
    Existing,
    /// Pick an open GitHub issue; its number and title fill the new-branch inputs (feature 034).
    Issue,
}

/// The Jira site and project a connected form reads, as the notice names them (feature 533,
/// data-model §4). Holds no credential: the shell looks the connection up again at each request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JiraSite {
    /// The site's origin, `https://host`, as the notice shows it.
    pub origin: String,
    /// The site's host, as the loading line and the failure notices name it.
    pub host: String,
    /// The connected project's key.
    pub key: String,
}

impl JiraSite {
    /// The site a connection names.
    pub fn of(connection: &JiraConnection) -> JiraSite {
        let origin = connection.site.as_str().to_string();
        let host = origin
            .split_once("://")
            .map_or(origin.as_str(), |(_, host)| host)
            .to_string();
        JiraSite {
            origin,
            host,
            key: connection.project_key.as_str().to_string(),
        }
    }
}

/// Whether Jira can be the issue source of this form (feature 533, data-model §4): derived from
/// the settings when the form opens and again on every `SettingsChanged`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum JiraAvailability {
    /// The switch is off, or this project has no connection: the form is the GitHub form of 034.
    #[default]
    Off,
    /// The switch is on and this project has a connection.
    Connected(JiraSite),
}

impl JiraAvailability {
    /// What the settings in force make of `project` (FR-001, FR-020): `Connected` only when the
    /// integration is on and the project has a connection.
    pub fn from_settings(
        enabled: bool,
        connections: &std::collections::BTreeMap<String, JiraConnection>,
        project: Option<&std::path::Path>,
    ) -> JiraAvailability {
        if !enabled {
            return JiraAvailability::Off;
        }
        project
            .and_then(|project| connections.get(&connection_key(project)))
            .map_or(JiraAvailability::Off, |connection| {
                JiraAvailability::Connected(JiraSite::of(connection))
            })
    }
}

/// Whether this repository has a GitHub remote the issue source can read (feature 034,
/// data-model §5). Decided once per form, from the daemon's `RemoteList` answer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum GithubAvailability {
    /// The remotes are still being read.
    #[default]
    Checking,
    /// The source cannot be chosen, and this says why.
    Unavailable(String),
    /// The source can be chosen and reads this repository.
    Available(GithubRepo),
}

/// The issue source's list, from first choice to a loaded listing (feature 034, data-model §5).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum IssueList {
    /// Nothing asked for: the source was never chosen on this form, or was left.
    #[default]
    NotRequested,
    /// A load is in flight; only the result carrying `seq` applies (FR-007a).
    Loading { seq: u64 },
    /// The load failed; Retry starts another.
    Failed { error: IssueLoadError },
    /// The open issues, and the `gh` that read them (kept for the search beyond the cap, M3).
    Loaded {
        listing: IssueListing,
        gh: PathBuf,
        /// Issues GitHub search found beyond the loaded ones, none of them loaded (invariant 4).
        searched: Vec<Issue>,
        /// Where that search is.
        search: SearchState,
        /// Where the pass that reads the listed issues' descriptions is (038 FR-024).
        descriptions: DescriptionPass,
    },
}

impl IssueList {
    /// Every issue held, loaded first and then searched: the order `issue_matches` indexes.
    pub fn held(&self) -> Vec<&Issue> {
        match self {
            IssueList::Loaded {
                listing, searched, ..
            } => listing.issues.iter().chain(searched.iter()).collect(),
            _ => Vec::new(),
        }
    }
}

/// The search beyond the loaded issues (feature 034, FR-005a, data-model §5).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SearchState {
    /// No search pending or running: the listing is complete, nothing is typed, or the last search
    /// answered.
    #[default]
    Idle,
    /// A keystroke is waiting out the debounce (research R9); only `IssueSearchDue { seq }` starts it.
    Pending { seq: u64 },
    /// GitHub is being searched; only the answer carrying `seq` applies (FR-007a).
    Searching { seq: u64 },
    /// The search failed; the loaded matches stay shown and Retry searches again.
    Failed { error: IssueLoadError },
}

/// The conflict-resolution sub-state of the add-worktree form (feature 016, contract
/// `branch-conflict.md` §3).
///
/// Lives INSIDE the form rather than as a dialog of its own: only one dialog shows at a time, so
/// routing the prompt through the registry would tear down the form — and with it the inputs
/// FR-007 requires to survive a cancel (research R9).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ResolutionState {
    /// No prompt showing.
    #[default]
    Idle,
    /// Pre-flight found something; the user is choosing what to do (FR-002).
    Choosing { situation: BranchSituation },
    /// Overwrite was chosen; the destructive confirmation is showing (FR-005).
    ConfirmingOverwrite { situation: BranchSituation },
}

impl ResolutionState {
    /// The situation being resolved, if any.
    pub fn situation(&self) -> Option<&BranchSituation> {
        match self {
            ResolutionState::Idle => None,
            ResolutionState::Choosing { situation }
            | ResolutionState::ConfirmingOverwrite { situation } => Some(situation),
        }
    }

    /// Whether a prompt is currently awaiting the user.
    pub fn is_prompting(&self) -> bool {
        !matches!(self, ResolutionState::Idle)
    }
}

/// What the form says when no daemon is configured (feature 491, T037): no runtime is offered, and
/// the way to add one is named.
pub const NO_DAEMONS_NOTICE: &str = "No daemon is set up. Add one in Settings, under Daemons.";

/// A daemon the form offers for the new worktree (feature 491, FR-006).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonChoice {
    /// The stored identity the binding is keyed by.
    pub id: micold_core::daemons::DaemonId,
    /// The name the picker shows.
    pub name: String,
}

impl std::fmt::Display for DaemonChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

/// One entry of the dialog's AI CLI choice (feature 776, FR-006): a CLI, or no session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionChoice {
    /// Start a session on this CLI together with the worktree.
    Cli(AiCli),
    /// Create the worktree with no session (FR-010).
    NoSession,
}

impl std::fmt::Display for SessionChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cli(cli) => write!(f, "{cli}"),
            Self::NoSession => f.write_str("None"),
        }
    }
}

/// The caption of an issue source that is not offered because the switch is off (feature 1102).
pub const GITHUB_ISSUES_OFF_NOTICE: &str = "GitHub issues are turned off in Settings.";

/// In-progress add-worktree form state, present only while the form overlay is open (FR-005).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorktreeForm {
    /// The daemons the new worktree may run on: every registry entry, in registry order. Empty
    /// when none is registered, and then no choice is offered (feature 491, FR-006).
    pub daemons: Vec<DaemonChoice>,
    /// The chosen daemon: the legacy default when registered, else the first. `None` only while
    /// `daemons` is empty.
    pub daemon: Option<micold_core::daemons::DaemonId>,
    /// The daemon label of each worktree folder the project holds, by folder name, so a refusal
    /// that names a holder can say where it runs (feature 491, FR-016). Fixed when the form opens.
    pub holder_daemons: std::collections::BTreeMap<String, String>,
    /// Selected Conventional-Commits type (FR-005a).
    pub type_: Option<ConventionalType>,
    /// Optional ticket reference (FR-005b).
    pub ticket: String,
    /// Free-text name.
    pub name: String,
    /// The last validation error shown after a rejected submit.
    pub error: Option<NamingError>,
    /// Which fields the user has left and whether a submit was attempted, for the required
    /// errors (feature 937). Fresh on open, dropped with the form.
    pub required: RequiredMarks<FieldId>,
    /// Whether a create is in flight (feature 010, data-model.md).
    pub status: WorktreeFormStatus,
    /// Which half of the form is active (feature 016, FR-010).
    pub source: BranchSource,
    /// Branches that already exist, listed when `source` becomes `Existing` (FR-011). Empty
    /// until then — the listing is not run on every keystroke.
    pub candidates: Vec<BranchCandidate>,
    /// The picked existing branch, if any (FR-014). Written by exactly one message, and only for a
    /// candidate that is available — which is what makes "a blocked branch can never be the
    /// selection" a property rather than a promise (feature 021, FR-012a).
    pub selected_branch: Option<BranchCandidate>,
    /// The branch search text, exactly as typed (feature 021, FR-001). Never holds a branch name:
    /// the selection lives in `selected_branch` and is shown by the preview, so clearing the field
    /// means one thing only (FR-014a).
    pub branch_query: String,
    /// Which candidates match `branch_query`, in the order they should be shown — derived on every
    /// keystroke and never edited in place (FR-005). Indices address `candidates`.
    pub branch_matches: Vec<(usize, Match)>,
    /// Whether the result list is showing. Set by focus, typing, a pick, a dismissal or a source
    /// change — never inferred from whether `branch_matches` is empty, because an open list with no
    /// matches is exactly the state that shows the no-match message (FR-015).
    pub branch_list_open: bool,
    /// Where the keyboard is, as an index into `branch_matches` rather than into `candidates`: a
    /// highlight that indexed the unfiltered list could name a row the developer cannot see
    /// (feature 021, research R15).
    pub branch_highlight: Option<usize>,
    /// The conflict prompt's state (feature 016, FR-001/FR-005).
    pub resolution: ResolutionState,
    /// The mode the in-flight create is running under. Set when the create is sent, and read
    /// only to word [`Self::stage`] — a reuse must not say "Creating branch" (FR-024).
    pub mode: CreateMode,
    /// The stage the daemon last reported for the in-flight create (feature 016, FR-024).
    /// `None` until the first `OperationProgress` arrives; reset when a new attempt starts.
    pub stage: Option<CreateStage>,
    /// The latest live output line for [`Self::stage`], when the daemon has sent one (BUG-009,
    /// T123). Only long stages produce these — a submodule fetch, in practice — so it stays `None`
    /// for the fast ones, and is cleared on every stage change and new attempt.
    pub stage_detail: Option<String>,
    /// What to say about a create whose connection dropped before the daemon answered (feature
    /// 010, BUG-020). Set by [`create_interrupted`]; cleared when a new attempt starts, and with
    /// the form when it closes.
    ///
    /// # Why this is not `State::worktree_error`
    ///
    /// The error line is cleared whenever the worktree list is replaced, on the grounds that a
    /// create failure shown against the old list is stale (T067a-4). That is right for a failure
    /// and exactly wrong for this: the whole content of this message is *the list is the authority
    /// now*, and the list arriving is what makes it true. Reconnection follows the drop within a
    /// second or two and `State::set_worktrees` reports its outcome unconditionally, so a notice
    /// living on the error line is one the user gets no chance to read.
    pub interrupted: Option<String>,
    /// Whether the issue source may be chosen (feature 034, FR-002).
    pub github: GithubAvailability,
    /// Whether Jira can be an issue source here (feature 533).
    pub jira: JiraAvailability,
    /// The GitHub issues switch was off when the form opened (feature 1102, FR-003). Stored
    /// inverted so a form built with `Default` keeps GitHub issues on, as the switch does. While
    /// set, [`Self::github_repo`] is `None`, so no GitHub issue request is built and GitHub is
    /// never the issue provider.
    pub github_issues_off: bool,
    /// Which provider the Issue source lists (feature 533). Jira only while `jira` is
    /// `Connected` and the repository has no GitHub remote, or the project has both and Jira is the
    /// remembered choice (`remembered_provider`, set by `IssueProviderChanged`).
    pub provider: IssueProvider,
    /// The source this project last used, from its state file or chosen in this form (feature 533,
    /// FR-016). `None` opens on GitHub. Only read while both sources are available.
    pub remembered_provider: Option<IssueProvider>,
    /// The issue source's list (feature 034).
    pub issues: IssueList,
    /// The issue search text, exactly as typed (FR-005).
    pub issue_query: String,
    /// Which loaded issues match `issue_query`, in order. Indices address the listing's issues.
    pub issue_matches: Vec<(usize, Match)>,
    /// Whether the issue result list is showing.
    pub issue_list_open: bool,
    /// Where the keyboard is, as an index into `issue_matches`.
    pub issue_highlight: Option<usize>,
    /// The issue last picked, if any (FR-010a).
    pub picked_issue: Option<u64>,
    /// The picked GitHub issue's relationships (837). Never read by Create (FR-006).
    pub issue_details: IssueDetailsState,
    /// The last details seq handed out; bumped on every change of the pick.
    pub details_seq: u64,
    /// The AI CLI the worktree's session starts on (feature 776, FR-006); `None` creates the
    /// worktree with no session. Preselected to the default CLI when the form opens.
    pub session_cli: Option<AiCli>,
    /// The entries the choice offers, "None" last. Fixed when the form opens.
    pub session_choices: Vec<SessionChoice>,
    /// The session's first prompt, as typed. Only sent with a CLI: see [`Self::session_definition`].
    pub first_prompt: String,
    /// The default prompt template from Settings, copied when the form opens (feature 1161).
    pub default_prompt_template: String,
    /// The user has typed in the first-prompt field, so the template no longer rewrites it
    /// (feature 1161, FR-007).
    pub first_prompt_edited: bool,
}

impl WorktreeForm {
    /// Render the default template into the first prompt while the user has not edited it and a CLI
    /// is chosen (feature 1161, FR-007). A template that is empty or does not parse gives an empty
    /// prompt (FR-012, FR-013).
    pub fn refresh_default_prompt(&mut self) {
        if self.first_prompt_edited || self.session_cli.is_none() {
            return;
        }
        let picked = self
            .picked_issue
            .and_then(|n| self.issues.held().into_iter().find(|i| i.number() == n));
        let facts = micold_core::prompt_template::WorktreeFacts {
            issue: self.ticket.trim().to_string(),
            issue_title: picked.map(|i| i.title().to_string()).unwrap_or_default(),
            issue_body: picked.map(|i| i.body().to_string()).unwrap_or_default(),
            branch: self.preview().map(|names| names.branch).unwrap_or_default(),
            type_: self
                .type_
                .map(|t| t.as_str().to_string())
                .unwrap_or_default(),
            name: self.name.clone(),
        };
        self.first_prompt =
            micold_core::prompt_template::render_text(&self.default_prompt_template, &facts)
                .unwrap_or_default();
    }

    /// The user typed in the first-prompt field. Ignored with no CLI chosen.
    pub fn edit_first_prompt(&mut self, text: String) {
        if self.session_cli.is_some() {
            self.first_prompt = text;
            self.first_prompt_edited = true;
        }
    }

    /// Choose the session's CLI, or none. None drops the prompt and the edit; a CLI chosen after
    /// none follows the template again.
    pub fn choose_session_cli(&mut self, cli: Option<AiCli>) {
        self.session_cli = cli;
        if cli.is_none() {
            self.first_prompt.clear();
            self.first_prompt_edited = false;
        }
        self.refresh_default_prompt();
    }

    /// The chosen entry of [`Self::session_choices`].
    pub fn session_choice(&self) -> SessionChoice {
        self.session_cli
            .map_or(SessionChoice::NoSession, SessionChoice::Cli)
    }

    /// The session to create with the worktree (FR-006): whole or absent. A prompt without a CLI
    /// is not representable, so "None" sends nothing whatever was typed.
    pub fn session_definition(&self) -> Option<SessionDefinition> {
        self.session_cli
            .map(|cli| SessionDefinition::new(cli, Some(self.first_prompt.clone())))
    }

    /// The daemon label of the worktree at `path`, when the app manages one there.
    pub fn holder_daemon(&self, path: &std::path::Path) -> Option<&str> {
        let folder = path.file_name()?.to_str()?;
        self.holder_daemons.get(folder).map(String::as_str)
    }

    /// [`BlockReason::explain_on`] with the holder's daemon filled in (feature 491, FR-016).
    pub fn explain_block(
        &self,
        reason: &micold_core::worktree::BlockReason,
        branch: &str,
    ) -> String {
        use micold_core::worktree::BlockReason;
        let holder = match reason {
            BlockReason::CheckedOutAt { path, .. } => self.holder_daemon(path),
            _ => None,
        };
        reason.explain_on(branch, holder)
    }

    /// [`explain_directory_taken_on`] with the holder's daemon filled in (feature 491, FR-016).
    pub fn explain_taken(&self, dir: &std::path::Path) -> micold_core::worktree::DirectoryClash {
        micold_core::worktree::explain_directory_taken_on(dir, self.holder_daemon(dir))
    }

    /// The issue number of row `index` of `issue_matches`.
    pub fn issue_number_at(&self, index: usize) -> Option<u64> {
        let (held, _) = self.issue_matches.get(index)?;
        self.issues.held().get(*held).map(|issue| issue.number())
    }

    /// The caption under the source switch (FR-002, FR-025): why the issue chip is disabled, or —
    /// before it is chosen — which repository choosing it reads from GitHub. `None` once the issue
    /// source is chosen, when [`Self::issue_notice`] says it instead.
    pub fn source_caption(&self) -> Option<String> {
        if self.provider == IssueProvider::Jira {
            return match self.source {
                BranchSource::Issue => None,
                _ => self.jira_notice(),
            };
        }
        if self.github_issues_off {
            return Some(GITHUB_ISSUES_OFF_NOTICE.to_string());
        }
        match &self.github {
            GithubAvailability::Checking => Some("Checking for a GitHub remote…".to_string()),
            GithubAvailability::Unavailable(reason) => Some(reason.clone()),
            GithubAvailability::Available(_) if self.source == BranchSource::Issue => None,
            GithubAvailability::Available(repo) => Some(format!(
                "GitHub issue reads open issues of {repo} from GitHub."
            )),
        }
    }

    /// The issue source's own notice, naming the repository it reads (FR-025).
    pub fn issue_notice(&self) -> Option<String> {
        if self.provider == IssueProvider::Jira {
            return match self.source {
                BranchSource::Issue => self.jira_notice(),
                _ => None,
            };
        }
        match (self.github_repo(), self.source) {
            (Some(repo), BranchSource::Issue) => {
                Some(format!("Reads open issues of {repo} from GitHub."))
            }
            _ => None,
        }
    }

    /// What the Jira source reads and sends, shown before, during and after a load (FR-005b).
    fn jira_notice(&self) -> Option<String> {
        let JiraAvailability::Connected(site) = &self.jira else {
            return None;
        };
        Some(format!(
            "Reads open issues of project {} from {}. Sends the project key, your search text and your credentials, and, with this dialog closed, the keys of the tickets your worktrees are linked to, to show their status.",
            site.key, site.origin
        ))
    }

    /// The Jira site the Issue source reads, while Jira is the provider.
    pub fn jira_site(&self) -> Option<&JiraSite> {
        match (&self.jira, self.provider) {
            (JiraAvailability::Connected(site), IssueProvider::Jira) => Some(site),
            _ => None,
        }
    }

    /// Whether the Issue source can be chosen: the repository is on GitHub, or Jira is the
    /// provider (feature 533, US4 AS3).
    pub fn issue_source_ready(&self) -> bool {
        self.github_repo().is_some() || self.jira_site().is_some()
    }

    /// Where the provider's failure notices name the source (feature 533, FR-018).
    pub fn notice_source(&self) -> Option<NoticeSource<'_>> {
        match self.jira_site() {
            Some(site) => Some(NoticeSource::Jira {
                host: &site.host,
                key: &site.key,
            }),
            None => self.github_repo().map(NoticeSource::from),
        }
    }

    /// Choose the provider the Issue source lists from what is known now: Jira when it is
    /// connected and the repository is known to have no GitHub remote, else GitHub.
    fn resolve_provider(&mut self) {
        let connected = matches!(self.jira, JiraAvailability::Connected(_));
        let no_github =
            self.github_issues_off || matches!(self.github, GithubAvailability::Unavailable(_));
        self.provider = if connected
            && (no_github
                || (self.offers_issue_provider_choice()
                    && self.remembered_provider == Some(IssueProvider::Jira)))
        {
            IssueProvider::Jira
        } else {
            IssueProvider::Github
        };
    }

    /// Whether the dialog offers the GitHub / Jira choice: the repository has a GitHub remote and
    /// Jira is connected (feature 533, FR-016). With one source only, no choice is shown.
    pub fn offers_issue_provider_choice(&self) -> bool {
        self.github_repo().is_some() && matches!(self.jira, JiraAvailability::Connected(_))
    }

    /// The caption under a list that does not hold every open issue (FR-004): how many were loaded
    /// and how many are open. `None` when every open issue is held.
    pub fn issue_cap_caption(&self) -> Option<String> {
        match &self.issues {
            IssueList::Loaded { listing, .. }
                if !listing.complete && self.provider == IssueProvider::Jira =>
            {
                Some(format!(
                    "Showing the {} most recently updated open issues.",
                    group_digits(listing.issues.len() as u64)
                ))
            }
            IssueList::Loaded { listing, .. } if !listing.complete => Some(format!(
                "Showing the {} most recently updated of {} open issues — search also looks on GitHub.",
                group_digits(listing.issues.len() as u64),
                group_digits(listing.total_open)
            )),
            _ => None,
        }
    }

    /// The line under the picker while the search beyond the loaded issues runs or after it failed
    /// (FR-005a, FR-007). `None` otherwise.
    pub fn issue_search_status(&self) -> Option<String> {
        match &self.issues {
            IssueList::Loaded {
                search: SearchState::Searching { .. },
                ..
            } => Some(format!("Searching {}…", self.provider_name())),
            IssueList::Loaded {
                search: SearchState::Failed { error },
                ..
            } => Some(format!(
                "Search beyond the loaded issues failed — {}",
                error.message(self.notice_source()?)
            )),
            _ => None,
        }
    }

    /// What the provider is called in a status line.
    fn provider_name(&self) -> &'static str {
        match self.provider {
            IssueProvider::Jira => "Jira",
            IssueProvider::Github => "GitHub",
        }
    }

    /// The repository the issue source reads, once known.
    pub fn github_repo(&self) -> Option<&GithubRepo> {
        match &self.github {
            GithubAvailability::Available(repo) if !self.github_issues_off => Some(repo),
            _ => None,
        }
    }

    /// Re-rank the held issues against `issue_query`, re-seating the highlight (invariant 3).
    fn rematch_issues(&mut self) {
        let held = self.issues.held();
        let query = Query::new(&self.issue_query);
        self.issue_matches = rank(&held, |issue| issue.row_text(), &query);
        self.issue_highlight = clamp_highlight(self.issue_highlight, self.issue_matches.len());
    }

    /// The one writer of `picked_issue` (837 data-model §6): moves the details with the pick. A GitHub
    /// pick awaits its details under a fresh seq; a cleared pick or a Jira one has none.
    pub fn set_picked_issue(&mut self, picked: Option<u64>) {
        self.picked_issue = picked;
        self.details_seq += 1;
        self.issue_details = match picked {
            Some(number) if self.provider == IssueProvider::Github => IssueDetailsState::Loading {
                seq: self.details_seq,
                number,
            },
            _ => IssueDetailsState::Idle,
        };
    }

    /// Forget the issues and their search: the source was left, so any load in flight is stale.
    fn reset_issues(&mut self) {
        self.issues = IssueList::NotRequested;
        self.issue_query.clear();
        self.issue_matches.clear();
        self.issue_list_open = false;
        self.issue_highlight = None;
        self.set_picked_issue(None);
        self.refresh_default_prompt();
    }

    /// Recompute the search results from `candidates` and `branch_query`, and re-seat the keyboard
    /// highlight so it cannot point past the end (feature 021, data-model §2 invariants 1–3).
    ///
    /// One function rather than a line in each arm: `branch_matches` is derived state, and derived
    /// state recomputed in several places is derived state that eventually is not. Every message
    /// that can change either input calls this.
    /// Re-rank the branch list against the current query.
    ///
    /// `pub(crate)` only because the reducer still lives in `crate::app`; it returns to
    /// private in Tier 3 when the reducer moves in beside it (feature 021, T062).
    pub(crate) fn rematch_branches(&mut self) {
        let query = Query::new(&self.branch_query);
        self.branch_matches = rank(&self.candidates, |c| c.name.as_str(), &query);
        self.branch_highlight = clamp_highlight(self.branch_highlight, self.branch_matches.len());
    }

    /// Clear everything the search owns, for when the picker is left or reopened. The search text
    /// is per-open-picker: branch relevance changes too fast for a remembered query to help.
    /// Clear the branch query and its ranking.
    ///
    /// `pub(crate)` for the same reason as [`Self::rematch_branches`].
    pub(crate) fn reset_branch_search(&mut self) {
        self.branch_query.clear();
        self.branch_highlight = None;
        // Open as soon as the picker is the picker, and closed whenever it is not (invariant 5).
        //
        // FR-001b asks for the list to open "when the search field takes focus — before anything is
        // typed", and the point of that is the second half: the branches on offer are visible from
        // the outset. Hanging it on focus alone cannot deliver it, because the rendering stack's
        // text input publishes nothing when it gains focus — so it was reachable only by a press,
        // and a developer arriving with Tab saw an empty field and no list until they typed.
        // Opening it with the picker is the same guarantee by every route in. A dismissal still
        // closes it, and nothing reopens it until the developer returns to the field.
        self.branch_list_open = self.source == BranchSource::Existing;
        self.rematch_branches();
    }

    /// The candidate the keyboard is currently on, if any.
    pub fn highlighted_branch(&self) -> Option<&BranchCandidate> {
        let (index, _) = self.branch_matches.get(self.branch_highlight?)?;
        self.candidates.get(*index)
    }

    /// The live derived directory/branch preview, or the validation error (FR-008a).
    ///
    /// Under [`BranchSource::Existing`] the names come from the selected branch instead of the
    /// type/ticket/name inputs (feature 016, FR-014), so the user sees the directory that will
    /// be created before committing to it.
    pub fn preview(&self) -> Result<DerivedNames, NamingError> {
        match self.source {
            // The issue source only fills the new-branch inputs; it names nothing of its own
            // (FR-012).
            BranchSource::New | BranchSource::Issue => derive(&WorktreeNaming {
                type_: self.type_,
                ticket: if self.ticket.trim().is_empty() {
                    None
                } else {
                    Some(self.ticket.clone())
                },
                name: self.name.clone(),
            }),
            BranchSource::Existing => {
                let candidate = self
                    .selected_branch
                    .as_ref()
                    .ok_or(NamingError::EmptyNameAfterSlug)?;
                let dir_name = dir_name_from_branch(&candidate.name);
                if dir_name.is_empty() {
                    return Err(NamingError::EmptyNameAfterSlug);
                }
                Ok(DerivedNames {
                    dir_name,
                    branch: candidate.name.clone(),
                })
            }
        }
    }

    /// The fields this form refuses empty, in display order, with whether each is empty now
    /// (feature 937, FR-002). Mirrors [`Self::preview`]: Type and Name for a new branch or an
    /// issue (Name blank after trim), the picked branch for an existing one.
    pub fn required_fields(&self) -> Vec<(FieldId, Requirement)> {
        match self.source {
            BranchSource::New | BranchSource::Issue => vec![
                (
                    FieldId::AddWorktreeType,
                    Requirement::choice(true, self.type_.as_ref()),
                ),
                (
                    FieldId::AddWorktreeName,
                    Requirement::text(true, &self.name),
                ),
            ],
            BranchSource::Existing => vec![(
                FieldId::AddWorktreeBranch,
                Requirement::choice(true, self.selected_branch.as_ref()),
            )],
        }
    }

    /// Plain-language description of what the create is currently doing (FR-024), or `None`
    /// before the first stage lands.
    pub fn stage_label(&self) -> Option<&'static str> {
        self.stage.map(|s| s.label(&self.mode))
    }

    /// Whether pressing Create is a submit attempt right now (feature 937, FR-012).
    ///
    /// An empty required field does not disable the button: pressing it shows the required errors
    /// and submits nothing ([`submitted`]). What disables it is a create in flight, a prompt, or a
    /// blocked branch.
    ///
    /// A blocked candidate is deliberately still *selectable* (research R8 — `pick_list` has no
    /// per-item disabling, and forking a list widget is what the Component-reuse gate rejects),
    /// so the refusal happens here, at the point of action (FR-012).
    pub fn can_press(&self) -> bool {
        if self.status != WorktreeFormStatus::Editing || self.resolution.is_prompting() {
            return false;
        }
        if let Some(candidate) = &self.selected_branch {
            if self.source == BranchSource::Existing && !candidate.is_available() {
                return false;
            }
        }
        true
    }

    /// The mode implied by picking a candidate outright, when no prompt is needed
    /// (contract `branch-picker.md` §5).
    ///
    /// Picking a branch IS the intent to use it — but never the intent to destroy it, so this
    /// can never yield [`CreateMode::Overwrite`].
    /// `preferred_remote` is the remote the user already named by picking a specific row in the
    /// branch list. When the name exists on several remotes and no preference is given, this
    /// returns `None` so the prompt opens and the user chooses — the app must never pick a
    /// remote on the user's behalf (spec Edge Cases).
    pub fn mode_for(
        situation: &BranchSituation,
        preferred_remote: Option<&str>,
    ) -> Option<CreateMode> {
        match situation {
            BranchSituation::Free => Some(CreateMode::NewBranch),
            BranchSituation::LocalAvailable { .. } => Some(CreateMode::ReuseLocal),
            BranchSituation::RemoteOnly { remotes, .. } => {
                let remote = match preferred_remote {
                    // Honour the picked row, but only if that remote really carries the ref.
                    Some(preferred) if remotes.iter().any(|r| r == preferred) => preferred,
                    // Unambiguous: exactly one remote has it.
                    None if remotes.len() == 1 => remotes[0].as_str(),
                    // Ambiguous, or a preference that no longer holds — ask.
                    _ => return None,
                };
                Some(CreateMode::TrackRemote {
                    remote: remote.to_string(),
                })
            }
            BranchSituation::Blocked { .. } | BranchSituation::DirectoryTaken { .. } => None,
        }
    }
}

/// The add-worktree form, as a floating surface (feature 021, T032).
///
/// Carries whether a create is in flight, because that decides how it may be closed (feature 013,
/// BUG-001): `dismissal` is asked of the surface value, not of the state it was read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddWorktreeDialog {
    creating: bool,
}

impl FloatingSurface for AddWorktreeDialog {
    fn id(&self) -> SurfaceId {
        SurfaceId::new("add_worktree")
    }

    fn layer(&self) -> Layer {
        Layer::Dialog
    }

    /// An ordinary dialog while the user is editing. While a create is in flight it protects its
    /// input (FR-010a): a stray scrim click or Escape would take the progress display away and
    /// leave the outcome nowhere to land, so only the in-dialog Cancel closes it.
    fn dismissal(&self) -> DismissalRules {
        let rules = DismissalRules::for_layer(Layer::Dialog)
            .cancelled_by(Message::WorktreeForm(Msg::Cancelled));
        if self.creating {
            rules.protecting_input()
        } else {
            rules
        }
    }
}

impl Registered for AddWorktreeDialog {
    fn open_in(state: &crate::app::State) -> Option<Self> {
        state
            .worktree_form
            .form
            .as_ref()
            .map(|form| AddWorktreeDialog {
                creating: form.status == WorktreeFormStatus::Creating,
            })
    }
}

/// The Add Worktree form was opened (feature 005, FR-005).
pub fn opened(state: &mut crate::app::State) {
    state.clear_for_dialog();
    let daemons: Vec<DaemonChoice> = state
        .settings
        .daemons
        .entries()
        .iter()
        .map(|entry| DaemonChoice {
            id: entry.id,
            name: entry.name.as_str().to_string(),
        })
        .collect();
    let daemon = state
        .settings
        .legacy_default_daemon
        .filter(|id| daemons.iter().any(|c| c.id == *id))
        .or_else(|| daemons.first().map(|c| c.id));
    let holder_daemons = state
        .worktree
        .worktrees
        .iter()
        .filter_map(|w| {
            let folder = w.path.file_name()?.to_str()?.to_string();
            Some((folder, state.daemon_label_of(&w.dir_name)))
        })
        .collect();
    // The CLIs offered are the ones known to exist at the home directory (the worktree has no
    // directory yet); "not asked yet" offers every supported one. The default is preselected when
    // offered, else the first.
    let known = state.session.known_clis(None);
    let session_clis: Vec<AiCli> = if known.is_empty() {
        AiCli::ALL.to_vec()
    } else {
        known.to_vec()
    };
    let session_cli = Some(state.session.default_ai_cli)
        .filter(|cli| session_clis.contains(cli))
        .or_else(|| session_clis.first().copied());
    let session_choices = session_clis
        .into_iter()
        .map(SessionChoice::Cli)
        .chain(std::iter::once(SessionChoice::NoSession))
        .collect();
    let jira = JiraAvailability::from_settings(
        state.settings.jira_enabled,
        &state.settings.jira_connections,
        state.workspace.active.as_deref(),
    );
    let remembered_provider = state
        .workspace
        .active
        .as_ref()
        .and_then(|project| state.workspace.issue_sources.get(project))
        .copied();
    let mut form = WorktreeForm {
        default_prompt_template: state.session.default_prompt_template.clone(),
        jira,
        github_issues_off: !state.settings.github_issues_enabled,
        remembered_provider,
        daemons,
        daemon,
        holder_daemons,
        session_cli,
        session_choices,
        ..WorktreeForm::default()
    };
    // Before the remotes answer, a switched-off GitHub must not leave Jira unchosen.
    form.resolve_provider();
    form.refresh_default_prompt();
    state.worktree_form.form = Some(form);
    state.worktree_form.worktree_error = None;
}

/// The form was dismissed.
pub fn cancelled(state: &mut crate::app::State) {
    // Only a form with a create in flight has an outcome still to come. An idle one leaves any
    // earlier cancelled create's record alone: that create is still running (FR-010b).
    if let Some(form) = state.worktree_form.form.take() {
        if form.status == WorktreeFormStatus::Creating {
            state.worktree_form.cancelled_create = Some(CancelledCreate {
                mode: form.mode,
                stage: form.stage,
            });
        }
    }
}

/// Apply a change to the open form, whatever it is doing.
fn with_form(state: &mut crate::app::State, change: impl FnOnce(&mut WorktreeForm)) {
    if let Some(form) = &mut state.worktree_form.form {
        change(form);
    }
}

/// Apply a change only while the form is accepting edits.
///
/// A create in flight makes the whole form inactive, not just its submit button (feature 010
/// follow-up), so every input arm is guarded — which is why the guard is written once here rather
/// than nine times.
fn while_editing(state: &mut crate::app::State, change: impl FnOnce(&mut WorktreeForm)) {
    with_form(state, |form| {
        if form.status == WorktreeFormStatus::Editing {
            change(form);
        }
    });
}

/// Apply a change only while the form is accepting edits **and** no conflict prompt is up.
///
/// Invariant 4 (feature 016): a resolution prompt and ordinary editing cannot both be live, so the
/// inputs behind the prompt are inert while it is showing.
fn while_editing_unprompted(state: &mut crate::app::State, change: impl FnOnce(&mut WorktreeForm)) {
    while_editing(state, |form| {
        if !form.resolution.is_prompting() {
            change(form);
        }
    });
}

/// A conventional type was chosen.
pub fn type_selected(state: &mut crate::app::State, type_: ConventionalType) {
    while_editing(state, |form| {
        form.type_ = Some(type_);
        form.error = None;
        form.refresh_default_prompt();
    });
}

/// The session's AI CLI was chosen, or "None" (feature 776, FR-006). Choosing none drops the
/// prompt: a prompt without a CLI is not representable.
pub fn session_cli_chosen(state: &mut crate::app::State, cli: Option<AiCli>) {
    while_editing(state, |form| {
        form.choose_session_cli(cli);
    });
}

/// The first-prompt field was edited. Ignored with no CLI chosen.
pub fn first_prompt_changed(state: &mut crate::app::State, text: String) {
    while_editing(state, |form| {
        form.edit_first_prompt(text);
    });
}

/// The ticket field was edited.
pub fn ticket_changed(state: &mut crate::app::State, text: String) {
    while_editing(state, |form| {
        form.ticket = text;
        form.error = None;
        form.refresh_default_prompt();
    });
}

/// The name field was edited.
pub fn name_changed(state: &mut crate::app::State, text: String) {
    while_editing(state, |form| {
        form.name = text;
        form.error = None;
        form.refresh_default_prompt();
    });
}

/// The form was submitted — **validated only** (FR-008).
///
/// The shell performs the git create on a valid form and dispatches `WorktreeCreated` /
/// `WorktreeCreateFailed`. A create already in flight makes this a no-op, so there is no
/// double-submit.
pub fn submitted(state: &mut crate::app::State) {
    while_editing(state, |form| {
        // Every required field that is empty reads `Missing` at once, not the first one only
        // (feature 937, FR-006). Emptiness is then the fields' own error; the form keeps the
        // refusals that are not about emptiness, such as a name with no letters or digits (FR-010).
        form.required.attempt();
        let empty = form
            .required_fields()
            .iter()
            .any(|(_, r)| r.required && r.empty);
        if let Err(error) = form.preview() {
            form.error = (!empty).then_some(error);
        }
    });
}

/// The branch source switched between a new and an existing branch (feature 016, FR-015).
///
/// Leaving the picker drops its selection, so no stale branch can be submitted from the new-branch
/// inputs — and takes the search with it, so returning never resumes someone else's half-finished
/// query.
///
/// The issue source can be chosen only once the repository is known to be on GitHub (invariant 1):
/// choosing it hands out a fresh load seq, and leaving it forgets the issues, so a load still in
/// flight becomes stale (invariant 7). Choosing the issue source while it is already chosen changes
/// nothing, so a second press cannot start a second load.
pub fn source_changed(state: &mut crate::app::State, source: BranchSource) {
    let next_seq = state.worktree_form.issue_request_seq + 1;
    let mut started = false;
    while_editing_unprompted(state, |form| {
        if source == BranchSource::Issue
            && (form.source == BranchSource::Issue || !form.issue_source_ready())
        {
            return;
        }
        form.source = source;
        form.error = None;
        if source != BranchSource::Existing {
            form.selected_branch = None;
        }
        form.reset_branch_search();
        form.reset_issues();
        if source == BranchSource::Issue {
            form.issues = IssueList::Loading { seq: next_seq };
            started = true;
        }
        form.refresh_default_prompt();
    });
    if started {
        state.worktree_form.issue_request_seq = next_seq;
    }
}

/// The user chose which issue source the dialog lists (feature 533, FR-016, FR-017).
///
/// Only offered, and so only accepted, while both sources are available, and a repeat of the
/// provider already chosen changes nothing. The list and the search are forgotten and a fresh
/// load seq is handed out, so the other provider's load still in flight is stale (invariant 2);
/// with the Issue source showing, the new provider's load starts at once. The choice is reported
/// so it is remembered for the project.
pub fn issue_provider_changed(
    state: &mut crate::app::State,
    provider: IssueProvider,
) -> Vec<crate::features::Outcome> {
    let next_seq = state.worktree_form.issue_request_seq + 1;
    let mut changed = false;
    while_editing_unprompted(state, |form| {
        if !form.offers_issue_provider_choice() || form.provider == provider {
            return;
        }
        form.provider = provider;
        form.remembered_provider = Some(provider);
        form.reset_issues();
        if form.source == BranchSource::Issue {
            form.issues = IssueList::Loading { seq: next_seq };
        }
        changed = true;
    });
    if !changed {
        return Vec::new();
    }
    state.worktree_form.issue_request_seq = next_seq;
    state
        .workspace
        .active
        .clone()
        .map(|project| crate::features::Outcome::IssueSourceChosen { project, provider })
        .into_iter()
        .collect()
}

/// The daemon answered the form's `RemoteList` (feature 034, FR-002).
pub fn remotes_listed(state: &mut crate::app::State, remotes: Result<Vec<GitRemote>, String>) {
    let next_seq = state.worktree_form.issue_request_seq + 1;
    let mut bumped = false;
    with_form(state, |form| {
        let before = form.provider;
        form.github = match remotes {
            Ok(remotes) => match choose_remote(&remotes) {
                RemoteChoice::Github { repo, .. } => GithubAvailability::Available(repo),
                RemoteChoice::NoGithubRemote => GithubAvailability::Unavailable(
                    "This repository has no GitHub remote.".to_string(),
                ),
            },
            Err(detail) => GithubAvailability::Unavailable(format!(
                "Couldn't read this repository's remotes: {detail}"
            )),
        };
        form.resolve_provider();
        if form.provider != before {
            form.reset_issues();
            if form.source == BranchSource::Issue {
                form.source = BranchSource::New;
            }
            bumped = true;
        }
    });
    if bumped {
        state.worktree_form.issue_request_seq = next_seq;
    }
}

/// What the settings in force now make of Jira changed while the form is open (feature 533,
/// data-model §4, FR-005): the provider is chosen again, and a Jira list that no longer applies is
/// forgotten with its in-flight load, which a bumped seq makes stale (invariants 1 and 4).
pub fn jira_settings_changed(state: &mut crate::app::State, jira: JiraAvailability) {
    let next_seq = state.worktree_form.issue_request_seq + 1;
    let mut bumped = false;
    with_form(state, |form| {
        if form.jira == jira {
            return;
        }
        let was_jira = form.provider == IssueProvider::Jira;
        form.jira = jira;
        form.resolve_provider();
        if was_jira || form.provider == IssueProvider::Jira {
            form.reset_issues();
            if form.source == BranchSource::Issue {
                form.source = BranchSource::New;
            }
            bumped = true;
        }
    });
    if bumped {
        state.worktree_form.issue_request_seq = next_seq;
    }
}

/// The GitHub issues switch changed while the form is open (feature 1102, FR-005): the provider is
/// chosen again and a GitHub list that no longer applies is forgotten with its in-flight load,
/// which a bumped seq makes stale. The remotes already read are kept, so on again offers GitHub
/// without asking git once more.
pub fn github_issues_changed(state: &mut crate::app::State, enabled: bool) {
    let next_seq = state.worktree_form.issue_request_seq + 1;
    let mut bumped = false;
    with_form(state, |form| {
        if form.github_issues_off == !enabled {
            return;
        }
        let was_github = form.provider == IssueProvider::Github;
        form.github_issues_off = !enabled;
        form.resolve_provider();
        if was_github || form.provider == IssueProvider::Github {
            form.reset_issues();
            if form.source == BranchSource::Issue {
                form.source = BranchSource::New;
            }
            bumped = true;
        }
    });
    if bumped {
        state.worktree_form.issue_request_seq = next_seq;
    }
}

/// An issue load finished. Only the awaited one applies (invariant 2, FR-007a).
pub fn issues_loaded(
    state: &mut crate::app::State,
    seq: u64,
    result: Result<(IssueListing, PathBuf), IssueLoadError>,
) {
    with_form(state, |form| {
        if form.issues != (IssueList::Loading { seq }) {
            return;
        }
        match result {
            Ok((listing, gh)) => {
                // The list arrived without descriptions; they follow under this load's seq, so
                // a later load's pass can never take this one's pages (038 FR-024).
                let descriptions =
                    if listing.issues.is_empty() || form.provider == IssueProvider::Jira {
                        DescriptionPass::Done
                    } else {
                        DescriptionPass::Loading {
                            seq,
                            cursor: None,
                            pages: 0,
                        }
                    };
                form.issues = IssueList::Loaded {
                    listing,
                    gh,
                    searched: Vec::new(),
                    search: SearchState::Idle,
                    descriptions,
                };
                form.issue_list_open = true;
                form.rematch_issues();
            }
            Err(error) => form.issues = IssueList::Failed { error },
        }
    });
}

/// A page of the description pass arrived (038 FR-024, contracts/issue-fields.md §6). Only the
/// page the pass awaits applies: the one asked under the load's `seq`, from `cursor`.
///
/// The page's descriptions go onto the loaded issues and the pass moves to the next cursor, or
/// ends. A failure ends it and changes nothing else: a tooltip is a convenience, so no error is
/// shown and nothing is retried (ledger D15). A description is not match text, so the matches,
/// the highlight and the open list are left as they are.
pub fn issue_descriptions_loaded(
    state: &mut crate::app::State,
    seq: u64,
    cursor: Option<String>,
    result: Result<DescriptionPage, IssueLoadError>,
) {
    with_form(state, |form| {
        let IssueList::Loaded {
            listing,
            descriptions,
            ..
        } = &mut form.issues
        else {
            return;
        };
        let DescriptionPass::Loading {
            seq: awaited_seq,
            cursor: awaited_cursor,
            pages,
        } = &*descriptions
        else {
            return;
        };
        if *awaited_seq != seq || *awaited_cursor != cursor {
            return;
        }
        let read = pages + 1;
        *descriptions = match result {
            Ok(page) => {
                describe_listed(&mut listing.issues, &page);
                match next_description_cursor(cursor.as_deref(), &page, read) {
                    Some(next) => DescriptionPass::Loading {
                        seq,
                        cursor: Some(next),
                        pages: read,
                    },
                    None => DescriptionPass::Done,
                }
            }
            Err(_) => DescriptionPass::Done,
        };
    });
}

/// Retry a failed load, or a failed search beyond the loaded issues, with a fresh seq (invariant
/// 1). Nothing else is retried: a first load is started by choosing the source, never from here.
pub fn issue_retry(state: &mut crate::app::State) {
    let next_seq = state.worktree_form.issue_request_seq + 1;
    let mut started = false;
    while_editing_unprompted(state, |form| {
        if form.source != BranchSource::Issue {
            return;
        }
        match &mut form.issues {
            IssueList::Failed { .. } => {
                form.issues = IssueList::Loading { seq: next_seq };
                started = true;
            }
            IssueList::Loaded { search, .. } if matches!(search, SearchState::Failed { .. }) => {
                *search = SearchState::Searching { seq: next_seq };
                started = true;
            }
            _ => {}
        }
    });
    if started {
        state.worktree_form.issue_request_seq = next_seq;
    }
}

/// The issue search text changed (FR-005). The held issues are re-ranked at once. When the list
/// is capped and something is typed, a search beyond it waits out the debounce under a fresh seq,
/// which makes any older search stale (FR-005a, FR-007a, invariant 6). A change of surrounding
/// whitespace alone keeps the search as it is; clearing the text forgets the searched issues.
pub fn issue_query_changed(state: &mut crate::app::State, text: String) {
    let next_seq = state.worktree_form.issue_request_seq + 1;
    let mut started = false;
    while_editing_unprompted(state, |form| {
        // Whitespace around the text changes nothing GitHub would be asked (review A #6).
        let same_search = form.issue_query.trim() == text.trim();
        form.issue_query = text;
        form.issue_list_open = true;
        let typed = !form.issue_query.trim().is_empty();
        if let IssueList::Loaded {
            listing,
            searched,
            search,
            ..
        } = &mut form.issues
        {
            if !typed {
                // Nothing typed: the list is the loaded issues alone again.
                searched.clear();
                *search = SearchState::Idle;
            } else if !listing.complete && !same_search {
                started = true;
                *search = SearchState::Pending { seq: next_seq };
            }
        }
        form.rematch_issues();
    });
    if started {
        state.worktree_form.issue_request_seq = next_seq;
    }
}

/// The debounce ran out: search, if this is still the latest keystroke's (research R9).
pub fn issue_search_due(state: &mut crate::app::State, seq: u64) {
    // Even while a create runs: the keystroke was the named event, and a search left pending here
    // would never run once the form is back to editing.
    with_form(state, |form| {
        if let IssueList::Loaded { search, .. } = &mut form.issues {
            if *search == (SearchState::Pending { seq }) {
                *search = SearchState::Searching { seq };
            }
        }
    });
}

/// The search beyond the loaded issues answered. Only the awaited answer applies (FR-007a); its
/// issues replace the previous search's, less any already loaded (invariant 4), and are ranked
/// with the loaded ones, so one that does not match the query is not shown (invariant 5). A
/// failure leaves the loaded matches in place.
pub fn issue_searched(
    state: &mut crate::app::State,
    seq: u64,
    result: Result<Vec<Issue>, IssueLoadError>,
) {
    with_form(state, |form| {
        let IssueList::Loaded {
            listing,
            searched,
            search,
            ..
        } = &mut form.issues
        else {
            return;
        };
        if *search != (SearchState::Searching { seq }) {
            return;
        }
        match result {
            Ok(found) => {
                *searched = merge_searched(&listing.issues, found);
                *search = SearchState::Idle;
            }
            Err(error) => *search = SearchState::Failed { error },
        }
        // The answer re-ranks the list under the keyboard: keep the highlight on its issue, so
        // Enter picks what the user is looking at (review A #1).
        let highlighted = form
            .issue_highlight
            .and_then(|row| form.issue_number_at(row));
        form.rematch_issues();
        // An issue the answer dropped takes the highlight with it: a row the user never chose must
        // not be under Enter.
        if let Some(number) = highlighted {
            form.issue_highlight = (0..form.issue_matches.len())
                .find(|row| form.issue_number_at(*row) == Some(number));
        }
    });
}

/// The issue search field took focus.
pub fn issue_focused(state: &mut crate::app::State) {
    while_editing_unprompted(state, |form| form.issue_list_open = true);
}

/// The keyboard moved through the issue results (saturating, like the branch picker).
pub fn issue_highlight_moved(state: &mut crate::app::State, direction: Direction) {
    with_form(state, |form| {
        if let Some(next) =
            move_highlight(form.issue_highlight, direction, form.issue_matches.len())
        {
            form.issue_highlight = Some(next);
        }
    });
}

/// The issue list closed without a pick.
pub fn issue_dismissed(state: &mut crate::app::State) {
    with_form(state, |form| form.issue_list_open = false);
}

/// An issue was picked: its number becomes the ticket and its title the name, replacing whatever
/// was there (FR-009, FR-010, FR-010a), and its labels choose the type through `mapping` — a match
/// replaces the selected type, no match clears it (FR-013, FR-014). A number the listing does not
/// hold changes nothing.
pub fn issue_picked(state: &mut crate::app::State, number: u64, mapping: &[LabelTypeEntry]) {
    while_editing_unprompted(state, |form| {
        let Some(naming) = form
            .issues
            .held()
            .into_iter()
            .find(|issue| issue.number() == number)
            .map(|issue| naming_for_issue(issue, mapping))
        else {
            return;
        };
        form.ticket = naming.ticket.unwrap_or_default();
        form.name = naming.name;
        form.type_ = naming.type_;
        form.error = None;
        form.set_picked_issue(Some(number));
        form.issue_list_open = false;
        form.refresh_default_prompt();
    });
}

/// The picked issue's details answered (837). Applies only while the form awaits exactly this
/// `seq` and `number`; anything else is an answer for an earlier pick or a closed form (FR-012).
pub fn issue_details_loaded(
    state: &mut crate::app::State,
    seq: u64,
    number: u64,
    result: Result<Box<IssueDetails>, IssueLoadError>,
) {
    with_form(state, |form| {
        if form.issue_details != (IssueDetailsState::Loading { seq, number }) {
            return;
        }
        let Some(repo) = form.github_repo().cloned() else {
            return;
        };
        form.issue_details = match result {
            Ok(details) => IssueDetailsState::Loaded { number, details },
            Err(error) => IssueDetailsState::Failed {
                number,
                message: error.message(&repo),
            },
        };
    });
}

/// Branch candidates arrived (feature 016).
///
/// Re-matched immediately, so the results describe the current query whenever they land.
pub fn branches_listed(state: &mut crate::app::State, candidates: Vec<BranchCandidate>) {
    with_form(state, |form| {
        form.candidates = candidates;
        form.rematch_branches();
    });
}

/// A branch was chosen from the picker (feature 016, FR-012a/FR-014a).
///
/// A branch held elsewhere cannot be chosen — silently, and without closing the list, because a
/// press that does nothing must not look like a press that did something. The query is deliberately
/// left alone.
pub fn branch_selected(state: &mut crate::app::State, candidate: BranchCandidate) {
    while_editing_unprompted(state, |form| {
        if !candidate.is_available() {
            return;
        }
        form.selected_branch = Some(candidate);
        form.error = None;
        form.branch_list_open = false;
        form.refresh_default_prompt();
    });
}

/// The branch field took focus, revealing the picker.
pub fn branch_focused(state: &mut crate::app::State) {
    while_editing_unprompted(state, |form| form.branch_list_open = true);
}

/// The branch search query was edited (feature 021).
pub fn branch_query_changed(state: &mut crate::app::State, text: String) {
    while_editing_unprompted(state, |form| {
        form.branch_query = text;
        form.branch_list_open = true;
        form.rematch_branches();
    });
}

/// The picker's highlight moved (feature 021, FR-017a/FR-021).
///
/// Saturating, not wrapping, and the rule itself is `micold_core`'s rather than this module's. An
/// empty list has nowhere to land, so the highlight is left exactly as it was.
pub fn branch_highlight_moved(state: &mut crate::app::State, direction: Direction) {
    with_form(state, |form| {
        let rows = form.branch_matches.len();
        if let Some(next) = move_highlight(form.branch_highlight, direction, rows) {
            form.branch_highlight = Some(next);
        }
    });
}

/// The branch picker was dismissed.
pub fn branch_dismissed(state: &mut crate::app::State) {
    with_form(state, |form| {
        form.branch_list_open = false;
        // Closing the picker is leaving it (feature 937, research R2).
        form.required.leave(FieldId::AddWorktreeBranch);
    });
}

/// A branch conflict was detected (feature 016).
///
/// Invariant 4: a prompt and an in-flight create cannot coexist.
pub fn conflict_detected(state: &mut crate::app::State, situation: BranchSituation) {
    while_editing(state, |form| {
        form.resolution = ResolutionState::Choosing { situation };
    });
}

/// Overwrite was requested from the choice (feature 016, FR-005).
///
/// Only ever from `Choosing`, and only for a situation that *has* a local branch to overwrite —
/// invariant 1.
pub fn overwrite_requested(state: &mut crate::app::State) {
    with_form(state, |form| {
        if let ResolutionState::Choosing { situation } = &form.resolution {
            if matches!(situation, BranchSituation::LocalAvailable { .. }) {
                form.resolution = ResolutionState::ConfirmingOverwrite {
                    situation: situation.clone(),
                };
            }
        }
    });
}

/// Overwrite was confirmed — the **only** route to `CreateMode::Overwrite`.
///
/// The shell picks the resolution up and runs the create; this clears the prompt.
pub fn overwrite_confirmed(state: &mut crate::app::State) {
    with_form(state, |form| {
        if matches!(form.resolution, ResolutionState::ConfirmingOverwrite { .. }) {
            form.resolution = ResolutionState::Idle;
        }
    });
}

/// A resolution was chosen (feature 016, invariant 1).
///
/// Overwrite must go through the confirmation and never straight from the choice — rejected here
/// rather than trusting call sites.
pub fn resolution_chosen(state: &mut crate::app::State, mode: CreateMode) {
    with_form(state, |form| {
        let allowed = !matches!(mode, CreateMode::Overwrite)
            && matches!(form.resolution, ResolutionState::Choosing { .. });
        if allowed {
            form.resolution = ResolutionState::Idle;
        }
    });
}

/// A resolution prompt was backed out of (feature 016, invariant 3, US2 AS3).
///
/// Backing out of the confirmation returns to the choice, not to the form. Cancelling the choice
/// leaves every input exactly as it was (FR-007).
pub fn resolution_cancelled(state: &mut crate::app::State) {
    with_form(state, |form| {
        form.resolution = match &form.resolution {
            ResolutionState::ConfirmingOverwrite { situation } => ResolutionState::Choosing {
                situation: situation.clone(),
            },
            _ => ResolutionState::Idle,
        };
    });
}

/// A create started (feature 010).
///
/// A new attempt never inherits the previous one's stage.
pub fn create_started(state: &mut crate::app::State, mode: CreateMode) {
    // A new attempt never inherits the previous one's message either. The stale error line used to
    // stand through the whole of the next attempt's pending state — an error and a progress bar on
    // screen together, describing different attempts (BUG-020).
    state.worktree_form.worktree_error = None;
    with_form(state, |form| {
        form.status = WorktreeFormStatus::Creating;
        form.mode = mode;
        form.stage = None;
        form.stage_detail = None;
        form.interrupted = None;
    });
}

/// A create reported progress (feature 010).
///
/// Entering a stage clears the previous stage's trailing line — it described work that is over. A
/// detail-only push keeps the stage and replaces the line.
pub fn create_stage_changed(
    state: &mut crate::app::State,
    stage: CreateStage,
    detail: Option<String>,
) {
    // A create whose form was cancelled runs on; its failure must name where it really got to.
    if let Some(create) = &mut state.worktree_form.cancelled_create {
        create.stage = Some(stage);
    }
    with_form(state, |form| {
        if form.stage != Some(stage) {
            form.stage = Some(stage);
            form.stage_detail = None;
        }
        if detail.is_some() {
            form.stage_detail = detail;
        }
    });
}

/// A worktree was created (feature 005, FR-017).
///
/// Idempotent by directory name, and sorted so it lands where the list would have put it. A form
/// cancelled mid-create never saw it succeed, so the success is announced (feature 013, FR-010b).
pub fn created(state: &mut crate::app::State, worktree: Worktree) -> Vec<crate::features::Outcome> {
    let owed = state.worktree_form.cancelled_create.take();
    let announce = owed.is_some() && state.worktree_form.form.is_none();
    state.worktree_form.form = None;
    state.worktree_form.worktree_error = None;
    let notice = announce.then(|| {
        crate::features::notifications::info(format!("Worktree \"{}\" created.", worktree.dir_name))
    });
    let mut outcomes = vec![crate::features::Outcome::WorktreeCreated(worktree)];
    outcomes.extend(notice);
    outcomes
}

/// The worktree list changed, so a create failure shown against the old one is stale (T067a-4).
///
/// `worktree_error` is the add-worktree modal's error line — `crate::ui::worktree_form` is its only
/// render site — so clearing it is the form's own business even when a *worktree* operation is what
/// makes it stale. Reached from the root's `WorktreesReplaced` arm.
pub fn worktree_list_changed(state: &mut crate::app::State) {
    state.worktree_form.worktree_error = None;
}

/// A create failed (feature 005 FR-017, feature 010).
///
/// The form stays open so the user can adjust, showing the error, and returns to `Editing` so a
/// retry is possible instead of being stuck in `Creating`. A form cancelled mid-create is not
/// there to show it, so the failure becomes a notification instead (feature 013, FR-010b).
pub fn create_failed(
    state: &mut crate::app::State,
    message: String,
) -> Vec<crate::features::Outcome> {
    let owed = state.worktree_form.cancelled_create.take();
    if let (Some(create), None) = (owed, &state.worktree_form.form) {
        return vec![crate::features::notifications::error(
            create.failure(&message),
        )];
    }
    state.worktree_form.worktree_error = Some(message);
    with_form(state, |form| {
        form.status = WorktreeFormStatus::Editing;
    });
    Vec::new()
}

/// A create's connection dropped before the daemon answered it (feature 010, BUG-020).
///
/// Not a failure: the daemon applies its mutation before replying, so the request may well have
/// taken effect — the worktree in the report's reproduction really was created. What is knowable is
/// that nothing on this connection will ever say which, so the form stops claiming the operation is
/// running and says so instead. The inputs stay exactly as they were, because the user may want to
/// retry and because a drop is not a reason to discard what they typed (FR-007).
pub fn create_interrupted(state: &mut crate::app::State, message: String) {
    with_form(state, |form| {
        form.status = WorktreeFormStatus::Editing;
        form.stage = None;
        form.stage_detail = None;
        form.interrupted = Some(message);
    });
}

/// Everything the add-worktree wizard says about itself (feature 021, T064 — FR-003).
///
/// # Why this feature nests and no other does
///
/// FR-003 permits a nested message type **only** where a feature "is opened, edited and dismissed
/// as a unit whose intermediate state no other feature reads". research.md §5 tested all ten
/// features against that bar and exactly one cleared it *and* was large enough for nesting to pay:
/// this one. Its 22 variants were 17% of the root enum, and nothing outside `ui/worktree_form.rs`
/// and the generic overlay snapshot ever read `state.worktree_form.form`.
///
/// Settings clears the same bar on the same evidence and is deliberately **not** nested: 7
/// variants over a flat four-field draft, where a wrapper and a routing arm cost about what they
/// save. FR-004b permits concluding a reducer module suffices, and §5 records that conclusion with
/// its evidence rather than leaving it implicit.
///
/// # The variants kept their meaning and lost their prefix
///
/// `AddWorktreeTicketChanged` is `Msg::TicketChanged` here: the type says which form, so the
/// variant does not have to. The four `WorktreeCreate*` variants joined them — the create is the
/// last step of this wizard, not a separate concern, which is why §5 counted 22 and not 18.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// Open the add-worktree form (FR-005).
    Opened,
    /// A Kubernetes daemon is cloning the project's remote before the create (feature 686,
    /// FR-007): the text to show while it runs.
    CloneStarted(String),
    /// The daemon picker changed (feature 491, FR-006).
    DaemonChosen(micold_core::daemons::DaemonId),
    /// The form's type selection changed.
    TypeSelected(ConventionalType),
    /// The form's ticket field changed.
    TicketChanged(String),
    /// The form's name field changed.
    NameChanged(String),
    /// The session's AI CLI was chosen; `None` is "None" (feature 776).
    SessionCliChosen(Option<AiCli>),
    /// The session's first prompt was edited (feature 776).
    FirstPromptChanged(String),
    /// Submit the form (FR-006). Validation happens here; the binary performs the git create.
    Submitted,
    /// Dismiss the form without creating (Cancel or Esc).
    Cancelled,
    /// Switch between the new-branch and existing-branch halves of the form (feature 016,
    /// FR-010). Switching back to `New` clears any selection (FR-015).
    SourceChanged(BranchSource),
    /// The binary listed the repository's branches for the picker (feature 016, FR-011).
    BranchesListed(Vec<BranchCandidate>),
    /// An existing branch was picked from the list (feature 016, FR-014).
    ///
    /// A blocked candidate is **ignored entirely** (feature 021, FR-012a): it does not become the
    /// selection and does not close the list. Feature 016 let it be selected and refused at the
    /// point of creating, because the list widget of the day could not disable a row; the
    /// type-ahead can, so the refusal moved to the point of choosing.
    BranchSelected(BranchCandidate),
    /// The branch search field took focus, so the list opens on what is already on offer (feature
    /// 021, FR-001b). Not a query change: focusing is not typing.
    BranchFocused,
    /// The branch search text changed (feature 021, FR-001, FR-005).
    BranchQueryChanged(String),
    /// The keyboard moved through the results (feature 021, FR-017). The saturating rule lives in
    /// `micold_core::typeahead`, not here — this arm applies its answer.
    BranchHighlightMoved(Direction),
    /// The result list closed without a pick — Escape, a press outside it, or Tab taking focus out
    /// of the field (feature 021, FR-001b). Three triggers, one effect.
    BranchDismissed,
    /// Pre-flight found something the user must decide about; raise the prompt (feature 016,
    /// FR-001). Never dispatched for [`BranchSituation::Free`].
    ConflictDetected(BranchSituation),
    /// The user answered the prompt (feature 016, FR-002). The binary performs the create with
    /// the chosen mode. `Overwrite` can only arrive via [`Msg::OverwriteConfirmed`].
    ResolutionChosen(CreateMode),
    /// The user chose Overwrite; show the destructive confirmation first (feature 016, FR-005).
    OverwriteRequested,
    /// The destructive confirmation was accepted (feature 016, FR-005).
    OverwriteConfirmed,
    /// Back out of the prompt (or its confirmation) without acting (feature 016, FR-007).
    ResolutionCancelled,
    /// The binary is about to send the `WorktreeCreate` RPC (feature 010; T055); marks the form
    /// `Creating` so it shows an in-progress state until the daemon's reply closes or reopens it.
    /// Carries the mode so the stage display can be worded for it (feature 016, FR-024).
    CreateStarted(CreateMode),
    /// The daemon reported progress on the in-flight create: a new stage (feature 016, FR-024), or
    /// — with the stage unchanged — its latest live output line, rate-limited daemon-side so a long
    /// stage reads as moving rather than frozen (BUG-009, T123). Ignored once the form has closed.
    CreateStageChanged(CreateStage, Option<String>),
    /// The daemon created a worktree successfully (FR-007); add it and close the form.
    Created(Worktree),
    /// The daemon reported a worktree create failure (FR-017); show it, keep the form open.
    CreateFailed(String),
    /// The connection carrying an in-flight create dropped, so its outcome is unknown (BUG-020);
    /// stop showing it as running and say so, keeping the form and its inputs.
    CreateInterrupted(String),
    /// The daemon answered the form's `RemoteList`, or it could not be asked (feature 034, FR-002).
    RemotesListed(Result<Vec<GitRemote>, String>),
    /// An issue load finished (feature 034). Applies only while the form awaits `seq`.
    /// `resolved_env` is the environment-include snapshot the load resolved on a cache miss, for
    /// the shell to keep; the reducer ignores it.
    IssuesLoaded {
        seq: u64,
        result: Result<(IssueListing, PathBuf), IssueLoadError>,
        resolved_env: Option<(PathBuf, EnvIncludeSnapshot)>,
    },
    /// The settings now make Jira available or not to this form (feature 533, FR-005).
    JiraSettingsChanged(JiraAvailability),
    /// The GitHub issues switch now in force changed (feature 1102, FR-005).
    GithubIssuesChanged(bool),
    /// The GitHub / Jira choice of a project with both sources (feature 533, FR-016, FR-017).
    IssueProviderChanged(IssueProvider),
    /// Retry a failed issue load (FR-007).
    IssueRetry,
    /// The issue search text changed (FR-005).
    IssueQueryChanged(String),
    /// The issue search field took focus.
    IssueFocused,
    /// The keyboard moved through the issue results.
    IssueHighlightMoved(Direction),
    /// The issue list closed without a pick.
    IssueDismissed,
    /// Row `index` of the shown issue results was picked; the shell resolves it to a number
    /// through [`State::issue_number_at`] and dispatches [`Msg::IssuePicked`].
    IssueRowPicked(usize),
    /// An issue was picked: its number and title fill ticket and name (FR-009, FR-010), and its
    /// labels choose the type through `mapping`, read by the shell at the pick (FR-013, FR-014a).
    IssuePicked {
        number: u64,
        mapping: Vec<LabelTypeEntry>,
    },
    /// The debounce after a keystroke ran out (research R9). Starts the search only while `seq` is
    /// still the pending one.
    IssueSearchDue { seq: u64 },
    /// The search beyond the loaded issues answered (FR-005a). Applies only while it awaits `seq`.
    IssueSearched {
        seq: u64,
        result: Result<Vec<Issue>, IssueLoadError>,
    },
    /// A page of the description pass answered (038 FR-024). Applies only while the pass awaits
    /// exactly this `seq` and `cursor`.
    IssueDescriptionsLoaded {
        seq: u64,
        cursor: Option<String>,
        result: Result<DescriptionPage, IssueLoadError>,
    },
    /// The picked issue's details answered (837). Applies only while the form awaits exactly this
    /// `seq` and `number`.
    IssueDetailsLoaded {
        seq: u64,
        number: u64,
        result: Result<Box<IssueDetails>, IssueLoadError>,
    },
}

/// The form's own reducer: one entry point, thirty-five answers (FR-004a).
///
/// The root sees a single arm. Everything the wizard knows about its own steps — which are inert
/// while a create is in flight, which are inert behind a conflict prompt, which reset the branch
/// search — lives on this side of the boundary and never had to be said out there.
pub fn update(state: &mut crate::app::State, msg: Msg) -> Vec<crate::features::Outcome> {
    match msg {
        Msg::Opened => opened(state),
        Msg::DaemonChosen(id) => with_form(state, |form| {
            if form.daemons.iter().any(|c| c.id == id) {
                form.daemon = Some(id);
            }
        }),
        Msg::TypeSelected(type_) => type_selected(state, type_),
        Msg::TicketChanged(text) => ticket_changed(state, text),
        Msg::NameChanged(text) => name_changed(state, text),
        Msg::SessionCliChosen(cli) => session_cli_chosen(state, cli),
        Msg::FirstPromptChanged(text) => first_prompt_changed(state, text),
        Msg::Submitted => submitted(state),
        Msg::Cancelled => cancelled(state),
        Msg::SourceChanged(source) => source_changed(state, source),
        Msg::BranchesListed(candidates) => branches_listed(state, candidates),
        Msg::BranchSelected(candidate) => branch_selected(state, candidate),
        Msg::BranchFocused => branch_focused(state),
        Msg::BranchQueryChanged(text) => branch_query_changed(state, text),
        Msg::BranchHighlightMoved(direction) => branch_highlight_moved(state, direction),
        Msg::BranchDismissed => branch_dismissed(state),
        Msg::ConflictDetected(situation) => conflict_detected(state, situation),
        Msg::ResolutionChosen(mode) => resolution_chosen(state, mode),
        Msg::OverwriteRequested => overwrite_requested(state),
        Msg::OverwriteConfirmed => overwrite_confirmed(state),
        Msg::ResolutionCancelled => resolution_cancelled(state),
        Msg::CreateStarted(mode) => create_started(state, mode),
        Msg::CloneStarted(text) => with_form(state, |form| form.stage_detail = Some(text)),
        Msg::CreateStageChanged(stage, detail) => create_stage_changed(state, stage, detail),
        Msg::Created(worktree) => return created(state, worktree),
        Msg::CreateFailed(message) => return create_failed(state, message),
        Msg::CreateInterrupted(message) => create_interrupted(state, message),
        Msg::RemotesListed(remotes) => remotes_listed(state, remotes),
        Msg::IssuesLoaded { seq, result, .. } => issues_loaded(state, seq, result),
        Msg::JiraSettingsChanged(jira) => jira_settings_changed(state, jira),
        Msg::GithubIssuesChanged(enabled) => github_issues_changed(state, enabled),
        Msg::IssueProviderChanged(provider) => return issue_provider_changed(state, provider),
        Msg::IssueRetry => issue_retry(state),
        Msg::IssueQueryChanged(text) => issue_query_changed(state, text),
        Msg::IssueFocused => issue_focused(state),
        Msg::IssueHighlightMoved(direction) => issue_highlight_moved(state, direction),
        Msg::IssueDismissed => issue_dismissed(state),
        // Resolved by the shell, which holds the index and the results together.
        Msg::IssueRowPicked(_) => {}
        Msg::IssuePicked { number, mapping } => issue_picked(state, number, &mapping),
        Msg::IssueSearchDue { seq } => issue_search_due(state, seq),
        Msg::IssueSearched { seq, result } => issue_searched(state, seq, result),
        Msg::IssueDescriptionsLoaded {
            seq,
            cursor,
            result,
        } => issue_descriptions_loaded(state, seq, cursor, result),
        Msg::IssueDetailsLoaded {
            seq,
            number,
            result,
        } => issue_details_loaded(state, seq, number, result),
    }
    Vec::new()
}
