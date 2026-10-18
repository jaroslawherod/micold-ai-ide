//! The quick switcher (feature 968): one palette that jumps to a location, a session or a project
//! by typing part of its name. The matching rules live in `micold_core::quick_switch`; this module
//! holds the palette's own state and message vocabulary (data-model.md *Client*).

use micold_core::navigation::Dir;
use micold_core::quick_switch::{self, Candidate, EntryKey, Ranked};
use micold_core::typeahead::Query;

use crate::app::{Message, State as Root};
use crate::features::navigation::{self, Step};
use crate::features::sidebar::RowRef;
use crate::floating_surface;
use crate::overlay::registry::Registered;
use crate::overlay::SurfaceId;

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

/// The palette as a floating surface: a dialog open exactly while its state is (contracts/ui.md U6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuickSwitcherDialog;

floating_surface!(QuickSwitcherDialog, id: SurfaceId::new("quick_switcher"), layer: Dialog, cancelled_by: Message::QuickSwitch(Msg::Dismissed));

impl Registered for QuickSwitcherDialog {
    fn open_in(state: &Root) -> Option<Self> {
        state
            .quick_switch
            .open
            .is_some()
            .then_some(QuickSwitcherDialog)
    }
}

/// What choosing an entry asks the shell to do (contracts/ui.md U3): the step to apply with
/// `apply_navigation` / `finish_navigation`, the keyboard owner recorded when the palette opened,
/// and whether the Overview closes first so a stop's row is visible (research R5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    pub step: Step,
    pub terminal_had_focus: bool,
    pub close_overview_first: bool,
}

/// Why the list shows no entry (FR-014).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Empty {
    /// A non-empty query kept nothing.
    NoMatches,
    /// There is nothing to switch to at all.
    NothingToSwitchTo,
}

/// The list as the view shows it, derived on every read and never stored (contracts/ui.md U2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct List {
    pub entries: Vec<Entry>,
    /// What the query keeps, best first; indexes into `entries`.
    pub ranked: Vec<Ranked>,
    /// The highlighted position in `ranked`.
    pub highlighted: Option<usize>,
    pub empty: Option<Empty>,
}

/// The list for the open palette's query and highlight (empty query, no highlight when closed).
pub fn list(state: &Root) -> List {
    derive(state).0
}

/// [`list`] with the rule-facing candidates it was ranked from, built once.
fn derive(state: &Root) -> (List, Vec<Candidate>) {
    let open = state.quick_switch.open.as_ref();
    let entries = candidates(state);
    let rule: Vec<Candidate> = entries.iter().map(|e| e.candidate.clone()).collect();
    let query = Query::new(open.map_or("", |o| o.query.as_str()));
    let ranked = quick_switch::filter(&rule, &query);
    let highlighted =
        quick_switch::reseat(&rule, &ranked, open.and_then(|o| o.highlighted.as_ref()));
    let empty = if !ranked.is_empty() {
        None
    } else if entries.is_empty() {
        Some(Empty::NothingToSwitchTo)
    } else {
        Some(Empty::NoMatches)
    };
    let list = List {
        entries,
        ranked,
        highlighted,
        empty,
    };
    (list, rule)
}

/// The key of the highlighted entry of `list`.
fn highlighted_key(list: &List) -> Option<EntryKey> {
    let at = list.highlighted?;
    Some(list.entries[list.ranked[at].index].candidate.key.clone())
}

/// The step choosing `key` resolves to, from a fresh list; `None` for a key not listed and for the
/// kinds that are not listed yet (projects).
fn resolve(state: &Root, key: &EntryKey) -> Option<Step> {
    let listed = candidates(state);
    if !listed
        .iter()
        .any(|e| e.candidate.key == *key && e.candidate.selectable)
    {
        return None;
    }
    match key {
        EntryKey::Location(location) => Some(
            navigation::main_session_at(state, location)
                .map_or_else(|| Step::Stop(location.clone()), Step::Session),
        ),
        // A click on the session's sub-row: `Selected` opens its location's row and reveals it.
        EntryKey::Session(id) => Some(Step::Session(*id)),
        EntryKey::Project(_) => None,
    }
}

