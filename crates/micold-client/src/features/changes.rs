//! The Changes view of one entry: which entry is open, its toggles, the changed-file list and the
//! selected file (feature 482, contracts/changes-view.md V1–V3 and L1–L3, data-model § Client).
//!
//! Render-free. The shell half (`shell/changes.rs`) turns [`Effect::ReadList`] and
//! [`Effect::ReadDiff`] into git reads off the update thread and answers [`Msg::ListRead`] and
//! [`Msg::DiffRead`]. What the list pane says when it has no rows
//! is chosen here ([`list_body`], [`base_line`]) so the glue in `ui/changes.rs` only renders it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use micold_core::protocol::messages::ReviewEditOp;
use micold_core::pull_request::{
    BranchPrState, BranchPullRequest, CheckStatus, DiffReadFailure, ReadingFailure,
};
use micold_core::review::anchor::{self, locate, Located};
use micold_core::review::base::{Base, BaseUnavailable, ReviewScope, Toggles};
use micold_core::review::changes::{ChangeKind, ChangeList, ChangedFile};
use micold_core::review::comment::{CommentId, CommentState, Move, Outcome, ReviewComment, Status};
use micold_core::review::diff::{
    parse_unified, side_by_side_rows, unified_rows, FileDiff, LoadedDiff, SideRow, Spans,
    UnifiedRow,
};
use micold_core::review::pr_diff::PrDiff;
use micold_core::review::reply::{self, CARD_CHARS};
use micold_core::review::{LineRange, RelPath, Side};
use micold_core::session::SessionLocation;
use micold_core::settings::DiffLayout;
use micold_core::tokens::Rgb;

/// The note beside the Default entry's unavailable Committed toggle (L1, US1 s9).
pub const DEFAULT_ENTRY_NOTE: &str =
    "The project root has no branch of its own to compare; only uncommitted changes are listed";

/// The empty state when every toggle that can list something is off (Edge "Both toggles off").
pub const BOTH_HIDDEN: &str = "Committed and uncommitted changes are both hidden";

/// A value read off the update thread (the pr_status `Phase` pattern).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Load<T> {
    /// Nothing asked yet.
    #[default]
    Idle,
    /// Read `seq` is under way; `again`: one more read follows it. `last` is what was shown before
    /// it started, kept on screen meanwhile.
    Loading {
        /// The read's sequence number; an answer with another is dropped.
        seq: u64,
        /// A read was asked while this one ran: it runs once more when this one ends.
        again: bool,
        /// The value shown before this read started.
        last: Option<T>,
    },
    /// The last read's value.
    Ready(T),
    /// The last read failed, with git's message.
    Failed(String),
}

/// The pull requests of the open view's branch (feature 775, data-model §2). Held per view, so
/// every session of one worktree shares one list and one read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum PrList {
    /// No section: the setting is off, the entry is the project root, or there is no GitHub
    /// access.
    #[default]
    Absent,
    /// A read is under way; `again`: one more follows it. `last` stays on screen meanwhile.
    Loading {
        /// The read's sequence number; an answer with another is dropped.
        seq: u64,
        /// A read was asked while this one ran: it runs once more when this one ends.
        again: bool,
        /// The list and when it was read, kept on screen.
        last: Option<(Vec<BranchPullRequest>, u64)>,
    },
    /// The last read's list, newest first, read at `read_at` (Unix seconds).
    Ready {
        /// The pull requests.
        prs: Vec<BranchPullRequest>,
        /// When the read started.
        read_at: u64,
    },
    /// The last read failed for a reason that may pass: what was listed stays (FR-008).
    Unavailable {
        /// The last list and when it was read.
        last: Option<(Vec<BranchPullRequest>, u64)>,
    },
}

impl PrList {
    /// The pull requests to show, newest first, with when they were read; `None` for no section.
    pub fn shown(&self) -> Option<(&[BranchPullRequest], u64)> {
        match self {
            PrList::Absent => None,
            PrList::Loading { last, .. } | PrList::Unavailable { last } => {
                last.as_ref().map(|(prs, at)| (prs.as_slice(), *at))
            }
            PrList::Ready { prs, read_at } => Some((prs.as_slice(), *read_at)),
        }
    }

    /// Whether the list shown is older than the app's PR status stale threshold at `now`: a
    /// refresh that failed (or was held back) leaves the last list, which is marked then
    /// (feature 775, FR-012). A fresh read replaces the mark.
    pub fn possibly_stale(&self, now: u64) -> bool {
        self.shown()
            .is_some_and(|(_, read_at)| micold_core::pull_request::is_stale(read_at, now))
    }
}

/// What the view shows the diff of (feature 775, data-model §2): the entry's own changes, or one
/// pull request's. A selected pull request is an enum arm, so it cannot hold a working-tree list.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Source {
    /// The working-tree/branch changes, as before the pull request list existed (FR-006).
    #[default]
    Changes,
    /// A listed pull request's base...head diff. The working tree's list, selection and diff are
    /// kept meanwhile, not read.
    PullRequest(Box<PullRequestView>),
}

/// The pull request shown in place of the entry's changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestView {
    /// Its number.
    pub number: u64,
    /// Its diff as read.
    pub diff: PrDiffLoad,
    /// The selected file.
    pub selected: Option<RelPath>,
    /// The selected file's diff, parsed from its patch: no read per file.
    pub file: Option<LoadedDiff>,
    /// Files over the size limits the user asked to see.
    pub shown_large: BTreeSet<RelPath>,
}

/// A pull request's diff, read once on selection (FR-004).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrDiffLoad {
    /// Read `seq` is under way.
    Loading {
        /// The read's sequence number; an answer with another is dropped.
        seq: u64,
    },
    /// Its files, and each one's patch.
    Ready {
        /// The rows, in GitHub's order.
        files: Vec<ChangedFile>,
        /// Each file's section of the diff.
        patches: BTreeMap<RelPath, Vec<u8>>,
    },
    /// GitHub will not diff it: above 300 files or 20 000 changed lines.
    TooLarge,
    /// The read failed; selecting the row again retries.
    Failed,
}

/// The one mapping from a listed pull request's state to the mark the sidebar uses (FR-003).
pub fn pr_mark(state: BranchPrState) -> crate::ui::material::PrMark {
    use crate::ui::material::PrMark;
    match state {
        BranchPrState::Open => PrMark::Open,
        BranchPrState::Draft => PrMark::Draft,
        BranchPrState::Merged => PrMark::Merged,
        BranchPrState::Closed => PrMark::Closed,
    }
}

/// The one mapping from a check status to the mark the sidebar and the list share (feature 1003,
/// FR-003); `None` is no check at all, so no mark.
pub fn check_mark(status: CheckStatus) -> Option<crate::ui::material::CheckMark> {
    use crate::ui::material::CheckMark;
    match status {
        CheckStatus::None => None,
        CheckStatus::Pending => Some(CheckMark::Pending),
        CheckStatus::Passing => Some(CheckMark::Passing),
        CheckStatus::Failing => Some(CheckMark::Failing),
    }
}

/// Which way previous/next moves through the pull request list (feature 1003, FR-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// The entry before the shown one, in list order.
    Previous,
    /// The entry after the shown one, in list order.
    Next,
}

/// The number of the pull request next to `number` in `prs` in the direction of `step`; `None` at
/// either end of the list, and for a number that is not listed (FR-006).
pub fn adjacent(prs: &[BranchPullRequest], number: u64, step: Step) -> Option<u64> {
    let at = prs.iter().position(|pr| pr.number == number)?;
    let to = match step {
        Step::Previous => at.checked_sub(1)?,
        Step::Next => at + 1,
    };
    prs.get(to).map(|pr| pr.number)
}

/// What the diff area's header says of the shown pull request (feature 1003, FR-007).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrHeader {
    /// Its number.
    pub number: u64,
    /// Its title.
    pub title: String,
    /// Its state's mark.
    pub mark: crate::ui::material::PrMark,
    /// Its checks' mark; `None` when it has no check.
    pub check: Option<crate::ui::material::CheckMark>,
    /// The base line: `Pull request #N: <base> ... <head branch>`.
    pub base: String,
}

/// The header of the shown pull request, from the list's current entry so a refresh updates it;
/// `None` for the working tree, or while no list holds the shown number.
pub fn pr_header(view: &OpenView) -> Option<PrHeader> {
    let Source::PullRequest(shown) = &view.source else {
        return None;
    };
    let (prs, _) = view.pull_requests.shown()?;
    let listed = prs.iter().find(|pr| pr.number == shown.number)?;
    Some(PrHeader {
        number: listed.number,
        title: listed.title.clone(),
        mark: pr_mark(listed.state),
        check: check_mark(listed.checks),
        base: format!(
            "Pull request #{}: {} ... {}",
            listed.number, listed.base, listed.head_branch
        ),
    })
}

