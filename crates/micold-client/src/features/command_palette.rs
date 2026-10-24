//! The command palette and the model picker of a structured session's composer, render-free
//! (#943 FR-007–FR-010, FR-013, FR-014, FR-018; data-model *Client*; research R10, R11).
//!
//! The palette is open while the draft is `/` and a name with the caret in it (core
//! `palette_query`); it lists the session's command list narrowed by that name (core `filter`)
//! with the first match selected. Its keys are decided by [`palette_key`], which the shell asks
//! before #941's `composer_key` and which answers `None` while the palette is closed. Accepting a
//! row fills `/<name> `; a terminal-only row instead offers **Open in terminal**
//! (`features::session::SwitchOffer`). Sending a draft goes through [`send_intent`]
//! (`crate::app::State::send_intent`): `/model` opens the picker, `/model <name>` picks it, a
//! terminal-only command is offered the terminal, anything else is a message.

use std::collections::HashMap;
use std::ops::Range;

use micold_core::session::SessionId;
use micold_core::structured::commands::palette::{accept, filter, palette_query, PaletteSelection};
use micold_core::structured::commands::{
    classify_draft, Command, CommandKind, CommandList, DraftCommand, ListSource, CLI_MARKED_WHY,
};
use micold_core::structured::model::{ModelChoice, ModelName, ModelRefusal, MODEL_SHORTCUTS};
use micold_core::structured::ReportedModel;
use micold_core::typeahead::{move_highlight, Direction};

use crate::keymap::{Key, KeyInput, NamedKey};

/// What the palette says when nothing matches the typed name (US2 scenario 5).
pub const NO_MATCH: &str = "No command matches. Enter sends the text as typed.";

/// What the palette says over the fallback list (FR-009, US2 scenario 8).
pub const FALLBACK_NOTE: &str =
    "The commands Claude Code always has. The session's own commands appear once it starts.";

/// The words a terminal-only row carries (FR-010, US2 scenario 6).
pub const OPENS_IN_TERMINAL: &str = "opens in terminal";

/// The model picker's name for the CLI's default model (FR-014).
pub const DEFAULT_MODEL: &str = "Default";

/// The header's words before the CLI has reported a model (FR-013).
pub const MODEL_NOT_REPORTED: &str = "Model not reported yet";

/// An open palette: the list it was opened over, the name typed so far and its matches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteState {
    list: CommandList,
    query: String,
    rows: Vec<usize>,
    selection: PaletteSelection,
}

impl PaletteState {
    fn new(list: CommandList, query: &str) -> Self {
        let rows = filter(&list, query);
        Self {
            selection: PaletteSelection::new(rows.len()),
            list,
            query: query.to_string(),
            rows,
        }
    }

    fn narrow(&mut self, query: &str) {
        if self.query != query {
            self.query = query.to_string();
            self.rows = filter(&self.list, query);
            self.selection.reset(self.rows.len());
        }
    }

    /// The name typed after the `/`, up to the caret.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// The matching commands, in FR-008's order.
    pub fn rows(&self) -> impl ExactSizeIterator<Item = &Command> {
        self.rows.iter().map(|&i| &self.list.commands()[i])
    }

    /// The selected row's position in [`Self::rows`].
    pub fn highlight(&self) -> Option<usize> {
        self.selection.highlight()
    }

    fn row(&self, at: usize) -> Option<&Command> {
        self.rows.get(at).map(|&i| &self.list.commands()[i])
    }

