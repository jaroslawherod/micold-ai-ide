//! The confirm-forget dialog for a project, rendered as a Material modal overlay (feature 014,
//! FR-002/FR-002a). Forgetting discards only what the application remembers about the project —
//! its catalog entry and per-project state (name, worktree-name overrides, session records).
//! Nothing on disk (the folder, its files, or any git worktrees) is deleted; the dialog says so.

use crate::app::Message;
use crate::ui::cdk::overlay::Surface;
use crate::ui::material::{self, Button, Modal, SurfaceKind, Text, TypeRole};
use iced::widget::{column, row};
use iced::Length;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, spacing};

/// The confirm-forget dialog for the project shown by `display_name` as a modal surface,
/// at transition `progress` (1.0 = fully shown, 0.0 = hidden — see [`Modal`]).
///
/// `running_sessions` is the number of the project's currently-running sessions that will be
/// stopped on confirm (FR-002a). When it is `0`, no session-stop line is shown; when it is `> 0`,
/// the dialog states how many sessions will be stopped, matching what the binary actually
/// terminates (a running session has a live process; idle/absent ones do not).
pub fn modal<'a>(
    display_name: &str,
    running_sessions: usize,
    scheme: ColorScheme,
    progress: f32,
) -> Option<Surface<'a, Message>> {
    let r = tokens::roles(scheme);

    let mut fields = column![
        Text::new(format!("Forget “{display_name}”?"), TypeRole::Headline, r),
        Text::new(
            "This removes the project from your list and discards what the app remembers about it \
             (its name, worktree-name overrides, and session records). Nothing on disk is deleted \
             — the folder, its files, and any git worktrees are left untouched.",
            TypeRole::Body,
            r
        )
        .muted(),
    ]
    .spacing(spacing::MD);

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

    let actions = row![
        Button::filled("Forget", r).on_press(Message::ProjectForgetConfirmed),
        Button::outlined("Cancel", r).on_press(Message::ProjectForgetCancelled),
    ]
    .spacing(spacing::SM);

    let dialog = material::Surface::new(fields.push(actions), SurfaceKind::Dialog, r)
        .padding(spacing::LG)
        .width(Length::Fixed(460.0));

    Modal::new(dialog, r).progress(progress).into()
}
