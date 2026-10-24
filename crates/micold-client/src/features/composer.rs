//! The structured session's composer, render-free (feature 941, data-model *Window*, research
//! R10, R11).
//!
//! Per session and per window (FR-007): the draft (text, caret, pictures), the walk through the
//! session's earlier messages (FR-023, FR-024), the send waiting for the service's answer and its
//! restore (FR-005, SC-005), the notice line, the queued message being edited (FR-020), and the
//! last `StructuredInput` the service sent (phase, hold, queue). The keys are decided by
//! [`composer_key`] and [`transcript_key`]; each composer keeps its text editor beside its draft
//! (autopilot Decision: here, not in the binary, so the view draws it), and the shell
//! (`shell/composer.rs`) sends what this decides.
//!
//! The caret is the editor's: `(line, column)` with the column a byte offset into the line.

use std::collections::HashMap;
use std::path::Path;

use micold_core::path_insert::{plan_insertion, InsertTarget, SandboxShare, ShellKind};
use micold_core::protocol::messages::{
    AttachmentMeta, ImageAttachment, InterruptAsk, QueueEntryId, QueuedSummary, TerminalRef,
    TurnPhase,
};
use micold_core::session::SessionId;
use micold_core::structured::attachment::{admit, AttachmentRefusal, DropPlan};
use micold_core::structured::transcript::Transcript;

use crate::features::command_palette::{classify, Classified, PaletteKey, Picked, SendIntent};
use crate::features::session::{SwitchOffer, SwitchRequest, LOGIN_COMMAND, LOGIN_REASON};
use crate::keymap::{Key, KeyInput, NamedKey};
use micold_core::session::SessionKind;
use micold_core::structured::model::ModelChoice;
use micold_core::typeahead::Direction;

/// The notice a send gets back when the connection to the service went before it was answered.
pub const CONNECTION_LOST: &str =
    "The connection to the session service was lost before it confirmed the message; it is back \
     in the composer.";

/// The notice of an interrupt the CLI has not acted on (FR-015).
pub const OVERDUE: &str =
    "The interrupt has not taken effect yet. You can interrupt again or stop the session.";

/// What the composer says while it cannot send (FR-006).
pub const NOT_CONNECTED: &str = "Not connected to the session service: the draft is kept.";
/// See [`NOT_CONNECTED`].
pub const NOT_RUNNING: &str = "The session is not running: the draft is kept.";

/// The notice of a queue edit whose entry was sent or removed meanwhile.
pub const EDIT_GONE: &str = "The queued message was sent or removed before the edit was saved.";

/// What a key press in the composer means (data-model *KeyIntent*).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyIntent {
    /// Send the draft (or queue it while busy).
    Send,
    /// Insert a line break.
    NewLine,
    /// Recall the next older message.
    HistoryBack,
    /// Recall the next newer message, or the draft past the newest.
    HistoryForward,
    /// Interrupt the turn in progress.
    Interrupt,
    /// Not the composer's: the editor has it (typing, caret moves, an IME composition).
    Edit,
}

fn is_busy(phase: TurnPhase) -> bool {
    matches!(phase, TurnPhase::Busy { .. })
}

fn plain(press: &KeyInput) -> bool {
    !press.mods.ctrl && !press.mods.alt && !press.mods.logo
}

/// What `press` means in a composer whose caret is at `caret` in a draft of `lines` lines, while
/// an IME composition is open (`composing`) or not, for a session in `phase` (FR-002, FR-012,
/// FR-023). No key needs a platform modifier (R12).
pub fn composer_key(
    press: &KeyInput,
    caret: (usize, usize),
    lines: usize,
    composing: bool,
    phase: TurnPhase,
) -> KeyIntent {
    if composing || !plain(press) {
        return KeyIntent::Edit;
    }
    let Key::Named(named) = press.key else {
        return KeyIntent::Edit;
    };
    match named {
        NamedKey::Enter if press.mods.shift => KeyIntent::NewLine,
        NamedKey::Enter => KeyIntent::Send,
        NamedKey::ArrowUp if !press.mods.shift && caret.0 == 0 => KeyIntent::HistoryBack,
        NamedKey::ArrowDown if !press.mods.shift && caret.0 + 1 >= lines.max(1) => {
            KeyIntent::HistoryForward
        }
        NamedKey::Escape if !press.mods.shift && is_busy(phase) => KeyIntent::Interrupt,
        _ => KeyIntent::Edit,
    }
}

