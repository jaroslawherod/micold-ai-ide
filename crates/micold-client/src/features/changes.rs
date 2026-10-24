//! The Changes view of one entry: which entry is open, its toggles, the changed-file list and the
//! selected file (feature 482, contracts/changes-view.md V1–V3 and L1–L3, data-model § Client).
//!
//! Render-free. The shell half (`shell/changes.rs`) turns [`Effect::ReadList`] and
//! [`Effect::ReadDiff`] into git reads off the update thread and answers [`Msg::ListRead`] and
//! [`Msg::DiffRead`]. What the list pane says when it has no rows
//! is chosen here ([`list_body`], [`base_line`]) so the glue in `ui/changes.rs` only renders it.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::PathBuf;

use micold_core::context_header::{ChangeSummary, CountsReading};
use micold_core::protocol::messages::{PrSend, ReviewEditOp};
use micold_core::pull_request::{
    BranchPrState, BranchPullRequest, CheckStatus, DiffReadFailure, ReadingFailure, TreeListing,
};
use micold_core::review::anchor::{self, locate, Located};
use micold_core::review::base::{Base, BaseUnavailable, ReviewScope, Toggles};
use micold_core::review::browse::{BrowseStack, FileReadFailure, FileText, Frame};
use micold_core::review::changes::{ChangeKind, ChangeList, ChangedFile};
use micold_core::review::comment::{
    CommentId, CommentState, Move, Outcome, PrAnchor, ReviewComment, Scope, Status,
};
use micold_core::review::diff::{
    parse_unified, plain_file, side_by_side_rows, unified_rows, FileDiff, Hunk, HunkLines,
    LoadedDiff, SideLines, SideRow, Spans, UnifiedRow,
};
use micold_core::review::expand::{gaps, shape, Expansion, GapKey, Shaped, ShapedRef};
use micold_core::review::pr_diff::PrDiff;
use micold_core::review::reference::{self, Reference};
use micold_core::review::reply::{self, CARD_CHARS};
use micold_core::review::{LineRange, RelPath, Side};
use micold_core::session::SessionLocation;
use micold_core::settings::DiffLayout;
use micold_core::tokens::Rgb;
use micold_core::typeahead::{rank, Query};

/// The note beside the Default entry's unavailable Committed toggle (L1, US1 s9), worded in core.
pub use micold_core::review::source_menu::DEFAULT_ENTRY_NOTE;

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
    /// No section: the setting is off, or the entry is the project root.
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

/// What the Pull requests section draws (feature 1003, spec § Section states): the core's list
/// state, which the source control's rows are built from too (feature 1272, data-model §2).
pub use micold_core::review::source_menu::PrRows as PrSection;

/// The section's state for `view` at `now` (Unix seconds); `enabled` is the PR status setting.
pub fn section(view: &OpenView, enabled: bool, now: u64) -> PrSection<'_> {
    if !enabled {
        return PrSection::Absent;
    }
    match (view.pull_requests.shown(), &view.pull_requests) {
        (Some(([], _)), _) => PrSection::Empty,
        (Some((prs, _)), _) => PrSection::List {
            prs,
            stale: view.pull_requests.possibly_stale(now),
        },
        (None, PrList::Loading { .. }) => PrSection::Loading,
        (None, PrList::Unavailable { .. }) => PrSection::Unavailable,
        (None, _) => PrSection::Absent,
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
    /// Its head commit when the diff read started (1149, R3): what a comment written on this
    /// diff records as `head_commit`. Empty when the list did not hold the number.
    pub head: String,
    /// A re-read of the diff under way because the list shows a newer head: its `seq` and the
    /// head it reads (1149 FR-012). The diff on screen stays until it answers.
    pub refresh: Option<(u64, String)>,
    /// Its diff as read.
    pub diff: PrDiffLoad,
    /// The selected file.
    pub selected: Option<RelPath>,
    /// The selected file's diff, parsed from its patch: no read per file.
    pub file: Option<LoadedDiff>,
    /// Files over the size limits the user asked to see.
    pub shown_large: BTreeSet<RelPath>,
    /// The selected file's head version, which revealed lines come from (feature 1150, FR-005).
    pub head_text: HeadText,
}

/// The selected pull request file's version at the pull request's head (feature 1150, FR-005).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum HeadText {
    /// Not asked for: nothing selected, or a file with no head version to show.
    #[default]
    NotRead,
    /// Read `seq` is under way.
    Reading {
        /// The read's sequence number; an answer with another is dropped.
        seq: u64,
    },
    /// The head version's lines.
    Ready(SideLines),
    /// The head version could not be obtained; the diff stays usable (FR-009).
    Failed(FileReadFailure),
}

/// The note above a pull request file's diff while its head version is read.
pub const READING_FILE: &str = "Reading file\u{2026}";

/// What the note above the shown pull request file says of its head version: that it is being
/// read, or why it cannot be shown (FR-009); `None` when there is nothing to say.
pub fn head_note(view: &OpenView) -> Option<String> {
    let Source::PullRequest(pr) = &view.source else {
        return None;
    };
    match &pr.head_text {
        HeadText::Reading { .. } => Some(READING_FILE.to_owned()),
        HeadText::Failed(why) => Some(why.message()),
        HeadText::NotRead | HeadText::Ready(_) => None,
    }
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

/// The one mapping from a listed pull request's state to the mark the sidebar uses (FR-003), and
/// an Overview card's pill (feature 1123, research R9).
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
    /// Which hidden runs of each file are revealed, per source and file; kept while the view is
    /// open (feature 1150, FR-008).
    pub expansions: BTreeMap<(SourceKey, RelPath), Expansion>,
    /// The selected file's hunks with its revealed runs inserted, and its markers; recomputed by
    /// [`update`] after every message that can change them. The hunks of the diff as read when
    /// nothing can be revealed.
    pub shaped: Shaped,
    /// The repository file shown in place of the diff, when the reviewer is browsing (feature
    /// 1150, FR-011). The diff, its selection, scroll and expansion stay as they were.
    pub browse: Option<BrowsedFile>,
    /// Where Back returns to (FR-013).
    pub stack: BrowseStack,
    /// The file tree and path picker.
    pub files: FilesPane,
    /// A line saying a followed reference named no file of the repository (FR-012); cleared by
    /// the next message.
    pub notice: Option<&'static str>,
}

/// A repository file opened from the tree or the picker (feature 1150, data-model §8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowsedFile {
    /// The file.
    pub path: RelPath,
    /// Its text, once read.
    pub load: BrowseLoad,
    /// The line a followed reference names: the file scrolls to it once read (FR-012).
    pub line: Option<u32>,
}

/// How far reading a browsed file got.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowseLoad {
    /// Read `seq` is under way; an answer with another is dropped.
    Reading {
        /// The read's sequence number.
        seq: u64,
    },
    /// The file as context-only lines, coloured.
    Ready(LoadedDiff),
    /// The file cannot be shown (FR-009).
    Failed(FileReadFailure),
}

/// What the diff pane shows while a repository file is browsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowseBody<'a> {
    /// The file is being read.
    Reading,
    /// The file cannot be shown; `large`: it is only over the size limit, so "Show anyway" reads it.
    Failed {
        /// The one line that says why.
        message: String,
        /// Offer "Show anyway".
        large: bool,
    },
    /// The file's lines.
    File(&'a LoadedDiff),
}

/// What the browsed file's pane shows, `None` while the diff is shown.
pub fn browse_body(view: &OpenView) -> Option<BrowseBody<'_>> {
    Some(match &view.browse.as_ref()?.load {
        BrowseLoad::Reading { .. } => BrowseBody::Reading,
        BrowseLoad::Failed(why) => BrowseBody::Failed {
            message: why.message(),
            large: matches!(why, FileReadFailure::TooLarge { .. }),
        },
        BrowseLoad::Ready(loaded) => BrowseBody::File(loaded),
    })
}

/// Back has somewhere to return to (FR-013).
pub fn can_go_back(view: &OpenView) -> bool {
    !view.stack.is_empty()
}

/// The browsed file is one the change lists, so its diff can be opened (FR-015).
pub fn file_diff_available(view: &OpenView) -> bool {
    view.browse
        .as_ref()
        .is_some_and(|b| file_kind(view, &b.path).is_some())
}

/// The file tree and path picker of the list pane (feature 1150, FR-011, research R4).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FilesPane {
    /// The pane shows the repository's files in place of the changed ones.
    pub shown: bool,
    /// The repository's files, as last read.
    pub tree: Load<Vec<RelPath>>,
    /// The paths of the last tree read, for resolving references in diff lines (FR-012).
    pub listed: HashSet<String>,
    /// GitHub cut the pull request's tree short: the list is partial (a typed path still opens).
    pub truncated: bool,
    /// What was typed in the path picker.
    pub picker: String,
    /// The folders the tree shows open, as `/`-separated paths.
    pub open_folders: BTreeSet<String>,
}

/// One row of the file tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeRow {
    /// Nesting depth, 0 at the top.
    pub depth: u16,
    /// The folder's or file's own name.
    pub name: String,
    /// What it is.
    pub kind: TreeKind,
}

