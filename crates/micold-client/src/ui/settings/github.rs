//! The GitHub issues section — which worktree type an issue's labels choose (feature 034, US3;
//! contracts/issue-picker-ui.md §4).
//!
//! One row per mapping entry, in the order that decides which entry wins (FR-017): the label as
//! typed, the type it selects, and the buttons that move or delete that entry. The draft holds the
//! labels as typed; a blank or repeated one is refused by the view's own Save and marked on its row
//! (FR-019), the same save-together rule every other section follows.

use crate::app::Message;
use crate::features::settings::Msg as SettingsMsg;
use crate::features::settings::{SettingsDraft, SettingsSection};
use crate::features::window::FieldId;
use crate::icons::Icon;
use crate::ui::focus::TrackFocus;
use crate::ui::material::{Button, IconButton, Select, TextField, Tooltip};
use crate::ui::settings::{note, page};
use iced::widget::{container, row};
use iced::{Alignment, Element, Length};
use micold_core::naming::ConventionalType;
use micold_core::tokens::{spacing, Roles};
use micold_core::typeahead::Direction;

/// What this section renders. See [`crate::ui::settings`].
// Read by `tests/settings_sections.rs`, which is a separate crate and cannot be seen from here —
// so to the compiler this is unused. Deleting it would take the gate's evidence with it.
#[allow(dead_code)]
pub const SETTINGS: &[(&str, &str)] = &[("issue_label_types", "IssueMappingLabelChanged")];

/// The GitHub issues page.
pub fn view<'a>(
    draft: &'a SettingsDraft,
    focused: Option<FieldId>,
    roles: Roles,
) -> Element<'a, Message> {
    let entries = &draft.github.entries;
    let last = entries.len().saturating_sub(1);

    let mut controls: Vec<Element<'a, Message>> = Vec::with_capacity(entries.len() + 2);
    if entries.is_empty() {
        controls.push(note(
            "No labels are mapped — picking an issue leaves the type for you to choose.",
            roles,
        ));
    }
    for (index, entry) in entries.iter().enumerate() {
        let label = TextField::new("", &entry.label, roles)
            .label("Label")
            .error(super::error_for(
                draft,
                SettingsSection::GithubIssues,
                FieldId::IssueMappingLabel(index),
            ))
            .track_focus(FieldId::IssueMappingLabel(index), focused)
            .on_input(move |v| Message::Settings(SettingsMsg::IssueMappingLabelChanged(index, v)))
            .on_submit(Message::Settings(SettingsMsg::Saved));

        let type_ = Select::new(
            ConventionalType::ALL,
            Some(entry.type_),
            move |t| Message::Settings(SettingsMsg::IssueMappingTypeChanged(index, t)),
            roles,
        )
        .label("Type");

        // Icon-only, so each says what it does on hover, as the sidebar's icon buttons do.
        let up = Tooltip::new(
            IconButton::new(Icon::MoveUp, roles).on_press_maybe((index > 0).then_some(
                Message::Settings(SettingsMsg::IssueMappingMoved(index, Direction::Prev)),
            )),
            "Move up",
            roles,
        );
        let down = Tooltip::new(
            IconButton::new(Icon::MoveDown, roles).on_press_maybe((index < last).then_some(
                Message::Settings(SettingsMsg::IssueMappingMoved(index, Direction::Next)),
            )),
            "Move down",
            roles,
        );
        let delete = Tooltip::new(
            IconButton::new(Icon::Delete, roles)
                .on_press(Message::Settings(SettingsMsg::IssueMappingRemoved(index))),
            "Delete entry",
            roles,
        );

        controls.push(
            row![
                container(label).width(Length::FillPortion(3)),
                container(type_).width(Length::FillPortion(2)),
                up,
                down,
                delete,
            ]
            .spacing(spacing::XS)
            .align_y(Alignment::Start)
            .width(Length::Fill)
            .into(),
        );
    }

    controls.push(
        row![
            Button::text("Add entry", roles)
                .on_press(Message::Settings(SettingsMsg::IssueMappingAdded)),
            Button::text("Restore defaults", roles)
                .on_press(Message::Settings(SettingsMsg::IssueMappingDefaultsRestored)),
        ]
        .spacing(spacing::XS)
        .into(),
    );

    page(
        "GitHub issues",
        "When you create a worktree from a GitHub issue, the first entry whose label the issue \
         carries sets the worktree's type. Applies to every project.",
        controls,
        roles,
    )
}
