//! Shared fixture for the review bar and drawer tests (feature 1270): review comments of a given
//! scope and status, built without the operations that move a comment through its states.
//! Included with `#[path = "support/review_comments.rs"] mod review_comments;`.

#![allow(dead_code)]

use micold_core::review::comment::{
    CommentId, CommentState, PrAnchor, ReviewComment, Scope, Status,
};
use micold_core::review::{LineRange, RelPath, Side};
use serde_json::json;

/// Every status, in the order the bar lists them.
pub const STATUSES: [Status; 4] = [
    Status::Draft,
    Status::Sent,
    Status::Applied,
    Status::Answered,
];

/// A state whose `status()` is `status`.
pub fn state(status: Status) -> CommentState {
    let value = match status {
        Status::Draft => json!({ "draft": null }),
        Status::Sent => json!({ "sent": { "at": 10 } }),
        Status::Applied | Status::Answered => json!({ "reported": {
            "sent_at": 10,
            "outcome": if status == Status::Applied { "applied" } else { "answered" },
            "reply": "done",
            "first_at": 11,
            "last_at": 11,
            "by": "user",
        } }),
    };
    serde_json::from_value(value).expect("a valid comment state")
}

/// A one-line comment on the new side of `path` at `line`, made at `created`.
pub fn comment(scope: Scope, status: Status, path: &str, line: u32, created: u64) -> ReviewComment {
    ReviewComment {
        id: CommentId::new(),
        path: RelPath::from_native(path).expect("a relative path"),
        side: Side::New,
        range: LineRange::new(line, line).expect("a line from 1"),
        quote: vec![String::from("line")],
        text: String::from("a comment"),
        state: state(status),
        created,
        pr: match scope {
            Scope::WorkingTree => None,
            Scope::PullRequest(number) => {
                Some(PrAnchor::checked(number, "feature", &"a".repeat(40)).expect("an anchor"))
            }
        },
    }
}

/// `n` comments of `scope` with `status`.
pub fn many(scope: Scope, status: Status, n: usize) -> Vec<ReviewComment> {
    (0..n)
        .map(|i| comment(scope, status, "src/lib.rs", 1, i as u64))
        .collect()
}