/// The open view (data-model § Client).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenView {
    /// The project of the entry: the window's active project when the view opened.
    pub project: PathBuf,
    /// The entry shown.
    pub entry: SessionLocation,
    /// Which kinds of change are listed; both on at open (FR-004).
    pub toggles: Toggles,
    /// The changed files.
    pub list: Load<ChangeList>,
    /// The selected file, kept across a re-read while it is still listed.
    pub selected: Option<RelPath>,
    /// The file list's scroll offset, in pixels (L4: only the visible rows are built).
    pub list_offset: u32,
    /// The file list's viewport height, in pixels.
    pub list_viewport: u32,
    /// The selected file's diff (D1); `Idle` with nothing selected.
    pub diff: Load<LoadedDiff>,
    /// Files over the size limits the user asked to see (D4); kept while the view is open.
    pub shown_large: BTreeSet<RelPath>,
    /// The diff's scroll offset, in pixels (D5).
    pub diff_offset: u32,
    /// The diff's viewport height, in pixels.
    pub diff_viewport: u32,
    /// The gutter selection (C1).
    pub pick: Option<Pick>,
    /// The comment being written or edited (C2); survives a refresh (C4).
    pub composer: Option<Composer>,
    /// The diff's slots (comment cards, the composer) as last measured, in whole pixels, keyed by
    /// the line they hang under; for the shown file in the layout in force.
    pub slot_heights: BTreeMap<(Side, u32), u32>,
    /// Where each comment of the entry is drawn and whether it shows the outdated mark (R14,
    /// feature 946 R2): kept until its file is read again.
    pub anchors: BTreeMap<CommentId, Anchor>,
    /// **Discard drafts…** was pressed: its confirmation is open (S3).
    pub confirm_discard: bool,
    /// The comment whose whole reply is open in its dialog (feature 946, FR-006); cleared when
    /// that comment leaves the entry.
    pub reply_open: Option<CommentId>,
    /// The review panel right of the diff (feature 946, FR-016 to FR-019).
    pub panel: Panel,
    /// The entry's branch, as the shell last told it; `None` for the Default entry and a
    /// detached HEAD (feature 775).
    pub branch: Option<String>,
    /// The pull requests of `branch` (feature 775).
    pub pull_requests: PrList,
    /// What the diff shows: the entry's changes, or a pull request's (feature 775).
    pub source: Source,
}

/// The review panel's state (feature 946, FR-016 to FR-019).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Panel {
    /// The panel is shown; open by default, closed by the toolbar's "Review (n)".
    pub open: bool,
    /// The comment a row jump went to: its card is drawn highlighted (FR-017).
    pub focus: Option<CommentId>,
    /// The row list's scroll offset, in pixels.
    pub offset: u32,
    /// The row list's viewport height, in pixels.
    pub viewport: u32,
    /// Why the last row press did not jump; cleared by the next press.
    pub hidden: Option<Why>,
    /// A jump waits for its file's diff to arrive, then scrolls to the focused comment.
    pub to_show: bool,
}

impl Default for Panel {
    fn default() -> Self {
        Self {
            open: true,
            focus: None,
            offset: 0,
            viewport: 0,
            hidden: None,
            to_show: false,
        }
    }
}

/// Where pressing a panel row goes (FR-017).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Jump {
    /// To `line` of `path`, which the diff lists.
    Shown {
        /// The file.
        path: RelPath,
        /// The comment's first line.
        line: u32,
    },
    /// Nowhere: the comment's file is not in the diff, and why.
    Hidden(Why),
}

/// Why a jump is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// The file is of a kind the toolbar's toggles hide.
    KindToggledOff,
    /// The file is not among the changes at all.
    NotListed,
}

impl Why {
    /// What the panel says.
    pub fn message(self) -> &'static str {
        match self {
            Self::KindToggledOff => {
                "That comment's file is hidden by the Committed or Uncommitted toggle. Turn the toggle back on to see it."
            }
            Self::NotListed => "That comment's file is not in the current diff.",
        }
    }
}

/// One row of the review panel (FR-016).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelRow {
    /// The comment.
    pub id: CommentId,
    /// Its file.
    pub path: RelPath,
    /// Its side.
    pub side: Side,
    /// "Line 3" or "Lines 3–5"; "Old line…" on the removed side.
    pub lines: String,
    /// The start of its text.
    pub text: String,
    /// Its status.
    pub status: Status,
    /// The start of its reply, made displayable.
    pub reply: Option<String>,
    /// Its file is not in the current diff.
    pub not_in_diff: bool,
    /// It is the comment a jump went to.
    pub selected: bool,
    /// Edit and Delete are offered: a draft no send holds.
    pub editable: bool,
    /// It has a reply longer than the row shows, so **Show all** is offered.
    pub has_full_reply: bool,
}

/// How many characters of a comment's text a panel row shows.
pub const PANEL_TEXT_CHARS: usize = 200;

/// The review panel's rows: by file path (bytewise), first line, side (current first), created.
pub fn panel_rows(state: &State) -> Vec<PanelRow> {
    let Some(view) = state.open.as_ref() else {
        return Vec::new();
    };
    let Some(review) = review_of(state, view) else {
        return Vec::new();
    };
    let listed = shown_list(view);
    let mut comments: Vec<&ReviewComment> = review.comments.iter().collect();
    comments.sort_by(|a, b| {
        (
            a.path.as_str().as_bytes(),
            shown_range(view, a).start(),
            a.side,
            a.created,
        )
            .cmp(&(
                b.path.as_str().as_bytes(),
                shown_range(view, b).start(),
                b.side,
                b.created,
            ))
    });
    comments
        .into_iter()
        .map(|c| {
            let card = card_model(c, review.sending);
            let shown = shown_range(view, c);
            let (start, end) = (shown.start(), shown.end());
            let noun = if start == end { "line" } else { "lines" };
            let span = if start == end {
                start.to_string()
            } else {
                format!("{start}–{end}")
            };
            let lines = match (c.side, start == end) {
                (Side::New, true) => format!("Line {span}"),
                (Side::New, false) => format!("Lines {span}"),
                (Side::Old, _) => format!("Old {noun} {span}"),
            };
            PanelRow {
                id: c.id,
                path: c.path.clone(),
                side: c.side,
                lines,
                text: reply::shortened(&c.text, PANEL_TEXT_CHARS).0,
                status: card.status,
                reply: card.reply.map(|(text, _)| text),
                not_in_diff: listed.is_some_and(|l| l.files.iter().all(|f| f.path != c.path)),
                selected: view.panel.focus == Some(c.id),
                editable: card.editable,
                has_full_reply: full_reply(c).is_some(),
            }
        })
        .collect()
}

/// How many comments of the open entry have each status, in the order Draft, Sent, Applied,
/// Answered; every status is listed, with zero where none has it.
pub fn status_counts(state: &State) -> [(Status, usize); 4] {
    let mut counts = [
        (Status::Draft, 0),
        (Status::Sent, 0),
        (Status::Applied, 0),
        (Status::Answered, 0),
    ];
    let review = state.open.as_ref().and_then(|view| review_of(state, view));
    for comment in review.iter().flat_map(|r| &r.comments) {
        let status = comment.state.status();
        if let Some(slot) = counts.iter_mut().find(|(s, _)| *s == status) {
            slot.1 += 1;
        }
    }
    counts
}

/// What the panel says with no comment (US3 s5).
pub const PANEL_EMPTY_TEXT: &str =
    "No review comments yet. Click a line number in the diff to write one.";

/// Where a comment is drawn and whether it shows the outdated mark (feature 946, data-model §
/// Client).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Anchor {
    /// The lines the card hangs under: where its code is now, or its last place within the file.
    pub shown: LineRange,
    /// Its quoted code is gone or ambiguous, and it is a Draft or Sent comment (FR-021, FR-024).
    pub outdated: bool,
}

/// Where `comment` is drawn: its anchor, or its stored range before its file was read.
pub fn shown_range(view: &OpenView, comment: &ReviewComment) -> LineRange {
    view.anchors
        .get(&comment.id)
        .map_or(comment.range, |anchor| anchor.shown)
}

/// Whether `comment` shows the outdated mark.
pub fn is_outdated(view: &OpenView, comment: &ReviewComment) -> bool {
    view.anchors
        .get(&comment.id)
        .is_some_and(|anchor| anchor.outdated)
}

/// The toolbar toggle's label: "Review (n)", n being every comment of the entry.
pub fn review_toolbar_label(state: &State) -> String {
    let count = state
        .open
        .as_ref()
        .and_then(|view| review_of(state, view))
        .map_or(0, |r| r.comments.len());
    format!("Review ({count})")
}

/// Where pressing the row of comment `id` goes; `None` for an unknown comment.
pub fn jump_target(state: &State, id: CommentId) -> Option<Jump> {
    let view = state.open.as_ref()?;
    let comment = review_of(state, view)?
        .comments
        .iter()
        .find(|c| c.id == id)?;
    let listed = view.source == Source::Changes
        && shown_list(view).is_some_and(|l| l.files.iter().any(|f| f.path == comment.path));
    Some(if listed {
        Jump::Shown {
            path: comment.path.clone(),
            line: shown_range(view, comment).start(),
        }
    } else if view.source == Source::Changes
        && (!view.toggles.committed || !view.toggles.uncommitted)
    {
        Jump::Hidden(Why::KindToggledOff)
    } else {
        Jump::Hidden(Why::NotListed)
    })
}

/// The diff row index of `line` of `side` in the layout in force, for a jump.
fn row_of_line(diff: &LoadedDiff, layout: DiffLayout, side: Side, line: u32) -> Option<usize> {
    match layout {
        DiffLayout::Unified => unified_rows(&diff.diff).iter().position(|row| match row {
            UnifiedRow::Line(l) => match side {
                Side::New => l.new == Some(line),
                Side::Old => l.old == Some(line),
            },
            UnifiedRow::Header(_) => false,
        }),
        DiffLayout::SideBySide => side_by_side_rows(&diff.diff)
            .iter()
            .position(|row| match row {
                SideRow::Pair { left, right } => match side {
                    Side::New => right.is_some_and(|c| c.number == line),
                    Side::Old => left.is_some_and(|c| c.number == line),
                },
                SideRow::Header(_) => false,
            }),
    }
}

