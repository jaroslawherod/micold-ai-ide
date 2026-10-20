//! A Jira ticket's current status (feature 1101, data-model.md).
//!
//! The status name is what Jira calls the step ("In Review"); the category is the coarse group
//! Jira files every status under, which is what a chip is tinted by.

/// The group Jira files a status under (`statusCategory.key`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StatusCategory {
    /// `new`: not started.
    ToDo,
    /// `indeterminate`: under way.
    InProgress,
    /// `done`: finished.
    Done,
}

impl StatusCategory {
    /// The category for a Jira `statusCategory.key`; `None` for a key this does not know.
    pub fn from_key(key: &str) -> Option<StatusCategory> {
        match key {
            "new" => Some(StatusCategory::ToDo),
            "indeterminate" => Some(StatusCategory::InProgress),
            "done" => Some(StatusCategory::Done),
            _ => None,
        }
    }
}

/// A ticket's status: its name and, when Jira reported a known one, its category.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TicketStatus {
    /// Trimmed and never empty.
    pub name: String,
    /// `None` renders the name in a neutral tint.
    pub category: Option<StatusCategory>,
}

impl TicketStatus {
    /// The status Jira reported, or `None` when the name is missing or blank.
    pub fn from_jira(name: Option<&str>, category_key: Option<&str>) -> Option<TicketStatus> {
        let name = name?.trim();
        if name.is_empty() {
            return None;
        }
        Some(TicketStatus {
            name: name.to_string(),
            category: category_key.and_then(StatusCategory::from_key),
        })
    }

    /// The text a screen reader and the chip's tooltip give (FR-013): the name, and "may be out
    /// of date" when the status is stale.
    pub fn accessible_text(&self, stale: bool) -> String {
        if stale {
            format!("{}, may be out of date", self.name)
        } else {
            self.name.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_keys_map_to_categories() {
        for (key, want) in [
            ("new", StatusCategory::ToDo),
            ("indeterminate", StatusCategory::InProgress),
            ("done", StatusCategory::Done),
        ] {
            let status = TicketStatus::from_jira(Some("X"), Some(key)).unwrap();
            assert_eq!(status.category, Some(want), "{key}");
        }
    }

    #[test]
    fn an_unknown_or_missing_key_gives_no_category() {
        assert_eq!(
            TicketStatus::from_jira(Some("X"), Some("undefined"))
                .unwrap()
                .category,
            None
        );
        assert_eq!(
            TicketStatus::from_jira(Some("X"), None).unwrap().category,
            None
        );
    }

    #[test]
    fn a_blank_or_missing_name_gives_no_status() {
        assert_eq!(TicketStatus::from_jira(None, Some("new")), None);
        assert_eq!(TicketStatus::from_jira(Some("  \t"), Some("new")), None);
    }

    #[test]
    fn the_name_is_trimmed() {
        let status = TicketStatus::from_jira(Some("  In Review "), None).unwrap();
        assert_eq!(status.name, "In Review");
    }

    #[test]
    fn accessible_text_adds_staleness() {
        let status = TicketStatus::from_jira(Some("In Review"), Some("indeterminate")).unwrap();
        assert_eq!(status.accessible_text(false), "In Review");
        assert_eq!(
            status.accessible_text(true),
            "In Review, may be out of date"
        );
    }
}
