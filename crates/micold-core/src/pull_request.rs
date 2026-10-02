//! Pull requests of a project's branches, read through the user's own `gh` (feature 040,
//! contracts/pull-request-source.md).
//!
//! Everything here is held in memory only (FR-032): [`PullRequestStatus`] cannot be serialised, and
//! its `Debug` output leaves out the title and the address, so neither reaches a stored file or a
//! log. `tests/pull_request_is_never_stored.rs` holds both.

use std::fmt;

/// What one reading found for one branch (data-model §1).
#[derive(Clone, PartialEq, Eq)]
pub struct PullRequestStatus {
    /// The pull request's number in its repository.
    pub number: u64,
    /// Its title. Never logged, never serialised.
    pub title: String,
    /// Its address on GitHub. Never logged, never serialised.
    pub url: String,
    /// Open, draft, merged or closed; an open or draft one carries its check status.
    pub state: PrState,
    /// GitHub's review decision.
    pub review: ReviewState,
    /// The commit the pull request ends at (`headRefOid`).
    pub head: String,
}

/// Prints the number, the state and the review, and nothing else (FR-032).
impl fmt::Debug for PullRequestStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PullRequestStatus")
            .field("number", &self.number)
            .field("state", &self.state)
            .field("review", &self.review)
            .finish_non_exhaustive()
    }
}

/// The one state a pull request is shown in (FR-002). Only an open or a draft pull request has a
/// check status (FR-003): a merged or closed one with checks cannot be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrState {
    /// Open and ready for review.
    Open {
        /// The combined result of its checks.
        checks: CheckStatus,
    },
    /// Open, marked as a draft.
    Draft {
        /// The combined result of its checks.
        checks: CheckStatus,
    },
    /// Merged.
    Merged,
    /// Closed without merging.
    Closed,
}

/// The combined result of a pull request's checks (FR-008).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    /// The pull request has no checks at all.
    None,
    /// No check failed, and at least one is queued or running.
    Pending,
    /// Every check finished without a failure.
    Passing,
    /// At least one check failed, was cancelled, timed out or needs action.
    Failing,
}

/// GitHub's review decision for a pull request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewState {
    /// No decision: the repository asks for no review, or GitHub named one this version does not
    /// know.
    None,
    /// Approved.
    Approved,
    /// Changes were requested.
    ChangesRequested,
    /// A review is required and has not been given.
    ReviewRequired,
}

/// Why a reading produced no statuses (data-model §2). No variant carries text: nothing about a
/// failure is shown (FR-025).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadingFailure {
    /// Pull requests cannot be read at all: no GitHub remote, `gh` missing, not signed in, or the
    /// sign-in cannot see the repository. What a row shows is removed (FR-025).
    Unavailable,
    /// Waiting may help: offline, no answer in time, an answer that cannot be understood. What a
    /// row shows stays (FR-019).
    Passing,
    /// GitHub's request limit was reached; nothing is sent before `until` (FR-024).
    RateLimited {
        /// Unix seconds.
        until: u64,
    },
}
