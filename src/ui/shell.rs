//! The main content area beneath the app bar: the active-project surface, the empty
//! state, and the known-projects list (reopen / last-active / unavailable), all rendered
//! as Material surfaces from the active scheme's design tokens.

use crate::ui::{icon, style};
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length};
use micold_ai_ide::app::{Message, State};
use micold_ai_ide::icons::{icon_role, Icon, IconSurface};
use micold_ai_ide::project::Availability;
use micold_ai_ide::theme::ColorScheme;
use micold_ai_ide::tokens::{self, spacing, type_scale, Rgb};

/// An icon + text label laid out as a horizontal button/badge content.
fn labeled<'a>(glyph: Icon, tint: Rgb, size: u16, label: &str) -> iced::widget::Row<'a, Message> {
    row![icon(glyph, size, tint), text(label.to_string()).size(size)]
        .spacing(spacing::XS)
        .align_y(Alignment::Center)
}

/// Render the shell body for the current workspace state.
pub fn view(state: &State, scheme: ColorScheme) -> Element<'_, Message> {
    let r = tokens::roles(scheme);

    // Foreground tints per surface context (FR-004, FR-007).
    let on_surface_tint = icon_role(IconSurface::AppBarAction, r);
    let on_primary_tint = icon_role(IconSurface::PrimaryButton, r);
    let badge_tint = icon_role(IconSurface::Badge, r);
    let error_tint = icon_role(IconSurface::Unavailable, r);

    // Header: the active project (FR-014, FR-015) or the empty state (FR-016).
    let header: Element<'_, Message> = match state.workspace.active_project() {
        Some(project) => container(
            column![
                text(format!("Active project: {}", project.display_name))
                    .size(type_scale::HEADLINE),
                text(project.path.display().to_string())
                    .size(type_scale::LABEL)
                    .style(style::muted(r)),
                button(labeled(
                    Icon::OpenProject,
                    on_surface_tint,
                    type_scale::BODY,
                    "Open another project"
                ))
                .on_press(Message::ProjectSelectorOpened)
                .style(style::outlined(r)),
            ]
            .spacing(spacing::SM),
        )
        .padding(spacing::LG)
        .width(Length::Fill)
        .style(style::surface(r))
        .into(),
        None => container(
            column![
                text("No project open").size(type_scale::DISPLAY),
                text("Open a folder to set it as your working space.")
                    .size(type_scale::BODY)
                    .style(style::muted(r)),
                button(labeled(
                    Icon::OpenProject,
                    on_primary_tint,
                    type_scale::BODY,
                    "Open a project"
                ))
                .on_press(Message::ProjectSelectorOpened)
                .style(style::filled(r)),
            ]
            .spacing(spacing::MD),
        )
        .padding(spacing::LG)
        .width(Length::Fill)
        .style(style::surface(r))
        .into(),
    };

    let mut body = column![header].spacing(spacing::LG);

    // Known-projects list: reopen without browsing (FR-011); mark the active one (FR-010)
    // and unavailable folders, blocking their reopen (FR-022, FR-023).
    if !state.workspace.projects.is_empty() {
        let active = state.workspace.active.clone();
        let mut list = column![text("Known projects").size(type_scale::TITLE)].spacing(spacing::SM);

        for project in &state.workspace.projects {
            let is_active = active.as_ref() == Some(&project.path);
            let available = project.availability == Availability::Available;

            // The active project is marked with a check icon; unavailable folders with an
            // error icon — both replacing the former text decorations (FR-005).
            let reopen = if available {
                button(labeled(
                    Icon::OpenProject,
                    on_primary_tint,
                    type_scale::BODY,
                    "Open",
                ))
                .on_press(Message::KnownProjectReopened(project.path.clone()))
                .style(style::filled(r))
            } else {
                button(text("Unavailable").size(type_scale::BODY)).style(style::filled(r))
            };

            // Renaming affects only the stored name, so it is allowed even when the folder
            // is unavailable (FR-017, FR-018).
            let rename = button(labeled(
                Icon::Rename,
                on_surface_tint,
                type_scale::BODY,
                "Rename",
            ))
            .on_press(Message::RenameStarted(project.path.clone()))
            .style(style::outlined(r));

            let mut entry = row![].spacing(spacing::SM).align_y(Alignment::Center);
            if is_active {
                entry = entry.push(icon(Icon::ActiveMarker, type_scale::BODY, badge_tint));
            }
            if !available {
                entry = entry.push(icon(Icon::Unavailable, type_scale::BODY, error_tint));
            }
            entry = entry.push(
                text(project.display_name.clone())
                    .size(type_scale::BODY)
                    .width(Length::Fill),
            );

            // Git repositories carry a "git" badge in the known list too (FR-006).
            if project.is_git_repo {
                entry = entry.push(
                    row![
                        icon(Icon::Git, type_scale::LABEL, badge_tint),
                        text("git").size(type_scale::LABEL).style(style::muted(r)),
                    ]
                    .spacing(spacing::XS)
                    .align_y(Alignment::Center),
                );
            }

            let entry = entry.push(reopen).push(rename);
            list = list.push(
                container(entry)
                    .padding(spacing::MD)
                    .width(Length::Fill)
                    .style(style::list_item(r)),
            );
        }
        body = body.push(list);
    }

    container(body)
        .padding(spacing::LG)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
