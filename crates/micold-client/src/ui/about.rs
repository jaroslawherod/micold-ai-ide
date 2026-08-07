//! The About dialog, rendered as a Material modal overlay within the main window (FR-013).

use crate::app::Message;
use crate::ui::material::{Button, SurfaceKind, Text, TypeRole};
use iced::widget::column;
use iced::Element;
use micold_core::metadata::AppMetadata;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self};

/// The About dialog as the dialog body; `ui::view` wraps it in the shared
/// [`Modal`](crate::ui::material::Modal) transition.
///
/// The overlay captures all input while shown, so the content beneath is non-interactive
/// (FR-013). Dismissal is via the Close button (FR-010), Esc (FR-011), or a click on the dimmed
/// scrim — the last of these is new in feature 017, which unified dismissal across every floating
/// surface (FR-009, FR-024).
pub fn modal<'a>(scheme: ColorScheme) -> Element<'a, Message> {
    let r = tokens::roles(scheme);
    let meta = AppMetadata::from_env();

    // The About box's single action is the last line of its column rather than a row of its own,
    // so it takes §7.4's body-to-actions gap the same way a two-button dialog does.
    let dialog = crate::ui::material::Surface::new(
        crate::ui::material::dialog::body(
            crate::ui::material::dialog::fields(column![
                Text::new(meta.name, TypeRole::Headline, r),
                Text::new(format!("Version {}", meta.version), TypeRole::Caption, r).muted(),
                Text::new(format!("License: {}", meta.license), TypeRole::Caption, r).muted(),
                Text::new(meta.description, TypeRole::Body, r),
            ]),
            Button::filled("Close", r).on_press(Message::AboutClosed),
        ),
        SurfaceKind::Dialog,
        r,
    );

    dialog.into()
}
