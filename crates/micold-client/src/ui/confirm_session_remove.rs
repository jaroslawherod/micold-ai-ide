//! The confirm-remove dialog for a session, rendered as a Material modal overlay (bugfix
//! BUG-003, FR-015c). Removing is permanent — unlike Close, there is no marker-based recovery
//! path back into the sidebar (the removed session's record is dropped outright).

use crate::app::Message;
use micold_core::tokens::{self, spacing, type_scale};
use crate::ui::cdk::overlay::Surface;
use crate::ui::material::Modal;
use crate::ui::style;
use iced::widget::{button, column, container, row, text};
use iced::Length;
use micold_core::theme::ColorScheme;

/// The confirm-remove dialog for a session (shown by its sidebar `label`) as a modal surface,
/// at transition `progress` (1.0 = fully shown, 0.0 = hidden — see [`Modal`]).
pub fn modal<'a>(
    label: &str,
    scheme: ColorScheme,
    progress: f32,
) -> Option<Surface<'a, Message>> {
    let r = tokens::roles(scheme);

    let warning = "This permanently deletes the session. Unlike Close, it cannot be recovered — \
                   the underlying `claude` conversation itself is untouched, but the app will \
                   never show or resume it again."
        .to_string();

    let fields = column![
        text(format!("Remove “{label}”?")).size(type_scale::HEADLINE),
        text(warning).size(type_scale::BODY).style(style::muted(r)),
    ]
    .spacing(spacing::MD);

    let actions = row![
        button(text("Remove").size(type_scale::BODY))
            .on_press(Message::SessionRemoveConfirmed)
            .style(style::filled(r)),
        button(text("Cancel").size(type_scale::BODY))
            .on_press(Message::SessionRemoveCancelled)
            .style(style::outlined(r)),
    ]
    .spacing(spacing::SM);

    let dialog = container(fields.push(actions))
        .padding(spacing::LG)
        .width(Length::Fixed(460.0))
        .style(style::dialog(r));

    Modal::new(dialog, r).progress(progress).into()
}