/// A tree row's kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeKind {
    /// A folder, by its path from the root.
    Folder {
        /// The path.
        path: String,
        /// Its files are listed below it.
        open: bool,
    },
    /// A file.
    File(RelPath),
}

/// The files the tree last read: the ready list, or the one kept while it is read again.
fn tree_files(view: &OpenView) -> &[RelPath] {
    match &view.files.tree {
        Load::Ready(files) => files,
        Load::Loading {
            last: Some(files), ..
        } => files,
        _ => &[],
    }
}

/// A folder's contents while the tree is laid out.
#[derive(Default)]
struct Folder {
    folders: BTreeMap<String, Folder>,
    files: Vec<(String, RelPath)>,
}

/// The tree's visible rows: folders before files at each level, a folder's contents only while it
/// is open (FR-011).
pub fn tree_rows(view: &OpenView) -> Vec<TreeRow> {
    let mut root = Folder::default();
    for file in tree_files(view) {
        let mut at = &mut root;
        let mut parts = file.as_str().split('/').peekable();
        while let Some(part) = parts.next() {
            if parts.peek().is_some() {
                at = at.folders.entry(part.to_owned()).or_default();
            } else {
                at.files.push((part.to_owned(), file.clone()));
            }
        }
    }
    fn walk(
        folder: &Folder,
        prefix: &str,
        depth: u16,
        open: &BTreeSet<String>,
        out: &mut Vec<TreeRow>,
    ) {
        for (name, inner) in &folder.folders {
            let path = format!("{prefix}{name}");
            let is_open = open.contains(&path);
            out.push(TreeRow {
                depth,
                name: name.clone(),
                kind: TreeKind::Folder {
                    path: path.clone(),
                    open: is_open,
                },
            });
            if is_open {
                walk(inner, &format!("{path}/"), depth + 1, open, out);
            }
        }
        for (name, path) in &folder.files {
            out.push(TreeRow {
                depth,
                name: name.clone(),
                kind: TreeKind::File(path.clone()),
            });
        }
    }
    let mut out = Vec::new();
    walk(&root, "", 0, &view.files.open_folders, &mut out);
    out
}

/// The most files the path picker lists.
const PICKER_ROWS: usize = 50;

/// The path picker's candidates for what was typed: the tree's files that match, best first, then
/// the typed path itself when the tree does not hold it — a git-ignored file is reachable only so,
/// and the read says whether it exists (FR-011).
pub fn picker_matches(view: &OpenView) -> Vec<RelPath> {
    let text = view.files.picker.trim();
    if text.is_empty() {
        return Vec::new();
    }
    let files = tree_files(view);
    let mut out: Vec<RelPath> = rank(files, |p| p.as_str(), &Query::new(text))
        .into_iter()
        .take(PICKER_ROWS)
        .map(|(i, _)| files[i].clone())
        .collect();
    if let Some(typed) = RelPath::from_native(text).filter(|typed| !files.contains(typed)) {
        out.push(typed);
    }
    out
}

/// Which diff an expansion belongs to (feature 1150, data-model §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceKey {
    /// The entry's own changes.
    Worktree,
    /// The pull request with this number.
    Pr(u64),
}

/// How the selected file's whole-file toggle appears (feature 1150, FR-004).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WholeFile {
    /// No toggle: no file is selected, or it is added and shows every line already.
    Hidden,
    /// Shown disabled with a reason: the file has no text version to show whole (FR-009).
    Unavailable,
    /// Offered; `on` when every line of the file is shown.
    Available {
        /// Whole-file mode is on.
        on: bool,
    },
}

/// The tooltip of a disabled whole-file toggle.
pub const WHOLE_FILE_UNAVAILABLE: &str = "No text version of this file to show";

/// The selected file and the source its expansion belongs to.
fn selected_key(view: &OpenView) -> Option<(SourceKey, RelPath)> {
    match &view.source {
        Source::Changes => view.selected.clone().map(|p| (SourceKey::Worktree, p)),
        Source::PullRequest(pr) => pr.selected.clone().map(|p| (SourceKey::Pr(pr.number), p)),
    }
}

/// The kind the shown list gives `path`: the entry's changes, or the pull request's files.
fn file_kind<'a>(view: &'a OpenView, path: &RelPath) -> Option<&'a ChangeKind> {
    let files: &[ChangedFile] = match &view.source {
        Source::Changes => &shown_list(view)?.files,
        Source::PullRequest(pr) => match &pr.diff {
            PrDiffLoad::Ready { files, .. } => files,
            _ => return None,
        },
    };
    files.iter().find(|f| &f.path == path).map(|f| &f.kind)
}

/// Whether the selected file can have hidden lines revealed: it is listed with a text version on
/// both sides, so not added, untracked or deleted (FR-009).
fn revealable(view: &OpenView, path: &RelPath) -> bool {
    file_kind(view, path).is_some_and(|kind| {
        !matches!(
            kind,
            ChangeKind::Added | ChangeKind::Untracked | ChangeKind::Deleted
        )
    })
}

/// The lines revealed ones come from: the diff's new side for the entry's changes, the pull
/// request's head version for a pull request (FR-005).
fn new_side(view: &OpenView) -> Option<&SideLines> {
    match &view.source {
        Source::Changes => match diff_body(view) {
            DiffBody::Diff(LoadedDiff { new, .. }) => new.as_ref(),
            _ => None,
        },
        Source::PullRequest(pr) => match &pr.head_text {
            HeadText::Ready(lines) => Some(lines),
            _ => None,
        },
    }
}

/// The whole-file toggle of the selected file: hidden for an added file or while a pull request's
/// head version is read, disabled when there is no text version to show, else on or off (FR-004).
pub fn whole_file(view: &OpenView) -> WholeFile {
    let Some((key, path)) = selected_key(view).filter(|_| view.browse.is_none()) else {
        return WholeFile::Hidden;
    };
    if matches!(
        file_kind(view, &path),
        Some(ChangeKind::Added | ChangeKind::Untracked)
    ) {
        return WholeFile::Hidden;
    }
    if let Source::PullRequest(pr) = &view.source {
        if matches!(pr.head_text, HeadText::NotRead | HeadText::Reading { .. })
            && matches!(
                diff_body(view),
                DiffBody::Diff(LoadedDiff {
                    diff: FileDiff::Text(_),
                    ..
                })
            )
            && revealable(view, &path)
        {
            return WholeFile::Hidden;
        }
    }
    let text = matches!(
        diff_body(view),
        DiffBody::Diff(LoadedDiff {
            diff: FileDiff::Text(_),
            ..
        })
    );
    if !(text && revealable(view, &path) && new_side(view).is_some()) {
        return WholeFile::Unavailable;
    }
    let on = view.expansions.get(&(key, path)).is_some_and(|e| e.whole);
    WholeFile::Available { on }
}

/// Recompute `view.shaped` from the diff on screen and the file's expansion, dropping open runs
/// that no longer exist (FR-008).
fn reshape(view: &mut OpenView) {
    let shaped = match (selected_key(view), diff_body(view)) {
        (
            Some((source, path)),
            DiffBody::Diff(LoadedDiff {
                diff: FileDiff::Text(hunks),
                ..
            }),
        ) => match new_side(view) {
            Some(new) if revealable(view, &path) => {
                let total = u32::try_from(new.len()).unwrap_or(u32::MAX);
                let key = (source, path);
                let current = view.expansions.get(&key).cloned().unwrap_or_default();
                let kept = current.reconcile(&gaps(hunks, total));
                let shaped = shape(hunks, new, &kept);
                if kept == Expansion::default() {
                    view.expansions.remove(&key);
                } else {
                    view.expansions.insert(key, kept);
                }
                shaped
            }
            _ => Shaped::unshaped(hunks),
        },
        _ => Shaped::default(),
    };
    view.shaped = shaped;
}

/// The hunks the shown diff displays: the selected file's shaped hunks, whichever the source.
fn shown_hunks(view: &OpenView) -> &[Hunk] {
    &view.shaped.hunks
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
    /// The scope it belongs to: the working tree, or one pull request (1149, FR-004).
    pub scope: Scope,
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
            a.scope(),
            a.path.as_str().as_bytes(),
            shown_range(view, a).start(),
            a.side,
            a.created,
        )
            .cmp(&(
                b.scope(),
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
                scope: c.scope(),
                path: c.path.clone(),
                side: c.side,
                lines,
                text: reply::shortened(&c.text, PANEL_TEXT_CHARS).0,
                status: card.status,
                reply: card.reply.map(|(text, _)| text),
                not_in_diff: match (&c.pr, &view.source) {
                    (None, _) => listed.is_some_and(|l| l.files.iter().all(|f| f.path != c.path)),
                    (Some(pr), Source::PullRequest(shown)) if pr.number == shown.number => {
                        match &shown.diff {
                            PrDiffLoad::Ready { files, .. } => {
                                files.iter().all(|f| f.path != c.path)
                            }
                            _ => false,
                        }
                    }
                    (Some(_), _) => false,
                },
                selected: view.panel.focus == Some(c.id),
                editable: card.editable,
                has_full_reply: full_reply(c).is_some(),
            }
        })
        .collect()
}

