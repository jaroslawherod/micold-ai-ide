//! What the app's top bar shows for the selected worktree or session (feature 1793, data-model.md):
//! the context meter, the Idle/Working status, the Changes label and which header menu is open.
//! Render-free: the model decides *what* shows and the collapse ranks; `ui/toolbar.rs` draws it and
//! the layout does the collapsing, so there is one source of truth for the order.

use crate::app::{Message, State as App};
use crate::features::composer::Msg as ComposerMsg;
use crate::features::session::{switch_entries, Msg as SessionMsg};
use crate::features::session_strip::session_working;
use crate::features::structured_transcript::TranscriptMsg;
use micold_core::context_header::changes_label;
use micold_core::format::group_digits;
use micold_core::session::{SessionId, SessionKind};
use micold_core::structured::insights::ContextState;
use micold_core::structured::transcript::{format_cost, format_turns};

/// The collapse rank of the ring's text: it hides first (FR-011).
pub const RING_TEXT_RANK: u8 = 0;

/// The collapse rank of the Changes label: it hides after the ring text, the icon staying (FR-011).
pub const CHANGES_LABEL_RANK: u8 = 1;

/// What the meter's text says with no usage yet: no figure is invented (FR-005).
pub const NO_USAGE_TEXT: &str = "\u{2013}";

/// What the meter's tooltip says with no usage yet.
pub const NO_USAGE_TOOLTIP: &str = "No usage yet";

/// The worktree menu trigger's accessible name (FR-013).
pub const WORKTREE_MENU_NAME: &str = "Worktree actions";

/// The raw-events toggle's label, in the worktree menu.
pub const SHOW_RAW_EVENTS: &str = crate::features::transcript_selection::shown::SHOW_RAW_EVENTS;

/// Whether the shown session is working, as the status badge says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Not running, or not able to: failed, exited, starting or stopped.
    Idle,
    /// Its process is running.
    Working,
}

impl Status {
    /// The badge's words.
    pub fn label(self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Working => "Working",
        }
    }
}

/// The context meter as the top bar draws it (data-model.md *MeterText*).
#[derive(Debug, Clone, PartialEq)]
pub struct MeterText {
    /// The ring's fill, `0.0..=1.0`; `None` is the empty state.
    pub ratio: Option<f32>,
    /// `8% · $1.28`: the ring's text, which hides first when the bar is narrow.
    pub text: String,
    /// Tokens used of the window, cost and turns, for the hover.
    pub tooltip: String,
    /// What a screen reader says: the tooltip's content, so the cost survives the text hiding.
    pub accessible: String,
    /// At or near compaction: the ring is drawn in the error role.
    pub warning: bool,
}

impl MeterText {
    /// The meter of `session`, from the insights this window holds; the empty state when it has
    /// none.
    fn of(state: &App, session: Option<SessionId>) -> Self {
        let Some(insights) = session.and_then(|id| state.structured_insights.insights(id)) else {
            return Self::empty();
        };
        let totals = insights.totals();
        let cost = (totals.turns_counted > 0).then(|| format_cost(totals.cost_usd));
        let (ratio, percent, mut lines, warning) = match insights.context().state() {
            ContextState::NoData => return Self::empty(),
            ContextState::TokensOnly { used } => (
                None,
                None,
                vec![format!("Tokens: {} used", group_digits(used))],
                false,
            ),
            ContextState::Measured {
                used,
                window,
                percent,
                warning,
                ..
            } => (
                Some((used as f64 / window.max(1) as f64).min(1.0) as f32),
                Some(percent),
                vec![format!(
                    "Tokens: {} of {} ({percent}%)",
                    group_digits(used),
                    group_digits(window)
                )],
                warning,
            ),
        };
        let text = match (percent, &cost) {
            (Some(p), Some(c)) => format!("{p}% \u{b7} {c}"),
            (Some(p), None) => format!("{p}%"),
            (None, Some(c)) => c.clone(),
            (None, None) => NO_USAGE_TEXT.to_string(),
        };
        lines.extend(cost.map(|c| format!("Cost: {c}")));
        if totals.turns_counted > 0 {
            lines.push(format!("Turns: {}", format_turns(totals.turns_counted)));
        }
        if warning {
            lines.push("Near compaction".to_string());
        }
        let tooltip = lines.join("\n");
        Self {
            ratio,
            text,
            accessible: tooltip.clone(),
            tooltip,
            warning,
        }
    }

    /// No usage yet: the empty ring and no invented cost.
    fn empty() -> Self {
        Self {
            ratio: None,
            text: NO_USAGE_TEXT.to_string(),
            tooltip: NO_USAGE_TOOLTIP.to_string(),
            accessible: NO_USAGE_TOOLTIP.to_string(),
            warning: false,
        }
    }
}

/// One segment of the worktree menu's view control.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewSegment {
    /// The segment's words, also its accessible name.
    pub label: &'static str,
    /// Whether this is the session's current view.
    pub selected: bool,
    /// What pressing it sends: nothing for the current view.
    pub press: Option<Message>,
}

/// The session's kind and what switching it needs (data-model.md *WorktreeMenuModel*).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewSwitch {
    /// The session the control switches.
    pub session: SessionId,
    /// Its current view.
    pub current: SessionKind,
}

/// What the worktree `⋮` menu holds (data-model.md *WorktreeMenuModel*).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeMenuModel {
    /// Whether the structured view shows raw events (`settings.show_raw_events`).
    pub show_raw: bool,
    /// The view control: `None` for a session that is not Claude Code's, which has one view.
    pub view: Option<ViewSwitch>,
}

