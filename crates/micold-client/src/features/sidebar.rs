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

use crate::app::State;
use crate::features::worktree::worktree_tags;
use crate::overlay::registry::Registered;
use crate::overlay::{DismissalRules, FloatingSurface, SurfaceId};
use micold_core::naming::{ConventionalType, Tag};
use micold_core::overlay::Layer;
use micold_core::session::{Session, SessionId, SessionLocation};
use micold_core::tokens::{density, spacing};
use micold_core::worktree::Worktree;
use std::collections::BTreeSet;
use std::path::Path;

/// One row in the sidebar's location list (feature 010): either a worktree or the single
/// "Default" project-root entry. A closed enum (Principle V) so a row can never be ambiguously
/// styled as a worktree when it isn't one (FR-006).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidebarEntry {
    /// A discovered worktree row.
    Worktree(WorktreeNode),
    /// The single project-root row (constitution v1.3.0, Principle III exception).
    Default(DefaultNode),
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

/// The rendered height of every sidebar row, top to bottom (feature 024, research R6).
///
/// A location row is Material's two-line list item when it carries tags and its one-line item
/// otherwise; a session row is always one line. Both figures come from the same `density::height`
/// the renderer uses, at [`SIDEBAR_DENSITY`].
///
/// Only *open* locations contribute session rows, because only those are drawn.
pub fn row_heights(entries: &[SidebarEntry]) -> Vec<f32> {
    let one_line = density::height(density::LIST_ROW_BASE, SIDEBAR_DENSITY);
    let two_line = density::height(density::LIST_ROW_TWO_LINE_BASE, SIDEBAR_DENSITY);
    let mut heights = Vec::new();
    for entry in entries {
        match entry {
            SidebarEntry::Default(node) => {
                heights.push(one_line);
                if node.expanded {
                    heights.extend(std::iter::repeat_n(one_line, node.sessions.len()));
                }
            }
            SidebarEntry::Worktree(node) => {
                heights.push(if node.tags.is_empty() {
                    one_line
                } else {
                    two_line
                });
                if node.expanded {
                    heights.extend(std::iter::repeat_n(one_line, node.sessions.len()));
                }
            }
        }
    }
    heights
}

