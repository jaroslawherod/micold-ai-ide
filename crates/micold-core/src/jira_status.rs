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

use std::collections::BTreeMap;
use std::time::Duration;

use crate::github::IssueLoadError;
use crate::jira::{ApiToken, JiraConnection, JiraSource, ProjectKey};

/// Keys one status request carries at most (contracts/jira-status-source.md §3).
pub const CHUNK: usize = 100;

/// How old the last read must be before a window focus asks again (FR-005).
pub const FOCUS_REFRESH_AFTER: Duration = Duration::from_secs(5 * 60);

/// Consecutive failed refreshes after which a status is retired (FR-008).
const FAILURES_BEFORE_RETIRING: u8 = 3;

/// One status request: the keys it asks for and the JQL that asks for them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusQuery {
    /// The validated, uppercased keys, at most [`CHUNK`].
    pub keys: Vec<String>,
    /// `project = "KEY" AND key in (...) ORDER BY key ASC`.
    pub jql: String,
}

/// Whether `key` is `<project>-<digits>`.
fn is_key_of_project(key: &str, project: &ProjectKey) -> bool {
    key.strip_prefix(project.as_str())
        .and_then(|rest| rest.strip_prefix('-'))
        .is_some_and(|digits| !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()))
}

/// The JQL that reads exactly `keys` (already validated, see [`statuses_jql`]).
pub fn status_jql(project: &ProjectKey, keys: &[String]) -> String {
    let list: Vec<String> = keys.iter().map(|k| format!("\"{k}\"")).collect();
    format!(
        "project = \"{}\" AND key in ({}) ORDER BY key ASC",
        project.as_str(),
        list.join(",")
    )
}

/// The requests that read `keys` of `project`: keys are uppercased and kept only when they are
/// `<PROJECT>-<digits>` (so no quote can enter the JQL), duplicates dropped, then cut into chunks
/// of [`CHUNK`] in order. No done filter: a finished ticket is read too (FR-007).
pub fn statuses_jql(project: &ProjectKey, keys: &[String]) -> Vec<StatusQuery> {
    let mut valid: Vec<String> = Vec::new();
    for key in keys {
        let key = key.to_ascii_uppercase();
        if is_key_of_project(&key, project) && !valid.contains(&key) {
            valid.push(key);
        }
    }
    valid
        .chunks(CHUNK)
        .map(|chunk| StatusQuery {
            keys: chunk.to_vec(),
            jql: status_jql(project, chunk),
        })
        .collect()
}

/// One ticket's key and its usable status.
pub type KeyedStatus = (String, Option<TicketStatus>);

/// Parse the body of a status search (contracts/jira-status-source.md §4). Non-JSON, or no `issues`
/// array, is [`IssueLoadError::NotJiraCloud`].
pub fn parse_statuses(body: &[u8]) -> Result<Vec<KeyedStatus>, IssueLoadError> {
    let json: serde_json::Value =
        serde_json::from_slice(body).map_err(|_| IssueLoadError::NotJiraCloud)?;
    let nodes = json["issues"]
        .as_array()
        .ok_or(IssueLoadError::NotJiraCloud)?;
    Ok(nodes
        .iter()
        .filter_map(|node| {
            let key = node["key"].as_str()?.to_string();
            let status = &node["fields"]["status"];
            Some((
                key,
                TicketStatus::from_jira(
                    status["name"].as_str(),
                    status["statusCategory"]["key"].as_str(),
                ),
            ))
        })
        .collect())
}

/// Read the statuses of `keys` through `source`, one request per chunk of [`CHUNK`]; the first
/// failure aborts the rest (contracts §3).
pub fn read_statuses(
    source: &dyn JiraSource,
    conn: &JiraConnection,
    token: &ApiToken,
    keys: &[String],
) -> Result<Vec<KeyedStatus>, IssueLoadError> {
    let mut out = Vec::new();
    for query in statuses_jql(&conn.project_key, keys) {
        out.extend(source.statuses(conn, token, &query.keys)?);
    }
    Ok(out)
}

/// What asks for a refresh (FR-005): nothing else does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    /// The project opened.
    Open,
    /// The window regained focus.
    Focus,
}

