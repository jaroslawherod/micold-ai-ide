//! The session bar at the top of a structured session's transcript (feature 944, X1): the context
//! meter, the session totals and the running background-task count on one row that wraps when the
//! pane is narrower than it, each with its details in the shared tooltip; under it the folded
//! background-task list, while the session has background tasks (FR-001–FR-008, FR-020).
//!
//! Everything it says comes from [`session_bar_view`] (E-1).

use super::entries::task_label_view;
use super::insights::{session_bar_view, MeterView, SessionBarView, TaskRow, BACKGROUND_TASKS};
use crate::features::structured_insights::{InsightsMsg, State};
use crate::features::structured_transcript::TranscriptMsg;
use crate::icons::Icon;
use crate::ui::material::{AccordionSection, IconLabel, ProgressBar, Text, Tooltip, TypeRole};
use iced::widget::{column, row, Space};
use iced::{Alignment, Element, Length};
use micold_core::session::SessionId;
use micold_core::structured::insights::SessionInsights;
use micold_core::tokens::{spacing, Roles};

/// How wide the meter's bar is drawn, in dp: long enough to read a share and a threshold tick at a
/// glance, short enough to leave the row its figures (SC-005).
pub const METER_WIDTH: f32 = 96.0;

/// `widget` with `details` in the shared tooltip, or alone when there are none.
fn with_details<'a>(
    widget: impl Into<Element<'a, TranscriptMsg>>,
    details: &[String],
    roles: Roles,
) -> Element<'a, TranscriptMsg> {
    if details.is_empty() {
        widget.into()
    } else {
        Tooltip::new(widget, details.join("\n"), roles).into()
    }
}

/// The meter: its bar (only with a window, FR-004), then its label; the warning adds
/// [`Icon::UsageWarning`] and "Near compaction" in the error role, never colour alone (E-4).
fn meter(view: &MeterView, roles: Roles) -> Element<'static, TranscriptMsg> {
    let mut parts = row![].spacing(spacing::SM).align_y(Alignment::Center);
    if let Some((ratio, marker)) = view.bar {
        let mut bar = ProgressBar::new(ratio, roles)
            .width(Length::Fixed(METER_WIDTH))
            .warning(view.warning);
        if let Some(at) = marker {
            bar = bar.marker(at);
        }
        parts = parts.push(bar);
    }
    let label: Element<'static, TranscriptMsg> = if view.warning {
        IconLabel::new(
            Icon::UsageWarning,
            view.label.clone(),
            TypeRole::Label,
            roles,
        )
        .tint(roles.error)
        .label_tint(roles.error)
        .into()
    } else {
        Text::new(view.label.clone(), TypeRole::Label, roles)
            .muted()
            .into()
    };
    parts = parts.push(label);
    with_details(parts, &view.details, roles)
}

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

/// The bar as `view` says, its list unfolded while `tasks_open`.
pub fn bar(
    view: &SessionBarView,
    session: SessionId,
    tasks_open: bool,
    roles: Roles,
) -> Element<'static, TranscriptMsg> {
    let mut line = row![meter(&view.meter, roles)]
        .spacing(spacing::MD)
        .align_y(Alignment::Center);
    if let Some(totals) = &view.totals {
        line = line.push(with_details(
            Text::new(totals.clone(), TypeRole::Label, roles).muted(),
            &view.totals_details,
            roles,
        ));
    }
    if let Some(running) = &view.running {
        line = line.push(
            IconLabel::new(
                Icon::BackgroundTask,
                running.clone(),
                TypeRole::Label,
                roles,
            )
            .muted(),
        );
    }
    let mut bar = column![line.wrap().vertical_spacing(spacing::XS)]
        .spacing(spacing::SM)
        .width(Length::Fill);
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
    bar.into()
}

/// The session bar of `session` as `state` holds it, with its CLI `cli_running` or not.
pub fn view(
    state: &State,
    session: SessionId,
    cli_running: bool,
    roles: Roles,
) -> Element<'static, TranscriptMsg> {
    let empty = SessionInsights::new();
    let insights = state.insights(session).unwrap_or(&empty);
    bar(
        &session_bar_view(insights, cli_running),
        session,
        state.tasks_open(session),
        roles,
    )
}
