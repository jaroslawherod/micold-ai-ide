//! The label-to-type mapping (feature 034, FR-013–FR-021): an ordered, application-wide list that
//! turns an issue's labels into the worktree type the create form pre-selects.
//!
//! Pure; no I/O. Contract: `specs/034-github-issue-worktree/contracts/issue-naming-and-typing.md` §2.

use crate::naming::ConventionalType;
use serde::{Deserialize, Serialize};

/// One mapping entry: an issue label and the worktree type it selects. Stored as
/// `{"label": …, "type": …}` (contract §3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabelTypeEntry {
    pub label: String,
    #[serde(rename = "type")]
    pub type_: ConventionalType,
}

/// The mapping every install starts with (FR-021).
pub fn default_mapping() -> Vec<LabelTypeEntry> {
    [
        ("bug", ConventionalType::Fix),
        ("enhancement", ConventionalType::Feat),
        ("documentation", ConventionalType::Docs),
    ]
    .into_iter()
    .map(|(label, type_)| LabelTypeEntry {
        label: label.to_string(),
        type_,
    })
    .collect()
}

/// The type of the first mapping entry, in mapping order, whose label matches any of `labels`
/// (FR-013, AS2). Both sides are trimmed and lower-cased (AS5); no match is `None` (AS3).
pub fn type_for_labels(mapping: &[LabelTypeEntry], labels: &[String]) -> Option<ConventionalType> {
    let labels: Vec<String> = labels.iter().map(|l| fold(l)).collect();
    mapping
        .iter()
        .find(|entry| labels.contains(&fold(&entry.label)))
        .map(|entry| entry.type_)
}

/// The first entry, in order, that cannot be saved (FR-019).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MappingError {
    pub index: usize,
    pub kind: MappingErrorKind,
}

/// Why a mapping entry cannot be saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingErrorKind {
    /// The label is blank after trimming.
    Blank,
    /// The label repeats the entry at `of`, ignoring case.
    Duplicate { of: usize },
}

/// Checks a mapping before it is saved: the first entry, in order, whose label is blank after
/// trimming or repeats an earlier entry's label ignoring case (FR-019). Several labels may map to
/// one type.
pub fn validate_mapping(mapping: &[LabelTypeEntry]) -> Result<(), MappingError> {
    let folded: Vec<String> = mapping.iter().map(|entry| fold(&entry.label)).collect();
    for (index, label) in folded.iter().enumerate() {
        let kind = if label.is_empty() {
            MappingErrorKind::Blank
        } else if let Some(of) = folded[..index].iter().position(|earlier| earlier == label) {
            MappingErrorKind::Duplicate { of }
        } else {
            continue;
        };
        return Err(MappingError { index, kind });
    }
    Ok(())
}

/// The form a label is compared in: trimmed, lower-cased.
fn fold(label: &str) -> String {
    label.trim().to_lowercase()
}