    /// [`NO_MATCH`] when no command matches (US2 scenario 5).
    pub fn no_match_note(&self) -> Option<&'static str> {
        self.rows.is_empty().then_some(NO_MATCH)
    }

    /// [`FALLBACK_NOTE`] over the fallback list (US2 scenario 8).
    pub fn source_note(&self) -> Option<&'static str> {
        (self.list.source() == ListSource::Fallback).then_some(FALLBACK_NOTE)
    }

    /// What the window draws, in order: with no name typed, each group under its header
    /// (FR-007); with a name, its matches in FR-008's order, which mixes the groups, under none.
    pub fn lines(&self) -> Vec<PaletteLine> {
        let mut lines = Vec::with_capacity(self.rows.len() + 4);
        let mut group = None;
        for command in self.rows() {
            if self.query.is_empty() && group != Some(command.kind) {
                group = Some(command.kind);
                lines.push(PaletteLine::Group(group_title(command.kind)));
            }
            lines.push(PaletteLine::Command {
                label: format!("/{}", command.name),
                matched: row_match(&command.name, &self.query).map(|m| m.start + 1..m.end + 1),
                details: row_details(command),
            });
        }
        lines
    }
}

/// One line of the drawn palette.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteLine {
    /// A group's header.
    Group(&'static str),
    /// A command: `/name`, the typed name's place in it, and what [`row_details`] says.
    Command {
        label: String,
        matched: Option<Range<usize>>,
        details: Option<String>,
    },
}

/// A group's header (FR-007).
pub fn group_title(kind: CommandKind) -> &'static str {
    match kind {
        CommandKind::Skill => "Skills",
        CommandKind::Custom => "Custom commands",
        CommandKind::BuiltIn => "Built-in commands",
        CommandKind::App => "In this app",
    }
}

/// Where `query` lies in `name`, ignoring case and in `name`'s own bytes, as the palette matched
/// it: the name's start, else a word's start (after `-`, `_` or `:`), else the first occurrence —
/// core `filter`'s tiers, so the emphasis falls where the row was ranked.
pub fn row_match(name: &str, query: &str) -> Option<Range<usize>> {
    if query.is_empty() {
        return None;
    }
    let at = |start: usize| {
        let rest = name.get(start..)?;
        let mut taken = 0;
        let mut wanted = query.chars().flat_map(char::to_lowercase);
        for c in rest.chars() {
            let mut lowered = c.to_lowercase();
            if !lowered.all(|l| wanted.next() == Some(l)) {
                return None;
            }
            taken += c.len_utf8();
            if wanted.clone().next().is_none() {
                return Some(start..start + taken);
            }
        }
        wanted.next().is_none().then_some(start..start + taken)
    };
    let starts = || name.char_indices().map(|(at, _)| at);
    let word_starts = || {
        name.char_indices()
            .filter(|(_, c)| matches!(c, '-' | '_' | ':'))
            .map(|(at, c)| at + c.len_utf8())
    };
    at(0)
        .or_else(|| word_starts().find_map(at))
        .or_else(|| starts().find_map(at))
}

/// What a row says under its name: [`row_tag`] first, so a description cut to the row's width
/// never takes it (FR-010), then the description.
pub fn row_details(command: &Command) -> Option<String> {
    let parts: Vec<&str> = row_tag(command)
        .into_iter()
        .chain(command.description.as_deref())
        .collect();
    (!parts.is_empty()).then(|| parts.join(" · "))
}

/// The words a row carries beside its name: [`OPENS_IN_TERMINAL`] for a terminal-only command.
pub fn row_tag(command: &Command) -> Option<&'static str> {
    command.terminal_only.map(|_| OPENS_IN_TERMINAL)
}

/// A key the open palette takes (FR-008a).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteKey {
    Up,
    Down,
    /// Tab, or Enter on a selected row.
    Accept,
    /// Escape: close, the draft unchanged, the turn not interrupted.
    Close,
}

