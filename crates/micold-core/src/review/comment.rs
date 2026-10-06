//! Review comments (feature 482, data-model "Core: review comments"): what one comment holds and
//! its state. The operations on an entry's comments arrive with the daemon's review store.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{LineRange, RelPath, Side};

/// A comment's identity: a v4 UUID the daemon assigns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CommentId(pub Uuid);

impl CommentId {
    /// A fresh random id.
    pub fn new() -> Self {
        Self(Uuid::nil())
    }
}

impl Default for CommentId {
    fn default() -> Self {
        Self::new()
    }
}

/// Whether a comment has reached its session. `Pending → Sent` only on delivery (FR-017), and
/// never back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommentState {
    /// Not yet delivered.
    Pending,
    /// Typed into a session at `at` (Unix seconds).
    Sent {
        /// When the prompt was written.
        at: u64,
    },
}

/// One comment on a line range of one side of one file (FR-011, FR-012).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewComment {
    /// Its identity.
    pub id: CommentId,
    /// The file, relative to the entry root.
    pub path: RelPath,
    /// Which version the lines are numbered in.
    pub side: Side,
    /// The lines it is about.
    pub range: LineRange,
    /// Those lines' text when the comment was made, one entry per line.
    pub quote: Vec<String>,
    /// What the user wrote, trimmed and non-empty.
    pub text: String,
    /// Pending or sent.
    pub state: CommentState,
    /// When it was made (Unix seconds).
    pub created: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample(state: CommentState) -> ReviewComment {
        ReviewComment {
            id: CommentId(Uuid::from_u128(0x0123_4567_89ab_4def_8123_4567_89ab_cdef)),
            path: RelPath::from_git("src/a.rs"),
            side: Side::New,
            range: LineRange::new(12, 14).expect("12..=14"),
            quote: vec!["a".into(), "b".into(), "c".into()],
            text: "Why clone here?".into(),
            state,
            created: 1_790_000_000,
        }
    }

    #[test]
    fn a_pending_comment_has_the_persisted_json_shape() {
        let value = serde_json::to_value(sample(CommentState::Pending)).expect("serialises");
        assert_eq!(
            value,
            json!({
                "id": "01234567-89ab-4def-8123-456789abcdef",
                "path": "src/a.rs",
                "side": "new",
                "start": 12,
                "end": 14,
                "quote": ["a", "b", "c"],
                "text": "Why clone here?",
                "state": { "pending": null },
                "created": 1_790_000_000u64,
            }),
            "the comment is stored in the shape data-model.md § Core: persistence gives"
        );
    }

    #[test]
    fn comments_round_trip_in_both_states_and_on_the_old_side() {
        for state in [CommentState::Pending, CommentState::Sent { at: 1_790_000_100 }] {
            let mut comment = sample(state);
            comment.side = Side::Old;
            let text = serde_json::to_string(&comment).expect("serialises");
            let back: ReviewComment = serde_json::from_str(&text).expect("deserialises");
            assert_eq!(back, comment, "a comment survives a JSON round trip: {text}");
        }
        let sent = serde_json::to_value(sample(CommentState::Sent { at: 5 })).expect("serialises");
        assert_eq!(sent["state"], json!({ "sent": { "at": 5 } }));
    }

    #[test]
    fn a_reversed_range_in_stored_json_is_refused() {
        let mut value = serde_json::to_value(sample(CommentState::Pending)).expect("serialises");
        value["start"] = json!(9);
        value["end"] = json!(3);
        assert!(
            serde_json::from_value::<ReviewComment>(value).is_err(),
            "a stored range with start after end cannot come back as a LineRange"
        );
    }

    #[test]
    fn new_ids_are_random_v4_uuids() {
        let (a, b) = (CommentId::new(), CommentId::new());
        assert_ne!(a, b, "two comments never share an id");
        assert_eq!(a.0.get_version_num(), 4, "ids are UUID v4");
    }
}
