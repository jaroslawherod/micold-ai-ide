//! The sidebar's rows and tag filters (feature 021, T016).
//!
//! A sidebar row is a *projection*: a worktree or the project root, joined with its sessions and
//! with the tags rendered beside its name. Nothing here holds the sidebar's own mutable state —
//! that lives in `State` until Tier 3 — so this module is the row vocabulary, the pure functions
//! over it, and the projections that build the rows, kept together per FR-001.
//!
//! The projections at the bottom arrived in T019, which had filed them under "worktree". They are
//! named for worktrees but typed for the sidebar — `worktree_tree` returns `WorktreeNode`,
//! `available_tag_filters` returns `TagFilter`, and `worktree_tree`'s doc comment opens "Build the
//! sidebar tree". Grouping by name rather than by feature is what FR-001 argues against, and
//! SC-010 is answered by where a feature's code sits, not by what it is called.
//!
//! They are `impl State` blocks because `State` is still monolithic in Tier 1. Methods resolve on
//! the type rather than the module, so relocating them changed no call site.
//!
//! # The vocabulary this feature declares
//!
//! The transitions in [`Msg`] that are this module's own: expansion (`WorktreeExpansionToggled`,
//! `DefaultExpansionToggled`, `GroupToggled`), the tag filters (`FilterToggled`, `FiltersCleared`),
//! the chip bar's more (⋯) menu (`MoreMenuToggled`, `MoreMenuAnchored`, `MoreMenuChosen`; feature
//! 1125, which removed the filter panel and its `FilterMenuToggled`), the scroll position
//! (`Scrolled`, `ViewportResized`), and the panel itself (`Toggled`, `DragMoved`). [`update`]
//! routes them and is pure (data-model.md §1.1 shape A). One message has a shell follow-up:
//! after `MoreMenuChosen` closes the menu, the binary sends the chosen item's header-button message
//! (`ui::sidebar::more_action_message`), so a refresh keeps the shell's request path (D19).
//!
//! # The state this feature remembers (feature 028, contract S1)
//!
//! Eleven fields in [`State`], reached as `state.sidebar`, in three groups:
//!
//! - **what is expanded**: `expanded` (the worktree rows the user has opened), `default_expanded`,
//!   and `pending_reveal_scroll`, the flag that arms a scroll-into-view once layout knows the height;
//! - **what is shown**: `filters`, `more_menu_open` and `more_anchor` (feature 1125: the chip
//!   bar's more menu and the corner it hangs from), and `group_choices` (feature 1125: the user's
//!   open or closed choice per project and state group);
//! - **the panel's own geometry**: `hidden`, `width`, `scroll_offset`, `viewport_height`.
//!
//! Three kept the names they had flat on the root. Six shed the `sidebar` the qualifier carries:
//! `sidebar_filters`, `sidebar_filter_open`, `sidebar_hidden`, `sidebar_width`,
//! `sidebar_scroll_offset` and `sidebar_viewport_height` (T035).
//!
//! # State groups (feature 1125)
//!
//! [`crate::app::State::sidebar_groups`] puts the entries in [`SidebarGroup`]s by their card
//! state, through `micold_core::sidebar_groups`; [`crate::app::State::sidebar_list`] flattens them
//! into the drawn list of [`ListRow`]s, a [`GroupHeaderRow`] per group then its entries when open.
//! The row metrics ([`row_heights`], [`row_index`], [`location_row`], [`current_session_row`])
//! walk that list. `Msg::GroupToggled` flips a group for the active project.
//!
//! **The tree is not here.** Which projects, worktrees and sessions the sidebar lists is derived on
//! every view from `state.workspace` and `state.worktree`, which is what lets a wholesale
//! re-discovery of the worktrees leave this feature's expansion untouched — see
//! [`crate::app::State::set_worktrees`]. `expanded` is a set of names, not of rows, for the same
//! reason.
//!
//! Note that a `SidebarEntry`'s node carries an `expanded` of its own. That is one row's flag in a
//! rendered tree and is not this field; the two are unrelated despite the shared name.

use crate::features::worktree::worktree_tags;
use crate::floating_surface;
use crate::overlay::registry::Registered;
use crate::overlay::SurfaceId;
use micold_core::location_state::{location_state, LocationState};
use micold_core::naming::{ConventionalType, Tag};
use micold_core::pull_request::{CheckStatus, PrState, PullRequestStatus, ReviewState};
use micold_core::runs::{GroupId, RunGroup, RunStatus};
use micold_core::session::{AiCli, Session, SessionId, SessionLocation};
use micold_core::sidebar_groups::{
    group_by_state, group_open, run_group_state, RunCard, StateGroup,
};
use micold_core::tokens::{density, spacing};
use micold_core::worktree::Worktree;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// What this feature remembers (feature 028, contract S1).
///
/// Six fields shed the `sidebar` the qualifier now carries: `sidebar_filter_open`,
/// `sidebar_filters`, `sidebar_hidden`, `sidebar_scroll_offset`, `sidebar_viewport_height` and
/// `sidebar_width` became `filter_open`, `filters`, `hidden`, `scroll_offset`, `viewport_height`
/// and `width`; feature 1125 then replaced `filter_open` with `more_menu_open` and added
/// `more_anchor` and `group_choices`. `default_expanded`, `expanded` and `pending_reveal_scroll`
/// never carried it and keep their names. The reducers below
/// spell the root's type `crate::app::State` now that `State` here means this struct.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// Whether the "Default" (project-root) sidebar row is expanded to reveal its sessions
    /// (feature 010, mirrors `expanded` for worktree rows — a dedicated field rather than a
    /// sentinel key in `expanded`, since there is always exactly one Default row).
    pub default_expanded: bool,
    /// Which worktree rows are expanded to reveal their sessions (FR-003). By `dir_name`.
    pub expanded: BTreeSet<String>,
    /// The origins whose children the user folded away (feature 551, FR-011). Stored as folded,
    /// so an origin never folded is expanded (FR-017). Mirrored in `Settings.folded_origins`,
    /// which the shell writes after every toggle and reads at boot. Never pruned (FR-017).
    pub folded: BTreeSet<SessionId>,
    /// Whether a reveal is waiting to scroll its row into view (feature 024, FR-008).
    ///
    /// A flag, not a target. The offset cannot be computed when the reveal is armed: the incoming
    /// project's worktree list may not have arrived yet, and the viewport height is not known
    /// until layout. So the reducer arms this, and the binary computes and applies the scroll on
    /// the first frame where a row for the current session actually exists (research R7,
    /// invariant I4).
    pub pending_reveal_scroll: bool,
    /// The location the Overview sent the user to (feature 813, R12): scrolled into view and drawn
    /// with a session row's selected look until the next selection.
    pub revealed: Option<SessionLocation>,
    /// Whether the chip bar's more (⋯) menu is shown (feature 1125, FR-015). It took the place of
    /// the tag-filter panel's flag, whose chips are always on the chip bar now. Mutually exclusive
    /// with `help.help_menu_open`/`project.switcher_open`. Transient — not persisted; closing it
    /// never alters `filters`.
    pub more_menu_open: bool,
    /// Where the more menu's top-left corner goes, in window pixels: the bottom-left corner of the
    /// trigger whose press opened it, reported ahead of the toggle (the switcher's BUG-1023
    /// pattern). `None` until a pointer press has reported one; cleared when the menu closes.
    pub more_anchor: Option<(u16, u16)>,
    /// Active sidebar tag filters (feature 008, FR-024). Empty ⇒ all worktrees shown. Multiple
    /// filters combine with OR (FR-025). Transient — not persisted.
    pub filters: BTreeSet<TagFilter>,
    /// Whether the sidebar is collapsed/hidden. Default (`false`) is visible.
    pub hidden: bool,
    /// How far the worktree sidebar is scrolled, in logical pixels.
    ///
    /// The app bar's elevation derives from this and nothing else (FR-025a) — see
    /// [`crate::app::State::app_bar_elevated`] for why a second source would be a defect rather
    /// than a feature.
    pub scroll_offset: u32,
    /// The sidebar scroll viewport's laid-out height in whole logical pixels (feature 024).
    ///
    /// Reported by the `Scrollable`'s viewport sensor. `0` until the first layout, which reads as
    /// "cannot decide visibility yet" and never as "zero tall" — nothing is scrolled on a guess
    /// (contract §6.3).
    ///
    /// `u32` rather than `f32` for two reasons that happen to agree: `State` derives `Eq`, and the
    /// offset this is compared against ([`Self::scroll_offset`]) is already whole pixels.
    /// Keeping both in the same unit is what stops the scroll arithmetic from having a rounding
    /// seam in the middle of it.
    pub viewport_height: u32,
    /// The sidebar width in pixels. `0` means "use the default width" (see
    /// [`crate::app::State::sidebar_width_px`]).
    pub width: u16,
    /// The location a location chord landed on that has no session to show (feature 803, a
    /// navigation stop): the next location press steps on from it (contracts/ui.md U2, U5).
    pub nav_stop: Option<SessionLocation>,
    /// The row a navigation chord gave the sidebar's keyboard (feature 803, contracts/ui.md U3):
    /// the reveal scroll reaches it rather than the current session's row.
    pub keyboard_row: Option<RowRef>,
    /// The user's open (`true`) or closed (`false`) choice per project and state group (feature
    /// 1125, FR-005, research R4); a group never chosen is absent and follows
    /// [`group_open`]'s default. Keyed by group, not by row, so it is never pruned; untouched by a
    /// project switch; in memory only, for the run.
    pub group_choices: BTreeMap<PathBuf, BTreeMap<StateGroup, bool>>,
}

/// A sidebar row named by what it shows, not by its position, so it survives the list arriving
/// late (feature 803, data-model.md).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RowRef {
    /// A location's own row (Default or a worktree).
    Location(SessionLocation),
    /// The row holding a session: its sub-row, or its location's row when that draws none.
    Session(SessionId),
}

impl RowRef {
    /// Whether this names the row of `location`: its own, or its session's when the location draws
    /// no sub-rows (`shown` false), matching [`row_index`].
    pub fn is_location_row(
        &self,
        location: &SessionLocation,
        shown: bool,
        sessions: &[&Session],
    ) -> bool {
        match self {
            RowRef::Location(wanted) => wanted == location,
            RowRef::Session(id) => !shown && sessions.iter().any(|s| s.id == *id),
        }
    }

    /// Whether this names the sub-row of `session`.
    pub fn is_session_row(&self, session: SessionId) -> bool {
        *self == RowRef::Session(session)
    }
}

/// One row in the sidebar's location list (feature 010): either a worktree or the single
/// "Default" project-root entry. A closed enum (Principle V) so a row can never be ambiguously
/// styled as a worktree when it isn't one (FR-006).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidebarEntry {
    /// A discovered worktree row.
    Worktree(WorktreeNode),
    /// The single project-root row (constitution v1.3.0, Principle III exception).
    Default(DefaultNode),
    /// A run group's row with its runs one level in (feature 483, G1–G3).
    Group(GroupNode),
}

/// One state group of the sidebar (feature 1125, data-model *SidebarGroup*): its entries in
/// [`crate::app::State::sidebar_entries`] order, and whether it is open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarGroup {
    /// Which group.
    pub group: StateGroup,
    /// Whether its entries are drawn ([`group_open`], research R4).
    pub open: bool,
    /// The group's entries; a run group is one (FR-002).
    pub entries: Vec<SidebarEntry>,
}