/// What `press` means on a focused transcript: Escape interrupts a busy session (FR-012).
pub fn transcript_key(press: &KeyInput, phase: TurnPhase) -> KeyIntent {
    match press.key {
        Key::Named(NamedKey::Escape) if plain(press) && !press.mods.shift && is_busy(phase) => {
            KeyIntent::Interrupt
        }
        _ => KeyIntent::Edit,
    }
}

/// Whether a pane showing `pane` (none: an empty pane) shows the composer: only a structured
/// session's primary, which the pane draws as its transcript (FR-001).
pub fn shows_composer(state: &crate::app::State, pane: Option<TerminalRef>) -> bool {
    pane.is_some_and(|terminal| state.is_structured_primary(terminal))
}

/// A message being written: its text, the caret and its pictures in the order attached.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Draft {
    pub text: String,
    pub caret: (usize, usize),
    pub images: Vec<ImageAttachment>,
}

impl Draft {
    /// Text other than whitespace, or a picture (FR-004).
    pub fn is_sendable(&self) -> bool {
        !self.text.trim().is_empty() || !self.images.is_empty()
    }

    /// Nothing at all: no text, not even whitespace, and no picture.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty() && self.images.is_empty()
    }

    /// A draft of `text` with the caret at its end.
    fn at_end(text: String, images: Vec<ImageAttachment>) -> Self {
        let caret = end_of(&text);
        Self {
            text,
            caret,
            images,
        }
    }

    /// `returned` put back into this draft without losing either: into an empty draft as it
    /// was; otherwise first, with this draft's text after it on a new line and the caret kept in
    /// that text (autopilot Decision 15).
    fn put_back(self, returned: Draft) -> Draft {
        if self.is_empty() {
            return returned;
        }
        let shift = returned.text.split('\n').count();
        let mut images = returned.images;
        images.extend(self.images);
        Draft {
            text: format!("{}\n{}", returned.text, self.text),
            caret: (self.caret.0 + shift, self.caret.1),
            images,
        }
    }
}

/// The caret at the end of `text`.
fn end_of(text: &str) -> (usize, usize) {
    let mut lines = text.split('\n');
    let last = lines.next_back().unwrap_or("");
    (text.matches('\n').count(), last.len())
}

/// The byte offset of `caret` in `text`, clamped to the text and to a character boundary.
fn offset_of(text: &str, caret: (usize, usize)) -> usize {
    let mut start = 0;
    for (index, line) in text.split('\n').enumerate() {
        if index == caret.0 {
            let mut column = caret.1.min(line.len());
            while !line.is_char_boundary(column) {
                column -= 1;
            }
            return start + column;
        }
        start += line.len() + 1;
    }
    text.len()
}

/// Where the user is in the session's earlier messages (FR-023, FR-024).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryWalk {
    /// How far back: 0 is the newest message.
    pub index: usize,
    /// The draft the user had before stepping in, restored past the newest.
    pub saved: Draft,
}

/// The last `StructuredInput` the service sent for the session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputState {
    pub phase: TurnPhase,
    /// The queue is on hold until **Send queued**.
    pub hold: bool,
    pub queue: Vec<QueuedSummary>,
}

/// A queued message being edited in place (FR-020): its new text and the pictures it keeps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueEditDraft {
    pub entry: QueueEntryId,
    pub text: String,
    /// The pictures kept, as the window shows them.
    pub attachments: Vec<AttachmentMeta>,
    /// Their indexes in the queued message, for `QueueEdit::keep`.
    pub keep: Vec<u32>,
}

/// One session's composer in this window.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Composer {
    pub draft: Draft,
    pub history: Option<HistoryWalk>,
    /// The send waiting for its answer: its request and the draft that was sent.
    pub pending: Option<(u64, Draft)>,
    pub notice: Option<String>,
    pub editing: Option<QueueEditDraft>,
    pub remote: Option<InputState>,
    /// The text editor the composer draws: the draft's text and caret, with its selection and
    /// scroll. Typing edits it and then the draft; the composer replacing the draft (a send, a
    /// restore, a history step, a drop, a queued message taken back) shows the new draft in it.
    pub editor: crate::app::Editor,
    /// A preview of each of the draft's pictures, in order, made once per picture.
    pub previews: Vec<crate::app::Preview>,
}

