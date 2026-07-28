//! The interactive controls (feature 020, T019): buttons, a checkbox, a chip, a field, a dropdown.
//!
//! This is the section US2's independent test walks and the one feature 018's SC-002/SC-004 need: a
//! row of every interactive component in the library, all hoverable and pressable in one pass. Every
//! instance here is real and live — none of hover, pressed or focus is faked (FR-004), which is why
//! each of these entries declares them as `live` rather than posing them.
//!
//! Where a control needs a message it has nowhere to send, it sends [`Message::NoOp`]. That keeps the
//! instance genuinely interactive without the gallery inventing behaviour the application owns.

use iced::Element;
use micold_core::tokens::Roles;

use crate::icons::Icon;
use crate::showcase::catalogue::Layout;
use crate::showcase::gallery::{arrange, posed};
use crate::showcase::samples;
use crate::showcase::state::{Message, Showcase};
use crate::ui::material::{self, ButtonVariant, TypeRole};

/// How tall the resize handle's swatch is. A layout dimension, not a text size (see `atoms.rs`).
const HANDLE_HEIGHT: f32 = 96.0;

/// `Button` — every variant, enabled and disabled.
///
/// Disabled is posed by withholding the press message rather than by setting a flag, which is how the
/// component itself expresses it: "this action is unavailable" is having nothing to send.
pub fn button<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    let variants: &[(&str, ButtonVariant)] = &[
        ("Filled", ButtonVariant::Filled),
        ("Outlined", ButtonVariant::Outlined),
        ("Text", ButtonVariant::Text),
    ];
    let mut instances: Vec<Element<'a, Message>> = Vec::new();
    for (label, variant) in variants {
        instances.push(posed(
            label,
            material::Button::with_content(
                material::Text::new(samples::LABEL, TypeRole::Body, roles),
                *variant,
                roles,
            )
            .on_press(Message::NoOp),
            roles,
        ));
    }
    for (label, variant) in variants {
        instances.push(posed(
            label,
            // No `on_press`: disabled, the way the component says it.
            material::Button::<Message>::with_content(
                material::Text::new(samples::OTHER_LABEL, TypeRole::Body, roles),
                *variant,
                roles,
            ),
            roles,
        ));
    }
    arrange(instances, Layout::Inline)
}

/// `IconButton` — square and circular, sized at two roles, enabled and disabled.
pub fn icon_button<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    arrange(
        vec![
            posed(
                "default",
                material::IconButton::new(Icon::Settings, roles).on_press(Message::NoOp),
                roles,
            ),
            posed(
                "circular",
                material::IconButton::new(Icon::AddSession, roles)
                    .circular()
                    .on_press(Message::NoOp),
                roles,
            ),
            posed(
                "at the title size",
                material::IconButton::new(Icon::Menu, roles)
                    .size(TypeRole::Title)
                    .on_press(Message::NoOp),
                roles,
            ),
            posed(
                "tinted",
                material::IconButton::new(Icon::Git, roles)
                    .tint(roles.primary)
                    .on_press(Message::NoOp),
                roles,
            ),
            posed(
                "disabled",
                material::IconButton::<Message>::new(Icon::Delete, roles),
                roles,
            ),
        ],
        Layout::Inline,
    )
}

/// `Checkbox` — checked and unchecked, and one with no toggle message (disabled).
pub fn checkbox<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    arrange(
        vec![
            posed(
                "unchecked",
                material::Checkbox::new(samples::LABEL, false, roles).on_toggle(|_| Message::NoOp),
                roles,
            ),
            posed(
                "checked",
                material::Checkbox::new(samples::LABEL, true, roles).on_toggle(|_| Message::NoOp),
                roles,
            ),
            posed(
                "disabled",
                material::Checkbox::<Message>::new(samples::OTHER_LABEL, false, roles),
                roles,
            ),
        ],
        Layout::Inline,
    )
}

/// `ToggleChip` — active and inactive, and with an explicit accent.
pub fn toggle_chip<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    arrange(
        vec![
            posed(
                "inactive",
                material::ToggleChip::new(samples::TAG, Message::NoOp, roles).active(false),
                roles,
            ),
            posed(
                "active",
                material::ToggleChip::new(samples::TAG, Message::NoOp, roles).active(true),
                roles,
            ),
            posed(
                "accented, active",
                material::ToggleChip::new("fix", Message::NoOp, roles)
                    .active(true)
                    .accent(roles.tag_fix, roles.on_tag),
                roles,
            ),
        ],
        Layout::Inline,
    )
}

/// `TextField` — empty (showing its placeholder) and filled.
pub fn text_field<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    arrange(
        vec![
            posed(
                "empty",
                material::TextField::new(samples::PLACEHOLDER, "", roles)
                    .on_input(|_| Message::NoOp),
                roles,
            ),
            posed(
                "filled",
                material::TextField::new(samples::PLACEHOLDER, samples::FILLED, roles)
                    .on_input(|_| Message::NoOp),
                roles,
            ),
            posed(
                "read-only",
                material::TextField::<Message>::new(samples::PLACEHOLDER, samples::FILLED, roles),
                roles,
            ),
        ],
        Layout::Inline,
    )
}

/// `Select` — nothing selected (so the placeholder shows) and a choice made.
///
/// Its dropdown panel is opened by the rendering stack rather than by the gallery, so it is exercised
/// live like hover and focus: click it and the menu appears.
pub fn select<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    arrange(
        vec![
            posed(
                "unset",
                material::Select::new(samples::CHOICES, None, |_| Message::NoOp, roles)
                    .placeholder("Choose a theme…"),
                roles,
            ),
            posed(
                "selected",
                material::Select::new(
                    samples::CHOICES,
                    Some(samples::CHOICES[1]),
                    |_| Message::NoOp,
                    roles,
                ),
                roles,
            ),
        ],
        Layout::Inline,
    )
}

/// `FilterTrigger` — the sidebar's filter button, active and inactive.
pub fn filter_trigger<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    arrange(
        vec![
            posed(
                "inactive",
                material::FilterTrigger::new(Message::NoOp, roles).active(false),
                roles,
            ),
            posed(
                "active",
                material::FilterTrigger::new(Message::NoOp, roles).active(true),
                roles,
            ),
        ],
        Layout::Inline,
    )
}

/// `ResizeHandle` — the draggable edge between two panes.
///
/// It has nothing to pose: a handle is a handle. Its whole behaviour is the drag, which is exercised
/// live by pressing and moving the pointer across it.
pub fn resize_handle<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    arrange(
        vec![posed(
            "drag it",
            iced::widget::container(
                material::ResizeHandle::new(roles).on_resize(|_| Message::NoOp),
            )
            .height(iced::Length::Fixed(HANDLE_HEIGHT)),
            roles,
        )],
        Layout::Inline,
    )
}