/// A status the board holds, and whether the last refresh failed to confirm it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusEntry {
    /// The status last read.
    pub status: TicketStatus,
    /// The last refresh failed: the status may be out of date (FR-008).
    pub stale: bool,
}

/// The statuses of the open project's linked tickets, read by every surface (FR-010).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatusBoard {
    entries: BTreeMap<String, StatusEntry>,
    /// Monotonic time of the last `Ok` read.
    pub last_read: Option<Duration>,
    /// Consecutive failed refreshes.
    pub failures: u8,
    /// A request is out.
    pub in_flight: bool,
    epoch: u64,
}

impl StatusBoard {
    /// Whether `trigger` should start a read at `now`.
    pub fn due(&self, now: Duration, trigger: Trigger) -> bool {
        if self.in_flight {
            return false;
        }
        match trigger {
            Trigger::Open => true,
            Trigger::Focus => self
                .last_read
                .is_none_or(|read| now.saturating_sub(read) > FOCUS_REFRESH_AFTER),
        }
    }

    /// Start a read of `keys`: the epoch to answer with and the keys, or `None` when there are none.
    pub fn begin(&mut self, keys: Vec<String>) -> Option<(u64, Vec<String>)> {
        if keys.is_empty() {
            return None;
        }
        self.in_flight = true;
        Some((self.epoch, keys))
    }

    /// Take the answer of a read begun in `epoch` for `asked`. An answer from before a
    /// [`clear`](Self::clear) is dropped.
    pub fn apply(
        &mut self,
        epoch: u64,
        asked: &[String],
        result: Result<Vec<KeyedStatus>, IssueLoadError>,
        now: Duration,
    ) {
        if epoch != self.epoch {
            return;
        }
        self.in_flight = false;
        match result {
            Ok(read) => {
                for key in asked {
                    self.entries.remove(key);
                }
                for (key, status) in read {
                    if let (true, Some(status)) = (asked.contains(&key), status) {
                        self.entries.insert(
                            key,
                            StatusEntry {
                                status,
                                stale: false,
                            },
                        );
                    }
                }
                self.failures = 0;
                self.last_read = Some(now);
            }
            Err(_) => {
                self.failures = self.failures.saturating_add(1);
                if self.failures >= FAILURES_BEFORE_RETIRING {
                    self.entries.clear();
                } else {
                    for entry in self.entries.values_mut() {
                        entry.stale = true;
                    }
                }
            }
        }
    }

