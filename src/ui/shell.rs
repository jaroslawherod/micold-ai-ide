//! The main content area beneath the app bar: the active-project surface, the empty
//! state, and the known-projects list (reopen / last-active / unavailable), all rendered
//! as Material surfaces from the active scheme's design tokens.

use crate::ui::style;
use iced::widget::{button, column, container, row, text};
use iced::{Element, Length};
use micold_ai_ide::app::{Message, State};
use micold_ai_ide::project::Availability;
use micold_ai_ide::theme::ColorScheme;
use micold_ai_ide::tokens::{self, spacing, type_scale};

/// Render the shell body for the current workspace state.
pub fn view(state: &State, scheme: ColorScheme) -> Element<'_, Message> {
    let r = tokens::roles(scheme);

    // Header: the active project (FR-014, FR-015) or the empty state (FR-016).
    let header: Element<'_, Message> = match state.workspace.active_project() {
        Some(project) => container(
            column![
                text(format!("Active project: {}", project.display_name))
                    .size(type_scale::HEADLINE),
                text(project.path.display().to_string())
                    .size(type_scale::LABEL)
                    .style(style::muted(r)),
                button(text("Open another project").size(type_scale::BODY))
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
                button(text("Open a project").size(type_scale::BODY))
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

            let mut label = project.display_name.clone();
            if is_active {
                label = format!("● {label}");
            }
            if !available {
                label = format!("{label}  (unavailable)");
            }

            let reopen =
                button(text(if available { "Open" } else { "Unavailable" }).size(type_scale::BODY))
                    .on_press_maybe(
                        available.then(|| Message::KnownProjectReopened(project.path.clone())),
                    )
                    .style(style::filled(r));

            // Renaming affects only the stored name, so it is allowed even when the folder
            // is unavailable (FR-017, FR-018).
            let rename = button(text("Rename").size(type_scale::BODY))
                .on_press(Message::RenameStarted(project.path.clone()))
                .style(style::outlined(r));

            // Git repositories carry a "git" badge in the known list too (FR-006).
            let mut entry = row![text(label).size(type_scale::BODY).width(Length::Fill)]
                .spacing(spacing::SM)
                .align_y(iced::Alignment::Center);
            if project.is_git_repo {
                entry = entry.push(text("git").size(type_scale::LABEL).style(style::muted(r)));
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