/// One group of the review panel: the working-tree comments, or one pull request's (1149, FR-004,
/// FR-013).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelGroup {
    /// The scope its rows belong to.
    pub scope: Scope,
    /// The pull request's title; `None` for the working tree, and for a pull request the list
    /// does not hold.
    pub title: Option<String>,
    /// The pull request is in the list (the working tree always is). A group that is not keeps
    /// its comments, flagged, and cannot be sent.
    pub listed: bool,
}

impl PanelGroup {
    /// The heading naming the group; `None` for the working tree. A pull request the list does
    /// not hold is flagged "not in the list" (FR-013).
    pub fn heading(&self) -> Option<String> {
        let Scope::PullRequest(number) = self.scope else {
            return None;
        };
        Some(match (&self.title, self.listed) {
            (Some(title), true) => format!("Pull request #{number}: {title}"),
            _ => format!("Pull request #{number} (not in the list)"),
        })
    }
}

/// The panel's groups, in row order: the working tree first, then pull requests by number; only
/// scopes that have a comment.
pub fn panel_groups(state: &State) -> Vec<PanelGroup> {
    let Some(view) = state.open.as_ref() else {
        return Vec::new();
    };
    let Some(review) = review_of(state, view) else {
        return Vec::new();
    };
    let scopes: BTreeSet<Scope> = review.comments.iter().map(ReviewComment::scope).collect();
    scopes
        .into_iter()
        .map(|scope| match scope {
            Scope::WorkingTree => PanelGroup {
                scope,
                title: None,
                listed: true,
            },
            Scope::PullRequest(number) => {
                let listed = listed_pull_request(view, number);
                PanelGroup {
                    scope,
                    title: listed.map(|pr| pr.title.clone()),
                    listed: listed.is_some(),
                }
            }
        })
        .collect()
}