/// A group header as the list draws it (feature 1125, FR-002).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupHeaderRow {
    /// Which group.
    pub group: StateGroup,
    /// How many entries it holds, drawn or not.
    pub count: usize,
    /// Whether its entries follow it.
    pub open: bool,
}

/// One row of the sidebar's drawn list, top to bottom (feature 1125, research R3): a group's header,
/// or an entry of an open group. A separate enum rather than a [`SidebarEntry`] variant, so a walk
/// over locations never meets a header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListRow {
    /// A state group's header.
    Header(GroupHeaderRow),
    /// An entry of an open group, drawn as today.
    Entry(SidebarEntry),
}

impl SidebarEntry {
    /// Whether this entry draws `location`'s row: its own, or one of a run group's runs'.
    pub fn holds(&self, location: &SessionLocation) -> bool {
        match self {
            SidebarEntry::Default(_) => *location == SessionLocation::Default,
            SidebarEntry::Worktree(node) => node.location() == *location,
            SidebarEntry::Group(group) => group.runs.iter().any(
                |run| matches!(&run.row, RunRow::Worktree(node) if node.location() == *location),
            ),
        }
    }
}

/// A run group's row (feature 483, parallel-surfaces G1, G2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupNode {
    /// The group.
    pub id: GroupId,
    /// The group's name, shown on its row.
    pub name: String,
    /// Whether its runs are shown (G2).
    pub expanded: bool,
    /// How many runs it has, shown whether or not it is expanded.
    pub run_count: usize,
    /// How many of them failed, shown whether or not it is expanded.
    pub failed_count: usize,
    /// The runs, in run order.
    pub runs: Vec<RunNode>,
}

/// One run under its group row (G1, G1a).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunNode {
    /// The run's number.
    pub number: u8,
    /// The run's provider.
    pub provider: AiCli,
    /// Where the run got to.
    pub status: RunStatus,
    /// Its worktree row, or a leaf when it has none.
    pub row: RunRow,
}

/// What a run row is made of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunRow {
    /// The run's worktree, with the usual tags and session children.
    Worktree(WorktreeNode),
    /// The run has no worktree row: it failed while creating it, or is still creating it, or the
    /// sidebar's filters hide it. A leaf with no worktree menu (G1a).
    NoWorktree,
}

impl RunNode {
    /// Whether the run failed (the `failed` tag).
    pub fn failed(&self) -> bool {
        self.status.is_failed()
    }

    /// Why the run failed or its prompt was not delivered: the row's tooltip.
    pub fn reason(&self) -> Option<&str> {
        match &self.status {
            RunStatus::Failed { reason, .. } | RunStatus::PromptNotDelivered { reason } => {
                Some(reason)
            }
            _ => None,
        }
    }
}

/// The chip bar's more (⋯) menu's actions (feature 1125, D4, FR-015): the three header buttons
/// that did not stay on the chip bar beside add and hide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MoreAction {
    /// Re-read the worktree list (feature 029).
    Refresh,
    /// Open the "Attach existing…" list (feature 582).
    AttachExisting,
    /// Open the Run in parallel dialog (feature 483).
    RunInParallel,
}

/// The more menu's items, top to bottom (contract S3).
pub const MORE_MENU: [MoreAction; 3] = [
    MoreAction::Refresh,
    MoreAction::AttachExisting,
    MoreAction::RunInParallel,
];

/// One item of the more menu as the view draws it: the header button's tooltip and enabled rule,
/// unchanged (FR-015).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoreMenuItem {
    pub action: MoreAction,
    pub label: &'static str,
    pub tooltip: &'static str,
    pub enabled: bool,
}

/// The more menu's items for a sidebar with a project open (`project_active`) and a refresh
/// running (`refreshing`): refresh is offered exactly when `can_refresh_worktrees` is true, the
/// other two whenever a project is open, as the header's buttons were (feature 029 FR-005,
/// features 582 and 483).
pub fn more_menu_items(project_active: bool, refreshing: bool) -> Vec<MoreMenuItem> {
    MORE_MENU
        .iter()
        .map(|&action| {
            let (label, tooltip, enabled) = match action {
                // The label is the tooltip, so a running refresh says so on the item itself: a
                // menu item's tooltip is drawn only over a trailing icon, which this has none of.
                MoreAction::Refresh => {
                    let text = if refreshing {
                        "Refreshing worktrees…"
                    } else {
                        "Refresh the worktree list"
                    };
                    (text, text, project_active && !refreshing)
                }
                MoreAction::AttachExisting => {
                    ("Attach existing…", "Attach existing…", project_active)
                }
                MoreAction::RunInParallel => {
                    ("Run in parallel…", "Run in parallel…", project_active)
                }
            };
            MoreMenuItem {
                action,
                label,
                tooltip,
                enabled,
            }
        })
        .collect()
}

/// One chip of the chip bar (feature 1125, FR-014): **All**, or one tag filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChipBarChip {
    All,
    Filter(TagFilter),
}

/// The chip bar's chips, left to right, each with whether it looks selected: **All** first,
/// selected exactly when no filter is active, then one chip per filter on offer in `available`'s
/// order (US4.1, US4.2).
pub fn chip_bar_chips(
    available: &[TagFilter],
    active: &BTreeSet<TagFilter>,
) -> Vec<(ChipBarChip, bool)> {
    std::iter::once((ChipBarChip::All, active.is_empty()))
        .chain(
            available
                .iter()
                .map(|&filter| (ChipBarChip::Filter(filter), active.contains(&filter))),
        )
        .collect()
}

/// **Run in parallel** was chosen: the root opens the dialog for the active project.
pub fn run_in_parallel() -> Vec<crate::features::Outcome> {
    vec![crate::features::Outcome::RunInParallelRequested]
}

/// Put the runs of `groups` under their group rows (feature 483, G1–G3).
///
/// `entries` is the Default row and the worktree rows. A worktree that is a run's leaves the
/// worktree level and becomes that run's child; the group row sits where its first such worktree
/// sat, or after every row when none is listed. A run with no row of its own is still a child, as
/// a leaf (G1a). Unrelated rows keep their order.
pub fn arrange_groups(
    entries: Vec<SidebarEntry>,
    groups: &[RunGroup],
    expanded: impl Fn(GroupId) -> bool,
) -> Vec<SidebarEntry> {
    let mut entries: Vec<Option<SidebarEntry>> = entries.into_iter().map(Some).collect();
    let mut placed: Vec<(usize, SidebarEntry)> = Vec::new();
    for group in groups {
        let mut first = None;
        let runs = group
            .runs
            .iter()
            .map(|run| {
                let at = entries.iter().position(|e| {
                    matches!(e, Some(SidebarEntry::Worktree(n)) if n.worktree.dir_name == run.names.dir_name)
                });
                let row = match at.and_then(|i| entries[i].take().map(|e| (i, e))) {
                    Some((i, SidebarEntry::Worktree(node))) => {
                        first = Some(first.map_or(i, |f: usize| f.min(i)));
                        RunRow::Worktree(node)
                    }
                    _ => RunRow::NoWorktree,
                };
                RunNode {
                    number: run.number,
                    provider: run.provider,
                    status: run.status.clone(),
                    row,
                }
            })
            .collect();
        placed.push((
            first.unwrap_or(usize::MAX),
            SidebarEntry::Group(GroupNode {
                id: group.id,
                name: group.name.clone(),
                expanded: expanded(group.id),
                run_count: group.runs.len(),
                failed_count: group.failed_count(),
                runs,
            }),
        ));
    }
    let mut out = Vec::new();
    for (index, entry) in entries.into_iter().enumerate() {
        for (_, group) in placed.iter().filter(|(at, _)| *at == index) {
            out.push(group.clone());
        }
        out.extend(entry);
    }
    out.extend(
        placed
            .into_iter()
            .filter(|(at, _)| *at == usize::MAX)
            .map(|(_, group)| group),
    );
    out
}

/// The "Default" (project-root) sidebar row, joined with its sessions (feature 010, FR-001,
/// FR-006). Unlike [`WorktreeNode`] it carries no [`Tag`]s and is never subject to the sidebar's
/// tag-filter panel (feature 009) — type/issue/status tags are derived from worktree branch
/// naming and do not apply to the project root (research.md R4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultNode {
    /// Always the literal "Default" (FR-006) — never derived or user-renamable.
    pub display_name: &'static str,
    /// Whether its session sub-items are shown.
    pub expanded: bool,
    /// Sessions with `SessionLocation::Default` for the active project.
    pub sessions: Vec<Session>,
    /// The origins folded in the sidebar (feature 551, FR-011): their descendants are not listed.
    pub folded: std::sync::Arc<BTreeSet<SessionId>>,
}

/// One worktree row in the sidebar tree, joined with its (expanded) sessions (FR-002/003).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeNode {
    /// The worktree itself.
    pub worktree: Worktree,
    /// The human-friendly display name shown on the first line (FR-001, FR-017): the custom
    /// rename override if set, else derived from `dir_name`.
    pub display_name: String,
    /// Color-coded tags shown beneath the name (FR-001..003, FR-011): the conventional type,
    /// an optional Jira issue, and a status tag for non-`Valid` worktrees.
    pub tags: Vec<Tag>,
    /// Whether its session sub-items are shown.
    pub expanded: bool,
    /// The sessions hosted by this worktree (empty unless expanded is irrelevant to data).
    pub sessions: Vec<Session>,
    /// Whether this row is listed *only* because it holds the current session — the active tag
    /// filters, or the hidden-agent-worktree setting, would otherwise have excluded it (feature
    /// 024, FR-012a).
    ///
    /// `false` for a row the filters admit on their own, and that distinction is the requirement
    /// rather than an implementation detail: the row carries a chip saying why it is there, and a
    /// row that would have been listed anyway must not claim an exemption it did not need.
    pub shown_for_current_session: bool,
    /// The name of the daemon this worktree runs on, or [`NO_DAEMON_LABEL`] (feature 491, FR-005).
    pub daemon: String,
    /// The origins folded in the sidebar (feature 551, FR-011): their descendants are not listed.
    pub folded: std::sync::Arc<BTreeSet<SessionId>>,
}

impl WorktreeNode {
    /// The location this row stands for.
    pub fn location(&self) -> SessionLocation {
        SessionLocation::Worktree(self.worktree.dir_name.clone())
    }

    /// Whether the row has an expander: only a worktree with several sessions does (FR-017).
    pub fn has_expander(&self) -> bool {
        self.sessions.len() > 1
    }

    /// Whether the row's session sub-rows are drawn: an expander that is open.
    pub fn sessions_shown(&self) -> bool {
        self.expanded && self.has_expander()
    }

    /// The session a press of this row selects (FR-018): the main one, never a helper, whatever
    /// was last viewed. `None` when the worktree has no session.
    pub fn main_session(&self) -> Option<SessionId> {
        micold_core::occupancy::main_session(&self.sessions, &self.location())
    }

    /// The sessions in the order the expander lists them: the main one first, then the helpers,
    /// each followed by the sessions it started (feature 551).
    pub fn listed_sessions(&self) -> Vec<&Session> {
        session_rows(&self.sessions, self.main_session(), &self.folded)
            .into_iter()
            .map(|row| row.session)
            .collect()
    }

