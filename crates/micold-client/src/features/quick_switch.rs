//! The quick switcher (feature 968): one palette that jumps to a location, a session or a project
//! by typing part of its name. The matching rules live in `micold_core::quick_switch`; this module
//! holds the palette's own state and message vocabulary (data-model.md *Client*).

use micold_core::navigation::Dir;
use micold_core::quick_switch::{Candidate, EntryKey};

use crate::features::sidebar::RowRef;

/// What an entry stands for, for its icon and its context line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Location,
    Session,
    Project,
}

/// One row of the palette: the rule-facing candidate plus what the view shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub candidate: Candidate,
    pub kind: Kind,
    /// The dimmed second line: a session's location, a project's path.
    pub context: String,
    /// Whether this is the current location, session or project.
    pub current: bool,
}

/// The palette while it is open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Open {
    pub query: String,
    /// The highlighted entry, by key so it survives the list changing (FR-013).
    pub highlighted: Option<EntryKey>,
    /// Whether the terminal had the keyboard when the palette opened (FR-011).
    pub terminal_had_focus: bool,
    /// The sidebar's keyboard row when the palette opened, restored on dismissal.
    pub keyboard_row_before: Option<RowRef>,
}

/// The quick switcher's state: closed, or open.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct State {
    pub open: Option<Open>,
}

/// Everything the quick switcher says about itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// The chord was pressed: open when closed, dismiss when open.
    Toggled { terminal_had_focus: bool },
    /// The query text changed.
    QueryChanged(String),
    /// The highlight moved.
    Moved(Dir),
    /// An entry was chosen (Enter or a click).
    Chosen(EntryKey),
    /// Escape, a click outside, or the chord again.
    Dismissed,
}