impl Composer {
    fn phase(&self) -> TurnPhase {
        self.remote
            .as_ref()
            .map_or(TurnPhase::NotRunning, |r| r.phase)
    }

    fn replace_draft(&mut self, draft: Draft) {
        self.draft = draft;
        self.show_draft();
    }

    /// Show the draft in the editor again, after the composer changed it.
    fn show_draft(&mut self) {
        self.editor = crate::app::Editor::at(&self.draft.text, self.draft.caret);
        self.sync_previews();
    }

    /// One preview per picture of the draft, keeping each one already made.
    fn sync_previews(&mut self) {
        let made = std::mem::take(&mut self.previews);
        self.previews = self
            .draft
            .images
            .iter()
            .map(|image| {
                made.iter()
                    .find(|p| p.shows(image.bytes()))
                    .cloned()
                    .unwrap_or_else(|| crate::app::Preview::of(image.bytes()))
            })
            .collect();
    }
}

/// Files dropped on a structured session's pane, as the shell read them (FR-026 to FR-029).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DroppedFiles {
    /// The drop split into pictures and other paths.
    pub plan: DropPlan,
    /// Each of `plan.images`' reads, in order: its bytes, or why it could not be read.
    pub reads: Vec<Result<Vec<u8>, String>>,
    /// The shell the paths are quoted for, as for a terminal drop.
    pub shell: ShellKind,
    /// What a sandboxed session shares, as for a terminal drop.
    pub sandbox: Option<SandboxShare>,
}

/// What the composer feature hears: the shell's half (`shell/composer.rs`) runs each.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// A key pressed in `session`'s composer, while an IME composition is open or not.
    Key {
        session: SessionId,
        press: KeyInput,
        composing: bool,
    },
    /// A key pressed on `session`'s focused transcript.
    TranscriptKey { session: SessionId, press: KeyInput },
    /// The text editor of `session`'s composer did something.
    Edited {
        session: SessionId,
        action: crate::app::EditorAction,
    },
    /// The send control.
    SendRequested(SessionId),
    /// The stop control: interrupt the turn in progress.
    StopRequested(SessionId),
    /// **Stop session**, offered while an interrupt is overdue (FR-015).
    StopSessionRequested(SessionId),
    /// **Send queued** (FR-022).
    SendQueuedRequested(SessionId),
    /// Remove a picture from the draft.
    AttachmentRemoved { session: SessionId, index: usize },
    /// Remove a queued message.
    QueueRemoveRequested {
        session: SessionId,
        entry: QueueEntryId,
    },
    /// Move a queued message back into the composer.
    QueueTakeRequested {
        session: SessionId,
        entry: QueueEntryId,
    },
    /// Start editing a queued message.
    QueueEditStarted {
        session: SessionId,
        entry: QueueEntryId,
    },
    /// The queued message's new text.
    QueueEditChanged { session: SessionId, text: String },
    /// Drop one of the queued message's pictures.
    QueueEditAttachmentRemoved { session: SessionId, index: usize },
    /// Save the queue edit.
    QueueEditSaved(SessionId),
    /// Leave the queue edit unsaved.
    QueueEditCancelled(SessionId),
    /// Files dropped on `session`'s pane, read.
    Dropped {
        session: SessionId,
        files: Box<DroppedFiles>,
    },
    /// A click on the command palette's row `row` (#943 FR-008a).
    PaletteClicked { session: SessionId, row: usize },
    /// Open the model picker from the view (#943 FR-014).
    ModelPickerRequested(SessionId),
    /// The name typed into the model picker.
    ModelPickerTyped { session: SessionId, text: String },
    /// A model picked: item `item`, or the typed name when `None`.
    ModelPicked {
        session: SessionId,
        item: Option<usize>,
    },
    /// The arrows in the model picker.
    ModelPickerMoved {
        session: SessionId,
        direction: micold_core::typeahead::Direction,
    },
    /// The model picker left without a pick.
    ModelPickerClosed(SessionId),
    /// **Open in terminal** / **Open in structured view** chosen (#943 US4 scenarios 3, 4).
    SwitchRequested {
        session: SessionId,
        to: micold_core::session::SessionKind,
    },
    /// The switch offer or its confirmation accepted (#943 FR-010, FR-021).
    SwitchConfirmed(SessionId),
    /// The switch offer dismissed: nothing changes.
    SwitchDismissed(SessionId),
    /// The `/login` offer beside an authentication failure taken (#943 FR-022a).
    LoginOffered(SessionId),
}