    /// [`listed_sessions`](Self::listed_sessions) with each session's depth below the location and
    /// its place in the tree (feature 551, FR-003, FR-004, FR-016).
    pub fn session_rows(&self) -> Vec<SessionRow<'_>> {
        session_rows(&self.sessions, self.main_session(), &self.folded)
    }

    /// The worktree's state for its row's state icon (feature 905, FR-001, FR-004): taken from
    /// every un-archived session, expanded or not.
    pub fn state(&self) -> LocationState {
        location_state(&self.sessions)
    }

    /// [`listed_sessions`](Self::listed_sessions) with the folds ignored: the order a chord steps
    /// through, so a session under a folded origin stays reachable and a current session that is
    /// hidden still has a place in it (feature 551).
    pub fn unfolded_sessions(&self) -> Vec<&Session> {
        unfolded_sessions(&self.sessions, self.main_session())
    }
}

impl DefaultNode {
    /// Whether the row has an expander: only a root with several sessions does (FR-017).
    pub fn has_expander(&self) -> bool {
        self.sessions.len() > 1
    }

    /// Whether the row's session sub-rows are drawn: an expander that is open.
    pub fn sessions_shown(&self) -> bool {
        self.expanded && self.has_expander()
    }

    /// The session a press of this row selects (FR-018).
    pub fn main_session(&self) -> Option<SessionId> {
        micold_core::occupancy::main_session(&self.sessions, &SessionLocation::Default)
    }

    /// The sessions in the order the expander lists them: the main one first, each followed by the
    /// sessions it started (feature 551).
    pub fn listed_sessions(&self) -> Vec<&Session> {
        session_rows(&self.sessions, self.main_session(), &self.folded)
            .into_iter()
            .map(|row| row.session)
            .collect()
    }

    /// [`listed_sessions`](Self::listed_sessions) with depth and place (feature 551).
    pub fn session_rows(&self) -> Vec<SessionRow<'_>> {
        session_rows(&self.sessions, self.main_session(), &self.folded)
    }

    /// The project root's state for the Default row's state icon, by the worktree rows' rules
    /// (feature 905, FR-002).
    pub fn state(&self) -> LocationState {
        location_state(&self.sessions)
    }

    /// [`listed_sessions`](Self::listed_sessions) with the folds ignored: the order a chord steps
    /// through, so a session under a folded origin stays reachable and a current session that is
    /// hidden still has a place in it (feature 551).
    pub fn unfolded_sessions(&self) -> Vec<&Session> {
        unfolded_sessions(&self.sessions, self.main_session())
    }
}

fn unfolded_sessions(sessions: &[Session], main: Option<SessionId>) -> Vec<&Session> {
    session_rows(sessions, main, &BTreeSet::new())
        .into_iter()
        .map(|row| row.session)
        .collect()
}

/// One session row of a location: the session, its depth below the location's own sessions
/// (0 = listed directly) and its place in the origin tree (feature 551).
#[derive(Debug, Clone)]
pub struct SessionRow<'a> {
    pub session: &'a Session,
    pub depth: usize,
    pub place: micold_core::origin::TreePlace,
    /// Set on a session that has children (feature 551, FR-011).
    pub fold: Option<Fold<'a>>,
}

/// What a session with children shows of them: whether they are folded away, how many are its
/// direct children and, while folded, every descendant that is hidden (feature 551, FR-011).
#[derive(Debug, Clone)]
pub struct Fold<'a> {
    pub folded: bool,
    /// Direct children, folded or not.
    pub children: usize,
    hidden: Vec<&'a Session>,
}

impl Fold<'_> {
    /// Whether a hidden descendant needs the user (FR-011): unread by the rule the location rows
    /// count with. Always `false` while expanded: the child shows its own mark.
    pub fn needs_attention(&self, in_view: Option<SessionId>) -> bool {
        self.hidden
            .iter()
            .any(|s| micold_core::attention::counts_as_unread(s, in_view))
    }
}

/// `sessions` as the origin tree, the one `main` first among the roots, the rest in their order,
/// children after the session that started them (feature 551).
fn session_rows<'a>(
    sessions: &'a [Session],
    main: Option<SessionId>,
    folded: &BTreeSet<SessionId>,
) -> Vec<SessionRow<'a>> {
    let (first, rest): (Vec<&Session>, Vec<&Session>) =
        sessions.iter().partition(|s| Some(s.id) == main);
    let ordered: Vec<&Session> = first.into_iter().chain(rest).collect();
    let input: Vec<_> = ordered.iter().map(|s| (s.id, s.origin)).collect();
    let rows = micold_core::origin::SessionTree::build(&input).rows;
    let by_id: std::collections::HashMap<SessionId, &Session> =
        ordered.iter().map(|s| (s.id, *s)).collect();
    let session_of = |id| by_id.get(&id).copied();
    let mut out = Vec::new();
    let mut at = 0;
    while at < rows.len() {
        let row = &rows[at];
        // Depth first: a row's descendants are the rows after it that sit deeper.
        let end = rows[at + 1..]
            .iter()
            .position(|r| r.depth <= row.depth)
            .map_or(rows.len(), |p| at + 1 + p);
        let below = &rows[at + 1..end];
        let children = below.iter().filter(|r| r.depth == row.depth + 1).count();
        let is_folded = children > 0 && folded.contains(&row.id);
        let fold = (children > 0).then(|| Fold {
            folded: is_folded,
            children,
            hidden: if is_folded {
                below.iter().filter_map(|r| session_of(r.id)).collect()
            } else {
                Vec::new()
            },
        });
        at = if is_folded { end } else { at + 1 };
        if let Some(session) = session_of(row.id) {
            out.push(SessionRow {
                session,
                depth: row.depth,
                place: row.place,
                fold,
            });
        }
    }
    out
}

impl SidebarEntry {
    /// The number of this location's sessions that count as unread (feature 575, FR-002).
    ///
    /// Counted over every session of the location, whether the row is expanded or not (FR-003):
    /// the indicator is what a collapsed row has instead of its session rows.
    pub fn unread_count(&self, in_view: Option<SessionId>) -> usize {
        let sessions = match self {
            SidebarEntry::Worktree(node) => &node.sessions,
            SidebarEntry::Default(node) => &node.sessions,
            SidebarEntry::Group(group) => {
                return group
                    .runs
                    .iter()
                    .filter_map(|run| match &run.row {
                        RunRow::Worktree(node) => Some(&node.sessions),
                        RunRow::NoWorktree => None,
                    })
                    .flatten()
                    .filter(|session| micold_core::attention::counts_as_unread(session, in_view))
                    .count();
            }
        };
        sessions
            .iter()
            .filter(|session| micold_core::attention::counts_as_unread(session, in_view))
            .count()
    }
}

/// A tag filter the sidebar can apply (feature 008, FR-024). Typed so an impossible filter is
/// unrepresentable (Principle V); ordered so it lives in a `BTreeSet`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TagFilter {
    /// Match worktrees of a specific conventional type.
    Type(ConventionalType),
    /// Match worktrees that embed a Jira/issue key.
    HasIssue,
    /// Match worktrees whose name does not follow the convention (no type tag).
    Untyped,
}

/// The environment variable that pre-applies sidebar tag filters at launch.
///
/// A **test hook**, for the manual visual pass (quickstart §B5). The filter itself is a popover:
/// it opens on a click and dismisses on any interaction elsewhere, including the pointer moving
/// onto a row — which a screenshot harness cannot avoid, because it drives the pointer to reach
/// anything at all. So §B5, the step that checks the current session's location survives a filter
/// that excludes it, could not be run at all without this.
///
/// It sets the *same* state the popover sets, through the same message, so what the pass then
/// observes is the real filter and not a second implementation of one.
pub const FILTER_ENV_VAR: &str = "MICOLD_SIDEBAR_FILTER";

impl TagFilter {
    /// Parse one filter token: a conventional type (`feat`, `fix`, …), `issue`, or `untyped`.
    pub fn from_token(token: &str) -> Option<Self> {
        match token {
            "issue" => Some(Self::HasIssue),
            "untyped" => Some(Self::Untyped),
            other => ConventionalType::from_token(other).map(Self::Type),
        }
    }
}