/// The scroll offset that brings the focused comment's first line near the top of the diff, and
/// takes the pending jump; `None` when there is nothing to scroll to yet.
fn jump_offset(state: &mut State) -> Option<u32> {
    let layout = state.layout;
    let view = state.open.as_ref()?;
    let focus = view.panel.focus?;
    let comment = review_of(state, view)
        .and_then(|r| r.comments.iter().find(|c| c.id == focus))
        .filter(|c| view.selected.as_ref() == Some(&c.path));
    let Some(comment) = comment else {
        // The comment is gone or another file was chosen: the jump is abandoned.
        if let Some(view) = state.open.as_mut() {
            view.panel.to_show = false;
        }
        return None;
    };
    let Load::Ready(diff) = &view.diff else {
        return None;
    };
    // The cards and composers hung above the line push it down by their measured heights, known
    // once the file has been laid out; a cold jump lands a little short of them.
    let first = shown_range(view, comment).start();
    let hung: u32 = view
        .slot_heights
        .iter()
        .filter(|((_, line), _)| *line < first)
        .map(|(_, height)| *height)
        .sum();
    let row = row_of_line(diff, layout, comment.side, first);
    let offset = row.map_or(0.0, |r| {
        r.saturating_sub(2) as f32 * crate::ui::material::DIFF_ROW_HEIGHT + hung as f32
    });
    if let Some(view) = state.open.as_mut() {
        view.panel.to_show = false;
        view.diff_offset = offset as u32;
    }
    Some(offset as u32)
}

/// A gutter selection: lines `anchor` to `head` (either order) of one side (C1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pick {
    /// The side every picked line is on.
    pub side: Side,
    /// The line the pick started on.
    pub anchor: u32,
    /// The line a shift-click extended it to; the composer opens under it.
    pub head: u32,
}

impl Pick {
    /// The picked lines, in order.
    pub fn range(&self) -> LineRange {
        let (start, end) = (self.anchor.min(self.head), self.anchor.max(self.head));
        LineRange::new(start.max(1), end.max(1)).expect("start <= end, both >= 1")
    }
}

/// What the composer writes (C2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposerTarget {
    /// A new comment on the picked lines.
    New(Pick),
    /// A new text for this pending comment.
    Edit(CommentId),
    /// The note of marking this Sent comment answered by hand (feature 946, FR-006).
    Answer(CommentId),
}

/// The open composer (data-model § Client).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Composer {
    /// What Save does.
    pub target: ComposerTarget,
    /// What the user has written so far.
    pub text: String,
}

/// The last `ReviewChanged` of one entry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReviewView {
    /// Every comment of the entry, pending and sent.
    pub comments: Vec<ReviewComment>,
    /// A send is in progress: editing and deleting are unavailable.
    pub sending: bool,
}

/// What this window holds about the Changes view.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// `None`: the view is closed.
    pub open: Option<OpenView>,
    /// The sequence number the next read gets.
    pub next_seq: u64,
    /// A read the root started while interpreting an outcome, which only the shell can run. Taken
    /// by the shell right after the message that caused it (`State::take_changes_effect`).
    pub pending: Option<Effect>,
    /// The diff layout in force: the service-owned `Settings::diff_layout`, so it outlives a file,
    /// the view and a restart (D1, FR-006, R12).
    pub layout: DiffLayout,
    /// The last `ReviewChanged` per entry, keyed by project and the wire's worktree dir (`""` is
    /// the Default entry), so no entry ever shows another's comments (FR-021).
    pub reviews: BTreeMap<(PathBuf, String), ReviewView>,
    /// The PR status setting in force (`github.pr_status_enabled`): the pull request section is
    /// absent while it is off (feature 775, FR-014).
    pub pr_enabled: bool,
    /// The pull request chosen per entry, kept while another entry's view is open (FR-013). In
    /// memory only: nothing about pull requests is stored (feature 040 FR-032).
    pub remembered: BTreeMap<(PathBuf, SessionLocation), u64>,
}

/// What this feature is told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// Open the view of `entry` (V1); reopening resets the toggles.
    Opened {
        /// The entry's project.
        project: PathBuf,
        /// The entry.
        entry: SessionLocation,
    },
    /// The view's close action (V2).
    Closed,
    /// The Committed toggle was pressed.
    CommittedToggled,
    /// The Uncommitted toggle was pressed.
    UncommittedToggled,
    /// A file row was pressed.
    FileSelected(RelPath),
    /// The file list scrolled or was resized.
    ListScrolled {
        /// Offset from the top, pixels.
        offset: u32,
        /// Viewport height, pixels.
        viewport: u32,
    },
    /// The large gate's Show diff was pressed (D4).
    ShowLarge,
    /// The diff scrolled or was resized.
    DiffScrolled {
        /// Offset from the top, pixels.
        offset: u32,
        /// Viewport height, pixels.
        viewport: u32,
    },
    /// Diff read `seq` ended.
    DiffRead {
        /// The read it answers.
        seq: u64,
        /// The diff, or git's message.
        result: Result<LoadedDiff, String>,
    },
    /// Read `seq` ended.
    ListRead {
        /// The read it answers.
        seq: u64,
        /// The list, or git's message.
        result: Result<ChangeList, String>,
    },
    /// The user selected a session: the terminal takes the main area back (V2).
    SessionSelected,
    /// The project's worktrees are now these `dir_name`s (V3): a view of one that is gone closes.
    WorktreesListed(BTreeSet<String>),
    /// The user chose a layout on the diff pane's toggle (D1).
    LayoutChosen(DiffLayout),
    /// The layout in force, from `Welcome` or `SettingsChanged` (R12).
    LayoutInForce(DiffLayout),
    /// A diff row's gutter was pressed: the row's old and new numbers (C1).
    GutterPressed {
        /// Its number in the old version; `None` for an added line.
        old: Option<u32>,
        /// Its number in the new version; `None` for a removed line.
        new: Option<u32>,
        /// Shift was held: extend the pick on the same side.
        extend: bool,
    },
    /// A slot under line `line` of `side` was laid out `height` pixels tall.
    SlotMeasured {
        /// The side of the line it hangs under.
        side: Side,
        /// The line it hangs under.
        line: u32,
        /// Its height, whole pixels (rounded up).
        height: u32,
    },
    /// **Add comment** was pressed (C2).
    AddComment,
    /// An edit or a cursor move in the composer's text area (C2). The editor's content is the
    /// binary's (runtime, not state), so the shell performs the action before the reducer sees
    /// this and reports the new text as [`Msg::ComposerEdited`]; the reducer ignores it.
    ComposerAction(crate::app::EditorAction),
    /// The composer's text changed.
    ComposerEdited(String),
    /// Save (or Ctrl/Cmd+Enter) in the composer.
    ComposerSaved,
    /// Cancel in the composer.
    ComposerCancelled,
    /// Edit on a pending comment's card.
    EditComment(CommentId),
    /// Delete on a pending comment's card.
    DeleteComment(CommentId),
    /// **Mark applied** on a Sent comment's card (feature 946, FR-006).
    MarkApplied(CommentId),
    /// **Answer…** on a Sent comment's card: open the composer for the note.
    AnswerComment(CommentId),
    /// **Show all** under a shortened reply: open the whole reply.
    ShowReply(CommentId),
    /// The full-reply dialog's Close, or its dismissal.
    ReplyClosed,
    /// An entry's comments, as the service pushed them.
    ReviewChanged {
        /// The project.
        project: PathBuf,
        /// The wire's worktree dir; `""` is the Default entry.
        worktree_dir: String,
        /// Every comment of the entry.
        comments: Vec<ReviewComment>,
        /// A send is in progress.
        sending: bool,
    },
    /// **Send review (n)** was pressed (S1).
    SendPressed,
    /// A file of the open entry changed on disk or in git (R1, R9).
    Changed,
    /// **Clear sent** was pressed (S3).
    ClearSentPressed,
    /// **Discard drafts…** was pressed: open its confirmation (S3).
    DiscardPendingPressed,
    /// The shell asks for the open view's pull requests: right after the view opened, and on the
    /// PR status triggers (feature 775). `branch` is the entry's own branch, if it has one.
    PullRequestsWanted {
        /// The entry's branch.
        branch: Option<String>,
    },
    /// Pull request read `seq` ended; `read_at` is when it started, Unix seconds.
    PullRequestsRead {
        /// The read it answers.
        seq: u64,
        /// When the read started.
        read_at: u64,
        /// The list, or why there is none.
        result: Result<Vec<BranchPullRequest>, ReadingFailure>,
    },
    /// The PR status setting changed, or was learned (feature 775, FR-014).
    PullRequestSettingChanged(bool),
    /// A row of the pull request section was pressed: `Some(number)` for a pull request, `None`
    /// for "Working tree" (FR-004, FR-005).
    SourceChosen(Option<u64>),
    /// Previous or next was pressed: the adjacent listed pull request is chosen as if its row were
    /// pressed; nothing at an end, or while the working tree is shown (feature 1003, FR-006).
    PullRequestStepped(Step),
    /// Diff read `seq` of pull request `number` ended, already split per file.
    PullRequestDiffRead {
        /// The read it answers.
        seq: u64,
        /// The pull request it was asked of.
        number: u64,
        /// The files, or why there are none.
        result: Result<PrDiff, DiffReadFailure>,
    },
    /// **Open on GitHub** was pressed on a pull request's "too large" line; the shell opens the
    /// held address and the reducer changes nothing.
    PullRequestOpenRequested(u64),
    /// The discard confirmation's Discard (S3).
    DiscardConfirmed,
    /// The discard confirmation's Cancel, or its dismissal (S3).
    DiscardCancelled,
    /// The toolbar's "Review (n)" toggle: close or reopen the panel (feature 946, FR-016).
    PanelToggled,
    /// The panel's row list scrolled.
    PanelScrolled {
        /// The offset, in pixels.
        offset: u32,
        /// The viewport height, in pixels.
        viewport: u32,
    },
    /// A panel row was pressed: jump to its comment (FR-017).
    PanelRowSelected(CommentId),
}