/// Every session's composer in this window (data-model *Composers*).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    composers: HashMap<SessionId, Composer>,
    /// The command palettes and model pickers open over these composers (#943).
    pub palette: crate::features::command_palette::State,
    /// The open offer to switch a Claude session's kind, with its confirmation step once the
    /// service asked for one (#943 FR-010, FR-021, FR-022a). Its presence *is* the offer shown.
    pub switch_offer: Option<crate::features::session::SwitchOffer>,
    /// The editor of a session that has no composer yet: empty.
    blank: crate::app::Editor,
}

impl State {
    /// The editor `session`'s composer draws: an empty one before it has a composer.
    pub fn editor(&self, session: SessionId) -> &crate::app::Editor {
        self.composer(session).map_or(&self.blank, |c| &c.editor)
    }

    /// `session`'s editor, to act on; its composer is made the first time.
    pub fn editor_mut(&mut self, session: SessionId) -> &mut crate::app::Editor {
        &mut self.entry(session).editor
    }

    pub fn composer(&self, session: SessionId) -> Option<&Composer> {
        self.composers.get(&session)
    }

    pub fn draft(&self, session: SessionId) -> Option<&Draft> {
        self.composer(session).map(|c| &c.draft)
    }

    fn entry(&mut self, session: SessionId) -> &mut Composer {
        self.composers.entry(session).or_default()
    }

    /// The phase the service last reported for `session`; not running until it reports.
    pub fn phase(&self, session: SessionId) -> TurnPhase {
        self.composer(session)
            .map_or(TurnPhase::NotRunning, Composer::phase)
    }

    /// The editor now shows `text` with the caret at `caret`: the user typed or moved.
    pub fn edited(&mut self, session: SessionId, text: String, caret: (usize, usize)) {
        let draft = &mut self.entry(session).draft;
        draft.text = text;
        draft.caret = caret;
    }

    /// The draft's text becomes `text` with the caret at `caret`, its pictures kept: the
    /// palette filled a command in, or a command the app handles was used up (#943 FR-008a).
    pub fn replace_text(&mut self, session: SessionId, text: String, caret: (usize, usize)) {
        let composer = self.entry(session);
        composer.history = None;
        let images = std::mem::take(&mut composer.draft.images);
        composer.replace_draft(Draft {
            text,
            caret,
            images,
        });
    }

    /// Forget `session`'s composer (the session is gone).
    pub fn forget(&mut self, session: SessionId) {
        self.composers.remove(&session);
        self.palette.forget(session);
        if self
            .switch_offer
            .as_ref()
            .is_some_and(|o| o.session == session)
        {
            self.switch_offer = None;
        }
    }

    /// Step back to the next older message of `transcript`. `false` when there is none to step
    /// to, so Up stays a caret move (FR-023).
    pub fn history_back(&mut self, session: SessionId, transcript: Option<&Transcript>) -> bool {
        let Some(transcript) = transcript else {
            return false;
        };
        let count = transcript.user_messages().count();
        if count == 0 {
            return false;
        }
        let composer = self.entry(session);
        let index = match &composer.history {
            None => {
                composer.history = Some(HistoryWalk {
                    index: 0,
                    saved: composer.draft.clone(),
                });
                0
            }
            // At the oldest already: nothing older, and an edit of it stays.
            Some(walk) if walk.index + 1 >= count => return true,
            Some(walk) => walk.index + 1,
        };
        if let Some(walk) = composer.history.as_mut() {
            walk.index = index;
        }
        recall(composer, transcript, index, false);
        true
    }

    /// Step forward to the next newer message, or past the newest to the draft the user had
    /// (FR-024). `false` outside the history, so Down stays a caret move.
    pub fn history_forward(&mut self, session: SessionId, transcript: Option<&Transcript>) -> bool {
        let composer = self.entry(session);
        let Some(walk) = composer.history.take() else {
            return false;
        };
        match (walk.index, transcript) {
            (0, _) | (_, None) => composer.replace_draft(walk.saved),
            (index, Some(transcript)) => {
                composer.history = Some(HistoryWalk {
                    index: index - 1,
                    saved: walk.saved,
                });
                recall(composer, transcript, index - 1, true);
            }
        }
        true
    }