/// The filters named by [`FILTER_ENV_VAR`]'s value: a comma-separated list of tokens.
///
/// - `None`, empty, or all-whitespace → no filters, and the application starts normally.
/// - `"fix"`, `"fix,docs"`, `"issue"`, `"untyped"` → those filters, in the order given.
/// - anything else → `Err` naming the variable, the bad token, and the grammar.
///
/// A malformed value is an error rather than a silent "no filter", for the reason the frame probe
/// gives for the same choice: someone who types `MICOLD_SIDEBAR_FILTER=feature` and gets an
/// unfiltered list would conclude the hook is broken, and a typo mid-pass would record an
/// unfiltered panel as evidence that filtering works.
pub fn filters_from_env_value(raw: Option<&str>) -> Result<Vec<TagFilter>, String> {
    let Some(raw) = raw else {
        return Ok(Vec::new());
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let mut filters = Vec::new();
    for token in trimmed.split(',') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        let filter = TagFilter::from_token(token).ok_or_else(|| {
            format!(
                "{FILTER_ENV_VAR}: {token:?} is not a filter. Expected a comma-separated list of \
                 conventional types ({}), `issue`, or `untyped`.",
                ConventionalType::ALL
                    .iter()
                    .map(|t| t.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;
        filters.push(filter);
    }
    Ok(filters)
}

/// Whether a worktree with `tags` passes the active `filters` (feature 008, FR-025). An empty
/// filter set shows everything; otherwise a worktree matches if it satisfies ANY active filter
/// (logical OR).
pub fn matches_filters(tags: &[Tag], filters: &BTreeSet<TagFilter>) -> bool {
    if filters.is_empty() {
        return true;
    }
    filters.iter().any(|f| match f {
        TagFilter::Type(t) => tags.iter().any(|tag| matches!(tag, Tag::Type(x) if x == t)),
        TagFilter::HasIssue => tags.iter().any(|tag| matches!(tag, Tag::Issue(_))),
        TagFilter::Untyped => !tags.iter().any(|tag| matches!(tag, Tag::Type(_))),
    })
}

/// Whether a location's row is shown open (feature 024, contract §1.1).
///
/// Three booleans, because that is all the rule is: the user's own expansion, whether this is the
/// location holding the current session, and whether the user has closed the app's reveal for that
/// session. A free function like [`matches_filters`] above, and for the same reason — the rule is
/// worth being able to state, and check, without a `State` to state it against.
///
/// The one thing worth reading twice is what is *not* here: nothing about worktree lists arriving
/// late or being replaced. That is FR-001b, and it is answered by this being a function rather than
/// a stored flag — a replaced list changes the inputs, never a remembered answer.
pub fn effective_open(
    user_open: bool,
    holds_current_session: bool,
    reveal_suppressed: bool,
) -> bool {
    user_open || (holds_current_session && !reveal_suppressed)
}

/// The density the sidebar's list is drawn at (§7.2, FR-026a).
///
/// Named here rather than at the call site so the row metrics below and the `TreeView` that renders
/// them cannot disagree about it. They are the same number by construction, which is what stops a
/// computed scroll target from drifting from the rendered rows — the one place in this feature
/// where a wrong answer is silent.
pub const SIDEBAR_DENSITY: i8 = density::DENSE;

/// The vertical gap `TreeView` leaves between rows (`tree_view.rs`'s `column![].spacing(...)`).
///
/// Not a parameter: the gap is a property of the component that draws the rows, not of the caller
/// that measures them, and passing it in would invite two answers.
const ROW_GAP: f32 = spacing::XS;

/// The drawn list of `groups` (feature 1125, contract S4): each group's header, then its entries
/// when it is open.
pub fn list_rows(groups: Vec<SidebarGroup>) -> Vec<ListRow> {
    let mut rows = Vec::new();
    for group in groups {
        rows.push(ListRow::Header(GroupHeaderRow {
            group: group.group,
            count: group.entries.len(),
            open: group.open,
        }));
        if group.open {
            rows.extend(group.entries.into_iter().map(ListRow::Entry));
        }
    }
    rows
}

/// The rendered height of every sidebar row, top to bottom (feature 024, research R6).
///
/// A location row is Material's two-line list item, *Default* included (feature 1125, R6); a
/// session row is always one line. Both figures come from the same `density::height`
/// the renderer uses, at [`SIDEBAR_DENSITY`]. A group header (feature 1125) is one line.
///
/// Only *open* locations contribute session rows, because only those are drawn, and only the
/// entries of open groups are in `rows` ([`crate::app::State::sidebar_list`]).
pub fn row_heights(rows: &[ListRow]) -> Vec<f32> {
    let one_line = density::height(density::LIST_ROW_BASE, SIDEBAR_DENSITY);
    let two_line = density::height(density::LIST_ROW_TWO_LINE_BASE, SIDEBAR_DENSITY);
    let mut heights = Vec::new();
    let location = |heights: &mut Vec<f32>, shown: bool, sessions: usize| {
        heights.push(two_line);
        if shown {
            heights.extend(std::iter::repeat_n(one_line, sessions));
        }
    };
    for row in rows {
        let ListRow::Entry(entry) = row else {
            heights.push(one_line);
            continue;
        };
        match entry {
            SidebarEntry::Default(node) => {
                location(&mut heights, node.sessions_shown(), node.sessions.len());
            }
            SidebarEntry::Worktree(node) => {
                location(&mut heights, node.sessions_shown(), node.sessions.len())
            }
            SidebarEntry::Group(group) => {
                heights.push(one_line);
                if group.expanded {
                    for run in &group.runs {
                        match &run.row {
                            RunRow::Worktree(node) => {
                                location(&mut heights, node.sessions_shown(), node.sessions.len())
                            }
                            RunRow::NoWorktree => heights.push(one_line),
                        }
                    }
                }
            }
        }
    }
    heights
}

/// What decides which sidebar row wears the selected look (feature 1125, FR-009): read from the
/// Overview, the session area and the sidebar, so the view asks one question per row.
#[derive(Debug, Clone, Copy, Default)]
pub struct RowSelection<'a> {
    /// Whether the Overview is open: then its own row is the selected one and no other is.
    pub overview_open: bool,
    /// The session the main area shows.
    pub viewed: Option<SessionId>,
    /// The location the Overview sent the user to ([`State::revealed`]).
    pub revealed: Option<&'a SessionLocation>,
}

/// A row [`row_selected`] is asked about.
#[derive(Debug, Clone, Copy)]
pub enum SelectableRow<'a> {
    /// A location row: where it is, its sessions, and whether it draws them as rows beneath it.
    Location {
        /// The row's location.
        location: &'a SessionLocation,
        /// The sessions it holds.
        sessions: &'a [Session],
        /// Whether its sessions are drawn as their own rows.
        sessions_shown: bool,
    },
    /// A session row.
    Session(SessionId),
}

/// Whether `row` wears the selected look (feature 1125, FR-009, US2.4): none while the Overview is
/// open; otherwise the row holding the viewed session (its own row when the sessions are drawn,
/// else its location's), and the location row the Overview sent the user to (feature 813, R12).
pub fn row_selected(selection: &RowSelection<'_>, row: SelectableRow<'_>) -> bool {
    if selection.overview_open {
        return false;
    }
    match row {
        SelectableRow::Session(id) => selection.viewed == Some(id),
        SelectableRow::Location {
            location,
            sessions,
            sessions_shown,
        } => {
            let holds_viewed = selection
                .viewed
                .is_some_and(|viewed| sessions.iter().any(|s| s.id == viewed));
            (holds_viewed && !sessions_shown) || selection.revealed == Some(location)
        }
    }
}

/// Which rendered row holds the current session, if any (feature 024).
///
/// Counted the way the list is built — headers, locations, and the sessions of the open ones —
/// because the answer is a position in what is on screen, not a position in the model.
pub fn current_session_row(rows: &[ListRow], current: Option<SessionId>) -> Option<usize> {
    row_index(rows, &RowRef::Session(current?))
}

/// Which rendered row `target` is, if the list draws it (feature 803, plan D4).
///
/// Counted the way the list is built — headers, locations, and the sessions of the open ones —
/// because the answer is a position in what is on screen, not a position in the model. A session
/// whose location draws no sub-rows is on its location's row. A target in a closed group is not
/// drawn, so it is `None`.
pub fn row_index(rows: &[ListRow], target: &RowRef) -> Option<usize> {
    let mut row = 0usize;
    let location = |row: &mut usize,
                    location: SessionLocation,
                    shown: bool,
                    sessions: &[&Session]|
     -> Option<usize> {
        let at = *row;
        *row += 1;
        if shown {
            *row += sessions.len();
        }
        match target {
            RowRef::Location(wanted) => (*wanted == location).then_some(at),
            RowRef::Session(id) if !shown => sessions.iter().any(|s| s.id == *id).then_some(at),
            RowRef::Session(id) => sessions
                .iter()
                .position(|s| s.id == *id)
                .map(|i| at + 1 + i),
        }
    };
    for list_row in rows {
        let ListRow::Entry(entry) = list_row else {
            row += 1;
            continue;
        };
        let found = match entry {
            SidebarEntry::Default(node) => location(
                &mut row,
                SessionLocation::Default,
                node.sessions_shown(),
                &node.listed_sessions(),
            ),
            SidebarEntry::Worktree(node) => location(
                &mut row,
                node.location(),
                node.sessions_shown(),
                &node.listed_sessions(),
            ),
            SidebarEntry::Group(group) => {
                row += 1;
                let mut found = None;
                if group.expanded {
                    for run in &group.runs {
                        if let RunRow::Worktree(node) = &run.row {
                            found = found.or(location(
                                &mut row,
                                node.location(),
                                node.sessions_shown(),
                                &node.listed_sessions(),
                            ));
                        } else {
                            row += 1;
                        }
                    }
                }
                found
            }
        };
        if found.is_some() {
            return found;
        }
    }
    None
}

/// Whether the pointer that entered `dir`'s row at `index` is still over it (feature 1125, R14):
/// the row is still drawn at `index` in [`crate::app::State::sidebar_list`]. iced sends no exit when
/// content moves under a still pointer, so a row a state change moved out from under it loses its
/// hover.
pub fn hover_holds(state: &crate::app::State, dir: &str, index: usize) -> bool {
    location_row(
        &state.sidebar_list(),
        &SessionLocation::Worktree(dir.to_string()),
    ) == Some(index)
}

/// Which rendered row is `target`'s own location row, if the list draws it (feature 813, R12).
///
/// Counted exactly as [`row_heights`] lays the list out, so the index addresses the same row.
pub fn location_row(rows: &[ListRow], target: &SessionLocation) -> Option<usize> {
    let mut row = 0usize;
    let step = |at: &SessionLocation, shown: bool, sessions: usize, row: &mut usize| {
        let found = (at == target).then_some(*row);
        *row += 1 + if shown { sessions } else { 0 };
        found
    };
    for list_row in rows {
        let ListRow::Entry(entry) = list_row else {
            row += 1;
            continue;
        };
        match entry {
            SidebarEntry::Default(node) => {
                let at = SessionLocation::Default;
                if let Some(found) = step(&at, node.sessions_shown(), node.sessions.len(), &mut row)
                {
                    return Some(found);
                }
            }
            SidebarEntry::Worktree(node) => {
                let at = node.location();
                if let Some(found) = step(&at, node.sessions_shown(), node.sessions.len(), &mut row)
                {
                    return Some(found);
                }
            }
            SidebarEntry::Group(group) => {
                row += 1;
                if group.expanded {
                    for run in &group.runs {
                        match &run.row {
                            RunRow::Worktree(node) => {
                                let at = node.location();
                                if let Some(found) =
                                    step(&at, node.sessions_shown(), node.sessions.len(), &mut row)
                                {
                                    return Some(found);
                                }
                            }
                            RunRow::NoWorktree => row += 1,
                        }
                    }
                }
            }
        }
    }
    None
}

/// Where the sidebar's list should sit so that row `index` is fully visible, or `None` if it
/// already is (feature 024, contract §6).
///
/// The minimal move rather than a centring one: the spec asks only that the row be visible, and the
/// smallest movement is the one least likely to disturb what the user was reading (FR-008, FR-009).
///
/// `None` also covers the three cases where scrolling would be a guess rather than a decision: no
/// layout yet (`viewport_height` of zero — which means "unknown", never "nothing fits"), an index
/// the projection does not hold, and a list shorter than its own viewport.
pub fn scroll_target(
    heights: &[f32],
    index: usize,
    viewport_height: f32,
    current_offset: f32,
) -> Option<f32> {
    if viewport_height <= 0.0 || index >= heights.len() {
        return None;
    }
    let top: f32 = heights[..index].iter().sum::<f32>() + ROW_GAP * index as f32;
    let bottom = top + heights[index];
    if top >= current_offset && bottom <= current_offset + viewport_height {
        return None;
    }
    let content = heights.iter().sum::<f32>() + ROW_GAP * (heights.len().saturating_sub(1)) as f32;
    let max_offset = (content - viewport_height).max(0.0);
    let wanted = if top < current_offset {
        top
    } else {
        bottom - viewport_height
    };
    let clamped = wanted.clamp(0.0, max_offset);
    (clamped != current_offset).then_some(clamped)
}

/// The fixed location-tooltip label for the "Default" sidebar entry (feature 010, FR-010) —
/// unlike a worktree's label, this never varies, since the Default entry is always exactly the
/// project root.
pub const DEFAULT_LOCATION_LABEL: &str = "Project root";

/// What a row says when its daemon was removed, or none was ever bound and no legacy default
/// supplies one (feature 491, FR-014).
pub const NO_DAEMON_LABEL: &str = "no daemon";

/// Everything a worktree row's hover tooltip says, as one labelled line per fact
/// (feature 029, contract `worktree-tooltip.md`).
///
/// Pure and total: no I/O and no clock, defined for every combination of its inputs — an unknown
/// `project_root`, a worktree with no branch, any [`WorktreeStatus`]. Every fact it renders is
/// already in the `Worktree` the row was built from, which is what keeps the hover path free of
/// work (FR-012).
///
/// A fact the worktree does not have is an **absent line**, never a blank one and never a label
/// with nothing after it (§2.2). That is the rule the whole shape turns on: a tooltip padded with
/// empty rows is worse than a short one, because the reader cannot tell an unknown from a missing
/// answer.
///
/// It replaces `worktree_location_label`, whose single value is now the `Location` line. That
/// line's own behaviour is unchanged from feature 010's FR-010: relative to the project root when
/// the worktree lives under it, absolute otherwise. Every worktree this app creates lives directly
/// under `<project_root>/.claude/worktrees/`, so a plain `strip_prefix` suffices — the absolute
/// fallback covers a worktree the app did not create (BUG-002's `included` rows), where rendering
/// nothing would be worse than rendering a long path.
pub fn worktree_tooltip(
    project_root: Option<&Path>,
    worktree: &Worktree,
    display_name: &str,
    pull_request: Option<RowPullRequest<'_>>,
) -> String {
    let mut lines: Vec<String> = Vec::new();

    // First, always: the fact a shortened row could not show is the fact the reader hovered for
    // (§2.3). `display_name` is the caller's — the same string the row renders, which a user's
    // rename overrides — rather than a second derivation from `dir_name`, so the row and its
    // tooltip cannot disagree about a name (FR-002).
    lines.push(format!("Name: {display_name}"));

    // The row's label is prettified — the type token and the ticket are stripped out of it — so
    // the branch it is bound to appears nowhere on the row (FR-004).
    if let Some(branch) = &worktree.branch {
        lines.push(format!("Branch: {branch}"));
    }

    // The folder you would `cd` into, but only when it is not already on the line above. Printing
    // the same string under two labels teaches nothing and costs the reader a comparison (FR-005).
    if worktree.dir_name != display_name {
        lines.push(format!("Folder: {}", worktree.dir_name));
    }

    if let Some(root) = project_root {
        // A remote worktree is known by the host's own path (feature 489, FR-012).
        let mut location = if micold_core::ssh::RemoteLocation::parse(&worktree.path).is_some() {
            micold_core::ssh::remote_path::display_path(&worktree.path)
        } else {
            worktree
                .path
                .strip_prefix(root)
                .map(|rel| rel.display().to_string())
                .unwrap_or_else(|_| worktree.path.display().to_string())
        };
        // On the location line rather than a line of its own, because the location is the thing it
        // explains: an included worktree's path is absolute by construction (BUG-002), and among
        // the relative ones that reads as an oddity until something says why (FR-007). The phrase
        // is the row's own chip, word for word.
        if worktree.included {
            location.push_str(" (outside this app)");
        }
        lines.push(format!("Location: {location}"));
    }

    // Last: the condition is read after the thing it is a condition of. `label()` returning `None`
    // for a healthy worktree is what keeps `Status: ok` off every row — normal reads as normal.
    if let Some(status) = worktree.status.label() {
        lines.push(format!("Status: {status}"));
    }

    // Feature 040 (FR-010), after everything the tooltip has today, so a row with no pull request
    // is untouched (FR-011). Held facts only: nothing here reads a disk or asks GitHub (FR-012).
    if let Some(row) = pull_request {
        lines.extend(pull_request_lines(row.status));
        // After the review line (UI §3, line 5): how old the reading is, only once it is stale
        // (FR-019). A fresh reading adds nothing.
        if row.stale {
            lines.push(format!("Read: {}", read_age(row.age_secs)));
        }
        // Last (UI §3, line 6): the removal suggestion, only for a merged pull request that the
        // repository shows holds all of the branch's work (FR-015, FR-017). It says how to remove
        // the worktree by the route that exists; nothing is removed by it (FR-016).
        if row.removable {
            lines.push(
                "Cleanup: merged — this worktree can be removed (right-click, Delete)".into(),
            );
        }
    }

    lines.join("\n")
}

/// How long ago a reading was made, in words: whole minutes, and from 120 minutes whole hours.
fn read_age(age_secs: u64) -> String {
    let minutes = age_secs / 60;
    if minutes >= 120 {
        format!("{} h ago", minutes / 60)
    } else {
        format!("{minutes} min ago")
    }
}

/// The longest title the tooltip shows, in characters, the last being `…` when it was cut.
const TITLE_LIMIT: usize = 72;

/// The tooltip lines for a pull request (contract pull-request-ui §3, lines 1 to 4): the number
/// and title, the state, the check status when it has one, and the review decision when GitHub
/// reports one. A fact the pull request does not have is an absent line.
fn pull_request_lines(status: &PullRequestStatus) -> Vec<String> {
    let (state, checks) = match status.state {
        PrState::Open { checks } => ("open", Some(checks)),
        PrState::Draft { checks } => ("draft", Some(checks)),
        PrState::Merged => ("merged", None),
        PrState::Closed => ("closed", None),
    };
    let mut lines = vec![
        format!(
            "Pull request: #{} {}",
            status.number,
            tooltip_title(&status.title)
        ),
        format!("PR state: {state}"),
    ];
    let checks = match checks {
        Some(CheckStatus::Passing) => Some("passing"),
        Some(CheckStatus::Pending) => Some("pending"),
        Some(CheckStatus::Failing) => Some("failing"),
        Some(CheckStatus::None) | None => None,
    };
    if let Some(checks) = checks {
        lines.push(format!("Checks: {checks}"));
    }
    let review = match status.review {
        ReviewState::Approved => Some("approved"),
        ReviewState::ChangesRequested => Some("changes requested"),
        ReviewState::ReviewRequired => Some("review required"),
        ReviewState::None => None,
    };
    if let Some(review) = review {
        lines.push(format!("Review: {review}"));
    }
    lines
}

/// A pull request's title as one tooltip line: control characters and line breaks become spaces (a
/// title is typed by someone else and must not add lines or escape sequences), and a title of more
/// than [`TITLE_LIMIT`] characters is cut so its last character is `…`. The number comes first on
/// the line, so it is what stays readable (story 2 scenario 6).
fn tooltip_title(title: &str) -> String {
    let flat: String = title
        .chars()
        .map(|c| {
            // Cc, plus the invisible format and line-separator characters that reorder or hide
            // text (zero-width, bidi marks and overrides, U+2028/2029, BOM).
            let hides = matches!(c, '\u{200B}'..='\u{200F}' | '\u{2028}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}');
            if c.is_control() || hides {
                ' '
            } else {
                c
            }
        })
        .collect();
    if flat.chars().count() <= TITLE_LIMIT {
        return flat;
    }
    let mut cut: String = flat.chars().take(TITLE_LIMIT - 1).collect();
    cut.push('…');
    cut
}

/// What the row of a worktree shows of its pull request (feature 040, data-model §4): the status
/// held for its branch and how old the reading is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowPullRequest<'a> {
    /// The status held for the row's branch.
    pub status: &'a PullRequestStatus,
    /// Seconds since the reading that produced it started.
    pub age_secs: u64,
    /// The reading is older than two refresh intervals (FR-019): the indicator is drawn in its
    /// lower-emphasis form and the tooltip says how long ago it was read.
    pub stale: bool,
    /// The pull request was merged and the branch holds nothing newer (FR-015): the row suggests
    /// removing the worktree.
    pub removable: bool,
}

/// The pull request `entry`'s row shows, if any (FR-001, FR-007). Only a worktree with a branch
/// can have one: the join is by branch name, and the "Default" entry never does. `now` is passed in:
/// the reducer has no clock.
pub fn row_pull_request<'a>(
    entry: &SidebarEntry,
    statuses: &'a BTreeMap<String, PullRequestStatus>,
    removable: &BTreeSet<String>,
    read_at: Option<u64>,
    now: u64,
) -> Option<RowPullRequest<'a>> {
    let SidebarEntry::Worktree(node) = entry else {
        return None;
    };
    let branch = node.worktree.branch.as_deref()?;
    let status = statuses.get(branch)?;
    let read = read_at.unwrap_or(now);
    Some(RowPullRequest {
        status,
        age_secs: now.saturating_sub(read),
        stale: micold_core::pull_request::is_stale(read, now),
        removable: removable.contains(branch),
    })
}

