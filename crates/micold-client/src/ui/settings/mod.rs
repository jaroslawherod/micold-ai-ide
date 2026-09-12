//! The Settings sections (feature 027, US3 — FR-026, FR-027, FR-028).
//!
//! One module per page of the view. Each exports two things: a `view` that renders its controls
//! from the one shared [`SettingsDraft`], and a `SETTINGS` declaration naming the persisted
//! settings it renders and the message each control emits.
//!
//! # Why a section declares what it renders
//!
//! Turning a modal into a sectioned view is a migration, and a migration's failure mode is not a
//! crash but a setting that quietly stops being reachable — it still parses, still saves, still
//! round-trips, and simply has no control any more. `tests/settings_sections.rs` reads these
//! declarations against the persisted shape in `micold-core` and against each module's own source,
//! so a field added to `Settings` with no control is a failing test rather than a silent loss.
//!
//! The pairing with a `Message` variant is what makes the declaration evidence rather than a
//! comment: it names a line that has to be in the module and a variant that has to be on
//! `Message`.

pub(crate) mod appearance;
pub(crate) mod daemon;
pub(crate) mod environment;
pub(crate) mod terminal;

use crate::app::Message;
use crate::features::settings::{SettingsDraft, SettingsSection};
use crate::features::window::FieldId;
use crate::ui::material::{Text, TypeRole};
use iced::widget::{column, container, Space};
use iced::{Element, Length};
use micold_core::tokens::{anatomy, spacing, Roles};

/// A durable enum, paired with the name the picker shows for it.
///
/// [`Select`](crate::ui::material::Select) needs its options to be `ToString`, and the settings
/// enums live in `micold-core` where a `Display` impl would be a rendering decision in the wrong
/// crate — the same reasoning that keeps the icon map out of the core. This is where the name is
/// chosen, beside the section that shows it.
///
/// Equality is on the value alone, deliberately: the select compares the current selection against
/// its options to decide which row is current, and comparing the label as well would make a
/// mistyped name look like "nothing is selected" rather than like a typo.
#[derive(Debug, Clone, Copy)]
pub struct Named<T>(pub T, pub &'static str);

impl<T: PartialEq> PartialEq for Named<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: PartialEq> Eq for Named<T> where T: Eq {}

impl<T> std::fmt::Display for Named<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.1)
    }
}

/// The name of a value, from a section's option list — so a summary line and the picker beside it
/// cannot disagree about what a setting is called.
pub fn name_of<T: PartialEq + Copy>(options: &[Named<T>], value: T) -> &'static str {
    options
        .iter()
        .find(|n| n.0 == value)
        .map(|n| n.1)
        .unwrap_or("Unknown")
}

/// A section's page: its title, a one-line description of what the page governs, and its controls.
///
/// Shared rather than repeated per section so that the four pages are laid out identically — the
/// thing a user notices immediately when they are not.
pub fn page<'a>(
    title: &'static str,
    blurb: &'static str,
    controls: Vec<Element<'a, Message>>,
    roles: Roles,
) -> Element<'a, Message> {
    let heading = column![
        Text::new(title, TypeRole::Headline, roles),
        Text::new(blurb, TypeRole::Caption, roles).muted(),
    ]
    .spacing(spacing::XS)
    .width(Length::Fill);

    let mut body = column![heading].spacing(spacing::MD).width(Length::Fill);
    for control in controls {
        body = body.push(control);
    }
    body.into()
}

/// A heading over a group of controls inside a page — a name for the group, never for one control,
/// which carries its own label inside its container (§7.7, FR-031a).
pub fn group<'a>(title: &'static str, roles: Roles) -> Element<'a, Message> {
    Text::new(title, TypeRole::Label, roles).muted().into()
}

/// An explanatory line under a control, in the ordinary muted tone.
pub fn note<'a>(text: impl Into<std::borrow::Cow<'a, str>>, roles: Roles) -> Element<'a, Message> {
    Text::new(text, TypeRole::Caption, roles).muted().into()
}

/// A note that belongs to the **control above it**, not to the page.
///
/// Same tone as [`note`], but returned attached to the control and inset to the column that
/// control's own supporting text sits in. A page stacks its controls at one margin, so a bare
/// `note` between two of them lands on the left edge of the *next* control and reads as belonging
/// to that one — which is exactly backwards for a line explaining the field above. Feature 027's
/// §B.6 pass found that: the missing-CLI sentence lined up with the checkbox under it rather than
/// with the select it was about. Sharing the field's inset and its own tighter spacing makes the
/// pair read as one block.
/// Takes the note as an `Option` and emits the column either way, with a `Space` where there is no
/// note — the same shape [`FormField`](crate::ui::material::FormField) gives its own supporting
/// text. A control that changes depth in the widget tree depending on whether it has something to
/// say makes every layout assertion about the page conditional on that, and the tests that press a
/// control address it by path.
pub fn field_note<'a>(
    control: impl Into<Element<'a, Message>>,
    text: Option<impl Into<std::borrow::Cow<'a, str>>>,
    roles: Roles,
) -> Element<'a, Message> {
    let beneath: Element<'a, Message> = match text {
        Some(text) => container(note(text, roles))
            .padding(iced::Padding {
                top: 0.0,
                bottom: 0.0,
                left: anatomy::text_field::PADDING,
                right: anatomy::text_field::PADDING,
            })
            .into(),
        None => Space::new().into(),
    };
    column![control.into(), beneath]
        .spacing(spacing::XS)
        .width(Length::Fill)
        .into()
}

/// An explanatory line that is a *warning* — a capability being granted, a restart being implied.
pub fn caution<'a>(
    text: impl Into<std::borrow::Cow<'a, str>>,
    roles: Roles,
) -> Element<'a, Message> {
    Text::new(text, TypeRole::Caption, roles)
        .tint(roles.error)
        .into()
}

/// The message to show against `field`, or `None` when the last rejected save was about some other
/// control.
///
/// Asked per control rather than per page so that a section holding three fields marks the one the
/// message is about. The section is checked as well as the field, so that a `FieldId` shared by two
/// sections could never light up both.
pub fn error_for(
    draft: &SettingsDraft,
    section: SettingsSection,
    field: FieldId,
) -> Option<String> {
    draft
        .error
        .as_ref()
        .filter(|e| e.section == section && e.field == field)
        .map(|e| e.message.clone())
}
