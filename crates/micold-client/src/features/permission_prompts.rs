//! The structured sessions' permission prompts this window holds (#942, data-model *Client
//! state*): each session's pending requests with what the user typed and chose for them, how the
//! ones that ended ended, and a mode change on its way or awaiting its Bypass confirmation.
//!
//! Render-free: it decides when an answer can be submitted and builds the [`PromptChoice`] it
//! sends (FR-006–FR-008), says whether a session waits for the user (FR-009) and where in the
//! transcript each request goes. `ui/transcript/permission.rs` draws it (M7);
//! `shell/permission_prompts.rs` sends what it builds and `shell/daemon_sync.rs` routes the
//! daemon's pushes and replies back to it.

use micold_core::protocol::messages::{
    PermissionRequest, PromptChoice, PromptId, QuestionAnswer, Resolution,
};
use micold_core::session::{PermissionMode, SessionId};
use micold_core::structured::permission::request::RequestKind;
use micold_core::structured::transcript::{EntryKey, Transcript};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// What the user answers a request with: the buttons of the three request views. The draft
/// supplies the typed parts (the deny message, the note, the answers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// A tool request's **Allow**.
    Allow,
    /// A tool request's **Always allow**, offered when the request names a rule.
    AlwaysAllow,
    /// A tool request's **Deny**, with the typed message.
    Deny,
    /// A plan approval's **Approve and accept edits** (`true`) or **Approve** (`false`).
    ApprovePlan { accept_edits: bool },
    /// A plan approval's **Keep planning**, with the typed note.
    KeepPlanning,
    /// A question request's **Submit**, with the chosen answers.
    Answer,
    /// A question request's **Decline**.
    Decline,
}

/// What the request views and the mode control say. Shape B: an answer, the whole input and a
/// mode change are requests only the shell can send, so the entry is `shell/permission_prompts.rs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// The deny message field changed.
    DenyMessageEdited {
        session: SessionId,
        id: PromptId,
        text: String,
    },
    /// The keep-planning note field changed.
    NoteEdited {
        session: SessionId,
        id: PromptId,
        text: String,
    },
    /// An option of question `question` was pressed.
    OptionToggled {
        session: SessionId,
        id: PromptId,
        question: usize,
        option: usize,
    },
    /// The Other row of question `question` was pressed.
    OtherChosen {
        session: SessionId,
        id: PromptId,
        question: usize,
    },
    /// The Other field of question `question` changed.
    OtherEdited {
        session: SessionId,
        id: PromptId,
        question: usize,
        text: String,
    },
    /// A request's button was pressed.
    Submitted {
        session: SessionId,
        id: PromptId,
        action: Action,
    },
    /// **Show all** of a cut input preview (research R10).
    ShowAllRequested { session: SessionId, id: PromptId },
    /// The mode control chose `mode` for a running or stopped session (FR-017).
    ModeRequested {
        session: SessionId,
        mode: PermissionMode,
    },
    /// The Bypass confirmation was accepted.
    ModeConfirmed,
    /// The Bypass confirmation was dismissed.
    ModeCancelled,
}

/// Where a request goes in its session's transcript (research R11).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Placement {
    /// Directly under the tool card it belongs to.
    UnderCard(EntryKey),
    /// After the last entry: the transcript does not hold its card (spec edge case).
    AtEnd,
}

/// One question's answer as the user is making it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuestionDraft {
    /// The chosen options, by position: at most one on a single-choice question.
    pub chosen: BTreeSet<usize>,
    /// The Other field's text.
    pub other: String,
    /// Whether Other is chosen; on a single-choice question it excludes an option.
    pub other_chosen: bool,
}

impl QuestionDraft {
    /// The free answer, when Other is chosen and not blank.
    fn other_answer(&self) -> Option<String> {
        (self.other_chosen && !self.other.trim().is_empty()).then(|| self.other.clone())
    }

    fn answered(&self) -> bool {
        !self.chosen.is_empty() || self.other_answer().is_some()
    }
}

/// What the user typed and chose for one request.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Draft {
    /// A tool request's deny message.
    pub deny_message: String,
    /// A plan approval's keep-planning note.
    pub note: String,
    /// A question request's answers, one per question in order.
    pub answers: Vec<QuestionDraft>,
}

/// A pending request as this window shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shown {
    pub request: PermissionRequest,
    pub draft: Draft,
    /// An answer is on its way: no other is sent until the daemon refuses it.
    pub sending: bool,
    /// Why the daemon refused the last answer or the whole input (FR-012).
    pub error: Option<String>,
    /// The whole input, once **Show all** fetched it (research R10).
    pub full_input: Option<String>,
}

