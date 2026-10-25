//! The Jira issue selector's advanced mode (feature 1186, contracts/selector-ui.md §1–§3): the
//! Basic/Advanced switch, and the query field with its **Run** control over the results.
//!
//! What the selector shows is decided render-free in `features::worktree_form::jira_query`; this
//! module only lays it out. The rows are the basic picker's own (`worktree_form::issue_rows_with`),
//! so an advanced result looks exactly like a listed issue (FR-006). Nothing here logs or notifies:
//! the query reaches the screen and nowhere else (FR-016). A failed run shows Jira's own messages, or
//! basic mode's notice with **Retry** (FR-010, FR-011).
//!
//! View glue under the GUI exception (Constitution I): what it shows is `QueryRun`, whose
//! transitions `tests/jira_query_state.rs` covers; its look is the layout snapshot's and the
//! recorded pass's (quickstart §B).

use crate::app::Message;
use crate::features::worktree_form::jira_query::{Msg as QueryMsg, QueryRun};
use crate::features::worktree_form::Msg as FormMsg;
use crate::features::worktree_form::WorktreeForm;
use crate::ui::material::{self, Button, StageProgress, Text, ToggleChip, TypeRole};
use iced::widget::{row, Column};
use iced::{Element, Length};
use micold_core::format::group_digits;
use micold_core::jira_query::{QueryFailure, QueryMode, NOT_VALID_JQL};
use micold_core::jira_status::StatusBoard;
use micold_core::tokens::{spacing, Roles};

/// The query field's placeholder, and the prompt shown before any query has run (FR-004a).
pub const PROMPT: &str = "Type a JQL query, then press Enter or Run";

/// What an empty result says in place of the rows, distinct from an error (FR-012).
pub const NO_MATCH: &str = "No issue matches this query.";

fn message(msg: QueryMsg) -> Message {
    Message::WorktreeForm(FormMsg::JiraQuery(msg))
}

/// The Basic/Advanced switch heading the issue picker (FR-001): two chips, the active one the mode
/// shown. The picker shows it only while the Jira source is showing.
pub fn mode_switch<'a>(form: &WorktreeForm, r: Roles) -> Element<'a, Message> {
    let chip = |label: &'static str, mode: QueryMode| {
        ToggleChip::new(label, message(QueryMsg::ModeChanged(mode)), r)
            .active(form.jira_query.mode == mode)
    };
    row![
        chip("Basic", QueryMode::Basic),
        chip("Advanced", QueryMode::Advanced)
    ]
    .spacing(spacing::SM)
    .into()
}

/// The selector's body in advanced mode, in place of the basic picker (contracts/selector-ui.md
/// §1), below what `col` already holds (the switch): the notice, the query field beside **Run**,
/// then where the run is.
pub fn picker<'a>(
    mut col: Column<'a, Message>,
    form: &'a WorktreeForm,
    board: &StatusBoard,
    r: Roles,
) -> Element<'a, Message> {
    let query = &form.jira_query;
    if let Some(notice) = form.issue_notice() {
        col = col.push(Text::new(notice, TypeRole::Caption, r).muted());
    }

    // The rows the form shows: the results once a query ran, the basic list before (US1 AS1).
    let (rows, selected) = super::worktree_form::issue_rows_with(form, board);
    let field = material::Typeahead::new(
        &query.draft,
        rows,
        |text| message(QueryMsg::DraftChanged(text)),
        r,
    )
    .label("Query")
    .placeholder(PROMPT)
    .open(query.list_open)
    .highlighted(query.highlight)
    .selected(selected)
    .on_submit(message(QueryMsg::Run))
    .on_focus(message(QueryMsg::Focused))
    .on_move(|direction| message(QueryMsg::HighlightMoved(direction)))
    .on_dismiss(message(QueryMsg::Dismissed))
    .on_pick(|index| message(QueryMsg::RowPicked(index)));
    col = col.push(
        row![
            Element::from(field),
            Button::text("Run", r).on_press(message(QueryMsg::Run)),
        ]
        .spacing(spacing::SM)
        .width(Length::Fill),
    );

    match &query.run {
        QueryRun::Idle if query.last_run.trim().is_empty() => {
            col = col.push(Text::new(PROMPT, TypeRole::Caption, r).muted());
        }
        QueryRun::Idle => {}
        QueryRun::Refused(refusal) => {
            col = col.push(Text::new(refusal.message(), TypeRole::Caption, r).tint(r.error));
        }
        QueryRun::Running { .. } => {
            col = col.push(StageProgress::new(running(form), r));
        }
        QueryRun::Failed {
            failure: QueryFailure::Rejected(messages),
            ..
        } => {
            // FR-010: Jira's messages in Jira's order, one line each, or the plain words.
            if messages.is_empty() {
                col = col.push(Text::new(NOT_VALID_JQL, TypeRole::Caption, r).tint(r.error));
            }
            for line in messages {
                col = col.push(Text::new(line.as_str(), TypeRole::Caption, r).tint(r.error));
            }
        }
        QueryRun::Failed {
            failure: QueryFailure::Load(error),
            ..
        } => {
            // FR-011: basic mode's notice and Retry (533 FR-018); Retry runs the same query.
            let notice = form
                .notice_source()
                .map(|source| error.message(source))
                .unwrap_or_default();
            col = col
                .push(Text::new(notice, TypeRole::Caption, r).tint(r.error))
                .push(Button::text("Retry", r).on_press(message(QueryMsg::Retry)));
        }
        QueryRun::Loaded { listing, .. } if listing.issues.is_empty() => {
            col = col.push(Text::new(NO_MATCH, TypeRole::Caption, r).muted());
        }
        QueryRun::Loaded { listing, .. } => {
            // FR-006: say the list is capped rather than presenting a part as the whole.
            if !listing.complete {
                let caption = format!(
                    "Showing the first {} issues this query returns.",
                    group_digits(listing.issues.len() as u64)
                );
                col = col.push(Text::new(caption, TypeRole::Caption, r).muted());
            }
        }
    }
    col.into()
}

/// The progress line while a query runs. Names the site, never the query (FR-016).
fn running(form: &WorktreeForm) -> String {
    match form.jira_site() {
        Some(site) => format!("Running the query on {}…", site.host),
        None => "Running the query…".to_string(),
    }
}
