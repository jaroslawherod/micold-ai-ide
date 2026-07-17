//! The About dialog, rendered as a Material modal overlay within the main window (FR-013).

use crate::ui::material::Modal;
use crate::ui::style;
use iced::widget::{button, column, container, text};
use iced::Element;
use micold_ai_ide::app::Message;
use micold_ai_ide::metadata::AppMetadata;
use micold_ai_ide::theme::ColorScheme;
use micold_ai_ide::tokens::{self, spacing, type_scale};

/// Stack the About dialog as a modal overlay on top of `base`, at transition `progress`
/// (1.0 = fully shown, 0.0 = hidden — see [`Modal`]).
///
/// The overlay captures all input while shown, so the content beneath is non-interactive
/// (FR-013). Dismissal is via the Close button (FR-010) or Esc (FR-011) — clicking the dimmed
/// backdrop does not dismiss.
pub fn modal(
    base: Element<'_, Message>,
    scheme: ColorScheme,
    progress: f32,
) -> Element<'_, Message> {
    let r = tokens::roles(scheme);
    let meta = AppMetadata::from_env();

    let dialog = container(
        column![
            text(meta.name).size(type_scale::HEADLINE),
            text(format!("Version {}", meta.version))
                .size(type_scale::LABEL)
                .style(style::muted(r)),
            text(format!("License: {}", meta.license))
                .size(type_scale::LABEL)
                .style(style::muted(r)),
            text(meta.description).size(type_scale::BODY),
            button(text("Close").size(type_scale::BODY))
                .on_press(Message::AboutClosed)
                .style(style::filled(r)),
        ]
        .spacing(spacing::MD),
    )
    .padding(spacing::LG)
    .style(style::dialog(r));

    Modal::new(base, dialog, r).progress(progress).into()
}