    /// Why `session`'s composer cannot send now, whatever the draft holds (FR-006).
    pub fn unavailable(&self, session: SessionId, connected: bool) -> Option<&'static str> {
        if !connected {
            Some(NOT_CONNECTED)
        } else if self.phase(session) == TurnPhase::NotRunning {
            Some(NOT_RUNNING)
        } else {
            None
        }
    }

    /// Whether the draft can be sent: sendable, connected, running and no send waiting (FR-004,
    /// FR-006). A busy session takes it into its queue.
    pub fn can_send(&self, session: SessionId, connected: bool) -> bool {
        self.unavailable(session, connected).is_none()
            && self
                .composer(session)
                .is_some_and(|c| c.pending.is_none() && c.draft.is_sendable())
    }

    /// Take the draft to send it, emptying the composer; `None` when it cannot be sent. The
    /// shell sends it and hands it back with [`State::sending`] (or [`State::returned`]).
    pub fn take_send(&mut self, session: SessionId, connected: bool) -> Option<Draft> {
        if !self.can_send(session, connected) {
            return None;
        }
        let composer = self.entry(session);
        composer.history = None;
        composer.notice = None;
        let draft = std::mem::take(&mut composer.draft);
        composer.show_draft();
        Some(draft)
    }

    /// `draft` went out as request `req`; kept until its answer.
    pub fn sending(&mut self, session: SessionId, req: u64, draft: Draft) {
        self.entry(session).pending = Some((req, draft));
    }

    /// `draft` could not be sent at all: back into the composer, with `reason`.
    pub fn returned(&mut self, session: SessionId, draft: Draft, reason: &str) {
        let composer = self.entry(session);
        let current = std::mem::take(&mut composer.draft);
        composer.replace_draft(current.put_back(draft));
        composer.notice = Some(reason.to_string());
    }

    /// The service refused or failed send `req`: its draft comes back unchanged, with the reason
    /// (FR-005, SC-005). `false` when no composer waits on `req`.
    pub fn send_failed(&mut self, req: u64, reason: &str) -> bool {
        let Some((session, draft)) = self.take_pending(req) else {
            return false;
        };
        self.returned(session, draft, reason);
        true
    }

    /// The connection carrying sends `reqs` went: each comes back (FR-005, SC-005).
    pub fn connection_lost(&mut self, reqs: &[u64]) {
        for req in reqs {
            self.send_failed(*req, CONNECTION_LOST);
        }
    }

    /// The service took send `req`.
    pub fn send_acked(&mut self, req: u64) {
        self.take_pending(req);
    }

    fn take_pending(&mut self, req: u64) -> Option<(SessionId, Draft)> {
        self.composers
            .iter_mut()
            .find_map(|(session, composer)| match composer.pending.take() {
                Some((r, draft)) if r == req => Some((*session, draft)),
                other => {
                    composer.pending = other;
                    None
                }
            })
    }

    /// The service's `StructuredInput` for `session`. A queue edit whose entry left the queue
    /// ends with a notice.
    pub fn input_reported(&mut self, session: SessionId, input: InputState) {
        let composer = self.entry(session);
        let gone = composer
            .editing
            .as_ref()
            .is_some_and(|e| !input.queue.iter().any(|q| q.id == e.entry));
        if gone {
            composer.editing = None;
            composer.notice = Some(EDIT_GONE.to_string());
        }
        composer.remote = Some(input);
    }

    /// Put `notice` on `session`'s composer (a queue request's refusal).
    pub fn notice(&mut self, session: SessionId, notice: String) {
        self.entry(session).notice = Some(notice);
    }

    /// The stop control and the interrupt key (FR-012): only while busy.
    pub fn offers_stop(&self, session: SessionId) -> bool {
        is_busy(self.phase(session))
    }

    /// Stopping the session, offered once the interrupt is overdue (FR-015).
    pub fn offers_stop_session(&self, session: SessionId) -> bool {
        matches!(
            self.phase(session),
            TurnPhase::Busy {
                interrupt: InterruptAsk::Overdue,
                ..
            }
        )
    }

    /// The overdue interrupt's notice, while it is overdue (FR-015).
    pub fn overdue_notice(&self, session: SessionId) -> Option<&'static str> {
        self.offers_stop_session(session).then_some(OVERDUE)
    }

    /// **Send queued**: a held queue of a running session (FR-022).
    pub fn offers_send_queued(&self, session: SessionId) -> bool {
        self.phase(session) != TurnPhase::NotRunning
            && self
                .composer(session)
                .and_then(|c| c.remote.as_ref())
                .is_some_and(|r| r.hold)
    }

    fn queued(&self, session: SessionId, entry: QueueEntryId) -> Option<&QueuedSummary> {
        self.composer(session)?
            .remote
            .as_ref()?
            .queue
            .iter()
            .find(|q| q.id == entry)
    }

    /// Moving `entry` back into the composer: only while the draft is empty, so no draft is
    /// displaced (FR-020).
    pub fn can_take(&self, session: SessionId, entry: QueueEntryId) -> bool {
        self.queued(session, entry).is_some() && self.draft(session).is_none_or(Draft::is_empty)
    }

    /// A queued message came back (`Taken`): it becomes the draft. Typed meanwhile, the draft is
    /// kept after it, as a send's restore keeps it.
    pub fn taken(&mut self, session: SessionId, text: String, images: Vec<ImageAttachment>) {
        let composer = self.entry(session);
        composer.history = None;
        let current = std::mem::take(&mut composer.draft);
        composer.replace_draft(current.put_back(Draft::at_end(text, images)));
    }

    /// Start editing queued message `entry` in place; `false` when it is not queued.
    pub fn start_queue_edit(&mut self, session: SessionId, entry: QueueEntryId) -> bool {
        let Some(queued) = self.queued(session, entry).cloned() else {
            return false;
        };
        let keep = (0..queued.attachments.len() as u32).collect();
        self.entry(session).editing = Some(QueueEditDraft {
            entry,
            text: queued.text,
            attachments: queued.attachments,
            keep,
        });
        true
    }

    pub fn queue_edit_text(&mut self, session: SessionId, text: String) {
        if let Some(editing) = self.entry(session).editing.as_mut() {
            editing.text = text;
        }
    }

    /// Drop the edited message's picture shown at `index`.
    pub fn queue_edit_remove_attachment(&mut self, session: SessionId, index: usize) {
        if let Some(editing) = self.entry(session).editing.as_mut() {
            if index < editing.keep.len() {
                editing.keep.remove(index);
                editing.attachments.remove(index);
            }
        }
    }

    /// The `QueueEdit` to send: the entry, its new text and the pictures it keeps.
    pub fn queue_edit_request(
        &self,
        session: SessionId,
    ) -> Option<(QueueEntryId, String, Vec<u32>)> {
        let editing = self.composer(session)?.editing.as_ref()?;
        Some((editing.entry, editing.text.clone(), editing.keep.clone()))
    }

    /// The edit was saved, or left: it ends.
    pub fn queue_edit_done(&mut self, session: SessionId) {
        self.entry(session).editing = None;
    }

    /// Remove the draft's picture at `index` (US5 AS4).
    pub fn remove_attachment(&mut self, session: SessionId, index: usize) {
        let composer = self.entry(session);
        if index < composer.draft.images.len() {
            composer.draft.images.remove(index);
            composer.sync_previews();
        }
    }

    /// Files dropped on `session`'s pane (FR-026 to FR-029): each picture read is admitted in
    /// order, the other paths are inserted at the caret as a terminal drop of the same files
    /// inserts them, and every refusal is named in the notice.
    pub fn apply_drop(&mut self, session: SessionId, files: DroppedFiles) {
        let composer = self.entry(session);
        let mut refusals: Vec<String> = Vec::new();
        for (path, read) in files.plan.images.iter().zip(files.reads) {
            let name = file_name(path);
            let admitted = read
                .map_err(|why| AttachmentRefusal::Unreadable {
                    name: name.clone(),
                    why,
                })
                .and_then(|bytes| admit(&composer.draft.images, &name, bytes));
            match admitted {
                Ok(image) => composer.draft.images.push(image),
                Err(refusal) => refusals.push(refusal.to_string()),
            }
        }
        if !files.plan.paths.is_empty() {
            let target = files
                .sandbox
                .as_ref()
                .map_or(InsertTarget::Host, SandboxShare::target);
            let plan = plan_insertion(&files.plan.paths, files.shell, target);
            if let Some(text) = plan.text() {
                insert_at_caret(&mut composer.draft, &text);
            }
            refusals.extend(plan.refused.iter().map(|r| r.message()));
        }
        composer.show_draft();
        if !refusals.is_empty() {
            composer.notice = Some(refusals.join("\n"));
        }
    }
}

