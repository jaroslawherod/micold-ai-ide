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

pub use resolve::{LinkContext, Reason, ResolvedLink, SandboxLinkContext, SharedLocation, Target};

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