impl Shown {
    fn new(request: PermissionRequest) -> Self {
        let questions = match &request.kind {
            RequestKind::Question { questions } => questions.len(),
            RequestKind::Tool | RequestKind::PlanApproval { .. } => 0,
        };
        Self {
            request,
            draft: Draft {
                answers: vec![QuestionDraft::default(); questions],
                ..Draft::default()
            },
            sending: false,
            error: None,
            full_input: None,
        }
    }

    /// Whether the input preview is cut: the whole input is longer than it.
    pub fn input_cut(&self) -> bool {
        self.request.input_bytes > self.request.input_preview.len() as u64
    }

    /// Whether every question has a chosen option or a chosen, non-blank Other (FR-008); true
    /// for the other kinds.
    fn complete(&self) -> bool {
        self.draft.answers.iter().all(QuestionDraft::answered)
    }

    fn choice(&self, action: Action) -> Option<PromptChoice> {
        if self.sending {
            return None;
        }
        let typed = |text: &str| (!text.trim().is_empty()).then(|| text.to_string());
        match (&self.request.kind, action) {
            (RequestKind::Tool, Action::Allow) => Some(PromptChoice::Allow),
            (RequestKind::Tool, Action::AlwaysAllow) => self
                .request
                .always_allow
                .is_some()
                .then_some(PromptChoice::AlwaysAllow),
            (RequestKind::Tool, Action::Deny) => Some(PromptChoice::Deny {
                message: typed(&self.draft.deny_message),
            }),
            (RequestKind::PlanApproval { .. }, Action::ApprovePlan { accept_edits }) => {
                Some(PromptChoice::ApprovePlan { accept_edits })
            }
            (RequestKind::PlanApproval { .. }, Action::KeepPlanning) => {
                Some(PromptChoice::KeepPlanning {
                    note: typed(&self.draft.note),
                })
            }
            (RequestKind::Question { questions }, Action::Answer) if self.complete() => {
                Some(PromptChoice::Answer {
                    answers: questions
                        .iter()
                        .zip(&self.draft.answers)
                        .map(|(question, draft)| QuestionAnswer {
                            labels: draft
                                .chosen
                                .iter()
                                .filter_map(|&at| question.options.get(at))
                                .map(|option| option.label.clone())
                                .collect(),
                            other: draft.other_answer(),
                        })
                        .collect(),
                })
            }
            (RequestKind::Question { .. }, Action::Decline) => Some(PromptChoice::DeclineQuestion),
            _ => None,
        }
    }

    /// Question `question`'s draft, and whether it allows several options; `None` when the
    /// request has no such question.
    fn question(&mut self, question: usize) -> Option<(&mut QuestionDraft, bool, usize)> {
        let RequestKind::Question { questions } = &self.request.kind else {
            return None;
        };
        let asked = questions.get(question)?;
        let (multi, options) = (asked.multi_select, asked.options.len());
        Some((self.draft.answers.get_mut(question)?, multi, options))
    }
}

/// One session's prompts in this window.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct SessionPrompts {
    pending: BTreeMap<PromptId, Shown>,
    /// How each request that ended here ended, by tool use id: its card's label. This window run
    /// only.
    resolved: HashMap<String, Resolution>,
    /// The mode a `SetPermissionMode` sent is asking for, until its reply.
    mode_change: Option<PermissionMode>,
    /// Why the last mode change was refused.
    notice: Option<String>,
}

impl SessionPrompts {
    fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// The structured sessions' permission prompts this window holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    sessions: HashMap<SessionId, SessionPrompts>,
    /// The session whose mode change to Bypass awaits its confirmation (FR-017). Its presence is
    /// the confirmation dialog being shown.
    mode_confirmation: Option<SessionId>,
}

impl State {
    fn shown_mut(&mut self, session: SessionId, id: PromptId) -> Option<&mut Shown> {
        self.sessions.get_mut(&session)?.pending.get_mut(&id)
    }

    /// Drop `session`'s entry once nothing is left in it.
    fn tidy(&mut self, session: SessionId) {
        if self
            .sessions
            .get(&session)
            .is_some_and(SessionPrompts::is_empty)
        {
            self.sessions.remove(&session);
        }
    }

    /// A `PermissionRequested` arrived. One for a request already held (a handshake replay) is
    /// ignored, so its draft survives.
    pub fn requested(&mut self, request: PermissionRequest) {
        self.sessions
            .entry(request.session)
            .or_default()
            .pending
            .entry(request.id)
            .or_insert_with(|| Shown::new(request));
    }

