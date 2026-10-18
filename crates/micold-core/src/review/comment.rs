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

/// Where a comment is in its life (feature 946, data-model § `CommentState`, FR-001):
/// `Draft → Sent → Reported ⟲ Reported`, and never back to `Draft`. Stored as `{ "draft": null }`,
/// `{ "sent": { "at": … } }` or `{ "reported": { "sent_at", "outcome", "reply", "first_at",
/// "last_at", "by" } }`; version 1's `{ "pending": null }` reads as `Draft`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "StateRepr", into = "StateRepr")]
pub enum CommentState {
    /// Written, not sent (482's pending). Editable and deletable.
    Draft,
    /// Typed into a session at `at` (Unix seconds).
    Sent {
        /// When the batch was written.
        at: u64,
    },
    /// The agent (or the user, by hand) said it was applied or answered.
    Reported {
        /// When the batch carrying it was written.
        sent_at: u64,
        /// The latest report.
        report: Report,
    },
}

impl CommentState {
    /// The one of four statuses the UI and the panel's counts show (FR-001).
    pub fn status(&self) -> Status {
        match self {
            Self::Draft => Status::Draft,
            Self::Sent { .. } => Status::Sent,
            Self::Reported { report, .. } => match report.outcome {
                Outcome::Applied => Status::Applied,
                Outcome::Answered => Status::Answered,
            },
        }
    }
}

/// What a comment's state means to the user (FR-001).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Status {
    /// Not sent yet.
    Draft,
    /// Sent, no report yet.
    Sent,
    /// Reported as applied.
    Applied,
    /// Reported as answered.
    Answered,
}

/// How a comment was handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// The change was made.
    Applied,
    /// Not made; the reply says why.
    Answered,
}

/// Who made the latest report. Stored as `{ "agent": "<session uuid>" }` or `"user"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "ReporterRepr", into = "ReporterRepr")]
pub enum Reporter {
    /// A session, through the agent tool server.
    Agent {
        /// The reporting session.
        session: Uuid,
    },
    /// The user, by hand.
    User,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ReporterRepr {
    Agent(Uuid),
    User,
}

impl From<ReporterRepr> for Reporter {
    fn from(repr: ReporterRepr) -> Self {
        match repr {
            ReporterRepr::Agent(session) => Self::Agent { session },
            ReporterRepr::User => Self::User,
        }
    }
}

impl From<Reporter> for ReporterRepr {
    fn from(reporter: Reporter) -> Self {
        match reporter {
            Reporter::Agent { session } => Self::Agent(session),
            Reporter::User => Self::User,
        }
    }
}

/// A comment's latest report (data-model § `Report`). Built only by [`EntryReview`]'s report and
/// mark operations (research R1), so a Reported comment always holds a checked reply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    outcome: Outcome,
    reply: String,
    first_at: u64,
    last_at: u64,
    by: Reporter,
}

impl Report {
    /// Applied or answered.
    pub fn outcome(&self) -> Outcome {
        self.outcome
    }

    /// The reply: `\n` line breaks, trimmed; never empty when answered.
    pub fn reply(&self) -> &str {
        &self.reply
    }

    /// When the comment was first reported (Unix seconds), kept by later reports (FR-005).
    pub fn first_at(&self) -> u64 {
        self.first_at
    }

    /// When it was last reported (Unix seconds).
    pub fn last_at(&self) -> u64 {
        self.last_at
    }

    /// Who reported last.
    pub fn by(&self) -> Reporter {
        self.by
    }
}

/// The stored shape of [`CommentState`]: a newtype `Draft(())` writes `{ "draft": null }` where a
/// unit variant would write a bare string. `pending` is version 1's name for a draft.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StateRepr {
    #[serde(alias = "pending")]
    Draft(()),
    Sent {
        at: u64,
    },
    Reported {
        sent_at: u64,
        outcome: Outcome,
        reply: String,
        first_at: u64,
        last_at: u64,
        by: Reporter,
    },
}

impl TryFrom<StateRepr> for CommentState {
    type Error = String;

    /// Loads a stored state, refusing a report its operations could not have built: a blank
    /// answered reply, or a first report later than the last.
    fn try_from(repr: StateRepr) -> Result<Self, Self::Error> {
        Ok(match repr {
            StateRepr::Draft(()) => Self::Draft,
            StateRepr::Sent { at } => Self::Sent { at },
            StateRepr::Reported {
                outcome: Outcome::Answered,
                ref reply,
                ..
            } if reply.trim().is_empty() => {
                return Err("an answered report has a reply".to_owned());
            }
            StateRepr::Reported {
                first_at, last_at, ..
            } if first_at > last_at => {
                return Err("a report's first time is not after its last".to_owned());
            }
            StateRepr::Reported {
                sent_at,
                outcome,
                reply,
                first_at,
                last_at,
                by,
            } => Self::Reported {
                sent_at,
                report: Report {
                    outcome,
                    reply,
                    first_at,
                    last_at,
                    by,
                },
            },
        })
    }
}