/// Which rendered row holds the current session, if any (feature 024).
///
/// Counted the way the list is built — locations, and the sessions of the open ones — because the
/// answer is a position in what is on screen, not a position in the model.
pub fn current_session_row(entries: &[SidebarEntry], current: Option<SessionId>) -> Option<usize> {
    let current = current?;
    let mut row = 0usize;
    for entry in entries {
        let (expanded, sessions) = match entry {
            SidebarEntry::Default(node) => (node.expanded, &node.sessions),
            SidebarEntry::Worktree(node) => (node.expanded, &node.sessions),
        };
        row += 1;
        if expanded {
            for session in sessions {
                if session.id == current {
                    return Some(row);
                }
                row += 1;
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

/// A worktree's location, expressed relative to the project root, for its sidebar tooltip
/// (feature 010, FR-010, research.md R6). Every worktree lives directly under
/// `<project_root>/.claude/worktrees/`, so a plain `strip_prefix` suffices — no
/// general-purpose relative-path algorithm is needed. Falls back to the absolute path in the
/// unreachable case where a worktree's path is not actually under the project root.
pub fn worktree_location_label(project_root: &Path, worktree: &Worktree) -> String {
    worktree
        .path
        .strip_prefix(project_root)
        .map(|rel| rel.display().to_string())
        .unwrap_or_else(|_| worktree.path.display().to_string())
}

impl State {
    /// Where the current session lives, if the panel can point at it (feature 024, contract §1.2).
    ///
    /// `None` in three cases, all of which reduce [`Self::location_open`] to the user's own
    /// expansion and open nothing: there is no current session; its record is gone from the active
    /// project; or its location is not among the project's known ones — a worktree removed or
    /// missing while the project was inactive (FR-013). The third is why this resolves against
    /// `self.worktrees` rather than trusting the session's own `location`.
    pub fn current_session_location(&self) -> Option<SessionLocation> {
        let id = self.active_session?;
        let location = self
            .active_sessions()
            .iter()
            .find(|s| s.id == id)?
            .location
            .clone();
        match &location {
            SessionLocation::Default => Some(location),
            SessionLocation::Worktree(dir) => self
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
        self.active_session.is_some() && self.reveal_suppressed_for == self.active_session
    }

    /// Whether a location's row is shown open (feature 024, contract §1.1).
    ///
    /// The single answer to "is this row open", derived on every call. `expanded` and
    /// `default_expanded` keep their meaning and lose their monopoly: they are now strictly the
    /// *user's* open set, and this is the union of that with the app's one revealed row.
    pub fn location_open(&self, location: &SessionLocation) -> bool {
        let user_open = match location {
            SessionLocation::Worktree(dir) => self.expanded.contains(dir),
            SessionLocation::Default => self.default_expanded,
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
        let entries = self.sidebar_entries();
        let index = current_session_row(&entries, self.active_session)?;
        scroll_target(
            &row_heights(&entries),
            index,
            self.sidebar_viewport_height as f32,
            self.sidebar_scroll_offset as f32,
        )
        .map(|offset| offset.round().max(0.0) as u32)
    }

    /// Whether the projection currently holds a row for the current session (feature 024).
    ///
    /// The condition for draining an armed reveal: until it is true there is nothing to scroll to,
    /// and the arm stays set.
    pub fn current_session_is_listed(&self) -> bool {
        current_session_row(&self.sidebar_entries(), self.active_session).is_some()
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
    pub fn toggle_location(&mut self, location: SessionLocation) {
        let open = self.location_open(&location);
        let holds_current = self.current_session_location().as_ref() == Some(&location);
        match &location {
            SessionLocation::Worktree(dir) => {
                if open {
                    self.expanded.remove(dir);
                } else {
                    self.expanded.insert(dir.clone());
                }
            }
            SessionLocation::Default => self.default_expanded = !open,
        }
        if holds_current {
            self.reveal_suppressed_for = if open { self.active_session } else { None };
        }
    }

    /// Build the sidebar tree: worktrees (top level) each joined with their sessions and
    /// expansion state (FR-002, FR-003). Sessions are matched to worktrees by `dir_name`.
    /// Sourced from [`State::visible_worktrees`], so agent-owned worktrees produce no row while
    /// hidden (feature 014, FR-002).
    pub fn worktree_tree(&self) -> Vec<WorktreeNode> {
        let sessions = self.active_sessions();
        self.visible_worktrees()
            .map(|worktree| WorktreeNode {
                display_name: self.worktree_display_name(&worktree.dir_name),
                tags: worktree_tags(worktree),
                expanded: self.location_open(&SessionLocation::Worktree(worktree.dir_name.clone())),
                sessions: sessions
                    .iter()
                    .filter(|s| s.location.is_worktree(&worktree.dir_name) && !s.archived)
                    .cloned()
                    .collect(),
                worktree: worktree.clone(),
                shown_for_current_session: false,
            })
            .collect()
    }

    /// The worktree tree narrowed to the active tag filters (feature 008, FR-025). With no
    /// filter active this equals [`State::worktree_tree`]. Used by the sidebar to render only
    /// matching worktrees; a subsequent add/rename/delete re-runs this so the list stays
    /// consistent (FR-028).
    /// (Feature 024) The location holding the current session survives the filters, and says so.
    ///
    /// Two mechanisms hide a row and the exemption has to reach past both: the tag filters here,
    /// and the hidden-agent-worktree setting, which excludes rows *earlier* — in
    /// [`State::visible_worktrees`], before [`State::worktree_tree`] ever sees them. So the
    /// re-admitted row is built from `self.worktrees` rather than from the tree above.
    ///
    /// Exactly one row can be re-admitted, because there is one current session. That is what keeps
    /// this an exemption rather than a filter bypass (FR-012), and it is why the row carries
    /// `shown_for_current_session`: a row that survived a filter it does not match is otherwise
    /// unexplained, and the user is the one who set that filter.
    pub fn filtered_worktree_tree(&self) -> Vec<WorktreeNode> {
        let mut tree: Vec<WorktreeNode> = self
            .worktree_tree()
            .into_iter()
            .filter(|node| matches_filters(&node.tags, &self.sidebar_filters))
            .collect();

        let Some(SessionLocation::Worktree(dir)) = self.current_session_location() else {
            return tree;
        };
        if tree.iter().any(|node| node.worktree.dir_name == dir) {
            // Already listed on its own merits, so it needs no exemption and must not claim one.
            return tree;
        }
        let Some(worktree) = self.worktrees.iter().find(|w| w.dir_name == dir) else {
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
            worktree: worktree.clone(),
            shown_for_current_session: true,
        };
        // Inserted where it would have sat unfiltered rather than appended: the exemption changes
        // which rows are listed, never their order (FR-012a). `worktrees` is the order the panel
        // draws, so its position in that list is the answer.
        let at = self
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
    /// open, regardless of `sidebar_filters`.
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
        })];
        entries.extend(
            self.filtered_worktree_tree()
                .into_iter()
                .map(SidebarEntry::Worktree),
        );
        entries
    }

    /// The distinct tag filters offered for the current worktrees (feature 008, FR-024): a
    /// `Type` per conventional type present, `HasIssue` if any worktree embeds an issue key,
    /// and `Untyped` if any worktree lacks a type. Order: types first, then HasIssue, Untyped.
    ///
    /// Sourced from [`State::visible_worktrees`] (feature 014, FR-003): a hidden agent worktree
    /// must not conjure a chip — its machine name has no conventional type, so it would otherwise
    /// offer an `Untyped` filter matching nothing the user can see (research R7).
    pub fn available_tag_filters(&self) -> Vec<TagFilter> {
        let mut types = BTreeSet::new();
        let mut has_issue = false;
        let mut has_untyped = false;
        for worktree in self.visible_worktrees() {
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
                    // Feature 014: label only, never a filter (research R5). Note what the empty
                    // arm implies: carrying no `Type`, a REVEALED agent worktree still counts as
                    // untyped and so can be matched by an `Untyped` chip — correct, and required
                    // by FR-010d (filters apply to revealed rows exactly as to user-created ones).
                    Tag::Agent => {}
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

/// The sidebar's tag-filter panel, as a floating surface (feature 021, T029).
///
/// The panel is a popover with no state of its own beyond "is it showing": the filters it edits
/// live in `State::sidebar_filters` and outlive it, which is contract D3 — closing the panel is
/// not a decision about the filters. So the surface is a marker, and everything dispatch needs is
/// the three answers below.
///
/// The one popover Escape reaches today. Registering it is a faithful description of what the
/// code already does, which is why it is the popover Tier 2 registers first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SidebarFilterPanel;

impl FloatingSurface for SidebarFilterPanel {
    fn id(&self) -> SurfaceId {
        SurfaceId::new("sidebar_filter")
    }

    fn layer(&self) -> Layer {
        Layer::Popover
    }

    fn dismissal(&self) -> DismissalRules {
        DismissalRules::for_layer(Layer::Popover)
            .cancelled_by(crate::app::Message::SidebarFilterMenuToggled)
    }
}

impl Registered for SidebarFilterPanel {
    fn open_in(state: &State) -> Option<Self> {
        state.sidebar_filter_open.then_some(SidebarFilterPanel)
    }
}

/// A tag filter was toggled on or off (feature 008, FR-024).
///
/// Filters combine with OR (FR-025); an empty set shows everything.
pub fn filter_toggled(state: &mut State, filter: TagFilter) {
    if !state.sidebar_filters.remove(&filter) {
        state.sidebar_filters.insert(filter);
    }
}

/// Every tag filter was cleared (feature 008, FR-024).
pub fn filters_cleared(state: &mut State) {
    state.sidebar_filters.clear();
}

/// The scroll viewport reported its laid-out height (feature 024).
pub fn viewport_resized(state: &mut State, height: u32) {
    state.sidebar_viewport_height = height;
}

/// The sidebar was scrolled (feature 024, FR-025a).
///
/// The scroll is *also* the dismissal trigger, and the rendering stack gives a scrollable one
/// message per event — so this does both rather than the view emitting two. Same rule, one call,
/// no second copy of it.
pub fn scrolled(state: &mut State, offset: u32) {
    state.sidebar_scroll_offset = offset;
    state.dismiss_on_scroll_beneath();
}

/// The tag-filter panel was toggled (feature 009, FR-002/FR-003).
///
/// Mutually exclusive with the other two lightweight popovers and with the project context menu,
/// which is why this writes three fields it does not own — catalogued in
/// `tests/feature_write_isolation.rs`, and the same shared toolbar rule `help::menu_toggled`
/// carries from the other side.
pub fn filter_menu_toggled(state: &mut State) {
    state.sidebar_filter_open = !state.sidebar_filter_open;
    state.help_menu_open = false;
    state.project_switcher_open = false;
    state.project_menu_open = None;
}

/// Agent-owned worktrees were shown or hidden (feature 014, FR-010).
///
/// Sole mutation (FR-010d): tag filters, expansion state and overlays are left exactly as they
/// were, and nothing is re-discovered — this is a pure view recomputation, so no git call and no
/// `Task` (FR-008).
pub fn show_agent_worktrees_toggled(state: &mut State) {
    state.show_agent_worktrees = !state.show_agent_worktrees;
}

/// The sidebar was collapsed or revealed.
pub fn toggled(state: &mut State) {
    state.sidebar_hidden = !state.sidebar_hidden;
}

/// The sidebar's drag handle moved (feature 007).
pub fn drag_moved(state: &mut State, x: u16) {
    state.sidebar_width = x.clamp(crate::app::SIDEBAR_MIN_WIDTH, crate::app::SIDEBAR_MAX_WIDTH);
}

/// The worktree list was replaced; drop expansion for rows that no longer exist (T066).
///
/// The sidebar's answer to `Outcome::WorktreesReplaced`. Pruning this used to happen inside
/// `State::set_worktrees`, which is how the worktree feature came to write sidebar data — the
/// first entry converted out of `tests/feature_write_isolation.rs`'s allowlist.
pub fn worktrees_replaced(state: &mut State, names: &std::collections::BTreeSet<String>) {
    state.expanded.retain(|dir| names.contains(dir));
}
