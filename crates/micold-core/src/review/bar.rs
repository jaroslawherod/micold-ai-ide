//! The Review tab's bottom bar (feature 1270, data model § Core: `review::bar`): the counts of
//! the shown diff, the **Send review (n)** button and the overflow menu's items, decided here
//! without drawing (FR-002, FR-003, FR-005–FR-007, FR-023).
//! See `specs/1270-review-bar-drawer/data-model.md`.

use super::comment::{ReviewComment, Scope, Status};

/// How many comments of one scope (the working tree, or one pull request) are in each status.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScopeCounts {
    /// Not sent yet.
    pub draft: usize,
    /// Sent, no report yet.
    pub sent: usize,
    /// Reported as applied.
    pub applied: usize,
    /// Reported as answered.
    pub answered: usize,
}

impl ScopeCounts {
    /// The counts of `scope`'s comments only (FR-002, Story 2 #5).
    pub fn of(comments: &[ReviewComment], scope: Scope) -> Self {
        let mut counts = Self::default();
        for comment in comments.iter().filter(|c| c.scope() == scope) {
            match comment.state.status() {
                Status::Draft => counts.draft += 1,
                Status::Sent => counts.sent += 1,
                Status::Applied => counts.applied += 1,
                Status::Answered => counts.answered += 1,
            }
        }
        counts
    }

    /// Whether the scope has no comments at all.
    pub fn is_empty(&self) -> bool {
        self.draft == 0 && self.reported_or_sent() == 0
    }

    /// The comments **Clear sent** removes: sent, applied and answered (research R2).
    pub fn reported_or_sent(&self) -> usize {
        self.sent + self.applied + self.answered
    }
}

/// How much of the counts the bar has room for, longest first (research R3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountsForm {
    /// Every non-zero status, or "No review comments yet".
    Full,
    /// The drafts only.
    DraftsOnly,
    /// Nothing.
    Hidden,
}

/// The counts as the bar writes them in `form`; `None` when that form shows nothing (FR-002).
pub fn counts_text(counts: &ScopeCounts, form: CountsForm) -> Option<String> {
    let drafts = |n: usize| format!("{n} {}", if n == 1 { "draft" } else { "drafts" });
    match form {
        CountsForm::Full if counts.is_empty() => Some("No review comments yet".to_owned()),
        CountsForm::Full => {
            let parts = [
                (counts.draft > 0).then(|| drafts(counts.draft)),
                (counts.sent > 0).then(|| format!("{} sent", counts.sent)),
                (counts.applied > 0).then(|| format!("{} applied", counts.applied)),
                (counts.answered > 0).then(|| format!("{} answered", counts.answered)),
            ];
            Some(parts.into_iter().flatten().collect::<Vec<_>>().join(" · "))
        }
        CountsForm::DraftsOnly => (counts.draft > 0).then(|| drafts(counts.draft)),
        CountsForm::Hidden => None,
    }
}

/// The most drafts the send button's label spells out; more read as `999+` (research R4).
const LABEL_MAX: usize = 999;

/// The bar's send button.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendButton {
    /// `"Sending…"`, or `"Send review (n)"` with n capped as `"999+"`.
    pub label: String,
    /// Whether pressing it sends.
    pub enabled: bool,
    /// Whether it is drawn filled; always equal to `enabled`.
    pub primary: bool,
}

/// The send button for `counts`: `"Sending…"` and disabled while a send of the entry is under way
/// in any window (FR-005); otherwise enabled when there are drafts and the shown diff can be sent
/// (`sendable` is false for a pull request not in the list) (FR-003).
pub fn send_button(counts: &ScopeCounts, sending: bool, sendable: bool) -> SendButton {
    let label = if sending {
        "Sending…".to_owned()
    } else if counts.draft > LABEL_MAX {
        format!("Send review ({LABEL_MAX}+)")
    } else {
        format!("Send review ({})", counts.draft)
    };
    let enabled = !sending && sendable && counts.draft > 0;
    SendButton {
        label,
        enabled,
        primary: enabled,
    }
}

/// Which of the overflow menu's items are available (FR-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Overflow {
    /// **Clear sent**: the scope has sent, applied or answered comments.
    pub clear_sent: bool,
    /// **Discard drafts…**: the scope has drafts and no send of the entry is under way.
    pub discard_drafts: bool,
}

/// The overflow items' availability for `counts` (FR-007, research R2).
pub fn overflow(counts: &ScopeCounts, sending: bool) -> Overflow {
    Overflow {
        clear_sent: counts.reported_or_sent() > 0,
        discard_drafts: counts.draft > 0 && !sending,
    }
}

/// The bar's message after a failed send, today's wording (FR-006).
pub fn error_text(why: &str) -> String {
    format!("Couldn't send the review: {why}")
}