/// Apply a message (shape A, FR-015); `Some` when an entry was chosen, for the shell to apply.
pub fn update(state: &mut Root, msg: Msg) -> Option<Choice> {
    match msg {
        Msg::Toggled { terminal_had_focus } => {
            if state.quick_switch.open.is_some() {
                close(state);
            } else if state.settings.settings_draft.is_none()
                && crate::overlay::registry::topmost(state).is_none()
            {
                state.quick_switch.open = Some(Open {
                    query: String::new(),
                    highlighted: None,
                    terminal_had_focus,
                    keyboard_row_before: state.sidebar.keyboard_row.clone(),
                });
                reseat_open(state);
            }
            None
        }
        Msg::QueryChanged(query) => {
            if let Some(open) = &mut state.quick_switch.open {
                open.query = query;
                open.highlighted = None;
                reseat_open(state);
            }
            None
        }
        Msg::Moved(dir) => {
            let (current, rule) = derive(state);
            if let Some(at) = current.highlighted {
                let to = quick_switch::step(&rule, &current.ranked, at, dir);
                let key = rule[current.ranked[to].index].key.clone();
                if let Some(open) = &mut state.quick_switch.open {
                    open.highlighted = Some(key);
                }
            }
            None
        }
        Msg::Chosen(key) => {
            let terminal_had_focus = state.quick_switch.open.as_ref()?.terminal_had_focus;
            let step = resolve(state, &key)?;
            state.quick_switch.open = None;
            let close_overview_first = matches!(step, Step::Stop(_)) && state.overview.open;
            Some(Choice {
                step,
                terminal_had_focus,
                close_overview_first,
            })
        }
        Msg::Dismissed => {
            close(state);
            None
        }
    }
}

/// Store the key `list` highlights for the open palette.
fn reseat_open(state: &mut Root) {
    let key = highlighted_key(&list(state));
    if let Some(open) = &mut state.quick_switch.open {
        open.highlighted = key;
    }
}

/// Close without a choice: the keyboard row goes back where it was (contracts/ui.md U4).
fn close(state: &mut Root) {
    if let Some(open) = state.quick_switch.open.take() {
        state.sidebar.keyboard_row = open.keyboard_row_before;
    }
}

/// The palette's entries in FR-004 order (data-model.md *Entry*): the active project's locations
/// in `navigation::location_order`, each followed by its open sessions in the order a chord steps
/// through them, collapsed rows included. Other projects (M3) are not listed yet.
pub fn candidates(state: &Root) -> Vec<Entry> {
    let nodes = navigation::location_nodes(state);
    let current = state
        .sidebar
        .nav_stop
        .clone()
        .filter(|stop| nodes.iter().any(|(at, _)| at == stop))
        .or_else(|| state.current_session_location());
    let mut entries = Vec::new();
    for (location, node) in nodes {
        let name = node.name().to_string();
        let context = node
            .tags()
            .iter()
            .map(micold_core::naming::Tag::label)
            .collect::<Vec<_>>()
            .join(" · ");
        let parent = entries.len();
        entries.push(Entry {
            candidate: Candidate {
                key: EntryKey::Location(location.clone()),
                name: name.clone(),
                parent: None,
                selectable: true,
            },
            kind: Kind::Location,
            context,
            current: current.as_ref() == Some(&location),
        });
        entries.extend(node.unfolded_sessions().into_iter().map(|session| Entry {
            candidate: Candidate {
                key: EntryKey::Session(session.id),
                name: session.label.display().to_owned(),
                parent: Some(parent),
                selectable: true,
            },
            kind: Kind::Session,
            context: name.clone(),
            current: state.session.active == Some(session.id),
        }));
    }
    entries
}
