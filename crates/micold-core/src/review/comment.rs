//! Review comments (feature 482, data-model "Core: review comments"): what one comment holds, its
//! state, and the pure operations on one entry's comments ([`EntryReview`]).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::diff::SideLines;
use super::prompt::{self, EntryKind};
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

impl ReviewComment {
    /// Whether the lines at its range on its side no longer hold its quote (R14, FR-013). `lines`
    /// is that side's text now; `None` when the file is gone or not text.
    pub fn is_outdated(&self, lines: Option<&SideLines>) -> bool {
        let Some(lines) = lines else {
            return true;
        };
        let now = (self.range.start()..=self.range.end()).map(|number| lines.line(number));
        !now.eq(self.quote.iter().map(|line| Some(line.as_str())))
    }
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

/// Why an operation on an entry's comments was refused (data-model `EntryReview`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewError {
    /// The input breaks a rule of W1 (the message says which).
    Invalid(String),
    /// No comment of this entry has that id (W3).
    NotFound,
    /// The comment is sent: sent comments are only cleared (W3).
    Refused,
    /// The comment is inside an open send (W3, R6).
    InSend,
    /// A send is already open for this entry (W6, R6).
    Busy,
    /// There is no pending comment to send (W6, US2 s5).
    NothingPending,
}

impl std::fmt::Display for ReviewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(why) => f.write_str(why),
            Self::NotFound => f.write_str("no such comment in this entry"),
            Self::Refused => f.write_str("a sent comment cannot be changed; clear it instead"),
            Self::InSend => f.write_str("the comment is being sent; try again once the send ends"),
            Self::Busy => f.write_str("these comments are already being sent"),
            Self::NothingPending => f.write_str("there are no pending comments to send"),
        }
    }
}

impl std::error::Error for ReviewError {}

/// What the user picked and wrote, before it becomes a [`ReviewComment`]. One `side` for the whole
/// range, so a range never mixes sides by construction (FR-011).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    /// The file, relative to the entry root.
    pub path: RelPath,
    /// Which version the lines are numbered in.
    pub side: Side,
    /// The lines.
    pub range: LineRange,
    /// Those lines' text, one entry per line.
    pub quote: Vec<String>,
    /// What the user wrote (trimmed when stored).
    pub text: String,
}

/// The comments one send carries, taken when it begins (data-model `SendSnapshot`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendSnapshot {
    /// The pending comments at `begin_send`, in the prompt's order.
    pub ids: Vec<CommentId>,
    /// The prompt built from them (contracts/review-prompt.md).
    pub prompt: String,
}

/// One entry's comments, in the order they were made, and the send open on them if any
/// (data-model `EntryReview`). The open send is never persisted (W10).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EntryReview {
    comments: Vec<ReviewComment>,
    sending: Option<Vec<CommentId>>,
}

impl EntryReview {
    /// An entry holding `comments` as loaded.
    pub fn from_comments(comments: Vec<ReviewComment>) -> Self {
        Self {
            comments,
            sending: None,
        }
    }

    /// A send is open for this entry.
    pub fn sending(&self) -> bool {
        self.sending.is_some()
    }

    /// Open a send of every pending comment (W6): `NothingPending` with none, `Busy` while
    /// another send is open. Until it finishes or aborts, its comments refuse edits (`InSend`).
    pub fn begin_send(
        &mut self,
        entry: EntryKind,
        outdated: &[CommentId],
    ) -> Result<SendSnapshot, ReviewError> {
        if self.sending.is_some() {
            return Err(ReviewError::Busy);
        }
        let pending: Vec<ReviewComment> = self
            .comments
            .iter()
            .filter(|comment| comment.state == CommentState::Pending)
            .cloned()
            .collect();
        if pending.is_empty() {
            return Err(ReviewError::NothingPending);
        }
        let ids: Vec<CommentId> = pending.iter().map(|comment| comment.id).collect();
        self.sending = Some(ids.clone());
        Ok(SendSnapshot {
            ids,
            prompt: prompt::build(entry, &pending, outdated),
        })
    }

