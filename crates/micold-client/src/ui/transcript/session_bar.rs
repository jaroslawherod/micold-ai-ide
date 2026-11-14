//! The slim first row of a structured session's transcript body (feature 1793, R6): the folded
//! background-task list while the session has background tasks (feature 944 FR-020), and above it,
//! while the session's permission mode is Bypass, the label that says so (#942 FR-014). The context
//! meter, the totals and the running count moved to the top bar's ring and its tooltip; the row is
//! drawn only when there is something to show, and is part of the body, not a header row.
//!
//! Everything it says comes from [`session_bar_view`] (E-1).

use super::entries::task_label_view;
use super::insights::{session_bar_view, SessionBarView, TaskRow, BACKGROUND_TASKS};
use crate::features::structured_insights::{InsightsMsg, State};
use crate::features::structured_transcript::TranscriptMsg;
use crate::icons::Icon;
use crate::ui::material::{AccordionSection, IconLabel, TypeRole};
use iced::widget::{column, row, Space};
use iced::{Alignment, Element, Length};
use micold_core::session::{PermissionMode, SessionId};
use micold_core::structured::insights::SessionInsights;
use micold_core::tokens::{spacing, Roles};

/// One row of the background-task list: kind icon and description, then its state (FR-020).
fn task_row(task: &TaskRow, roles: Roles) -> Element<'static, TranscriptMsg> {
    row![
        IconLabel::new(task.kind, task.description.clone(), TypeRole::Body, roles),
        task_label_view(task.status.clone(), roles),
    ]
    .spacing(spacing::MD)
    .align_y(Alignment::Center)
    .wrap()
    .into()
}

/// The label of a session in `mode` (#942 US2 scenario 4): [`mode_warning`]'s words with the
/// warning icon in the error role, so the risk does not rest on colour alone; none outside Bypass.
///
/// [`mode_warning`]: crate::features::permission_prompts::mode_warning
fn mode_label(mode: PermissionMode, roles: Roles) -> Option<Element<'static, TranscriptMsg>> {
    let text = crate::features::permission_prompts::mode_warning(mode)?;
    Some(
        IconLabel::new(Icon::UsageWarning, text, TypeRole::Label, roles)
            .tint(roles.error)
            .label_tint(roles.error)
            .into(),
    )
}

/// The slim row as `view` says, its list unfolded while `tasks_open`, for a session in `mode`;
/// `None` when there is nothing to show.
pub fn bar(
    view: &SessionBarView,
    session: SessionId,
    tasks_open: bool,
    mode: PermissionMode,
    roles: Roles,
) -> Option<Element<'static, TranscriptMsg>> {
    let label = mode_label(mode, roles);
    if label.is_none() && view.tasks.is_empty() {
        return None;
    }
    let mut bar = column![].spacing(spacing::SM).width(Length::Fill);
    if let Some(label) = label {
        bar = bar.push(label);
    }
    if !view.tasks.is_empty() {
        let rows: Vec<Element<'static, TranscriptMsg>> =
            view.tasks.iter().map(|t| task_row(t, roles)).collect();
        let body: Element<'static, TranscriptMsg> = if tasks_open {
            column(rows).spacing(spacing::SM).into()
        } else {
            Space::new().into()
        };
        bar = bar.push(
            AccordionSection::new(BACKGROUND_TASKS, body, roles)
                .leading(Icon::BackgroundTask)
                .summary(view.tasks.len().to_string())
                .open(tasks_open)
                .on_toggle(TranscriptMsg::Insights(
                    InsightsMsg::ToggleBackgroundTasks { session },
                )),
        );
    }
    Some(bar.into())
}

/// The slim row of `session` as `state` holds it, with its CLI `cli_running` or not, in `mode`.
pub fn view(
    state: &State,
    session: SessionId,
    cli_running: bool,
    mode: PermissionMode,
    roles: Roles,
) -> Option<Element<'static, TranscriptMsg>> {
    let empty = SessionInsights::new();
    let insights = state.insights(session).unwrap_or(&empty);
    bar(
        &session_bar_view(insights, cli_running),
        session,
        state.tasks_open(session),
        mode,
        roles,
    )
}
