//! The in-app project selector (folder browser), rendered as a Material modal overlay
//! within the main window (research R3). Lists the current directory's subfolders, supports
//! navigation, and lets the user open the current folder as a project.

use crate::app::{Message, State};
use crate::icons::{icon_role, Icon, IconSurface};
use crate::ui::material::{
    self, Button, ButtonVariant, IconLabel, Scrollable, SurfaceKind, Text, TypeRole,
};
use iced::widget::{column, row};
use iced::{Element, Length};
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::selector::{Selector, SelectorStatus};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, spacing};

/// The folder browser as the dialog body; `ui::view` wraps it in the shared
/// [`Modal`](crate::ui::material::Modal) transition.
pub fn modal<'a>(selector: &'a Selector, scheme: ColorScheme) -> Element<'a, Message> {
    let r = tokens::roles(scheme);
    // No button-glyph tints: `Button::leading` takes the variant's content colour (018's BUG-007).
    let badge_tint = icon_role(IconSurface::Badge, r);

    let header = row![
        Button::outlined("Up", r)
            .leading(Icon::NavigateUp)
            .on_press(Message::SelectorNavigatedUp),
        Text::new(
            selector.current_dir.display().to_string(),
            TypeRole::Caption,
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
                            IconLabel::new(Icon::Git, "git", TypeRole::Label, r)
                                .tint(badge_tint)
                                .muted(),
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
        Button::filled("Open this folder", r)
            .leading(Icon::OpenProject)
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

/// This dialog's body, built from the state that opened it — `None` when the surface is open but
/// the live state it renders is absent, so nothing is drawn rather than an empty dialog.
///
/// The uniform shape every registered dialog has, and the reason `ui::view` no longer needs a
/// match: the registration line in [`crate::overlay::registry`] names this beside the surface it
/// draws, so a dialog says where its own state lives instead of a central arm saying it for them
/// all (feature 021, T035 — FR-008, FR-009).
pub fn dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    _env_include_outcome: &'a EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    state
        .selector
        .as_ref()
        .map(|selector| modal(selector, scheme))
}