    /// The send was delivered (W8): its comments become `Sent { at }`; comments added meanwhile
    /// stay pending.
    pub fn finish_send(&mut self, snapshot: &SendSnapshot, at: u64) {
        for comment in &mut self.comments {
            if snapshot.ids.contains(&comment.id) {
                comment.state = CommentState::Sent { at };
            }
        }
        self.sending = None;
    }

    /// The send failed (W9): every comment stays as it was, and edits are allowed again.
    pub fn abort_send(&mut self) {
        self.sending = None;
    }

    /// Every comment, pending and sent.
    pub fn comments(&self) -> &[ReviewComment] {
        &self.comments
    }

    /// No comments at all.
    pub fn is_empty(&self) -> bool {
        self.comments.is_empty()
    }

    /// Add a pending comment `id` made at `created` (W1): refused when the quote does not hold
    /// one line per line of the range or the trimmed text is empty; the text is stored trimmed.
    pub fn add(&mut self, draft: Draft, id: CommentId, created: u64) -> Result<(), ReviewError> {
        let text = checked_text(&draft.text)?;
        if draft.quote.len() != draft.range.len() as usize {
            return Err(ReviewError::Invalid(format!(
                "the quote holds {} lines but the range {}..={} covers {}",
                draft.quote.len(),
                draft.range.start(),
                draft.range.end(),
                draft.range.len()
            )));
        }
        self.comments.push(ReviewComment {
            id,
            path: draft.path,
            side: draft.side,
            range: draft.range,
            quote: draft.quote,
            text,
            state: CommentState::Pending,
            created,
        });
        Ok(())
    }

    /// Replace a pending comment's text (trimmed, non-empty).
    pub fn set_text(&mut self, id: CommentId, text: &str) -> Result<(), ReviewError> {
        let text = checked_text(text)?;
        let index = self.pending_index(id)?;
        self.comments[index].text = text;
        Ok(())
    }

    /// Delete a pending comment.
    pub fn delete(&mut self, id: CommentId) -> Result<(), ReviewError> {
        let index = self.pending_index(id)?;
        self.comments.remove(index);
        Ok(())
    }

    /// Remove every sent comment (W4); pending ones stay.
    pub fn clear_sent(&mut self) {
        self.comments
            .retain(|comment| comment.state == CommentState::Pending);
    }

    /// Remove every pending comment not inside an open send (W4); sent ones and those the send
    /// holds stay.
    pub fn discard_pending(&mut self) {
        let sending = self.sending.as_deref().unwrap_or_default();
        self.comments.retain(|comment| {
            comment.state != CommentState::Pending || sending.contains(&comment.id)
        });
    }

    /// Where pending comment `id` is: `NotFound` when this entry has no such comment, `InSend` when
    /// an open send holds it, `Refused` when it is sent.
    fn pending_index(&self, id: CommentId) -> Result<usize, ReviewError> {
        let index = self
            .comments
            .iter()
            .position(|comment| comment.id == id)
            .ok_or(ReviewError::NotFound)?;
        if self.sending.as_ref().is_some_and(|ids| ids.contains(&id)) {
            return Err(ReviewError::InSend);
        }
        match self.comments[index].state {
            CommentState::Pending => Ok(index),
            CommentState::Sent { .. } => Err(ReviewError::Refused),
        }
    }
}

