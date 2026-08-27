//! The add-worktree form: its state, its sub-states, and the operations over them.
//!
//! This is the one feature whose intermediate state no other feature reads — it is opened,
//! filled in across several steps (type, ticket, name, branch source, typeahead, conflict
//! resolution), then submitted or cancelled as a unit. That independent lifecycle is what
//! qualifies it as feature 021's sole nested-unit candidate (research.md §5); until Tier 3 it is
//! an ordinary feature module holding data and operations only.
//!
//! Render-free, like every module here: `tests/features_are_render_free.rs` holds that line.

use crate::app::{Message, State};
use crate::overlay::registry::Registered;
use crate::overlay::{DismissalRules, FloatingSurface, SurfaceId};
use micold_core::naming::{
    derive, dir_name_from_branch, ConventionalType, DerivedNames, NamingError, WorktreeNaming,
};
use micold_core::overlay::Layer;
use micold_core::typeahead::{move_highlight, rank, Direction, Match, Query};
use micold_core::worktree::{BranchCandidate, BranchSituation, CreateMode, CreateStage, Worktree};

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
}

impl WorktreeForm {
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
            BranchSource::New => derive(&WorktreeNaming {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddWorktreeDialog;

impl FloatingSurface for AddWorktreeDialog {
    fn id(&self) -> SurfaceId {
        SurfaceId::new("add_worktree")
    }

    fn layer(&self) -> Layer {
        Layer::Dialog
    }

    fn dismissal(&self) -> DismissalRules {
        DismissalRules::for_layer(Layer::Dialog).cancelled_by(Message::WorktreeForm(Msg::Cancelled))
    }
}

impl Registered for AddWorktreeDialog {
    fn open_in(state: &State) -> Option<Self> {
        state.worktree_form.as_ref().map(|_| AddWorktreeDialog)
    }
}

/// The Add Worktree form was opened (feature 005, FR-005).
pub fn opened(state: &mut State) {
    state.clear_for_dialog();
    state.worktree_form = Some(WorktreeForm::default());
    state.worktree_error = None;
}

/// The form was dismissed.
pub fn cancelled(state: &mut State) {
    state.worktree_form = None;
}

/// Apply a change to the open form, whatever it is doing.
fn with_form(state: &mut State, change: impl FnOnce(&mut WorktreeForm)) {
    if let Some(form) = &mut state.worktree_form {
        change(form);
    }
}

/// Apply a change only while the form is accepting edits.
///
/// A create in flight makes the whole form inactive, not just its submit button (feature 010
/// follow-up), so every input arm is guarded — which is why the guard is written once here rather
/// than nine times.
fn while_editing(state: &mut State, change: impl FnOnce(&mut WorktreeForm)) {
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
fn while_editing_unprompted(state: &mut State, change: impl FnOnce(&mut WorktreeForm)) {
    while_editing(state, |form| {
        if !form.resolution.is_prompting() {
            change(form);
        }
    });
}

/// A conventional type was chosen.
pub fn type_selected(state: &mut State, type_: ConventionalType) {
    while_editing(state, |form| {
        form.type_ = Some(type_);
        form.error = None;
    });
}

/// The ticket field was edited.
pub fn ticket_changed(state: &mut State, text: String) {
    while_editing(state, |form| {
        form.ticket = text;
        form.error = None;
    });
}

/// The name field was edited.
pub fn name_changed(state: &mut State, text: String) {
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
pub fn submitted(state: &mut State) {
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
pub fn source_changed(state: &mut State, source: BranchSource) {
    while_editing_unprompted(state, |form| {
        form.source = source;
        form.error = None;
        if source == BranchSource::New {
            form.selected_branch = None;
        }
        form.reset_branch_search();
    });
}

/// Branch candidates arrived (feature 016).
///
/// Re-matched immediately, so the results describe the current query whenever they land.
pub fn branches_listed(state: &mut State, candidates: Vec<BranchCandidate>) {
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
pub fn branch_selected(state: &mut State, candidate: BranchCandidate) {
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
pub fn branch_focused(state: &mut State) {
    while_editing_unprompted(state, |form| form.branch_list_open = true);
}

/// The branch search query was edited (feature 021).
pub fn branch_query_changed(state: &mut State, text: String) {
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
pub fn branch_highlight_moved(state: &mut State, direction: Direction) {
    with_form(state, |form| {
        let rows = form.branch_matches.len();
        if let Some(next) = move_highlight(form.branch_highlight, direction, rows) {
            form.branch_highlight = Some(next);
        }
    });
}

/// The branch picker was dismissed.
pub fn branch_dismissed(state: &mut State) {
    with_form(state, |form| form.branch_list_open = false);
}

/// A branch conflict was detected (feature 016).
///
/// Invariant 4: a prompt and an in-flight create cannot coexist.
pub fn conflict_detected(state: &mut State, situation: BranchSituation) {
    while_editing(state, |form| {
        form.resolution = ResolutionState::Choosing { situation };
    });
}

/// Overwrite was requested from the choice (feature 016, FR-005).
///
/// Only ever from `Choosing`, and only for a situation that *has* a local branch to overwrite —
/// invariant 1.
pub fn overwrite_requested(state: &mut State) {
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
pub fn overwrite_confirmed(state: &mut State) {
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
pub fn resolution_chosen(state: &mut State, mode: CreateMode) {
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
pub fn resolution_cancelled(state: &mut State) {
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
pub fn create_started(state: &mut State, mode: CreateMode) {
    // A new attempt never inherits the previous one's message either. The stale error line used to
    // stand through the whole of the next attempt's pending state — an error and a progress bar on
    // screen together, describing different attempts (BUG-020).
    state.worktree_error = None;
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
pub fn create_stage_changed(state: &mut State, stage: CreateStage, detail: Option<String>) {
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
/// Idempotent by directory name, and sorted so it lands where the list would have put it.
pub fn created(state: &mut State, worktree: Worktree) -> Vec<crate::features::Outcome> {
    state.worktree_form = None;
    state.worktree_error = None;
    vec![crate::features::Outcome::WorktreeCreated(worktree)]
}

/// The worktree list changed, so a create failure shown against the old one is stale (T067a-4).
///
/// `worktree_error` is the add-worktree modal's error line — `crate::ui::worktree_form` is its only
/// render site — so clearing it is the form's own business even when a *worktree* operation is what
/// makes it stale. Reached from the root's `WorktreesReplaced` arm.
pub fn worktree_list_changed(state: &mut State) {
    state.worktree_error = None;
}

/// A create failed (feature 005 FR-017, feature 010).
///
/// The form stays open so the user can adjust, showing the error, and returns to `Editing` so a
/// retry is possible instead of being stuck in `Creating`.
pub fn create_failed(state: &mut State, message: String) {
    state.worktree_error = Some(message);
    with_form(state, |form| {
        form.status = WorktreeFormStatus::Editing;
    });
}

/// A create's connection dropped before the daemon answered it (feature 010, BUG-020).
///
/// Not a failure: the daemon applies its mutation before replying, so the request may well have
/// taken effect — the worktree in the report's reproduction really was created. What is knowable is
/// that nothing on this connection will ever say which, so the form stops claiming the operation is
/// running and says so instead. The inputs stay exactly as they were, because the user may want to
/// retry and because a drop is not a reason to discard what they typed (FR-007).
pub fn create_interrupted(state: &mut State, message: String) {
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
/// and the generic overlay snapshot ever read `state.worktree_form`.
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
    /// the chosen mode. `Overwrite` can only arrive via [`Message::OverwriteConfirmed`].
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
}

/// The form's own reducer: one entry point, twenty-two answers (FR-004a).
///
/// The root sees a single arm. Everything the wizard knows about its own steps — which are inert
/// while a create is in flight, which are inert behind a conflict prompt, which reset the branch
/// search — lives on this side of the boundary and never had to be said out there.
pub fn update(state: &mut State, msg: Msg) -> Vec<crate::features::Outcome> {
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
        Msg::CreateFailed(message) => create_failed(state, message),
        Msg::CreateInterrupted(message) => create_interrupted(state, message),
    }
    Vec::new()
}