/// Where an address GitHub reported must start for **Open pull request** to open it (FR-014): a
/// pull request's address comes from a program's answer, and a browser is handed it, so nothing
/// else than a GitHub page is.
pub const GITHUB_ADDRESS_PREFIX: &str = "https://github.com/";

/// The address **Open pull request** opens for the worktree `dir_name`, or `None` when nothing
/// should be opened: the row is gone, has no branch, has lost its status, or the address GitHub
/// reported does not start with [`GITHUB_ADDRESS_PREFIX`]. It is the held address, never one
/// built from the number (FR-014).
pub fn pull_request_address_to_open(
    worktrees: &[Worktree],
    statuses: &BTreeMap<String, PullRequestStatus>,
    dir_name: &str,
) -> Option<String> {
    let branch = worktrees
        .iter()
        .find(|w| w.dir_name == dir_name)?
        .branch
        .as_deref()?;
    let address = &statuses.get(branch)?.url;
    address
        .starts_with(GITHUB_ADDRESS_PREFIX)
        .then(|| address.clone())
}

/// The tooltip line that states a location's unread count in words (feature 575, FR-005).
pub fn unread_tooltip_line(n: usize) -> Option<String> {
    match n {
        0 => None,
        1 => Some("1 unread session".to_string()),
        n => Some(format!("{n} unread sessions")),
    }
}

/// `tooltip` with the state's line in words appended on a line of its own (feature 905, FR-009).
/// Composed inside [`with_unread_line`], so the state line comes before the unread line.
pub fn with_state_line(tooltip: String, state: LocationState) -> String {
    format!("{tooltip}\n{}", state.tooltip_line())
}

/// `tooltip` with [`unread_tooltip_line`] appended on a line of its own (feature 575, FR-005).
pub fn with_unread_line(tooltip: String, n: usize) -> String {
    match unread_tooltip_line(n) {
        Some(line) => format!("{tooltip}\n{line}"),
        None => tooltip,
    }
}

/// `tooltip` with the row's full title as its first line (feature 1125, FR-012): the first line of
/// the row is the one a narrow sidebar shortens, so the tooltip leads with it in full. A title that
/// is the row's name or its branch is already on the tooltip's `Name` or `Branch` line, so it is not
/// added a second time.
pub fn with_title_line(node: &WorktreeNode, title: &str, tooltip: String) -> String {
    if title == node.display_name || node.worktree.branch.as_deref() == Some(title) {
        tooltip
    } else {
        format!("{title}\n{tooltip}")
    }
}

/// A worktree row's first line (feature 1125, data-model *Row text*): the user's rename of the
/// worktree when there is one (it wins over everything the app derives), else the known issue
/// title, else the branch, else the display name, as the Overview's card reads it (feature 1120,
/// FR-006).
pub fn row_title(node: &WorktreeNode, issue_title: Option<&str>, renamed: Option<&str>) -> String {
    match renamed.map(str::trim).filter(|name| !name.is_empty()) {
        Some(name) => name.to_string(),
        None => micold_core::overview_text::card_title(
            issue_title,
            node.worktree.branch.as_deref(),
            &node.display_name,
        ),
    }
}

/// A worktree row's second line (feature 1125, FR-011): the short branch, unless `title` is the
/// branch, then its sessions.
pub fn row_second_line(node: &WorktreeNode, title: &str) -> String {
    micold_core::overview_text::sidebar_second_line(
        micold_core::overview_text::RowKind::Worktree,
        title,
        node.worktree.branch.as_deref(),
        micold_core::overview::session_counts(&node.sessions),
    )
}

/// The *Default* row's second line (feature 1125, FR-011): "project root", then its sessions.
pub fn default_second_line(node: &DefaultNode) -> String {
    micold_core::overview_text::sidebar_second_line(
        micold_core::overview_text::RowKind::ProjectRoot,
        node.display_name,
        None,
        micold_core::overview::session_counts(&node.sessions),
    )
}