impl WorktreeMenuModel {
    /// What the raw-events toggle sends: the opposite of what shows now.
    pub fn raw_events_message(&self) -> Message {
        Message::Session(SessionMsg::Transcript(TranscriptMsg::ShowRawEvents(
            !self.show_raw,
        )))
    }

    /// The view control's two segments, Structured then Terminal; none without a view control. A
    /// press asks for the existing switch ([`ComposerMsg::SwitchRequested`]), which keeps its own
    /// confirmation while work is in progress.
    pub fn view_segments(&self) -> Vec<ViewSegment> {
        let Some(view) = self.view else {
            return Vec::new();
        };
        [
            ("Structured", SessionKind::Structured),
            ("Terminal", SessionKind::Terminal),
        ]
        .into_iter()
        .map(|(label, kind)| {
            let selected = view.current == kind;
            ViewSegment {
                label,
                selected,
                press: (!selected).then_some(Message::Composer(ComposerMsg::SwitchRequested {
                    session: view.session,
                    to: kind,
                })),
            }
        })
        .collect()
    }
}

/// What the top bar shows for the selected worktree or session (data-model.md *HeaderBarModel*).
#[derive(Debug, Clone, PartialEq)]
pub struct HeaderBarModel {
    /// The context meter.
    pub meter: MeterText,
    /// Idle or Working.
    pub status: Status,
    /// The Changes button's label: `Changes · no changes`, or with counts.
    pub changes: String,
    /// The location the Changes button opens.
    pub location: micold_core::session::SessionLocation,
    /// The worktree `⋮` menu's content.
    pub worktree_menu: WorktreeMenuModel,
}

impl HeaderBarModel {
    /// The model for `state`: `None` on the Overview and with nothing selected (FR-003). It reads
    /// only the shown location's own session, so another session's figures never show (Principle
    /// II).
    pub fn of(state: &App) -> Option<Self> {
        if state.overview.open {
            return None;
        }
        let location = state.shown_location()?;
        let session = state.session.active.filter(|id| {
            state
                .active_sessions()
                .iter()
                .any(|s| s.id == *id && s.location == location)
        });
        Some(Self {
            meter: MeterText::of(state, session),
            status: if session.is_some_and(|id| session_working(state, id)) {
                Status::Working
            } else {
                Status::Idle
            },
            changes: changes_label(crate::ui::session_area::counts_of(state, &location)),
            worktree_menu: WorktreeMenuModel {
                show_raw: state.settings.show_raw_events,
                view: session.and_then(|id| {
                    let current = state.workspace.find_session(id)?.1.kind;
                    (!switch_entries(state, id).is_empty()).then_some(ViewSwitch {
                        session: id,
                        current,
                    })
                }),
            },
            location,
        })
    }
}

/// Which header menu is open: at most one (data-model.md *HeaderMenu*).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HeaderMenu {
    /// None is open.
    #[default]
    None,
    /// The `+` menu of the tab row.
    Plus,
    /// The worktree `⋮` menu of the top bar.
    Worktree,
}

/// The worktree `⋮` menu as a floating surface: a popover, so Escape or an outside click closes it
/// before anything under it (FR-013).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorktreeMenu;

crate::floating_surface!(
    WorktreeMenu,
    id: crate::overlay::SurfaceId::new("header_worktree_menu"),
    layer: Popover,
    cancelled_by: Message::HeaderBar(Msg::Dismissed),
);

impl crate::overlay::registry::Registered for WorktreeMenu {
    fn open_in(state: &App) -> Option<Self> {
        // The flag alone: the bar draws the menu only where a top-bar model exists, and a
        // selection change closes it (`follow`), so a stale flag is never drawn.
        (state.header_bar.menu == HeaderMenu::Worktree).then_some(WorktreeMenu)
    }
}

/// The `+` menu of the tab row as a floating surface: a popover, so Escape or an outside click
/// closes it (FR-013).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlusMenu;

crate::floating_surface!(
    PlusMenu,
    id: crate::overlay::SurfaceId::new("header_plus_menu"),
    layer: Popover,
    cancelled_by: Message::HeaderBar(Msg::Dismissed),
);

impl crate::overlay::registry::Registered for PlusMenu {
    fn open_in(state: &App) -> Option<Self> {
        (state.header_bar.menu == HeaderMenu::Plus).then_some(PlusMenu)
    }
}

/// What the header menus' state is told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Msg {
    /// A menu's trigger was pressed: open it, or close it when it is the open one.
    Toggled(HeaderMenu),
    /// Escape or a press outside the open menu.
    Dismissed,
    /// A menu entry was chosen: the menu closes.
    Chosen,
}

/// Apply `msg` to the header menus (shape A): pure, and nothing for the root to run.
pub fn update(state: &mut crate::app::State, msg: Msg) -> Vec<crate::features::Outcome> {
    state.header_bar.update(msg);
    Vec::new()
}

/// What the header menus remember.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// The open menu.
    pub menu: HeaderMenu,
    /// The selection the menu was opened under; a different one closes it.
    under: Option<(micold_core::session::SessionLocation, Option<SessionId>)>,
}

impl State {
    /// Apply `msg`.
    pub fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Toggled(menu) => {
                self.menu = if self.menu == menu {
                    HeaderMenu::None
                } else {
                    menu
                };
            }
            Msg::Dismissed | Msg::Chosen => self.menu = HeaderMenu::None,
        }
    }

    /// Close the menu once the selection is not the one it was opened under (a selection change
    /// closes it); remember the selection otherwise.
    pub fn follow(
        &mut self,
        selection: Option<(micold_core::session::SessionLocation, Option<SessionId>)>,
    ) {
        if self.under != selection {
            self.menu = HeaderMenu::None;
            self.under = selection;
        }
    }
}
