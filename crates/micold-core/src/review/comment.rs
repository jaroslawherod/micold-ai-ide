//! Review comments (feature 482, data-model "Core: review comments"): what one comment holds and
//! its state. The operations on an entry's comments arrive with the daemon's review store.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{LineRange, RelPath, Side};

/// A comment's identity: a v4 UUID the daemon assigns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommentId(pub Uuid);

impl CommentId {
    /// A fresh random id.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for CommentId {
    fn default() -> Self {
        Self::new()
    }
}

/// Whether a comment has reached its session. `Pending → Sent` only on delivery (FR-017), and
/// never back. Stored as `{ "pending": null }` or `{ "sent": { "at": … } }` (data-model § Core:
/// persistence).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "StateRepr", into = "StateRepr")]
pub enum CommentState {
    /// Not yet delivered.
    Pending,
    /// Typed into a session at `at` (Unix seconds).
    Sent {
        /// When the prompt was written.
        at: u64,
    },
}

/// The stored shape of [`CommentState`]: a newtype `Pending(())` writes `{ "pending": null }`
/// where a unit variant would write a bare string.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StateRepr {
    Pending(()),
    Sent { at: u64 },
}

impl From<StateRepr> for CommentState {
    fn from(repr: StateRepr) -> Self {
        match repr {
            StateRepr::Pending(()) => Self::Pending,
            StateRepr::Sent { at } => Self::Sent { at },
        }
    }
}

impl From<CommentState> for StateRepr {
    fn from(state: CommentState) -> Self {
        match state {
            CommentState::Pending => Self::Pending(()),
            CommentState::Sent { at } => Self::Sent { at },
        }
    }
}

/// One comment on a line range of one side of one file (FR-011, FR-012).
///
/// Serialised through [`StoredComment`], which spells the range as top-level `start` and `end`
/// without `#[serde(flatten)]`: flatten needs a self-describing format, and the comment also
/// travels on the postcard wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "StoredComment", into = "StoredComment")]
pub struct ReviewComment {
    /// Its identity.
    pub id: CommentId,
    /// The file, relative to the entry root.
    pub path: RelPath,
    /// Which version the lines are numbered in.
    pub side: Side,
    /// The lines it is about, stored as top-level `start` and `end`.
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

/// The stored and wire shape of [`ReviewComment`].
#[derive(Serialize, Deserialize)]
struct StoredComment {
    id: CommentId,
    path: RelPath,
    side: Side,
    start: u32,
    end: u32,
    quote: Vec<String>,
    text: String,
    state: CommentState,
    created: u64,
}

impl TryFrom<StoredComment> for ReviewComment {
    type Error = String;

    fn try_from(stored: StoredComment) -> Result<Self, Self::Error> {
        let range = LineRange::new(stored.start, stored.end).ok_or_else(|| {
            format!(
                "invalid line range {}..={}: lines start at 1 and start <= end",
                stored.start, stored.end
            )
        })?;
        Ok(Self {
            id: stored.id,
            path: stored.path,
            side: stored.side,
            range,
            quote: stored.quote,
            text: stored.text,
            state: stored.state,
            created: stored.created,
        })
    }
}

impl From<ReviewComment> for StoredComment {
    fn from(comment: ReviewComment) -> Self {
        Self {
            id: comment.id,
            path: comment.path,
            side: comment.side,
            start: comment.range.start(),
            end: comment.range.end(),
            quote: comment.quote,
            text: comment.text,
            state: comment.state,
            created: comment.created,
        }
    }
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
        for state in [
            CommentState::Pending,
            CommentState::Sent { at: 1_790_000_100 },
        ] {
            let mut comment = sample(state);
            comment.side = Side::Old;
            let text = serde_json::to_string(&comment).expect("serialises");
            let back: ReviewComment = serde_json::from_str(&text).expect("deserialises");
            assert_eq!(
                back, comment,
                "a comment survives a JSON round trip: {text}"
            );
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
