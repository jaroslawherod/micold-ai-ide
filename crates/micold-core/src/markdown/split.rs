//! Top-level block slices (research R2): where the text is cut, and which cuts are final.
//!
//! pulldown-cmark is run over the text and every top-level block's start, moved back to the start
//! of its line, is a cut. A slice runs from its cut to the next one, trailing blank lines included,
//! and is parsed on its own, so nothing later can reach back into it.

use pulldown_cmark::{Event, Parser};

/// Where `text`'s top-level blocks start: `0`, then each later block's line start (a link
/// reference definition is a block here). Empty for an empty text.
pub(super) fn cuts(text: &str) -> Vec<usize> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut starts = Vec::new();
    let mut depth = 0usize;
    let mut events = Parser::new_ext(text, super::options()).into_offset_iter();
    for (event, range) in events.by_ref() {
        let top = match event {
            Event::Start(_) => {
                depth += 1;
                depth == 1
            }
            Event::End(_) => {
                depth = depth.saturating_sub(1);
                false
            }
            _ => depth == 0,
        };
        if top {
            starts.push(range.start);
        }
    }
    // A link reference definition is a block of its own, though it opens no event: cut there too,
    // so it never resolves a reference in another slice (research R2).
    starts.extend(
        events
            .reference_definitions()
            .iter()
            .map(|(_, d)| d.span.start),
    );
    let mut cuts: Vec<usize> = starts.into_iter().map(|at| line_start(text, at)).collect();
    cuts.sort_unstable();
    if let Some(first) = cuts.first_mut() {
        *first = 0;
    } else {
        cuts.push(0);
    }
    cuts.dedup();
    cuts
}

/// Where the line holding byte `at` starts.
pub(super) fn line_start(text: &str, at: usize) -> usize {
    text[..at].rfind(['\n', '\r']).map_or(0, |i| i + 1)
}

/// The start of the last line of `text` that has its line ending: a cut before it is final while
/// the text streams, because the line it starts and the line after it are complete (a table's
/// header row needs its separator row to interrupt a paragraph; a lone `***` or `#` on an
/// unfinished line may still turn into paragraph text). `0` when no line is complete.
pub(super) fn last_complete_line(text: &str) -> usize {
    let mut end = text.rfind(['\n', '\r']).map_or(0, |i| i + 1);
    for ending in ['\n', '\r'] {
        if text[..end].ends_with(ending) {
            end -= 1;
        }
    }
    line_start(text, end)
}
