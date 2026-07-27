//! The in-app project selector (folder browser), rendered as a Material modal overlay
//! within the main window (research R3). Lists the current directory's subfolders, supports
//! navigation, and lets the user open the current folder as a project.

use crate::app::Message;
use crate::icons::{icon_role, Icon, IconSurface};
use crate::ui::material::{
    self, Button, ButtonVariant, Glyph, Scrollable, SurfaceKind, Text, TypeRole,
};
use iced::widget::{column, row};
use iced::{Alignment, Element, Length};
use micold_core::selector::{Selector, SelectorStatus};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, spacing};

/// The folder browser as the dialog body; `ui::view` wraps it in the shared
/// [`Modal`](crate::ui::material::Modal) transition.
pub fn modal<'a>(selector: &'a Selector, scheme: ColorScheme) -> Element<'a, Message> {
    let r = tokens::roles(scheme);
    let on_surface_tint = icon_role(IconSurface::AppBarAction, r);
    let on_primary_tint = icon_role(IconSurface::PrimaryButton, r);
    let badge_tint = icon_role(IconSurface::Badge, r);

    let header = row![
        Button::with_content(
            row![
                Glyph::new(Icon::NavigateUp, TypeRole::Body, r).tint(on_surface_tint),
                Text::new("Up", TypeRole::Body, r),
            ]
            .spacing(spacing::XS)
            .align_y(Alignment::Center),
            ButtonVariant::Outlined,
            r
        )
        .on_press(Message::SelectorNavigatedUp),
        Text::new(
            selector.current_dir.display().to_string(),
            TypeRole::Label,
            r
        )
        .muted(),
    ]
    .spacing(spacing::SM)
    .align_y(iced::Alignment::Center);

    let body: Element<'a, Message> = match &selector.status {
        SelectorStatus::Loading => Text::new("Loading…", TypeRole::Body, r).into(),
        SelectorStatus::Error(message) => Text::new(
            format!("Cannot read this folder: {message}"),
            TypeRole::Body,
            r,
        )
        .muted()
        .into(),
        SelectorStatus::Ready => {
            let mut list = column![].spacing(spacing::XS);
            if selector.entries.is_empty() {
                list = list.push(Text::new("(no subfolders here)", TypeRole::Body, r).muted());
            } else {
                for entry in &selector.entries {
                    // Git repositories are marked with a "git" badge (FR-006).
                    let mut label =
                        row![Text::new(entry.name.clone(), TypeRole::Body, r).width(Length::Fill)]
                            .spacing(spacing::SM)
                            .align_y(iced::Alignment::Center);
                    if entry.is_git_repo {
                        label = label.push(
                            row![
                                Glyph::new(Icon::Git, TypeRole::Label, r).tint(badge_tint),
                                Text::new("git", TypeRole::Label, r).muted(),
                            ]
                            .spacing(spacing::XS)
                            .align_y(Alignment::Center),
                        );
                    }
                    list = list.push(
                        Button::with_content(label, ButtonVariant::Text, r)
                            .on_press(Message::SelectorNavigatedInto(entry.path.clone()))
                            .width(Length::Fill),
                    );
                }
            }
            Scrollable::new(list, r).height(Length::Fill).into()
        }
    };

    let actions = row![
        Button::with_content(
            row![
                Glyph::new(Icon::OpenProject, TypeRole::Body, r).tint(on_primary_tint),
                Text::new("Open this folder", TypeRole::Body, r),
            ]
            .spacing(spacing::XS)
            .align_y(Alignment::Center),
            ButtonVariant::Filled,
            r
        )
        .on_press(Message::FolderChosen(selector.current_dir.clone())),
        Button::outlined("Cancel", r).on_press(Message::ProjectSelectorClosed),
    ]
    .spacing(spacing::SM);

    let dialog = material::Surface::new(
        column![
            Text::new("Open a project", TypeRole::Headline, r),
            header,
            body,
            actions
        ]
        .spacing(spacing::MD),
        SurfaceKind::Dialog,
        r,
    )
    .padding(spacing::MD)
    .width(Length::Fixed(560.0))
    .height(Length::Fixed(420.0));

    dialog.into()
}
