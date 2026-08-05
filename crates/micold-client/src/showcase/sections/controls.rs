//! The interactive controls (feature 020, T019): buttons, a checkbox, a chip, a field, a dropdown.
//!
//! This is the section US2's independent test walks and the one feature 018's SC-002/SC-004 need: a
//! row of every interactive component in the library, all hoverable and pressable in one pass. Every
//! instance here is real and live — none of hover, pressed or focus is faked (FR-004), which is why
//! each of these entries declares them as `live` rather than posing them.
//!
//! Where a control needs a message it has nowhere to send, it sends [`Message::NoOp`]. That keeps the
//! instance genuinely interactive without the gallery inventing behaviour the application owns.

use iced::{Element, Length};
use micold_core::naming::ConventionalType;
use micold_core::tokens::{spacing, Roles};

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
                    .accent(
                        roles.tag(ConventionalType::Fix).0,
                        roles.tag(ConventionalType::Fix).1,
                    ),
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

/// `FormField` — the shared chrome, posed through every state that changes it.
///
/// Wrapped around a **`Select`** rather than a text input, and deliberately: `TextField` composes
/// itself inside a `FormField` (T046), so posing one inside another would draw two containers and
/// two indicators — the gallery would be demonstrating a mistake. The select is the control that
/// still arrives bare, and it is also the one that cannot report focus, which is why the wrapper
/// takes its active state as a parameter rather than asking.
///
/// The states are side by side because the differences between them are the whole component: at
/// rest a muted hairline, active a thicker accent line, invalid the error role in *both* the
/// indicator and the text beneath. Seeing them apart, each looks plausible.
pub fn form_field<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    let control = || material::Select::new(samples::CHOICES, None, |_| Message::NoOp, roles);
    arrange(
        vec![
            posed(
                "label + supporting",
                material::FormField::new(control(), roles)
                    .label("Theme")
                    .supporting("Applies immediately"),
                roles,
            ),
            posed(
                "active",
                material::FormField::new(control(), roles)
                    .label("Theme")
                    .supporting("Applies immediately")
                    .active(true),
                roles,
            ),
            posed(
                "error",
                material::FormField::new(control(), roles)
                    .label("Theme")
                    .error(Some("Pick one to continue")),
                roles,
            ),
            posed(
                "no label",
                material::FormField::new(control(), roles),
                roles,
            ),
        ],
        Layout::FullWidth,
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

/// `Typeahead` — a live, typeable search over fixed sample results (FR-020).
///
/// The one entry on this page that cannot be posed: what a type-ahead looks like *is* what it does
/// as you type, and a still of a closed field would be a picture of the frame around the component
/// rather than of the component. So the gallery owns a query of its own, runs the sample rows
/// through the same matching logic the branch picker uses, and hands the result over — emphasis,
/// keyboard highlight, selection marker, disabled row and all.
///
/// One instance rather than several: its list floats over the page, and two open lists would sit on
/// top of each other.
pub fn typeahead<'a>(s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    let rows = s.typeahead_rows();
    arrange(
        vec![posed(
            "type to narrow it",
            material::Typeahead::new(
                s.typeahead_query(),
                rows,
                Message::TypeaheadQueryChanged,
                roles,
            )
            .placeholder("Search branches…")
            // Always open, because the list is the half worth looking at and this page exists to be
            // looked at. In the application the caller opens it on focus instead.
            .open(true)
            .highlighted(s.typeahead_highlight())
            .selected(s.typeahead_selected())
            .empty_message("Nothing matches that search.")
            .on_move(Message::TypeaheadHighlightMoved)
            .on_pick(Message::TypeaheadPicked),
            roles,
        )],
        Layout::FullWidth,
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

/// `Ripple` — the press indication.
///
/// Lives among the interactive components rather than in the motion section, even though what it
/// shows is an animation: that section is for `animation.rs`'s replayable helpers, and a ripple has
/// nothing to replay. It is driven by pressing it, which is also the only way to show the thing
/// that matters — that it starts from *where* you pressed.
pub fn ripple<'a>(_showcase: &'a Showcase, roles: Roles, _index: usize) -> Element<'a, Message> {
    material::Ripple::new(
        material::Surface::new(
            material::Text::<Message>::new(
                "press anywhere on this surface",
                material::TypeRole::Body,
                roles,
            ),
            material::SurfaceKind::Plain,
            roles,
        )
        .padding(spacing::LG)
        .width(Length::Fill)
        .center_x(),
        roles.on_surface,
        // Asked of the surface rather than restated, so the gallery cannot drift from the shape it
        // is demonstrating — the exact drift that let the ripple overhang a pill in the first place.
        material::SurfaceKind::Plain.shape(),
    )
    .into()
}

pub fn ripple_component<'a>(
    showcase: &'a Showcase,
    roles: Roles,
    index: usize,
) -> Element<'a, Message> {
    arrange(
        vec![posed(
            "press it, anywhere",
            ripple(showcase, roles, index),
            roles,
        )],
        Layout::FullWidth,
    )
}