/// Show message `index` (0 the newest) of `transcript` as the draft, the caret on its first line
/// stepping back and on its last stepping forward, so the next press steps again.
fn recall(composer: &mut Composer, transcript: &Transcript, index: usize, forward: bool) {
    let text = transcript
        .user_messages()
        .rev()
        .nth(index)
        .unwrap_or_default()
        .to_string();
    let caret = if forward {
        end_of(&text)
    } else {
        (0, text.split('\n').next().unwrap_or("").len())
    };
    composer.replace_draft(Draft {
        text,
        caret,
        images: Vec::new(),
    });
}

fn insert_at_caret(draft: &mut Draft, text: &str) {
    let at = offset_of(&draft.text, draft.caret);
    draft.text.insert_str(at, text);
    draft.caret = end_of(&draft.text[..at + text.len()]);
}

/// The name a dropped file is attached under: its file name, no directory.
fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// The composer's commands (#943): the palette and the picker over the draft, what a send means,
/// and the switch offer a terminal-only command or an authentication failure opens. Here because
/// each writes the composer's own state (`composers`, its `palette` and `switch_offer`).
impl crate::app::State {
    /// The composer's draft changed: the palette follows it.
    pub fn composer_edited(&mut self, session: SessionId) {
        let (text, caret) = self
            .composers
            .draft(session)
            .map(|d| (d.text.clone(), d.caret))
            .unwrap_or_default();
        let transcript = self.structured_transcript.transcript(session);
        self.composers
            .palette
            .draft_changed(session, &text, caret, || {
                transcript.map_or_else(
                    micold_core::structured::commands::CommandList::fallback,
                    |t| t.command_list(),
                )
            });
    }