/// A label chip on a worktree row's second line (feature 1125, FR-013).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowChip {
    /// A missing or invalid worktree's status, in its tag's word (feature 008, FR-011).
    Status(String),
    /// Listed only because it holds the current session (feature 024, FR-012a).
    CurrentSession,
    /// The daemon it runs on, or "no daemon" (feature 491, FR-005).
    Daemon(String),
    /// A worktree the user included from elsewhere (016 BUG-002, FR-029).
    OutsideThisApp,
    /// A merged pull request holds all its work (feature 040, FR-015).
    CanBeRemoved,
}

impl RowChip {
    /// The words the chip shows.
    pub fn label(&self) -> String {
        match self {
            Self::Status(label) | Self::Daemon(label) => label.clone(),
            Self::CurrentSession => "current session".into(),
            Self::OutsideThisApp => "outside this app".into(),
            Self::CanBeRemoved => "can be removed".into(),
        }
    }
}

/// A worktree row's label chips in the order the row draws them (feature 1125, FR-013): today's
/// chips without the type and issue tags, which the second line's short branch carries.
/// `node.tags` still drives the filters. `removable` is the row's pull request's say.
pub fn row_chips(node: &WorktreeNode, removable: bool) -> Vec<RowChip> {
    let mut chips: Vec<RowChip> = node
        .tags
        .iter()
        .filter(|tag| matches!(tag, Tag::Status(_)))
        .map(|tag| RowChip::Status(tag.label()))
        .collect();
    if node.shown_for_current_session {
        chips.push(RowChip::CurrentSession);
    }
    chips.push(RowChip::Daemon(node.daemon.clone()));
    if node.worktree.included {
        chips.push(RowChip::OutsideThisApp);
    }
    if removable {
        chips.push(RowChip::CanBeRemoved);
    }
    chips
}

impl crate::app::State {
    /// The daemon an active-project location is bound to (feature 491, FR-003, FR-006): its stored
    /// binding, else the legacy default, and `NoDaemon` when that id is not in the registry.
    /// `key` is a worktree's `dir_name`, or `""` for the Default location.
    pub fn daemon_binding(&self, key: &str) -> micold_core::daemons::Binding {
        let stored = self
            .workspace
            .active
            .as_ref()
            .and_then(|project| self.workspace.bindings.get(project))
            .cloned()
            .unwrap_or_default();
        let project = self.workspace.active.clone().unwrap_or_default();
        micold_core::daemons::Binding::resolve_for(
            &project,
            &stored,
            key,
            &self.settings.daemons,
            self.settings.legacy_default_daemon,
        )
    }

    /// [`Self::daemon_binding`] as the text a row shows: the daemon's name, or [`NO_DAEMON_LABEL`].
    pub fn daemon_label_of(&self, key: &str) -> String {
        self.daemon_text_of(key, |entry| entry.name.as_str().to_string())
    }

    /// Where `key`'s daemon runs, as the status bar names it (feature 1124, research R6): "local"
    /// for the host daemon, the daemon's name for any other runtime, or [`NO_DAEMON_LABEL`].
    pub fn daemon_location_of(&self, key: &str) -> String {
        self.daemon_text_of(key, |entry| match entry.runtime {
            micold_core::daemons::DaemonRuntime::Host => "local".to_string(),
            _ => entry.name.as_str().to_string(),
        })
    }

    /// The one lookup behind [`Self::daemon_label_of`] and [`Self::daemon_location_of`]: `key`'s
    /// bound daemon put into words by `text`, or [`NO_DAEMON_LABEL`] when it is bound to none or to
    /// an id the registry no longer holds.
    fn daemon_text_of(
        &self,
        key: &str,
        text: impl Fn(&micold_core::daemons::DaemonEntry) -> String,
    ) -> String {
        match self.daemon_binding(key) {
            micold_core::daemons::Binding::Bound(id) => self
                .settings
                .daemons
                .get(id)
                .map(text)
                .unwrap_or_else(|| NO_DAEMON_LABEL.to_string()),
            micold_core::daemons::Binding::NoDaemon => NO_DAEMON_LABEL.to_string(),
        }
    }

    /// What a row shows in place of its daemon chip given the live connections (feature 491, T028):
    /// the contract's unavailable / version-mismatch / paths-mapped text when its daemon is not
    /// plainly connected, and [`Self::daemon_label_of`] otherwise, so rows on healthy daemons are
    /// unchanged. Computed from `links` on every call and never cached, so a state change reaches
    /// the row in the step that applied it.
    pub fn daemon_status_label_of(
        &self,
        key: &str,
        links: &crate::links::DaemonLinks,
        facts: crate::links::LabelFacts,
    ) -> String {
        crate::links::unavailable_label(
            &self.settings.daemons,
            links,
            self.daemon_binding(key),
            facts,
        )
        .unwrap_or_else(|| self.daemon_label_of(key))
    }

    /// Whether a live session of the row `key` is one its daemon did not list on reconnect.
    fn has_session_lost_to_daemon(&self, key: &str) -> bool {
        self.active_sessions().iter().any(|s| {
            let here = match &s.location {
                micold_core::session::SessionLocation::Worktree(dir) => dir == key,
                micold_core::session::SessionLocation::Default => key.is_empty(),
            };
            here && !s.archived && self.settings.lost_sessions.contains(&s.id)
        })
    }

    /// The row's daemon text from this window's own links and the facts it holds (feature 491,
    /// T028): what [`worktree_tree`](Self::worktree_tree) puts on each row.
    pub fn daemon_row_label(&self, key: &str) -> String {
        use micold_core::daemons::{Binding, DaemonRuntime};
        let container = match self.daemon_binding(key) {
            Binding::Bound(id) => self
                .settings
                .daemons
                .get(id)
                .is_some_and(|e| matches!(e.runtime, DaemonRuntime::Container(_))),
            Binding::NoDaemon => false,
        };
        let facts = crate::links::LabelFacts {
            runtime_missing: container && self.settings.runtime_missing,
            paths_mapped: container && cfg!(windows),
            lost_sessions: self.active_sessions().iter().any(|s| {
                s.location.is_worktree(key)
                    && !s.archived
                    && matches!(
                        s.lifecycle,
                        micold_core::session::SessionLifecycle::Running
                            | micold_core::session::SessionLifecycle::Starting
                    )
            }),
        };
        // A daemon this window holds no link for has nothing to report: its plain name.
        match self.daemon_binding(key) {
            Binding::Bound(id) if self.settings.links.state(id).is_none() => {
                self.daemon_label_of(key)
            }
            // Reconnected, and the daemon no longer has a live session of this row (FR-019).
            Binding::Bound(id) if self.has_session_lost_to_daemon(key) => {
                crate::links::session_lost_label(
                    &self.settings.daemons,
                    &self.settings.links,
                    id,
                    &micold_core::session::SessionLifecycle::Running,
                    true,
                )
                .unwrap_or_else(|| self.daemon_status_label_of(key, &self.settings.links, facts))
            }
            _ => self.daemon_status_label_of(key, &self.settings.links, facts),
        }
    }

    /// Where the current session lives, if the panel can point at it (feature 024, contract §1.2).
    ///
    /// `None` in three cases, all of which reduce [`Self::location_open`] to the user's own
    /// expansion and open nothing: there is no current session; its record is gone from the active
    /// project; or its location is not among the project's known ones — a worktree removed or
    /// missing while the project was inactive (FR-013). The third is why this resolves against
    /// `self.worktree.worktrees` rather than trusting the session's own `location`.
    pub fn current_session_location(&self) -> Option<SessionLocation> {
        let id = self.session.active?;
        let location = self
            .active_sessions()
            .iter()
            .find(|s| s.id == id)?
            .location
            .clone();
        match &location {
            SessionLocation::Default => Some(location),
            SessionLocation::Worktree(dir) => self
                .worktree
                .worktrees
                .iter()
                .any(|w| &w.dir_name == dir)
                .then_some(location),
        }
    }

    /// Whether the user has closed the revealed row for the session that is current *now*
    /// (feature 024, invariant I2).
    ///
    /// The `is_some` guard is not redundant: without it, "no session is current and nothing is
    /// suppressed" would compare `None == None` and report a suppression.
    pub fn reveal_suppressed(&self) -> bool {
        self.session.active.is_some() && self.session.reveal_suppressed_for == self.session.active
    }

    /// Fold a location into the user's own open set (T067a-6).
    ///
    /// Reached from `Outcome::LocationOpened`. Idempotent: a location already open stays open.
    pub fn location_opened(&mut self, location: &SessionLocation) {
        match location {
            SessionLocation::Worktree(dir) => {
                self.sidebar.expanded.insert(dir.clone());
            }
            SessionLocation::Default => self.sidebar.default_expanded = true,
        }
    }

    /// Arm the scroll that reaches the current session's row on the first frame one exists.
    pub fn reveal_scroll_armed(&mut self) {
        self.sidebar.pending_reveal_scroll = true;
    }

    /// Drop view state that must not follow the user into a different project (T067a-6).
    pub fn project_entered(&mut self) {
        self.sidebar.default_expanded = false;
        // Feature 803 (U5): a stop or keyboard row of the project left behind means nothing here.
        self.keyboard_row_left();
    }

    /// Whether a location's row is shown open (feature 024, contract §1.1).
    ///
    /// The single answer to "is this row open", derived on every call. `expanded` and
    /// `default_expanded` keep their meaning and lose their monopoly: they are now strictly the
    /// *user's* open set, and this is the union of that with the app's one revealed row.
    pub fn location_open(&self, location: &SessionLocation) -> bool {
        let user_open = match location {
            SessionLocation::Worktree(dir) => self.sidebar.expanded.contains(dir),
            SessionLocation::Default => self.sidebar.default_expanded,
        };
        effective_open(
            user_open,
            self.current_session_location().as_ref() == Some(location),
            self.reveal_suppressed(),
        )
    }

    /// Where the sidebar should scroll so the current session's row is visible, or `None` when it
    /// already is — or when the answer would be a guess (feature 024, contract §6).
    ///
    /// `None` while the projection holds no row for the current session, which is the async case
    /// research R7 is about: the incoming project's worktree list arrives after the switch, so the
    /// row does not exist for the first frame or two. The caller keeps the reveal armed rather than
    /// scrolling to a stale offset (invariant I4).
    pub fn reveal_scroll_offset(&self) -> Option<u32> {
        let rows = self.sidebar_list();
        let index = self.reveal_row(&rows)?;
        let heights = row_heights(&rows);
        if crate::reveal_trace::enabled() {
            let top: f32 = heights[..index].iter().sum::<f32>() + ROW_GAP * index as f32;
            let content =
                heights.iter().sum::<f32>() + ROW_GAP * heights.len().saturating_sub(1) as f32;
            crate::reveal_trace::line(format_args!(
                "geometry: row {index} of {} spans {top}..{}, content {content}, viewport {}, \
                 offset {}",
                heights.len(),
                top + heights[index],
                self.sidebar.viewport_height,
                self.sidebar.scroll_offset,
            ));
        }
        scroll_target(
            &heights,
            index,
            self.sidebar.viewport_height as f32,
            self.sidebar.scroll_offset as f32,
        )
        .map(|offset| offset.round().max(0.0) as u32)
    }

