//! The view model of what feature 944 draws with a transcript (contracts/transcript-extensions.md
//! X1–X5): every figure, label and state comes from a pure function here, over the core insights,
//! and the tests read them; the drawing only lays them out (E-1).

use super::entries::{footer_view, FooterView, Status, Tone};
use crate::icons::Icon;
use micold_core::structured::insights::figures::{
    context_details, context_label, task_label, thread_summary_line, totals_details, totals_line,
    turn_details, turn_figures, ESTIMATE_NOTE,
};
use micold_core::structured::insights::{
    ContextState, SessionInsights, SubagentState, Task, TaskKind, TaskState, ThreadView, TurnUsage,
};
use micold_core::structured::transcript::TurnFooter;
use micold_core::task_progress::Ratio;

/// The background-task list's title (FR-020).
pub const BACKGROUND_TASKS: &str = "Background tasks";

/// What the meter's warning says beside its icon (FR-003, E-4).
pub const NEAR_COMPACTION: &str = "Near compaction";

/// The steps a meter's bar is drawn in: its ratio is in thousandths of the window, so any token
/// count fits the bar's `u32` ratio.
const BAR_STEPS: u64 = 1_000;

/// The context meter as drawn (FR-001–FR-004).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeterView {
    /// [`context_label`]: "No data yet", tokens only, or used / window · percent.
    pub label: String,
    /// The bar's fill, with the threshold's mark when one is reported; `None` without a window.
    pub bar: Option<(Ratio, Option<Ratio>)>,
    /// At or near compaction: drawn with [`Icon::UsageWarning`], the label saying
    /// [`NEAR_COMPACTION`] (E-4).
    pub warning: bool,
    /// [`context_details`], for the meter's tooltip.
    pub details: Vec<String>,
}

/// `n` of `window` as thousandths, at most the whole bar.
fn share(n: u64, window: u64) -> Ratio {
    let done = (u128::from(n.min(window)) * u128::from(BAR_STEPS)) / u128::from(window.max(1));
    Ratio {
        done: u32::try_from(done).unwrap_or(u32::MAX),
        total: BAR_STEPS as u32,
    }
}

/// The meter of `state`.
pub fn meter_view(state: &ContextState) -> MeterView {
    let (bar, warning) = match *state {
        ContextState::Measured {
            used,
            window,
            threshold,
            warning,
            ..
        } => (
            Some((share(used, window), threshold.map(|t| share(t, window)))),
            warning,
        ),
        ContextState::NoData | ContextState::TokensOnly { .. } => (None, false),
    };
    MeterView {
        label: context_label(state),
        bar,
        warning,
        details: context_details(state),
    }
}

/// A task's state as an icon, its label and a tone: a failure never rests on colour alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskStatus {
    pub icon: Icon,
    /// [`task_label`]: the state in words, with the outcome when reported.
    pub label: String,
    pub tone: Tone,
}

/// The icon and tone of a task's state, #939's tool-card icons reused (FR-020).
fn state_look(state: TaskState) -> (Icon, Tone) {
    match state {
        TaskState::Running => (Icon::ToolRunning, Tone::Muted),
        TaskState::Finished => (Icon::ToolDone, Tone::Muted),
        TaskState::Failed => (Icon::ToolFailed, Tone::Error),
        TaskState::Stopped | TaskState::EndedWithoutReport => (Icon::ToolNoResult, Tone::Muted),
    }
}

/// `task`'s state as shown, a running task of a session whose CLI is not running shown ended
/// (FR-022).
pub fn task_status(task: &Task, cli_running: bool) -> TaskStatus {
    let (icon, tone) = state_look(task.display_state(cli_running));
    TaskStatus {
        icon,
        label: task_label(task, cli_running),
        tone,
    }
}

/// The icon of a task's kind (FR-020): a subagent, or a command (also for a kind the CLI names
/// that is neither).
pub fn kind_icon(kind: Option<TaskKind>) -> Icon {
    match kind {
        Some(TaskKind::Subagent) => Icon::Subagent,
        Some(TaskKind::Command) | None => Icon::BackgroundTask,
    }
}

/// One row of the background-task list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRow {
    pub kind: Icon,
    pub description: String,
    pub status: TaskStatus,
}

/// `"1 background task running"`, `"3 background tasks running"`; nothing while none runs
/// (FR-020).
pub fn running_text(n: usize) -> Option<String> {
    match n {
        0 => None,
        1 => Some("1 background task running".to_owned()),
        n => Some(format!("{n} background tasks running")),
    }
}

/// The session bar as drawn (X1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionBarView {
    pub meter: MeterView,
    /// [`totals_line`]: cost and turns counted, once a turn is counted (FR-007).
    pub totals: Option<String>,
    /// [`totals_details`], for the totals' tooltip.
    pub totals_details: Vec<String>,
    /// [`running_text`].
    pub running: Option<String>,
    /// One row per background task; empty: no list is drawn (FR-020).
    pub tasks: Vec<TaskRow>,
}

/// The session bar of `insights`, with the session's CLI `cli_running` or not (FR-022).
pub fn session_bar_view(insights: &SessionInsights, cli_running: bool) -> SessionBarView {
    let totals = insights.totals();
    SessionBarView {
        meter: meter_view(&insights.context().state()),
        totals: totals_line(totals),
        totals_details: totals_details(totals),
        running: running_text(insights.running_background(cli_running)),
        tasks: insights
            .background_tasks()
            .map(|task| TaskRow {
                kind: kind_icon(task.kind),
                description: task.description.clone(),
                status: task_status(task, cli_running),
            })
            .collect(),
    }
}

/// A subagent thread's header (X3, FR-015).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadHeader {
    /// [`thread_summary_line`].
    pub summary: String,
    /// Drawn failed: "Failed" with [`Icon::ToolFailed`] and the error outline (E-4).
    pub failed: Option<Status>,
}

/// The header of `view`'s thread.
pub fn thread_header(view: &ThreadView<'_>) -> ThreadHeader {
    ThreadHeader {
        summary: thread_summary_line(view),
        failed: (view.state == SubagentState::Failed).then_some(Status {
            icon: Icon::ToolFailed,
            label: "Failed",
            tone: Tone::Error,
        }),
    }
}

/// What heads the thread of a parent tool call the session does not hold (X5, FR-017).
pub fn placeholder_title(parent: &str) -> String {
    format!("Subagent of an earlier step ({parent})")
}

/// `footer`'s view with the turn's own figures after #939's (X2, FR-006): its cost and tokens,
/// and in its details the cached input, the per-model rows and the estimate note (FR-026). A turn
/// without figures adds nothing but the note, when #939's session total is a cost.
pub fn footer_view_with(footer: &TurnFooter, turn: Option<&TurnUsage>) -> FooterView {
    let mut view = footer_view(footer);
    let Some(turn) = turn else {
        if footer.cost_usd.is_some() {
            view.details.push(ESTIMATE_NOTE.to_owned());
        }
        return view;
    };
    view.figures.extend(turn_figures(turn));
    view.details = turn_details(turn);
    if footer.cost_usd.is_some() && !view.details.iter().any(|l| l == ESTIMATE_NOTE) {
        view.details.push(ESTIMATE_NOTE.to_owned());
    }
    view
}
