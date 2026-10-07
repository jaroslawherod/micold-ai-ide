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
use micold_core::review::base::{Base, BaseUnavailable, ReviewScope, Toggles};
use micold_core::review::changes::{ChangeKind, ChangeList, ChangedFile};
use micold_core::review::comment::{CommentId, CommentState, ReviewComment};
use micold_core::review::diff::{FileDiff, LoadedDiff};
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
    /// **Add comment** was pressed (C2).
    AddComment,
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
}

/// Apply `msg` (contracts/changes-view.md V1–V3, L1).
pub fn update(state: &mut State, msg: Msg) -> Effect {
    match msg {
        Msg::LayoutChosen(layout) => {
            state.layout = layout;
            Effect::SetLayout(layout)
        }
        Msg::LayoutInForce(layout) => {
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
            });
            request_read(state)
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
            request_read(state)
        }
        Msg::UncommittedToggled => {
            let Some(view) = state.open.as_mut() else {
                return Effect::None;
            };
            view.toggles.uncommitted = !view.toggles.uncommitted;
            request_read(state)
        }
        Msg::FileSelected(path) => {
            let Some(view) = state.open.as_mut() else {
                return Effect::None;
            };
            // The file already shown keeps its diff and scroll; only a failed read is retried.
            if view.selected.as_ref() == Some(&path) && !matches!(view.diff, Load::Failed(_)) {
                return Effect::None;
            }
            view.selected = Some(path);
            // Picked lines and a new comment on them belong to the file they were picked in.
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
            view.diff = Load::Idle;
            view.diff_offset = 0;
            request_diff(state, false)
        }
        Msg::ShowLarge => {
            let Some(view) = state.open.as_mut() else {
                return Effect::None;
            };
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
                }
            }
            Effect::None
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
                view.pick = Some(match view.pick {
                    Some(p) if extend && p.side == side => Pick { head: line, ..p },
                    _ => Pick {
                        side,
                        anchor: line,
                        head: line,
                    },
                });
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
        Msg::DeleteComment(id) => {
            let Some(view) = state.open.as_ref() else {
                return Effect::None;
            };
            if review_of(state, view).is_some_and(|r| r.sending) {
                return Effect::None;
            }
            review_edit(view, ReviewEditOp::Delete { id })
        }
        Msg::ReviewChanged {
            project,
            worktree_dir,
            comments,
            sending,
        } => {
            state
                .reviews
                .insert((project, worktree_dir), ReviewView { comments, sending });
            Effect::None
        }
    }
}

/// An edit of the open view's entry.
fn review_edit(view: &OpenView, edit: ReviewEditOp) -> Effect {
    Effect::ReviewEdit {
        project: view.project.clone(),
        worktree_dir: entry_dir(&view.entry),
        edit,
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
    let edit = match composer.target {
        ComposerTarget::Edit(id) => ReviewEditOp::SetText { id, text },
        ComposerTarget::New(pick) => {
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
        let end = comment.range.end();
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

/// Whether a comment is still pending.
pub fn is_pending(comment: &ReviewComment) -> bool {
    comment.state == CommentState::Pending
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
                }
            }
            Load::Ready(list)
        }
        Err(message) => {
            // Nothing listed backs the selection any more.
            view.selected = None;
            view.diff = Load::Idle;
            Load::Failed(message)
        }
    };
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

/// Whether the Committed toggle can be used: not for the Default entry (L1, US1 s9).
pub fn committed_available(view: &OpenView) -> bool {
    view.entry != SessionLocation::Default
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
}

/// What the list pane shows for `view` (L3).
pub fn list_body(view: &OpenView) -> ListBody<'_> {
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
    matches!(
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