/// What the shell must do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Nothing.
    None,
    /// Read the change list of `entry` under `toggles` as read `seq`.
    ReadList {
        /// The read's sequence number.
        seq: u64,
        /// The entry.
        entry: SessionLocation,
        /// Which kinds of change to list.
        toggles: Toggles,
    },
    /// Read one file's diff as read `seq` (D1). The scope and the old path are the shown list's,
    /// so the shell needs no lookup.
    ReadDiff {
        /// The read's sequence number.
        seq: u64,
        /// The entry.
        entry: SessionLocation,
        /// The scope the list was read under.
        scope: ReviewScope,
        /// Which kinds of change to include.
        toggles: Toggles,
        /// The file.
        path: RelPath,
        /// A renamed file's old path.
        from: Option<RelPath>,
        /// Read it even over the size limits (the user pressed Show diff).
        force_large: bool,
    },
    /// Read the pull requests of `entry`'s branch set (feature 1003, FR-002) in `project` as read
    /// `seq`. Never issued as part of a `ReadList`: the working-tree list does not wait for it
    /// (feature 775, FR-009).
    ReadPullRequests {
        /// The read's sequence number.
        seq: u64,
        /// The project.
        project: PathBuf,
        /// The worktree whose branch set is read.
        entry: SessionLocation,
        /// The entry's current branch; `None` on a detached HEAD.
        current: Option<String>,
    },
    /// Read pull request `number`'s diff in `project` as read `seq` (feature 775, FR-004).
    ReadPullRequestDiff {
        /// The read's sequence number.
        seq: u64,
        /// The project.
        project: PathBuf,
        /// The pull request's number.
        number: u64,
    },
    /// Scroll the diff to this offset, in pixels (a panel jump).
    ScrollDiff {
        /// The offset.
        offset: u32,
    },
    /// Tell the service the chosen layout (`SettingsSet { diff_layout }`, R12).
    SetLayout(DiffLayout),
    /// Send `ClientMsg::ReviewEdit` for the entry (contracts/review-wire.md).
    ReviewEdit {
        /// The project.
        project: PathBuf,
        /// The wire's worktree dir; `""` is the Default entry.
        worktree_dir: String,
        /// The change.
        edit: ReviewEditOp,
    },
    /// Send `ClientMsg::ReviewSend` for the entry (W6, S1).
    ReviewSend {
        /// The project.
        project: PathBuf,
        /// The wire's worktree dir; `""` is the Default entry.
        worktree_dir: String,
        /// The pending removed-side comments judged outdated (R14), in the entry's order; the
        /// service judges the others itself (W16).
        outdated: Vec<CommentId>,
        /// How many pending comments the send carries, for the success snackbar (S2).
        count: usize,
    },
}