    /// Forget everything (switch off, connection removed, project switch); an answer still on its
    /// way is dropped.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.last_read = None;
        self.failures = 0;
        self.in_flight = false;
        self.epoch += 1;
    }

    /// Nothing held and nothing out: a [`clear`](Self::clear) would change nothing.
    pub fn is_idle(&self) -> bool {
        self.entries.is_empty() && !self.in_flight
    }

    /// The status of `key`: the one read path of every surface (FR-010).
    pub fn status_of(&self, key: &str) -> Option<&StatusEntry> {
        self.entries.get(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::IssueLoadError;
    use crate::jira::ProjectKey;
    use std::time::Duration;

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

    // --- T014: statuses_jql -------------------------------------------------------------

    fn project() -> ProjectKey {
        ProjectKey::parse("ABC").unwrap()
    }

    fn keys(range: std::ops::RangeInclusive<u32>) -> Vec<String> {
        range.map(|n| format!("ABC-{n}")).collect()
    }

    #[test]
    fn keys_are_validated_and_uppercased_and_others_dropped() {
        let asked = vec![
            "abc-7".to_string(),
            "ABC-8\") OR project = \"X".to_string(),
            "XYZ-1".to_string(),
            "ABC-".to_string(),
            "ABC-x".to_string(),
            "ABC-7".to_string(),
        ];
        let queries = statuses_jql(&project(), &asked);
        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].keys, vec!["ABC-7".to_string()]);
        assert_eq!(
            queries[0].jql,
            "project = \"ABC\" AND key in (\"ABC-7\") ORDER BY key ASC"
        );
    }

    #[test]
    fn no_usable_key_gives_no_query() {
        assert!(statuses_jql(&project(), &["XYZ-1".to_string()]).is_empty());
        assert!(statuses_jql(&project(), &[]).is_empty());
    }

    #[test]
    fn keys_go_in_chunks_of_100_in_order() {
        let all = keys(1..=101);
        let queries = statuses_jql(&project(), &all);
        assert_eq!(queries.len(), 2);
        assert_eq!(queries[0].keys, keys(1..=100));
        assert_eq!(queries[1].keys, keys(101..=101));
        let fifty = statuses_jql(&project(), &keys(1..=50));
        assert_eq!(fifty.len(), 1);
    }

    #[test]
    fn the_query_has_no_done_filter() {
        let queries = statuses_jql(&project(), &keys(1..=2));
        assert!(!queries[0].jql.contains("statusCategory"));
        assert!(!queries[0].jql.contains("resolution"));
    }

    // --- T015: parse_statuses -----------------------------------------------------------

    #[test]
    fn statuses_parse_by_key() {
        let body = br#"{"issues":[
            {"key":"ABC-1","fields":{"status":{"name":"In Review","statusCategory":{"key":"indeterminate"}}}},
            {"key":"ABC-2","fields":{"status":{"name":" ","statusCategory":{"key":"new"}}}},
            {"key":"ABC-3","fields":{}}]}"#;
        let parsed = parse_statuses(body).unwrap();
        assert_eq!(parsed.len(), 3);
        assert_eq!(
            parsed[0],
            (
                "ABC-1".to_string(),
                Some(TicketStatus {
                    name: "In Review".into(),
                    category: Some(StatusCategory::InProgress)
                })
            )
        );
        assert_eq!(parsed[1], ("ABC-2".to_string(), None));
        assert_eq!(parsed[2], ("ABC-3".to_string(), None));
    }

    #[test]
    fn a_body_that_is_not_jira_gives_not_jira_cloud() {
        assert_eq!(parse_statuses(b"<html>"), Err(IssueLoadError::NotJiraCloud));
        assert_eq!(parse_statuses(b"{}"), Err(IssueLoadError::NotJiraCloud));
    }

    // --- T016: StatusBoard --------------------------------------------------------------

    fn st(name: &str) -> TicketStatus {
        TicketStatus::from_jira(Some(name), Some("new")).unwrap()
    }

    const MIN: Duration = Duration::from_secs(60);

    fn begun(board: &mut StatusBoard, asked: &[&str]) -> (u64, Vec<String>) {
        board
            .begin(asked.iter().map(|k| k.to_string()).collect())
            .expect("begin")
    }

    fn ok(items: &[(&str, Option<&str>)]) -> Result<Vec<(String, Option<TicketStatus>)>, IssueLoadError> {
        Ok(items
            .iter()
            .map(|(k, s)| (k.to_string(), s.map(st)))
            .collect())
    }

    #[test]
    fn open_is_due_unless_in_flight() {
        let mut board = StatusBoard::default();
        assert!(board.due(MIN, Trigger::Open));
        let _ = begun(&mut board, &["ABC-1"]);
        assert!(!board.due(MIN, Trigger::Open));
        assert!(!board.due(MIN, Trigger::Focus));
    }

    #[test]
    fn focus_is_due_only_after_five_minutes() {
        let mut board = StatusBoard::default();
        assert!(board.due(Duration::ZERO, Trigger::Focus), "never read");
        let (epoch, asked) = begun(&mut board, &["ABC-1"]);
        board.apply(epoch, &asked, ok(&[("ABC-1", Some("To Do"))]), 10 * MIN);
        assert!(!board.due(10 * MIN + 5 * MIN, Trigger::Focus), "exactly 5 min");
        assert!(board.due(10 * MIN + 5 * MIN + Duration::from_secs(1), Trigger::Focus));
        assert!(board.due(11 * MIN, Trigger::Open));
    }

    #[test]
    fn begin_with_no_keys_asks_nothing() {
        let mut board = StatusBoard::default();
        assert_eq!(board.begin(Vec::new()), None);
        assert!(board.due(MIN, Trigger::Open), "nothing in flight");
    }

    #[test]
    fn an_ok_read_replaces_and_removes_absent_keys() {
        let mut board = StatusBoard::default();
        let (epoch, asked) = begun(&mut board, &["ABC-1", "ABC-2"]);
        board.apply(epoch, &asked, ok(&[("ABC-1", Some("A")), ("ABC-2", Some("B"))]), MIN);
        let (epoch, asked) = begun(&mut board, &["ABC-1", "ABC-2"]);
        board.apply(epoch, &asked, ok(&[("ABC-1", Some("C"))]), 2 * MIN);
        assert_eq!(board.status_of("ABC-1").unwrap().status.name, "C");
        assert!(!board.status_of("ABC-1").unwrap().stale);
        assert!(board.status_of("ABC-2").is_none());
        assert_eq!(board.failures, 0);
        assert_eq!(board.last_read, Some(2 * MIN));
        assert!(!board.in_flight);
    }

    #[test]
    fn an_unusable_status_removes_the_entry() {
        let mut board = StatusBoard::default();
        let (epoch, asked) = begun(&mut board, &["ABC-1"]);
        board.apply(epoch, &asked, ok(&[("ABC-1", Some("A"))]), MIN);
        let (epoch, asked) = begun(&mut board, &["ABC-1"]);
        board.apply(epoch, &asked, ok(&[("ABC-1", None)]), 2 * MIN);
        assert!(board.status_of("ABC-1").is_none());
    }

    #[test]
    fn a_failure_marks_stale_and_the_third_clears() {
        let mut board = StatusBoard::default();
        let (epoch, asked) = begun(&mut board, &["ABC-1"]);
        board.apply(epoch, &asked, ok(&[("ABC-1", Some("A"))]), MIN);
        for round in 1..=2u8 {
            let (epoch, asked) = begun(&mut board, &["ABC-1"]);
            board.apply(epoch, &asked, Err(IssueLoadError::RateLimited), 2 * MIN);
            let entry = board.status_of("ABC-1").expect("kept");
            assert!(entry.stale);
            assert_eq!(board.failures, round);
            assert!(!board.in_flight);
        }
        assert_eq!(board.last_read, Some(MIN), "a failure is not a read");
        let (epoch, asked) = begun(&mut board, &["ABC-1"]);
        board.apply(epoch, &asked, Err(IssueLoadError::TimedOut), 3 * MIN);
        assert!(board.status_of("ABC-1").is_none());
    }

    #[test]
    fn an_ok_read_after_a_failure_is_fresh_again() {
        let mut board = StatusBoard::default();
        let (epoch, asked) = begun(&mut board, &["ABC-1"]);
        board.apply(epoch, &asked, ok(&[("ABC-1", Some("A"))]), MIN);
        let (epoch, asked) = begun(&mut board, &["ABC-1"]);
        board.apply(epoch, &asked, Err(IssueLoadError::TimedOut), 2 * MIN);
        let (epoch, asked) = begun(&mut board, &["ABC-1"]);
        board.apply(epoch, &asked, ok(&[("ABC-1", Some("A"))]), 3 * MIN);
        assert!(!board.status_of("ABC-1").unwrap().stale);
        assert_eq!(board.failures, 0);
    }

    #[test]
    fn an_answer_from_before_a_clear_is_dropped() {
        let mut board = StatusBoard::default();
        let (epoch, asked) = begun(&mut board, &["ABC-1"]);
        board.clear();
        assert!(!board.in_flight);
        board.apply(epoch, &asked, ok(&[("ABC-1", Some("A"))]), MIN);
        assert!(board.status_of("ABC-1").is_none());
        assert_eq!(board.last_read, None);
    }

    #[test]
    fn clear_drops_everything() {
        let mut board = StatusBoard::default();
        let (epoch, asked) = begun(&mut board, &["ABC-1"]);
        board.apply(epoch, &asked, ok(&[("ABC-1", Some("A"))]), MIN);
        board.clear();
        assert!(board.status_of("ABC-1").is_none());
        assert_eq!(board.last_read, None);
        assert_eq!(board.failures, 0);
    }
}
