//! The window's rich render cache (feature 940, data-model.md *Render cache*, research R5, R7, R8):
//! per structured session, the drawing state derived from its transcript — markdown documents for
//! assistant text and thinking, diffs for Edit/MultiEdit/Write cards — with the highlight results,
//! copy confirmations and "show in full" choices of the window's code blocks and diffs.
//!
//! Render-free: this decides what is derived, when it is pruned and which parts are highlighted;
//! the shell runs the [`HighlightJob`]s off the update thread with `ui::syntax` and performs the
//! clipboard and timer requests, and `ui/transcript/` draws what is held here.

use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::time::Duration;

use micold_core::markdown::{CodeId, Document};
use micold_core::review::diff::Spans;
use micold_core::review::limits;
use micold_core::session::SessionId;
use micold_core::structured::edit_diff::{edit_diff, EditDiff};
use micold_core::structured::transcript::{EntryKey, EntryKind, Transcript};

/// How long a copy control shows "Copied" or "Copy failed" before it is idle again (FR-012).
pub const COPY_CONFIRM: Duration = Duration::from_secs(2);

/// One copy's identity: monotonic per window, so a stale timer or read-back cannot touch a newer
/// copy's state (research R5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Ticket(u64);

/// A highlighted or size-limited part of an entry: one of its code blocks, or its diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Part {
    Code(CodeId),
    Diff,
}

/// A code block's copy control; absent from [`Derived::copy`] is idle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyState {
    /// Written; the read-back has not answered yet.
    Pending(Ticket),
    /// The read-back held the code.
    Copied(Ticket),
    /// The read-back held something else, or nothing.
    Failed(Ticket),
}

/// What an entry is drawn from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DerivedBody {
    /// Assistant text or thinking, as markdown.
    Text(Document),
    /// An Edit/MultiEdit/Write card's change; `None` while its input streams or when it is
    /// malformed, and the card shows #939's plain input (FR-018).
    Diff(Option<EditDiff>),
}

/// One entry's derived drawing state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Derived {
    /// The transcript revision it was derived from.
    pub revision: u64,
    /// Bumped on every change to the drawing that is not a revision change (spans arrived, copy
    /// state, show in full); part of the entry's lazy key (RR-3).
    pub stamp: u64,
    pub body: DerivedBody,
    /// Highlight results in both schemes, by part: the part's lines in [`Spans`]' `new` side, in
    /// the order of its [`HighlightJob::lines`].
    pub spans: HashMap<Part, Spans>,
    pub copy: HashMap<CodeId, CopyState>,
    /// Parts the user chose to show in full (FR-019).
    pub full: HashSet<Part>,
    known: HashMap<Part, Known>,
}

/// A part to highlight, run off the update thread by the shell (research R4) and answered by
/// `TranscriptMsg::Highlighted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightJob {
    pub session: SessionId,
    pub key: EntryKey,
    pub part: Part,
    /// The grammar token.
    pub token: String,
    /// The lines to colour, without line endings.
    pub lines: Vec<String>,
}

/// What the cache asks of a grammar set, injected by the shell (`ui::syntax::{knows, token}`) so
/// the cache decides which parts get a job without depending on `ui`.
#[derive(Debug, Clone, Copy)]
pub struct Grammar {
    /// Whether a grammar other than plain text exists for a token.
    pub knows: fn(&str) -> bool,
    /// A file path's token.
    pub token: fn(&str) -> String,
}

/// One session's derived entries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionRich {
    entries: HashMap<EntryKey, Derived>,
}

/// The window's cache: one [`SessionRich`] per structured session (RR-5).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RichCache {
    sessions: HashMap<SessionId, SessionRich>,
    /// The last ticket given out.
    next_ticket: u64,
    /// The copies whose control is not idle, by ticket.
    tickets: HashMap<Ticket, Copying>,
}

/// What an entry is derived from, when it is one the cache derives.
enum Source<'a> {
    Text {
        text: &'a str,
        streaming: bool,
    },
    Edit {
        tool: &'a str,
        input: &'a str,
        streaming: bool,
    },
}