/// Apply `msg` (contracts/changes-view.md V1–V3, L1).
pub fn update(state: &mut State, msg: Msg) -> Effect {
    match msg {
        Msg::LayoutChosen(layout) => {
            if state.layout != layout {
                if let Some(view) = state.open.as_mut() {
                    view.slot_heights.clear();
                }
            }
            state.layout = layout;
            Effect::SetLayout(layout)
        }
        Msg::LayoutInForce(layout) => {
            if state.layout != layout {
                if let Some(view) = state.open.as_mut() {
                    view.slot_heights.clear();
                }
            }
            state.layout = layout;
            Effect::None
        }
        Msg::Opened { project, entry } => {
            state.open = Some(OpenView {
                project,
                entry,
                toggles: Toggles::default(),
                list: Load::Idle,
                selected: None,
                list_offset: 0,
                list_viewport: 0,
                diff: Load::Idle,
                shown_large: BTreeSet::new(),
                diff_offset: 0,
                diff_viewport: 0,
                pick: None,
                composer: None,
                slot_heights: BTreeMap::new(),
                anchors: BTreeMap::new(),
                confirm_discard: false,
                reply_open: None,
                panel: Panel::default(),
                branch: None,
                pull_requests: PrList::Absent,
                source: Source::Changes,
            });
            request_read(state)
        }
        Msg::PullRequestsWanted { branch } => {
            let Some(view) = state.open.as_mut() else {
                return Effect::None;
            };
            view.branch = branch;
            request_pull_requests(state)
        }
        Msg::PullRequestsRead {
            seq,
            read_at,
            result,
        } => {
            let Some(view) = state.open.as_mut() else {
                return Effect::None;
            };
            let (again, last) = match &mut view.pull_requests {
                PrList::Loading {
                    seq: s,
                    again,
                    last,
                } if *s == seq => (*again, last.take()),
                _ => return Effect::None,
            };
            let numbers: Option<Vec<u64>> = result
                .as_ref()
                .ok()
                .map(|prs| prs.iter().map(|pr| pr.number).collect());
            view.pull_requests = match result {
                Ok(prs) => PrList::Ready { prs, read_at },
                // No GitHub access of any kind: no section, as the sidebar drops its rows.
                Err(ReadingFailure::Unavailable) => {
                    view.source = Source::Changes;
                    PrList::Absent
                }
                // Passing, or a rate limit the shell holds back: keep what was listed (FR-008).
                Err(_) => PrList::Unavailable { last },
            };
            let restored = numbers.and_then(|numbers| reconcile_source(state, &numbers, again));
            if again {
                request_pull_requests(state)
            } else {
                restored.unwrap_or(Effect::None)
            }
        }
        Msg::SourceChosen(None) => {
            let Some(view) = state.open.as_mut() else {
                return Effect::None;
            };
            if view.source == Source::Changes {
                return Effect::None;
            }
            view.source = Source::Changes;
            state
                .remembered
                .remove(&(view.project.clone(), view.entry.clone()));
            reset_diff_pane(view);
            Effect::None
        }
        Msg::SourceChosen(Some(number)) => {
            let Some(view) = state.open.as_ref() else {
                return Effect::None;
            };
            let listed = view
                .pull_requests
                .shown()
                .is_some_and(|(prs, _)| prs.iter().any(|pr| pr.number == number));
            let current = matches!(
                &view.source,
                Source::PullRequest(p) if p.number == number && p.diff != PrDiffLoad::Failed
            );
            if !listed || current {
                return Effect::None;
            }
            let key = (view.project.clone(), view.entry.clone());
            state.remembered.insert(key, number);
            read_pull_request_diff(state, number)
        }
        Msg::PullRequestStepped(step) => {
            let Some(view) = state.open.as_ref() else {
                return Effect::None;
            };
            let Source::PullRequest(shown) = &view.source else {
                return Effect::None;
            };
            let next = view
                .pull_requests
                .shown()
                .and_then(|(prs, _)| adjacent(prs, shown.number, step));
            match next {
                Some(number) => update(state, Msg::SourceChosen(Some(number))),
                None => Effect::None,
            }
        }
        Msg::PullRequestDiffRead {
            seq,
            number,
            result,
        } => {
            let Some(view) = state.open.as_mut() else {
                return Effect::None;
            };
            let Source::PullRequest(pr) = &mut view.source else {
                return Effect::None;
            };
            if pr.number != number || pr.diff != (PrDiffLoad::Loading { seq }) {
                return Effect::None;
            }
            pr.diff = match result {
                Ok(diff) => {
                    let mut files = Vec::with_capacity(diff.files.len());
                    let mut patches = BTreeMap::new();
                    for file in diff.files {
                        patches.insert(file.file.path.clone(), file.patch);
                        files.push(file.file);
                    }
                    PrDiffLoad::Ready { files, patches }
                }
                Err(DiffReadFailure::TooLarge) => PrDiffLoad::TooLarge,
                Err(DiffReadFailure::Failed(_)) => PrDiffLoad::Failed,
            };
            Effect::None
        }
        Msg::PullRequestOpenRequested(_) => Effect::None,
        Msg::PullRequestSettingChanged(enabled) => {
            let was = std::mem::replace(&mut state.pr_enabled, enabled);
            if enabled && !was {
                request_pull_requests(state)
            } else if !enabled {
                if let Some(view) = state.open.as_mut() {
                    view.pull_requests = PrList::Absent;
                    if view.source != Source::Changes {
                        view.source = Source::Changes;
                        reset_diff_pane(view);
                    }
                }
                Effect::None
            } else {
                Effect::None
            }
        }
        Msg::Closed | Msg::SessionSelected => {
            state.open = None;
            Effect::None
        }
        Msg::CommittedToggled => {
            let Some(view) = state.open.as_mut() else {
                return Effect::None;
            };
            if !committed_available(view) {
                return Effect::None;
            }
            view.toggles.committed = !view.toggles.committed;
            view.panel.hidden = None;
            request_read(state)
        }
        Msg::UncommittedToggled => {
            let Some(view) = state.open.as_mut() else {
                return Effect::None;
            };
            view.toggles.uncommitted = !view.toggles.uncommitted;
            view.panel.hidden = None;
            request_read(state)
        }
        Msg::FileSelected(path) => {
            let Some(view) = state.open.as_mut() else {
                return Effect::None;
            };
            if let Source::PullRequest(pr) = &mut view.source {
                // A pull request's patches are held: choosing a file parses one, reads nothing.
                pr.selected = Some(path);
                pr.file = pr_file_diff(pr);
                reset_diff_pane(view);
                return Effect::None;
            }
            // The file already shown keeps its diff and scroll; only a failed read is retried.
            if view.selected.as_ref() == Some(&path) && !matches!(view.diff, Load::Failed(_)) {
                return Effect::None;
            }
            view.selected = Some(path);
            // Picked lines and a new comment on them belong to the file they were picked in.
            close_new(view);
            view.diff = Load::Idle;
            view.diff_offset = 0;
            view.slot_heights.clear();
            request_diff(state, false)
        }
        Msg::ShowLarge => {
            let Some(view) = state.open.as_mut() else {
                return Effect::None;
            };
            if let Source::PullRequest(pr) = &mut view.source {
                if let Some(path) = pr.selected.clone() {
                    pr.shown_large.insert(path);
                    pr.file = pr_file_diff(pr);
                }
                return Effect::None;
            }
            let Some(path) = view.selected.clone() else {
                return Effect::None;
            };
            view.shown_large.insert(path);
            request_diff(state, false)
        }
        Msg::DiffScrolled { offset, viewport } => {
            if let Some(view) = state.open.as_mut() {
                view.diff_offset = offset;
                view.diff_viewport = viewport;
            }
            Effect::None
        }
        Msg::DiffRead { seq, result } => {
            if let Some(view) = state.open.as_mut() {
                if matches!(view.diff, Load::Loading { seq: s, .. } if s == seq) {
                    view.diff = match result {
                        Ok(diff) => Load::Ready(diff),
                        Err(message) => Load::Failed(message),
                    };
                    // R2: a pick stays only while its lines are still rows of the diff.
                    if let Some(pick) = view.pick {
                        let rows = diff_rows(view);
                        let shown = |line| rows.contains(&(pick.side, line));
                        if !(shown(pick.anchor) && shown(pick.head)) {
                            view.pick = None;
                        }
                    }
                }
            }
            let moves = judge_anchors(state);
            let waiting = state.open.as_ref().is_some_and(|v| v.panel.to_show);
            match waiting.then(|| jump_offset(state)).flatten() {
                Some(offset) => Effect::ScrollDiff { offset },
                None => reanchor_effect(state, moves),
            }
        }
        Msg::PanelToggled => {
            if let Some(view) = state.open.as_mut() {
                view.panel.open = !view.panel.open;
            }
            Effect::None
        }
        Msg::PanelScrolled { offset, viewport } => {
            if let Some(view) = state.open.as_mut() {
                view.panel.offset = offset;
                view.panel.viewport = viewport;
            }
            Effect::None
        }
        Msg::PanelRowSelected(id) => {
            let Some(jump) = jump_target(state, id) else {
                return Effect::None;
            };
            let Some(view) = state.open.as_mut() else {
                return Effect::None;
            };
            let path = match jump {
                Jump::Shown { path, .. } => path,
                Jump::Hidden(why) => {
                    view.panel.hidden = Some(why);
                    view.panel.focus = None;
                    view.panel.to_show = false;
                    return Effect::None;
                }
            };
            view.panel.hidden = None;
            view.panel.focus = Some(id);
            view.panel.to_show = true;
            let effect = update(state, Msg::FileSelected(path));
            match (effect, jump_offset(state)) {
                (Effect::None, Some(offset)) => Effect::ScrollDiff { offset },
                (effect, _) => effect,
            }
        }
        Msg::ListScrolled { offset, viewport } => {
            if let Some(view) = state.open.as_mut() {
                view.list_offset = offset;
                view.list_viewport = viewport;
            }
            Effect::None
        }
        Msg::ListRead { seq, result } => list_read(state, seq, result),
        Msg::WorktreesListed(names) => {
            let gone = matches!(
                state.open.as_ref().map(|v| &v.entry),
                Some(SessionLocation::Worktree(name)) if !names.contains(name)
            );
            if gone {
                state.open = None;
            }
            Effect::None
        }
        Msg::GutterPressed { old, new, extend } => {
            if let Some(view) = state.open.as_mut().filter(|v| can_pick(v)) {
                // C1: a context line has both numbers and counts as the new side.
                let (side, line) = match (old, new) {
                    (_, Some(n)) => (Side::New, n),
                    (Some(o), None) => (Side::Old, o),
                    (None, None) => return Effect::None,
                };
                let pick = match view.pick {
                    Some(p) if extend && p.side == side => Pick { head: line, ..p },
                    _ => Pick {
                        side,
                        anchor: line,
                        head: line,
                    },
                };
                view.pick = Some(pick);
                // A new comment's composer follows the pick, keeping its text (C4).
                if let Some(composer) = view.composer.as_mut() {
                    if let ComposerTarget::New(_) = composer.target {
                        composer.target = ComposerTarget::New(pick);
                    }
                }
            }
            Effect::None
        }
        Msg::SlotMeasured { side, line, height } => {
            if let Some(view) = state.open.as_mut() {
                view.slot_heights.insert((side, line), height);
            }
            Effect::None
        }
        Msg::AddComment => {
            if let Some(view) = state.open.as_mut() {
                if let Some(pick) = view.pick {
                    view.composer = Some(Composer {
                        target: ComposerTarget::New(pick),
                        text: String::new(),
                    });
                }
            }
            Effect::None
        }
        Msg::ComposerAction(_) => Effect::None,
        Msg::ComposerEdited(text) => {
            if let Some(composer) = state.open.as_mut().and_then(|v| v.composer.as_mut()) {
                composer.text = text;
            }
            Effect::None
        }
        Msg::ComposerCancelled => {
            if let Some(view) = state.open.as_mut() {
                view.composer = None;
            }
            Effect::None
        }
        Msg::ComposerSaved => composer_saved(state),
        Msg::Changed => request_read(state),
        Msg::SendPressed => {
            let Some(view) = state.open.as_ref() else {
                return Effect::None;
            };
            let count = review_of(state, view)
                .filter(|r| !r.sending)
                .map_or(0, |r| r.comments.iter().filter(|c| is_pending(c)).count());
            if count == 0 {
                return Effect::None;
            }
            let outdated = review_of(state, view)
                .map(|r| {
                    r.comments
                        .iter()
                        .filter(|c| c.side == Side::Old && is_pending(c) && is_outdated(view, c))
                        .map(|c| c.id)
                        .collect()
                })
                .unwrap_or_default();
            Effect::ReviewSend {
                project: view.project.clone(),
                worktree_dir: entry_dir(&view.entry),
                outdated,
                count,
            }
        }
        Msg::ClearSentPressed => match state.open.as_ref() {
            Some(view) if tidy_actions(state).is_some_and(|a| a.clear_sent) => {
                review_edit(view, ReviewEditOp::ClearSent)
            }
            _ => Effect::None,
        },
        Msg::DiscardPendingPressed => {
            let enabled = tidy_actions(state).is_some_and(|a| a.discard_pending);
            if let Some(view) = state.open.as_mut() {
                view.confirm_discard = enabled;
            }
            Effect::None
        }
        Msg::DiscardCancelled => {
            if let Some(view) = state.open.as_mut() {
                view.confirm_discard = false;
            }
            Effect::None
        }
        Msg::DiscardConfirmed => {
            // Re-checked: a push since the question opened may have left nothing to discard.
            let enabled = tidy_actions(state).is_some_and(|a| a.discard_pending);
            match state.open.as_mut() {
                Some(view) if view.confirm_discard => {
                    view.confirm_discard = false;
                    if enabled {
                        review_edit(view, ReviewEditOp::DiscardPending)
                    } else {
                        Effect::None
                    }
                }
                _ => Effect::None,
            }
        }
        Msg::EditComment(id) => {
            let Some(view) = state.open.as_ref() else {
                return Effect::None;
            };
            let text = review_of(state, view)
                .filter(|r| !r.sending)
                .and_then(|r| r.comments.iter().find(|c| c.id == id && is_pending(c)))
                .map(|c| c.text.clone());
            if let (Some(text), Some(view)) = (text, state.open.as_mut()) {
                view.composer = Some(Composer {
                    target: ComposerTarget::Edit(id),
                    text,
                });
            }
            Effect::None
        }
        Msg::MarkApplied(id) => {
            let Some(view) = state.open.as_ref() else {
                return Effect::None;
            };
            if !is_markable(state, view, id) {
                return Effect::None;
            }
            review_edit(
                view,
                ReviewEditOp::Mark {
                    id,
                    outcome: Outcome::Applied,
                    note: String::new(),
                },
            )
        }
        Msg::AnswerComment(id) => {
            let markable = state
                .open
                .as_ref()
                .is_some_and(|view| is_markable(state, view, id));
            if let Some(view) = state.open.as_mut().filter(|_| markable) {
                view.composer = Some(Composer {
                    target: ComposerTarget::Answer(id),
                    text: String::new(),
                });
            }
            Effect::None
        }
        Msg::ShowReply(id) => {
            let known = state.open.as_ref().is_some_and(|view| {
                review_of(state, view)
                    .and_then(|r| r.comments.iter().find(|c| c.id == id))
                    .is_some_and(|c| full_reply(c).is_some())
            });
            if let Some(view) = state.open.as_mut().filter(|_| known) {
                view.reply_open = Some(id);
            }
            Effect::None
        }
        Msg::ReplyClosed => {
            if let Some(view) = state.open.as_mut() {
                view.reply_open = None;
            }
            Effect::None
        }
        Msg::DeleteComment(id) => {
            let Some(view) = state.open.as_ref() else {
                return Effect::None;
            };
            if review_of(state, view).is_some_and(|r| r.sending) {
                return Effect::None;
            }
            let effect = review_edit(view, ReviewEditOp::Delete { id });
            if let Some(view) = state.open.as_mut() {
                close_edit_unless(view, |editing| editing != id);
            }
            effect
        }
        Msg::ReviewChanged {
            project,
            worktree_dir,
            comments,
            sending,
        } => {
            if let Some(view) = state.open.as_mut() {
                if view.project == project && entry_dir(&view.entry) == worktree_dir {
                    close_edit_unless(view, |id| comments.iter().any(|c| c.id == id));
                    close_answer_unless(view, |id| {
                        comments
                            .iter()
                            .any(|c| c.id == id && c.state.status() == Status::Sent)
                    });
                    if view
                        .reply_open
                        .is_some_and(|id| !comments.iter().any(|c| c.id == id))
                    {
                        view.reply_open = None;
                    }
                }
            }
            state
                .reviews
                .insert((project, worktree_dir), ReviewView { comments, sending });
            judge_anchors(state);
            Effect::None
        }
    }
}

/// Close an edit composer whose comment fails `kept` (deleted here or from another window), so
/// Save never targets a comment that is gone.
fn close_edit_unless(view: &mut OpenView, kept: impl Fn(CommentId) -> bool) {
    if let Some(Composer {
        target: ComposerTarget::Edit(id),
        ..
    }) = view.composer
    {
        if !kept(id) {
            view.composer = None;
        }
    }
}

/// Close an answer composer whose comment fails `kept` (gone, or reported meanwhile), so Save
/// never marks a comment that can no longer be marked.
fn close_answer_unless(view: &mut OpenView, kept: impl Fn(CommentId) -> bool) {
    if let Some(Composer {
        target: ComposerTarget::Answer(id),
        ..
    }) = view.composer
    {
        if !kept(id) {
            view.composer = None;
        }
    }
}

