//! The confirm-forget dialog for a project, rendered as a Material modal overlay (feature 014,
//! FR-002/FR-002a). Forgetting discards only what the application remembers about the project —
//! its catalog entry and per-project state (name, worktree-name overrides, session records).
//! Nothing on disk (the folder, its files, or any git worktrees) is deleted; the dialog says so.

use crate::app::Message;
use crate::tokens::{self, spacing, type_scale};
use crate::ui::material::Modal;
use crate::ui::style;
use iced::widget::{button, column, container, row, text};
use iced::{Element, Length};
use micold_core::theme::ColorScheme;

/// Stack the confirm-forget dialog for the project shown by `display_name` as a modal over
/// `base`, at transition `progress` (1.0 = fully shown, 0.0 = hidden — see [`Modal`]).
///
/// `running_sessions` is the number of the project's currently-running sessions that will be
/// stopped on confirm (FR-002a). When it is `0`, no session-stop line is shown; when it is `> 0`,
/// the dialog states how many sessions will be stopped, matching what the binary actually
/// terminates (a running session has a live process; idle/absent ones do not).
pub fn modal<'a>(
    base: Element<'a, Message>,
    display_name: &str,
    running_sessions: usize,
    scheme: ColorScheme,
    progress: f32,
) -> Element<'a, Message> {
    let r = tokens::roles(scheme);

    let mut fields = column![
        text(format!("Forget “{display_name}”?")).size(type_scale::HEADLINE),
        text(
            "This removes the project from your list and discards what the app remembers about it \
             (its name, worktree-name overrides, and session records). Nothing on disk is deleted \
             — the folder, its files, and any git worktrees are left untouched."
        )
        .size(type_scale::BODY)
        .style(style::muted(r)),
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
            text(format!("This will stop {running_sessions} running {noun}."))
                .size(type_scale::BODY)
                .style(style::muted(r)),
        );
    }

    let actions = row![
        button(text("Forget").size(type_scale::BODY))
            .on_press(Message::ProjectForgetConfirmed)
            .style(style::filled(r)),
        button(text("Cancel").size(type_scale::BODY))
            .on_press(Message::ProjectForgetCancelled)
            .style(style::outlined(r)),
    ]
    .spacing(spacing::SM);

    let dialog = container(fields.push(actions))
        .padding(spacing::LG)
        .width(Length::Fixed(460.0))
        .style(style::dialog(r));

    Modal::new(base, dialog, r).progress(progress).into()
}
