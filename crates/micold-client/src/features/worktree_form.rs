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
use crate::overlay::registry::Registered;
use crate::overlay::{DismissalRules, FloatingSurface, SurfaceId};
use micold_core::env_include::EnvIncludeSnapshot;
use micold_core::git::GitRemote;
use micold_core::github::{
    choose_remote, merge_searched, GithubRepo, Issue, IssueListing, IssueLoadError, RemoteChoice,
};
use micold_core::naming::name_from_title;
use micold_core::naming::{
    derive, dir_name_from_branch, ConventionalType, DerivedNames, NamingError, WorktreeNaming,
};
use micold_core::overlay::Layer;
use micold_core::typeahead::{move_highlight, rank, Direction, Match, Query};
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

/// `1234567` as "1,234,567", for a count the user reads.
fn thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
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

/// In-progress add-worktree form state, present only while the form overlay is open (FR-005).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorktreeForm {
    /// Selected Conventional-Commits type (FR-005a).
    pub type_: Option<ConventionalType>,
    /// Optional ticket reference (FR-005b).
    pub ticket: String,
    /// Free-text name.
    pub name: String,
    /// The last validation error shown after a rejected submit.
    pub error: Option<NamingError>,
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
}

impl WorktreeForm {
    /// The issue number of row `index` of `issue_matches`.
    pub fn issue_number_at(&self, index: usize) -> Option<u64> {
        let (held, _) = self.issue_matches.get(index)?;
        self.issues.held().get(*held).map(|issue| issue.number())
    }

    /// The caption under the source switch (FR-002, FR-025): why the issue chip is disabled, or —
    /// before it is chosen — which repository choosing it reads from GitHub. `None` once the issue
    /// source is chosen, when [`Self::issue_notice`] says it instead.
    pub fn source_caption(&self) -> Option<String> {
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
        match (&self.github, self.source) {
            (GithubAvailability::Available(repo), BranchSource::Issue) => {
                Some(format!("Reads open issues of {repo} from GitHub."))
            }
            _ => None,
        }
    }

    /// The caption under a list that does not hold every open issue (FR-004): how many were loaded
    /// and how many are open. `None` when every open issue is held.
    pub fn issue_cap_caption(&self) -> Option<String> {
        match &self.issues {
            IssueList::Loaded { listing, .. } if !listing.complete => Some(format!(
                "Showing the {} most recently updated of {} open issues — search also looks on GitHub.",
                thousands(listing.issues.len() as u64),
                thousands(listing.total_open)
            )),
            _ => None,
        }
    }

    /// The line under the picker while the search beyond the loaded issues runs or after it failed
    /// (FR-005a, FR-007). `None` otherwise.
    pub fn issue_search_status(&self) -> Option<String> {
        match (&self.issues, &self.github) {
            (
                IssueList::Loaded {
                    search: SearchState::Searching { .. },
                    ..
                },
                _,
            ) => Some("Searching GitHub…".to_string()),
            (
                IssueList::Loaded {
                    search: SearchState::Failed { error },
                    ..
                },
                GithubAvailability::Available(repo),
            ) => Some(format!(
                "Search beyond the loaded issues failed — {}",
                error.message(repo)
            )),
            _ => None,
        }
    }

    /// The repository the issue source reads, once known.
    pub fn github_repo(&self) -> Option<&GithubRepo> {
        match &self.github {
            GithubAvailability::Available(repo) => Some(repo),
            _ => None,
        }
    }

    /// Re-rank the held issues against `issue_query`, re-seating the highlight (invariant 3).
    fn rematch_issues(&mut self) {
        let held = self.issues.held();
        let query = Query::new(&self.issue_query);
        self.issue_matches = rank(&held, |issue| issue.row_text(), &query);
        self.issue_highlight = match self.issue_highlight {
            Some(_) if self.issue_matches.is_empty() => None,
            Some(i) if i >= self.issue_matches.len() => Some(0),
            other => other,
        };
    }

    /// Forget the issues and their search: the source was left, so any load in flight is stale.
    fn reset_issues(&mut self) {
        self.issues = IssueList::NotRequested;
        self.issue_query.clear();
        self.issue_matches.clear();
        self.issue_list_open = false;
        self.issue_highlight = None;
        self.picked_issue = None;
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
        self.branch_highlight = match self.branch_highlight {
            // The list shrank under the highlight. Dropping to the first row keeps the keyboard
            // somewhere real; leaving it dangling would let the next Enter pick a row that is no
            // longer shown.
            Some(_) if self.branch_matches.is_empty() => None,
            Some(i) if i >= self.branch_matches.len() => Some(0),
            other => other,
        };
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

    /// Plain-language description of what the create is currently doing (FR-024), or `None`
    /// before the first stage lands.
    pub fn stage_label(&self) -> Option<&'static str> {
        self.stage.map(|s| s.label(&self.mode))
    }