/// Whether `id` is a Sent comment of the open view's entry: the only kind marked by hand.
fn is_markable(state: &State, view: &OpenView, id: CommentId) -> bool {
    review_of(state, view)
        .and_then(|r| r.comments.iter().find(|c| c.id == id))
        .is_some_and(|c| c.state.status() == Status::Sent)
}

/// A reported comment's whole reply, made displayable; `None` without one.
fn full_reply(comment: &ReviewComment) -> Option<String> {
    match &comment.state {
        CommentState::Reported { report, .. } if !report.reply().is_empty() => {
            Some(reply::display(report.reply()))
        }
        _ => None,
    }
}

/// The text of the full-reply dialog, while it is open (FR-006): the whole reply of the comment
/// **Show all** was pressed on.
pub fn reply_dialog(state: &State) -> Option<String> {
    let view = state.open.as_ref()?;
    let id = view.reply_open?;
    let comment = review_of(state, view)?
        .comments
        .iter()
        .find(|c| c.id == id)?;
    full_reply(comment)
}

/// An edit of the open view's entry.
fn review_edit(view: &OpenView, edit: ReviewEditOp) -> Effect {
    Effect::ReviewEdit {
        project: view.project.clone(),
        worktree_dir: entry_dir(&view.entry),
        edit,
    }
}

/// The lines a new comment's composer is on, while they are still picked (C4). A refresh that
/// removes them drops the pick (R2) and leaves the composer, with its text, unplaced until the
/// next pick places it again.
pub fn composer_placed(view: &OpenView) -> Option<Pick> {
    match view.composer.as_ref()?.target {
        ComposerTarget::New(pick) if view.pick == Some(pick) => Some(pick),
        _ => None,
    }
}

/// Close a new comment's composer: the file it was picked in is no longer shown.
fn close_new(view: &mut OpenView) {
    view.pick = None;
    if matches!(
        view.composer,
        Some(Composer {
            target: ComposerTarget::New(_),
            ..
        })
    ) {
        view.composer = None;
    }
}

/// Save in the composer (C2): `Add` with the quote from the loaded lines, or `SetText`; blank
/// text sends nothing and keeps the composer open.
fn composer_saved(state: &mut State) -> Effect {
    let Some(view) = state.open.as_mut() else {
        return Effect::None;
    };
    let Some(composer) = view.composer.as_ref() else {
        return Effect::None;
    };
    if composer.text.trim().is_empty() {
        return Effect::None;
    }
    let text = composer.text.clone();
    let placed = composer_placed(view);
    let edit = match composer.target {
        ComposerTarget::Edit(id) => ReviewEditOp::SetText { id, text },
        ComposerTarget::Answer(id) => ReviewEditOp::Mark {
            id,
            outcome: Outcome::Answered,
            note: text,
        },
        ComposerTarget::New(_) => {
            // Unplaced: its lines are gone and nothing is picked yet.
            let Some(pick) = placed else {
                return Effect::None;
            };
            let (Some(path), Load::Ready(loaded)) = (view.selected.as_ref(), &view.diff) else {
                return Effect::None;
            };
            let lines = match pick.side {
                Side::New => loaded.new.as_ref(),
                Side::Old => loaded.old.as_ref(),
            };
            let range = pick.range();
            let quote: Option<Vec<String>> = (range.start()..=range.end())
                .map(|n| lines.and_then(|l| l.line(n)).map(str::to_string))
                .collect();
            let Some(quote) = quote else {
                return Effect::None;
            };
            view.pick = None;
            ReviewEditOp::Add {
                path: path.as_str().to_string(),
                side: pick.side,
                start: range.start(),
                end: range.end(),
                quote,
                text,
            }
        }
    };
    view.composer = None;
    review_edit(view, edit)
}

/// The wire's form of an entry: its worktree `dir_name`, `""` for the Default entry.
pub fn entry_dir(entry: &SessionLocation) -> String {
    match entry {
        SessionLocation::Worktree(dir) => dir.clone(),
        SessionLocation::Default => String::new(),
    }
}

/// The open view's entry's comments, if the service pushed any.
pub fn review_of<'a>(state: &'a State, view: &OpenView) -> Option<&'a ReviewView> {
    state
        .reviews
        .get(&(view.project.clone(), entry_dir(&view.entry)))
}

/// The toolbar's **Send review (n)** (S1; 946 FR-009).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendAction {
    /// "Send review (n)", or "Sending…" while a send is open.
    pub label: String,
    /// Whether pressing it sends.
    pub enabled: bool,
}

/// The open view's **Send review (n)**; `None` with no view open (S1).
pub fn send_action(state: &State) -> Option<SendAction> {
    let view = state.open.as_ref()?;
    let review = review_of(state, view);
    if review.is_some_and(|r| r.sending) {
        return Some(SendAction {
            label: "Sending…".into(),
            enabled: false,
        });
    }
    let count = review.map_or(0, |r| r.comments.iter().filter(|c| is_pending(c)).count());
    Some(SendAction {
        label: format!("Send review ({count})"),
        enabled: count > 0,
    })
}

/// The **Discard drafts…** confirmation, as a floating surface (S3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfirmDiscardPendingDialog;

impl crate::overlay::FloatingSurface for ConfirmDiscardPendingDialog {
    fn id(&self) -> crate::overlay::SurfaceId {
        crate::overlay::SurfaceId::new("confirm_discard_pending")
    }

    fn layer(&self) -> micold_core::overlay::Layer {
        micold_core::overlay::Layer::Dialog
    }

    fn dismissal(&self) -> crate::overlay::DismissalRules {
        crate::overlay::DismissalRules::for_layer(micold_core::overlay::Layer::Dialog)
            .cancelled_by(crate::app::Message::Changes(Msg::DiscardCancelled))
    }
}

impl crate::overlay::registry::Registered for ConfirmDiscardPendingDialog {
    fn open_in(state: &crate::app::State) -> Option<Self> {
        discard_prompt(&state.changes).map(|_| ConfirmDiscardPendingDialog)
    }
}

/// The full-reply dialog (feature 946, FR-006), as a floating surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FullReplyDialog;

impl crate::overlay::FloatingSurface for FullReplyDialog {
    fn id(&self) -> crate::overlay::SurfaceId {
        crate::overlay::SurfaceId::new("review_full_reply")
    }

    fn layer(&self) -> micold_core::overlay::Layer {
        micold_core::overlay::Layer::Dialog
    }

    fn dismissal(&self) -> crate::overlay::DismissalRules {
        crate::overlay::DismissalRules::for_layer(micold_core::overlay::Layer::Dialog)
            .cancelled_by(crate::app::Message::Changes(Msg::ReplyClosed))
    }
}

impl crate::overlay::registry::Registered for FullReplyDialog {
    fn open_in(state: &crate::app::State) -> Option<Self> {
        reply_dialog(&state.changes).map(|_| FullReplyDialog)
    }
}

/// The toolbar's **Clear sent** and **Discard drafts…**, each enabled or not (S3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TidyActions {
    /// The open entry has a sent comment.
    pub clear_sent: bool,
    /// The open entry has a pending comment and no send is open.
    pub discard_pending: bool,
}

/// The open view's **Clear sent** and **Discard drafts…**; `None` with no view open (S3).
pub fn tidy_actions(state: &State) -> Option<TidyActions> {
    let view = state.open.as_ref()?;
    let review = review_of(state, view);
    let any =
        |pending: bool| review.is_some_and(|r| r.comments.iter().any(|c| is_pending(c) == pending));
    Some(TidyActions {
        clear_sent: any(false),
        discard_pending: any(true) && !review.is_some_and(|r| r.sending),
    })
}

/// The discard confirmation's question while it is open (S3): "Discard n draft comments? This
/// cannot be undone."
pub fn discard_prompt(state: &State) -> Option<String> {
    let view = state.open.as_ref().filter(|view| view.confirm_discard)?;
    let count =
        review_of(state, view).map_or(0, |r| r.comments.iter().filter(|c| is_pending(c)).count());
    let noun = if count == 1 { "comment" } else { "comments" };
    Some(format!(
        "Discard {count} draft {noun}? This cannot be undone."
    ))
}

/// The success snackbar (S2).
pub fn sent_text(count: usize, session: &str, started: bool) -> String {
    let noun = if count == 1 { "comment" } else { "comments" };
    if started {
        format!("Started a session and sent a review of {count} {noun}")
    } else {
        format!("Sent a review of {count} {noun} to {session}")
    }
}

/// The error snackbar (S2): the service's message.
pub fn send_error_text(message: &str) -> String {
    format!("Couldn't send the review: {message}")
}

/// Each file's pending-comment count in the open view's entry (L2).
pub fn pending_counts(state: &State) -> BTreeMap<RelPath, usize> {
    let mut counts = BTreeMap::new();
    let review = state.open.as_ref().and_then(|view| review_of(state, view));
    for comment in review.iter().flat_map(|r| &r.comments) {
        if is_pending(comment) {
            *counts.entry(comment.path.clone()).or_insert(0) += 1;
        }
    }
    counts
}

/// The selected file's comments, placed for the diff on screen (C3).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileComments<'a> {
    /// Comments shown under the row of their last line, keyed by that line's side and number.
    pub placed: BTreeMap<(Side, u32), Vec<&'a ReviewComment>>,
    /// Comments whose last line is not in the diff on screen ("Not in the current diff").
    pub not_in_diff: Vec<&'a ReviewComment>,
}

impl<'a> FileComments<'a> {
    /// The comments shown under the row of line `line` of `side`.
    pub fn under(&self, side: Side, line: u32) -> Vec<&'a ReviewComment> {
        self.placed.get(&(side, line)).cloned().unwrap_or_default()
    }
}