impl<'a> Source<'a> {
    fn of(kind: &'a EntryKind) -> Option<Self> {
        match kind {
            EntryKind::AssistantText { text, streaming }
            | EntryKind::Thinking { text, streaming } => Some(Self::Text {
                text,
                streaming: *streaming,
            }),
            EntryKind::ToolCall(card) if EDIT_TOOLS.contains(&card.name.as_str()) => {
                Some(Self::Edit {
                    tool: &card.name,
                    input: &card.input,
                    streaming: card.streaming,
                })
            }
            _ => None,
        }
    }
}

/// The tools whose cards draw a diff (FR-013).
const EDIT_TOOLS: [&str; 3] = ["Edit", "MultiEdit", "Write"];

impl DerivedBody {
    fn of(source: &Source<'_>) -> Self {
        match *source {
            Source::Text { text, streaming } => Self::Text(Document::parse(text, streaming)),
            Source::Edit {
                streaming: true, ..
            } => Self::Diff(None),
            Source::Edit { tool, input, .. } => Self::Diff(edit_diff(tool, input)),
        }
    }

    /// The body of `source`, as `self` updated: a document re-parses only its open blocks (R2).
    fn update(&mut self, source: &Source<'_>) {
        match (self, source) {
            (Self::Text(doc), Source::Text { text, streaming }) => doc.update(text, *streaming),
            (body, source) => *body = Self::of(source),
        }
    }

    /// `part`'s lines, its size in lines and in bytes, when the body holds it.
    fn part(&self, part: Part) -> Option<(Vec<String>, u64, u64)> {
        match (self, part) {
            (Self::Text(doc), Part::Code(id)) => {
                let code = doc.code(id)?;
                let lines = code.code.lines().map(str::to_owned).collect();
                Some((lines, u64::from(code.lines), code.bytes))
            }
            (Self::Diff(Some(diff)), Part::Diff) => {
                let lines = diff
                    .hunks
                    .iter()
                    .flat_map(|hunk| &hunk.lines)
                    .map(|line| line.text.clone())
                    .collect();
                Some((
                    lines,
                    u64::from(diff.added) + u64::from(diff.removed),
                    diff.bytes,
                ))
            }
            _ => None,
        }
    }

    /// The parts a grammar would colour now, with their tokens: settled code blocks of a known
    /// language, and a complete diff of a file of one (research R4).
    fn colourable(&self, grammar: &Grammar) -> Vec<(Part, String)> {
        match self {
            Self::Text(doc) => doc
                .code_blocks()
                .filter(|(_, _, settled)| *settled)
                .filter_map(|(id, code, _)| {
                    let token = code.token.as_ref()?;
                    (grammar.knows)(token).then(|| (Part::Code(id), token.clone()))
                })
                .collect(),
            Self::Diff(Some(diff)) => {
                let token = (grammar.token)(&diff.path);
                if (grammar.knows)(&token) {
                    vec![(Part::Diff, token)]
                } else {
                    Vec::new()
                }
            }
            Self::Diff(None) => Vec::new(),
        }
    }

    /// Whether `part` exists in the body: a code block it holds, or its diff.
    fn holds(&self, part: Part) -> bool {
        match (self, part) {
            (Self::Text(doc), Part::Code(id)) => doc.code(id).is_some(),
            (Self::Diff(Some(_)), Part::Diff) => true,
            _ => false,
        }
    }
}

/// A part a grammar colours: its token, a fingerprint of its lines, and whether its job was
/// issued (so it is issued once, research R8).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Known {
    token: String,
    print: u64,
    asked: bool,
}

fn fingerprint(lines: &[String]) -> u64 {
    let mut hasher = DefaultHasher::new();
    lines.hash(&mut hasher);
    hasher.finish()
}

impl Derived {
    fn new(revision: u64, body: DerivedBody) -> Self {
        Self {
            revision,
            stamp: 0,
            body,
            spans: HashMap::new(),
            copy: HashMap::new(),
            full: HashSet::new(),
            known: HashMap::new(),
        }
    }

    /// Whether `part` is over the Changes view's limits (`review::limits`: lines, or bytes) and
    /// not chosen to be shown in full (FR-019).
    pub fn is_large(&self, part: Part) -> bool {
        !self.full.contains(&part)
            && self.body.part(part).is_some_and(|(_, lines, bytes)| {
                lines > u64::from(limits::MAX_CHANGED_LINES) || bytes > limits::MAX_VERSION_BYTES
            })
    }