    /// Whether the form can be submitted right now.
    ///
    /// A blocked candidate is deliberately still *selectable* (research R8 — `pick_list` has no
    /// per-item disabling, and forking a list widget is what the Component-reuse gate rejects),
    /// so the refusal happens here, at the point of action (FR-012).
    pub fn can_submit(&self) -> bool {
        if self.status != WorktreeFormStatus::Editing || self.resolution.is_prompting() {
            return false;
        }
        if let Some(candidate) = &self.selected_branch {
            if self.source == BranchSource::Existing && !candidate.is_available() {
                return false;
            }
        }
        self.preview().is_ok()
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
    state.worktree_form.form = Some(WorktreeForm::default());
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
    });
}

/// The ticket field was edited.
pub fn ticket_changed(state: &mut crate::app::State, text: String) {
    while_editing(state, |form| {
        form.ticket = text;
        form.error = None;
    });
}

/// The name field was edited.
pub fn name_changed(state: &mut crate::app::State, text: String) {
    while_editing(state, |form| {
        form.name = text;
        form.error = None;
    });
}

/// The form was submitted — **validated only** (FR-008).
///
/// The shell performs the git create on a valid form and dispatches `WorktreeCreated` /
/// `WorktreeCreateFailed`. A create already in flight makes this a no-op, so there is no
/// double-submit.
pub fn submitted(state: &mut crate::app::State) {
    while_editing(state, |form| {
        if let Err(error) = form.preview() {
            form.error = Some(error);
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
            && (form.source == BranchSource::Issue || form.github_repo().is_none())
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
    });
    if started {
        state.worktree_form.issue_request_seq = next_seq;
    }
}

/// The daemon answered the form's `RemoteList` (feature 034, FR-002).
pub fn remotes_listed(state: &mut crate::app::State, remotes: Result<Vec<GitRemote>, String>) {
    with_form(state, |form| {
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
    });
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
                form.issues = IssueList::Loaded {
                    listing,
                    gh,
                    searched: Vec::new(),
                    search: SearchState::Idle,
                };
                form.issue_list_open = true;
                form.rematch_issues();
            }
            Err(error) => form.issues = IssueList::Failed { error },
        }
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
/// was there (FR-009, FR-010, FR-010a). A number the listing does not hold changes nothing.
pub fn issue_picked(state: &mut crate::app::State, number: u64) {
    while_editing_unprompted(state, |form| {
        let Some(title) = form
            .issues
            .held()
            .into_iter()
            .find(|issue| issue.number() == number)
            .map(|issue| issue.title().to_string())
        else {
            return;
        };
        form.ticket = number.to_string();
        form.name = name_from_title(&title);
        form.error = None;
        form.picked_issue = Some(number);
        form.issue_list_open = false;
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
    with_form(state, |form| form.branch_list_open = false);
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
    /// The form's type selection changed.
    TypeSelected(ConventionalType),
    /// The form's ticket field changed.
    TicketChanged(String),
    /// The form's name field changed.
    NameChanged(String),
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
    /// An issue was picked: its number and title fill ticket and name (FR-009, FR-010).
    IssuePicked { number: u64 },
    /// The debounce after a keystroke ran out (research R9). Starts the search only while `seq` is
    /// still the pending one.
    IssueSearchDue { seq: u64 },
    /// The search beyond the loaded issues answered (FR-005a). Applies only while it awaits `seq`.
    IssueSearched {
        seq: u64,
        result: Result<Vec<Issue>, IssueLoadError>,
    },
}

/// The form's own reducer: one entry point, twenty-two answers (FR-004a).
///
/// The root sees a single arm. Everything the wizard knows about its own steps — which are inert
/// while a create is in flight, which are inert behind a conflict prompt, which reset the branch
/// search — lives on this side of the boundary and never had to be said out there.
pub fn update(state: &mut crate::app::State, msg: Msg) -> Vec<crate::features::Outcome> {
    match msg {
        Msg::Opened => opened(state),
        Msg::TypeSelected(type_) => type_selected(state, type_),
        Msg::TicketChanged(text) => ticket_changed(state, text),
        Msg::NameChanged(text) => name_changed(state, text),
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
        Msg::CreateStageChanged(stage, detail) => create_stage_changed(state, stage, detail),
        Msg::Created(worktree) => return created(state, worktree),
        Msg::CreateFailed(message) => return create_failed(state, message),
        Msg::CreateInterrupted(message) => create_interrupted(state, message),
        Msg::RemotesListed(remotes) => remotes_listed(state, remotes),
        Msg::IssuesLoaded { seq, result, .. } => issues_loaded(state, seq, result),
        Msg::IssueRetry => issue_retry(state),
        Msg::IssueQueryChanged(text) => issue_query_changed(state, text),
        Msg::IssueFocused => issue_focused(state),
        Msg::IssueHighlightMoved(direction) => issue_highlight_moved(state, direction),
        Msg::IssueDismissed => issue_dismissed(state),
        // Resolved by the shell, which holds the index and the results together.
        Msg::IssueRowPicked(_) => {}
        Msg::IssuePicked { number } => issue_picked(state, number),
        Msg::IssueSearchDue { seq } => issue_search_due(state, seq),
        Msg::IssueSearched { seq, result } => issue_searched(state, seq, result),
    }
    Vec::new()
}
