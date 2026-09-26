//! Links in terminal output (feature 031): which text is a link, and what it opens.
//!
//! Render-free and free of I/O. The pane lends its rows through [`LinkRows`]; everything here is a
//! pure function of those rows and a [`LinkContext`], so each rule is a unit test in
//! `mise run test-core` (contracts/link-recognition.md).

use std::ops::Range;

pub mod address;
pub mod detect;
pub mod line;
pub mod resolve;
pub mod runnable;

pub use resolve::{
    container_host_names, host_names_from, LinkContext, Reason, ResolvedLink, SandboxLinkContext,
    SharedLocation, Target,
};

/// The rows core reads, lent by the client's grid cache (research R4).
///
/// `row` is relative to the viewport's top line, so it is negative in scrollback.
pub trait LinkRows {
    /// One `char` per cell, wide-char spacer cells included; `None` when the row is not available.
    fn text(&self, row: i64) -> Option<&str>;
    /// This row soft-wraps into `row + 1`.
    fn wrapped(&self, row: i64) -> bool;
    /// The URI a program declared at this cell (OSC 8).
    fn hyperlink(&self, row: i64, col: u16) -> Option<&str>;
    /// The cell holds no char of its own: the second cell of a wide char, or the padding left at a
    /// row's end when a wide char wrapped onto the next row. Its `text` char means nothing.
    fn spacer(&self, row: i64, col: u16) -> bool;
}

/// A followable span under the pointer.
///
/// Two `Link`s are the same link when `address`, `origin` and `cells` are equal, which is what
/// keeps two separated runs with the same declared address two links (US2 scenario 4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    /// What opens: the declared URI, or the detected text after trimming (FR-005, FR-021).
    pub address: String,
    pub origin: LinkOrigin,
    /// One span per row the link covers, relative to the viewport top.
    pub cells: Vec<CellSpan>,
}

/// How a link was found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkOrigin {
    /// Recognised in the visible text (FR-001).
    Detected,
    /// Declared by the program with OSC 8 (FR-002).
    Declared,
}

/// The cells a link covers on one row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellSpan {
    pub row: i64,
    pub cols: Range<u16>,
}

#[cfg(test)]
mod tests {
    /// Every source in `link/`, by module name. A module `mod.rs` declares but this list omits
    /// fails [`link_performs_no_io`] rather than escaping it.
    const SOURCES: [(&str, &str); 6] = [
        ("mod", include_str!("mod.rs")),
        ("address", include_str!("address.rs")),
        ("detect", include_str!("detect.rs")),
        ("line", include_str!("line.rs")),
        ("resolve", include_str!("resolve.rs")),
        ("runnable", include_str!("runnable.rs")),
    ];

    #[test]
    fn link_performs_no_io() {
        let declared: Vec<&str> = include_str!("mod.rs")
            .lines()
            .filter_map(|line| line.split_once("mod "))
            .filter(|(visibility, _)| visibility.is_empty() || visibility.starts_with("pub"))
            .filter_map(|(_, name)| name.strip_suffix(';'))
            .collect();
        let listed: Vec<&str> = SOURCES[1..].iter().map(|(name, _)| *name).collect();
        assert_eq!(
            listed, declared,
            "the scan reads exactly the modules link/mod.rs declares"
        );

        // Built at run time so this file's own source never matches them.
        const IO: [&str; 3] = ["net", "fs", "process"];
        let group = ["std", "{"].join("::");
        for (name, source) in SOURCES {
            for module in IO {
                let needle = ["std", module].join("::");
                assert!(
                    !source.contains(&needle),
                    "link/{name}.rs names {needle}: recognising and resolving a link does no I/O (FR-019)"
                );
            }
            for (start, _) in source.match_indices(&group) {
                let names = grouped_import(&source[start + group.len()..]);
                assert!(
                    !names.iter().any(|name| IO.contains(name)),
                    "link/{name}.rs imports {names:?} from std: recognising and resolving a link does no I/O (FR-019)"
                );
            }
        }
    }

    /// The names inside a `std::{…}` import, nested groups included, given the text after its `{`.
    fn grouped_import(after_brace: &str) -> Vec<&str> {
        let mut depth = 1;
        let end = after_brace
            .char_indices()
            .find(|&(_, c)| {
                depth += match c {
                    '{' => 1,
                    '}' => -1,
                    _ => 0,
                };
                depth == 0
            })
            .map_or(after_brace.len(), |(end, _)| end);
        after_brace[..end]
            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .filter(|name| !name.is_empty())
            .collect()
    }
}