    /// After the body changed: keep what still holds of the parts' state, drop the rest.
    fn settle(&mut self, grammar: &Grammar) {
        let mut known = HashMap::new();
        for (part, token) in self.body.colourable(grammar) {
            let Some((lines, ..)) = self.body.part(part) else {
                continue;
            };
            let print = fingerprint(&lines);
            let kept = self
                .known
                .remove(&part)
                .filter(|k| k.token == token && k.print == print);
            if kept.is_none() {
                self.spans.remove(&part);
            }
            known.insert(
                part,
                kept.unwrap_or(Known {
                    token,
                    print,
                    asked: false,
                }),
            );
        }
        self.known = known;
        let (body, known) = (&self.body, &self.known);
        self.spans.retain(|part, _| known.contains_key(part));
        self.full.retain(|part| body.holds(*part));
        self.copy.retain(|id, _| body.holds(Part::Code(*id)));
    }

    /// `part`'s job, when it is due: colourable, not asked yet and not over the limits.
    fn job(&mut self, session: SessionId, key: &EntryKey, part: Part) -> Option<HighlightJob> {
        let large = self.is_large(part);
        let known = self.known.get_mut(&part).filter(|k| !k.asked && !large)?;
        let (lines, ..) = self.body.part(part)?;
        known.asked = true;
        Some(HighlightJob {
            session,
            key: key.clone(),
            part,
            token: known.token.clone(),
            lines,
        })
    }

    /// Every job that is due, in part order.
    fn jobs(&mut self, session: SessionId, key: &EntryKey) -> Vec<HighlightJob> {
        let mut parts: Vec<Part> = self.known.keys().copied().collect();
        parts.sort();
        parts
            .into_iter()
            .filter_map(|part| self.job(session, key, part))
            .collect()
    }
}

impl CopyState {
    fn ticket(self) -> Ticket {
        match self {
            Self::Pending(t) | Self::Copied(t) | Self::Failed(t) => t,
        }
    }
}

/// Where a ticket's copy is, and the text it wrote until the read-back answers.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Copying {
    session: SessionId,
    key: EntryKey,
    code: CodeId,
    text: Option<String>,
}

impl RichCache {
    /// Derive `session`'s entries whose revision changed, prune keys `transcript` no longer
    /// holds, and return the highlight jobs that became due (RR-4).
    pub fn refresh(
        &mut self,
        session: SessionId,
        transcript: &Transcript,
        grammar: &Grammar,
    ) -> Vec<HighlightJob> {
        let s = self.sessions.entry(session).or_default();
        let mut present = HashSet::new();
        let mut jobs = Vec::new();
        for entry in transcript.entries() {
            let Some(source) = Source::of(entry.kind()) else {
                continue;
            };
            let key = entry.key();
            present.insert(key.clone());
            let revision = entry.revision();
            if s.entries.get(key).is_some_and(|d| d.revision == revision) {
                continue;
            }
            let derived = s
                .entries
                .entry(key.clone())
                .and_modify(|d| {
                    d.revision = revision;
                    d.body.update(&source);
                })
                .or_insert_with(|| Derived::new(revision, DerivedBody::of(&source)));
            derived.settle(grammar);
            jobs.extend(derived.jobs(session, key));
        }
        s.entries.retain(|key, _| present.contains(key));
        // A ticket whose copy state went with its entry or block answers nothing any more.
        let entries = &s.entries;
        self.tickets.retain(|ticket, c| {
            c.session != session
                || entries
                    .get(&c.key)
                    .and_then(|d| d.copy.get(&c.code))
                    .is_some_and(|state| state.ticket() == *ticket)
        });
        jobs
    }
    /// Store a highlight job's `spans` for `part` when the part still exists and its job was
    /// issued, one line of colour per line of it; bumps the stamp (research R4, R8). A late answer
    /// for content that changed meanwhile is replaced when the newer job answers.
    pub fn highlighted(
        &mut self,
        session: SessionId,
        key: &EntryKey,
        part: Part,
        spans: Spans,
    ) -> bool {
        let Some(derived) = self.entry_mut(session, key) else {
            return false;
        };
        let asked = derived.known.get(&part).is_some_and(|k| k.asked);
        let lines = derived.body.part(part).map(|(lines, ..)| lines.len());
        let fits =
            lines.is_some_and(|n| spans.light.new.0.len() == n && spans.dark.new.0.len() == n);
        if !(asked && fits) {
            return false;
        }
        derived.spans.insert(part, spans);
        derived.stamp += 1;
        true
    }

