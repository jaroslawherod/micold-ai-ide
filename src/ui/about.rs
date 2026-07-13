//! The About dialog, rendered as a modal overlay within the main window (FR-013).

use iced::widget::{button, center, column, container, opaque, stack, text};
use iced::{Color, Element};
use micold_ai_ide::app::Message;
use micold_ai_ide::metadata::AppMetadata;

/// Stack the About dialog as a modal overlay on top of `base`.
///
/// The overlay is wrapped in `opaque`, so it captures all input and the content beneath
/// is non-interactive while the dialog is open (FR-013). Dismissal is via the Close button
/// (FR-010) or Esc (FR-011) — clicking the dimmed backdrop does not dismiss.
pub fn modal(base: Element<'_, Message>) -> Element<'_, Message> {
    let meta = AppMetadata::from_env();

    let dialog = container(
        column![
            text(meta.name).size(24),
            text(format!("Version {}", meta.version)),
            text(format!("License: {}", meta.license)),
            text(meta.description),
            button(text("Close")).on_press(Message::AboutClosed),
        ]
        .spacing(12),
    )
    .padding(24)
    .style(container::rounded_box);

    let backdrop = center(dialog).style(|_theme| container::Style {
        background: Some(
            Color {
                a: 0.6,
                ..Color::BLACK
            }
            .into(),
        ),
        ..container::Style::default()
    });

    stack![base, opaque(backdrop)].into()
}