/// The selected file's comments in the open view's entry, placed for the diff on screen (C3).
pub fn file_comments(state: &State) -> FileComments<'_> {
    let mut placed = FileComments::default();
    let Some(view) = state.open.as_ref() else {
        return placed;
    };
    if view.source != Source::Changes {
        return placed;
    }
    let (Some(path), Some(review)) = (view.selected.as_ref(), review_of(state, view)) else {
        return placed;
    };
    let hunks = match diff_body(view) {
        DiffBody::Diff(LoadedDiff {
            diff: FileDiff::Text(hunks),
            ..
        }) => hunks.as_slice(),
        _ => &[],
    };
    for comment in review.comments.iter().filter(|c| &c.path == path) {
        let end = shown_range(view, comment).end();
        // The row a comment sits under: its last line on its side; a removed-side comment may
        // end on a context line, which carries both numbers.
        let shown = hunks
            .iter()
            .flat_map(|h| &h.lines)
            .any(|line| match comment.side {
                Side::New => line.new == Some(end),
                Side::Old => line.old == Some(end),
            });
        if shown {
            placed
                .placed
                .entry((comment.side, end))
                .or_default()
                .push(comment);
        } else {
            placed.not_in_diff.push(comment);
        }
    }
    placed
}

/// Whether a comment is still a draft.
pub fn is_pending(comment: &ReviewComment) -> bool {
    comment.state == CommentState::Draft
}

/// What a comment's card shows (FR-002, FR-007): its status, whether a send holds it, the reply
/// made displayable and shortened to [`CARD_CHARS`] (`true` when cut), and whether Edit and Delete
/// are offered (a draft no send holds).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardModel {
    /// One of the four statuses.
    pub status: Status,
    /// A draft taken by the send under way.
    pub in_send: bool,
    /// The reply to draw under the text, and whether it was shortened; `None` without one.
    pub reply: Option<(String, bool)>,
    /// Edit and Delete are offered.
    pub editable: bool,
    /// **Mark applied** and **Answer…** are offered: the comment is Sent (feature 946, FR-006).
    pub markable: bool,
}

/// The card of `comment` while the entry's `sending` flag is as given.
pub fn card_model(comment: &ReviewComment, sending: bool) -> CardModel {
    let status = comment.state.status();
    let draft = status == Status::Draft;
    let reply = match &comment.state {
        CommentState::Reported { report, .. } if !report.reply().is_empty() => Some(
            reply::shortened(&reply::display(report.reply()), CARD_CHARS),
        ),
        _ => None,
    };
    CardModel {
        status,
        in_send: draft && sending,
        reply,
        editable: draft && !sending,
        markable: status == Status::Sent,
    }
}

/// The selected file of a pull request, parsed from the patch GitHub gave: a file over the limits
/// waits for **Show diff** like a working-tree one. There are no full versions to show, so the
/// diff carries no syntax colours and no context beyond its hunks.
fn pr_file_diff(pr: &PullRequestView) -> Option<LoadedDiff> {
    let PrDiffLoad::Ready { files, patches } = &pr.diff else {
        return None;
    };
    let path = pr.selected.as_ref()?;
    let file = files.iter().find(|f| &f.path == path)?;
    let diff = if file.large && !pr.shown_large.contains(path) {
        FileDiff::TooLarge {
            added: file.added,
            removed: file.removed,
        }
    } else {
        parse_unified(patches.get(path).map_or(&[][..], Vec::as_slice))
    };
    Some(LoadedDiff {
        diff,
        old: None,
        new: None,
        spans: Spans::default(),
    })
}

/// What a change of source resets in the diff pane: picks, the composer, scroll and slot heights
/// belong to the file that was shown.
fn reset_diff_pane(view: &mut OpenView) {
    close_new(view);
    view.diff_offset = 0;
    view.slot_heights.clear();
}

/// Show pull request `number`: ask for its diff, replacing whatever source was shown.
fn read_pull_request_diff(state: &mut State, number: u64) -> Effect {
    let seq = state.next_seq;
    let Some(view) = state.open.as_mut() else {
        return Effect::None;
    };
    view.source = Source::PullRequest(Box::new(PullRequestView {
        number,
        diff: PrDiffLoad::Loading { seq },
        selected: None,
        file: None,
        shown_large: BTreeSet::new(),
    }));
    reset_diff_pane(view);
    state.next_seq += 1;
    Effect::ReadPullRequestDiff {
        seq,
        project: view.project.clone(),
        number,
    }
}

/// A fresh list of `numbers` arrived (FR-013): a shown pull request that is not in it returns the
/// view to the working tree and is forgotten; the one this entry remembered is shown again once a
/// list holds it (`again`: another list read follows, so wait for it).
fn reconcile_source(state: &mut State, numbers: &[u64], again: bool) -> Option<Effect> {
    let view = state.open.as_mut()?;
    let key = (view.project.clone(), view.entry.clone());
    match &view.source {
        Source::PullRequest(pr) if !numbers.contains(&pr.number) => {
            view.source = Source::Changes;
            reset_diff_pane(view);
            state.remembered.remove(&key);
            None
        }
        Source::PullRequest(_) => None,
        Source::Changes => {
            let wanted = state.remembered.get(&key).copied()?;
            if !numbers.contains(&wanted) {
                state.remembered.remove(&key);
                return None;
            }
            (!again).then(|| read_pull_request_diff(state, wanted))
        }
    }
}

/// Start a read of the open view's pull requests, or queue one behind the read under way. Nothing
/// is read, and no section shown, while the setting is off or the entry is the project root
/// (FR-012); a worktree on a detached HEAD still reads the branches created there (feature 1003).
fn request_pull_requests(state: &mut State) -> Effect {
    let seq = state.next_seq;
    let enabled = state.pr_enabled;
    let Some(view) = state.open.as_mut() else {
        return Effect::None;
    };
    if !enabled || !matches!(view.entry, SessionLocation::Worktree(_)) {
        view.pull_requests = PrList::Absent;
        view.source = Source::Changes;
        return Effect::None;
    }
    let last = match std::mem::take(&mut view.pull_requests) {
        PrList::Loading {
            seq,
            last,
            again: _,
        } => {
            view.pull_requests = PrList::Loading {
                seq,
                again: true,
                last,
            };
            return Effect::None;
        }
        PrList::Ready { prs, read_at } => Some((prs, read_at)),
        PrList::Unavailable { last } => last,
        PrList::Absent => None,
    };
    view.pull_requests = PrList::Loading {
        seq,
        again: false,
        last,
    };
    state.next_seq += 1;
    Effect::ReadPullRequests {
        seq,
        project: view.project.clone(),
        entry: view.entry.clone(),
        current: view.branch.clone(),
    }
}

/// Start a read of the open view's list, or queue one behind the read under way.
fn request_read(state: &mut State) -> Effect {
    let seq = state.next_seq;
    let Some(view) = state.open.as_mut() else {
        return Effect::None;
    };
    if let Load::Loading { again, .. } = &mut view.list {
        *again = true;
        return Effect::None;
    }
    let last = match std::mem::take(&mut view.list) {
        Load::Ready(list) => Some(list),
        _ => None,
    };
    view.list = Load::Loading {
        seq,
        again: false,
        last,
    };
    state.next_seq += 1;
    Effect::ReadList {
        seq,
        entry: view.entry.clone(),
        toggles: view.toggles,
    }
}

/// Read `seq` ended: show its answer unless a newer read replaced it, then run a queued read.
fn list_read(state: &mut State, seq: u64, result: Result<ChangeList, String>) -> Effect {
    let Some(view) = state.open.as_mut() else {
        return Effect::None;
    };
    let again = match view.list {
        Load::Loading { seq: s, again, .. } if s == seq => again,
        _ => return Effect::None,
    };
    view.list = match result {
        Ok(list) => {
            if let Some(selected) = &view.selected {
                if !list.files.iter().any(|f| &f.path == selected) {
                    view.selected = None;
                    view.diff = Load::Idle;
                    close_new(view);
                }
            }
            Load::Ready(list)
        }
        Err(message) => {
            // Nothing listed backs the selection any more.
            view.selected = None;
            view.diff = Load::Idle;
            close_new(view);
            Load::Failed(message)
        }
    };
    judge_anchors(state);
    if again {
        request_read(state)
    } else {
        // The kept selection may have changed under the new list: read its diff again, keeping
        // the old one on screen meanwhile.
        request_diff(state, true)
    }
}

/// Start a read of the selected file's diff under the shown list's scope; `keep`: the diff on
/// screen stays there until the answer arrives. A newer read replaces an older one: the older
/// answer is dropped by its `seq`.
fn request_diff(state: &mut State, keep: bool) -> Effect {
    let seq = state.next_seq;
    let Some(view) = state.open.as_mut() else {
        return Effect::None;
    };
    let Some(path) = view.selected.clone() else {
        return Effect::None;
    };
    let Some(list) = shown_list(view) else {
        return Effect::None;
    };
    let Some(file) = list.files.iter().find(|f| f.path == path) else {
        // No read is in flight and none will start: an idle pane would say "loading" for good
        // (#771). A later list that has the file asks again and replaces this.
        if matches!(view.diff, Load::Idle) {
            view.diff = Load::Failed(format!("{path} is not among the changed files"));
        }
        return Effect::None;
    };
    let from = match &file.kind {
        ChangeKind::Renamed { from } => Some(from.clone()),
        _ => None,
    };
    let scope = list.scope.clone();
    let last = match std::mem::take(&mut view.diff) {
        Load::Ready(diff) if keep => Some(diff),
        Load::Loading { last, .. } if keep => last,
        _ => None,
    };
    view.diff = Load::Loading {
        seq,
        again: false,
        last,
    };
    state.next_seq += 1;
    Effect::ReadDiff {
        seq,
        entry: view.entry.clone(),
        scope,
        toggles: view.toggles,
        force_large: view.shown_large.contains(&path),
        path,
        from,
    }
}