    /// A key [`palette_key`] gave to the open palette.
    pub fn palette_key(&mut self, session: SessionId, key: PaletteKey) {
        match key {
            PaletteKey::Up => self.composers.palette.step(session, Direction::Prev),
            PaletteKey::Down => self.composers.palette.step(session, Direction::Next),
            PaletteKey::Accept => self.palette_picked(session, None),
            PaletteKey::Close => {
                let text = self
                    .composers
                    .draft(session)
                    .map(|d| d.text.clone())
                    .unwrap_or_default();
                self.composers.palette.dismiss(session, &text);
            }
        }
    }

    /// A click on the palette's row `row`.
    pub fn palette_clicked(&mut self, session: SessionId, row: usize) {
        self.palette_picked(session, Some(row));
    }

    fn palette_picked(&mut self, session: SessionId, row: Option<usize>) {
        match self.composers.palette.pick(session, row) {
            Some(Picked::Fill { text, caret }) => {
                self.composers.replace_text(session, text, caret);
            }
            Some(Picked::TerminalOnly { command, reason }) => {
                // The draft stays as typed; it must not reopen the palette beside the offer.
                let text = self.composers.draft(session).map(|d| d.text.clone());
                self.composers
                    .palette
                    .dismiss(session, &text.unwrap_or_default());
                self.offer_switch(session, SessionKind::Terminal, Some(command), reason);
            }
            None => {}
        }
    }

    /// Open `session`'s model picker (FR-014, FR-018).
    pub fn open_model_picker(&mut self, session: SessionId) {
        let current = self.model_choice(session);
        let reported = self
            .structured_transcript
            .transcript(session)
            .map(|t| t.reported_models().to_vec())
            .unwrap_or_default();
        self.composers
            .palette
            .open_picker(session, &reported, &current);
    }

    /// The arrows in `session`'s picker.
    pub fn model_picker_moved(&mut self, session: SessionId, direction: Direction) {
        self.composers.palette.picker_step(session, direction);
    }

    /// The name typed into `session`'s picker.
    pub fn model_picker_typed(&mut self, session: SessionId, text: String) {
        self.composers.palette.picker_typed(session, text);
    }

    /// Choose item `item` of `session`'s picker, else its typed name: the choice to send, or
    /// `None` (nothing open, or the typed name refused, said in the composer's notice).
    pub fn model_picked(&mut self, session: SessionId, item: Option<usize>) -> Option<ModelChoice> {
        match self.composers.palette.picker_chose(session, item)? {
            Ok(choice) => Some(choice),
            Err(refusal) => {
                self.composers.notice(session, refusal.to_string());
                None
            }
        }
    }

