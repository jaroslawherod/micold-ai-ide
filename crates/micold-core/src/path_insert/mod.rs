//! Turning dropped files into text for a terminal's input line (feature 487).
//!
//! Pure: a list of host paths and the terminal's shell go in, and the quoted text to type comes
//! out, with the paths that could not be written for that shell set aside and named. Nothing here
//! sends anything; the client types the text without a newline, so nothing runs until Enter
//! (FR-003).

pub mod quote;

use std::path::{Path, PathBuf};

pub use quote::{quote, Unrepresentable};

/// The shell that will read the inserted text. Each quotes differently (research R2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShellKind {
    /// `sh`, `dash`, `ksh` and anything unknown on Unix: single quotes only.
    Posix,
    /// bash and zsh: as [`ShellKind::Posix`], plus `$'…'` for names that are not UTF-8.
    Bash,
    /// fish: single quotes with `\\` and `\'` escaped.
    Fish,
    /// PowerShell: single quotes, doubled inside (curly quotes too).
    PowerShell,
    /// `cmd.exe`: double quotes, with no way to write `"` or `%` inside.
    Cmd,
}

/// Where the text will be typed (data-model: `InsertTarget`). Only the host so far.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertTarget {
    /// A terminal running on this machine: the paths are used as they are.
    Host,
}

/// One path that will be inserted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InsertedPath {
    /// The path as it was dropped.
    pub original: PathBuf,
    /// The path the terminal will see.
    pub shown: PathBuf,
    /// `shown`, quoted for the shell.
    pub text: String,
}

/// Why a path was left out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The shell has no way to write this name as one literal argument.
    Unrepresentable {
        /// The file.
        path: PathBuf,
        /// The shell that cannot write it.
        shell: ShellKind,
    },
}

impl Refusal {
    /// A sentence for the user that names the file and the reason (FR-011).
    pub fn message(&self) -> String {
        match self {
            Refusal::Unrepresentable { path, shell } => format!(
                "Could not insert {}: {} cannot take a name with these characters.",
                file_name(path),
                shell_name(*shell)
            ),
        }
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

fn shell_name(shell: ShellKind) -> &'static str {
    match shell {
        ShellKind::Posix => "this shell",
        ShellKind::Bash => "bash",
        ShellKind::Fish => "fish",
        ShellKind::PowerShell => "PowerShell",
        ShellKind::Cmd => "cmd",
    }
}

/// What a drop will type, and what it leaves out.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InsertionPlan {
    /// The paths to type, in the order they were given.
    pub accepted: Vec<InsertedPath>,
    /// The paths left out, each with its reason.
    pub refused: Vec<Refusal>,
}

impl InsertionPlan {
    /// The accepted paths joined by single spaces, or `None` when nothing is accepted. No leading
    /// or trailing space and no line break: what the user already typed is untouched (FR-004).
    pub fn text(&self) -> Option<String> {
        if self.accepted.is_empty() {
            return None;
        }
        Some(
            self.accepted
                .iter()
                .map(|p| p.text.as_str())
                .collect::<Vec<_>>()
                .join(" "),
        )
    }
}

/// Plan the insertion of `paths` for a terminal whose shell is `shell`.
///
/// A directory is inserted like a file and a missing file is still inserted: the terminal's
/// program decides what to make of either.
pub fn plan_insertion(paths: &[PathBuf], shell: ShellKind, target: InsertTarget) -> InsertionPlan {
    let InsertTarget::Host = target;
    let mut plan = InsertionPlan::default();
    for path in paths {
        match quote(shell, path.as_os_str()) {
            Ok(text) => plan.accepted.push(InsertedPath {
                original: path.clone(),
                shown: path.clone(),
                text,
            }),
            Err(Unrepresentable) => plan.refused.push(Refusal::Unrepresentable {
                path: path.clone(),
                shell,
            }),
        }
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(p: &[&str]) -> Vec<PathBuf> {
        p.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn accepted_paths_keep_input_order_and_join_with_single_spaces() {
        let plan = plan_insertion(
            &paths(&["/b/two.png", "/a/one file.png", "/c"]),
            ShellKind::Posix,
            InsertTarget::Host,
        );
        assert_eq!(
            plan.text().as_deref(),
            Some("'/b/two.png' '/a/one file.png' '/c'")
        );
        assert!(plan.refused.is_empty());
    }

    #[test]
    fn nothing_accepted_means_no_text() {
        let plan = plan_insertion(&[], ShellKind::Posix, InsertTarget::Host);
        assert_eq!(plan.text(), None);
        let plan = plan_insertion(&paths(&["/a\"b"]), ShellKind::Cmd, InsertTarget::Host);
        assert_eq!(plan.text(), None);
        assert_eq!(plan.refused.len(), 1);
    }

    #[test]
    fn a_directory_and_a_missing_file_are_inserted_like_any_file() {
        let dir = std::env::temp_dir();
        let missing = dir.join("487-no-such-file.png");
        let plan = plan_insertion(
            &[dir.clone(), missing.clone()],
            ShellKind::Posix,
            InsertTarget::Host,
        );
        assert_eq!(plan.accepted.len(), 2);
        assert_eq!(plan.accepted[1].original, missing);
    }

    #[test]
    fn an_unrepresentable_name_is_refused_with_the_file_named() {
        let plan = plan_insertion(
            &paths(&["/ok/a.png", "/x/50%.png"]),
            ShellKind::Cmd,
            InsertTarget::Host,
        );
        assert_eq!(plan.accepted.len(), 1);
        let msg = plan.refused[0].message();
        assert!(msg.contains("50%.png"), "{msg}");
        assert!(msg.contains("cmd"), "{msg}");
    }
}
