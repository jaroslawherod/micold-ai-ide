//! The effort picker beside a structured Claude Code session's model switcher, render-free
//! (#1554 FR-001–FR-003, FR-012; research R7).
//!
//! The trigger reads `Effort: Default` or `Effort: <level>`, the session's choice as the service
//! last reported it on the catalog. Its list is Default, then the levels the session model takes
//! (core `offered_levels`: the CLI's own when it lists some, else all five), the current choice
//! marked in words. Openness and the keyboard highlight are held here, not by the widget, so
//! `/effort` (M5) can open it too. A pick answers the choice to send; the shell sends
//! `SetSessionEffort` and the catalog brings the new choice back, as a model pick does.

use std::collections::HashMap;

use micold_core::session::{AiCli, SessionId, SessionKind};
use micold_core::structured::effort::{
    availability, effort_support, offered_levels, EffortAvailability, EffortChoice, EffortSupport,
};
use micold_core::typeahead::{move_highlight, Direction};

/// The words that mark the current choice's row (FR-002).
pub const CURRENT: &str = "current";

/// What the picker and the trigger's tooltip say when the session model does not support effort
/// (FR-003).
pub const NO_EFFORT_NOTE: &str =
    "This model does not support reasoning effort; Claude Code's default applies";

/// The trigger's text for `choice` (FR-001).
pub fn effort_label(choice: &EffortChoice) -> String {
    format!("Effort: {choice}")
}

/// What the view says when `choice` does not apply to a model with `support` (FR-003): `None`
/// when it applies, or support is unknown and the choice is sent as is.
pub fn availability_note(choice: &EffortChoice, support: Option<&EffortSupport>) -> Option<String> {
    match availability(choice, support) {
        EffortAvailability::Applies => None,
        EffortAvailability::ModelHasNoEffort => Some(NO_EFFORT_NOTE.to_owned()),
        EffortAvailability::LevelNotForModel(level) => Some(format!(
            "{level} is not available for this model; Claude Code's default applies"
        )),
    }
}

/// One row of the picker: a choice, and whether it is the session's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffortRow {
    pub choice: EffortChoice,
    pub current: bool,
}

impl EffortRow {
    /// What the row reads: `Default` or the level.
    pub fn label(&self) -> String {
        self.choice.to_string()
    }

    /// The words beside the current choice's row.
    pub fn mark(&self) -> Option<&'static str> {
        self.current.then_some(CURRENT)
    }
}

/// One session's open picker: its rows, fixed when it opened, and the keyboard's row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffortPicker {
    rows: Vec<EffortRow>,
    highlight: Option<usize>,
}

impl EffortPicker {
    /// Default, then the offered levels, `current` marked and highlighted.
    fn new(support: Option<&EffortSupport>, current: &EffortChoice) -> Self {
        let rows: Vec<EffortRow> = std::iter::once(EffortChoice::Default)
            .chain(offered_levels(support).into_iter().map(EffortChoice::Level))
            .map(|choice| EffortRow {
                current: &choice == current,
                choice,
            })
            .collect();
        let highlight = rows.iter().position(|r| r.current).or(Some(0));
        Self { rows, highlight }
    }

    pub fn rows(&self) -> &[EffortRow] {
        &self.rows
    }

    /// The row the keyboard is on.
    pub fn highlight(&self) -> Option<usize> {
        self.highlight
    }
}

/// The effort pickers open over the window's composers, one per session at most: the window's
/// `effort_picker`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    open: HashMap<SessionId, EffortPicker>,
}

impl State {
    /// `session`'s open picker.
    pub fn picker(&self, session: SessionId) -> Option<&EffortPicker> {
        self.open.get(&session)
    }

    fn open(
        &mut self,
        session: SessionId,
        support: Option<&EffortSupport>,
        current: &EffortChoice,
    ) {
        self.open
            .insert(session, EffortPicker::new(support, current));
    }

    /// Close `session`'s picker, picking nothing.
    pub fn close(&mut self, session: SessionId) {
        self.open.remove(&session);
    }

    /// The arrows in `session`'s picker.
    pub fn step(&mut self, session: SessionId, direction: Direction) {
        if let Some(picker) = self.open.get_mut(&session) {
            picker.highlight = move_highlight(picker.highlight, direction, picker.rows.len());
        }
    }

    /// Take row `row` (the highlighted one when `None`), closing the picker: the row's choice, or
    /// `None` when nothing is open or the row is not there.
    fn take(&mut self, session: SessionId, row: Option<usize>) -> Option<EffortRow> {
        let picker = self.open.remove(&session)?;
        let row = row.or(picker.highlight)?;
        picker.rows.into_iter().nth(row)
    }

    /// Forget `session` (it is gone).
    pub fn forget(&mut self, session: SessionId) {
        self.open.remove(&session);
    }
}

impl crate::app::State {
    /// Whether `session` offers an effort picker: a structured Claude Code session (FR-012).
    pub fn offers_effort(&self, session: SessionId) -> bool {
        self.workspace.find_session(session).is_some_and(|(_, s)| {
            s.provider == AiCli::ClaudeCode && s.kind == SessionKind::Structured
        })
    }

    /// The effort `session`'s user chose (FR-010), as the service last reported it.
    pub fn effort_choice(&self, session: SessionId) -> EffortChoice {
        self.workspace
            .find_session(session)
            .map(|(_, s)| s.effort.clone())
            .unwrap_or_default()
    }

    /// What the CLI reported about the effort of `session`'s model, when it did.
    fn effort_support_of(&self, session: SessionId) -> Option<EffortSupport> {
        let transcript = self.structured_transcript.transcript(session)?;
        effort_support(transcript.reported_models(), &self.model_choice(session)).cloned()
    }

    /// `session`'s effort trigger text (FR-001).
    pub fn effort_header(&self, session: SessionId) -> String {
        effort_label(&self.effort_choice(session))
    }

    /// What the view says when `session`'s choice does not apply to its model (FR-003).
    pub fn effort_note(&self, session: SessionId) -> Option<String> {
        availability_note(
            &self.effort_choice(session),
            self.effort_support_of(session).as_ref(),
        )
    }

    /// Open `session`'s effort picker (FR-002), when it offers one.
    pub fn open_effort_picker(&mut self, session: SessionId) {
        if !self.offers_effort(session) {
            return;
        }
        let support = self.effort_support_of(session);
        let current = self.effort_choice(session);
        self.effort_picker.open(session, support.as_ref(), &current);
    }

    /// A press on `session`'s trigger: open its picker, or close it when open.
    pub fn effort_picker_toggled(&mut self, session: SessionId) {
        if self.effort_picker.picker(session).is_some() {
            self.effort_picker.close(session);
        } else {
            self.open_effort_picker(session);
        }
    }

    /// The arrows in `session`'s effort picker.
    pub fn effort_picker_moved(&mut self, session: SessionId, direction: Direction) {
        self.effort_picker.step(session, direction);
    }

    /// Take row `row` of `session`'s effort picker (the highlighted one when `None`), closing it:
    /// the choice to ask the service for, or `None` when nothing is open or the row already is
    /// the session's choice.
    pub fn effort_picked(
        &mut self,
        session: SessionId,
        row: Option<usize>,
    ) -> Option<EffortChoice> {
        let row = self.effort_picker.take(session, row)?;
        (row.choice != self.effort_choice(session)).then_some(row.choice)
    }
}
