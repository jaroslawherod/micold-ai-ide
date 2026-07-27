//! The add-worktree form, rendered as a Material modal overlay (FR-005, FR-008a).
//!
//! Captures a Conventional-Commits type, an optional ticket, and a name, and shows the live
//! derived directory/branch preview so the outcome is predictable before creating (FR-008a).
//!
//! Feature 016 adds a second way in — picking a branch that already exists — and turns a
//! branch-name collision from a dead-end error into a decision panel rendered in place of the
//! normal actions, so cancelling leaves every input where the user left it (FR-007).

use crate::app::{BranchSource, Message, ResolutionState, WorktreeForm, WorktreeFormStatus};
use crate::ui::cdk::overlay::Surface;
use crate::ui::material::{
    self, Button, Modal, Select, StageProgress, SurfaceKind, Text, TextField, ToggleChip, TypeRole,
};
use iced::widget::{column, row, Space};
use iced::{Element, Length};
use micold_core::naming::ConventionalType;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, spacing, Roles};
use micold_core::worktree::{BlockReason, BranchOrigin, BranchSituation, CreateMode};

/// The add-worktree form as a modal surface, at transition `progress`
/// (1.0 = fully shown, 0.0 = hidden — see [`Modal`]).
pub fn modal<'a>(
    form: &'a WorktreeForm,
    error: Option<&'a str>,
    scheme: ColorScheme,
    progress: f32,
) -> Option<Surface<'a, Message>> {
    let r = tokens::roles(scheme);
    let is_creating = form.status == WorktreeFormStatus::Creating;

    let heading = Text::new("New worktree", TypeRole::Headline, r);

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

            let ticket = TextField::new("Ticket (optional, e.g. ABC-123)", &form.ticket, r)
                .on_input(Message::AddWorktreeTicketChanged);

            let name = TextField::new("Name (e.g. login page)", &form.name, r)
                .on_input(Message::AddWorktreeNameChanged)
                .on_submit(Message::AddWorktreeSubmitted);

            fields = fields
                .push(Text::new("Type", TypeRole::Label, r).muted())
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
                    Text::new(block_sentence(&candidate.name, reason), TypeRole::Label, r)
                        .tint(r.error),
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
        fields = fields.push(Text::new(message, TypeRole::Label, r).tint(r.error));
    }

    // In-progress state while the daemon runs the create (T055). Git now runs on the daemon, which
    // does not stream per-command/submodule progress, so this is a single continuous indicator until
    // the daemon's reply closes the form (or reopens it with an error).
    //
    // Feature 016 FR-024: name the step for the mode in flight — "Checking out existing branch"
    // rather than "Creating branch" — from the stage the daemon reports. Falls back to the
    // generic wording until the first stage lands, which is a real window: the RPC has been sent
    // but the daemon has not started git yet.
    if is_creating {
        let label = form.stage_label().unwrap_or("Creating worktree…");
        fields = fields.push(StageProgress::new(label, r));
    }

    // While a decision is pending, the prompt's own actions replace Create/Cancel — there is
    // exactly one thing to do next.
    let actions = match &form.resolution {
        ResolutionState::Idle => default_actions(form, r),
        state => resolution_panel(state, r),
    };

    let dialog = material::Surface::new(fields.push(actions), SurfaceKind::Dialog, r)
        .padding(spacing::LG)
        .width(Length::Fixed(520.0));

    Modal::new(dialog, r).progress(progress).into()
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
    let mut col = column![Text::new("Branch", TypeRole::Label, r).muted()].spacing(spacing::XS);

    if form.candidates.is_empty() {
        // Never an empty control with no explanation (FR-013).
        return col
            .push(Text::new("This repository has no other branches.", TypeRole::Label, r).muted())
            .into();
    }

    if !form.candidates.iter().any(|c| c.is_available()) {
        // The list is shown anyway, so the per-branch reasons stay visible (FR-012).
        col = col.push(
            Text::new(
                "No branches are available to reuse — every branch is already checked out.",
                TypeRole::Label,
                r,
            )
            .muted(),
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
        .any(|c| !matches!(c.origin, BranchOrigin::Local))
    {
        col = col.push(
            Text::new(
                "Remote branches reflect your last fetch. Nothing is downloaded here.",
                TypeRole::Label,
                r,
            )
            .muted(),
        );
    }

    col.into()
}

/// The ordinary Create / Cancel row.
fn default_actions<'a>(form: &WorktreeForm, r: Roles) -> Element<'a, Message> {
    row![
        // No press message when the form cannot be submitted — the button renders disabled from
        // having nowhere to send, rather than from a flag that could disagree with one.
        Button::filled("Create", r)
            .on_press_maybe(form.can_submit().then_some(Message::AddWorktreeSubmitted)),
        Button::outlined("Cancel", r).on_press(Message::AddWorktreeCancelled),
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
        Button::outlined(label.to_string(), r).on_press(Message::AddWorktreeResolutionCancelled)
    };

    match state {
        ResolutionState::Idle => Space::new().height(Length::Fixed(0.0)).into(),

        // FR-005: the destructive confirmation, reachable only from `Choosing`.
        ResolutionState::ConfirmingOverwrite { situation } => {
            let branch = situation_branch(situation);
            column![
                Text::new(format!("Delete the branch '{branch}'?"), TypeRole::Body, r)
                    .tint(r.error),
                Text::new(
                    "Its commits will be discarded and the branch recreated from the current \
                     checkout. This cannot be undone from the app.",
                    TypeRole::Label,
                    r
                )
                .muted(),
                row![
                    Button::filled("Delete and recreate", r)
                        .on_press(Message::AddWorktreeOverwriteConfirmed),
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
                Text::new(
                    format!("The branch '{branch}' already exists."),
                    TypeRole::Body,
                    r
                ),
                Text::new(
                    "Reuse it to continue that work with its history intact, or overwrite it to \
                     start again from the current checkout and discard its commits.",
                    TypeRole::Label,
                    r
                )
                .muted(),
                row![
                    Button::filled("Reuse branch", r)
                        .on_press(Message::AddWorktreeResolutionChosen(CreateMode::ReuseLocal)),
                    Button::outlined("Overwrite…", r)
                        .on_press(Message::AddWorktreeOverwriteRequested),
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
                        Button::filled(format!("Continue from {remote}"), r).on_press(
                            Message::AddWorktreeResolutionChosen(CreateMode::TrackRemote {
                                remote: remote.clone(),
                            }),
                        ),
                    );
                }
                choices = choices
                    .push(
                        Button::outlined("Start fresh", r)
                            .on_press(Message::AddWorktreeResolutionChosen(CreateMode::NewBranch)),
                    )
                    .push(cancel("Cancel"));

                column![
                    Text::new(
                        format!("'{branch}' exists {where_} but not on this machine."),
                        TypeRole::Body,
                        r
                    ),
                    Text::new(
                        "Continuing picks that work up where it was left, tracking the remote \
                         branch. Starting fresh instead creates a different branch of the same \
                         name, which will diverge from the remote one.",
                        TypeRole::Label,
                        r
                    )
                    .muted(),
                    choices,
                ]
                .spacing(spacing::SM)
                .into()
            }

            // FR-021 (US5): explain and name the holder — no reuse, no overwrite offered.
            BranchSituation::Blocked { branch, reason } => column![
                Text::new(block_sentence(branch, reason), TypeRole::Body, r).tint(r.error),
                Text::new(
                    "A branch can only be checked out in one place at a time. Open that \
                     location to continue there, or choose a different name.",
                    TypeRole::Label,
                    r
                )
                .muted(),
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
                    Text::new(
                        format!("A worktree folder named '{name}' already exists."),
                        TypeRole::Body,
                        r
                    )
                    .tint(r.error),
                    Text::new(
                        "Choose a different name, or remove the existing folder first.",
                        TypeRole::Label,
                        r
                    )
                    .muted(),
                    row![cancel("OK")].spacing(spacing::SM),
                ]
                .spacing(spacing::SM)
                .into()
            }

            // `Free` never raises a prompt (FR-025); nothing to render.
            BranchSituation::Free => Space::new().height(Length::Fixed(0.0)).into(),
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
                Text::new("Directory: ", TypeRole::Label, r).muted(),
                Text::new(
                    format!(".claude/worktrees/{}", derived.dir_name),
                    TypeRole::Label,
                    r
                ),
            ],
            row![
                Text::new("Branch: ", TypeRole::Label, r).muted(),
                Text::new(derived.branch, TypeRole::Label, r),
            ],
        ]
        .spacing(spacing::XS)
        .into(),
        Err(_) => Space::new().height(Length::Fixed(0.0)).into(),
    }
}
