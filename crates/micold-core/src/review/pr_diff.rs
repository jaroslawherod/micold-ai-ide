//! A pull request's diff split per file (feature 775, research R2): GitHub answers one unified
//! diff for the whole request; the Changes view lists its files and parses a file's `patch` with
//! [`super::diff::parse_unified`] when the user selects it. Pure: nothing here reads a file.

use super::changes::{ChangeKind, ChangedFile, Content, Origin};
use super::{limits, RelPath};

/// A pull request's files, in the order GitHub printed them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PrDiff {
    /// One per `diff --git` section.
    pub files: Vec<PrFile>,
}

/// One file of a pull request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrFile {
    /// The list row.
    pub file: ChangedFile,
    /// The file's own section of the diff, as `git diff` prints it.
    pub patch: Vec<u8>,
}

/// Split `raw` at each `diff --git` line.
pub fn split_pr_diff(raw: &str) -> PrDiff {
    let mut files = Vec::new();
    let mut start: Option<usize> = None;
    let mut offset = 0;
    for line in raw.split_inclusive('\n') {
        if line.starts_with("diff --git ") {
            if let Some(from) = start {
                files.extend(read_section(&raw[from..offset]));
            }
            start = Some(offset);
        }
        offset += line.len();
    }
    if let Some(from) = start {
        files.extend(read_section(&raw[from..]));
    }
    PrDiff { files }
}

fn read_section(section: &str) -> Option<PrFile> {
    let mut lines = section.lines();
    let header = lines.next()?.strip_prefix("diff --git ")?;
    let (mut old_path, mut new_path) = (None, None);
    let (mut added_file, mut deleted_file, mut mode_change) = (false, false, false);
    let (mut rename_from, mut rename_to) = (None, None);
    let mut binary = false;
    let (mut added, mut removed) = (0u32, 0u32);
    let mut in_hunk = false;
    for line in lines {
        if in_hunk {
            match line.as_bytes().first() {
                Some(b'+') => added = added.saturating_add(1),
                Some(b'-') => removed = removed.saturating_add(1),
                _ => {}
            }
            continue;
        }
        if line.starts_with("@@") {
            in_hunk = true;
        } else if line.starts_with("new file mode") {
            added_file = true;
        } else if line.starts_with("deleted file mode") {
            deleted_file = true;
        } else if line.starts_with("old mode") || line.starts_with("new mode") {
            mode_change = true;
        } else if let Some(path) = line.strip_prefix("rename from ") {
            rename_from = Some(unquote(path));
        } else if let Some(path) = line.strip_prefix("rename to ") {
            rename_to = Some(unquote(path));
        } else if let Some(path) = line.strip_prefix("--- ") {
            old_path = side(path, "a/");
        } else if let Some(path) = line.strip_prefix("+++ ") {
            new_path = side(path, "b/");
        } else if line.starts_with("Binary files ") || line.starts_with("GIT binary patch") {
            binary = true;
        }
    }
    let path = rename_to
        .or(new_path)
        .or(old_path)
        .or_else(|| header_path(header))?;
    let kind = if let Some(from) = rename_from {
        ChangeKind::Renamed {
            from: RelPath::from_git(&from),
        }
    } else if added_file {
        ChangeKind::Added
    } else if deleted_file {
        ChangeKind::Deleted
    } else if mode_change && !in_hunk && !binary {
        ChangeKind::ModeOnly
    } else {
        ChangeKind::Modified
    };
    let lines = u64::from(added) + u64::from(removed);
    Some(PrFile {
        file: ChangedFile {
            path: RelPath::from_git(&path),
            kind,
            content: if binary {
                Content::Binary
            } else {
                Content::Text
            },
            added,
            removed,
            large: lines > u64::from(limits::MAX_CHANGED_LINES),
            origin: Origin::Committed,
        },
        patch: section.as_bytes().to_vec(),
    })
}

/// The path of a `---`/`+++` line, `None` for `/dev/null`; `prefix` is `a/` or `b/`.
fn side(text: &str, prefix: &str) -> Option<String> {
    let text = text.split('\t').next().unwrap_or(text);
    let path = unquote(text);
    if path == "/dev/null" {
        return None;
    }
    Some(
        path.strip_prefix(prefix)
            .map_or(path.clone(), str::to_owned),
    )
}

/// The path in `a/<p> b/<p>` (the same path twice, quoted or not): a header with no `---`/`+++`
/// lines (a binary file, a mode change).
fn header_path(header: &str) -> Option<String> {
    if header.starts_with('"') {
        let (first, rest) = quoted_prefix(header)?;
        let _ = rest;
        return Some(
            first
                .strip_prefix("a/")
                .map_or(first.clone(), str::to_owned),
        );
    }
    // `a/<p> b/<p>`: both halves are the same path, so the length is fixed.
    let len = header.len().checked_sub(5)?;
    if len % 2 != 0 {
        return None;
    }
    let path = &header[2..2 + len / 2];
    header.starts_with("a/").then(|| path.to_owned())
}

/// Decode a git C-quoted string at the start of `text`; returns it and what follows.
fn quoted_prefix(text: &str) -> Option<(String, &str)> {
    let rest = text.strip_prefix('"')?;
    let bytes = rest.as_bytes();
    let mut out: Vec<u8> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                let decoded = String::from_utf8_lossy(&out).into_owned();
                return Some((decoded, &rest[i + 1..]));
            }
            b'\\' if i + 1 < bytes.len() => {
                i += 1;
                match bytes[i] {
                    b'n' => out.push(b'\n'),
                    b't' => out.push(b'\t'),
                    d @ b'0'..=b'7' => {
                        let mut value = u32::from(d - b'0');
                        let mut taken = 1;
                        while taken < 3
                            && i + 1 < bytes.len()
                            && (b'0'..=b'7').contains(&bytes[i + 1])
                        {
                            i += 1;
                            value = value * 8 + u32::from(bytes[i] - b'0');
                            taken += 1;
                        }
                        out.push(u8::try_from(value & 0xff).unwrap_or(b'?'));
                    }
                    other => out.push(other),
                }
            }
            byte => out.push(byte),
        }
        i += 1;
    }
    None
}

fn unquote(text: &str) -> String {
    match quoted_prefix(text) {
        Some((decoded, _)) => decoded,
        None => text.to_owned(),
    }
}
