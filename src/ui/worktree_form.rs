//! The add-worktree form, rendered as a Material modal overlay (FR-005, FR-008a).
//!
//! Captures a Conventional-Commits type, an optional ticket, and a name, and shows the live
//! derived directory/branch preview so the outcome is predictable before creating (FR-008a).
//!
//! Feature 016 adds a second way in — picking a branch that already exists — and turns a
//! branch-name collision from a dead-end error into a decision panel rendered in place of the
//! normal actions, so cancelling leaves every input where the user left it (FR-007).

use crate::ui::material::{Modal, Select, StageProgress, ToggleChip};
use crate::ui::style;
use iced::widget::{button, column, container, row, scrollable, text, text_input, Space};
use iced::{Element, Length};
use micold_ai_ide::app::{
    BranchSource, Message, ResolutionState, WorktreeForm, WorktreeFormStatus,
};
use micold_ai_ide::naming::ConventionalType;
use micold_ai_ide::theme::ColorScheme;
use micold_ai_ide::tokens::{self, spacing, type_scale, Roles};
use micold_ai_ide::worktree::{BlockReason, BranchSituation, CreateMode};

/// Stack the add-worktree form as a modal overlay on top of `base`, at transition `progress`
/// (1.0 = fully shown, 0.0 = hidden — see [`Modal`]).
pub fn modal<'a>(
    base: Element<'a, Message>,
    form: &'a WorktreeForm,
    error: Option<&'a str>,
    scheme: ColorScheme,
    progress: f32,
) -> Element<'a, Message> {
    let r = tokens::roles(scheme);
    let is_creating = form.status == WorktreeFormStatus::Creating;

    let heading = text("New worktree").size(type_scale::HEADLINE);

    let mut fields = column![heading, source_switch(form, r)].spacing(spacing::MD);

    // Source-specific inputs (feature 016, FR-010): the original type/ticket/name fields, or the
    // existing-branch picker.
    match form.source {
        BranchSource::New => {
            // Type selector: a Material select list, not a row of buttons (feature 013,
            // FR-001–FR-004).
            let type_select = Select::new(
                ConventionalType::ALL,
                form.type_,
                Message::AddWorktreeTypeSelected,
                r,
            )
            .placeholder("Select a type…");

            let ticket = text_input("Ticket (optional, e.g. ABC-123)", &form.ticket)
                .on_input(Message::AddWorktreeTicketChanged)
                .padding(spacing::SM)
                .style(style::input(r));

            let name = text_input("Name (e.g. login page)", &form.name)
                .on_input(Message::AddWorktreeNameChanged)
                .on_submit(Message::AddWorktreeSubmitted)
                .padding(spacing::SM)
                .style(style::input(r));

            fields = fields
                .push(text("Type").size(type_scale::LABEL).style(style::muted(r)))
                .push(type_select)
                .push(ticket)
                .push(name);
        }
        BranchSource::Existing => {
            fields = fields.push(branch_picker(form, r));
        }
    }

    fields = fields.push(preview(form, r));

    // Why a selected branch can't be used (FR-012). Shown here rather than as a disabled list
    // row because `Select` wraps `pick_list`, which has no per-item disabling — the refusal
    // happens at the point of action instead (research R8).
    if let Some(candidate) = &form.selected_branch {
        if form.source == BranchSource::Existing {
            if let Some(reason) = &candidate.blocked_by {
                fields = fields.push(
                    text(block_sentence(&candidate.name, reason))
                        .size(type_scale::LABEL)
                        .style(error_text(r)),
                );
            }
        }
    }

    // Validation / create error (FR-008, FR-017).
    let message = form
        .error
        .map(|e| e.to_string())
        .or_else(|| error.map(str::to_string));
    if let Some(message) = message {
        fields = fields.push(text(message).size(type_scale::LABEL).style(error_text(r)));
    }

    // In-progress state while the async create (and any submodule fetch) is running (feature
    // 010, research R4; replaced feature 010's static "Creating worktree…" text with a
    // continuously visible progress indicator + the current stage's plain-language description,
    // feature 013 US3, FR-006/FR-007). The stage label names the actual step for the mode in
    // flight — "Checking out existing branch" rather than "Creating branch" (feature 016,
    // FR-024).
    if is_creating {
        let label = form
            .stage
            .map(|s| s.label(&form.mode))
            .unwrap_or("Starting…");
        fields = fields.push(StageProgress::new(label, r));
    }

    // Progress log display (feature 010 follow-up) — show a scrollable area with executed commands
    // and live output from submodule fetches so the user can see what's happening.
    if !form.log.is_empty() {
        let mut log_content = column![].spacing(spacing::XS);
        for line in &form.log {
            log_content =
                log_content.push(text(line).size(type_scale::LABEL).style(style::muted(r)));
        }
        let log_area = container(scrollable(log_content))
            .width(Length::Fill)
            .height(Length::Fixed(150.0))
            .style(style::dialog(r));
        fields = fields.push(log_area);
    }

    // While a decision is pending, the prompt's own actions replace Create/Cancel — there is
    // exactly one thing to do next.
    let actions = match &form.resolution {
        ResolutionState::Idle => default_actions(form, r),
        state => resolution_panel(state, r),
    };

    let dialog = container(fields.push(actions))
        .padding(spacing::LG)
        .width(Length::Fixed(520.0))
        .style(style::dialog(r));

    Modal::new(base, dialog, r).progress(progress).into()
}

