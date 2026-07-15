//! The in-app project selector (folder browser), rendered as a Material modal overlay
//! within the main window (research R3). Lists the current directory's subfolders, supports
//! navigation, and lets the user open the current folder as a project.

use crate::ui::{icon, style};
use iced::widget::{button, center, column, container, opaque, row, scrollable, stack, text};
use iced::{Alignment, Element, Length};
use micold_ai_ide::app::Message;
use micold_ai_ide::icons::{icon_role, Icon, IconSurface};
use micold_ai_ide::selector::{Selector, SelectorStatus};
use micold_ai_ide::theme::ColorScheme;
use micold_ai_ide::tokens::{self, spacing, type_scale};

/// Stack the folder browser as a modal overlay on top of `base`.
pub fn modal<'a>(
    base: Element<'a, Message>,
    selector: &'a Selector,
    scheme: ColorScheme,
) -> Element<'a, Message> {
    let r = tokens::roles(scheme);
    let on_surface_tint = icon_role(IconSurface::AppBarAction, r);
    let on_primary_tint = icon_role(IconSurface::PrimaryButton, r);
    let badge_tint = icon_role(IconSurface::Badge, r);

    let header = row![
        button(
            row![
                icon(Icon::NavigateUp, type_scale::BODY, on_surface_tint),
                text("Up").size(type_scale::BODY),
            ]
            .spacing(spacing::XS)
            .align_y(Alignment::Center)
        )
        .on_press(Message::SelectorNavigatedUp)
        .style(style::outlined(r)),
        text(selector.current_dir.display().to_string())
            .size(type_scale::LABEL)
            .style(style::muted(r)),
    ]
    .spacing(spacing::SM)
    .align_y(iced::Alignment::Center);

    let body: Element<'a, Message> = match &selector.status {
        SelectorStatus::Loading => text("Loading…").size(type_scale::BODY).into(),
        SelectorStatus::Error(message) => text(format!("Cannot read this folder: {message}"))
            .size(type_scale::BODY)
            .style(style::muted(r))
            .into(),
        SelectorStatus::Ready => {
            let mut list = column![].spacing(spacing::XS);
            if selector.entries.is_empty() {
                list = list.push(
                    text("(no subfolders here)")
                        .size(type_scale::BODY)
                        .style(style::muted(r)),
                );
            } else {
                for entry in &selector.entries {
                    // Git repositories are marked with a "git" badge (FR-006).
                    let mut label = row![text(entry.name.clone())
                        .size(type_scale::BODY)
                        .width(Length::Fill)]
                    .spacing(spacing::SM)
                    .align_y(iced::Alignment::Center);
                    if entry.is_git_repo {
                        label = label.push(
                            row![
                                icon(Icon::Git, type_scale::LABEL, badge_tint),
                                text("git").size(type_scale::LABEL).style(style::muted(r)),
                            ]
                            .spacing(spacing::XS)
                            .align_y(Alignment::Center),
                        );
                    }
                    list = list.push(
                        button(label)
                            .on_press(Message::SelectorNavigatedInto(entry.path.clone()))
                            .width(Length::Fill)
                            .style(style::text_button(r)),
                    );
                }
            }
            scrollable(list).height(Length::Fill).into()
        }
    };

    let actions = row![
        button(
            row![
                icon(Icon::OpenProject, type_scale::BODY, on_primary_tint),
                text("Open this folder").size(type_scale::BODY),
            ]
            .spacing(spacing::XS)
            .align_y(Alignment::Center)
        )
        .on_press(Message::FolderChosen(selector.current_dir.clone()))
        .style(style::filled(r)),
        button(text("Cancel").size(type_scale::BODY))
            .on_press(Message::ProjectSelectorClosed)
            .style(style::outlined(r)),
    ]
    .spacing(spacing::SM);

    let dialog = container(
        column![
            text("Open a project").size(type_scale::HEADLINE),
            header,
            body,
            actions
        ]
        .spacing(spacing::MD),
    )
    .padding(spacing::MD)
    .width(Length::Fixed(560.0))
    .height(Length::Fixed(420.0))
    .style(style::dialog(r));

    let backdrop = center(dialog).style(style::backdrop());

    stack![base, opaque(backdrop)].into()
}
