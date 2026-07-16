//! Worktree naming derivation — the single source of truth mapping add-worktree form
//! inputs to a directory name and a git branch name (FR-006, FR-006a).
//!
//! Pure and unit-testable; no I/O. Kept in one place so the formats can become
//! user-configurable in a future version without touching the creation flow (FR-006a).
//! Contract: `specs/005-worktree-session-terminal/contracts/naming.md`.

use std::fmt;

/// The Conventional-Commits type vocabulary offered by the add-worktree form (FR-005a).
///
/// A closed enum: an invalid type is unrepresentable (Constitution Principle V). Fixed
/// defaults this version; the whole ruleset is designed to become configurable later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConventionalType {
    Feat,
    Fix,
    Chore,
    Docs,
    Refactor,
    Test,
    Build,
    Ci,
    Perf,
    Style,
}

impl ConventionalType {
    /// Every type, in display order, for the form's selector and exhaustive testing.
    pub const ALL: &'static [ConventionalType] = &[
        ConventionalType::Feat,
        ConventionalType::Fix,
        ConventionalType::Chore,
        ConventionalType::Docs,
        ConventionalType::Refactor,
        ConventionalType::Test,
        ConventionalType::Build,
        ConventionalType::Ci,
        ConventionalType::Perf,
        ConventionalType::Style,
    ];

    /// The lowercase token used in derived directory/branch names.
    pub const fn as_str(self) -> &'static str {
        match self {
            ConventionalType::Feat => "feat",
            ConventionalType::Fix => "fix",
            ConventionalType::Chore => "chore",
            ConventionalType::Docs => "docs",
            ConventionalType::Refactor => "refactor",
            ConventionalType::Test => "test",
            ConventionalType::Build => "build",
            ConventionalType::Ci => "ci",
            ConventionalType::Perf => "perf",
            ConventionalType::Style => "style",
        }
    }
}

impl fmt::Display for ConventionalType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The raw form inputs before derivation (FR-005).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeNaming {
    /// Selected Conventional-Commits type (required).
    pub type_: Option<ConventionalType>,
    /// Optional ticket reference; omitted from output when absent/blank (FR-005b).
    pub ticket: Option<String>,
    /// Free-text name (required; must be non-empty after slugify, FR-008).
    pub name: String,
}

/// The derived, validated names ready to hand to git.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedNames {
    /// Directory component under `.claude/worktrees/`: `${type}-${ticket}-${name}`.
    pub dir_name: String,
    /// Git branch: `${type}/${ticket}-${name}`.
    pub branch: String,
}

/// Why a proposed naming was rejected (FR-008).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamingError {
    /// No Conventional-Commits type was selected.
    NoType,
    /// The name slugified to an empty string.
    EmptyNameAfterSlug,
    /// The assembled branch fails git `check-ref-format` (defense in depth).
    InvalidBranchRef,
}

impl fmt::Display for NamingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            NamingError::NoType => "Select a type",
            NamingError::EmptyNameAfterSlug => "Enter a name (letters or digits)",
            NamingError::InvalidBranchRef => "The resulting branch name is not valid",
        })
    }
}

/// Normalize a free-text fragment into `[a-z0-9-]`, valid as BOTH a git ref component and a
/// cross-OS directory name (contract naming.md). Returns an empty string if nothing usable
/// remains (callers decide whether empty is an error).
pub fn slugify(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut prev_dash = false;
    for ch in raw.chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            Some(ch.to_ascii_lowercase())
        } else {
            None
        };
        match mapped {
            Some(c) => {
                out.push(c);
                prev_dash = false;
            }
            None => {
                // Collapse any run of non-alphanumerics into a single '-'.
                if !prev_dash && !out.is_empty() {
                    out.push('-');
                    prev_dash = true;
                }
            }
        }
    }
    // Trim a trailing dash left by a non-alphanumeric suffix.
    while out.ends_with('-') {
        out.pop();
    }
    // Guard git/OS tails: `.lock` suffix can't occur (no dots survive), but a Windows
    // reserved device name can — suffix it so it stays a valid, non-reserved segment.
    if is_windows_reserved(&out) {
        out.push_str("-wt");
    }
    out
}

/// Windows reserved device names (case-insensitive), which cannot be directory names.
fn is_windows_reserved(s: &str) -> bool {
    const RESERVED: &[&str] = &[
        "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
        "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
    ];
    RESERVED.contains(&s)
}

/// Validate the git-ref-format-relevant subset for a single branch string. Our slugified
/// output should always pass; this is defense in depth (contract naming.md, research R7).
fn is_valid_branch(branch: &str) -> bool {
    if branch.is_empty() || branch == "@" {
        return false;
    }
    if branch.starts_with('/') || branch.ends_with('/') || branch.contains("//") {
        return false;
    }
    if branch.starts_with('.') || branch.ends_with('.') || branch.contains("..") {
        return false;
    }
    if branch.ends_with(".lock") || branch.contains("@{") {
        return false;
    }
    // No control chars, spaces, or the forbidden set ` ~^:?*[\`.
    !branch
        .chars()
        .any(|c| c.is_control() || matches!(c, ' ' | '~' | '^' | ':' | '?' | '*' | '[' | '\\'))
}

/// Derive and validate the directory + branch names from form inputs (FR-006, FR-008).
///
/// - With a ticket: `dir = "{type}-{ticket}-{name}"`, `branch = "{type}/{ticket}-{name}"`.
/// - Without a ticket (or blank after slug): the ticket segment is dropped entirely — no
///   empty separators (FR-005b).
pub fn derive(input: &WorktreeNaming) -> Result<DerivedNames, NamingError> {
    let type_ = input.type_.ok_or(NamingError::NoType)?;
    let type_str = type_.as_str();

    let name = slugify(&input.name);
    if name.is_empty() {
        return Err(NamingError::EmptyNameAfterSlug);
    }

    let ticket = input
        .ticket
        .as_deref()
        .map(slugify)
        .filter(|t| !t.is_empty());

    let (dir_name, branch) = match ticket {
        Some(t) => (
            format!("{type_str}-{t}-{name}"),
            format!("{type_str}/{t}-{name}"),
        ),
        None => (format!("{type_str}-{name}"), format!("{type_str}/{name}")),
    };

    if !is_valid_branch(&branch) {
        return Err(NamingError::InvalidBranchRef);
    }

    Ok(DerivedNames { dir_name, branch })
}
