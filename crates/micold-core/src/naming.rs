//! Worktree naming derivation — the single source of truth mapping add-worktree form
//! inputs to a directory name and a git branch name (FR-006, FR-006a).
//!
//! Pure and unit-testable; no I/O. Kept in one place so the formats can become
//! user-configurable in a future version without touching the creation flow (FR-006a).
//! Contract: `specs/005-worktree-session-terminal/contracts/naming.md`.

use crate::worktree::WorktreeStatus;
use std::fmt;

/// The Conventional-Commits type vocabulary offered by the add-worktree form (FR-005a).
///
/// A closed enum: an invalid type is unrepresentable (Constitution Principle V). Fixed
/// defaults this version; the whole ruleset is designed to become configurable later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

    /// Parse a lowercase token (as produced by [`as_str`](Self::as_str)) back into a type.
    /// Used when deriving tags from an existing directory name (FR-002, FR-008).
    pub fn from_token(token: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|t| t.as_str() == token)
    }

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
/// - With a ticket: `dir = "{type}-{ticket}_{name}"`, `branch = "{type}/{ticket}_{name}"`.
/// - Without a ticket (or blank after slug): the ticket segment is dropped entirely — no
///   empty separators (FR-005b).
///
/// Both carry [`TICKET_SEP`] (BUG-003), so [`dir_name_from_branch`] recovers the ticket exactly
/// rather than guessing at it. The branch is the durable artifact — it outlives this directory,
/// gets pushed, and comes back through the existing-branch picker — so the boundary has to be on
/// it for a re-picked branch to keep its ticket.
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
            format!("{type_str}-{t}{TICKET_SEP}{name}"),
            format!("{type_str}/{t}{TICKET_SEP}{name}"),
        ),
        None => (format!("{type_str}-{name}"), format!("{type_str}/{name}")),
    };

    if !is_valid_branch(&branch) {
        return Err(NamingError::InvalidBranchRef);
    }

    Ok(DerivedNames { dir_name, branch })
}

/// Derive the worktree directory name for an EXISTING branch (feature 016, FR-014).
///
/// The inverse of [`derive`]'s branch→directory mapping: `feat/abc-123_login` →
/// `feat-abc-123_login`.
///
/// [`TICKET_SEP`] is carried across, so a branch this app derived from a ticket comes back through
/// the existing-branch picker with that ticket intact — an exact round trip rather than a guess at
/// where the ticket ended (BUG-003). It is never *invented*: a branch written without one yields a
/// directory without one, and a boundary with nothing usable on either side is dropped rather than
/// left dangling at an edge.
///
/// The cost is that a `snake_case` branch from outside this app reads as ticketed — `fix/some_bug`
/// becomes `fix-some_bug`, chip `SOME`, name "Bug". `_` means one thing everywhere and nothing can
/// tell the two apart. That is one wrong chip on a foreign branch, against every app-made branch
/// silently losing its ticket the moment it is re-picked.
///
/// Routing each segment through [`slugify`] inherits its guarantees: `[a-z0-9-]` only, collapsed
/// separators, and the Windows reserved-device-name guard (Constitution Principle VI).
///
/// Returns `""` when nothing usable remains (e.g. an all-punctuation branch name); callers treat
/// empty as "cannot derive a directory" and reject the candidate rather than creating `.../`.
pub fn dir_name_from_branch(branch: &str) -> String {
    branch
        .split('/')
        .map(|segment| {
            // Slugify around the boundary rather than through it: `slugify` maps `_` to `-`, which
            // is exactly what must not happen to the one character that carries meaning here.
            segment
                .split(TICKET_SEP)
                .map(slugify)
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(&TICKET_SEP.to_string())
        })
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// A typed, structured label shown beneath a worktree's name in the sidebar and used for
/// filtering (FR-001..005). Derived from the existing directory name — never persisted — so
/// the branch/directory naming convention is untouched (FR-007).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tag {
    /// Conventional-Commits type parsed from the leading name segment.
    Type(ConventionalType),
    /// Jira-style issue key (e.g. `ABC-123`), upper-cased, when the name embeds one.
    Issue(String),
    /// On-disk health, injected at render time for non-`Valid` worktrees (FR-011). Never
    /// produced by [`parse_tags`].
    Status(WorktreeStatus),
    /// The worktree belongs to an AI assistant rather than the user (feature 014, FR-010b), so a
    /// revealed row can never be mistaken for the user's own work.
    ///
    /// Label only, and deliberately WITHOUT a `TagFilter` counterpart in the client — it marks a
    /// row, it is not something to filter by. Like [`Tag::Status`] it is injected at render time
    /// (it needs the branch, which [`parse_tags`] never sees), never produced by [`parse_tags`].
    Agent,
}

