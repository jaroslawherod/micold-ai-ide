//! The statuses of the Jira tickets worktrees are linked to (feature 1101, contracts/status-chip-ui.md).
//!
//! One [`StatusBoard`](micold_core::jira_status::StatusBoard) for the open project, read by every
//! surface (FR-010). This module decides *when* a read starts and what its answer changes; the
//! request itself is the shell's (`shell/jira_status.rs`), behind feature 533's single Jira gate.
//!
//! A read starts on two events only (FR-005): the project opening, once its worktrees are known,
//! and the window regaining focus when the last read is over five minutes old.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use micold_core::github::IssueLoadError;
use micold_core::jira::ProjectKey;
use micold_core::jira_status::{statuses_jql, KeyedStatus, StatusBoard, Trigger};
use micold_core::naming::Tag;
use micold_core::worktree::Worktree;

use super::worktree::worktree_tags;

/// A read to run: the board's epoch when it began, and the keys to ask for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Read {
    /// Answer with this epoch.
    pub epoch: u64,
    /// The validated keys, in the order the worktrees list them.
    pub keys: Vec<String>,
}

/// What the Jira status feature remembers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// The statuses, for the open project.
    pub board: StatusBoard,
    /// The project opened and its worktrees were not known yet: the open read waits for them.
    pub open_pending: bool,
}

/// The Jira keys of `worktrees` of `project`: the issue tag of each worktree's name, uppercased,
/// kept when it is `<PROJECT>-<digits>`, each once.
pub fn linked_keys(worktrees: &[Worktree], project: &ProjectKey) -> Vec<String> {
    let labels: Vec<String> = worktrees
        .iter()
        .flat_map(|w| worktree_tags(w))
        .filter_map(|tag| match tag {
            Tag::Issue(label) => Some(label),
            _ => None,
        })
        .collect();
    statuses_jql(project, &labels)
        .into_iter()
        .flat_map(|query| query.keys)
        .collect()
}

/// The refresh clock: time since the Unix epoch. Tests pass their own `now` to [`State::plan`].
pub fn now() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
}

impl State {
    /// The project opened (or changed): forget the last one's statuses, and read the new one's once
    /// its worktrees are known.
    pub fn project_opened(&mut self) {
        self.board.clear();
        self.open_pending = true;
    }

    /// The statuses are no longer wanted (switch off, connection removed).
    pub fn clear(&mut self) {
        self.board.clear();
        self.open_pending = false;
    }

    /// Begin a read for `trigger`, or `None` when it is not due (one is out; the last read is
    /// recent) or no worktree is linked to a ticket of `project`.
    pub fn plan(
        &mut self,
        trigger: Trigger,
        now: Duration,
        worktrees: &[Worktree],
        project: &ProjectKey,
    ) -> Option<Read> {
        if !self.board.due(now, trigger) {
            return None;
        }
        let (epoch, keys) = self.board.begin(linked_keys(worktrees, project))?;
        Some(Read { epoch, keys })
    }

    /// Take the answer of a read.
    pub fn apply(
        &mut self,
        epoch: u64,
        keys: &[String],
        result: Result<Vec<KeyedStatus>, IssueLoadError>,
        at: Duration,
    ) {
        self.board.apply(epoch, keys, result, at);
    }
}