/// What `press` does to `palette`: `None` while it is closed, and for every key it does not use,
/// so #941's `composer_key` runs. Enter with no match is the composer's send (US2 scenario 5).
pub fn palette_key(palette: Option<&PaletteState>, press: &KeyInput) -> Option<PaletteKey> {
    let palette = palette?;
    if press.mods.ctrl || press.mods.alt || press.mods.logo || press.mods.shift {
        return None;
    }
    let Key::Named(named) = press.key else {
        return None;
    };
    let selected = palette.highlight().is_some();
    match named {
        NamedKey::ArrowUp => Some(PaletteKey::Up),
        NamedKey::ArrowDown => Some(PaletteKey::Down),
        NamedKey::Tab | NamedKey::Enter if selected => Some(PaletteKey::Accept),
        NamedKey::Escape => Some(PaletteKey::Close),
        _ => None,
    }
}

/// What accepting a row does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Picked {
    /// The draft becomes `text` with the caret at `caret`.
    Fill { text: String, caret: (usize, usize) },
    /// A terminal-only command: offer the terminal for it, with why.
    TerminalOnly { command: String, reason: String },
}

/// One model the picker offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelItem {
    pub choice: ModelChoice,
    /// The CLI's display name, else the value; [`DEFAULT_MODEL`] for the default.
    pub label: String,
    pub description: Option<String>,
}

/// An open model picker (FR-014): Default, the models the CLI reported (else
/// [`MODEL_SHORTCUTS`]), the current choice marked, and a typed name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelPickerState {
    items: Vec<ModelItem>,
    current: ModelChoice,
    typed: String,
    highlight: Option<usize>,
}

impl ModelPickerState {
    pub fn new(reported: &[ReportedModel], current: &ModelChoice) -> Self {
        let default = ModelItem {
            choice: ModelChoice::Default,
            label: DEFAULT_MODEL.to_string(),
            description: None,
        };
        let named = |value: &str, label: Option<&str>, description: Option<&str>| {
            ModelName::parse(value).ok().map(|name| ModelItem {
                choice: ModelChoice::Named(name),
                label: label.unwrap_or(value).to_string(),
                description: description.map(str::to_string),
            })
        };
        let models: Vec<ModelItem> = if reported.is_empty() {
            MODEL_SHORTCUTS
                .iter()
                .filter_map(|m| named(m, None, None))
                .collect()
        } else {
            reported
                .iter()
                .filter(|m| m.value != ModelChoice::Default.cli_value())
                .filter_map(|m| {
                    named(
                        &m.value,
                        m.display_name.as_deref(),
                        m.description.as_deref(),
                    )
                })
                .collect()
        };
        Self {
            items: std::iter::once(default).chain(models).collect(),
            current: current.clone(),
            typed: String::new(),
            highlight: None,
        }
    }

    pub fn items(&self) -> &[ModelItem] {
        &self.items
    }

    /// Whether `item` is the session's current choice.
    pub fn is_current(&self, item: &ModelItem) -> bool {
        item.choice == self.current
    }

    /// The name typed into the picker.
    pub fn typed(&self) -> &str {
        &self.typed
    }

    pub fn highlight(&self) -> Option<usize> {
        self.highlight
    }

    /// What the window lists: each item, then the typed name when one is typed (FR-014).
    pub fn rows(&self) -> Vec<ModelRow> {
        let items = self.items.iter().enumerate().map(|(at, item)| ModelRow {
            label: item.label.clone(),
            details: item.description.clone(),
            item: Some(at),
        });
        let typed = (!self.typed.trim().is_empty()).then(|| ModelRow {
            label: use_typed(self.typed.trim()),
            details: None,
            item: None,
        });
        items.chain(typed).collect()
    }

    /// The row of the current choice.
    pub fn current_row(&self) -> Option<usize> {
        self.items.iter().position(|item| self.is_current(item))
    }

    /// The row Enter takes: the highlighted item, else the typed name's.
    pub fn enter_row(&self) -> Option<usize> {
        self.highlight
            .or_else(|| (!self.typed.trim().is_empty()).then_some(self.items.len()))
    }

