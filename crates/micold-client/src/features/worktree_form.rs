//! The add-worktree form: its state, its sub-states, and the operations over them.
//!
//! This is the one feature whose intermediate state no other feature reads — it is opened,
//! filled in across several steps (type, ticket, name, branch source, typeahead, conflict
//! resolution), then submitted or cancelled as a unit. That independent lifecycle is what
//! qualifies it as feature 021's sole nested-unit candidate (research.md §5); until Tier 3 it is
//! an ordinary feature module holding data and operations only.
//!
//! Render-free, like every module here: `tests/features_are_render_free.rs` holds that line.

use crate::app::{Message, Overlay, State};
use crate::overlay::registry::Registered;
use crate::overlay::{DismissalRules, FloatingSurface, SurfaceId};
use micold_core::naming::{
    derive, dir_name_from_branch, ConventionalType, DerivedNames, NamingError, WorktreeNaming,
};
use micold_core::overlay::Layer;
use micold_core::typeahead::{rank, Match, Query};
use micold_core::worktree::{BranchCandidate, BranchSituation, CreateMode, CreateStage};

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
/// Lives INSIDE the form rather than as its own [`Overlay`] variant: `Overlay` holds one modal at
/// a time, so routing the prompt through it would tear down the form — and with it the inputs
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
        DismissalRules::for_layer(Layer::Dialog).cancelled_by(Message::AddWorktreeCancelled)
    }
}

impl Registered for AddWorktreeDialog {
    fn open_in(state: &State) -> Option<Self> {
        (state.overlay == Overlay::AddWorktree).then_some(AddWorktreeDialog)
    }
}