/// The `(side, line)` of every row of the diff on screen; a context row gives both sides.
fn diff_rows(view: &OpenView) -> BTreeSet<(Side, u32)> {
    let DiffBody::Diff(LoadedDiff {
        diff: FileDiff::Text(hunks),
        ..
    }) = diff_body(view)
    else {
        return BTreeSet::new();
    };
    let mut rows = BTreeSet::new();
    for line in hunks.iter().flat_map(|h| &h.lines) {
        rows.extend(line.old.map(|n| (Side::Old, n)));
        rows.extend(line.new.map(|n| (Side::New, n)));
    }
    rows
}

/// Judge the open entry's comments again (R14, feature 946 R2): a comment whose file left the shown
/// list is outdated; one on the shown file is located again in the lines read now (a removed-side
/// one only checks its quote); any other keeps its last anchor. Returns the moves of current-side
/// comments whose code moved.
fn judge_anchors(state: &mut State) -> Vec<Move> {
    let Some(view) = state.open.as_ref() else {
        return Vec::new();
    };
    let comments = review_of(state, view).map_or(&[][..], |r| r.comments.as_slice());
    let listed = shown_list(view);
    let read = match &view.diff {
        Load::Ready(diff) => Some(diff),
        _ => None,
    };
    let mut anchors: BTreeMap<CommentId, Anchor> = view
        .anchors
        .iter()
        .filter(|(id, _)| comments.iter().any(|c| c.id == **id))
        .map(|(id, anchor)| (*id, *anchor))
        .collect();
    let mut moves = Vec::new();
    for comment in comments {
        let status = comment.state.status();
        let held = anchors.get(&comment.id).map_or(comment.range, |a| a.shown);
        let judged = if listed.is_some_and(|l| !l.files.iter().any(|f| f.path == comment.path)) {
            Some(Anchor {
                shown: held,
                outdated: matches!(status, Status::Draft | Status::Sent),
            })
        } else if view.selected.as_ref() == Some(&comment.path) {
            read.map(|diff| match comment.side {
                Side::Old => Anchor {
                    shown: comment.range,
                    outdated: matches!(status, Status::Draft | Status::Sent)
                        && comment.is_outdated(diff.old.as_ref()),
                },
                Side::New => {
                    let located = diff.new.as_ref().map_or(Located::Gone, |lines| {
                        locate(&comment.quote, comment.range, lines)
                    });
                    if let Located::Moved(to) = located {
                        moves.extend(Move::new(comment.id, comment.range, to));
                    }
                    Anchor {
                        shown: located
                            .shown(comment.range, diff.new.as_ref().map_or(0, |l| l.len())),
                        outdated: anchor::is_outdated(&located, status),
                    }
                }
            })
        } else {
            None
        };
        match judged {
            Some(anchor) => {
                anchors.insert(comment.id, anchor);
            }
            None => {
                anchors.entry(comment.id).or_insert(Anchor {
                    shown: comment.range,
                    outdated: false,
                });
            }
        }
    }
    if let Some(view) = state.open.as_mut() {
        view.anchors = anchors;
    }
    moves
}

/// The `Reanchor` edit for `moves` (research R3), only while the shown new side is the working
/// tree: the Default entry, or the Uncommitted toggle on, and never for a pull request's diff.
fn reanchor_effect(state: &State, moves: Vec<Move>) -> Effect {
    let Some(view) = state.open.as_ref() else {
        return Effect::None;
    };
    let working_tree = view.source == Source::Changes
        && (view.entry == SessionLocation::Default || view.toggles.uncommitted);
    if moves.is_empty() || !working_tree {
        return Effect::None;
    }
    review_edit(view, ReviewEditOp::Reanchor { moves })
}

/// Whether the Committed toggle can be used: not for the Default entry (L1, US1 s9).
pub fn committed_available(view: &OpenView) -> bool {
    view.entry != SessionLocation::Default
}

/// The file the list and diff show as selected: the pull request's own while one is shown.
pub fn selected_file(view: &OpenView) -> Option<&RelPath> {
    match &view.source {
        Source::Changes => view.selected.as_ref(),
        Source::PullRequest(pr) => pr.selected.as_ref(),
    }
}

/// What the list pane shows (L3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListBody<'a> {
    /// The first read has not answered yet.
    Loading,
    /// The read failed, with git's message.
    Failed(&'a str),
    /// No rows, and the sentence that says why.
    Empty(String),
    /// The rows.
    Files(&'a [ChangedFile]),
    /// The shown pull request has no diff to list: GitHub refused it as too large, or the read
    /// failed (feature 775).
    PullRequestUnread {
        /// Too large rather than failed: the way out is its address.
        too_large: bool,
    },
}

/// What the list pane shows for `view` (L3).
pub fn list_body(view: &OpenView) -> ListBody<'_> {
    if let Source::PullRequest(pr) = &view.source {
        return match &pr.diff {
            PrDiffLoad::Loading { .. } => ListBody::Loading,
            PrDiffLoad::TooLarge => ListBody::PullRequestUnread { too_large: true },
            PrDiffLoad::Failed => ListBody::PullRequestUnread { too_large: false },
            PrDiffLoad::Ready { files, .. } if files.is_empty() => {
                ListBody::Empty("This pull request changes no files".into())
            }
            PrDiffLoad::Ready { files, .. } => ListBody::Files(files),
        };
    }
    if let Load::Failed(message) = &view.list {
        return ListBody::Failed(message);
    }
    let Some(list) = shown_list(view) else {
        return ListBody::Loading;
    };
    if !list.files.is_empty() {
        return ListBody::Files(&list.files);
    }
    let committed = view.toggles.committed && committed_available(view);
    if !committed && !view.toggles.uncommitted {
        return ListBody::Empty(BOTH_HIDDEN.into());
    }
    ListBody::Empty(match &list.scope {
        ReviewScope::Worktree {
            base: Base::MergeBase { branch, .. },
        } => format!("No changes against {branch}"),
        _ => "No uncommitted changes".into(),
    })
}

/// The header's base line (V2): `Compared with <branch> at <short sha>`, or why there is none.
/// `None` until the first read answers.
pub fn base_line(view: &OpenView) -> Option<String> {
    if matches!(view.source, Source::PullRequest(_)) {
        return pr_header(view).map(|header| header.base);
    }
    let list = shown_list(view)?;
    Some(match &list.scope {
        ReviewScope::Worktree {
            base: Base::MergeBase { branch, commit },
        } => {
            let short: String = commit.chars().take(7).collect();
            format!("Compared with {branch} at {short}")
        }
        ReviewScope::Worktree {
            base: Base::Unavailable(why),
        } => reason(*why).to_string(),
        ReviewScope::RootUncommitted => "Uncommitted changes in the project root".into(),
    })
}

/// The list currently on screen: the ready one, or the one kept while a re-read runs.
pub fn shown_list(view: &OpenView) -> Option<&ChangeList> {
    match &view.list {
        Load::Ready(list) => Some(list),
        Load::Loading { last, .. } => last.as_ref(),
        Load::Idle | Load::Failed(_) => None,
    }
}

/// Why a worktree has no base, as the header says it (L3, Edge "No base found").
fn reason(why: BaseUnavailable) -> &'static str {
    match why {
        BaseUnavailable::NoDefaultBranch => {
            "No default branch found (origin/HEAD, main or master); only uncommitted changes are listed"
        }
        BaseUnavailable::NoCommonHistory => {
            "This worktree has no history in common with the default branch; only uncommitted changes are listed"
        }
    }
}

/// What the diff pane shows (D1, D3, D4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffBody<'a> {
    /// No file is selected.
    NoSelection,
    /// The first read of the selected file has not answered yet.
    Loading,
    /// The read failed, with git's message.
    Failed(&'a str),
    /// The diff on screen (the ready one, or the one kept while it is re-read).
    Diff(&'a LoadedDiff),
}

/// What the diff pane shows for `view`.
pub fn diff_body(view: &OpenView) -> DiffBody<'_> {
    if let Source::PullRequest(pr) = &view.source {
        return match (&pr.selected, &pr.file) {
            (None, _) => DiffBody::NoSelection,
            (Some(_), Some(loaded)) => DiffBody::Diff(loaded),
            (Some(_), None) => DiffBody::Loading,
        };
    }
    if view.selected.is_none() {
        return DiffBody::NoSelection;
    }
    match &view.diff {
        Load::Ready(diff) => DiffBody::Diff(diff),
        Load::Loading {
            last: Some(diff), ..
        } => DiffBody::Diff(diff),
        Load::Failed(message) => DiffBody::Failed(message),
        Load::Idle | Load::Loading { last: None, .. } => DiffBody::Loading,
    }
}

/// Whether lines of the shown diff can be picked for a comment: text only (D3).
pub fn can_pick(view: &OpenView) -> bool {
    // Review comments belong to the entry's own files, never to a pull request's diff.
    view.source == Source::Changes
        && matches!(
            &view.diff,
            Load::Ready(LoadedDiff {
                diff: FileDiff::Text(_),
                ..
            })
        )
}

/// How many bytes of a line are syntax-coloured: a minified line cannot stall the highlighter, and
/// the rest of it shows in the plain text colour (R10).
pub const SPAN_CAP: usize = 2_000;

/// One line's spans as a highlighter gave them (a colour, or none for "the plain text colour"),
/// kept as the view paints them: coloured, non-empty, inside the first [`SPAN_CAP`] bytes. Plain
/// text, as an unknown extension highlights, has no colour and so keeps no spans (R10).
pub fn cap_spans(
    spans: impl IntoIterator<Item = (std::ops::Range<usize>, Option<Rgb>)>,
) -> Vec<(std::ops::Range<usize>, Rgb)> {
    spans
        .into_iter()
        .filter_map(|(range, colour)| {
            let range = range.start..range.end.min(SPAN_CAP);
            let colour = colour?;
            (!range.is_empty()).then_some((range, colour))
        })
        .collect()
}