    /// A `PermissionResolved` arrived: the request closes, and its card keeps how it ended.
    pub fn resolved(
        &mut self,
        session: SessionId,
        id: PromptId,
        tool_use_id: Option<String>,
        resolution: Resolution,
    ) {
        let prompts = self.sessions.entry(session).or_default();
        prompts.pending.remove(&id);
        if let Some(tool_use_id) = tool_use_id {
            prompts.resolved.insert(tool_use_id, resolution);
        }
        self.tidy(session);
    }

    /// The request `id` of `session`, while it is pending.
    pub fn shown(&self, session: SessionId, id: PromptId) -> Option<&Shown> {
        self.sessions.get(&session)?.pending.get(&id)
    }

    /// `session`'s pending requests, oldest first.
    pub fn pending(&self, session: SessionId) -> impl Iterator<Item = &Shown> {
        self.sessions
            .get(&session)
            .into_iter()
            .flat_map(|s| s.pending.values())
    }

    /// How the request of the tool call `tool_use_id` ended, when it ended in this window run.
    pub fn resolution(&self, session: SessionId, tool_use_id: &str) -> Option<Resolution> {
        self.sessions
            .get(&session)?
            .resolved
            .get(tool_use_id)
            .copied()
    }

    /// Whether `session` waits for the user: a request of it is pending (FR-009).
    pub fn waiting(&self, session: SessionId) -> bool {
        self.sessions
            .get(&session)
            .is_some_and(|s| !s.pending.is_empty())
    }

    /// Where each of `session`'s pending requests goes, oldest first: under its tool card when
    /// `transcript` holds it, else after the last entry.
    pub fn placement(
        &self,
        session: SessionId,
        transcript: Option<&Transcript>,
    ) -> Vec<(Placement, PromptId)> {
        self.pending(session)
            .map(|shown| {
                let card = shown
                    .request
                    .tool_use_id
                    .as_ref()
                    .map(|id| EntryKey::Tool(id.clone()))
                    .filter(|key| transcript.is_some_and(|t| t.entry(key).is_some()));
                (
                    card.map_or(Placement::AtEnd, Placement::UnderCard),
                    shown.request.id,
                )
            })
            .collect()
    }

    /// The deny message field changed.
    pub fn edit_deny_message(&mut self, session: SessionId, id: PromptId, text: String) {
        if let Some(shown) = self.shown_mut(session, id) {
            shown.draft.deny_message = text;
        }
    }

    /// The keep-planning note field changed.
    pub fn edit_note(&mut self, session: SessionId, id: PromptId, text: String) {
        if let Some(shown) = self.shown_mut(session, id) {
            shown.draft.note = text;
        }
    }

    /// Option `option` of question `question` was pressed: on a single-choice question it becomes
    /// the one answer, on a multi-select it is chosen or unchosen.
    pub fn toggle_option(
        &mut self,
        session: SessionId,
        id: PromptId,
        question: usize,
        option: usize,
    ) {
        let Some(shown) = self.shown_mut(session, id) else {
            return;
        };
        let Some((draft, multi, options)) = shown.question(question) else {
            return;
        };
        if option >= options {
            return;
        }
        if multi {
            if !draft.chosen.remove(&option) {
                draft.chosen.insert(option);
            }
        } else {
            draft.chosen = BTreeSet::from([option]);
            draft.other_chosen = false;
        }
    }

    /// The Other row of question `question` was pressed: on a single-choice question it becomes
    /// the one answer, on a multi-select it is chosen or unchosen.
    pub fn choose_other(&mut self, session: SessionId, id: PromptId, question: usize) {
        let Some(shown) = self.shown_mut(session, id) else {
            return;
        };
        let Some((draft, multi, _)) = shown.question(question) else {
            return;
        };
        if multi {
            draft.other_chosen = !draft.other_chosen;
        } else {
            draft.chosen.clear();
            draft.other_chosen = true;
        }
    }

    /// The Other field of question `question` changed.
    pub fn edit_other(&mut self, session: SessionId, id: PromptId, question: usize, text: String) {
        if let Some((draft, _, _)) = self
            .shown_mut(session, id)
            .and_then(|shown| shown.question(question))
        {
            draft.other = text;
        }
    }

    /// Whether the request can be answered now: pending, no answer on its way, and for a
    /// question request every question answered (FR-008).
    pub fn can_submit(&self, session: SessionId, id: PromptId) -> bool {
        self.shown(session, id)
            .is_some_and(|shown| !shown.sending && shown.complete())
    }

