//! The Run in parallel dialog (feature 483, contracts/parallel-surfaces.md D): one prompt, the
//! naming fields, a base branch and one provider per run, with the names the runs will get shown
//! before anything is created (FR-003).

use crate::app::{Message, State};
use crate::features::runs::{Msg, ParallelDialog};
use crate::features::window::FieldId;
use crate::ui::focus::TrackFocus;
use crate::ui::material::{self, Button, Select, SurfaceKind, Text, TextField, TypeRole};
use iced::widget::{column, row};
use iced::{Element, Length};
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::naming::ConventionalType;
use micold_core::runs::{MAX_RUNS, MIN_RUNS};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, spacing, Roles};

fn msg(m: Msg) -> Message {
    Message::Runs(m)
}

/// The dialog body; `ui::view` wraps it in the shared [`Modal`](crate::ui::material::Modal)
/// transition.
pub fn modal<'a>(
    dialog: &'a ParallelDialog,
    scheme: ColorScheme,
    focused: Option<FieldId>,
) -> Element<'a, Message> {
    let r = tokens::roles(scheme);

    let mut fields =
        material::dialog::fields(column![Text::new("Run in parallel", TypeRole::Headline, r)]);

    // The prompt every run receives. A single-line field: a multi-line editor needs a
    // binary-owned `text_editor::Content` (follow-up in the ledger).
    fields = fields.push(
        TextField::new("", &dialog.prompt, r)
            .label("Prompt")
            .supporting("Every run receives this prompt")
            .track_focus(FieldId::RunPrompt, focused)
            .on_input(|a0| msg(Msg::PromptChanged(a0))),
    );

    fields = fields.push(
        Select::new(
            ConventionalType::ALL,
            dialog.naming.type_,
            |a0| msg(Msg::TypeChanged(a0)),
            r,
        )
        .placeholder("Select a type…")
        .label("Type"),
    );
    fields = fields.push(
        TextField::new("", dialog.naming.ticket.as_deref().unwrap_or(""), r)
            .label("Ticket")
            .supporting("Optional — e.g. ABC-123")
            .track_focus(FieldId::RunTicket, focused)
            .on_input(|a0| msg(Msg::TicketChanged(a0))),
    );
    fields = fields.push(
        TextField::new("", &dialog.naming.name, r)
            .label("Name")
            .supporting("e.g. login page")
            .track_focus(FieldId::RunName, focused)
            .on_input(|a0| msg(Msg::NameChanged(a0))),
    );

    let base = (!dialog.base_branch.is_empty()).then(|| dialog.base_branch.clone());
    fields = fields.push(
        Select::new(
            &dialog.branches,
            base,
            |a0| msg(Msg::BaseBranchChanged(a0)),
            r,
        )
        .placeholder("Select a branch…")
        .label("Base branch"),
    );

    // One row per run: its derived name and its provider (D2, D3).
    let names = dialog.derived_names().ok();
    for (index, cli) in dialog.runs.iter().enumerate() {
        let label = names
            .as_ref()
            .and_then(|n| n.get(index))
            .map(|n| n.branch.clone())
            .unwrap_or_else(|| format!("Run {}", index + 1));
        let mut line = row![
            Text::new(label, TypeRole::Body, r).width(Length::Fill),
            Select::new(
                &dialog.offered,
                Some(*cli),
                move |a0| msg(Msg::ProviderChanged(index, a0)),
                r,
            )
            .placeholder("Provider")
        ]
        .spacing(spacing::SM);
        if dialog.runs.len() > MIN_RUNS {
            line = line.push(Button::outlined("Remove", r).on_press(msg(Msg::RunRemoved(index))));
        }
        fields = fields.push(line);
    }
    fields = fields.push(
        Button::outlined("Add run", r)
            .on_press_maybe((dialog.runs.len() < MAX_RUNS).then_some(msg(Msg::RunAdded))),
    );

    if let Some(error) = &dialog.error {
        fields = fields.push(Text::new(error.clone(), TypeRole::Caption, r).tint(r.error));
    }

    material::Surface::new(
        material::dialog::body(fields, actions(dialog, r)),
        SurfaceKind::Dialog,
        r,
    )
    .width(Length::Fixed(520.0))
    .into()
}

fn actions<'a>(dialog: &ParallelDialog, r: Roles) -> Element<'a, Message> {
    row![
        Button::filled("Start runs", r)
            .on_press_maybe(dialog.validate().is_ok().then_some(msg(Msg::Confirmed))),
        Button::outlined("Cancel", r).on_press(msg(Msg::Dismissed)),
    ]
    .spacing(spacing::SM)
    .into()
}

/// This dialog's body, built from the state that opened it.
pub fn dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    _env_include_outcome: &'a EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    state
        .runs
        .dialog
        .as_ref()
        .map(|d| modal(d, scheme, state.window.focused_field))
}
