//! The add-worktree form, rendered as a Material modal overlay (FR-005, FR-008a).
//!
//! Captures a Conventional-Commits type, an optional ticket, and a name, and shows the live
//! derived directory/branch preview so the outcome is predictable before creating (FR-008a).
//!
//! Feature 016 adds a second way in — picking a branch that already exists — and turns a
//! branch-name collision from a dead-end error into a decision panel rendered in place of the
//! normal actions, so cancelling leaves every input where the user left it (FR-007).

use crate::app::Message;
use crate::features::worktree_form::{
    BranchSource, ResolutionState, WorktreeForm, WorktreeFormStatus,
};
use crate::ui::material::{
    self, Button, Select, StageProgress, SurfaceKind, Text, TextField, ToggleChip, TypeRole,
    TypeaheadRow,
};
use iced::widget::{column, row, Space};
use iced::{Element, Length};
use micold_core::naming::ConventionalType;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, spacing, Roles};
use micold_core::worktree::{BranchOrigin, BranchSituation, CreateMode};

/// The add-worktree form as the dialog body; `ui::view` wraps it in the shared
/// [`Modal`](crate::ui::material::Modal) transition.
pub fn modal<'a>(
    form: &'a WorktreeForm,
    error: Option<&'a str>,
    scheme: ColorScheme,
) -> Element<'a, Message> {
    let r = tokens::roles(scheme);
    let is_creating = form.status == WorktreeFormStatus::Creating;

    let heading = Text::new("New worktree", TypeRole::Headline, r);

    let mut fields = material::dialog::fields(column![heading, source_switch(form, r)]);

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
            .placeholder("Select a type…")
            // §7.7: the label moves *inside* the control's container, where every other field
            // carries its own. It used to be a free-standing muted line above the select, which is
            // the arrangement FR-031a replaces.
            .label("Type");

            // No placeholder: §7.7's migration table moves the whole of the old one into the label
            // and the supporting text. Keeping the example in the placeholder as well would print
            // it twice on an empty field — once greyed inside the container and once beneath it.
            let ticket = TextField::new("", &form.ticket, r)
                .label("Ticket")
                .supporting("Optional — e.g. ABC-123")
                .on_input(Message::AddWorktreeTicketChanged);

            let name = TextField::new("", &form.name, r)
                .label("Name")
                .supporting("e.g. login page")
                .on_input(Message::AddWorktreeNameChanged)
                .on_submit(Message::AddWorktreeSubmitted);

            fields = fields.push(type_select).push(ticket).push(name);
        }
        BranchSource::Existing => {
            fields = fields.push(branch_picker(form, r));
        }
    }

    fields = fields.push(preview(form, r));

    // Why a selected branch can't be used (FR-012). Feature 021 moved the refusal to the point of
    // choice — a blocked branch is a disabled row and can no longer become the selection — so this
    // is now unreachable through the picker. Kept as the invariant's last line of defence, for the
    // same reason `can_submit()`'s guard is (contract §5): it costs one comparison, and its absence
    // would let a blocked branch pass silently if any future path ever set `selected_branch`.
    if let Some(candidate) = &form.selected_branch {
        if form.source == BranchSource::Existing {
            if let Some(reason) = &candidate.blocked_by {
                fields = fields.push(
                    Text::new(reason.explain(&candidate.name), TypeRole::Caption, r).tint(r.error),
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
        fields = fields.push(Text::new(message, TypeRole::Caption, r).tint(r.error));
    }

    // In-progress state while the daemon runs the create (T055). The daemon reports the stage, and
    // — for a stage long enough to have live output, i.e. a submodule fetch — its latest line at a
    // rate-limited cadence (BUG-009, T123), so a multi-minute fetch reads as moving rather than
    // frozen on one label. The indicator stands until the daemon's reply closes the form (or
    // reopens it with an error).
    //
    // Feature 016 FR-024: name the step for the mode in flight — "Checking out existing branch"
    // rather than "Creating branch" — from the stage the daemon reports. Falls back to the
    // generic wording until the first stage lands, which is a real window: the RPC has been sent
    // but the daemon has not started git yet.
    if is_creating {
        let label = form.stage_label().unwrap_or("Creating worktree…");
        fields = fields.push(StageProgress::new(label, r).detail(form.stage_detail.clone()));
    }

    // While a decision is pending, the prompt's own actions replace Create/Cancel — there is
    // exactly one thing to do next.
    let actions = match &form.resolution {
        ResolutionState::Idle => default_actions(form, r),
        state => resolution_panel(state, r),
    };

    let dialog = material::Surface::new(
        material::dialog::body(fields, actions),
        SurfaceKind::Dialog,
        r,
    )
    .width(Length::Fixed(520.0));

    dialog.into()
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
    // No free-standing label: the type-ahead carries its own now, inside its container, the same
    // way the `Type` select does (§7.7, FR-031a). This was the last field in the application still
    // showing its name above the box while every other one showed it inside.
    let mut col = column![].spacing(spacing::XS);

    if form.candidates.is_empty() {
        // Never an empty control with no explanation (FR-013).
        return col
            .push(
                Text::new(
                    "This repository has no other branches.",
                    TypeRole::Caption,
                    r,
                )
                .muted(),
            )
            .into();
    }

    if !form.candidates.iter().any(|c| c.is_available()) {
        // The list is shown anyway, so the per-branch reasons stay visible (FR-012).
        col = col.push(
            Text::new(
                "No branches are available to reuse — every branch is already checked out.",
                TypeRole::Caption,
                r,
            )
            .muted(),
        );
    }

    // The mapping — the one place branch vocabulary and component vocabulary meet (contract §5).
    // The label is `BranchCandidate`'s own `Display`, so the origin and in-use suffixes survive
    // verbatim; the emphasis spans index the branch name, which that `Display` writes first, so
    // they land on the same characters in the longer string.
    let rows: Vec<TypeaheadRow> = form
        .branch_matches
        .iter()
        .filter_map(|(index, matched)| {
            let candidate = form.candidates.get(*index)?;
            let row = TypeaheadRow::new(candidate.to_string(), matched.spans.clone());
            Some(if candidate.is_available() {
                row
            } else {
                row.disabled()
            })
        })
        .collect();

    // Where the current selection sits among the rows on screen, so the marker follows it as the
    // search narrows — and disappears while it is filtered out, rather than marking the wrong row.
    let selected = form.selected_branch.as_ref().and_then(|chosen| {
        form.branch_matches
            .iter()
            .position(|(index, _)| form.candidates.get(*index) == Some(chosen))
    });

    col = col.push(
        material::Typeahead::new(
            &form.branch_query,
            rows,
            Message::AddWorktreeBranchQueryChanged,
            r,
        )
        .placeholder("Search branches…")
        .label("Branch")
        .open(form.branch_list_open)
        .highlighted(form.branch_highlight)
        .selected(selected)
        .empty_message("No branches match that search.")
        .on_focus(Message::AddWorktreeBranchFocused)
        .on_move(Message::AddWorktreeBranchHighlightMoved)
        .on_dismiss(Message::AddWorktreeBranchDismissed)
        .on_pick(|index| {
            // The index is into the rows the component was handed, which are exactly
            // `branch_matches` — so it resolves back to a candidate here, where both are in hand.
            form.branch_matches
                .get(index)
                .and_then(|(candidate, _)| form.candidates.get(*candidate))
                .map(|c| Message::AddWorktreeBranchSelected(c.clone()))
                .unwrap_or(Message::NoOp)
        }),
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
                TypeRole::Caption,
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
                    TypeRole::Caption,
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
                    TypeRole::Caption,
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
                        TypeRole::Caption,
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
                Text::new(reason.explain(branch), TypeRole::Body, r).tint(r.error),
                Text::new(
                    "A branch can only be checked out in one place at a time. Open that \
                     location to continue there, or choose a different name.",
                    TypeRole::Caption,
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
                        TypeRole::Caption,
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
