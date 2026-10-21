//! Edit, MultiEdit and Write tool calls as line diffs (feature 940, research R6;
//! contracts/edit-diff.md).
//!
//! Render-free: [`edit_diff`] reads a complete tool input and gives the change it asks for, one
//! hunk per edit, or `None` when the input is not of the expected shape, so the card falls back to
//! the plain input (FR-018). Lines carry no file line numbers (FR-016): the tool call does not
//! say where in the file the text is.

use std::time::Duration;

use serde_json::{Map, Value};
use similar::{Algorithm, ChangeTag, TextDiff};

use crate::review::diff::{split_lines, LineKind};

/// The change one Edit, MultiEdit or Write call asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditDiff {
    /// The call's `file_path`.
    pub path: String,
    pub kind: EditKind,
    /// One per edit, in the call's order (FR-013); one for a Write.
    pub hunks: Vec<EditHunk>,
    /// The `Added` lines of every hunk, for the folded header (FR-017).
    pub added: u32,
    /// The `Removed` lines of every hunk.
    pub removed: u32,
    /// The size of the input text, for the size limit (research R7).
    pub bytes: u64,
}

/// Which tool asked for the change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditKind {
    Edit,
    MultiEdit,
    Write,
}

/// One edit's change: "Edit `ordinal` of `of`".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditHunk {
    /// 1-based position of the edit in the call.
    pub ordinal: u32,
    /// The number of edits in the call.
    pub of: u32,
    pub note: Option<HunkNote>,
    pub lines: Vec<EditLine>,
    /// The old text ends without a newline (a Write's old text is empty, so `false`).
    pub old_no_newline: bool,
    /// The new text ends without a newline.
    pub new_no_newline: bool,
}

/// What a hunk says beside its lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HunkNote {
    /// The edit replaces every occurrence (`replace_all: true`, FR-014).
    ReplaceAll,
    /// The old and new text are the same: no lines.
    NoChange,
    /// A Write of empty content.
    EmptyWrite,
    /// A Write: its lines are the file's new content (its previous content is unknown).
    NewContent,
}

/// One line of a hunk, without its line ending.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditLine {
    pub kind: LineKind,
    pub text: String,
}

/// The diff an Edit, MultiEdit or Write call's complete `input_json` asks for, or `None` for any
/// other tool or an input not of that tool's shape (contracts/edit-diff.md *Input shapes*).
pub fn edit_diff(tool: &str, input_json: &str) -> Option<EditDiff> {
    let kind = match tool {
        "Edit" => EditKind::Edit,
        "MultiEdit" => EditKind::MultiEdit,
        "Write" => EditKind::Write,
        _ => return None,
    };
    let value: Value = serde_json::from_str(input_json).ok()?;
    let input = value.as_object()?;
    let path = string(input, "file_path")?;
    let edits = match kind {
        EditKind::Edit => vec![edit_of(input)?],
        EditKind::MultiEdit => {
            let edits = input.get("edits")?.as_array()?;
            if edits.is_empty() {
                return None;
            }
            edits
                .iter()
                .map(|edit| edit_of(edit.as_object()?))
                .collect::<Option<Vec<_>>>()?
        }
        EditKind::Write => vec![Edit {
            old: "",
            new: string(input, "content")?,
            replace_all: false,
        }],
    };
    let of = count(edits.len());
    let hunks: Vec<EditHunk> = edits
        .iter()
        .zip(1..)
        .map(|(edit, ordinal)| hunk(kind, edit, ordinal, of))
        .collect();
    let lines_of = |wanted: LineKind| {
        count(
            hunks
                .iter()
                .flat_map(|h| &h.lines)
                .filter(|l| l.kind == wanted)
                .count(),
        )
    };
    Some(EditDiff {
        path: path.to_owned(),
        kind,
        added: lines_of(LineKind::Added),
        removed: lines_of(LineKind::Removed),
        bytes: input_json.len() as u64,
        hunks,
    })
}

/// How long the line diff of one edit may take before it settles for a coarser, still correct,
/// diff (research R6), so a pathological edit cannot stall an update (SC-004).
const DIFF_DEADLINE: Duration = Duration::from_millis(50);

/// One edit's old and new text, borrowed from the input.
struct Edit<'a> {
    old: &'a str,
    new: &'a str,
    replace_all: bool,
}

/// An edit object (`Edit`'s input or one of `MultiEdit`'s `edits`): `old_string` and `new_string`
/// strings, `replace_all` a bool when present.
fn edit_of(object: &Map<String, Value>) -> Option<Edit<'_>> {
    let replace_all = match object.get("replace_all") {
        None => false,
        Some(flag) => flag.as_bool()?,
    };
    Some(Edit {
        old: string(object, "old_string")?,
        new: string(object, "new_string")?,
        replace_all,
    })
}

fn string<'a>(object: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    object.get(key)?.as_str()
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn hunk(kind: EditKind, edit: &Edit<'_>, ordinal: u32, of: u32) -> EditHunk {
    let note = match kind {
        EditKind::Write if edit.new.is_empty() => Some(HunkNote::EmptyWrite),
        EditKind::Write => Some(HunkNote::NewContent),
        _ if edit.old == edit.new => Some(HunkNote::NoChange),
        _ if edit.replace_all => Some(HunkNote::ReplaceAll),
        _ => None,
    };
    let lines = if note == Some(HunkNote::NoChange) {
        Vec::new()
    } else {
        line_diff(edit.old, edit.new)
    };
    EditHunk {
        ordinal,
        of,
        note,
        lines,
        old_no_newline: no_final_newline(edit.old),
        new_no_newline: no_final_newline(edit.new),
    }
}

/// The text has a last line and it has no line ending.
fn no_final_newline(text: &str) -> bool {
    !text.is_empty() && !text.ends_with('\n')
}

/// `old` to `new` line by line: equal lines are `Context`, deleted ones `Removed`, inserted ones
/// `Added`, line endings stripped as the Changes view strips them.
fn line_diff(old: &str, new: &str) -> Vec<EditLine> {
    let old: Vec<&str> = split_lines(old).collect();
    let new: Vec<&str> = split_lines(new).collect();
    let diff = TextDiff::configure()
        .algorithm(Algorithm::Myers)
        .timeout(DIFF_DEADLINE)
        .diff_slices(&old, &new);
    diff.iter_all_changes()
        .map(|change| EditLine {
            kind: match change.tag() {
                ChangeTag::Equal => LineKind::Context,
                ChangeTag::Delete => LineKind::Removed,
                ChangeTag::Insert => LineKind::Added,
            },
            text: change.value().to_owned(),
        })
        .collect()
}
