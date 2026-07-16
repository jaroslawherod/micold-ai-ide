//! The Settings dialog, rendered as a Material modal overlay within the main window (feature
//! 006, FR-019/FR-020). Currently exposes the embedded-terminal scrollback limit.

use crate::ui::style;
use iced::widget::{button, center, column, container, opaque, row, stack, text, text_input};
use iced::{Element, Length};
use micold_ai_ide::app::{Message, SettingsDraft};
use micold_ai_ide::theme::ColorScheme;
use micold_ai_ide::tokens::{self, spacing, type_scale};

/// Stack the Settings dialog as a modal overlay on top of `base`.
pub fn modal<'a>(
    base: Element<'a, Message>,
    draft: &'a SettingsDraft,
    scheme: ColorScheme,
) -> Element<'a, Message> {
    let r = tokens::roles(scheme);

    let input = text_input("Scrollback lines", &draft.scrollback_lines)
        .on_input(Message::SettingsScrollbackChanged)
        .on_submit(Message::SettingsSaved)
        .padding(spacing::SM)
        .style(style::input(r));

    let mut fields = column![
        text("Settings").size(type_scale::HEADLINE),
        text("Terminal scrollback limit (lines)")
            .size(type_scale::LABEL)
            .style(style::muted(r)),
        input,
    ]
    .spacing(spacing::MD);

    if let Some(error) = &draft.error {
        let error = error.clone();
        fields = fields.push(text(error).size(type_scale::LABEL).style(
            move |_theme: &iced::Theme| iced::widget::text::Style {
                color: Some(style::color(r.error)),
            },
        ));
    }

    let actions = row![
        button(text("Save").size(type_scale::BODY))
            .on_press(Message::SettingsSaved)
            .style(style::filled(r)),
        button(text("Cancel").size(type_scale::BODY))
            .on_press(Message::SettingsCancelled)
            .style(style::outlined(r)),
    ]
    .spacing(spacing::SM);

    let dialog = container(fields.push(actions))
        .padding(spacing::LG)
        .width(Length::Fixed(420.0))
        .style(style::dialog(r));

    let backdrop = center(dialog).style(style::backdrop());

    stack![base, opaque(backdrop)].into()
}
