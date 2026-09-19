//! A session's label from the first thing the user typed in it (feature 032).
//!
//! When an AI CLI never titles a conversation, the row still needs something better than "New
//! session". This module turns the conversation's record file into that label: read a bounded
//! prefix (FR-014), find the first **typed** turn by each CLI's own record rules (FR-002, FR-012),
//! and shape it into one line of at most 80 user-perceived characters (FR-003).
//!
//! Record-format knowledge stays below the provider seam: only `provider.rs` calls this. The
//! contract is `specs/032-untitled-session-labels/contracts/first-turn-label.md`; the clause ids
//! (C2, C3, C5) below are its.

use std::io::Read;
use std::path::Path;

use unicode_segmentation::UnicodeSegmentation;

/// How much of a record file a label is worth reading: 1 MiB (C2.1, research R7). A first turn
/// sits in the first few records of every conversation observed; a conversation whose first turn
/// starts later yields no label rather than a read of the whole file (C2.4).
pub const LABEL_BUDGET_BYTES: u64 = 1024 * 1024;

/// The longest label, in extended grapheme clusters (FR-003).
const MAX_GRAPHEMES: usize = 80;

/// What a cut label ends with, so the reader can see it was cut (C5.4).
const ELLIPSIS: &str = "…";

/// The first [`LABEL_BUDGET_BYTES`] of `path`, cut back to the last complete line (C2.1, C2.2).
///
/// A trailing line with no `\n` — one still being written, or one the bound cut through — is
/// dropped, never parsed. `None` for a file that cannot be opened or read: a label read never fails
/// the session (FR-011).
pub fn read_prefix(path: &Path) -> Option<Vec<u8>> {
    let mut prefix = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(LABEL_BUDGET_BYTES)
        .read_to_end(&mut prefix)
        .ok()?;
    let complete = prefix
        .iter()
        .rposition(|&byte| byte == b'\n')
        .map_or(0, |last| last + 1);
    prefix.truncate(complete);
    Some(prefix)
}

/// `text` as a row label: one line, at most 80 grapheme clusters, or `None` when nothing is left
/// (C5).
///
/// Every run of whitespace, line breaks included, becomes one space and the ends are trimmed
/// (C5.1). A result over the bound keeps its first 79 clusters and gains `…` (C5.4). Counting and
/// cutting by grapheme cluster, never by `char`, is what keeps an emoji ZWJ sequence, a base with
/// its combining marks, or a CJK character whole (C5.5).
pub fn shape_label(text: &str) -> Option<String> {
    let one_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.is_empty() {
        return None;
    }
    let graphemes: Vec<&str> = one_line.graphemes(true).collect();
    if graphemes.len() <= MAX_GRAPHEMES {
        return Some(one_line);
    }
    let mut cut = graphemes[..MAX_GRAPHEMES - 1].concat();
    cut.push_str(ELLIPSIS);
    Some(cut)
}

/// Feature 032 stub.
pub fn claude_first_turn(_prefix: &[u8]) -> Option<String> {
    None
}