    /// What sending `session`'s draft does (FR-010, FR-018): a message for the composer to send,
    /// or handled here (the picker opened, the terminal offered, a refusal said), or a model to
    /// ask the service for, its draft left for the shell to clear once the request went out.
    /// `/model` alone leaves the composer; a terminal-only command stays until the switch is done.
    pub fn send_intent(&mut self, session: SessionId) -> SendIntent {
        let Some(text) = self.composers.draft(session).map(|d| d.text.clone()) else {
            return SendIntent::Message;
        };
        match classify(&text, &self.command_list(session)) {
            Classified::Message => SendIntent::Message,
            Classified::Picker => {
                self.composers.replace_text(session, String::new(), (0, 0));
                self.open_model_picker(session);
                SendIntent::Nothing
            }
            Classified::Model(Ok(choice)) => SendIntent::SetModel(choice),
            Classified::Model(Err(refusal)) => {
                self.composers.notice(session, refusal.to_string());
                SendIntent::Nothing
            }
            Classified::TerminalOnly { command, reason } => {
                self.offer_switch(session, SessionKind::Terminal, Some(command), reason);
                SendIntent::Nothing
            }
        }
    }

    /// Offer to switch `session` to `to` for `command`, saying `reason` (FR-010).
    pub fn offer_switch(
        &mut self,
        session: SessionId,
        to: SessionKind,
        command: Option<String>,
        reason: String,
    ) {
        self.composers.switch_offer = Some(SwitchOffer {
            session,
            to,
            command,
            reason,
            confirm: None,
        });
    }

    /// The offer was dismissed: nothing changes, the draft stays (US4 scenario 1).
    pub fn switch_dismissed(&mut self) {
        self.composers.switch_offer = None;
    }

    /// The offer was confirmed: the request to send, confirmed when it was the confirmation
    /// step. The draft stays until the service took the switch ([`Self::switch_completed`]).
    pub fn switch_confirmed(&mut self) -> Option<SwitchRequest> {
        let offer = self.composers.switch_offer.take()?;
        Some(SwitchRequest {
            session: offer.session,
            to: offer.to,
            command: offer.command,
            confirmed: offer.confirm.is_some(),
        })
    }

    /// The service answered `request` with `NeedsConfirmation`: ask the user, saying `message`
    /// (FR-021).
    pub fn switch_needs_confirmation(&mut self, request: SwitchRequest, message: String) {
        self.composers.switch_offer = Some(SwitchOffer {
            session: request.session,
            to: request.to,
            command: request.command,
            reason: message.clone(),
            confirm: Some(message),
        });
    }

    /// The service took `request`: a draft that is its command, as sent, is used up; any other
    /// draft stays (US4 scenario 2, FR-024).
    pub fn switch_completed(&mut self, request: &SwitchRequest) {
        let Some(command) = &request.command else {
            return;
        };
        let sent = self
            .composers
            .draft(request.session)
            .and_then(|d| micold_core::structured::commands::parse_invocation(&d.text))
            .is_some_and(|i| &i.name == command);
        if sent {
            self.composers
                .replace_text(request.session, String::new(), (0, 0));
        }
    }

    /// The `/login` offer was taken: offer the terminal for it (FR-022a).
    pub fn login_offered(&mut self, session: SessionId) {
        self.offer_switch(
            session,
            SessionKind::Terminal,
            Some(LOGIN_COMMAND.to_string()),
            LOGIN_REASON.to_string(),
        );
    }
}

/// The confirmation a switch asks for while work is in progress (#943 FR-021), as a floating
/// surface: open while the switch offer of `0` is the service's confirmation step. Escape and a
/// scrim click dismiss it, which changes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfirmSwitchKindDialog(pub SessionId);

impl crate::overlay::FloatingSurface for ConfirmSwitchKindDialog {
    fn id(&self) -> crate::overlay::SurfaceId {
        crate::overlay::SurfaceId::new("confirm_switch_kind")
    }

    fn layer(&self) -> micold_core::overlay::Layer {
        micold_core::overlay::Layer::Dialog
    }

    fn dismissal(&self) -> crate::overlay::DismissalRules {
        crate::overlay::DismissalRules::for_layer(micold_core::overlay::Layer::Dialog)
            .cancelled_by(crate::app::Message::Composer(Msg::SwitchDismissed(self.0)))
    }
}

impl crate::overlay::registry::Registered for ConfirmSwitchKindDialog {
    fn open_in(state: &crate::app::State) -> Option<Self> {
        state
            .composers
            .switch_offer
            .as_ref()
            .filter(|offer| offer.confirm.is_some())
            .map(|offer| ConfirmSwitchKindDialog(offer.session))
    }
}
