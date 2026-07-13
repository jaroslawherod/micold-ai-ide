//! The main content area beneath the toolbar: the active-project indicator, the empty
//! state, and the known-projects list (reopen / last-active / unavailable).

use iced::widget::{button, column, container, row, text};
use iced::{Element, Length};
use micold_ai_ide::app::{Message, State};
use micold_ai_ide::project::Availability;

/// Render the shell body for the current workspace state.
pub fn view(state: &State) -> Element<'_, Message> {
    // Header: the active project (FR-014, FR-015) or the empty state (FR-016).
    let header = match state.workspace.active_project() {
        Some(project) => column![
            text(format!("Active project: {}", project.display_name)).size(20),
            text(project.path.display().to_string()).size(12),
            button(text("Open another project")).on_press(Message::ProjectSelectorOpened),
        ]
        .spacing(8),
        None => column![
            text("No project open").size(20),
            text("Open a folder to set it as your working space."),
            button(text("Open a project")).on_press(Message::ProjectSelectorOpened),
        ]
        .spacing(8),
    };

    let mut body = column![header].spacing(20);

    // Known-projects list: reopen without browsing (FR-011); mark the active one
    // (FR-010) and unavailable folders, blocking their reopen (FR-022, FR-023).
    if !state.workspace.projects.is_empty() {
        let active = state.workspace.active.clone();
        let mut list = column![text("Known projects").size(16)].spacing(4);

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

            let reopen = button(text(if available { "Open" } else { "Unavailable" }))
                .on_press_maybe(
                    available.then(|| Message::KnownProjectReopened(project.path.clone())),
                );
            // Renaming affects only the stored name, so it is allowed even when the folder
            // is unavailable (FR-017, FR-018).
            let rename =
                button(text("Rename")).on_press(Message::RenameStarted(project.path.clone()));

            // Git repositories carry a "git" badge in the known list too (FR-006).
            let mut entry = row![text(label).width(Length::Fill)].spacing(8);
            if project.is_git_repo {
                entry = entry.push(text("git").size(12));
            }
            list = list.push(entry.push(reopen).push(rename));
        }
        body = body.push(list);
    }

    container(body)
        .padding(24)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