impl From<CommentState> for StateRepr {
    fn from(state: CommentState) -> Self {
        match state {
            CommentState::Draft => Self::Draft(()),
            CommentState::Sent { at } => Self::Sent { at },
            CommentState::Reported { sent_at, report } => Self::Reported {
                sent_at,
                outcome: report.outcome,
                reply: report.reply,
                first_at: report.first_at,
                last_at: report.last_at,
                by: report.by,
            },
        }
    }
}

/// A comment's short reference, `C-` + the first 8 lowercase hex digits of its id (FR-014,
/// research R4). Derived from the id, never stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommentRef(u32);

impl CommentRef {
    /// The reference of comment `id`.
    pub fn of(id: CommentId) -> Self {
        let bytes = id.0.as_bytes();
        Self(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Read a reference as an agent or the user wrote it: `C-` or `c-`, then 8 hex digits in
    /// either case, with optional surrounding whitespace. `None` for anything else.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let hex = text
            .strip_prefix("C-")
            .or_else(|| text.strip_prefix("c-"))?;
        if hex.len() != 8 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        u32::from_str_radix(hex, 16).ok().map(Self)
    }
}

impl std::fmt::Display for CommentRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "C-{:08x}", self.0)
    }
}

/// A comment's range moved from `from` to `to` (data-model § `Move`); both cover as many lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "MoveRepr", into = "MoveRepr")]
pub struct Move {
    id: CommentId,
    from: LineRange,
    to: LineRange,
}

#[derive(Serialize, Deserialize)]
struct MoveRepr {
    id: CommentId,
    from: LineRange,
    to: LineRange,
}

impl Move {
    /// Comment `id` moves from `from` to `to`; `None` when they cover different numbers of lines.
    pub fn new(id: CommentId, from: LineRange, to: LineRange) -> Option<Self> {
        (from.len() == to.len()).then_some(Self { id, from, to })
    }

    /// The comment.
    pub fn id(&self) -> CommentId {
        self.id
    }

    /// The range it must still have for the move to apply.
    pub fn from(&self) -> LineRange {
        self.from
    }

    /// Its new range.
    pub fn to(&self) -> LineRange {
        self.to
    }
}

impl TryFrom<MoveRepr> for Move {
    type Error = String;

    fn try_from(repr: MoveRepr) -> Result<Self, Self::Error> {
        Self::new(repr.id, repr.from, repr.to)
            .ok_or_else(|| "a move keeps the number of lines".to_owned())
    }
}

impl From<Move> for MoveRepr {
    fn from(m: Move) -> Self {
        Self {
            id: m.id,
            from: m.from,
            to: m.to,
        }
    }
}