/// Red body text, for validation and block explanations.
fn error_text(r: Roles) -> impl Fn(&iced::Theme) -> iced::widget::text::Style {
    move |_theme: &iced::Theme| iced::widget::text::Style {
        color: Some(style::color(r.error)),
    }
}

/// The new-branch / existing-branch switch (feature 016, FR-010), built from the shared
/// `ToggleChip` primitive rather than a bespoke control (Constitution Principle VIII).
fn source_switch<'a>(form: &WorktreeForm, r: Roles) -> Element<'a, Message> {
    row![
        ToggleChip::new(
            "New branch",
            Message::AddWorktreeSourceChanged(BranchSource::New),
            r
        )
        .active(form.source == BranchSource::New),
        ToggleChip::new(
            "Existing branch",
            Message::AddWorktreeSourceChanged(BranchSource::Existing),
            r
        )
        .active(form.source == BranchSource::Existing),
    ]
    .spacing(spacing::SM)
    .into()
}

/// The existing-branch picker (feature 016, FR-011–FR-013).
fn branch_picker<'a>(form: &'a WorktreeForm, r: Roles) -> Element<'a, Message> {
    let mut col = column![text("Branch")
        .size(type_scale::LABEL)
        .style(style::muted(r))]
    .spacing(spacing::XS);

    if form.candidates.is_empty() {
        // Never an empty control with no explanation (FR-013).
        return col
            .push(
                text("This repository has no other branches.")
                    .size(type_scale::LABEL)
                    .style(style::muted(r)),
            )
            .into();
    }

    if !form.candidates.iter().any(|c| c.is_available()) {
        // The list is shown anyway, so the per-branch reasons stay visible (FR-012).
        col = col.push(
            text("No branches are available to reuse — every branch is already checked out.")
                .size(type_scale::LABEL)
                .style(style::muted(r)),
        );
    }

    col = col.push(
        Select::new(
            &form.candidates,
            form.selected_branch.clone(),
            Message::AddWorktreeBranchSelected,
            r,
        )
        .placeholder("Select a branch…"),
    );

    // FR-020 / Constitution Principle IV: this list is read from local ref storage. Say so,
    // rather than silently presenting possibly-stale data as current.
    if form
        .candidates
        .iter()
        .any(|c| !matches!(c.origin, micold_ai_ide::worktree::BranchOrigin::Local))
    {
        col = col.push(
            text("Remote branches reflect your last fetch. Nothing is downloaded here.")
                .size(type_scale::LABEL)
                .style(style::muted(r)),
        );
    }

    col.into()
}

/// The ordinary Create / Cancel row.
fn default_actions<'a>(form: &WorktreeForm, r: Roles) -> Element<'a, Message> {
    let create_button = button(text("Create").size(type_scale::BODY)).style(style::filled(r));
    row![
        if form.can_submit() {
            create_button.on_press(Message::AddWorktreeSubmitted)
        } else {
            create_button
        },
        button(text("Cancel").size(type_scale::BODY))
            .on_press(Message::AddWorktreeCancelled)
            .style(style::outlined(r)),
    ]
    .spacing(spacing::SM)
    .into()
}

/// One sentence naming who holds a branch (feature 016, FR-021).
fn block_sentence(branch: &str, reason: &BlockReason) -> String {
    match reason {
        BlockReason::CheckedOutInProjectRoot => {
            format!("'{branch}' is currently checked out in the project itself.")
        }
        BlockReason::CheckedOutAt { path } => {
            let holder = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string());
            format!("'{branch}' is already checked out in the worktree '{holder}'.")
        }
    }
}

