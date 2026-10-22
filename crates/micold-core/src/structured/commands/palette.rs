//! The command palette's render-free half (#943 FR-007, FR-008, FR-008a, research R10).
//!
//! The palette is open while the draft is `/` and a name with the caret in it
//! ([`palette_query`]); it lists [`filter`]'s matches with the first selected
//! ([`PaletteSelection`]); accepting one fills `/<name> ` ([`accept`]).

use super::CommandList;
use crate::typeahead::{move_highlight, Direction};

/// The name typed so far while the palette is open: the draft starts with `/` and a name (no
/// whitespace right after the `/`), and the caret, a byte offset, is within that name. `None`
/// (closed) otherwise (FR-007, US2 scenarios 4 and 7).
pub fn palette_query(draft: &str, caret: usize) -> Option<&str> {
    let rest = draft.strip_prefix('/')?;
    if caret == 0 || caret > draft.len() || !draft.is_char_boundary(caret) {
        return None;
    }
    if rest.starts_with(char::is_whitespace) {
        return None;
    }
    let name_end = 1 + rest.find(char::is_whitespace).unwrap_or(rest.len());
    (caret <= name_end).then(|| &draft[1..caret])
}

/// Indices into `list`'s commands whose name contains `query`, ignoring case, in FR-008's order:
/// the equal name, names starting with it, names with a word (after `-`, `_` or `:`) starting
/// with it, then the rest, alphabetically within each. An empty query lists every command,
/// grouped Skill, Custom, BuiltIn, App, alphabetically within each (FR-007).
pub fn filter(list: &CommandList, query: &str) -> Vec<usize> {
    let query = query.to_lowercase();
    let mut ranked: Vec<(u8, String, usize)> = list
        .commands()
        .iter()
        .enumerate()
        .filter_map(|(index, command)| {
            let name = command.name.to_lowercase();
            let tier = if query.is_empty() {
                command.kind as u8
            } else if name == query {
                0
            } else if name.starts_with(&query) {
                1
            } else if name
                .match_indices(['-', '_', ':'])
                .any(|(at, _)| name[at + 1..].starts_with(&query))
            {
                2
            } else if name.contains(&query) {
                3
            } else {
                return None;
            };
            Some((tier, name, index))
        })
        .collect();
    ranked.sort_unstable();
    ranked.into_iter().map(|(_, _, index)| index).collect()
}

/// The palette's highlighted match: the first on each new query, moved by the arrows without
/// wrapping ([`move_highlight`], FR-008a).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaletteSelection {
    highlight: Option<usize>,
}

impl PaletteSelection {
    /// The selection over `rows` matches: the first, or none when there is none.
    pub fn new(rows: usize) -> Self {
        Self {
            highlight: (rows > 0).then_some(0),
        }
    }

    pub fn highlight(&self) -> Option<usize> {
        self.highlight
    }

    pub fn step(&mut self, direction: Direction, rows: usize) {
        self.highlight = move_highlight(self.highlight, direction, rows);
    }

    /// The query changed and `rows` match now.
    pub fn reset(&mut self, rows: usize) {
        *self = Self::new(rows);
    }
}

/// The draft and caret (a byte offset) after accepting `name`: `/<name> `, the caret at its end.
pub fn accept(name: &str) -> (String, usize) {
    let draft = format!("/{name} ");
    let caret = draft.len();
    (draft, caret)
}