/// A new id for a comment of `entry`: the first one `draw` gives whose reference no comment of
/// the entry already has (research R4).
pub fn fresh_id(entry: &EntryReview, mut draw: impl FnMut() -> CommentId) -> CommentId {
    loop {
        let id = draw();
        let reference = CommentRef::of(id);
        if !entry.comments().iter().any(|c| c.reference() == reference) {
            return id;
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
    /// Draft, sent or reported.
    pub state: CommentState,
    /// When it was made (Unix seconds).
    pub created: u64,
}

impl ReviewComment {
    /// Its short reference, `C-xxxxxxxx` (FR-014).
    pub fn reference(&self) -> CommentRef {
        CommentRef::of(self.id)
    }

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
    /// There is no draft comment to send (W6, US2 s5).
    NothingPending,
    /// Another comment of the entry already has the new id's reference (FR-014).
    RefTaken,
    /// The comment is a Draft: it was never sent, so it cannot be reported (FR-004).
    Draft,
}

impl std::fmt::Display for ReviewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(why) => f.write_str(why),
            Self::NotFound => f.write_str("no such comment in this entry"),
            Self::Refused => f.write_str("only a draft comment can be changed; clear it instead"),
            Self::InSend => f.write_str("the comment is being sent; try again once the send ends"),
            Self::Busy => f.write_str("these comments are already being sent"),
            Self::NothingPending => f.write_str("there are no draft comments to send"),
            Self::RefTaken => f.write_str("another comment of this entry has that reference"),
            Self::Draft => f.write_str("that comment is still a draft; it has not been sent"),
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

    /// Open a send of every draft comment (W6): `NothingPending` with none, `Busy` while
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
            .filter(|comment| comment.state == CommentState::Draft)
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

    /// Add a Draft comment `id` made at `created` (W1): refused when the quote does not hold
    /// one line per line of the range, the trimmed text is empty, or another comment of the
    /// entry has `id`'s reference (`RefTaken`, FR-014); the text is stored trimmed.
    pub fn add(&mut self, draft: Draft, id: CommentId, created: u64) -> Result<(), ReviewError> {
        let text = checked_text(&draft.text)?;
        let reference = CommentRef::of(id);
        if self.comments.iter().any(|c| c.reference() == reference) {
            return Err(ReviewError::RefTaken);
        }
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
            state: CommentState::Draft,
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

    /// Remove every sent or reported comment (W4); drafts stay.
    pub fn clear_sent(&mut self) {
        self.comments
            .retain(|comment| comment.state == CommentState::Draft);
    }

    /// Remove every pending comment not inside an open send (W4); sent ones and those the send
    /// holds stay.
    pub fn discard_pending(&mut self) {
        let sending = self.sending.as_deref().unwrap_or_default();
        self.comments.retain(|comment| {
            comment.state != CommentState::Draft || sending.contains(&comment.id)
        });
    }

    /// Record a report on the Sent or Reported comment `reference` (data-model `report`, FR-003,
    /// FR-005): the reply is `\r\n`/`\r` → `\n` and trimmed; a repeated report replaces outcome,
    /// reply and `by` and keeps `first_at`. Refused, changing nothing: `Invalid` for a blank
    /// answered reply or one over [`MAX_REPLY_CHARS`], `NotFound`, `InSend`, `Draft` (FR-004).
    /// Returns the status before the report.
    pub fn report(
        &mut self,
        reference: CommentRef,
        outcome: Outcome,
        reply: &str,
        by: Reporter,
        at: u64,
    ) -> Result<Status, ReviewError> {
        let reply = checked_reply(outcome, reply)?;
        let comment = self
            .comments
            .iter_mut()
            .find(|comment| comment.reference() == reference)
            .ok_or(ReviewError::NotFound)?;
        if self
            .sending
            .as_ref()
            .is_some_and(|ids| ids.contains(&comment.id))
        {
            return Err(ReviewError::InSend);
        }
        let previous = comment.state.status();
        let (sent_at, first_at) = match &comment.state {
            CommentState::Draft => return Err(ReviewError::Draft),
            CommentState::Sent { at: sent_at } => (*sent_at, at),
            CommentState::Reported { sent_at, report } => (*sent_at, report.first_at),
        };
        comment.state = CommentState::Reported {
            sent_at,
            report: Report {
                outcome,
                reply,
                first_at,
                last_at: at.max(first_at),
                by,
            },
        };
        Ok(previous)
    }

    /// The user marks the Sent comment `id` applied or answered by hand (feature 946, W14, FR-006):
    /// a report by [`Reporter::User`] at `at`, with `note` as the reply. Only a Sent comment can
    /// be marked (research R9). Refused, changing nothing: `Invalid` for a blank answered note or
    /// one over [`MAX_REPLY_CHARS`], `NotFound`, `InSend`, `Draft`, and `Refused` for a comment
    /// already reported.
    pub fn mark(
        &mut self,
        id: CommentId,
        outcome: Outcome,
        note: &str,
        at: u64,
    ) -> Result<(), ReviewError> {
        let reply = checked_reply(outcome, note)?;
        let comment = self
            .comments
            .iter_mut()
            .find(|comment| comment.id == id)
            .ok_or(ReviewError::NotFound)?;
        if self.sending.as_ref().is_some_and(|ids| ids.contains(&id)) {
            return Err(ReviewError::InSend);
        }
        let sent_at = match comment.state {
            CommentState::Sent { at: sent_at } => sent_at,
            CommentState::Draft => return Err(ReviewError::Draft),
            CommentState::Reported { .. } => return Err(ReviewError::Refused),
        };
        comment.state = CommentState::Reported {
            sent_at,
            report: Report {
                outcome,
                reply,
                first_at: at,
                last_at: at,
                by: Reporter::User,
            },
        };
        Ok(())
    }

    /// Apply `moves` (W15): a move applies only while its comment's range equals `from` and `to`
    /// covers as many lines as the quote; others are skipped. Quote, text and state never change
    /// (FR-023), and a comment in the open send moves too. How many moves applied.
    pub fn reanchor(&mut self, moves: &[Move]) -> usize {
        let mut applied = 0;
        for m in moves {
            let Some(comment) = self.comments.iter_mut().find(|c| c.id == m.id()) else {
                continue;
            };
            let same_size = usize::try_from(m.to().len()).is_ok_and(|n| n == comment.quote.len());
            if comment.range == m.from() && same_size && m.from() != m.to() {
                comment.range = m.to();
                applied += 1;
            }
        }
        applied
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
            CommentState::Draft => Ok(index),
            CommentState::Sent { .. } | CommentState::Reported { .. } => Err(ReviewError::Refused),
        }
    }
}

/// The longest reply a report may carry, in characters (FR-004).
pub const MAX_REPLY_CHARS: usize = 20_000;

/// `reply` with `\r\n` and `\r` as `\n` and trimmed, or `Invalid` when an answered comment's is
/// blank or any is over [`MAX_REPLY_CHARS`] (FR-004).
fn checked_reply(outcome: Outcome, reply: &str) -> Result<String, ReviewError> {
    let reply = reply.replace("\r\n", "\n").replace('\r', "\n");
    let reply = reply.trim();
    if outcome == Outcome::Answered && reply.is_empty() {
        return Err(ReviewError::Invalid(
            "an answered comment needs an explanation in reply".to_owned(),
        ));
    }
    if reply.chars().count() > MAX_REPLY_CHARS {
        return Err(ReviewError::Invalid(format!(
            "the reply is over the {MAX_REPLY_CHARS}-character limit"
        )));
    }
    Ok(reply.to_owned())
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

    /// `0000000n-…`: distinct leading digits, so each id has its own reference.
    fn id(n: u128) -> CommentId {
        CommentId(Uuid::from_u128(n << 96))
    }

    const NOW: u64 = 1_790_000_000;

    #[test]
    fn add_refuses_an_id_whose_reference_another_comment_has() {
        let mut review = EntryReview::default();
        let first = CommentId(Uuid::from_u128(0x1a2b_3c4d_0000_4000_8000_0000_0000_0001));
        let same_ref = CommentId(Uuid::from_u128(0x1a2b_3c4d_0000_4000_8000_0000_0000_0002));
        review
            .add(draft(1, 1, &["x"], "one"), first, NOW)
            .expect("the first is kept");
        assert_eq!(
            review.add(draft(2, 2, &["y"], "two"), same_ref, NOW),
            Err(ReviewError::RefTaken),
            "a reference is unique within the entry (FR-014)"
        );
        assert_eq!(review.comments().len(), 1, "nothing is stored on a refusal");
    }

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
            CommentState::Draft,
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
    fn a_draft_comment_has_the_persisted_json_shape() {
        let value = serde_json::to_value(sample(CommentState::Draft)).expect("serialises");
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
                "state": { "draft": null },
                "created": 1_790_000_000u64,
            }),
            "the comment is stored in the shape data-model.md § Core: persistence gives"
        );
    }

    #[test]
    fn comments_round_trip_in_both_states_and_on_the_old_side() {
        for state in [
            CommentState::Draft,
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
        let mut value = serde_json::to_value(sample(CommentState::Draft)).expect("serialises");
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
            .clone()
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
            .filter(|comment| comment.state == CommentState::Draft)
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
            CommentState::Draft,
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
            assert_eq!(state_of(&review, n), CommentState::Draft, "FR-017");
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
            .all(|comment| comment.state == CommentState::Draft));
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
        let comment = sample(CommentState::Draft);
        let mut text = "x\n".repeat(11);
        text.push_str("a\nb\nc\nd\n");
        assert!(
            !comment.is_outdated(Some(&lines(&text))),
            "lines 12..=14 are a, b, c"
        );
    }

    #[test]
    fn a_comment_whose_lines_changed_is_outdated() {
        let comment = sample(CommentState::Draft);
        let mut text = "x\n".repeat(11);
        text.push_str("a\nB\nc\n");
        assert!(comment.is_outdated(Some(&lines(&text))), "line 13 is now B");
    }

    #[test]
    fn a_comment_whose_range_runs_past_the_end_is_outdated() {
        let comment = sample(CommentState::Draft);
        let mut text = "x\n".repeat(11);
        text.push_str("a\nb\n");
        assert!(comment.is_outdated(Some(&lines(&text))), "line 14 is gone");
    }

    #[test]
    fn a_comment_on_a_file_that_is_gone_is_outdated() {
        assert!(sample(CommentState::Sent { at: NOW }).is_outdated(None));
    }

    // --- Feature 946 (T002): statuses, references, moves ---

    const AGENT: u128 = 0x6f1c_0000_0000_4000_8000_0000_0000_0001;

    fn reported(outcome: Outcome, reply: &str, by: Reporter) -> CommentState {
        CommentState::Reported {
            sent_at: NOW,
            report: Report {
                outcome,
                reply: reply.to_owned(),
                first_at: NOW + 100,
                last_at: NOW + 200,
                by,
            },
        }
    }

    #[test]
    fn each_state_has_the_data_model_store_shape() {
        let state =
            |s: CommentState| serde_json::to_value(sample(s)).expect("serialises")["state"].clone();
        assert_eq!(
            state(CommentState::Draft),
            json!({ "draft": null }),
            "a draft"
        );
        assert_eq!(
            state(CommentState::Sent { at: NOW }),
            json!({ "sent": { "at": NOW } }),
            "a sent comment"
        );
        assert_eq!(
            state(reported(
                Outcome::Answered,
                "Kept on purpose.",
                Reporter::Agent {
                    session: Uuid::from_u128(AGENT)
                }
            )),
            json!({ "reported": {
                "sent_at": NOW,
                "outcome": "answered",
                "reply": "Kept on purpose.",
                "first_at": NOW + 100,
                "last_at": NOW + 200,
                "by": { "agent": Uuid::from_u128(AGENT).to_string() },
            } }),
            "a report by an agent names its session"
        );
        assert_eq!(
            state(reported(Outcome::Applied, "", Reporter::User))["reported"]["by"],
            json!("user"),
            "a mark by hand says user"
        );
        for s in [
            CommentState::Draft,
            CommentState::Sent { at: NOW },
            reported(Outcome::Applied, "", Reporter::User),
            reported(
                Outcome::Answered,
                "why",
                Reporter::Agent {
                    session: Uuid::from_u128(AGENT),
                },
            ),
        ] {
            let back: ReviewComment = serde_json::from_value(
                serde_json::to_value(sample(s.clone())).expect("serialises"),
            )
            .expect("deserialises");
            assert_eq!(back.state, s, "every state round-trips");
        }
    }

    #[test]
    fn a_stored_report_its_operations_could_not_build_is_refused() {
        let stored = |reply: &str, first: u64, last: u64| {
            let mut value =
                serde_json::to_value(sample(reported(Outcome::Answered, "why", Reporter::User)))
                    .expect("serialises");
            value["state"]["reported"]["reply"] = json!(reply);
            value["state"]["reported"]["first_at"] = json!(first);
            value["state"]["reported"]["last_at"] = json!(last);
            serde_json::from_value::<ReviewComment>(value)
        };
        assert!(stored("why", NOW, NOW).is_ok(), "a sound report loads");
        assert!(stored("  ", NOW, NOW).is_err(), "a blank answered reply");
        assert!(stored("why", NOW + 1, NOW).is_err(), "first after last");
    }

    #[test]
    fn status_is_exactly_one_of_four() {
        assert_eq!(CommentState::Draft.status(), Status::Draft);
        assert_eq!(CommentState::Sent { at: NOW }.status(), Status::Sent);
        assert_eq!(
            reported(Outcome::Applied, "", Reporter::User).status(),
            Status::Applied,
            "an applied report shows Applied (FR-001)"
        );
        assert_eq!(
            reported(Outcome::Answered, "why", Reporter::User).status(),
            Status::Answered,
            "an answered report shows Answered (FR-001)"
        );
    }

    #[test]
    fn a_reference_is_c_and_the_first_eight_lowercase_hex_digits_of_the_id() {
        let comment = sample(CommentState::Draft);
        let reference = comment.reference().to_string();
        assert_eq!(reference, "C-01234567", "the id starts 01234567 (FR-014)");
        assert!(reference.len() <= 12, "short enough to quote (FR-014)");
        let upper = CommentRef::of(CommentId(Uuid::from_u128(
            0xABCD_EF12_0000_4000_8000_0000_0000_0000,
        )));
        assert_eq!(upper.to_string(), "C-abcdef12", "lowercase hex");
    }

    #[test]
    fn a_reference_parses_from_either_prefix_case_and_surrounding_whitespace() {
        let expected = CommentRef::of(CommentId(Uuid::from_u128(
            0x1a2b_3c4d_0000_4000_8000_0000_0000_0000,
        )));
        for text in [
            "C-1a2b3c4d",
            "c-1a2b3c4d",
            "C-1A2B3C4D",
            "  C-1a2b3c4d\n",
            "\tc-1A2b3C4d ",
        ] {
            assert_eq!(
                CommentRef::parse(text),
                Some(expected),
                "{text:?} names the comment"
            );
        }
        for text in [
            "C-123",
            "D-1a2b3c4d",
            "C-1a2b3c4d5",
            "C-1a2b3c4g",
            "1a2b3c4d",
            "C-",
            "",
            "C- 1a2b3c4d",
        ] {
            assert_eq!(CommentRef::parse(text), None, "{text:?} is no reference");
        }
    }

    #[test]
    fn fresh_id_skips_a_drawn_id_whose_reference_is_taken() {
        let taken = CommentId(Uuid::from_u128(0x1111_1111_0000_4000_8000_0000_0000_0001));
        let collides = CommentId(Uuid::from_u128(0x1111_1111_ffff_4fff_8fff_ffff_ffff_ffff));
        let free = CommentId(Uuid::from_u128(0x2222_2222_0000_4000_8000_0000_0000_0001));
        let mut existing = sample(CommentState::Draft);
        existing.id = taken;
        let review = EntryReview::from_comments(vec![existing]);
        let mut drawn = vec![collides, free].into_iter();
        let fresh = fresh_id(&review, || drawn.next().expect("a draw"));
        assert_eq!(fresh, free, "the colliding draw is skipped (research R4)");
        let mut once = vec![free].into_iter();
        assert_eq!(
            fresh_id(&review, || once.next().expect("a draw")),
            free,
            "a free first draw is kept"
        );
    }

    #[test]
    fn a_move_keeps_the_number_of_lines() {
        let range = |a, b| LineRange::new(a, b).expect("valid");
        let m = Move::new(id(1), range(3, 5), range(10, 12)).expect("three lines to three lines");
        assert_eq!(
            (m.id(), m.from(), m.to()),
            (id(1), range(3, 5), range(10, 12))
        );
        assert_eq!(
            Move::new(id(1), range(3, 5), range(10, 11)),
            None,
            "3 lines cannot become 2"
        );
        assert_eq!(
            Move::new(id(1), range(3, 3), range(4, 5)),
            None,
            "1 line cannot become 2"
        );
    }

    #[test]
    fn reanchor_applies_a_move_only_while_the_range_and_size_match() {
        let range = |a, b| LineRange::new(a, b).expect("valid");
        let mut review = three_pending_one_sent();
        let before = review.clone();
        let moves = [
            Move::new(id(1), range(1, 1), range(8, 8)).expect("move"),
            Move::new(id(2), range(5, 5), range(9, 9)).expect("stale from"),
            Move::new(id(3), range(3, 3), range(3, 3)).expect("same place"),
            Move::new(id(404), range(1, 1), range(2, 2)).expect("unknown comment"),
        ];
        assert_eq!(review.reanchor(&moves), 1, "only the first holds");
        assert_eq!(review.reanchor(&moves), 0, "the same moves apply once");
        let moved = review
            .comments()
            .iter()
            .find(|c| c.id == id(1))
            .expect("c1");
        assert_eq!(moved.range, range(8, 8));
        let mut restored = review.comments().to_vec();
        restored[0].range = range(1, 1);
        assert_eq!(
            EntryReview::from_comments(restored).comments(),
            before.comments(),
            "quote, text, state and every other range are unchanged"
        );
    }

    #[test]
    fn reanchor_refuses_a_move_to_a_range_of_another_size_than_the_quote() {
        let range = |a, b| LineRange::new(a, b).expect("valid");
        let mut review = EntryReview::default();
        review
            .add(draft(4, 4, &["p"], "one line"), id(1), NOW)
            .expect("valid");
        // A stored range two lines long over a one-line quote, so `from` matches the range and
        // only the size guard can refuse.
        let mut stored = review.comments().to_vec();
        stored[0].range = range(4, 5);
        let mut review = EntryReview::from_comments(stored);
        let wrong = Move::new(id(1), range(4, 5), range(9, 10)).expect("same-size pair");
        assert_eq!(
            review.reanchor(&[wrong]),
            0,
            "to is not as long as the quote"
        );
        assert_eq!(review.comments()[0].range, range(4, 5));
        let ok = Move::new(id(1), range(4, 4), range(9, 9)).expect("move");
        assert_eq!(review.reanchor(&[ok]), 0, "from is not the stored range");
    }

    #[test]
    fn reanchor_moves_a_comment_in_the_open_send() {
        let range = |a, b| LineRange::new(a, b).expect("valid");
        let mut review = three_pending_one_sent();
        review.begin_send(EntryKind::Worktree, &[]).expect("opens");
        let m = Move::new(id(2), range(2, 2), range(6, 6)).expect("move");
        assert_eq!(review.reanchor(&[m]), 1);
        assert!(review.sending(), "the send stays open");
    }

    #[test]
    fn clear_sent_removes_sent_and_reported_comments_and_keeps_drafts() {
        let mut review = three_pending_one_sent();
        let mut answered = sample(reported(Outcome::Answered, "why", Reporter::User));
        answered.id = id(10);
        let mut comments = review.comments().to_vec();
        comments.push(answered);
        review = EntryReview::from_comments(comments);
        review.clear_sent();
        let ids: Vec<_> = review.comments().iter().map(|comment| comment.id).collect();
        assert_eq!(
            ids,
            vec![id(1), id(2), id(3)],
            "Sent and Reported go, Drafts stay (FR-019)"
        );
    }

    #[test]
    fn a_reported_comment_refuses_set_text_and_delete() {
        let mut answered = sample(reported(Outcome::Answered, "why", Reporter::User));
        answered.id = id(1);
        let mut review = EntryReview::from_comments(vec![answered.clone()]);
        assert_eq!(
            review.set_text(id(1), "new"),
            Err(ReviewError::Refused),
            "FR-002"
        );
        assert_eq!(review.delete(id(1)), Err(ReviewError::Refused), "FR-002");
        assert_eq!(review.comments(), &[answered][..], "nothing changed");
    }

    /// One Sent comment `9` (sent at `NOW`), one Draft `1`.
    fn one_sent_one_draft() -> EntryReview {
        let mut review = EntryReview::default();
        review
            .add(draft(1, 1, &["x"], "c1"), id(1), NOW)
            .expect("valid");
        let mut sent = sample(CommentState::Sent { at: NOW });
        sent.id = id(9);
        let mut comments = review.comments().to_vec();
        comments.push(sent);
        EntryReview::from_comments(comments)
    }

    fn agent() -> Reporter {
        Reporter::Agent {
            session: Uuid::from_u128(AGENT),
        }
    }

    #[test]
    fn report_on_a_sent_comment_makes_it_reported_and_keeps_the_send_time() {
        let mut review = one_sent_one_draft();
        let previous = review
            .report(
                CommentRef::of(id(9)),
                Outcome::Applied,
                "Renamed as asked",
                agent(),
                NOW + 50,
            )
            .expect("reported");
        assert_eq!(previous, Status::Sent);
        assert_eq!(
            state_of(&review, 9),
            CommentState::Reported {
                sent_at: NOW,
                report: Report {
                    outcome: Outcome::Applied,
                    reply: "Renamed as asked".into(),
                    first_at: NOW + 50,
                    last_at: NOW + 50,
                    by: agent(),
                },
            }
        );
    }

    #[test]
    fn a_second_report_replaces_outcome_reply_and_by_and_keeps_first_at() {
        let mut review = one_sent_one_draft();
        review
            .report(
                CommentRef::of(id(9)),
                Outcome::Applied,
                "done",
                agent(),
                NOW + 50,
            )
            .expect("first");
        let previous = review
            .report(
                CommentRef::of(id(9)),
                Outcome::Answered,
                "on second thought, no",
                Reporter::User,
                NOW + 90,
            )
            .expect("second");
        assert_eq!(previous, Status::Applied);
        let CommentState::Reported { sent_at, report } = state_of(&review, 9) else {
            panic!("reported");
        };
        assert_eq!(sent_at, NOW);
        assert_eq!(report.outcome(), Outcome::Answered);
        assert_eq!(report.reply(), "on second thought, no");
        assert_eq!(report.by(), Reporter::User);
        assert_eq!((report.first_at(), report.last_at()), (NOW + 50, NOW + 90));
    }

    #[test]
    fn applied_may_have_an_empty_reply_and_replies_are_normalised() {
        let mut review = one_sent_one_draft();
        review
            .report(
                CommentRef::of(id(9)),
                Outcome::Applied,
                "  ",
                agent(),
                NOW + 1,
            )
            .expect("an empty applied reply is accepted");
        let CommentState::Reported { report, .. } = state_of(&review, 9) else {
            panic!("reported");
        };
        assert_eq!(report.reply(), "");
        review
            .report(
                CommentRef::of(id(9)),
                Outcome::Answered,
                "\r\n one\r\ntwo\rthree \n",
                agent(),
                NOW + 2,
            )
            .expect("answered");
        let CommentState::Reported { report, .. } = state_of(&review, 9) else {
            panic!("reported");
        };
        assert_eq!(report.reply(), "one\ntwo\nthree");
    }

    #[test]
    fn refused_reports_say_why_and_change_nothing() {
        let mut review = one_sent_one_draft();
        let before = review.clone();
        let sent = CommentRef::of(id(9));
        assert_eq!(
            review.report(CommentRef::of(id(5)), Outcome::Applied, "", agent(), NOW),
            Err(ReviewError::NotFound)
        );
        assert_eq!(
            review.report(CommentRef::of(id(1)), Outcome::Applied, "", agent(), NOW),
            Err(ReviewError::Draft)
        );
        assert!(matches!(
            review.report(sent, Outcome::Answered, " \n ", agent(), NOW),
            Err(ReviewError::Invalid(_))
        ));
        let over = "é".repeat(MAX_REPLY_CHARS + 1);
        assert!(matches!(
            review.report(sent, Outcome::Applied, &over, agent(), NOW),
            Err(ReviewError::Invalid(_))
        ));
        assert_eq!(review, before);
        let limit = "é".repeat(MAX_REPLY_CHARS);
        review
            .report(sent, Outcome::Applied, &limit, agent(), NOW)
            .expect("exactly the limit is accepted");
    }

    #[test]
    fn a_comment_in_the_open_send_refuses_a_report() {
        let mut review = one_sent_one_draft();
        review.begin_send(EntryKind::Worktree, &[]).expect("send");
        let before = review.clone();
        assert_eq!(
            review.report(CommentRef::of(id(1)), Outcome::Applied, "", agent(), NOW),
            Err(ReviewError::InSend)
        );
        assert_eq!(review, before);
    }

    #[test]
    fn mark_on_a_sent_comment_is_a_report_by_the_user() {
        let mut review = one_sent_one_draft();
        review
            .mark(id(9), Outcome::Applied, "", NOW + 7)
            .expect("applied with an empty note");
        assert_eq!(
            state_of(&review, 9),
            CommentState::Reported {
                sent_at: NOW,
                report: Report {
                    outcome: Outcome::Applied,
                    reply: String::new(),
                    first_at: NOW + 7,
                    last_at: NOW + 7,
                    by: Reporter::User,
                },
            }
        );
        let mut review = one_sent_one_draft();
        review
            .mark(
                id(9),
                Outcome::Answered,
                "\r\n  not now\r\nlater \n",
                NOW + 8,
            )
            .expect("answered");
        let CommentState::Reported { report, .. } = state_of(&review, 9) else {
            panic!("reported");
        };
        assert_eq!(report.reply(), "not now\nlater");
        assert_eq!(report.by(), Reporter::User);
        assert_eq!((report.first_at(), report.last_at()), (NOW + 8, NOW + 8));
    }

    #[test]
    fn refused_marks_say_why_and_change_nothing() {
        let mut review = one_sent_one_draft();
        let mut already = sample(reported(Outcome::Applied, "", Reporter::User));
        already.id = id(8);
        let mut comments = review.comments().to_vec();
        comments.push(already);
        review = EntryReview::from_comments(comments);
        let before = review.clone();
        assert_eq!(
            review.mark(id(5), Outcome::Applied, "", NOW),
            Err(ReviewError::NotFound)
        );
        assert_eq!(
            review.mark(id(1), Outcome::Applied, "", NOW),
            Err(ReviewError::Draft)
        );
        assert_eq!(
            review.mark(id(8), Outcome::Applied, "", NOW),
            Err(ReviewError::Refused)
        );
        assert!(matches!(
            review.mark(id(9), Outcome::Answered, " \n ", NOW),
            Err(ReviewError::Invalid(_))
        ));
        let over = "é".repeat(MAX_REPLY_CHARS + 1);
        assert!(matches!(
            review.mark(id(9), Outcome::Answered, &over, NOW),
            Err(ReviewError::Invalid(_))
        ));
        assert_eq!(review, before);
        review
            .mark(id(9), Outcome::Answered, &"é".repeat(MAX_REPLY_CHARS), NOW)
            .expect("exactly the limit is accepted");
    }

    #[test]
    fn a_comment_in_the_open_send_refuses_a_mark() {
        let mut review = one_sent_one_draft();
        review.begin_send(EntryKind::Worktree, &[]).expect("send");
        let before = review.clone();
        assert_eq!(
            review.mark(id(1), Outcome::Applied, "", NOW),
            Err(ReviewError::InSend)
        );
        assert_eq!(review, before);
    }

    #[test]
    fn no_operation_turns_a_sent_or_reported_comment_back_into_a_draft() {
        type Op = fn(&mut EntryReview);
        let ops: [(&str, Op); 6] = [
            ("finish_send", |r| {
                let snapshot = SendSnapshot {
                    ids: vec![id(9)],
                    prompt: String::new(),
                };
                r.finish_send(&snapshot, NOW + 1);
            }),
            ("report", |r| {
                let _ = r.report(CommentRef::of(id(9)), Outcome::Applied, "", agent(), NOW);
            }),
            ("mark", |r| {
                let _ = r.mark(id(9), Outcome::Answered, "no", NOW);
            }),
            ("clear_sent", |r| r.clear_sent()),
            ("set_text", |r| {
                let _ = r.set_text(id(9), "again");
            }),
            ("delete", |r| {
                let _ = r.delete(id(9));
            }),
        ];
        for starting in [
            CommentState::Sent { at: NOW },
            reported(Outcome::Answered, "why", agent()),
        ] {
            for (name, op) in &ops {
                let mut review = one_sent_one_draft();
                let mut comments = review.comments().to_vec();
                comments[1].state = starting.clone();
                review = EntryReview::from_comments(comments);
                op(&mut review);
                assert!(
                    review
                        .comments()
                        .iter()
                        .filter(|c| c.id == id(9))
                        .all(|c| c.state != CommentState::Draft),
                    "{name} turned {starting:?} back into a draft"
                );
            }
        }
    }
}