/// `text` trimmed, or `Invalid` when nothing is left.
fn checked_text(text: &str) -> Result<String, ReviewError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(ReviewError::Invalid("a comment needs some text".to_owned()));
    }
    Ok(trimmed.to_owned())
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

    fn draft(start: u32, end: u32, quote: &[&str], text: &str) -> Draft {
        Draft {
            path: RelPath::from_git("src/a.rs"),
            side: Side::New,
            range: LineRange::new(start, end).expect("a valid range"),
            quote: quote.iter().map(|line| (*line).to_owned()).collect(),
            text: text.to_owned(),
        }
    }

    fn id(n: u128) -> CommentId {
        CommentId(Uuid::from_u128(n))
    }

    const NOW: u64 = 1_790_000_000;

    #[test]
    fn add_stores_a_pending_comment_with_its_text_trimmed() {
        let mut review = EntryReview::default();
        review
            .add(
                draft(12, 14, &["a", "b", "c"], "  Why clone here?\n"),
                id(1),
                NOW,
            )
            .expect("a valid draft is accepted");
        let stored = review.comments();
        assert_eq!(stored.len(), 1, "the comment is kept");
        assert_eq!(stored[0].id, id(1));
        assert_eq!(
            stored[0].text, "Why clone here?",
            "text is stored trimmed (W1)"
        );
        assert_eq!(
            stored[0].state,
            CommentState::Pending,
            "a new comment is pending"
        );
        assert_eq!(stored[0].quote, vec!["a", "b", "c"]);
        assert_eq!((stored[0].range.start(), stored[0].range.end()), (12, 14));
        assert_eq!(stored[0].created, NOW);
    }

    #[test]
    fn add_refuses_a_quote_that_is_not_one_line_per_line_of_the_range() {
        let mut review = EntryReview::default();
        for quote in [&["a", "b"][..], &["a", "b", "c", "d"][..]] {
            assert!(
                matches!(
                    review.add(draft(12, 14, quote, "text"), id(1), NOW),
                    Err(ReviewError::Invalid(_))
                ),
                "a {}-line quote of a 3-line range is refused (FR-012)",
                quote.len()
            );
        }
        assert!(review.is_empty(), "nothing is stored on a refusal");
    }

    #[test]
    fn add_refuses_text_that_is_empty_once_trimmed() {
        let mut review = EntryReview::default();
        for text in ["", "   ", "\n\t "] {
            assert!(
                matches!(
                    review.add(draft(3, 3, &["x"], text), id(1), NOW),
                    Err(ReviewError::Invalid(_))
                ),
                "{text:?} is no comment (W1)"
            );
        }
        assert!(review.is_empty(), "nothing is stored on a refusal");
    }

    #[test]
    fn set_text_replaces_a_pending_comments_text_trimmed() {
        let mut review = EntryReview::default();
        review
            .add(draft(3, 3, &["x"], "first"), id(1), NOW)
            .expect("added");
        review
            .set_text(id(1), " second ")
            .expect("a pending comment can be edited");
        assert_eq!(
            review.comments()[0].text,
            "second",
            "the edit is stored trimmed"
        );
        assert!(
            matches!(review.set_text(id(1), "  "), Err(ReviewError::Invalid(_))),
            "an edit to empty text is refused; delete is the way to drop a comment"
        );
        assert_eq!(
            review.comments()[0].text,
            "second",
            "a refused edit changes nothing"
        );
    }

    #[test]
    fn delete_removes_a_pending_comment_only() {
        let mut review = EntryReview::default();
        review
            .add(draft(3, 3, &["x"], "one"), id(1), NOW)
            .expect("added");
        review
            .add(draft(4, 4, &["y"], "two"), id(2), NOW)
            .expect("added");
        review
            .delete(id(1))
            .expect("a pending comment can be deleted");
        let left: Vec<_> = review.comments().iter().map(|c| c.id).collect();
        assert_eq!(left, vec![id(2)], "only the deleted comment is gone");
    }

    #[test]
    fn an_unknown_id_is_not_found_for_set_text_and_delete() {
        let mut review = EntryReview::default();
        review
            .add(draft(3, 3, &["x"], "one"), id(1), NOW)
            .expect("added");
        assert_eq!(review.set_text(id(9), "t"), Err(ReviewError::NotFound));
        assert_eq!(review.delete(id(9)), Err(ReviewError::NotFound));
        assert_eq!(review.comments().len(), 1, "nothing changed");
    }

    #[test]
    fn a_sent_comment_refuses_set_text_and_delete() {
        let mut sent = sample(CommentState::Sent { at: NOW });
        sent.id = id(1);
        let mut review = EntryReview::from_comments(vec![sent.clone()]);
        assert_eq!(
            review.set_text(id(1), "new"),
            Err(ReviewError::Refused),
            "a sent comment's text is what the session got; it is not rewritten (W3)"
        );
        assert_eq!(
            review.delete(id(1)),
            Err(ReviewError::Refused),
            "sent comments are only cleared"
        );
        assert_eq!(review.comments(), &[sent][..], "nothing changed");
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

    fn three_pending_one_sent() -> EntryReview {
        let mut review = EntryReview::default();
        for n in 1..=3 {
            review
                .add(
                    draft(n, n, &["x"], &format!("c{n}")),
                    id(u128::from(n)),
                    NOW,
                )
                .expect("valid");
        }
        let mut sent = sample(CommentState::Sent { at: NOW });
        sent.id = id(9);
        let mut comments = review.comments().to_vec();
        comments.push(sent);
        EntryReview::from_comments(comments)
    }

    fn state_of(review: &EntryReview, n: u128) -> CommentState {
        review
            .comments()
            .iter()
            .find(|comment| comment.id == id(n))
            .expect("the comment is there")
            .state
    }

    #[test]
    fn begin_send_snapshots_exactly_the_pending_comments() {
        let mut review = three_pending_one_sent();
        let snapshot = review
            .begin_send(EntryKind::Worktree, &[])
            .expect("pending comments can be sent");
        assert_eq!(
            snapshot.ids,
            vec![id(1), id(2), id(3)],
            "sent comments are left out (US2 s6)"
        );
        assert!(review.sending(), "the send is open");
        let pending: Vec<_> = review
            .comments()
            .iter()
            .filter(|comment| comment.state == CommentState::Pending)
            .cloned()
            .collect();
        assert_eq!(
            snapshot.prompt,
            crate::review::prompt::build(EntryKind::Worktree, &pending, &[]),
            "the snapshot's prompt is built from exactly those comments"
        );
        assert!(
            !snapshot.prompt.contains("Why clone here?"),
            "no trace of the sent one"
        );
    }

    #[test]
    fn begin_send_with_nothing_pending_or_a_send_open_is_refused() {
        let mut empty = EntryReview::from_comments(vec![sample(CommentState::Sent { at: NOW })]);
        assert_eq!(
            empty.begin_send(EntryKind::Worktree, &[]),
            Err(ReviewError::NothingPending),
            "only sent comments: nothing to send (US2 s5)"
        );
        assert!(!empty.sending(), "a refused send opens nothing");

        let mut review = three_pending_one_sent();
        review
            .begin_send(EntryKind::Worktree, &[])
            .expect("first send");
        assert_eq!(
            review.begin_send(EntryKind::Worktree, &[]),
            Err(ReviewError::Busy),
            "a second send while one is open is Busy (FR-018)"
        );
    }

    #[test]
    fn comments_in_an_open_send_refuse_edits_and_others_do_not() {
        let mut review = three_pending_one_sent();
        review.begin_send(EntryKind::Worktree, &[]).expect("send");
        assert_eq!(review.set_text(id(1), "new"), Err(ReviewError::InSend));
        assert_eq!(review.delete(id(2)), Err(ReviewError::InSend));
        review
            .add(draft(5, 5, &["y"], "added meanwhile"), id(5), NOW)
            .expect("adding during a send is allowed");
        review
            .set_text(id(5), "edited")
            .expect("a comment outside the send can be edited");
        review.delete(id(5)).expect("and deleted");
    }

    #[test]
    fn finish_send_marks_only_the_snapshot_sent() {
        let mut review = three_pending_one_sent();
        let snapshot = review.begin_send(EntryKind::Worktree, &[]).expect("send");
        review
            .add(draft(5, 5, &["y"], "added meanwhile"), id(5), NOW)
            .expect("valid");
        review.finish_send(&snapshot, NOW + 7);
        for n in 1..=3 {
            assert_eq!(state_of(&review, n), CommentState::Sent { at: NOW + 7 });
        }
        assert_eq!(
            state_of(&review, 5),
            CommentState::Pending,
            "added meanwhile stays pending"
        );
        assert_eq!(
            state_of(&review, 9),
            CommentState::Sent { at: NOW },
            "the old send keeps its time"
        );
        assert!(!review.sending(), "the send is closed");
        review.set_text(id(5), "edit").expect("edits work again");
    }

    #[test]
    fn abort_send_leaves_every_comment_pending_and_editable() {
        let mut review = three_pending_one_sent();
        review.begin_send(EntryKind::Worktree, &[]).expect("send");
        review.abort_send();
        for n in 1..=3 {
            assert_eq!(state_of(&review, n), CommentState::Pending, "FR-017");
        }
        assert!(!review.sending(), "the send is closed");
        review.set_text(id(1), "edit").expect("edits work again");
        review
            .begin_send(EntryKind::Worktree, &[])
            .expect("a new send can begin");
    }

    #[test]
    fn clear_sent_removes_sent_comments_only() {
        let mut review = three_pending_one_sent();
        review.clear_sent();
        let ids: Vec<_> = review.comments().iter().map(|comment| comment.id).collect();
        assert_eq!(ids, vec![id(1), id(2), id(3)], "pending stay (US4 s3)");
        assert!(review
            .comments()
            .iter()
            .all(|comment| comment.state == CommentState::Pending));
    }

    #[test]
    fn discard_pending_removes_pending_comments_not_in_an_open_send() {
        let mut review = three_pending_one_sent();
        review.discard_pending();
        let ids: Vec<_> = review.comments().iter().map(|comment| comment.id).collect();
        assert_eq!(ids, vec![id(9)], "only the sent one stays (FR-019)");

        let mut sending = three_pending_one_sent();
        sending.begin_send(EntryKind::Worktree, &[]).expect("send");
        sending
            .add(draft(5, 5, &["y"], "added meanwhile"), id(5), NOW)
            .expect("valid");
        sending.discard_pending();
        let ids: Vec<_> = sending
            .comments()
            .iter()
            .map(|comment| comment.id)
            .collect();
        assert_eq!(
            ids,
            vec![id(1), id(2), id(3), id(9)],
            "comments in the open send stay; the one added meanwhile goes (W4)"
        );
        assert!(sending.sending(), "the send stays open");
    }

    fn lines(text: &str) -> SideLines {
        SideLines::from_bytes(text.as_bytes()).expect("text")
    }

    #[test]
    fn a_comment_whose_lines_still_hold_its_quote_is_not_outdated() {
        let comment = sample(CommentState::Pending);
        let mut text = "x\n".repeat(11);
        text.push_str("a\nb\nc\nd\n");
        assert!(
            !comment.is_outdated(Some(&lines(&text))),
            "lines 12..=14 are a, b, c"
        );
    }

    #[test]
    fn a_comment_whose_lines_changed_is_outdated() {
        let comment = sample(CommentState::Pending);
        let mut text = "x\n".repeat(11);
        text.push_str("a\nB\nc\n");
        assert!(comment.is_outdated(Some(&lines(&text))), "line 13 is now B");
    }

    #[test]
    fn a_comment_whose_range_runs_past_the_end_is_outdated() {
        let comment = sample(CommentState::Pending);
        let mut text = "x\n".repeat(11);
        text.push_str("a\nb\n");
        assert!(comment.is_outdated(Some(&lines(&text))), "line 14 is gone");
    }

    #[test]
    fn a_comment_on_a_file_that_is_gone_is_outdated() {
        assert!(sample(CommentState::Sent { at: NOW }).is_outdated(None));
    }
}