    /// Start a copy of code block `code` (FR-011): its code exactly, the code so far while it
    /// streams, and the copy's ticket; the block's control is `Pending` until the read-back.
    pub fn copy(
        &mut self,
        session: SessionId,
        key: &EntryKey,
        code: CodeId,
    ) -> Option<(String, Ticket)> {
        let derived = self.sessions.get_mut(&session)?.entries.get_mut(key)?;
        let DerivedBody::Text(doc) = &derived.body else {
            return None;
        };
        let text = doc.code(code)?.code.clone();
        self.next_ticket += 1;
        let ticket = Ticket(self.next_ticket);
        if let Some(old) = derived.copy.insert(code, CopyState::Pending(ticket)) {
            self.tickets.remove(&old.ticket());
        }
        derived.stamp += 1;
        self.tickets.insert(
            ticket,
            Copying {
                session,
                key: key.clone(),
                code,
                text: Some(text.clone()),
            },
        );
        Some((text, ticket))
    }

    /// The clipboard read back after `ticket`'s write (FR-012): `Copied` when it holds the code,
    /// else `Failed`. Whether it changed the control, which then wants its confirmation timer.
    pub fn copy_checked(&mut self, ticket: Ticket, read: Option<String>) -> bool {
        let Some(copying) = self.tickets.get_mut(&ticket) else {
            return false;
        };
        let Some(derived) = self
            .sessions
            .get_mut(&copying.session)
            .and_then(|s| s.entries.get_mut(&copying.key))
        else {
            return false;
        };
        let Some(state) = derived
            .copy
            .get_mut(&copying.code)
            .filter(|state| **state == CopyState::Pending(ticket))
        else {
            return false;
        };
        let written = copying.text.take();
        *state = if read.is_some() && read == written {
            CopyState::Copied(ticket)
        } else {
            CopyState::Failed(ticket)
        };
        derived.stamp += 1;
        true
    }

    /// `ticket`'s confirmation is over: the control is idle again, unless a newer copy of the
    /// block holds it. Whether it changed the control.
    pub fn copy_expired(&mut self, ticket: Ticket) -> bool {
        let Some(copying) = self.tickets.get(&ticket) else {
            return false;
        };
        let (session, key, code) = (copying.session, copying.key.clone(), copying.code);
        let Some(derived) = self.entry_mut(session, &key) else {
            self.tickets.remove(&ticket);
            return false;
        };
        match derived.copy.get(&code) {
            Some(CopyState::Copied(t) | CopyState::Failed(t)) if *t == ticket => {
                derived.copy.remove(&code);
                derived.stamp += 1;
                self.tickets.remove(&ticket);
                true
            }
            Some(CopyState::Pending(t)) if *t == ticket => false,
            _ => {
                self.tickets.remove(&ticket);
                false
            }
        }
    }

    /// The user chose to show `part` in full (FR-019): recorded for this session's entry only,
    /// and the part's highlight job when one is due.
    pub fn show_in_full(
        &mut self,
        session: SessionId,
        key: &EntryKey,
        part: Part,
    ) -> Option<HighlightJob> {
        let derived = self.entry_mut(session, key)?;
        if !derived.body.holds(part) {
            return None;
        }
        if derived.full.insert(part) {
            derived.stamp += 1;
        }
        derived.job(session, key, part)
    }

    /// Drop `session`'s cache with its transcript.
    pub fn forget(&mut self, session: SessionId) {
        self.sessions.remove(&session);
        self.tickets.retain(|_, c| c.session != session);
    }

    /// What the cache holds for `session`'s entry `key`.
    pub fn get(&self, session: SessionId, key: &EntryKey) -> Option<&Derived> {
        self.sessions.get(&session)?.entries.get(key)
    }

    /// An Edit/MultiEdit/Write card's `(added, removed)` lines, once its diff is known (FR-017).
    pub fn edit_counts(&self, session: SessionId, key: &EntryKey) -> Option<(u32, u32)> {
        match &self.get(session, key)?.body {
            DerivedBody::Diff(Some(diff)) => Some((diff.added, diff.removed)),
            _ => None,
        }
    }

    fn entry_mut(&mut self, session: SessionId, key: &EntryKey) -> Option<&mut Derived> {
        self.sessions.get_mut(&session)?.entries.get_mut(key)
    }
}