    /// Where the list should sit to show the row the Overview revealed (feature 813, R12), or
    /// `None` when nothing is revealed, the row is not drawn, no layout is known, or it is visible.
    pub fn revealed_scroll_offset(&self) -> Option<u32> {
        let rows = self.sidebar_list();
        let index = location_row(&rows, self.sidebar.revealed.as_ref()?)?;
        scroll_target(
            &row_heights(&rows),
            index,
            self.sidebar.viewport_height as f32,
            self.sidebar.scroll_offset as f32,
        )
        .map(|offset| offset.round().max(0.0) as u32)
    }

    /// Whether the projection currently holds a row for the current session (feature 024).
    ///
    /// The condition for draining an armed reveal: until it is true there is nothing to scroll to,
    /// and the arm stays set.
    pub fn current_session_is_listed(&self) -> bool {
        self.reveal_row(&self.sidebar_list()).is_some()
    }

    /// Whether an armed reveal can drain now: armed, the viewport laid out (a height of `0` means
    /// unknown, contract §6.3) and a row to reach (research R7). First opens the closed state group
    /// of the reveal's target (feature 1125, FR-006, R5), so a chord into a closed group reaches
    /// its row rather than falling back to the current session's drawn one.
    pub fn reveal_ready(&mut self) -> bool {
        if !self.sidebar.pending_reveal_scroll || self.sidebar.viewport_height == 0 {
            return false;
        }
        self.open_group_of_reveal_target();
        self.current_session_is_listed()
    }

    /// The row the reveal scroll reaches (feature 803, contracts/ui.md U3): the row a navigation
    /// chord gave the keyboard while the list draws it, else the current session's row.
    fn reveal_row(&self, rows: &[ListRow]) -> Option<usize> {
        self.sidebar
            .keyboard_row
            .as_ref()
            .and_then(|target| row_index(rows, target))
            .or_else(|| current_session_row(rows, self.session.active))
    }

    /// Open the closed state group holding the reveal's target (feature 1125, FR-006, research
    /// R5): the row a navigation chord gave the keyboard, else the current session's location.
    /// Recorded as the user's choice, so the group stays open after the reveal. Called by
    /// [`State::reveal_ready`], before asking whether its row is drawn.
    pub fn open_group_of_reveal_target(&mut self) {
        let groups = self.sidebar_groups();
        let listed = |location: &SessionLocation| {
            groups
                .iter()
                .any(|g| g.entries.iter().any(|e| e.holds(location)))
        };
        let keyboard = self
            .sidebar
            .keyboard_row
            .as_ref()
            .and_then(|row| match row {
                RowRef::Location(location) => Some(location.clone()),
                RowRef::Session(id) => self
                    .active_sessions()
                    .iter()
                    .find(|s| s.id == *id)
                    .map(|s| s.location.clone()),
            })
            .filter(|location| listed(location));
        if let Some(target) = keyboard.or_else(|| self.current_session_location()) {
            self.open_group_of(&groups, &target);
        }
    }

    /// Open the closed state group holding the row the Overview revealed (feature 1125, FR-006,
    /// research R5), before the reveal's scroll is computed.
    pub fn open_group_of_revealed(&mut self) {
        if let Some(target) = self.sidebar.revealed.clone() {
            let groups = self.sidebar_groups();
            self.open_group_of(&groups, &target);
        }
    }

    /// Record `true` for the group of `groups` holding `location` when that group is closed; an
    /// open group's choice stays unrecorded.
    fn open_group_of(&mut self, groups: &[SidebarGroup], location: &SessionLocation) {
        let Some(project) = self.workspace.active.clone() else {
            return;
        };
        let closed = groups
            .iter()
            .find(|g| g.entries.iter().any(|e| e.holds(location)))
            .filter(|g| !g.open);
        if let Some(group) = closed {
            self.sidebar
                .group_choices
                .entry(project)
                .or_default()
                .insert(group.group, true);
        }
    }

    /// The card state an entry's group is chosen by (feature 1125, R2): the location's own, from
    /// the same call the Overview's card makes; a run group's, the worst over its runs (G6).
    fn entry_card_state(&self, entry: &SidebarEntry) -> micold_core::overview::CardState {
        let of = |node: &WorktreeNode| {
            crate::features::overview::location_card_state(
                self,
                &node.location(),
                node.worktree.branch.as_deref(),
            )
            .0
        };
        match entry {
            SidebarEntry::Default(_) => {
                crate::features::overview::location_card_state(
                    self,
                    &SessionLocation::Default,
                    None,
                )
                .0
            }
            SidebarEntry::Worktree(node) => of(node),
            SidebarEntry::Group(group) => {
                run_group_state(group.runs.iter().map(|run| match &run.row {
                    RunRow::Worktree(node) => RunCard::Worktree(of(node)),
                    RunRow::NoWorktree => RunCard::NoWorktree {
                        failed: run.failed(),
                    },
                }))
            }
        }
    }

    /// The sidebar's entries in state groups (feature 1125, FR-001–FR-005, contract S4): the
    /// groups that hold an entry, in [`StateGroup::ALL`] order, each with its entries in
    /// [`Self::sidebar_entries`] order, and whether it is open. Empty when no project is open.
    pub fn sidebar_groups(&self) -> Vec<SidebarGroup> {
        let current = self.current_session_location();
        let choices = self
            .workspace
            .active
            .as_ref()
            .and_then(|project| self.sidebar.group_choices.get(project));
        let entries = self.sidebar_entries();
        group_by_state(
            entries
                .into_iter()
                .map(|entry| (self.entry_card_state(&entry), entry)),
        )
        .into_iter()
        .map(|group| {
            let holds_current = current
                .as_ref()
                .is_some_and(|c| group.items.iter().any(|e| e.holds(c)));
            let choice = choices.and_then(|c| c.get(&group.group).copied());
            SidebarGroup {
                group: group.group,
                open: group_open(choice, group.group, holds_current),
                entries: group.items,
            }
        })
        .collect()
    }

    /// The sidebar's drawn list (feature 1125, contract S4): [`list_rows`] of
    /// [`Self::sidebar_groups`].
    pub fn sidebar_list(&self) -> Vec<ListRow> {
        list_rows(self.sidebar_groups())
    }

    /// The sidebar's keyboard left the row a navigation chord gave it, or the current session
    /// changed (feature 803, contracts/ui.md U5): no row holds it and no stop is pending.
    pub fn keyboard_row_left(&mut self) {
        self.sidebar.keyboard_row = None;
        self.sidebar.nav_stop = None;
    }

    /// Open or close a location's row, from the user's own twisty (feature 024, contract §2.1).
    ///
    /// Toggling against [`Self::location_open`] rather than against `expanded` is the whole point:
    /// the row the app revealed is not in the user's set, so a toggle that only removed a key
    /// would leave the one row this feature adds with a twisty that visibly does nothing.
    ///
    /// Closing the revealed row is remembered against the session it was closed for; opening it
    /// again withdraws that, rather than leaving behind a suppression only a change of session
    /// could clear.
    pub fn toggle_location(&mut self, location: SessionLocation) -> Vec<crate::features::Outcome> {
        let open = self.location_open(&location);
        let holds_current = self.current_session_location().as_ref() == Some(&location);
        match &location {
            SessionLocation::Worktree(dir) => {
                if open {
                    self.sidebar.expanded.remove(dir);
                } else {
                    self.sidebar.expanded.insert(dir.clone());
                }
            }
            SessionLocation::Default => self.sidebar.default_expanded = !open,
        }
        if holds_current {
            // Whether the reveal is suppressed is a fact about the session, not the row (T067a-6).
            vec![crate::features::Outcome::RevealSuppressed(open)]
        } else {
            Vec::new()
        }
    }

    /// Build the sidebar tree: worktrees (top level) each joined with their sessions and
    /// expansion state (FR-002, FR-003). Sessions are matched to worktrees by `dir_name`.
    /// Built from every worktree in `self.worktree.worktrees` (552: no worktree is hidden by name).
    pub fn worktree_tree(&self) -> Vec<WorktreeNode> {
        let sessions = self.active_sessions();
        self.worktree
            .worktrees
            .iter()
            .map(|worktree| WorktreeNode {
                display_name: self.worktree_display_name(&worktree.dir_name),
                tags: worktree_tags(worktree),
                expanded: self.location_open(&SessionLocation::Worktree(worktree.dir_name.clone())),
                sessions: sessions
                    .iter()
                    .filter(|s| s.location.is_worktree(&worktree.dir_name) && !s.archived)
                    .cloned()
                    .collect(),
                daemon: self.daemon_row_label(&worktree.dir_name),
                worktree: worktree.clone(),
                shown_for_current_session: false,
                folded: std::sync::Arc::new(self.sidebar.folded.clone()),
            })
            .collect()
    }

    /// The worktree tree narrowed to the active tag filters (feature 008, FR-025). With no
    /// filter active this equals [`crate::app::State::worktree_tree`]. Used by the sidebar to render only
    /// matching worktrees; a subsequent add/rename/delete re-runs this so the list stays
    /// consistent (FR-028).
    /// (Feature 024) The location holding the current session survives the filters, and says so.
    ///
    /// The tag filters hide rows from the tree above, so the re-admitted row is built from
    /// `self.worktree.worktrees` rather than from that tree.
    ///
    /// Exactly one row can be re-admitted, because there is one current session. That is what keeps
    /// this an exemption rather than a filter bypass (FR-012), and it is why the row carries
    /// `shown_for_current_session`: a row that survived a filter it does not match is otherwise
    /// unexplained, and the user is the one who set that filter.
    pub fn filtered_worktree_tree(&self) -> Vec<WorktreeNode> {
        let mut tree: Vec<WorktreeNode> = self
            .worktree_tree()
            .into_iter()
            .filter(|node| matches_filters(&node.tags, &self.sidebar.filters))
            .collect();

        let Some(SessionLocation::Worktree(dir)) = self.current_session_location() else {
            return tree;
        };
        if tree.iter().any(|node| node.worktree.dir_name == dir) {
            // Already listed on its own merits, so it needs no exemption and must not claim one.
            return tree;
        }
        let Some(worktree) = self.worktree.worktrees.iter().find(|w| w.dir_name == dir) else {
            return tree;
        };
        let sessions = self.active_sessions();
        let node = WorktreeNode {
            display_name: self.worktree_display_name(&worktree.dir_name),
            tags: worktree_tags(worktree),
            expanded: self.location_open(&SessionLocation::Worktree(dir.clone())),
            sessions: sessions
                .iter()
                .filter(|s| s.location.is_worktree(&dir) && !s.archived)
                .cloned()
                .collect(),
            daemon: self.daemon_row_label(&worktree.dir_name),
            worktree: worktree.clone(),
            shown_for_current_session: true,
            folded: std::sync::Arc::new(self.sidebar.folded.clone()),
        };
        // Inserted where it would have sat unfiltered rather than appended: the exemption changes
        // which rows are listed, never their order (FR-012a). `worktrees` is the order the panel
        // draws, so its position in that list is the answer.
        let at = self
            .worktree
            .worktrees
            .iter()
            .filter(|w| tree.iter().any(|n| n.worktree.dir_name == w.dir_name) || w.dir_name == dir)
            .position(|w| w.dir_name == dir)
            .unwrap_or(tree.len());
        tree.insert(at.min(tree.len()), node);
        tree
    }