/// The boundary between a ticket and the descriptive name in a directory name (BUG-003).
///
/// [`slugify`] maps every non-alphanumeric character to `-`, so a derived name can never contain
/// `_` — which is exactly what frees the character to mean one thing. A directory name with no
/// boundary was not given a ticket, and that is the whole rule: nothing here inspects the *shape*
/// of a segment to decide whether it is one.
///
/// It replaced a rule that did. Any lowercase word followed by any all-digit word was read as a
/// Jira-style key, so `feat-reporting-2` reported issue `REPORTING-2`, emptied its own descriptive
/// remainder, and fell back to a label with the type token still in it. The shapes are genuinely
/// indistinguishable — `feat-abc-123` and `feat-auth-2` are one pattern — so no sharper heuristic
/// exists. The name has to carry the answer instead of implying it.
pub const TICKET_SEP: char = '_';

/// What a worktree directory name encodes, once split at the boundary.
struct NameParts {
    type_: Option<ConventionalType>,
    /// The slugified ticket exactly as [`derive`] wrote it — whatever shape it has, because it is
    /// what the user typed rather than something matched out of the name. `None` when there is no
    /// boundary, or nothing before it.
    ticket: Option<String>,
    /// The descriptive segments, separators dropped.
    desc: Vec<String>,
}

/// Split on `[-_]`, dropping empties — used for the descriptive half and for the never-empty
/// fallback, so both read a hand-made name the same way.
fn segments(s: &str) -> Vec<String> {
    s.split(['-', TICKET_SEP])
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

/// The single reading of a directory name that [`parse_tags`] and [`display_name`] both use, so
/// the chip and the label can never disagree about where the ticket ended.
fn split_name(dir_name: &str) -> NameParts {
    // Only the FIRST boundary divides: a derived name has at most one, and a hand-made name with
    // several still has to render rather than lose everything after the second.
    let (head, desc) = match dir_name.split_once(TICKET_SEP) {
        Some((head, desc)) => (head, Some(desc)),
        None => (dir_name, None),
    };
    let head_segments: Vec<&str> = head.split('-').filter(|s| !s.is_empty()).collect();
    let type_ = head_segments
        .first()
        .and_then(|token| ConventionalType::from_token(token));
    let after_type = &head_segments[usize::from(type_.is_some())..];

    match desc {
        Some(desc) => NameParts {
            type_,
            ticket: (!after_type.is_empty()).then(|| after_type.join("-")),
            desc: segments(desc),
        },
        // No boundary, so no ticket: everything after the type is descriptive.
        None => NameParts {
            type_,
            ticket: None,
            desc: after_type.iter().map(|s| s.to_string()).collect(),
        },
    }
}

/// Render a slugified ticket the way its tracker writes it: an all-digit reference is a
/// GitHub/GitLab issue number, anything else a Jira-style key.
///
/// The numeric case is not cosmetic — before the boundary existed, a ticket entered as `#123`
/// slugified to `123`, failed the old rule's "starts with a letter" test, and was dropped
/// entirely while its digits leaked into the display name.
fn issue_label(ticket: &str) -> String {
    if ticket.chars().all(|c| c.is_ascii_digit()) {
        format!("#{ticket}")
    } else {
        ticket.to_ascii_uppercase()
    }
}

/// Sentence-case a string whose remainder is already lowercase (slugified): upper-case only
/// the first character.
fn sentence_case(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

/// Parse the tags embedded in a worktree directory name (FR-002, FR-003, FR-008).
///
/// Returns at most one [`Tag::Type`] (when the leading segment is a known type) followed by at
/// most one [`Tag::Issue`] (when the name carries a [`TICKET_SEP`] with something before it).
/// Never returns a [`Tag::Status`] — health is injected by the caller. A non-conforming name
/// yields no `Type` tag (it is matched by the "untyped" filter bucket instead).
pub fn parse_tags(dir_name: &str) -> Vec<Tag> {
    let parts = split_name(dir_name);
    let mut tags = Vec::new();
    if let Some(type_) = parts.type_ {
        tags.push(Tag::Type(type_));
    }
    if let Some(ticket) = parts.ticket.as_deref() {
        tags.push(Tag::Issue(issue_label(ticket)));
    }
    tags
}

/// Derive a human-friendly display name from a worktree directory name (FR-017).
///
/// Removes the leading type token and the ticket, turns separators into spaces, and
/// sentence-cases. Never empty: a name with nothing descriptive left — a bare type, or a
/// boundary with nothing after it — falls back to a readable form of the whole `dir_name`,
/// because a blank row is worse than a redundant one.
pub fn display_name(dir_name: &str) -> String {
    let descriptive = sentence_case(&split_name(dir_name).desc.join(" "));
    if !descriptive.is_empty() {
        return descriptive;
    }
    let whole = sentence_case(&segments(dir_name).join(" "));
    if !whole.is_empty() {
        return whole;
    }
    // A name that is nothing but separators. Show it as it is rather than an empty row.
    sentence_case(dir_name)
}
