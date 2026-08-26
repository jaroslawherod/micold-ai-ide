//! The confirm-forget dialog for a project, rendered as a Material modal overlay (feature 014,
//! FR-002/FR-002a). Forgetting discards only what the application remembers about the project —
//! its catalog entry and per-project state (name, worktree-name overrides, session records).
//! Nothing on disk (the folder, its files, or any git worktrees) is deleted; the dialog says so.

use crate::app::{Message, State};
use crate::features::project::Msg as ProjectMsg;
use crate::ui::material::{self, Button, SurfaceKind, Text, TypeRole};
use iced::widget::{column, row};
use iced::Element;
use iced::Length;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self};

/// The confirm-forget dialog for the project shown by `display_name` as a modal surface, as the dialog body; `ui::view` wraps it
/// in the shared [`Modal`](crate::ui::material::Modal) transition.
///
/// `running_sessions` is the number of the project's currently-running sessions that will be
/// stopped on confirm (FR-002a). When it is `0`, no session-stop line is shown; when it is `> 0`,
/// the dialog states how many sessions will be stopped, matching what the binary actually
/// terminates (a running session has a live process; idle/absent ones do not).
pub fn modal<'a>(
    display_name: &str,
    running_sessions: usize,
    scheme: ColorScheme,
) -> Element<'a, Message> {
    let r = tokens::roles(scheme);

    let mut fields = material::dialog::fields(column![
        Text::new(format!("Forget “{display_name}”?"), TypeRole::Headline, r),
        Text::new(
            "This removes the project from your list and discards what the app remembers about it \
             (its name, worktree-name overrides, and session records). Nothing on disk is deleted \
             — the folder, its files, and any git worktrees are left untouched.",
            TypeRole::Body,
            r
        )
        .muted(),
    ]);

    // FR-002a: only warn about stopping sessions when there actually are running ones.
    if running_sessions > 0 {
        let noun = if running_sessions == 1 {
            "session"
        } else {
            "sessions"
        };
        fields = fields.push(
            Text::new(
                format!("This will stop {running_sessions} running {noun}."),
                TypeRole::Body,
                r,
            )
            .muted(),
        );
    }

    let actions = material::dialog::actions(row![
        Button::filled("Forget", r).on_press(Message::Project(ProjectMsg::ForgetConfirmed)),
        Button::outlined("Cancel", r).on_press(Message::Project(ProjectMsg::ForgetCancelled)),
    ]);

    let dialog = material::Surface::new(
        material::dialog::body(fields, actions),
        SurfaceKind::Dialog,
        r,
    )
    .width(Length::Fixed(460.0));

    dialog.into()
}

/// This dialog's body, built from the state that opened it — `None` when the surface is open but
/// the live state it renders is absent, so nothing is drawn rather than an empty dialog.
///
/// The uniform shape every registered dialog has, and the reason `ui::view` no longer needs a
/// match: the registration line in [`crate::overlay::registry`] names this beside the surface it
/// draws, so a dialog says where its own state lives instead of a central arm saying it for them
/// all (feature 021, T035 — FR-008, FR-009).
pub fn dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    _env_include_outcome: &'a EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    state.project.forget_target.as_ref().map(|path| {
        // The display name and running-session count are read from the catalog/sessions at
        // render time; the count (FR-002a) is exactly the set the binary will stop.
        let display_name = state
            .workspace
            .projects
            .iter()
            .find(|p| &p.path == path)
            .map(|p| p.display_name.clone())
            .unwrap_or_else(|| micold_core::project::default_display_name(path));
        let running = state.workspace.running_session_count(path);
        modal(&display_name, running, scheme)
    })
}
