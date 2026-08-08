//! The main content area beneath the app bar: the active-project surface, the empty
//! state, and the known-projects list (reopen / last-active / unavailable), all rendered
//! as Material surfaces from the active scheme's design tokens.

use crate::app::{Message, State};
use crate::icons::{icon_role, Icon, IconSurface};
use crate::ui::material::{self, Button, Glyph, IconLabel, SurfaceKind, Text, TypeRole};
use iced::widget::{column, container, row};
use iced::{Alignment, Element, Length};
use micold_core::project::Availability;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, spacing};

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
        Some(project) => material::Surface::new(
            column![
                Text::new(
                    format!("Active project: {}", project.display_name),
                    TypeRole::Headline,
                    r
                ),
                Text::new(project.path.display().to_string(), TypeRole::Caption, r).muted(),
                Button::outlined("Open another project", r)
                    .leading(Icon::OpenProject, on_surface_tint)
                    .on_press(Message::ProjectSelectorOpened),
            ]
            .spacing(spacing::SM),
            SurfaceKind::Plain,
            r,
        )
        .padding(spacing::LG)
        .width(Length::Fill)
        .into(),
        None => material::Surface::new(
            column![
                Text::new("No project open", TypeRole::Display, r),
                Text::new(
                    "Open a folder to set it as your working space.",
                    TypeRole::Body,
                    r
                )
                .muted(),
                Button::filled("Open a project", r)
                    .leading(Icon::OpenProject, on_primary_tint)
                    .on_press(Message::ProjectSelectorOpened),
            ]
            .spacing(spacing::MD),
            SurfaceKind::Plain,
            r,
        )
        .padding(spacing::LG)
        .width(Length::Fill)
        .into(),
    };

    let mut body = column![].spacing(spacing::LG);

    // The background-restart return notice (feature 008, FR-011 / SC-007) used to be drawn
    // here. It never appeared: this function is the *else* branch of
    // `if state.active_session.is_some()`, and returning to a project restores its foreground
    // session, so the branch was not taken in the one case the banner existed for. It is now
    // an ordinary entry on the global notification surface in `ui::view`.
    body = body.push(header);

    // Known-projects list: reopen without browsing (FR-011); mark the active one (FR-010)
    // and unavailable folders, blocking their reopen (FR-022, FR-023).
    if !state.workspace.projects.is_empty() {
        let active = state.workspace.active.clone();
        let mut list =
            column![Text::new("Known projects", TypeRole::Section, r)].spacing(spacing::SM);

        for project in &state.workspace.projects {
            let is_active = active.as_ref() == Some(&project.path);
            let available = project.availability == Availability::Available;

            // The active project is marked with a check icon; unavailable folders with an
            // error icon — both replacing the former text decorations (FR-005).
            let reopen = if available {
                Button::filled("Open", r)
                    .leading(Icon::OpenProject, on_primary_tint)
                    .on_press(Message::KnownProjectReopened(project.path.clone()))
            } else {
                Button::filled("Unavailable", r)
            };

            // Renaming affects only the stored name, so it is allowed even when the folder
            // is unavailable (FR-017, FR-018).
            let rename = Button::outlined("Rename", r)
                .leading(Icon::Rename, on_surface_tint)
                .on_press(Message::RenameStarted(project.path.clone()));

            // Forget removes the project (and its remembered metadata) from the list. Enabled for
            // every entry — including Unavailable ones, for which it is the primary way to clear a
            // stale entry (feature 014, FR-001/FR-011). The trash icon is error-tinted to signal
            // the destructive-to-metadata action; the confirmation dialog is the real safeguard.
            let forget = Button::outlined("Forget", r)
                .leading(Icon::Delete, error_tint)
                .on_press(Message::ProjectForgetRequested(project.path.clone()));

            let mut entry = row![].spacing(spacing::SM).align_y(Alignment::Center);
            if is_active {
                entry =
                    entry.push(Glyph::new(Icon::ActiveMarker, TypeRole::Body, r).tint(badge_tint));
            }
            if !available {
                entry =
                    entry.push(Glyph::new(Icon::Unavailable, TypeRole::Body, r).tint(error_tint));
            }
            entry = entry.push(
                Text::new(project.display_name.clone(), TypeRole::Body, r).width(Length::Fill),
            );

            // Git repositories carry a "git" badge in the known list too (FR-006).
            if project.is_git_repo {
                entry = entry.push(
                    IconLabel::new(Icon::Git, "git", TypeRole::Label, r)
                        .tint(badge_tint)
                        .muted(),
                );
            }

            let entry = entry.push(reopen).push(rename).push(forget);
            list = list.push(
                material::Surface::new(entry, SurfaceKind::ListItem, r)
                    .padding(spacing::MD)
                    .width(Length::Fill),
            );
        }
        body = body.push(list);
    }

    // A plain layout container: padding and size only, no surface of its own (FR-003).
    container(body)
        .padding(spacing::LG)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
