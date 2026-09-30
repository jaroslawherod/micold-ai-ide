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

/// The form a label is compared in: trimmed, lower-cased.
fn fold(label: &str) -> String {
    label.trim().to_lowercase()
}
