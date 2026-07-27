//! The About dialog, rendered as a Material modal overlay within the main window (FR-013).

use crate::app::Message;
use micold_core::tokens::{self, spacing, type_scale};
use crate::ui::cdk::overlay::Surface;
use crate::ui::material::Modal;
use crate::ui::style;
use iced::widget::{button, column, container, text};
use micold_core::metadata::AppMetadata;
use micold_core::theme::ColorScheme;

/// The About dialog as a modal surface, at transition `progress`
/// (1.0 = fully shown, 0.0 = hidden — see [`Modal`]).
///
/// The overlay captures all input while shown, so the content beneath is non-interactive
/// (FR-013). Dismissal is via the Close button (FR-010), Esc (FR-011), or a click on the dimmed
/// scrim — the last of these is new in feature 017, which unified dismissal across every floating
/// surface (FR-009, FR-024).
pub fn modal<'a>(scheme: ColorScheme, progress: f32) -> Option<Surface<'a, Message>> {
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

    Modal::new(dialog, r).progress(progress).into()
}