/// The comments of `review` in `scope`.
fn in_scope(review: &ReviewView, scope: Scope) -> impl Iterator<Item = &ReviewComment> {
    review.comments.iter().filter(move |c| c.scope() == scope)
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
    let view = state.open.as_ref();
    let review = view.and_then(|view| review_of(state, view));
    let scope = view.map_or(Scope::WorkingTree, view_scope);
    for comment in review.into_iter().flat_map(|r| in_scope(r, scope)) {
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

/// The toolbar toggle's label: "Review (n)", n being every comment of the shown scope.
pub fn review_toolbar_label(state: &State) -> String {
    let count = state.open.as_ref().map_or(0, |view| {
        review_of(state, view).map_or(0, |r| in_scope(r, view_scope(view)).count())
    });
    format!("Review ({count})")
}

/// Where pressing the row of comment `id` goes; `None` for an unknown comment.
pub fn jump_target(state: &State, id: CommentId) -> Option<Jump> {
    let view = state.open.as_ref()?;
    let comment = in_scope(review_of(state, view)?, view_scope(view)).find(|c| c.id == id)?;
    let listed = match &view.source {
        Source::Changes => {
            shown_list(view).is_some_and(|l| l.files.iter().any(|f| f.path == comment.path))
        }
        Source::PullRequest(shown) => match &shown.diff {
            PrDiffLoad::Ready { files, .. } => files.iter().any(|f| f.path == comment.path),
            _ => false,
        },
    };
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
fn row_of_line(shaped: ShapedRef<'_>, layout: DiffLayout, side: Side, line: u32) -> Option<usize> {
    match layout {
        DiffLayout::Unified => unified_rows(shaped).iter().position(|row| match row {
            UnifiedRow::Line(l) => match side {
                Side::New => l.new == Some(line),
                Side::Old => l.old == Some(line),
            },
            UnifiedRow::Header(_) | UnifiedRow::Gap { .. } => false,
        }),
        DiffLayout::SideBySide => side_by_side_rows(shaped).iter().position(|row| match row {
            SideRow::Pair { left, right } => match side {
                Side::New => right.is_some_and(|c| c.number == line),
                Side::Old => left.is_some_and(|c| c.number == line),
            },
            SideRow::Header(_) | SideRow::Gap { .. } => false,
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
    if !matches!(&view.diff, Load::Ready(_)) {
        return None;
    }
    // The cards and composers hung above the line push it down by their measured heights, known
    // once the file has been laid out; a cold jump lands a little short of them.
    let first = shown_range(view, comment).start();
    let hung: u32 = view
        .slot_heights
        .iter()
        .filter(|((_, line), _)| *line < first)
        .map(|(_, height)| *height)
        .sum();
    let row = row_of_line((&view.shaped).into(), layout, comment.side, first);
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
    /// What the session area's context header follows (feature 1126, data-model *HeaderCounts*).
    pub header: HeaderCounts,
}

/// The location the session area's header is for and its change counts (feature 1126, R5, R6;
/// data-model *HeaderCounts*).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeaderCounts {
    /// The project and location the header shows; `None` while no location is shown.
    pub entry: Option<(PathBuf, SessionLocation)>,
    /// The counts of `entry`, read with the Changes view's own list reader.
    pub load: Load<ChangeSummary>,
    /// The Overview hid the header, so no watch saw its files change: the counts are read again
    /// when the header shows once more (the root's follow step sets and clears it).
    pub unwatched: bool,
}

impl HeaderCounts {
    /// What the Changes button may say: the last finished read's figure, kept while a read is
    /// repeated (#1342, US3 scenario 4); `Unknown` only before the first read finished.
    pub fn reading(&self) -> CountsReading {
        match &self.load {
            Load::Ready(summary)
            | Load::Loading {
                last: Some(summary),
                ..
            } => CountsReading::Known(*summary),
            _ => CountsReading::Unknown,
        }
    }
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
    /// The shown location changed: the header follows `entry` of `project`, or nothing (feature
    /// 1126, R6). Sent by the root's follow step, never by a control.
    HeaderFollowed {
        /// The entry's project.
        project: PathBuf,
        /// The shown location; `None` while no location is shown.
        entry: Option<SessionLocation>,
    },
    /// The header shows again after the Overview hid it: read its counts once more (feature 1126,
    /// R6). Sent by the root's follow step.
    HeaderRecount,
    /// Header counts read `seq` ended (feature 1126, R5).
    HeaderCountsRead {
        /// The read it answers.
        seq: u64,
        /// The summary of the entry's change list, or why it could not be read.
        result: Result<ChangeSummary, String>,
    },
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
    /// A hidden-lines marker was pressed: reveal that run of the selected file, or hide it again
    /// (feature 1150, FR-002, FR-003).
    GapToggled(GapKey),
    /// The selected file's whole-file toggle was pressed (feature 1150, FR-004).
    WholeFileToggled,
    /// Read `seq` of a file's text ended (feature 1150, FR-005); dropped when it is no longer the
    /// read the view waits for.
    FileRead {
        /// The read it answers.
        seq: u64,
        /// The source it was asked of.
        source: SourceKey,
        /// The file.
        path: RelPath,
        /// The text, or why there is none.
        result: Result<FileText, FileReadFailure>,
        /// The text's syntax colours, when the shell coloured it (a browsed file); none otherwise.
        spans: Spans,
    },
    /// A file of the repository was picked in the tree or the path picker (feature 1150, FR-011).
    BrowseOpen(RelPath),
    /// A reference in a line of the diff, or of a browsed file, was activated (FR-012): the file it
    /// names, and the line the text gives.
    ReferenceActivated(RelPath, Option<u32>),
    /// Back from a browsed file (FR-013).
    BrowseBack,
    /// "Open diff" on a browsed file the change lists (FR-015).
    OpenFileDiff,
    /// The path picker's text changed.
    PathPickerChanged(String),
    /// The list pane's "Files" toggle was pressed: the repository's tree in place of the changed
    /// files, or back.
    FilesPaneToggled,
    /// A folder of the tree was opened or closed.
    FolderToggled(String),
    /// Tree read `seq` ended.
    TreeRead {
        /// The read it answers.
        seq: u64,
        /// The files, or why there are none.
        result: Result<TreeListing, FileReadFailure>,
    },
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
    /// Several effects, run together.
    Batch(Vec<Effect>),
    /// Read the change list of the header's `entry` under the default toggles as counts read
    /// `seq`, and answer its summary (feature 1126, R5).
    ReadCounts {
        /// The read's sequence number.
        seq: u64,
        /// The entry.
        entry: SessionLocation,
    },
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
    /// Read one file's text at a pull request's head as read `seq` (feature 1150, FR-005): the
    /// shell tries the entry's local objects, then GitHub through `address`'s repository.
    ReadFile {
        /// The read's sequence number.
        seq: u64,
        /// The project.
        project: PathBuf,
        /// The entry whose directory holds the objects.
        entry: SessionLocation,
        /// Whose version: a pull request's, or the worktree's own (a browsed file; `commit` and
        /// `address` are then empty).
        source: SourceKey,
        /// The head commit.
        commit: String,
        /// The pull request's address, which names its repository.
        address: String,
        /// The file.
        path: RelPath,
        /// Read it even over the size limit.
        force_large: bool,
    },
    /// List the repository's files for the entry as read `seq` (feature 1150, FR-011).
    ReadTree {
        /// The read's sequence number.
        seq: u64,
        /// The project.
        project: PathBuf,
        /// The entry whose directory is listed.
        entry: SessionLocation,
        /// Whose files: a pull request's head, or the worktree's own (`commit` and `address` are
        /// then empty).
        source: SourceKey,
        /// The head commit.
        commit: String,
        /// The pull request's address, which names its repository.
        address: String,
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
        /// The shown pull request, when the send is of its comments (feature 1149, W20); `None`
        /// sends the working tree's.
        pr: Option<PrSend>,
    },
}

/// Apply `msg` (contracts/changes-view.md V1–V3, L1).
pub fn update(state: &mut State, msg: Msg) -> Effect {
    // The header's follow touches nothing of the open view: not its notice, not its reads.
    match msg {
        Msg::HeaderFollowed { project, entry } => return header_followed(state, project, entry),
        Msg::HeaderCountsRead { seq, result } => return header_counts_read(state, seq, result),
        Msg::HeaderRecount => return request_counts(state),
        _ => {}
    }
    let scroll = matches!(
        msg,
        Msg::DiffScrolled { .. } | Msg::ListScrolled { .. } | Msg::PanelScrolled { .. }
    );
    let before = state.open.as_ref().filter(|_| !scroll).map(|view| {
        (
            view.project.clone(),
            view.entry.clone(),
            read_target(view),
            view.browse.as_ref().map(|b| b.path.clone()),
        )
    });
    let wants_tree = matches!(msg, Msg::DiffRead { .. } | Msg::PullRequestDiffRead { .. });
    if !scroll && !matches!(msg, Msg::ReferenceActivated(..)) {
        if let Some(view) = state.open.as_mut() {
            view.notice = None;
        }
    }
    let mut effect = apply(state, msg);
    let mut extras = reread_after_source_change(state, before).unwrap_or_default();
    if wants_tree {
        extras.extend(tree_for_references(state));
    }
    if !extras.is_empty() {
        let extra = extras;
        effect = match effect {
            Effect::None => Effect::Batch(extra),
            first => Effect::Batch(std::iter::once(first).chain(extra).collect()),
        };
    }
    if !scroll {
        if let Some(view) = state.open.as_mut() {
            reshape(view);
            // R2: a pick stays only while its lines are still rows of the diff (a collapsed
            // revealed run takes its rows away too).
            if let Some(pick) = view.pick {
                let rows = diff_rows(view);
                let shown = |line| rows.contains(&(pick.side, line));
                if !(shown(pick.anchor) && shown(pick.head)) {
                    view.pick = None;
                }
            }
        }
    }
    effect
}

/// What `read_target` and the browsed path were before a message, for [`reread_after_source_change`].
type SourceBefore = (
    PathBuf,
    SessionLocation,
    (SourceKey, String, String),
    Option<RelPath>,
);

/// The shown source changed under the same entry (another pull request, the worktree, or the
/// pull request's head moved): the files tree and the browsed path were of the old version, so
/// read them again at the new one. A path the new version lacks then says so (FR-009, FR-014);
/// where Back led is forgotten with the old source ([`reset_diff_pane`]).
fn reread_after_source_change(
    state: &mut State,
    before: Option<SourceBefore>,
) -> Option<Vec<Effect>> {
    let (project, entry, target, browsed) = before?;
    let view = state.open.as_mut()?;
    if view.project != project || view.entry != entry || read_target(view) == target {
        return None;
    }
    let mut effects = Vec::new();
    if let Some(path) = browsed {
        let force = view.shown_large.contains(&path);
        effects.push(request_browse(view, &mut state.next_seq, path, force));
    }
    // The old source's files are not the new one's, shown or not.
    view.files.tree = Load::Idle;
    view.files.listed.clear();
    view.files.truncated = false;
    if view.files.shown {
        effects.push(request_tree(view, &mut state.next_seq));
    }
    (!effects.is_empty()).then_some(effects)
}

/// The header follows `entry` of `project` (data-model *HeaderCounts* transitions): a new entry
/// is read at once; the same one is left as it is.
fn header_followed(state: &mut State, project: PathBuf, entry: Option<SessionLocation>) -> Effect {
    let entry = entry.map(|entry| (project, entry));
    if entry == state.header.entry {
        return Effect::None;
    }
    state.header.entry = entry;
    state.header.load = Load::Idle;
    request_counts(state)
}

/// Header counts read `seq` ended: kept only while it is the current read; a read asked for
/// meanwhile runs once more after it, as the view's list read does.
fn header_counts_read(
    state: &mut State,
    seq: u64,
    result: Result<ChangeSummary, String>,
) -> Effect {
    let Load::Loading {
        seq: current,
        again,
        last,
    } = state.header.load
    else {
        return Effect::None;
    };
    if current != seq {
        return Effect::None;
    }
    // A failed re-read keeps the last figure rather than blanking the label (#1342).
    state.header.load = match (result, last) {
        (Ok(summary), _) | (Err(_), Some(summary)) => Load::Ready(summary),
        (Err(why), None) => Load::Failed(why),
    };
    if again {
        request_counts(state)
    } else {
        Effect::None
    }
}

/// Read the header's counts again (a new entry, or the worktree changed); while a read is under
/// way, queue one more instead. `Effect::None` without an entry.
fn request_counts(state: &mut State) -> Effect {
    let Some((_, entry)) = state.header.entry.as_ref() else {
        return Effect::None;
    };
    if let Load::Loading { again, .. } = &mut state.header.load {
        *again = true;
        return Effect::None;
    }
    let last = match std::mem::take(&mut state.header.load) {
        Load::Ready(summary) => Some(summary),
        _ => None,
    };
    let seq = state.next_seq;
    state.next_seq += 1;
    state.header.load = Load::Loading {
        seq,
        again: false,
        last,
    };
    Effect::ReadCounts {
        seq,
        entry: entry.clone(),
    }
}

fn apply(state: &mut State, msg: Msg) -> Effect {
    match msg {
        // Handled in `update` before the view's bookkeeping.
        Msg::HeaderFollowed { .. } | Msg::HeaderCountsRead { .. } | Msg::HeaderRecount => {
            Effect::None
        }
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
                expansions: BTreeMap::new(),
                shaped: Shaped::default(),
                browse: None,
                stack: BrowseStack::default(),
                files: FilesPane::default(),
                notice: None,
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
                // Every failure kind, no GitHub access included, keeps what was listed (FR-008,
                // FR-015); with nothing listed the section shows its muted line (FR-011), and a
                // pull request shown is never outside every list.
                Err(_) => {
                    if last.is_none() {
                        view.source = Source::Changes;
                    }
                    PrList::Unavailable { last }
                }
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
            if pr.number != number {
                return Effect::None;
            }
            if pr.refresh.as_ref().is_some_and(|(s, _)| *s == seq) {
                let head = pr.refresh.take().map(|(_, head)| head).unwrap_or_default();
                // A failed re-read keeps the diff on screen; the next list read asks again.
                let Ok(diff) = result else {
                    return Effect::None;
                };
                pr.diff = pr_diff_ready(diff);
                pr.head = head;
                let dropped = pr.selected.as_ref().is_some_and(|p| !pr_lists(pr, p));
                if dropped {
                    pr.selected = None;
                }
                pr.file = pr_file_diff(pr);
                if dropped {
                    reset_diff_pane(view);
                }
            } else if pr.diff == (PrDiffLoad::Loading { seq }) {
                pr.diff = match result {
                    Ok(diff) => pr_diff_ready(diff),
                    Err(DiffReadFailure::TooLarge) => PrDiffLoad::TooLarge,
                    Err(DiffReadFailure::Failed(_)) => PrDiffLoad::Failed,
                };
            } else {
                return Effect::None;
            }
            let moves = judge_anchors(state);
            reanchor_effect(state, moves)
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
            // Choosing a changed file leaves the browser and forgets where Back led.
            let was_browsing = view.browse.take().is_some();
            view.stack.clear();
            if was_browsing {
                view.diff_offset = 0;
            }
            if let Source::PullRequest(pr) = &mut view.source {
                // A pull request's patches are held: choosing a file parses one, reads nothing.
                pr.selected = Some(path);
                pr.file = pr_file_diff(pr);
                pr.head_text = HeadText::NotRead;
                reset_diff_pane(view);
                return request_head(view, &mut state.next_seq, false);
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
            // A browsed file over the limit is read despite it.
            if let Some(BrowsedFile {
                path,
                load: BrowseLoad::Failed(FileReadFailure::TooLarge { .. }),
                ..
            }) = view.browse.clone()
            {
                view.shown_large.insert(path.clone());
                return request_browse(view, &mut state.next_seq, path, true);
            }
            if let Source::PullRequest(pr) = &mut view.source {
                if let Some(path) = pr.selected.clone() {
                    pr.shown_large.insert(path);
                    pr.file = pr_file_diff(pr);
                }
                // Asked for the large file: its head version is read despite the size limit.
                return request_head(view, &mut state.next_seq, true);
            }
            let Some(path) = view.selected.clone() else {
                return Effect::None;
            };
            view.shown_large.insert(path);
            request_diff(state, false)
        }
        Msg::BrowseOpen(path) => browse_open(state, path, None),
        Msg::ReferenceActivated(path, line) => reference_activated(state, path, line),
        Msg::BrowseBack => browse_back(state),
        Msg::OpenFileDiff => {
            let Some(view) = state.open.as_ref().filter(|v| file_diff_available(v)) else {
                return Effect::None;
            };
            let path = view.browse.as_ref().map(|b| b.path.clone());
            match path.map(|path| update(state, Msg::FileSelected(path))) {
                Some(Effect::None) => Effect::ScrollDiff { offset: 0 },
                Some(effect) => effect,
                None => Effect::None,
            }
        }
        Msg::PathPickerChanged(text) => {
            if let Some(view) = state.open.as_mut() {
                view.files.picker = text;
            }
            Effect::None
        }
        Msg::FolderToggled(folder) => {
            if let Some(view) = state.open.as_mut() {
                if !view.files.open_folders.remove(&folder) {
                    view.files.open_folders.insert(folder);
                }
            }
            Effect::None
        }
        Msg::FilesPaneToggled => {
            let Some(view) = state.open.as_mut() else {
                return Effect::None;
            };
            view.files.shown = !view.files.shown;
            if !view.files.shown {
                return Effect::None;
            }
            request_tree(view, &mut state.next_seq)
        }
        Msg::TreeRead { seq, result } => {
            if let Some(view) = state.open.as_mut() {
                if matches!(view.files.tree, Load::Loading { seq: s, .. } if s == seq) {
                    (view.files.truncated, view.files.tree) = match result {
                        Ok(listing) => {
                            view.files.listed = listing
                                .paths
                                .iter()
                                .map(|p| p.as_str().to_owned())
                                .collect();
                            (listing.truncated, Load::Ready(listing.paths))
                        }
                        Err(why) => (false, Load::Failed(why.message())),
                    };
                }
            }
            Effect::None
        }
        Msg::GapToggled(key) => {
            if let Some(view) = state.open.as_mut().filter(|v| v.browse.is_none()) {
                if let Some(at) = selected_key(view).filter(|(_, p)| revealable(view, p)) {
                    if new_side(view).is_some() {
                        view.expansions.entry(at).or_default().toggle_key(key);
                    }
                }
            }
            Effect::None
        }
        Msg::WholeFileToggled => {
            if let Some(view) = state.open.as_mut().filter(|v| v.browse.is_none()) {
                if let (WholeFile::Available { on }, Some(at)) =
                    (whole_file(view), selected_key(view))
                {
                    view.expansions.entry(at).or_default().set_whole(!on);
                }
            }
            Effect::None
        }
        Msg::FileRead {
            seq,
            source,
            path,
            result,
            spans,
        } => {
            if let Some(view) = state.open.as_mut() {
                let waiting = view
                    .browse
                    .as_ref()
                    .is_some_and(|b| b.path == path && b.load == (BrowseLoad::Reading { seq }));
                if waiting && source == read_target(view).0 {
                    let failed_read = result.is_err();
                    let load = match result {
                        Ok(text) => BrowseLoad::Ready(LoadedDiff {
                            diff: plain_file(&text.lines),
                            old: None,
                            new: Some(text.lines),
                            spans,
                        }),
                        Err(why) => BrowseLoad::Failed(why),
                    };
                    if let Some(browsed) = view.browse.as_mut() {
                        browsed.load = load;
                        if let Some(line) = browsed.line.take().filter(|_| !failed_read) {
                            view.diff_offset = offset_of_line(line);
                        }
                    }
                    return Effect::ScrollDiff {
                        offset: view.diff_offset,
                    };
                }
            }
            if let Some(Source::PullRequest(pr)) = state.open.as_mut().map(|v| &mut v.source) {
                let current = source == SourceKey::Pr(pr.number)
                    && pr.selected.as_ref() == Some(&path)
                    && pr.head_text == (HeadText::Reading { seq });
                if current {
                    pr.head_text = match result {
                        Ok(text) => HeadText::Ready(text.lines),
                        Err(why) => HeadText::Failed(why),
                    };
                }
            }
            Effect::None
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
                    reshape(view);
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
            // Feature 1126: the header's worktree gone, it follows nothing until the root's follow
            // step finds what is shown now.
            if matches!(
                state.header.entry.as_ref().map(|(_, entry)| entry),
                Some(SessionLocation::Worktree(name)) if !names.contains(name)
            ) {
                state.header = HeaderCounts::default();
            }
            Effect::None
        }
        Msg::GutterPressed { old, new, extend } => {
            if let Some(view) = state.open.as_mut().filter(|v| pick_allowed(v)) {
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
        // The watch's wake rereads the open view's list and the header's counts (R5, R6).
        Msg::Changed => {
            let list = request_read(state);
            match (list, request_counts(state)) {
                (list, Effect::None) => list,
                (Effect::None, counts) => counts,
                (list, counts) => Effect::Batch(vec![list, counts]),
            }
        }
        Msg::SendPressed => {
            let Some(view) = state.open.as_ref() else {
                return Effect::None;
            };
            // Only a listed pull request can be sent (FR-013).
            let scope = view_scope(view);
            let Some(pr) = send_target(view, scope) else {
                return Effect::None;
            };
            let count = review_of(state, view)
                .filter(|r| !r.sending)
                .map_or(0, |r| in_scope(r, scope).filter(|c| is_pending(c)).count());
            if count == 0 {
                return Effect::None;
            }
            // The removed side of a working-tree comment is judged here; the service judges the
            // rest, but not a pull request's comments, which it never reads off disk.
            let outdated = review_of(state, view)
                .map(|r| {
                    in_scope(r, scope)
                        .filter(|c| {
                            (scope != Scope::WorkingTree || c.side == Side::Old)
                                && is_pending(c)
                                && is_outdated(view, c)
                        })
                        .map(|c| c.id)
                        .collect()
                })
                .unwrap_or_default();
            Effect::ReviewSend {
                project: view.project.clone(),
                worktree_dir: entry_dir(&view.entry),
                outdated,
                count,
                pr,
            }
        }
        Msg::ClearSentPressed => match state.open.as_ref() {
            Some(view) if tidy_actions(state).is_some_and(|a| a.clear_sent) => review_edit(
                view,
                ReviewEditOp::ClearSent {
                    scope: view_scope(view),
                },
            ),
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
                        review_edit(
                            view,
                            ReviewEditOp::DiscardPending {
                                scope: view_scope(view),
                            },
                        )
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
            let range = pick.range();
            let (path, quote, pr) = match &view.source {
                Source::Changes => {
                    let (Some(path), Load::Ready(loaded)) = (view.selected.as_ref(), &view.diff)
                    else {
                        return Effect::None;
                    };
                    let lines = match pick.side {
                        Side::New => loaded.new.as_ref(),
                        Side::Old => loaded.old.as_ref(),
                    };
                    let quote: Option<Vec<String>> = (range.start()..=range.end())
                        .map(|n| lines.and_then(|l| l.line(n)).map(str::to_string))
                        .collect();
                    let Some(quote) = quote else {
                        return Effect::None;
                    };
                    (path.clone(), quote, None)
                }
                Source::PullRequest(shown) => {
                    let (Some(path), DiffBody::Diff(loaded)) =
                        (shown.selected.as_ref(), diff_body(view))
                    else {
                        return Effect::None;
                    };
                    let FileDiff::Text(hunks) = &loaded.diff else {
                        return Effect::None;
                    };
                    let Some(quote) = pr_quote(hunks, pick.side, range) else {
                        return Effect::None;
                    };
                    let Some(listed) =
                        listed_pull_request(view, shown.number).filter(|_| !shown.head.is_empty())
                    else {
                        return Effect::None;
                    };
                    let anchor = PrAnchor {
                        number: shown.number,
                        head_branch: listed.head_branch.clone(),
                        head_commit: shown.head.clone(),
                    };
                    (path.clone(), quote, Some(anchor))
                }
            };
            view.pick = None;
            ReviewEditOp::Add {
                path: path.as_str().to_string(),
                side: pick.side,
                start: range.start(),
                end: range.end(),
                quote,
                text,
                pr,
            }
        }
    };
    view.composer = None;
    review_edit(view, edit)
}

/// The text of lines `range` of `side` in a pull request file's `hunks`: a pull request's diff
/// carries no whole-file lines, so the quote is what the hunks show. `None` when a line is not in
/// a hunk.
fn pr_quote(hunks: &[Hunk], side: Side, range: LineRange) -> Option<Vec<String>> {
    (range.start()..=range.end())
        .map(|n| {
            hunks
                .iter()
                .flat_map(|h| &h.lines)
                .find(|line| match side {
                    Side::New => line.new == Some(n),
                    Side::Old => line.old == Some(n),
                })
                .map(|line| line.text.clone())
        })
        .collect()
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

/// What a send of `scope` carries as its pull request (1149, W20): `Some(None)` for the working
/// tree, `Some(Some(..))` from the list's pull request, `None` when the pull request is not listed.
fn send_target(view: &OpenView, scope: Scope) -> Option<Option<PrSend>> {
    let Scope::PullRequest(number) = scope else {
        return Some(None);
    };
    let listed = listed_pull_request(view, number).filter(|pr| !pr.head.is_empty())?;
    Some(Some(PrSend {
        number,
        title: listed.title.clone(),
        head_branch: listed.head_branch.clone(),
        state: listed.state,
        current_head: listed.head.clone(),
    }))
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
    // A pull request that is not listed cannot be sent (FR-013).
    let scope = view_scope(view);
    send_target(view, scope)?;
    let review = review_of(state, view);
    if review.is_some_and(|r| r.sending) {
        return Some(SendAction {
            label: "Sending…".into(),
            enabled: false,
        });
    }
    let count = review.map_or(0, |r| in_scope(r, scope).filter(|c| is_pending(c)).count());
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
    let scope = view_scope(view);
    let any = |pending: bool| {
        review.is_some_and(|r| in_scope(r, scope).any(|c| is_pending(c) == pending))
    };
    Some(TidyActions {
        clear_sent: any(false),
        discard_pending: any(true) && !review.is_some_and(|r| r.sending),
    })
}

/// The discard confirmation's question while it is open (S3): "Discard n draft comments? This
/// cannot be undone."
pub fn discard_prompt(state: &State) -> Option<String> {
    let view = state.open.as_ref().filter(|view| view.confirm_discard)?;
    let scope = view_scope(view);
    let count =
        review_of(state, view).map_or(0, |r| in_scope(r, scope).filter(|c| is_pending(c)).count());
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
    let view = state.open.as_ref();
    let review = view.and_then(|view| review_of(state, view));
    let scope = view.map_or(Scope::WorkingTree, view_scope);
    for comment in review.into_iter().flat_map(|r| in_scope(r, scope)) {
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
    let (Some(path), Some(review)) = (selected_file(view), review_of(state, view)) else {
        return placed;
    };
    let hunks = shown_hunks(view);
    for comment in in_scope(review, view_scope(view)).filter(|c| &c.path == path) {
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

/// The ready state of a pull request's diff read: its rows and each file's patch.
fn pr_diff_ready(diff: PrDiff) -> PrDiffLoad {
    let mut files = Vec::with_capacity(diff.files.len());
    let mut patches = BTreeMap::new();
    for file in diff.files {
        patches.insert(file.file.path.clone(), file.patch);
        files.push(file.file);
    }
    PrDiffLoad::Ready { files, patches }
}

/// Whether the read diff of `pr` lists `path`.
fn pr_lists(pr: &PullRequestView, path: &RelPath) -> bool {
    matches!(&pr.diff, PrDiffLoad::Ready { files, .. } if files.iter().any(|f| &f.path == path))
}

/// The shown pull request's head moved in the list (a push): read its diff again, keeping the
/// selected file and the diff on screen meanwhile (1149 FR-012). Nothing while no diff is ready,
/// or a read of that head is already under way.
fn follow_head(state: &mut State) -> Option<Effect> {
    let seq = state.next_seq;
    let view = state.open.as_mut()?;
    let Source::PullRequest(pr) = &view.source else {
        return None;
    };
    let number = pr.number;
    let listed = listed_pull_request(view, number)?.head.clone();
    if listed.is_empty()
        || listed == pr.head
        || !matches!(pr.diff, PrDiffLoad::Ready { .. })
        || pr.refresh.as_ref().is_some_and(|(_, head)| *head == listed)
    {
        return None;
    }
    let Source::PullRequest(pr) = &mut view.source else {
        return None;
    };
    pr.refresh = Some((seq, listed));
    state.next_seq += 1;
    Some(Effect::ReadPullRequestDiff {
        seq,
        project: view.project.clone(),
        number,
    })
}

/// Whose version of the repository's files is read: the pull request's head (its commit and the
/// address that names its repository, empty when the list holds neither), or the worktree's own.
fn read_target(view: &OpenView) -> (SourceKey, String, String) {
    match &view.source {
        Source::Changes => (SourceKey::Worktree, String::new(), String::new()),
        Source::PullRequest(pr) => (
            SourceKey::Pr(pr.number),
            pr.head.clone(),
            listed_pull_request(view, pr.number)
                .map(|listed| listed.url.clone())
                .unwrap_or_default(),
        ),
    }
}

/// Read the repository's files again, keeping the list on screen meanwhile.
fn request_tree(view: &mut OpenView, next_seq: &mut u64) -> Effect {
    let seq = *next_seq;
    *next_seq += 1;
    let last = match std::mem::take(&mut view.files.tree) {
        Load::Ready(files) => Some(files),
        Load::Loading { last, .. } => last,
        _ => None,
    };
    view.files.tree = Load::Loading {
        seq,
        again: false,
        last,
    };
    let (source, commit, address) = read_target(view);
    Effect::ReadTree {
        seq,
        project: view.project.clone(),
        entry: view.entry.clone(),
        source,
        commit,
        address,
    }
}

/// After a change, the worktree's files may differ: read the browsed file and the shown tree again
/// (FR-014: the worktree's current version).
fn refresh_browsed(state: &mut State) -> Vec<Effect> {
    let Some(view) = state.open.as_mut().filter(|v| v.source == Source::Changes) else {
        return Vec::new();
    };
    let mut effects = Vec::new();
    if let Some(path) = view.browse.as_ref().map(|b| b.path.clone()) {
        let force = view.shown_large.contains(&path);
        effects.push(request_browse(view, &mut state.next_seq, path, force));
    }
    if view.files.shown {
        effects.push(request_tree(view, &mut state.next_seq));
    }
    effects
}

/// Read `path` at the shown source's version (the worktree's, or the pull request's head) to show
/// it in place of the diff.
fn request_browse(
    view: &mut OpenView,
    next_seq: &mut u64,
    path: RelPath,
    force_large: bool,
) -> Effect {
    let seq = *next_seq;
    *next_seq += 1;
    view.browse = Some(BrowsedFile {
        path: path.clone(),
        load: BrowseLoad::Reading { seq },
        line: None,
    });
    let (source, commit, address) = read_target(view);
    Effect::ReadFile {
        seq,
        project: view.project.clone(),
        entry: view.entry.clone(),
        source,
        commit,
        address,
        path,
        force_large,
    }
}

/// Open `path` in the browser, remembering what was on screen as the place Back returns to.
fn browse_open(state: &mut State, path: RelPath, line: Option<u32>) -> Effect {
    let Some(view) = state.open.as_mut() else {
        return Effect::None;
    };
    let shown = view
        .browse
        .as_ref()
        .is_some_and(|b| b.path == path && !matches!(b.load, BrowseLoad::Failed(_)));
    if shown {
        // Followed again to a line of the file already shown: scroll to it, or once it is read.
        let Some((line, browsed)) = line.zip(view.browse.as_mut()) else {
            return Effect::None;
        };
        if matches!(browsed.load, BrowseLoad::Ready(_)) {
            view.diff_offset = offset_of_line(line);
            return Effect::ScrollDiff {
                offset: view.diff_offset,
            };
        }
        browsed.line = Some(line);
        return Effect::None;
    }
    let frame = match &view.browse {
        Some(b) => Frame::File {
            path: b.path.clone(),
            offset: view.diff_offset,
        },
        None => Frame::Diff {
            file: selected_file(view).cloned(),
            offset: view.diff_offset,
        },
    };
    view.stack.push(frame);
    view.diff_offset = 0;
    view.files.picker.clear();
    let force = view.shown_large.contains(&path);
    let effect = request_browse(view, &mut state.next_seq, path, force);
    if let Some(browsed) = view.browse.as_mut() {
        browsed.line = line;
    }
    effect
}

/// The scroll offset that puts line `line` of a browsed file near the top: the file is one hunk, so
/// its header is row 0 and line `n` row `n` (as [`jump_offset`] leaves two rows above a line).
fn offset_of_line(line: u32) -> u32 {
    (line.saturating_sub(2) as f32 * crate::ui::material::DIFF_ROW_HEIGHT) as u32
}

/// Follow a reference: open the file it names at its line, when the repository lists it; else say
/// so and change nothing (FR-012).
fn reference_activated(state: &mut State, path: RelPath, line: Option<u32>) -> Effect {
    let Some(view) = state.open.as_mut() else {
        return Effect::None;
    };
    if !view.files.listed.contains(path.as_str()) {
        view.notice = Some(NO_SUCH_FILE);
        return Effect::None;
    }
    browse_open(state, path, line)
}

/// What a followed reference to an unlisted file says.
pub const NO_SUCH_FILE: &str = "That file is not in the repository.";

/// The references in `text`, a line of the file the pane shows: only strings that name a file the
/// tree lists (FR-012). None until the tree is read.
pub fn references_in(view: &OpenView, text: &str) -> Vec<Reference> {
    if view.files.listed.is_empty() {
        return Vec::new();
    }
    let from = view
        .browse
        .as_ref()
        .map(|b| &b.path)
        .or_else(|| selected_file(view))
        .map_or("", RelPath::as_str);
    reference::find(text, from, &|p| view.files.listed.contains(p))
}

/// Read the repository's files once a diff is shown, so its lines can link to them.
fn tree_for_references(state: &mut State) -> Option<Effect> {
    let view = state.open.as_mut()?;
    matches!(view.files.tree, Load::Idle).then(|| request_tree(view, &mut state.next_seq))
}

/// Return to the place the browser was entered from: the diff with its file, scroll and
/// expansion (kept untouched meanwhile, and re-shaped against any refreshed diff), or the browsed
/// file that was left.
fn browse_back(state: &mut State) -> Effect {
    let Some(view) = state.open.as_mut() else {
        return Effect::None;
    };
    match view.stack.back() {
        None => Effect::None,
        Some(Frame::Diff { file, offset }) => {
            view.browse = None;
            // A file the change no longer lists leaves nothing to keep the place of.
            let offset = if selected_file(view) == file.as_ref() {
                offset
            } else {
                0
            };
            view.diff_offset = offset;
            Effect::ScrollDiff { offset }
        }
        Some(Frame::File { path, offset }) => {
            let force = view.shown_large.contains(&path);
            let effect = request_browse(view, &mut state.next_seq, path, force);
            view.diff_offset = offset;
            effect
        }
    }
}

/// Ask for the head version of the selected pull request file when it has one to show: a text
/// file that is neither added nor deleted (FR-009). A pull request the list does not hold has no
/// head commit to ask for, which the file's note says.
fn request_head(view: &mut OpenView, next_seq: &mut u64, force_large: bool) -> Effect {
    let Source::PullRequest(pr) = &view.source else {
        return Effect::None;
    };
    let (
        Some(path),
        Some(LoadedDiff {
            diff: FileDiff::Text(_),
            ..
        }),
    ) = (pr.selected.clone(), &pr.file)
    else {
        return Effect::None;
    };
    if !revealable(view, &path) {
        if let Source::PullRequest(pr) = &mut view.source {
            pr.head_text = HeadText::NotRead;
        }
        return Effect::None;
    }
    let number = pr.number;
    let commit = pr.head.clone();
    let address = listed_pull_request(view, number).map(|listed| listed.url.clone());
    let (project, entry) = (view.project.clone(), view.entry.clone());
    let Source::PullRequest(pr) = &mut view.source else {
        return Effect::None;
    };
    let Some(address) = address.filter(|_| !commit.is_empty()) else {
        pr.head_text = HeadText::Failed(FileReadFailure::HeadUnavailable);
        return Effect::None;
    };
    let seq = *next_seq;
    *next_seq += 1;
    pr.head_text = HeadText::Reading { seq };
    Effect::ReadFile {
        seq,
        project,
        entry,
        source: SourceKey::Pr(number),
        commit,
        address,
        path,
        force_large,
    }
}

/// What a change of source resets in the diff pane: picks, the composer, scroll and slot heights
/// belong to the file that was shown.
fn reset_diff_pane(view: &mut OpenView) {
    close_new(view);
    view.browse = None;
    view.stack.clear();
    view.diff_offset = 0;
    view.slot_heights.clear();
}

/// The listed pull request `number`, from the list the view shows.
fn listed_pull_request(view: &OpenView, number: u64) -> Option<&BranchPullRequest> {
    view.pull_requests
        .shown()?
        .0
        .iter()
        .find(|pr| pr.number == number)
}

/// The comments' scope of what `view` shows: the pull request's, or the working tree's (1149).
pub fn view_scope(view: &OpenView) -> Scope {
    match &view.source {
        Source::Changes => Scope::WorkingTree,
        Source::PullRequest(pr) => Scope::PullRequest(pr.number),
    }
}

/// Show pull request `number`: ask for its diff, replacing whatever source was shown.
fn read_pull_request_diff(state: &mut State, number: u64) -> Effect {
    let seq = state.next_seq;
    let Some(view) = state.open.as_mut() else {
        return Effect::None;
    };
    let head = listed_pull_request(view, number)
        .map(|pr| pr.head.clone())
        .unwrap_or_default();
    view.source = Source::PullRequest(Box::new(PullRequestView {
        number,
        head,
        refresh: None,
        diff: PrDiffLoad::Loading { seq },
        selected: None,
        file: None,
        shown_large: BTreeSet::new(),
        head_text: HeadText::NotRead,
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
        Source::PullRequest(_) => (!again).then(|| follow_head(state)).flatten(),
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
        let diff = request_diff(state, true);
        let mut effects = refresh_browsed(state);
        if effects.is_empty() {
            return diff;
        }
        effects.insert(0, diff);
        Effect::Batch(effects)
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
    let mut rows = BTreeSet::new();
    for line in shown_hunks(view).iter().flat_map(|h| &h.lines) {
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
    let shown_pr = match &view.source {
        Source::PullRequest(pr) => Some(pr.as_ref()),
        Source::Changes => None,
    };
    for comment in comments {
        if comment.pr.is_some() {
            // A pull request comment is judged against the shown pull request's patches; any
            // other keeps its last anchor.
            let held = anchors.get(&comment.id).map_or(comment.range, |a| a.shown);
            let judged = shown_pr.and_then(|pr| judge_pr_comment(pr, comment, held, &mut moves));
            anchors.insert(
                comment.id,
                judged.unwrap_or(Anchor {
                    shown: held,
                    outdated: anchors.get(&comment.id).is_some_and(|a| a.outdated),
                }),
            );
            continue;
        }
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

/// Where pull request comment `comment` is on `pr`'s diff (1149 R4), located in the lines of its
/// file's hunks; a move of a new-side comment is pushed to `moves`. `None` when the diff is not
/// ready, does not hold the comment's pull request, or the file waits for **Show diff**.
fn judge_pr_comment(
    pr: &PullRequestView,
    comment: &ReviewComment,
    held: LineRange,
    moves: &mut Vec<Move>,
) -> Option<Anchor> {
    let PrDiffLoad::Ready { files, patches } = &pr.diff else {
        return None;
    };
    if comment.pr.as_ref().map(|p| p.number) != Some(pr.number) {
        return None;
    }
    let status = comment.state.status();
    let pending = matches!(status, Status::Draft | Status::Sent);
    let Some(file) = files.iter().find(|f| f.path == comment.path) else {
        return Some(Anchor {
            shown: held,
            outdated: pending,
        });
    };
    if file.large && !pr.shown_large.contains(&comment.path) {
        return None;
    }
    let diff = parse_unified(patches.get(&comment.path).map_or(&[][..], Vec::as_slice));
    let lines = HunkLines::new(&diff, comment.side);
    let located = locate(&comment.quote, comment.range, &lines);
    if let (Located::Moved(to), Side::New) = (&located, comment.side) {
        moves.extend(Move::new(comment.id, comment.range, *to));
    }
    Some(Anchor {
        shown: located.shown(comment.range, anchor::Lines::len(&lines)),
        outdated: anchor::is_outdated(&located, status),
    })
}

/// The `Reanchor` edit for `moves` (research R3): a working-tree comment's only while the shown
/// new side is the working tree (the Default entry, or the Uncommitted toggle on); a pull request
/// comment's always, since its diff is the one it was judged on (1149 W21).
fn reanchor_effect(state: &State, moves: Vec<Move>) -> Effect {
    let Some(view) = state.open.as_ref() else {
        return Effect::None;
    };
    let working_tree = view.source == Source::Changes
        && (view.entry == SessionLocation::Default || view.toggles.uncommitted);
    let comments = review_of(state, view).map_or(&[][..], |r| r.comments.as_slice());
    let moves: Vec<Move> = moves
        .into_iter()
        .filter(|m| working_tree || comments.iter().any(|c| c.id == m.id() && c.pr.is_some()))
        .collect();
    if moves.is_empty() {
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

/// The one gate on picking (R7, FR-006): whether lines of `file` can be picked for a comment. Only
/// the shown file's diff rows can, and they can where [`can_pick`] allows; revealed lines are
/// context rows of that diff, so they follow it. Adds no rule of its own: a file the change does
/// not touch is not pickable until #1149 specifies an anchor for it.
pub fn pickable(view: &OpenView, file: &RelPath) -> bool {
    view.browse.is_none()
        && selected_key(view).is_some_and(|(_, shown)| &shown == file)
        && can_pick(view)
}

/// [`pickable`] for the file the view shows: the check the pick handling and the diff pane make.
pub fn pick_allowed(view: &OpenView) -> bool {
    selected_key(view).is_some_and(|(_, file)| pickable(view, &file))
}

/// Whether lines of the shown diff can be picked for a comment: text only (D3).
pub fn can_pick(view: &OpenView) -> bool {
    match &view.source {
        Source::Changes => matches!(
            &view.diff,
            Load::Ready(LoadedDiff {
                diff: FileDiff::Text(_),
                ..
            })
        ),
        // A pull request's text file once its diff is read (1149, FR-001, FR-014).
        Source::PullRequest(_) => matches!(
            diff_body(view),
            DiffBody::Diff(LoadedDiff {
                diff: FileDiff::Text(_),
                ..
            })
        ),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> PathBuf {
        PathBuf::from("/repo")
    }

    fn follow(state: &mut State, entry: Option<SessionLocation>) -> Effect {
        update(
            state,
            Msg::HeaderFollowed {
                project: project(),
                entry,
            },
        )
    }

    /// Feature 1126 T005: a new entry is followed; the same entry twice changes nothing; `None`
    /// clears it.
    #[test]
    fn the_header_follows_the_shown_location() {
        let mut state = State::default();
        let worktree = SessionLocation::Worktree("feat-1_x".into());
        assert!(matches!(
            follow(&mut state, Some(worktree.clone())),
            Effect::ReadCounts { .. }
        ));
        assert_eq!(
            state.header.entry,
            Some((project(), worktree.clone())),
            "a new entry is followed"
        );
        let before = state.clone();
        follow(&mut state, Some(worktree));
        assert_eq!(
            state, before,
            "the same entry twice leaves the state unchanged"
        );
        follow(&mut state, Some(SessionLocation::Default));
        assert_eq!(
            state.header.entry,
            Some((project(), SessionLocation::Default))
        );
        follow(&mut state, None);
        assert_eq!(state.header.entry, None, "no location shown: no entry");
    }

    fn summary(files: u32) -> ChangeSummary {
        ChangeSummary {
            files,
            added: 4,
            removed: 1,
        }
    }

    /// The header's state after following `entry` and its first read answering `result`.
    fn read_once(entry: SessionLocation, result: Result<ChangeSummary, String>) -> State {
        let mut state = State::default();
        let Effect::ReadCounts { seq, .. } = follow(&mut state, Some(entry)) else {
            panic!("a new entry is read");
        };
        assert_eq!(
            update(&mut state, Msg::HeaderCountsRead { seq, result }),
            Effect::None
        );
        state
    }

    /// Feature 1126 T044: following a new entry starts a counts read of it; the same entry again
    /// starts none; `None` clears and reads nothing.
    #[test]
    fn following_a_new_entry_reads_its_counts() {
        let mut state = State::default();
        let worktree = SessionLocation::Worktree("feat-1_x".into());
        let effect = follow(&mut state, Some(worktree.clone()));
        let Effect::ReadCounts { seq, entry } = effect else {
            panic!("a new entry is read, got {effect:?}");
        };
        assert_eq!(entry, worktree);
        assert_eq!(
            state.header.load,
            Load::Loading {
                seq,
                again: false,
                last: None
            }
        );
        assert_eq!(
            follow(&mut state, Some(worktree)),
            Effect::None,
            "same entry"
        );
        assert_eq!(follow(&mut state, None), Effect::None, "nothing shown");
        assert_eq!(state.header.load, Load::Idle);
    }

    /// T044: the current read's answer is kept (`Ready` or `Failed`); an older read's is dropped.
    #[test]
    fn the_current_counts_answer_is_kept_and_an_old_one_dropped() {
        let entry = SessionLocation::Default;
        let ready = read_once(entry.clone(), Ok(summary(3)));
        assert_eq!(ready.header.load, Load::Ready(summary(3)));
        assert_eq!(ready.header.reading(), CountsReading::Known(summary(3)));
        let failed = read_once(entry, Err("not a git repository".into()));
        assert_eq!(
            failed.header.load,
            Load::Failed("not a git repository".into())
        );

        let mut state = State::default();
        let Effect::ReadCounts { seq: old, .. } =
            follow(&mut state, Some(SessionLocation::Worktree("a".into())))
        else {
            panic!("read of a");
        };
        follow(&mut state, Some(SessionLocation::Worktree("b".into())));
        let before = state.clone();
        update(
            &mut state,
            Msg::HeaderCountsRead {
                seq: old,
                result: Ok(summary(9)),
            },
        );
        assert_eq!(
            state, before,
            "an answer for an earlier location is dropped"
        );
    }

    /// T044: a change while the counts are known, failed or idle reads them again (keeping the
    /// last figure); a change during a read queues one more, run when the read succeeds.
    #[test]
    fn a_change_rereads_the_counts_or_queues_a_read() {
        let entry = SessionLocation::Worktree("w".into());
        for start in [Ok(summary(2)), Err("boom".to_string())] {
            let mut state = read_once(entry.clone(), start.clone());
            let effect = update(&mut state, Msg::Changed);
            let Effect::ReadCounts { seq, entry: read } = effect else {
                panic!("a change rereads from {start:?}, got {effect:?}");
            };
            assert_eq!(read, entry);
            assert_eq!(
                state.header.load,
                Load::Loading {
                    seq,
                    again: false,
                    last: start.clone().ok(),
                }
            );
            let shown = start.map_or(CountsReading::Unknown, CountsReading::Known);
            assert_eq!(state.header.reading(), shown);
        }
        // Idle with an entry: a read that cannot be answered left none behind.
        let mut idle = State::default();
        idle.header.entry = Some((project(), entry.clone()));
        assert!(matches!(
            update(&mut idle, Msg::Changed),
            Effect::ReadCounts { .. }
        ));
        // No entry: nothing to read.
        assert_eq!(update(&mut State::default(), Msg::Changed), Effect::None);

        let mut state = State::default();
        let Effect::ReadCounts { seq, .. } = follow(&mut state, Some(entry.clone())) else {
            panic!("read");
        };
        assert_eq!(update(&mut state, Msg::Changed), Effect::None, "queued");
        assert!(matches!(
            state.header.load,
            Load::Loading { again: true, .. }
        ));
        let effect = update(
            &mut state,
            Msg::HeaderCountsRead {
                seq,
                result: Ok(summary(1)),
            },
        );
        let Effect::ReadCounts { seq: next, .. } = effect else {
            panic!("the queued read runs, got {effect:?}");
        };
        // A read queued behind one that fails runs too (review A M3 F2).
        assert_eq!(update(&mut state, Msg::Changed), Effect::None, "queued");
        let effect = update(
            &mut state,
            Msg::HeaderCountsRead {
                seq: next,
                result: Err("index.lock exists".into()),
            },
        );
        let Effect::ReadCounts { seq: next, .. } = effect else {
            panic!("the read queued behind a failure runs, got {effect:?}");
        };
        assert_eq!(
            state.header.load,
            Load::Loading {
                seq: next,
                again: false,
                last: Some(summary(1)),
            },
            "a failed re-read keeps the last figure (#1342)"
        );
        // Back to the success path's state for the assertion below.
        update(
            &mut state,
            Msg::HeaderCountsRead {
                seq: next,
                result: Ok(summary(1)),
            },
        );
        update(&mut state, Msg::Changed);
        let Load::Loading { seq: next, .. } = state.header.load else {
            panic!("reading");
        };
        assert_ne!(next, seq);
        assert_eq!(
            state.header.load,
            Load::Loading {
                seq: next,
                again: false,
                last: Some(summary(1)),
            }
        );
    }

    /// T044: the shown worktree removed clears the header's entry and counts.
    #[test]
    fn a_removed_worktree_clears_the_header() {
        let entry = SessionLocation::Worktree("w".into());
        let mut state = read_once(entry, Ok(summary(2)));
        update(
            &mut state,
            Msg::WorktreesListed(BTreeSet::from(["w".to_string()])),
        );
        assert!(state.header.entry.is_some(), "still listed: kept");
        update(
            &mut state,
            Msg::WorktreesListed(BTreeSet::from(["other".to_string()])),
        );
        assert_eq!(state.header.entry, None);
        assert_eq!(state.header.load, Load::Idle);
        // The Default entry is never a listed worktree.
        let mut root = read_once(SessionLocation::Default, Ok(summary(2)));
        update(&mut root, Msg::WorktreesListed(BTreeSet::new()));
        assert!(root.header.entry.is_some());
    }

    /// T044, US3 scenario 4 (#1342): the last finished read's figure stays while a read repeats.
    #[test]
    fn the_last_finished_figure_is_shown_while_rereading() {
        let header = |load| HeaderCounts {
            load,
            ..HeaderCounts::default()
        };
        for load in [
            Load::Idle,
            Load::Loading {
                seq: 1,
                again: false,
                last: None,
            },
            Load::Failed("boom".into()),
        ] {
            assert_eq!(
                header(load.clone()).reading(),
                CountsReading::Unknown,
                "{load:?}"
            );
        }
        assert_eq!(
            header(Load::Ready(summary(5))).reading(),
            CountsReading::Known(summary(5))
        );
        for again in [false, true] {
            assert_eq!(
                header(Load::Loading {
                    seq: 1,
                    again,
                    last: Some(summary(5)),
                })
                .reading(),
                CountsReading::Known(summary(5))
            );
        }
    }

    /// #1342: a failed re-read keeps the last figure; a failed first read stays failed.
    #[test]
    fn a_failed_reread_keeps_the_last_figure() {
        let mut state = read_once(SessionLocation::Default, Ok(summary(2)));
        let Effect::ReadCounts { seq, .. } = update(&mut state, Msg::Changed) else {
            panic!("a recount reads");
        };
        assert_eq!(state.header.reading(), CountsReading::Known(summary(2)));
        update(
            &mut state,
            Msg::HeaderCountsRead {
                seq,
                result: Err("boom".into()),
            },
        );
        assert_eq!(state.header.reading(), CountsReading::Known(summary(2)));
    }
}