    /// Item `item`'s choice; `None`, the typed name (FR-014). A row the window drew names one or
    /// the other ([`Self::rows`]): the typed row's `item` is `None`.
    fn chosen(&self, item: Option<usize>) -> Result<ModelChoice, ModelRefusal> {
        match item.and_then(|i| self.items.get(i)) {
            Some(item) => Ok(item.choice.clone()),
            None => model_choice(&self.typed),
        }
    }
}

/// One row of the drawn model picker: an item, or the typed name (`item` `None`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRow {
    pub label: String,
    pub details: Option<String>,
    pub item: Option<usize>,
}

/// The picker's row for a typed model name.
pub fn use_typed(name: &str) -> String {
    format!("Use {name}")
}

/// `name` as a choice: `default` is [`ModelChoice::Default`].
fn model_choice(name: &str) -> Result<ModelChoice, ModelRefusal> {
    let name = name.trim();
    if name == ModelChoice::Default.cli_value() {
        Ok(ModelChoice::Default)
    } else {
        ModelName::parse(name).map(ModelChoice::Named)
    }
}

/// The view's model line (FR-013): the model as the CLI last reported it, and whether the user
/// chose it or it is the default. A pick on a busy session shows the new choice beside the model
/// still answering, until the CLI reports the change (US3 scenario 4).
pub fn model_header(reported: Option<&str>, choice: &ModelChoice) -> String {
    let model = reported.unwrap_or(MODEL_NOT_REPORTED);
    match choice {
        ModelChoice::Default => format!("{model} · {DEFAULT_MODEL}"),
        ModelChoice::Named(name) => format!("{model} · your choice: {}", name.as_str()),
    }
}

/// What sending a draft does (FR-010, FR-018).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendIntent {
    /// A message for the CLI: the composer sends it.
    Message,
    /// Handled here: the picker opened, the terminal was offered, or a refusal was said.
    Nothing,
    /// `/model <name>`: ask the service for this model.
    SetModel(ModelChoice),
}

/// The palettes and model pickers open in this window, per session.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    palettes: HashMap<SessionId, PaletteState>,
    /// The draft text Escape closed the palette over: it stays closed until the text changes.
    dismissed: HashMap<SessionId, String>,
    pickers: HashMap<SessionId, ModelPickerState>,
}

impl State {
    pub fn palette(&self, session: SessionId) -> Option<&PaletteState> {
        self.palettes.get(&session)
    }

    pub fn picker(&self, session: SessionId) -> Option<&ModelPickerState> {
        self.pickers.get(&session)
    }

    /// The draft is now `text` with the caret at `caret`: open, narrow or close the palette.
    /// `list` is asked for only when the palette opens.
    pub fn draft_changed(
        &mut self,
        session: SessionId,
        text: &str,
        caret: (usize, usize),
        list: impl FnOnce() -> CommandList,
    ) {
        if self.dismissed.get(&session).is_some_and(|t| t == text) {
            return;
        }
        self.dismissed.remove(&session);
        let query = (caret.0 == 0)
            .then(|| palette_query(text, caret.1))
            .flatten();
        match (query, self.palettes.get_mut(&session)) {
            (None, _) => {
                self.palettes.remove(&session);
            }
            (Some(query), Some(open)) => open.narrow(query),
            (Some(query), None) => {
                self.palettes
                    .insert(session, PaletteState::new(list(), query));
            }
        }
    }

    /// Move the selection.
    pub fn step(&mut self, session: SessionId, direction: Direction) {
        if let Some(palette) = self.palettes.get_mut(&session) {
            palette.selection.step(direction, palette.rows.len());
        }
    }

    /// Escape, or a terminal-only row picked: close over the draft `text`, which stays as it is
    /// and does not reopen the palette until it changes.
    pub fn dismiss(&mut self, session: SessionId, text: &str) {
        self.palettes.remove(&session);
        self.dismissed.insert(session, text.to_string());
    }

