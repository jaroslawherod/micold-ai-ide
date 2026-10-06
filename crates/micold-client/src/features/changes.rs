//! The Changes view of one entry: which entry is open, its toggles, the changed-file list and the
//! selected file (feature 482, contracts/changes-view.md V1–V3 and L1–L3, data-model § Client).
//!
//! Render-free. The shell half (`shell/changes.rs`) turns [`Effect::ReadList`] into the git read
//! off the update thread and answers [`Msg::ListRead`]. What the list pane says when it has no rows
//! is chosen here ([`list_body`], [`base_line`]) so the glue in `ui/changes.rs` only renders it.

use std::collections::BTreeSet;

use micold_core::review::base::{Base, BaseUnavailable, ReviewScope, Toggles};
use micold_core::review::changes::{ChangeList, ChangedFile};
use micold_core::review::RelPath;
use micold_core::session::SessionLocation;

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
    /// The entry shown; its project is the window's active project.
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
}

/// What this feature is told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// Open the view of `entry` (V1); reopening resets the toggles.
    Opened {
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
}

/// Apply `msg` (contracts/changes-view.md V1–V3, L1).
pub fn update(state: &mut State, msg: Msg) -> Effect {
    let _ = (state, msg);
    Effect::None
}

/// Whether the Committed toggle can be used: not for the Default entry (L1, US1 s9).
pub fn committed_available(view: &OpenView) -> bool {
    let _ = view;
    true
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
    let _ = view;
    ListBody::Loading
}

/// The header's base line (V2): `Compared with <branch> at <short sha>`, or why there is none.
/// `None` until the first read answers.
pub fn base_line(view: &OpenView) -> Option<String> {
    let _ = view;
    None
}

/// The list currently on screen: the ready one, or the one kept while a re-read runs.
pub fn shown_list(view: &OpenView) -> Option<&ChangeList> {
    let _ = view;
    None
}

#[allow(dead_code)]
fn reason(why: BaseUnavailable) -> &'static str {
    let _ = why;
    ""
}

#[allow(dead_code)]
fn base_name(scope: &ReviewScope) -> String {
    let _ = (scope, Base::Unavailable(BaseUnavailable::NoDefaultBranch));
    String::new()
}