    /// The answer `action` sends, built from the draft; `None` when the request is not pending,
    /// an answer is on its way, the action does not fit the request's kind, Always allow names
    /// no rule, or a question request is not complete.
    pub fn choice(&self, session: SessionId, id: PromptId, action: Action) -> Option<PromptChoice> {
        self.shown(session, id)?.choice(action)
    }

    /// The answer was sent.
    pub fn sending(&mut self, session: SessionId, id: PromptId) {
        if let Some(shown) = self.shown_mut(session, id) {
            shown.sending = true;
            shown.error = None;
        }
    }

    /// The daemon refused the answer (`OperationError`, FR-012): the request can be answered
    /// again, and shows why.
    pub fn decision_failed(&mut self, session: SessionId, id: PromptId, message: String) {
        if let Some(shown) = self.shown_mut(session, id) {
            shown.sending = false;
            shown.error = Some(message);
        }
    }

    /// The daemon refused the whole input (`OperationError`): the request shows why, and an
    /// answer on its way stays on its way.
    pub fn input_failed(&mut self, session: SessionId, id: PromptId, message: String) {
        if let Some(shown) = self.shown_mut(session, id) {
            shown.error = Some(message);
        }
    }

    /// The whole input arrived (`PermissionInputText`).
    pub fn input_text(&mut self, session: SessionId, id: PromptId, text: String) {
        if let Some(shown) = self.shown_mut(session, id) {
            shown.full_input = Some(text);
        }
    }

    /// The connection to `session`'s daemon was lost: its requests are dropped, and the next
    /// handshake replays those still pending (contracts/wire.md). The card labels stay.
    pub fn connection_lost(&mut self, session: SessionId) {
        if let Some(prompts) = self.sessions.get_mut(&session) {
            prompts.pending.clear();
            prompts.mode_change = None;
        }
        self.tidy(session);
    }

    /// The sessions this window holds prompt state for.
    pub fn sessions(&self) -> Vec<SessionId> {
        self.sessions.keys().copied().collect()
    }

    /// The mode control chose `mode` for `session`: the mode to send now, or `None` when it is
    /// Bypass, which waits for its confirmation (FR-017).
    pub fn request_mode(
        &mut self,
        session: SessionId,
        mode: PermissionMode,
    ) -> Option<PermissionMode> {
        if mode == PermissionMode::Bypass {
            self.mode_confirmation = Some(session);
            return None;
        }
        Some(mode)
    }

    /// The session whose Bypass confirmation is shown.
    pub fn mode_confirmation(&self) -> Option<SessionId> {
        self.mode_confirmation
    }

    /// The Bypass confirmation was accepted: the change it sends, once.
    pub fn confirm_mode(&mut self) -> Option<(SessionId, PermissionMode)> {
        self.mode_confirmation
            .take()
            .map(|session| (session, PermissionMode::Bypass))
    }

    /// The Bypass confirmation was dismissed: nothing is sent.
    pub fn cancel_mode(&mut self) {
        self.mode_confirmation = None;
    }

    /// A `SetPermissionMode` for `mode` was sent.
    pub fn mode_sending(&mut self, session: SessionId, mode: PermissionMode) {
        let prompts = self.sessions.entry(session).or_default();
        prompts.mode_change = Some(mode);
        prompts.notice = None;
    }

    /// The mode change was applied (`OperationOk`); the session's summary carries the new mode.
    pub fn mode_applied(&mut self, session: SessionId) {
        if let Some(prompts) = self.sessions.get_mut(&session) {
            prompts.mode_change = None;
        }
        self.tidy(session);
    }

    /// The mode change was refused (`OperationError`): the reason is the session's notice.
    pub fn mode_failed(&mut self, session: SessionId, message: String) {
        let prompts = self.sessions.entry(session).or_default();
        prompts.mode_change = None;
        prompts.notice = Some(message);
    }

    /// The mode a change on its way asks for.
    pub fn mode_change(&self, session: SessionId) -> Option<PermissionMode> {
        self.sessions.get(&session)?.mode_change
    }

    /// Why the last mode change of `session` was refused.
    pub fn notice(&self, session: SessionId) -> Option<&str> {
        self.sessions.get(&session)?.notice.as_deref()
    }

    /// Forget `session`'s prompts (the session was closed or removed).
    pub fn forget(&mut self, session: SessionId) {
        self.sessions.remove(&session);
        if self.mode_confirmation == Some(session) {
            self.mode_confirmation = None;
        }
    }
}
