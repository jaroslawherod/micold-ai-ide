//! The confirm-remove dialog for a session, rendered as a Material modal overlay (bugfix
//! BUG-003, FR-015c). Removing is permanent — unlike Close, there is no marker-based recovery
//! path back into the sidebar (the removed session's record is dropped outright).

use crate::app::Message;
use crate::ui::material::{self, Button, SurfaceKind, Text, TypeRole};
use iced::widget::{column, row};
use iced::Element;
use iced::Length;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, spacing};

/// The confirm-remove dialog for a session (shown by its sidebar `label`) as a modal surface, as the dialog body; `ui::view` wraps it
/// in the shared [`Modal`](crate::ui::material::Modal) transition.
pub fn modal<'a>(label: &str, scheme: ColorScheme) -> Element<'a, Message> {
    let r = tokens::roles(scheme);

    let warning = "This permanently deletes the session. Unlike Close, it cannot be recovered — \
                   the underlying `claude` conversation itself is untouched, but the app will \
                   never show or resume it again.";

    let fields = column![
        Text::new(format!("Remove “{label}”?"), TypeRole::Headline, r),
        Text::new(warning, TypeRole::Body, r).muted(),
    ]
    .spacing(spacing::MD);

    let actions = row![
        Button::filled("Remove", r).on_press(Message::SessionRemoveConfirmed),
        Button::outlined("Cancel", r).on_press(Message::SessionRemoveCancelled),
    ]
    .spacing(spacing::SM);

    let dialog = material::Surface::new(fields.push(actions), SurfaceKind::Dialog, r)
        .padding(spacing::LG)
        .width(Length::Fixed(460.0));

    dialog.into()
}
