//! The plumbing the source-scanning guard tests share: walk a directory of `.rs` files, read them,
//! and strip what a guard must not match (comments, and for some guards string literals).
//!
//! Included with `#[path = "support/source_scan.rs"] mod source_scan;` rather than through
//! `support/mod.rs`, which pulls in the renderer-backed layout fixtures these guards have no use
//! for.
//!
//! # What each stripper is for
//!
//! They differ in what they keep, and a guard picks the one its old private copy was:
//!
//! - [`strip_comments`]: `//` and `/* */` comments gone, line structure kept for `//` only.
//! - [`strip_comments_keep_lines`]: the same, and a block comment leaves its newlines behind, so
//!   line numbers survive.
//! - [`strip_line_comments`]: cuts each line at its first `//`. Cheaper and cruder: it also cuts a
//!   `//` inside a string, and does not know block comments.
//! - [`code_only`]: comments *and* string literals gone, so a doc comment explaining a rule or a
//!   fixture quoting a name cannot trip it.
//!
//! Every directory walk fails loudly on an unreadable path: a scan that skipped part of the tree
//! would pass without having looked there.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

/// `<crate>/src`.
pub fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Every `.rs` file under `dir`, recursively, sorted.
pub fn rs_files(dir: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let entries =
            fs::read_dir(dir).unwrap_or_else(|e| panic!("cannot list {}: {e}", dir.display()));
        for entry in entries {
            let path = entry
                .unwrap_or_else(|e| panic!("cannot read an entry of {}: {e}", dir.display()))
                .path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, &mut out);
    out.sort();
    out
}

/// Every `.rs` file under each of `dirs`, as `(path relative to root with `/` separators, source)`,
/// sorted by that path.
pub fn read_rs_under(dirs: &[PathBuf], root: &Path) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = dirs
        .iter()
        .flat_map(|dir| rs_files(dir))
        .map(|path| {
            let name = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .display()
                .to_string()
                .replace('\\', "/");
            let src = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            (name, src)
        })
        .collect();
    out.sort();
    out
}

/// [`read_rs_under`] with paths relative to `src/`.
pub fn sources_under(dirs: &[PathBuf]) -> Vec<(String, String)> {
    read_rs_under(dirs, &src_dir())
}

fn strip(src: &str, keep_block_newlines: bool) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    let mut in_block = false;
    while let Some(c) = chars.next() {
        if in_block {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block = false;
            } else if keep_block_newlines && c == '\n' {
                out.push('\n');
            }
            continue;
        }
        match (c, chars.peek()) {
            ('/', Some('/')) => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            ('/', Some('*')) => {
                chars.next();
                in_block = true;
            }
            _ => out.push(c),
        }
    }
    out
}

/// `src` without `//` and `/* */` comments. Strings are kept.
pub fn strip_comments(src: &str) -> String {
    strip(src, false)
}

/// [`strip_comments`], but a block comment keeps its newlines so line numbers survive.
pub fn strip_comments_keep_lines(src: &str) -> String {
    strip(src, true)
}

/// Each line of `src` cut at its first `//`.
pub fn strip_line_comments(src: &str) -> String {
    src.lines()
        .map(|line| match line.find("//") {
            Some(at) => &line[..at],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Strips comments and string literals, so the doc comments explaining a rule — and any test
/// fixture quoting a name — cannot trip it.
pub fn code_only(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    let mut in_block = false;
    let mut in_line = false;
    let mut in_str = false;
    while let Some(c) = chars.next() {
        if in_block {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block = false;
            }
            continue;
        }
        if in_line {
            if c == '\n' {
                in_line = false;
                out.push('\n');
            }
            continue;
        }
        if in_str {
            if c == '\\' {
                chars.next();
            } else if c == '"' {
                in_str = false;
            }
            continue;
        }
        match c {
            '"' => {
                in_str = true;
                continue;
            }
            '/' if chars.peek() == Some(&'/') => {
                in_line = true;
                continue;
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                in_block = true;
                continue;
            }
            _ => {}
        }
        out.push(c);
    }
    out
}