    /// The full sidebar location list (feature 010): the "Default" entry first, then worktree
    /// entries narrowed to the active tag filters (`filtered_worktree_tree`). Empty when no
    /// project is open (contracts/sidebar-default-entry.md invariant 1) — mirrors how
    /// `worktree_tree` is empty with no active project. The Default entry is exempt from tag
    /// filtering (FR-011, research.md R4): it is included unconditionally whenever a project is
    /// open, regardless of `filters`.
    pub fn sidebar_entries(&self) -> Vec<SidebarEntry> {
        if self.workspace.active.is_none() {
            return Vec::new();
        }
        let default_sessions: Vec<Session> = self
            .active_sessions()
            .iter()
            .filter(|s| s.location == SessionLocation::Default && !s.archived)
            .cloned()
            .collect();
        let mut entries = vec![SidebarEntry::Default(DefaultNode {
            display_name: "Default",
            expanded: self.location_open(&SessionLocation::Default),
            sessions: default_sessions,
            folded: std::sync::Arc::new(self.sidebar.folded.clone()),
        })];
        entries.extend(
            self.filtered_worktree_tree()
                .into_iter()
                .map(SidebarEntry::Worktree),
        );
        arrange_groups(entries, &self.runs.groups, |id| self.runs.is_expanded(id))
    }

    /// The distinct tag filters offered for the current worktrees (feature 008, FR-024): a
    /// `Type` per conventional type present, `HasIssue` if any worktree embeds an issue key,
    /// and `Untyped` if any worktree lacks a type. Order: types first, then HasIssue, Untyped.
    ///
    /// Sourced from every worktree (552): an `agent-<hex>` worktree is listed like any other, so
    /// its missing type is a real `Untyped` row.
    pub fn available_tag_filters(&self) -> Vec<TagFilter> {
        let mut types = BTreeSet::new();
        let mut has_issue = false;
        let mut has_untyped = false;
        for worktree in &self.worktree.worktrees {
            let tags = worktree_tags(worktree);
            let mut typed = false;
            for tag in &tags {
                match tag {
                    Tag::Type(t) => {
                        types.insert(*t);
                        typed = true;
                    }
                    Tag::Issue(_) => has_issue = true,
                    Tag::Status(_) => {}
                }
            }
            if !typed {
                has_untyped = true;
            }
        }
        let mut out: Vec<TagFilter> = types.into_iter().map(TagFilter::Type).collect();
        if has_issue {
            out.push(TagFilter::HasIssue);
        }
        if has_untyped {
            out.push(TagFilter::Untyped);
        }
        out
    }
}

/// The chip bar's more (⋯) menu, as a floating surface (feature 1125, FR-015, contract S3).
///
/// It took the registry place of the tag-filter panel (feature 021, T029), whose chips are on the
/// chip bar at all times now. A popover with no state of its own beyond "is it showing" and the
/// corner it hangs from, so the surface is a marker and everything dispatch needs is the three
/// answers below: feature 017's Escape, outside press and displacement close it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SidebarMoreMenu;

impl SidebarMoreMenu {
    /// This surface's identity, nameable by the surfaces that displace it or that it
    /// displaces (T067a-2). The declaration has to point at something, and pointing at the
    /// literal string in two places is how the two would come to disagree.
    pub const ID: SurfaceId = SurfaceId::new("sidebar_more");
}

floating_surface!(SidebarMoreMenu, id: Self::ID, layer: Popover, cancelled_by: crate::app::Message::Sidebar(Msg::MoreMenuToggled));

impl Registered for SidebarMoreMenu {
    fn open_in(state: &crate::app::State) -> Option<Self> {
        state.sidebar.more_menu_open.then_some(SidebarMoreMenu)
    }
}

/// A tag filter was toggled on or off (feature 008, FR-024).
///
/// Filters combine with OR (FR-025); an empty set shows everything.
pub fn filter_toggled(state: &mut crate::app::State, filter: TagFilter) {
    if !state.sidebar.filters.remove(&filter) {
        state.sidebar.filters.insert(filter);
    }
}

/// Every tag filter was cleared (feature 008, FR-024).
pub fn filters_cleared(state: &mut crate::app::State) {
    state.sidebar.filters.clear();
}

/// The scroll viewport reported its laid-out height (feature 024).
pub fn viewport_resized(state: &mut crate::app::State, height: u32) {
    state.sidebar.viewport_height = height;
}

/// The sidebar was scrolled (feature 024, FR-025a).
///
/// The scroll is *also* the dismissal trigger, and the rendering stack gives a scrollable one
/// message per event — so this does both rather than the view emitting two. Same rule, one call,
/// no second copy of it.
pub fn scrolled(state: &mut crate::app::State, offset: u32) {
    crate::reveal_trace::line(format_args!("the scrollable reports offset {offset}"));
    state.sidebar.scroll_offset = offset;
    state.dismiss_on_scroll_beneath();
}

/// The more menu was toggled (feature 1125, FR-015).
///
/// Mutually exclusive with the other two panel popovers, and it closes the project row menu too:
/// it reports that the menu opened and the registry closes what this surface declares it
/// displaces (T067a-2). Closing it forgets the corner, which belongs to the press that opened it.
#[must_use = "what an opening popover displaces is the registry's business, not the caller's"]
pub fn more_menu_toggled(state: &mut crate::app::State) -> Vec<crate::features::Outcome> {
    state.sidebar.more_menu_open = !state.sidebar.more_menu_open;
    if !state.sidebar.more_menu_open {
        state.sidebar.more_anchor = None;
    }
    crate::features::surface_opened(state.sidebar.more_menu_open, SidebarMoreMenu::ID)
}

/// An item of the more menu was chosen: the menu closes. What the item does is the header
/// button's own message, which the shell sends next (`ui::sidebar::more_action_message`), so the
/// refresh still goes through the shell's request path.
pub fn more_menu_chosen(state: &mut crate::app::State) {
    state.sidebar.more_menu_open = false;
    state.sidebar.more_anchor = None;
}

/// The sidebar was collapsed or revealed.
pub fn toggled(state: &mut crate::app::State) {
    state.sidebar.hidden = !state.sidebar.hidden;
}

/// The sidebar's drag handle moved (feature 007).
pub fn drag_moved(state: &mut crate::app::State, x: u16) {
    state.sidebar.width = x.clamp(crate::app::SIDEBAR_MIN_WIDTH, crate::app::SIDEBAR_MAX_WIDTH);
}

/// The worktree list was replaced; drop expansion for rows that no longer exist (T066).
///
/// The sidebar's answer to `Outcome::WorktreesReplaced`. Pruning this used to happen inside
/// `State::set_worktrees`, which is how the worktree feature came to write sidebar data — the
/// first entry converted out of `tests/feature_write_isolation.rs`'s allowlist.
pub fn worktrees_replaced(
    state: &mut crate::app::State,
    names: &std::collections::BTreeSet<String>,
) {
    state.sidebar.expanded.retain(|dir| names.contains(dir));
}

/// Everything the user can do to the sidebar (feature 028, FR-001).
///
/// # The variants kept their meaning and lost their prefix
///
/// The six that began with `Sidebar` do not any more — the type says which surface (contract M1),
/// so `SidebarFiltersCleared` is `Msg::FiltersCleared`. The other three keep their names:
/// `WorktreeExpansionToggled` and `DefaultExpansionToggled` say *which row* expanded, and
/// dropping the word would leave two messages with the same name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    WorktreeExpansionToggled(String),
    DefaultExpansionToggled,
    FilterToggled(TagFilter),
    FiltersCleared,
    /// The chip bar's more (⋯) action was pressed (feature 1125, FR-015).
    MoreMenuToggled,
    /// The more action's press reported its trigger's bottom-left corner, ahead of the toggle.
    MoreMenuAnchored((u16, u16)),
    /// An item of the more menu was chosen: the menu closes, and the shell sends the item's
    /// header-button message (`ui::sidebar::more_action_message`).
    MoreMenuChosen(MoreAction),
    Scrolled(u32),
    ViewportResized(u32),
    Toggled,
    DragMoved(u16),
    /// **Review changes** on a worktree row's or the Default row's menu (feature 482, V1).
    ReviewChangesRequested(SessionLocation),
    /// **Run in parallel**, on the chip bar's more menu since feature 1125 (feature 483, D).
    RunInParallelRequested,
    /// The row a navigation chord focused lost the keyboard (feature 803, contracts/ui.md U4).
    KeyboardRowLeft,
    /// Fold or expand the children of an origin (feature 551, FR-011). The shell persists the
    /// set after it (FR-017).
    ToggleOrigin(SessionId),
    /// A group header was pressed (feature 1125, FR-005, US5.2): the group's open state flips for
    /// the active project, against its effective state (research R4).
    GroupToggled(StateGroup),
}

/// **Review changes** was chosen for `entry` (feature 482, V1): the root opens the Changes view.
pub fn review_changes(entry: SessionLocation) -> Vec<crate::features::Outcome> {
    vec![crate::features::Outcome::ChangesRequested(entry)]
}

/// This feature's whole reducer surface: one entry point, shape A (contract M2).
///
/// Two of the ten report something back — expanding a row can reveal the current session, and
/// opening the more menu displaces the other popovers — so this returns outcomes rather than
/// swallowing them. The other eight write fields this module owns and have nothing to say.
pub fn update(state: &mut crate::app::State, msg: Msg) -> Vec<crate::features::Outcome> {
    match msg {
        Msg::WorktreeExpansionToggled(dir) => {
            return state.toggle_location(SessionLocation::Worktree(dir));
        }
        Msg::DefaultExpansionToggled => return state.toggle_location(SessionLocation::Default),
        Msg::MoreMenuToggled => return more_menu_toggled(state),
        Msg::MoreMenuAnchored(anchor) => state.sidebar.more_anchor = Some(anchor),
        Msg::MoreMenuChosen(_) => more_menu_chosen(state),
        Msg::FilterToggled(filter) => filter_toggled(state, filter),
        Msg::FiltersCleared => filters_cleared(state),
        Msg::Scrolled(offset) => scrolled(state, offset),
        Msg::ViewportResized(height) => viewport_resized(state, height),
        Msg::Toggled => toggled(state),
        Msg::DragMoved(x) => drag_moved(state, x),
        Msg::ReviewChangesRequested(entry) => return review_changes(entry),
        Msg::RunInParallelRequested => return run_in_parallel(),
        Msg::KeyboardRowLeft => state.keyboard_row_left(),
        Msg::ToggleOrigin(id) => {
            if !state.sidebar.folded.remove(&id) {
                state.sidebar.folded.insert(id);
            }
        }
        Msg::GroupToggled(group) => group_toggled(state, group),
    }
    Vec::new()
}

/// A group header was pressed (feature 1125, research R4): store the negation of the group's
/// *effective* open state for the active project, so a Done group open only because it holds the
/// current session closes on the first press.
pub fn group_toggled(state: &mut crate::app::State, group: StateGroup) {
    let Some(project) = state.workspace.active.clone() else {
        return;
    };
    // Only a shown group has a header to press.
    let Some(open) = state
        .sidebar_groups()
        .iter()
        .find(|g| g.group == group)
        .map(|g| g.open)
    else {
        return;
    };
    state
        .sidebar
        .group_choices
        .entry(project)
        .or_default()
        .insert(group, !open);
}
