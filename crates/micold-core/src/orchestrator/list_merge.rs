//! Merging the per-daemon answers of a list (#1726 FR-001): the union of the rows, with every
//! daemon that gave none named and why. Pure; the orchestrator gathers the answers.

use crate::protocol::messages::ListRows;
use crate::protocol::routing::{ListKind, MissingDaemon, MissingReason, WireDaemonId};

/// What one linked daemon gave: its rows, or why it gave none.
pub type DaemonAnswer = (WireDaemonId, Result<ListRows, MissingReason>);

/// The merged answer of a list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Merged {
    /// Every row of every daemon that answered, daemons in id order, each daemon's rows as it sent
    /// them. Rows are daemon-qualified and never merged: one path on two daemons is two rows.
    pub rows: ListRows,
    /// Every daemon not in `rows`, in id order.
    pub missing: Vec<MissingDaemon>,
    /// No daemon was asked about at all.
    pub no_daemons: bool,
}

/// Union the `answers` of a `kind` list. A daemon is in the rows or in `missing`, never both: an
/// answer of the wrong kind counts as an error, and a daemon listed twice counts once (the first).
pub fn merge_lists(kind: ListKind, answers: Vec<DaemonAnswer>) -> Merged {
    let no_daemons = answers.is_empty();
    let mut answers = answers;
    answers.sort_by(|a, b| a.0.cmp(&b.0));
    answers.dedup_by(|later, first| later.0 == first.0);
    let mut rows = ListRows::empty(kind);
    let mut missing = Vec::new();
    for (daemon, answer) in answers {
        let reason = match answer {
            Ok(daemon_rows) if daemon_rows.kind() == kind => {
                rows.extend(daemon_rows);
                continue;
            }
            Ok(other) => MissingReason::Error(format!(
                "answered {:?} rows to a {kind:?} list",
                other.kind()
            )),
            Err(reason) => reason,
        };
        missing.push(MissingDaemon { daemon, reason });
    }
    Merged {
        rows,
        missing,
        no_daemons,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::project_identity::ProjectIdentity;
    use crate::protocol::messages::ProjectSnapshot;

    fn id(s: &str) -> WireDaemonId {
        WireDaemonId::new(s).unwrap()
    }

    fn project(daemon: &str, path: &str) -> ProjectSnapshot {
        ProjectSnapshot {
            daemon: id(daemon),
            identity: ProjectIdentity::new(format!("local:{daemon}:{path}")).unwrap(),
            path: PathBuf::from(path),
            display_name: path.to_owned(),
            is_git_repo: true,
            available: true,
            worktrees: Vec::new(),
            sessions: Vec::new(),
            pane_layout: None,
        }
    }

    fn projects(daemon: &str, paths: &[&str]) -> DaemonAnswer {
        let rows = paths.iter().map(|p| project(daemon, p)).collect();
        (id(daemon), Ok(ListRows::Projects(rows)))
    }

    fn paths(rows: &ListRows) -> Vec<(String, String)> {
        match rows {
            ListRows::Projects(p) => p
                .iter()
                .map(|p| (p.daemon.to_string(), p.path.display().to_string()))
                .collect(),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_rows_of_every_daemon_are_united_in_daemon_order() {
        let merged = merge_lists(
            ListKind::Projects,
            vec![projects("b", &["/y"]), projects("a", &["/x", "/z"])],
        );
        assert_eq!(
            paths(&merged.rows),
            [("a", "/x"), ("a", "/z"), ("b", "/y")].map(|(d, p)| (d.to_owned(), p.to_owned()))
        );
        assert!(merged.missing.is_empty());
        assert!(!merged.no_daemons);
    }

    #[test]
    fn one_path_on_two_daemons_stays_two_rows() {
        let merged = merge_lists(
            ListKind::Projects,
            vec![projects("a", &["/same"]), projects("b", &["/same"])],
        );
        assert_eq!(merged.rows.len(), 2);
    }

    #[test]
    fn a_daemon_is_in_the_rows_or_in_missing_never_both() {
        let merged = merge_lists(
            ListKind::Projects,
            vec![
                projects("a", &["/x"]),
                (id("b"), Err(MissingReason::Timeout)),
                (id("c"), Err(MissingReason::Offline)),
                (id("d"), Ok(ListRows::Sessions(vec![]))),
            ],
        );
        assert_eq!(merged.rows.len(), 1);
        let named: Vec<_> = merged.missing.iter().map(|m| m.daemon.as_str()).collect();
        assert_eq!(named, ["b", "c", "d"]);
        assert!(matches!(merged.missing[2].reason, MissingReason::Error(_)));
    }

    #[test]
    fn a_daemon_listed_twice_counts_once() {
        let merged = merge_lists(
            ListKind::Projects,
            vec![projects("a", &["/x"]), projects("a", &["/x"])],
        );
        assert_eq!(merged.rows.len(), 1);
    }

    #[test]
    fn no_daemons_is_flagged_and_is_not_an_error() {
        let merged = merge_lists(ListKind::Sessions, vec![]);
        assert!(merged.no_daemons);
        assert!(merged.rows.is_empty());
        assert!(merged.missing.is_empty());
        assert_eq!(merged.rows.kind(), ListKind::Sessions);
    }

    #[test]
    fn daemons_that_all_failed_are_not_no_daemons() {
        let merged = merge_lists(
            ListKind::Sessions,
            vec![(id("a"), Err(MissingReason::Offline))],
        );
        assert!(!merged.no_daemons);
        assert_eq!(merged.missing.len(), 1);
    }
}