/// The conflict prompt and its confirmation (feature 016, contract `branch-conflict.md` §3).
fn resolution_panel<'a>(state: &ResolutionState, r: Roles) -> Element<'a, Message> {
    let cancel = |label: &str| {
        button(text(label.to_string()).size(type_scale::BODY))
            .on_press(Message::AddWorktreeResolutionCancelled)
            .style(style::outlined(r))
    };

    match state {
        ResolutionState::Idle => Space::with_height(Length::Fixed(0.0)).into(),

        // FR-005: the destructive confirmation, reachable only from `Choosing`.
        ResolutionState::ConfirmingOverwrite { situation } => {
            let branch = situation_branch(situation);
            column![
                text(format!("Delete the branch '{branch}'?"))
                    .size(type_scale::BODY)
                    .style(error_text(r)),
                text(
                    "Its commits will be discarded and the branch recreated from the current \
                     checkout. This cannot be undone from the app."
                )
                .size(type_scale::LABEL)
                .style(style::muted(r)),
                row![
                    button(text("Delete and recreate").size(type_scale::BODY))
                        .on_press(Message::AddWorktreeOverwriteConfirmed)
                        .style(style::filled(r)),
                    // Back, not Cancel: returns to the choice (invariant 3, US2 AS3).
                    cancel("Back"),
                ]
                .spacing(spacing::SM),
            ]
            .spacing(spacing::SM)
            .into()
        }

        ResolutionState::Choosing { situation } => match situation {
            // FR-002: the choice this whole feature exists to offer.
            BranchSituation::LocalAvailable { branch } => column![
                text(format!("The branch '{branch}' already exists.")).size(type_scale::BODY),
                text(
                    "Reuse it to continue that work with its history intact, or overwrite it to \
                     start again from the current checkout and discard its commits."
                )
                .size(type_scale::LABEL)
                .style(style::muted(r)),
                row![
                    button(text("Reuse branch").size(type_scale::BODY))
                        .on_press(Message::AddWorktreeResolutionChosen(CreateMode::ReuseLocal))
                        .style(style::filled(r)),
                    button(text("Overwrite…").size(type_scale::BODY))
                        .on_press(Message::AddWorktreeOverwriteRequested)
                        .style(style::outlined(r)),
                    cancel("Cancel"),
                ]
                .spacing(spacing::SM),
            ]
            .spacing(spacing::SM)
            .into(),

            // FR-016/FR-018: continue from the remote, or deliberately diverge from it.
            BranchSituation::RemoteOnly { branch, remotes } => {
                let where_ = match remotes.as_slice() {
                    [one] => format!("on {one}"),
                    many => format!("on {}", many.join(" and ")),
                };
                // One button per remote: when the name exists on several, the user says which
                // one to continue from — the app never picks for them (spec Edge Cases).
                let mut choices = row![].spacing(spacing::SM);
                for remote in remotes {
                    choices = choices.push(
                        button(text(format!("Continue from {remote}")).size(type_scale::BODY))
                            .on_press(Message::AddWorktreeResolutionChosen(
                                CreateMode::TrackRemote {
                                    remote: remote.clone(),
                                },
                            ))
                            .style(style::filled(r)),
                    );
                }
                choices = choices
                    .push(
                        button(text("Start fresh").size(type_scale::BODY))
                            .on_press(Message::AddWorktreeResolutionChosen(CreateMode::NewBranch))
                            .style(style::outlined(r)),
                    )
                    .push(cancel("Cancel"));

                column![
                    text(format!(
                        "'{branch}' exists {where_} but not on this machine."
                    ))
                    .size(type_scale::BODY),
                    text(
                        "Continuing picks that work up where it was left, tracking the remote \
                         branch. Starting fresh instead creates a different branch of the same \
                         name, which will diverge from the remote one."
                    )
                    .size(type_scale::LABEL)
                    .style(style::muted(r)),
                    choices,
                ]
                .spacing(spacing::SM)
                .into()
            }

            // FR-021 (US5): explain and name the holder — no reuse, no overwrite offered.
            BranchSituation::Blocked { branch, reason } => column![
                text(block_sentence(branch, reason))
                    .size(type_scale::BODY)
                    .style(error_text(r)),
                text(
                    "A branch can only be checked out in one place at a time. Open that \
                     location to continue there, or choose a different name."
                )
                .size(type_scale::LABEL)
                .style(style::muted(r)),
                row![cancel("OK")].spacing(spacing::SM),
            ]
            .spacing(spacing::SM)
            .into(),

            // FR-022: no branch choice can resolve a directory clash.
            BranchSituation::DirectoryTaken { dir } => {
                let name = dir
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| dir.display().to_string());
                column![
                    text(format!("A worktree folder named '{name}' already exists."))
                        .size(type_scale::BODY)
                        .style(error_text(r)),
                    text("Choose a different name, or remove the existing folder first.")
                        .size(type_scale::LABEL)
                        .style(style::muted(r)),
                    row![cancel("OK")].spacing(spacing::SM),
                ]
                .spacing(spacing::SM)
                .into()
            }

            // `Free` never raises a prompt (FR-025); nothing to render.
            BranchSituation::Free => Space::with_height(Length::Fixed(0.0)).into(),
        },
    }
}

/// The branch a situation concerns, for headings.
fn situation_branch(situation: &BranchSituation) -> &str {
    match situation {
        BranchSituation::LocalAvailable { branch }
        | BranchSituation::RemoteOnly { branch, .. }
        | BranchSituation::Blocked { branch, .. } => branch,
        BranchSituation::Free | BranchSituation::DirectoryTaken { .. } => "",
    }
}

/// The live derived directory/branch preview (FR-008a), or a hint when input is incomplete.
fn preview<'a>(form: &WorktreeForm, r: Roles) -> Element<'a, Message> {
    match form.preview() {
        Ok(derived) => column![
            row![
                text("Directory: ")
                    .size(type_scale::LABEL)
                    .style(style::muted(r)),
                text(format!(".claude/worktrees/{}", derived.dir_name)).size(type_scale::LABEL),
            ],
            row![
                text("Branch: ")
                    .size(type_scale::LABEL)
                    .style(style::muted(r)),
                text(derived.branch).size(type_scale::LABEL),
            ],
        ]
        .spacing(spacing::XS)
        .into(),
        Err(_) => Space::with_height(Length::Fixed(0.0)).into(),
    }
}