    /// Accept row `row` (the selected one when `None`), closing the palette.
    pub fn pick(&mut self, session: SessionId, row: Option<usize>) -> Option<Picked> {
        let palette = self.palettes.get(&session)?;
        let command = palette.row(row.or(palette.highlight())?)?.clone();
        self.palettes.remove(&session);
        Some(match command.terminal_only {
            Some(why) => Picked::TerminalOnly {
                command: command.name,
                reason: why.to_string(),
            },
            None => {
                let (text, caret) = accept(&command.name);
                Picked::Fill {
                    text,
                    caret: (0, caret),
                }
            }
        })
    }

    /// Open the model picker over `reported` with `current` marked.
    pub fn open_picker(
        &mut self,
        session: SessionId,
        reported: &[ReportedModel],
        current: &ModelChoice,
    ) {
        self.pickers
            .insert(session, ModelPickerState::new(reported, current));
    }

    /// The name typed into the picker; the highlight leaves the items.
    pub fn picker_typed(&mut self, session: SessionId, text: String) {
        if let Some(picker) = self.pickers.get_mut(&session) {
            picker.typed = text;
            picker.highlight = None;
        }
    }

    pub fn picker_step(&mut self, session: SessionId, direction: Direction) {
        if let Some(picker) = self.pickers.get_mut(&session) {
            // Over every row the window draws, so the typed name is reachable with the arrows
            // as the items are.
            let rows = picker.rows().len();
            picker.highlight = move_highlight(picker.highlight, direction, rows);
        }
    }

    /// Choose item `item` (the typed name when `None`), closing the picker. `None` when no picker
    /// is open.
    pub fn picker_chose(
        &mut self,
        session: SessionId,
        item: Option<usize>,
    ) -> Option<Result<ModelChoice, ModelRefusal>> {
        let picker = self.pickers.remove(&session)?;
        Some(picker.chosen(item))
    }

    pub fn close_picker(&mut self, session: SessionId) {
        self.pickers.remove(&session);
    }

    /// Forget `session` (it is gone).
    pub fn forget(&mut self, session: SessionId) {
        self.palettes.remove(&session);
        self.dismissed.remove(&session);
        self.pickers.remove(&session);
    }
}

/// How a draft of `text` is sent, with `list` the session's commands (FR-010, FR-018).
pub(crate) fn classify(text: &str, list: &CommandList) -> Classified {
    match classify_draft(text, list) {
        DraftCommand::Send => Classified::Message,
        DraftCommand::Model(None) => Classified::Picker,
        DraftCommand::Model(Some(name)) => Classified::Model(model_choice(&name)),
        DraftCommand::TerminalOnly(name) => {
            let why = list
                .commands()
                .iter()
                .find(|c| c.answers_to(&name))
                .and_then(|c| c.terminal_only)
                .unwrap_or(CLI_MARKED_WHY);
            Classified::TerminalOnly {
                command: name,
                reason: why.to_string(),
            }
        }
    }
}

pub(crate) enum Classified {
    Message,
    Picker,
    Model(Result<ModelChoice, ModelRefusal>),
    TerminalOnly { command: String, reason: String },
}

impl crate::app::State {
    /// `session`'s command list: what its transcript holds, else the fallback (FR-009).
    pub fn command_list(&self, session: SessionId) -> CommandList {
        self.structured_transcript
            .transcript(session)
            .map_or_else(CommandList::fallback, |t| t.command_list())
    }

    /// The model `session`'s user chose (FR-017), as the service last reported it.
    pub fn model_choice(&self, session: SessionId) -> ModelChoice {
        self.workspace
            .find_session(session)
            .map(|(_, s)| s.model.clone())
            .unwrap_or_default()
    }

    /// `session`'s model line (FR-013, [`model_header`]).
    pub fn model_header(&self, session: SessionId) -> String {
        let reported = self
            .structured_transcript
            .transcript(session)
            .and_then(|t| t.session_model());
        model_header(reported.as_deref(), &self.model_choice(session))
    }
}
