//! The in-app project selector (folder browser), rendered as a modal overlay within the
//! main window (research R3). Lists the current directory's subfolders, supports
//! navigation, and lets the user open the current folder as a project. Git icons are
//! added with User Story 3.

use iced::widget::{button, center, column, container, opaque, row, scrollable, stack, text};
use iced::{Color, Element, Length};
use micold_ai_ide::app::Message;
use micold_ai_ide::selector::{Selector, SelectorStatus};

/// Stack the folder browser as a modal overlay on top of `base`.
pub fn modal<'a>(base: Element<'a, Message>, selector: &'a Selector) -> Element<'a, Message> {
    let header = row![
        button(text("↑ Up")).on_press(Message::SelectorNavigatedUp),
        text(selector.current_dir.display().to_string()),
    ]
    .spacing(8);

    let body: Element<'a, Message> = match &selector.status {
        SelectorStatus::Loading => text("Loading…").into(),
        SelectorStatus::Error(message) => {
            text(format!("Cannot read this folder: {message}")).into()
        }
        SelectorStatus::Ready => {
            let mut list = column![].spacing(2);
            if selector.entries.is_empty() {
                list = list.push(text("(no subfolders here)"));
            } else {
                for entry in &selector.entries {
                    // Git repositories are marked with a "git" badge (FR-006).
                    let mut label = row![text(entry.name.clone()).width(Length::Fill)].spacing(6);
                    if entry.is_git_repo {
                        label = label.push(text("git").size(12));
                    }
                    list = list.push(
                        button(label)
                            .on_press(Message::SelectorNavigatedInto(entry.path.clone()))
                            .width(Length::Fill),
                    );
                }
            }
            scrollable(list).height(Length::Fill).into()
        }
    };

    let actions = row![
        button(text("Open this folder"))
            .on_press(Message::FolderChosen(selector.current_dir.clone())),
        button(text("Cancel")).on_press(Message::ProjectSelectorClosed),
    ]
    .spacing(8);

    let dialog =
        container(column![text("Open a project").size(20), header, body, actions].spacing(12))
            .padding(20)
            .width(Length::Fixed(560.0))
            .height(Length::Fixed(420.0))
            .style(container::rounded_box);

    let backdrop = center(dialog).style(|_theme| container::Style {
        background: Some(
            Color {
                a: 0.6,
                ..Color::BLACK
            }
            .into(),
        ),
        ..container::Style::default()
    });

    stack![base, opaque(backdrop)].into()
}
